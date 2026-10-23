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
//! `TimingMode::Real` dùng đồng hồ thật từ lúc bắt đầu; transport ngủ theo độ trễ mạng.
//! `TimingMode::Virtual` tiến bước cố định sau rào reader/writer, không dùng tốc độ máy để
//! quyết tick, retry hay liveness. MemoryTransport giao ngay khi nhận hỏi: độ trễ ảo đổi thứ
//! tự frame trong batch, không phải mô phỏng delay theo tick authority. Số đo chế độ ảo
//! là thời gian fixture, không dùng làm benchmark độ trễ mạng thật.
//!
//! # Hội tụ state hash
//!
//! Khi trận kết thúc, harness ghi lại `state_hash` của server (state cuối round). Bot xin
//! `Snapshot` (resync) và dựng lại `SimulationState` bằng `boom_sync::from_snapshot`, rồi so
//! hash của state mình dựng với hash server. Trận chỉ "hội tụ" khi **mọi** bot khớp.

use std::collections::{BTreeMap, BTreeSet};
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
use boom_server::auth::{Argon2Settings, AuthConfig, AuthService};
use boom_server::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use boom_server::{
    Authority, HandshakeConfig, InternalServerEvent, MatchSetup, Server, ServerConfig, ServerPhase,
};
use sha2::{Digest, Sha256};

