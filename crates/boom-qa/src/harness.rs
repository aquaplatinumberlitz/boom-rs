//! Chạy một phiên QA: **1 server + N bot headless** qua `boom-net::MemoryTransport`.
//!
//! # Trình tự
//!
//! 1. Dựng server thật (`boom-server`) với identity SQLite trong thư mục tạm (ADR 0011 §10).
//! 2. Mỗi bot nối qua hai nửa `MemoryTransport` (client→server và server→client) cấu hình theo
//!    hồ sơ mạng: RTT, jitter, loss, trùng, đảo thứ tự.
//! 3. Vòng lặp: `server.poll(now)` → bot nhận frame → bot gửi lệnh → đo độ trễ input.
//!
//! # Điểm thay thế bot
//!
//! Harness **không** tự dựng bot: nó gọi [`BotFactory`] cho từng bot của lần chạy.
//! [`run`] là wrapper tiện lợi dùng factory mặc định (`DirectDtoBot`); [`run_with`] nhận
//! factory bên ngoài để một implementation [`BotClient`] khác (ví dụ bot dự đoán state dùng
//! module net của `boom-client`) chạy được trong harness mà không phải sửa harness.
//!
//! # Đồng hồ
//!
//! Mọi thời gian của lần chạy là **đồng hồ thật tính từ lúc bắt đầu** (ms kể từ 0). Server nhận
//! `now_ms` tiêm vào, nên tick server chạy đúng nhịp thật; transport `Timing::Real` ngủ đúng số
//! ms ảo nên độ trễ đo được là độ trễ thật, không phải số ảo.
//!
//! # Hội tụ state hash
//!
//! Khi trận kết thúc, harness ghi lại `state_hash` của server (state cuối round). Bot xin
//! `Snapshot` (resync) và dựng lại `SimulationState` bằng `boom_sync::from_snapshot`, rồi so
//! hash của state mình dựng với hash server. Trận chỉ "hội tụ" khi **mọi** bot khớp.

use std::collections::BTreeSet;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use boom_content::map::Terrain;
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStatsTable, World, mode_by_id};
use boom_net::{MemoryTransport, Transport};
use boom_room::PlayerId;
use boom_server::auth::{AuthConfig, AuthService};
use boom_server::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use boom_server::{
    Authority, HandshakeConfig, InternalServerEvent, MatchSetup, Server, ServerConfig, ServerPhase,
};
use sha2::{Digest, Sha256};

use crate::bot::{BotClient, BotConfig, BotFactory, BotPhase, DirectDtoBotFactory};
use crate::metrics::Samples;
use crate::profile::{ProfileError, QaProfile, builtin_map, builtin_rules, map_rows};
use crate::report::{
    BotStatus, CorrectionReport, InputReport, LatencyReport, NetworkReport, RoundReport, RunReport,
};

/// Lỗi của một lần chạy QA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QaError {
    /// Hồ sơ không đọc được.
    Profile(ProfileError),
    /// Profile rules / bản đồ / world không dựng được.
    Setup(String),
    /// Server lỗi.
    Server(String),
    /// Bot lỗi.
    Bot(String),
    /// Không kết nối được transport.
    Net(String),
}

impl fmt::Display for QaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Profile(err) => write!(f, "hồ sơ QA: {err}"),
            Self::Setup(detail) => write!(f, "dựng môi trường QA: {detail}"),
            Self::Server(detail) => write!(f, "server: {detail}"),
            Self::Bot(detail) => write!(f, "bot: {detail}"),
            Self::Net(detail) => write!(f, "transport: {detail}"),
        }
    }
}

impl std::error::Error for QaError {}

/// Round ID **public** của server, ở kiểu `u64`.
///
/// Server ánh xạ ID round nội bộ sang ID public (`RoomState.round.id`, `RoundResult.round_id`);
/// `Authority::current_round_id` trả về dạng `u32` (giới hạn wire của `InputBatch`), nên crate
/// này quy đổi về `u64` để khớp với `LoadingAck`/`RoundResult` mà bot nhận được.
fn public_round_id(server: &Server) -> Option<u64> {
    server.authority().current_round_id().map(u64::from)
}

/// Cấu hình một lần chạy.
#[derive(Clone, Debug)]
pub struct RunConfig {
    /// Hồ sơ mạng + tham số harness.
    pub profile: QaProfile,
    /// Số bot (2..=4 theo SCOPE_V1 §1).
    pub clients: usize,
    /// Seed cho lỗi mạng và input ngẫu nhiên của bot.
    pub seed: u64,
    /// Số trận; `None` = lấy theo hồ sơ.
    pub rounds: Option<u32>,
}

impl RunConfig {
    /// Số trận của lần chạy này.
    pub fn rounds(&self) -> u32 {
        self.rounds.unwrap_or(self.profile.harness.rounds)
    }
}

/// Một bot trong lúc chạy (transport + logic).
struct BotRuntime {
    client: Box<dyn BotClient>,
    config: BotConfig,
    writer: MemoryTransport,
    frames: Receiver<Vec<u8>>,
    player: PlayerId,
    name: String,
    /// Chỉ số bot trong lần chạy (0..clients).
    index: usize,
    /// Sequence server đã giải quyết, của lượt hiện tại.
    last_seq: u32,
    /// Lượt mà `last_seq` thuộc về (0 = chưa vào lượt nào).
    last_round: u64,
    /// Mốc (ms) mở phiên hiện tại.
    session_started_at_ms: u64,
    /// Số lần đã mở lại phiên (chốt lặp vô hạn).
    reconnects: u32,
    /// Phiên vừa bị server đóng (reader thấy EOF).
    dead: bool,
}

/// Mở lại phiên cho bot khi phiên cũ chết hoặc kẹt ở bước auth.
///
/// V1: reconnect cần **auth mới** (SCOPE_V1 §3) và tài khoản chỉ có một phiên sống, nên bot đăng
/// nhập lại bằng `Login` (lần đăng ký đầu đã tạo tài khoản).
///
/// Bot giữ **cùng `PlayerId`**: `PlayerId` là định danh người chơi trên server, đổi nó sẽ
/// thành người khác trong phòng (và thành người khác trong roster của lượt). Server giải phóng
/// session cũ khi thấy EOF, nên harness chờ `accept` thành công rồi mới mở phiên mới.
fn reconnect(
    server: &mut Server,
    profile: &QaProfile,
    seed: u64,
    bot: &mut BotRuntime,
    now_ms: u64,
) -> Result<(), QaError> {
    bot.reconnects += 1;
    // Đóng nửa ghi: server thấy EOF và tự thả session cũ (kể cả phiên account).
    let _ = bot.writer.close();
    // Server báo `Closed` khi thấy EOF; `Closed` mang theo **PlayerId** nên nếu tới sau
    // khi session mới đăng ký, nó sẽ đóng nhầm session mới (và phiên mới chết ngay sau
    // khi vừa mở). Vì vậy phải để server xử lý hết frame còn tồn đọng — kể cả `Closed`
    // của session cũ — TRƯỚC khi đăng ký session mới.
    for _ in 0..RECONNECT_DRAIN_ROUNDS {
        let _ = server.poll(now_ms, Duration::ZERO);
        thread::sleep(Duration::from_millis(RECONNECT_DRAIN_STEP_MS));
    }
    bot.config.seed = mix(seed, 0x51ed_270b + u64::from(bot.reconnects));
    let player = bot.player;
    let slot = u8::try_from(bot.index).unwrap_or(u8::MAX);
    let spec = ConnectSpec {
        seed,
        index: bot.index,
        player,
        slot,
        attempt: bot.reconnects,
        now_ms,
    };
    let (writer, frames) = connect(server, profile, &spec)?;
    bot.writer = writer;
    bot.frames = frames;
    bot.last_seq = 0;
    bot.last_round = 0;
    bot.dead = false;
    bot.session_started_at_ms = now_ms;
    bot.client.reset(player.0);
    Ok(())
}