use crate::bot::{BotClient, BotConfig, BotFactory, BotPhase, DirectDtoBotFactory};
use crate::harness_io::{EnqueuedReader, Io, wait_until};
use crate::metrics::Samples;
use crate::profile::TimingMode;
use crate::profile::{ProfileError, QaProfile, builtin_map, builtin_rules, map_rows};
use crate::report::{
    BotStatus, CorrectionReport, InputReport, LatencyReport, NetworkReport, RoundReport, RunReport,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
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
    io: Io,
    buffered: Vec<Vec<u8>>,
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
/// Bot tái dùng `PlayerId` kết nối; định danh trong phòng/roster vẫn là `account_id`
/// học từ AuthOk (ADR 0009 Addendum B). Server giải phóng
/// session cũ khi thấy EOF. Chế độ ảo chờ reader enqueue Closed và writer đóng trước
/// khi accept phiên mới; chế độ đo thật giữ khoảng drain vận hành.
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
    if profile.network.timing == TimingMode::Virtual {
        wait_until(|| bot.io.up.closed.load(Ordering::Acquire))?;
        server
            .poll(now_ms, Duration::ZERO)
            .map_err(|err| QaError::Server(err.to_string()))?;
        let reader = bot.io.reader.as_mut().expect("virtual reader");
        wait_until(|| {
            let _ = reader.try_recv_frame();
            reader.is_eof()
        })?;
    } else {
        for _ in 0..RECONNECT_DRAIN_ROUNDS {
            let _ = server.poll(now_ms, Duration::ZERO);
            thread::sleep(Duration::from_millis(RECONNECT_DRAIN_STEP_MS));
        }
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
    let (writer, frames, io) = connect(server, profile, &spec)?;
    bot.writer = writer;
    bot.frames = frames;
    bot.buffered.clear();
    bot.io = io;
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
) -> Result<(MemoryTransport, Receiver<Vec<u8>>, Io), QaError> {
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
        let (server_reader, up_progress) = EnqueuedReader::new(server_reader);
        let retired = Arc::new(AtomicBool::new(false));
        let retired_signal = retired.clone();
        match server.accept_with_shutdown(
            spec.player,
            spec.slot,
            source_ip,
            server_reader,
            server_writer,
            move || {
                retired_signal.store(true, Ordering::Release);
            },
        ) {
            Ok(()) => {
                let (tx, frames) = mpsc::channel();
                let virtual_reader = if profile.network.timing == TimingMode::Virtual {
                    Some(client_reader)
                } else {
                    thread::spawn(move || {
                        let mut reader = client_reader;
                        while let Ok(Some(frame)) = reader.recv_frame() {
                            if tx.send(frame).is_err() {
                                break;
                            }
                        }
                    });
                    None
                };
                return Ok((
                    client_writer,
                    frames,
                    Io {
                        reader: virtual_reader,
                        retired,
                        up: up_progress,
                    },
                ));
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

/// Lỗi được tiêm đúng một lần giữa trận, vào client cuối.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecoveryScenario {
    /// Mạng theo profile, không ngắt chủ động.
    #[default]
    Steady,
    /// Đóng kết nối giữa trận, auth và vào lại ở lượt sau.
    Disconnect,
    /// Bỏ Pong tới khi server thu hồi phiên vì hết liveness.
    Liveness,
    /// Mất frame RoundResult đầu ở mỗi bot; kiểm request recovery và bảng authority.
    RoundResultLoss,
}

/// Chạy cùng bot/predictor với một lỗi vòng đời có kiểm soát.
pub fn run_scenario(config: &RunConfig, scenario: RecoveryScenario) -> Result<RunReport, QaError> {
    run_impl(config, &DirectDtoBotFactory, scenario)
}

/// Chạy một phiên QA với **factory bot** do bên ngoài cài, rồi trả về báo cáo.
///
/// Đây là **điểm thay thế**: harness không tự dựng bot, nên một implementation khác của
/// [`BotClient`] (ví dụ bot dự đoán state dùng module net của `boom-client`) chạy được
/// trong harness **mà không phải sửa harness**. Factory nhận [`BotConfig`] mà harness đã
/// tính sẵn cho từng bot (tên, tài khoản, `PlayerId`, vai trò host, thế giới, profile
/// rules, nhịp input/ping…).
pub fn run_with(config: &RunConfig, factory: &dyn BotFactory) -> Result<RunReport, QaError> {
    run_impl(config, factory, RecoveryScenario::Steady)
}

fn run_impl(
    config: &RunConfig,
    factory: &dyn BotFactory,
    scenario: RecoveryScenario,
) -> Result<RunReport, QaError> {
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
        let (writer, frames, io) = connect(&mut server, &config.profile, &spec)?;
        bots.push(BotRuntime {
            client: factory.create(&bot_config),
            config: bot_config,
            writer,
            frames,
            io,
            buffered: Vec::new(),
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
    let mut drain_started = None;
    let mut fault_round = None;
    let mut fault_disconnected = false;
    let mut dropped_results = BTreeMap::<usize, boom_proto::RoundResult>::new();
    let drain_ms = u64::from(
        config.profile.network.one_way_delay_ms()
            + config.profile.network.jitter_ms
            + config.profile.network.reorder_window_ms,
    ) * 2
        + harness.step_ms * 4;

    let virtual_time = config.profile.network.timing == TimingMode::Virtual;
    let mut simulated_ms = 0;
    loop {
        let now = if virtual_time {
            simulated_ms
        } else {
            start.elapsed().as_millis() as u64
        };
        if virtual_time {
            for bot in &bots {
                if !bot.dead {
                    bot.io.up.wait(bot.writer.sent_frames())?;
                }
            }
        }
        let t0 = Instant::now();
        let events = server
            .poll(now, Duration::ZERO)
            .map_err(|err| QaError::Server(err.to_string()))?;
        if virtual_time {
            server
                .flush_outbound(Duration::from_secs(10))
                .map_err(QaError::Server)?;
            for bot in &mut bots {
                if bot.io.retired.load(Ordering::Acquire) {
                    // Writer phiên đã đóng phải xong trước khi xử lý EOF ở bot.
                    let reader = bot.io.reader.as_mut().expect("virtual reader");
                    // Giữ frame còn tồn để bot xử lý trước EOF ở dưới.
                    wait_until(|| {
                        if let Ok(Some(frame)) = reader.try_recv_frame() {
                            bot.buffered.push(frame);
                        }
                        reader.is_eof()
                    })?;
                }
            }
        }
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
        if matches!(
            scenario,
            RecoveryScenario::Disconnect | RecoveryScenario::Liveness
        ) && fault_round.is_none()
            && server.phase() == ServerPhase::Playing
            && server.authority().tick().unwrap_or(0) >= 10
            && bots
                .last()
                .is_some_and(|bot| bot.client.phase() == BotPhase::Playing)
        {
            fault_round = public_round_id(&server);
            if scenario == RecoveryScenario::Disconnect {
                let bot = bots.last_mut().expect("bots");
                bot.writer
                    .close()
                    .map_err(|err| QaError::Net(err.to_string()))?;
                bot.dead = true;
                fault_disconnected = true;
            }
        }
        let fault_index = bots.len() - 1;
        for bot in &mut bots {
            if bot.dead {
                continue;
            }
            let mut pending: Vec<Vec<u8>> = Vec::new();
            loop {
                let received = if !bot.buffered.is_empty() {
                    Ok(bot.buffered.remove(0))
                } else if let Some(reader) = bot.io.reader.as_mut() {
                    match reader.try_recv_frame() {
                        Ok(Some(frame)) => Ok(frame),
                        Ok(None) if !reader.is_eof() => Err(TryRecvError::Empty),
                        _ => Err(TryRecvError::Disconnected),
                    }
                } else {
                    bot.frames.try_recv()
                };
                match received {
                    Ok(frame) => {
                        if scenario == RecoveryScenario::RoundResultLoss
                            && boom_proto::read_envelope(&frame)
                                .map(|(envelope, _)| {
                                    envelope.kind == boom_proto::MessageKind::RoundResult
                                })
                                .unwrap_or(false)
                        {
                            let result: boom_proto::RoundResult =
                                boom_proto::decode_frame(&frame, &boom_proto::ProtocolLimits::v1())
                                    .map_err(|err| QaError::Bot(err.to_string()))?;
                            if let Some(committed) = dropped_results.get(&bot.index) {
                                if result.round_id == committed.round_id && &result != committed {
                                    return Err(QaError::Bot(
                                        "Bảng recovery khác bảng đã commit".into(),
                                    ));
                                }
                            } else {
                                fault_round.get_or_insert(result.round_id);
                                dropped_results.insert(bot.index, result);
                                continue;
                            }
                        }
                        pending.extend(
                            bot.client
                                .on_frame(now, &frame)
                                .map_err(|err| QaError::Bot(format!("{}: {err}", bot.name)))?,
                        );
                    }
                    Err(TryRecvError::Empty) => {
                        break;
                    }
                    Err(TryRecvError::Disconnected) => {
                        // Server đóng phiên (EOF): bot sẽ được mở lại phiên nếu còn hạn.
                        if std::env::var_os("BOOM_QA_TRACE").is_some() {
                            eprintln!("[harness] t={now} {} bị đóng phiên", bot.name);
                        }
                        bot.dead = true;
                        if bot.index == fault_index && fault_round.is_some() {
                            fault_disconnected = true;
                        }
                        break;
                    }
                }
            }
            pending.extend(
                bot.client
                    .on_tick(now)
                    .map_err(|err| QaError::Bot(format!("{}: {err}", bot.name)))?,
            );
            if bot.dead {
                continue;
            }
            for frame in pending {
                let kind = boom_proto::read_envelope(&frame)
                    .map_err(|err| QaError::Net(err.to_string()))?
                    .0
                    .kind;
                if drain_started.is_some() && kind == boom_proto::MessageKind::RoomCommand {
                    continue;
                }
                if scenario == RecoveryScenario::Liveness
                    && fault_round.is_some()
                    && bot.index == fault_index
                    && bot.reconnects == 0
                    && kind == boom_proto::MessageKind::Pong
                {
                    continue;
                }
                bot.writer
                    .send_frame(&frame)
                    .map_err(|err| QaError::Net(err.to_string()))?;
                if virtual_time {
                    // Giữ thứ tự enqueue giữa các bot, không chỉ đợi đủ tổng số frame.
                    bot.io.up.wait(bot.writer.sent_frames())?;
                }
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
                        // `retired`/`resolved` cùng đơn vị sequence: chỉ đếm lệnh đã được
                        // server giải quyết và bot đã gửi (có trong `sent`). Retire không có
                        // trong `sent` (seq lỗ hổng do reconnect/round cũ) không vào báo cáo.
                        retired_count += 1;
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
        let recovery_done = scenario == RecoveryScenario::Steady
            || (scenario == RecoveryScenario::RoundResultLoss
                && dropped_results.len() == config.clients)
            || (fault_disconnected
                && bots
                    .last()
                    .is_some_and(|bot| bot.reconnects > 0 && bot.client.phase() >= BotPhase::Room)
                && rounds
                    .last()
                    .is_some_and(|round| Some(round.round_id) > fault_round)
                && rounds.last().is_some_and(|round| {
                    bots.iter().all(|bot| {
                        bot.client.stats().round_hashes.get(&round.round_id)
                            == Some(&round.server_hash)
                    })
                }));
        if done && recovery_done {
            let since = *drain_started.get_or_insert(now);
            if now.saturating_sub(since) >= drain_ms && server.phase() == ServerPhase::Waiting {
                break;
            }
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
        if virtual_time {
            simulated_ms += harness.step_ms.max(1);
        } else {
            thread::sleep(Duration::from_millis(harness.step_ms));
        }
    }

    // Dọn: đóng nửa ghi của bot để server và các luồng đọc kết thúc.
    for bot in &mut bots {
        let _ = bot.writer.close();
    }
    for _ in 0..50 {
        let _ = server.poll(
            if virtual_time {
                simulated_ms
            } else {
                start.elapsed().as_millis() as u64
            },
            Duration::ZERO,
        );
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
    let mut amplitudes = Samples::new();
    for bot in &bots {
        for value in bot.client.stats().correction_mpx.values() {
            amplitudes.push(*value);
        }
    }
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
        server_ping_interval_ms: harness.server_ping_interval_ms,
        server_liveness_timeout_ms: harness.server_liveness_timeout_ms,
        drain_ms,
        schema: crate::report::REPORT_SCHEMA,
        scenario: format!("{scenario:?}"),
        fault_round,
        fault_disconnected,
        drained: drain_started.is_some() && !timed_out,
        recovered: if scenario == RecoveryScenario::RoundResultLoss {
            dropped_results.len() == config.clients && converged
        } else {
            fault_round.is_some() && fault_disconnected && !timed_out
                && rounds.last().is_some_and(|round| round.converged)
        },
        profile: config.profile.name.clone(),
        rules_profile: rules_profile.profile_id.clone(),
        map: harness.map.clone(),
        seed: config.seed,
        clients: config.clients,
        rounds_target: config.rounds(),
        duration_ms: if virtual_time { simulated_ms } else { start.elapsed().as_millis() as u64 },
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
            amplitude_mpx: LatencyReport::from_samples(&amplitudes),
            note: "Predictor client chung; sai số Manhattan tại cùng tick, gồm mẫu 0; không phải độ trễ ms"
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
        // QA mạng không benchmark mật khẩu: hash fixture nhẹ để auth không chặn
        // server poll lâu hơn timer liveness thử nghiệm. Không đổi server production.
        argon2: Argon2Settings {
            m_cost: 64,
            t_cost: 1,
            p_cost: 1,
            ..Argon2Settings::default()
        },
        ..AuthConfig::default()
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    fn config() -> RunConfig {
        let mut profile = QaProfile::builtin("fast").unwrap();
        // Giữ lỗi mạng fast; kiểm liveness theo đồng hồ ảo, độc lập tải CI.
        profile.network.timing = TimingMode::Virtual;
        profile.harness.server_ping_interval_ms = 50;
        profile.harness.server_liveness_timeout_ms = 500;
        profile.harness.timeout_ms = 20_000;
        RunConfig {
            profile,
            clients: 4,
            seed: 7,
            rounds: Some(1),
        }
    }

    #[test]
    fn mat_round_result_bon_bot_xin_lai_va_hoi_tu() {
        let report = run_scenario(&config(), RecoveryScenario::RoundResultLoss).unwrap();
        assert!(
            !report.timed_out && report.converged && report.drained,
            "{report:?}"
        );
        assert_eq!(report.rounds.last().unwrap().bot_hashes.len(), 4);
        assert!(report.bots.iter().all(|bot| bot.results_acked >= 1));
    }

    #[test]
    fn bon_predictor_drain_va_hoi_tu() {
        let report = run_scenario(&config(), RecoveryScenario::Steady).unwrap();
        assert_eq!(
            report,
            run_scenario(&config(), RecoveryScenario::Steady).unwrap()
        );
        assert!(report.converged && report.drained, "{report:?}");
        assert_eq!(report.rounds.last().unwrap().bot_hashes.len(), 4);
        assert!(report.corrections.amplitude_mpx.count > 0, "{report:?}");
        assert!(report.corrections.amplitude_mpx.p95.is_some());
    }

    fn recovery(scenario: RecoveryScenario) {
        let report = run_scenario(&config(), scenario).unwrap();
        assert_eq!(report, run_scenario(&config(), scenario).unwrap());
        assert!(!report.timed_out, "{report:?}");
        assert!(
            report.fault_disconnected && report.recovered && report.drained,
            "{report:?}"
        );
        let last = report.rounds.last().unwrap();
        assert!(
            last.converged && Some(last.round_id) > report.fault_round,
            "{report:?}"
        );
        assert_eq!(last.bot_hashes.len(), 4);
        assert_eq!(report.bots.last().unwrap().reconnects, 1);
        assert!(
            report.bots[..3].iter().all(|bot| bot.reconnects == 0),
            "{report:?}"
        );
        assert!(
            report.bots.iter().all(|bot| bot.auth_failed == 0),
            "{report:?}"
        );
        let abandoned = report
            .rounds
            .iter()
            .find(|round| Some(round.round_id) == report.fault_round)
            .unwrap();
        assert!(!abandoned.converged);
        assert!(!abandoned.bot_hashes.iter().any(|(name, _)| name == "bot3"));
    }

    #[test]
    fn disconnect_giua_tran_auth_rejoin_va_hoi_tu_luot_sau() {
        recovery(RecoveryScenario::Disconnect);
    }

    #[test]
    fn mat_pong_het_liveness_auth_rejoin_va_hoi_tu_luot_sau() {
        recovery(RecoveryScenario::Liveness);
    }
}