/// Tham số mở một phiên cho bot.
struct ConnectSpec {
    /// Seed mạng tổng của lần chạy.
    seed: u64,
    /// Chỉ số bot (dùng để tách dòng lỗi mạng và IP nguồn).
    index: usize,
    /// `PlayerId` của bot.
    player: PlayerId,
    /// Slot session (server bind theo `PlayerId`, không phải slot trong phòng).
    slot: u8,
    /// Số lần mở phiên (tách dòng lỗi mạng giữa các lần thử).
    attempt: u32,
    /// Mốc thời gian hiện tại của server (ms).
    now_ms: u64,
}

/// Mở một cặp transport và đăng ký session cho server; trả về nửa ghi của bot và kênh nhận.
///
/// Server chỉ nhận một session cho mỗi `PlayerId`; harness chờ server thả session cũ (do EOF)
/// rồi mới mở phiên mới, nên phải thử lại cho tới khi được.
fn connect(
    server: &mut Server,
    profile: &QaProfile,
    spec: &ConnectSpec,
) -> Result<(MemoryTransport, Receiver<Vec<u8>>), QaError> {
    let salt = u64::from(spec.attempt) * 1_000 + spec.index as u64;
    let up = profile.memory_config(mix(spec.seed, salt * 2 + 1));
    let down = profile.memory_config(mix(spec.seed, salt * 2 + 2));
    let source_ip = IpAddr::V4(Ipv4Addr::new(
        127,
        0,
        0,
        u8::try_from(spec.index + 1).unwrap_or(1),
    ));
    let mut last_error = String::new();
    for _ in 0..CONNECT_ATTEMPTS {
        let (client_writer, server_reader) = MemoryTransport::pair(up);
        let (server_writer, client_reader) = MemoryTransport::pair(down);
        match server.accept(
            spec.player,
            spec.slot,
            source_ip,
            server_reader,
            server_writer,
        ) {
            Ok(()) => {
                let (tx, frames) = mpsc::channel();
                thread::spawn(move || {
                    let mut reader = client_reader;
                    while let Ok(Some(frame)) = reader.recv_frame() {
                        if tx.send(frame).is_err() {
                            break;
                        }
                    }
                });
                return Ok((client_writer, frames));
            }
            Err(err) => {
                last_error = err.to_string();
                // Server chưa thả session cũ: cho nó xử lý EOF rồi thử lại.
                let _ = server.poll(spec.now_ms, Duration::ZERO);
                thread::sleep(Duration::from_millis(CONNECT_RETRY_MS));
            }
        }
    }
    Err(QaError::Server(format!(
        "mở phiên cho {:?} không được: {last_error}",
        spec.player
    )))
}

/// Trộn seed để mỗi bot và mỗi chiều có dòng lỗi mạng riêng, vẫn tái lập được.
fn mix(seed: u64, salt: u64) -> u64 {
    seed ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

/// Số lần thử mở lại phiên khi server chưa thả session cũ.
const CONNECT_ATTEMPTS: u32 = 40;

/// Thời gian chờ giữa hai lần thử mở phiên (ms).
const CONNECT_RETRY_MS: u64 = 50;

/// Số vòng "poll server + ngủ" để server xử lý EOF và dẹp frame tồn đọng trước khi
/// đăng ký session mới (xem [`reconnect`]).
const RECONNECT_DRAIN_ROUNDS: u32 = 5;

/// Thời gian ngủ giữa hai vòng drain (ms).
const RECONNECT_DRAIN_STEP_MS: u64 = 20;

/// Chạy một phiên QA và trả về báo cáo.
///
/// Đây là **wrapper tiện lợi** của [`run_with`]: bot mặc định là [`DirectDtoBotFactory`]
/// (bot nói DTO trực tiếp). Muốn chạy với implementation bot khác thì dùng [`run_with`].
pub fn run(config: &RunConfig) -> Result<RunReport, QaError> {
    run_with(config, &DirectDtoBotFactory)
}

/// Chạy một phiên QA với **factory bot** do bên ngoài cài, rồi trả về báo cáo.
///
/// Đây là **điểm thay thế**: harness không tự dựng bot, nên một implementation khác của
/// [`BotClient`] (ví dụ bot dự đoán state dùng module net của `boom-client`) chạy được
/// trong harness **mà không phải sửa harness**. Factory nhận [`BotConfig`] mà harness đã
/// tính sẵn cho từng bot (tên, tài khoản, `PlayerId`, vai trò host, thế giới, profile
/// rules, nhịp input/ping…).
pub fn run_with(config: &RunConfig, factory: &dyn BotFactory) -> Result<RunReport, QaError> {
    let harness = &config.profile.harness;
    let rules_text = builtin_rules(&harness.rules_profile).map_err(QaError::Profile)?;
    let rules_profile = RulesProfile::from_toml(rules_text)
        .map_err(|err| QaError::Setup(format!("profile rules: {err}")))?;
    let rules = boom_core::Rules::from_profile(&rules_profile)
        .map_err(|err| QaError::Setup(format!("rules: {err}")))?;
    let map_text = builtin_map(&harness.map).map_err(QaError::Profile)?;
    let rows = map_rows(map_text);
    let terrain = Terrain::from_ascii(&rows)
        .map_err(|err| QaError::Setup(format!("bản đồ `{}`: {err}", harness.map)))?;
    let mode = mode_by_id("battle_ffa")
        .ok_or_else(|| QaError::Setup("không có mode battle_ffa".to_owned()))?;
    let world =
        World::new(terrain, rules, mode).map_err(|err| QaError::Setup(format!("world: {err}")))?;
    let content_sha256: [u8; 32] = Sha256::digest(map_text.as_bytes()).into();

    let stats = CharacterStatsTable::from_profile(&rules_profile)
        .map_err(|err| QaError::Setup(format!("chỉ số nhân vật: {err}")))?;
    let setup = MatchSetup {
        world: world.clone(),
        stats,
        map: harness.map.clone(),
        replay: None,
    };
    let authority = Authority::new(ServerConfig::v1(), setup);
    let handshake = HandshakeConfig {
        rules_profile_id: rules_profile.profile_id.clone(),
        rules_hash: authority.rules_hash(),
        content_sha256,
        content_epoch: 1,
        snapshot_interval_ms: harness.snapshot_interval_ms,
    };

    let db_dir =
        tempfile::tempdir().map_err(|err| QaError::Setup(format!("tạo thư mục tạm: {err}")))?;
    let db_path = db_dir.path().join("qa-auth.sqlite3");
    let store = SqliteAccountStore::open(&db_path, SqliteOptions::default())
        .map_err(|err| QaError::Setup(format!("mở store identity: {err}")))?;
    let auth = AuthService::new(
        qa_auth_config(),
        format!("qa-instance-{}", config.seed),
        Box::new(OsRandomSource),
    )
    .map_err(|err| QaError::Setup(format!("auth service: {err}")))?;
    let mut server = Server::with_auth_and_timers(
        authority,
        handshake,
        auth,
        Box::new(store),
        0,
        harness.server_ping_interval_ms,
        harness.server_liveness_timeout_ms,
    )
    .map_err(|err| QaError::Server(err.to_string()))?;

    let mut bots: Vec<BotRuntime> = Vec::with_capacity(config.clients);
    for index in 0..config.clients {
        let slot = u8::try_from(index).unwrap_or(u8::MAX);
        let bot_config = BotConfig {
            name: format!("bot{index}"),
            username: format!("qa_bot_{}_{index}", config.seed),
            password: format!("qa-pass-{}-{index}", config.seed),
            player: index as u64 + 1,
            is_host: index == 0,
            expected_members: config.clients,
            seed: mix(config.seed, 0x51ed_270b),
            world: world.clone(),
            rules_profile_id: rules_profile.profile_id.clone(),
            rules_hash: server.authority().rules_hash(),
            content_sha256,
            input_interval_ms: harness.input_interval_ms,
            input_lead_ticks: harness.input_lead_ticks,
            ping_interval_ms: harness.ping_interval_ms,
            retry_ms: harness.retry_ms,
            lobby_retry_ms: harness.lobby_retry_ms,
            one_way_delay_ms: u64::from(config.profile.network.one_way_delay_ms()),
            auth_retry_ms: harness.auth_retry_ms,
            snapshot_retry_ms: harness.snapshot_retry_ms,
            snapshot_deadline_ms: harness.snapshot_deadline_ms,
            room_state_deadline_ms: harness.room_state_deadline_ms,
        };
        let player = PlayerId(bot_config.player);
        let spec = ConnectSpec {
            seed: config.seed,
            index,
            player,
            slot,
            attempt: 0,
            now_ms: 0,
        };
        let (writer, frames) = connect(&mut server, &config.profile, &spec)?;
        bots.push(BotRuntime {
            client: factory.create(&bot_config),
            config: bot_config,
            writer,
            frames,
            player,
            name: format!("bot{index}"),
            index,
            last_seq: 0,
            last_round: 0,
            session_started_at_ms: 0,
            reconnects: 0,
            dead: false,
        });
    }

    let start = Instant::now();
    let mut retired: BTreeSet<(u64, u64, u32)> = BTreeSet::new();
    let mut rounds: Vec<RoundReport> = Vec::new();
    let mut last_round: Option<u64> = None;
    let mut latency = Samples::new();
    let mut resolved = 0u32;
    let mut retired_count = 0u32;
    let mut timed_out = false;

    loop {
        let now = start.elapsed().as_millis() as u64;
        let t0 = Instant::now();
        let events = server
            .poll(now, Duration::ZERO)
            .map_err(|err| QaError::Server(err.to_string()))?;
        let poll_us = t0.elapsed().as_micros() as u64;
        let t1 = Instant::now();
        for event in events {
            if std::env::var_os("BOOM_QA_TRACE").is_some() {
                let name = match &event {
                    InternalServerEvent::LoadingAcked { .. } => "LoadingAcked",
                    InternalServerEvent::MatchStarted { .. } => "MatchStarted",
                    InternalServerEvent::RoundFinished { .. } => "RoundFinished",
                    InternalServerEvent::ReturnedToWaiting { .. } => "ReturnedToWaiting",
                    InternalServerEvent::RoundCancelled { .. } => "RoundCancelled",
                    InternalServerEvent::PlayerDisconnected { .. } => "PlayerDisconnected",
                    InternalServerEvent::PlayerEliminated { .. } => "PlayerEliminated",
                    InternalServerEvent::InputRetired { .. } => "InputRetired",
                };
                eprintln!("[harness] t={now} event {name}");
            }
            if let InternalServerEvent::InputRetired { player, seq, .. } = event {
                let round = public_round_id(&server).unwrap_or(0);
                retired.insert((round, player.0, seq));
                retired_count += 1;
            }
        }
        if server.phase() == ServerPhase::Results
            && let Some(round_id) = public_round_id(&server)
            && last_round != Some(round_id)
        {
            last_round = Some(round_id);
            rounds.push(RoundReport {
                round_id,
                last_tick: server.authority().tick().unwrap_or(0),
                server_hash: server.authority().state_hash().unwrap_or(0),
                bot_hashes: Vec::new(),
                converged: false,
                bots: config.clients,
            });
        }

        // Bot nhận frame rồi gửi lệnh.
        for bot in &mut bots {
            let mut pending: Vec<Vec<u8>> = Vec::new();
            loop {
                match bot.frames.try_recv() {
                    Ok(frame) => {
                        pending.extend(
                            bot.client
                                .on_frame(now, &frame)
                                .map_err(|err| QaError::Bot(format!("{}: {err}", bot.name)))?,
                        );
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        // Server đóng phiên (EOF): bot sẽ được mở lại phiên nếu còn hạn.
                        if std::env::var_os("BOOM_QA_TRACE").is_some() {
                            eprintln!("[harness] t={now} {} bị đóng phiên", bot.name);
                        }
                        bot.dead = true;
                        break;
                    }
                }
            }
            pending.extend(
                bot.client
                    .on_tick(now)
                    .map_err(|err| QaError::Bot(format!("{}: {err}", bot.name)))?,
            );
            for frame in pending {
                bot.writer
                    .send_frame(&frame)
                    .map_err(|err| QaError::Net(err.to_string()))?;
            }
        }

        // Mở lại phiên cho bot chết hoặc kẹt ở bước auth.
        for bot in &mut bots {
            let stuck = bot.client.phase() < BotPhase::Lobby
                && now.saturating_sub(bot.session_started_at_ms) >= harness.auth_deadline_ms;
            if (bot.dead || stuck) && bot.reconnects < harness.max_reconnects {
                if std::env::var_os("BOOM_QA_TRACE").is_some() {
                    eprintln!(
                        "[harness] t={now} mở lại phiên cho {} (dead={}, stuck={}, pha={:?}, lần {})",
                        bot.name,
                        bot.dead,
                        stuck,
                        bot.client.phase(),
                        bot.reconnects + 1
                    );
                }
                reconnect(&mut server, &config.profile, config.seed, bot, now)?;
            }
        }

        // Đo độ trễ input: sequence server vừa giải quyết.
        //
        // Sequence input **đánh số lại theo từng lượt** (ADR 0009 §3.1), nên khi vào lượt
        // mới phải đặt lại mốc đã đo, nếu không sẽ so nhầm với lượt trước.
        for bot in &mut bots {
            let round = public_round_id(&server).unwrap_or(0);
            if bot.last_round != round {
                bot.last_round = round;
                bot.last_seq = 0;
            }
            // Server khoá input theo **account_id** (định danh người chơi trên dây, ADR
            // 0009 Addendum B §B.2), không phải `PlayerId` của kết nối: phải tra bằng
            // `account_id` mà bot học từ `AuthOk`, nếu không thì `applied_seq` luôn rỗng
            // ⇒ không đo được độ trễ nào.
            let Some(account_id) = bot.client.stats().account_id else {
                continue;
            };
            let applied = server
                .authority()
                .applied_seq(PlayerId(account_id))
                .unwrap_or(0);
            if applied > bot.last_seq {
                for seq in (bot.last_seq + 1)..=applied {
                    let Some(sent) = bot.client.stats().sent.get(&(round, seq)) else {
                        continue;
                    };
                    resolved += 1;
                    if retired.contains(&(round, account_id, seq)) {
                        continue;
                    }
                    latency.push(now.saturating_sub(sent.sent_at_ms));
                }
                bot.last_seq = applied;
            }
        }

        // Chạy tới khi mọi bot đã chơi xong **và** đã ack kết quả: bot phải nhận được
        // state cuối round (để dựng lại hash) trước khi ack, nên lần chạy chỉ kết thúc sau
        // bước đó — nếu không, báo cáo sẽ thiếu hash của bot.
        //
        // Mỗi bot được tính là đã chơi xong lượt khi **nhận được** kết quả lượt và đã ack;
        // `rounds` của harness là lượt server đã kết thúc.
        let done = bots
            .iter()
            .all(|bot| bot.client.stats().rounds_completed >= config.rounds())
            && bots
                .iter()
                .all(|bot| bot.client.stats().results_acked >= config.rounds())
            && rounds.len() >= config.rounds() as usize;
        if done {
            break;
        }
        if now > harness.timeout_ms {
            timed_out = true;
            break;
        }
        if std::env::var_os("BOOM_QA_TRACE").is_some()
            && (poll_us > 5_000 || t1.elapsed().as_micros() > 5_000)
        {
            eprintln!(
                "[harness] t={now} poll {poll_us} µs, bot {} µs",
                t1.elapsed().as_micros()
            );
        }
        thread::sleep(Duration::from_millis(harness.step_ms));
    }

    // Dọn: đóng nửa ghi của bot để server và các luồng đọc kết thúc.
    for bot in &mut bots {
        let _ = bot.writer.close();
    }
    for _ in 0..50 {
        let _ = server.poll(start.elapsed().as_millis() as u64, Duration::ZERO);
    }

    // Gom hash bot theo trận và so với server.
    for round in &mut rounds {
        let mut hashes = Vec::new();
        let mut all_match = true;
        for bot in &bots {
            match bot.client.stats().round_hashes.get(&round.round_id) {
                Some(hash) => {
                    hashes.push((bot.name.clone(), *hash));
                    if *hash != round.server_hash {
                        all_match = false;
                    }
                }
                None => all_match = false,
            }
        }
        round.bot_hashes = hashes;
        round.converged = all_match;
    }

    let sent: u32 = bots
        .iter()
        .map(|bot| bot.client.stats().commands_sent)
        .sum();
    let rejected: u32 = bots
        .iter()
        .map(|bot| bot.client.stats().input_rejected)
        .sum();
    let room_rejected: u32 = bots
        .iter()
        .map(|bot| bot.client.stats().room_rejected)
        .sum();
    let snapshot_missed: u32 = bots
        .iter()
        .map(|bot| bot.client.stats().snapshot_missed)
        .sum();
    let corrections: u32 = bots.iter().map(|bot| bot.client.stats().corrections).sum();
    let max_correction: i32 = bots
        .iter()
        .map(|bot| bot.client.stats().max_correction_mpx)
        .max()
        .unwrap_or(0);
    let mut rtt = Samples::new();
    for bot in &bots {
        for value in bot.client.stats().rtt.values() {
            rtt.push(*value);
        }
    }
    let converged = !timed_out && rounds.iter().all(|round| round.converged);

    // Trạng thái cuối của từng bot: giải thích lần chạy hết ngân sách (bot kẹt ở bước
    // nào, đã mở lại phiên bao nhiêu lần, đã chơi được mấy lượt).
    let bot_status: Vec<BotStatus> = bots
        .iter()
        .map(|bot| BotStatus {
            name: bot.name.clone(),
            phase: format!("{:?}", bot.client.phase()),
            reconnects: bot.reconnects,
            rounds_completed: bot.client.stats().rounds_completed,
            results_acked: bot.client.stats().results_acked,
            auth_failed: bot.client.stats().auth_failed,
            input_rejected: bot.client.stats().input_rejected,
            room_rejected: bot.client.stats().room_rejected,
            snapshot_missed: bot.client.stats().snapshot_missed,
        })
        .collect();

    let report = RunReport {
        schema: crate::report::REPORT_SCHEMA,
        profile: config.profile.name.clone(),
        rules_profile: rules_profile.profile_id.clone(),
        map: harness.map.clone(),
        seed: config.seed,
        clients: config.clients,
        rounds_target: config.rounds(),
        duration_ms: start.elapsed().as_millis() as u64,
        network: NetworkReport {
            rtt_ms: config.profile.network.rtt_ms,
            one_way_delay_ms: config.profile.network.one_way_delay_ms(),
            jitter_ms: config.profile.network.jitter_ms,
            loss_permille: config.profile.network.loss_permille,
            duplicate_permille: config.profile.network.duplicate_permille,
            reorder_permille: config.profile.network.reorder_permille,
        },
        rounds,
        input: InputReport {
            sent,
            resolved,
            retired: retired_count,
            rejected,
            room_rejected,
            snapshot_missed,
        },
        latency_ms: LatencyReport::from_samples(&latency),
        rtt_ms: LatencyReport::from_samples(&rtt),
        corrections: CorrectionReport {
            count: corrections,
            max_amplitude_mpx: max_correction,
            note: "giai đoạn 1 bot nói DTO trực tiếp, không dự đoán state nên không có correction"
                .to_owned(),
        },
        converged,
        timed_out,
        bots: bot_status,
    };
    Ok(report)
}

/// Cấu hình auth cho lần chạy QA.
///
/// Mặc định của server giới hạn **5 lần** đăng nhập/đăng ký thất bại (hoặc 5 lượt đăng ký)
/// trong 5 phút, tính theo tài khoản **và** theo nguồn. Một lần chạy QA cố ý làm mất frame
/// nên bot có thể phải đăng nhập lại nhiều lần, và mọi bot đều đi từ loopback: giữ nguyên
/// mặc định sẽ khóa chính lần chạy đó. Đây là **tham số fixture kiểm thử**, không phải luật
/// chơi và không đổi mặc định vận hành của server (ADR 0011 §5).
fn qa_auth_config() -> AuthConfig {
    AuthConfig {
        rate_limit_max_failures: 64,
        register_rate_limit_max_attempts: 64,
        ..AuthConfig::default()
    }
}
