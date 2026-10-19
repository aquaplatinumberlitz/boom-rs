//! Lớp chạy thật của `boom-server serve` (V1-002 handshake, V1-029 vận hành, V1-036 nền).
//!
//! `serve` nối các mảnh đã có sẵn thành một tiến trình chạy được cho 2–4 client trong LAN:
//!
//! 1. **Nạp content** — rules profile + bundle map + bảng phân loại ô + chỉ số nhân vật.
//!    Thiếu/sai ⇒ lỗi rõ ràng, không panic (exit ≠ 0).
//! 2. **Mở DB** có khóa instance (ADR 0011 §4) và dựng `Server` với auth thật.
//! 3. **Bind game listener** + admin listener + heartbeat; `HealthState` báo ready khi
//!    đã bind và nạp content xong (ADR 0011 §8.2).
//! 4. **Vòng accept** trên luồng riêng: mỗi kết nối mới được cấp `PlayerId` mới.
//! 5. **Vòng tick** gọi `Server::poll` theo đồng hồ đơn điệu ở biên server (core không
//!    đọc đồng hồ wall-clock — ADR 0004).
//! 6. **Dừng sạch** khi Ctrl-C/SIGTERM: ngừng nhận kết nối, thu hồi phiên của instance
//!    này, ghi heartbeat cuối, thoát 0.
//!
//! Không có luật chơi nào nằm trong module này: mọi giá trị đọc từ rules profile, bundle
//! hoặc file cấu hình (AGENTS §3).

use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use boom_content::bundle::Bundle;
use boom_content::map::{CellIds, MapGrid, MapProfile, Terrain};
use boom_content::rules::RulesProfile;
use boom_core::{Rules, World, mode_by_id};
use boom_net::{TcpTransport, Transport};
use boom_room::PlayerId;

use crate::admin::{AdminServer, Heartbeat, probe_database, write_health_json};
use crate::auth::AuthService;
use crate::authority::{Authority, MatchSetup, ServerError};
use crate::config::{
    CharacterStatsTable, HandshakeConfig, OperationalConfig, ReplaySetup, ServerConfig,
};
use crate::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use crate::observability::{LogFields, Logger, new_instance_id, now_ms};
use crate::server::Server;

/// ID mode FFA của V1 (SCOPE_V1 §1).
const MODE_FFA: &str = "battle_ffa";

/// Chu kỳ thăm listener khi không có kết nối mới (ms).
const ACCEPT_POLL_MS: u64 = 10;

/// Epoch content của phiên server (bộ đếm, không phải luật chơi).
const CONTENT_EPOCH: u32 = 1;

/// Cờ dừng: handler tín hiệu (Ctrl-C/SIGTERM) đặt, vòng tick thấy thì dừng sạch.
#[derive(Clone, Debug, Default)]
pub struct StopFlag {
    flag: Arc<AtomicBool>,
}

impl StopFlag {
    /// Yêu cầu dừng (idempotent).
    pub fn request(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Đã yêu cầu dừng chưa.
    pub fn is_requested(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Lỗi khởi động: báo rõ cho người vận hành, không panic.
#[derive(Debug)]
pub enum StartupError {
    /// Không đọc được rules profile.
    Rules { path: String, message: String },
    /// Không đọc được bảng phân loại ô.
    MapProfile { path: String, message: String },
    /// Không nạp được bundle.
    Bundle { path: String, message: String },
    /// Thiếu/sai chỉ số nhân vật.
    Characters { message: String },
    /// Không mở được DB identity.
    Database { message: String },
    /// Không bind được game listener.
    Listen { addr: String, message: String },
    /// Không mở được admin listener.
    Admin { message: String },
    /// Không ghi được health artifact.
    Heartbeat { message: String },
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rules { path, message } => {
                write!(f, "không nạp được rules profile {path}: {message}")
            }
            Self::MapProfile { path, message } => {
                write!(f, "không nạp được bảng phân loại ô {path}: {message}")
            }
            Self::Bundle { path, message } => write!(f, "không nạp được bundle {path}: {message}"),
            Self::Characters { message } => write!(f, "chỉ số nhân vật không dùng được: {message}"),
            Self::Database { message } => write!(f, "không mở được DB identity: {message}"),
            Self::Listen { addr, message } => {
                write!(f, "không bind được game listener {addr}: {message}")
            }
            Self::Admin { message } => write!(f, "không mở được admin listener: {message}"),
            Self::Heartbeat { message } => write!(f, "không ghi được health.json: {message}"),
        }
    }
}

impl std::error::Error for StartupError {}

/// Lỗi lúc chạy (sau khi đã khởi động).
#[derive(Debug)]
pub enum ServeError {
    /// Lỗi khởi động (content/DB/listener).
    Startup(StartupError),
    /// Lỗi authority/server.
    Server(ServerError),
    /// Lỗi socket.
    Io(io::Error),
    /// Lỗi transport.
    Net(boom_net::NetError),
}

impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Startup(err) => write!(f, "{err}"),
            Self::Server(err) => write!(f, "lỗi server: {err}"),
            Self::Io(err) => write!(f, "lỗi socket: {err}"),
            Self::Net(err) => write!(f, "lỗi mạng: {err}"),
        }
    }
}

impl std::error::Error for ServeError {}

impl From<ServerError> for ServeError {
    fn from(value: ServerError) -> Self {
        Self::Server(value)
    }
}

impl From<io::Error> for ServeError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<boom_net::NetError> for ServeError {
    fn from(value: boom_net::NetError) -> Self {
        Self::Net(value)
    }
}

/// Nội dung đã nạp: rules + map + chỉ số nhân vật.
#[derive(Clone, Debug)]
pub struct LoadedContent {
    /// Rules đã đổi sang đơn vị mô phỏng.
    pub rules: Rules,
    /// `profile_id` của rules profile.
    pub rules_profile_id: String,
    /// Hash định danh rules profile (`fnv1a64-v0`).
    pub rules_hash: u64,
    /// SHA-256 của bundle (content).
    pub content_sha256: [u8; 32],
    /// Tên map đã nạp.
    pub map: String,
    /// Địa hình đã phân loại.
    pub terrain: Terrain,
    /// Chỉ số C00/C01.
    pub stats: CharacterStatsTable,
}

/// Nạp content cho `serve`: rules profile, bundle map, bảng phân loại ô, chỉ số nhân vật.
///
/// Lỗi luôn kèm đường dẫn để người vận hành biết cần sửa gì; không có `unwrap` trên dữ liệu
/// ngoài ⇒ không panic.
pub fn load_content(config: &OperationalConfig) -> Result<LoadedContent, StartupError> {
    // 1. Rules profile.
    let rules_path = config.rules_path.clone();
    let rules_text = std::fs::read_to_string(&rules_path).map_err(|err| StartupError::Rules {
        path: rules_path.display().to_string(),
        message: err.to_string(),
    })?;
    let profile = RulesProfile::from_toml(&rules_text).map_err(|err| StartupError::Rules {
        path: rules_path.display().to_string(),
        message: err.to_string(),
    })?;
    let rules = Rules::from_profile(&profile).map_err(|err| StartupError::Rules {
        path: rules_path.display().to_string(),
        message: err.to_string(),
    })?;

    // 2. Bảng phân loại ô theo build (một nguồn chuẩn: `data/maps/*.toml`).
    let map_path = config.map_profile_path.clone();
    let map_text = std::fs::read_to_string(&map_path).map_err(|err| StartupError::MapProfile {
        path: map_path.display().to_string(),
        message: err.to_string(),
    })?;
    let map_profile = MapProfile::from_toml(&map_text).map_err(|err| StartupError::MapProfile {
        path: map_path.display().to_string(),
        message: err.to_string(),
    })?;

    // 3. Bundle map.
    let bundle = load_bundle(&config.bundle_dir)?;
    if bundle.map != map_profile.map || bundle.build != map_profile.build {
        return Err(StartupError::MapProfile {
            path: map_path.display().to_string(),
            message: format!(
                "bảng phân loại ô là {}/{} nhưng bundle là {}/{}",
                map_profile.map, map_profile.build, bundle.map, bundle.build
            ),
        });
    }
    let terrain =
        map_profile
            .classify(&map_grid(&bundle))
            .map_err(|err| StartupError::MapProfile {
                path: map_path.display().to_string(),
                message: err.to_string(),
            })?;

    // 4. Chỉ số nhân vật (xem `CharacterConfig`): `serve` không tự đặt giá trị luật.
    let characters = config.characters.ok_or_else(|| StartupError::Characters {
        message: "thiếu mục `[characters]` trong file cấu hình (C00/C01)".to_owned(),
    })?;
    let stats = characters
        .stats_table()
        .map_err(|err| StartupError::Characters {
            message: err.to_string(),
        })?;

    let content_sha256 =
        parse_sha256_hex(&bundle.content_hash).ok_or_else(|| StartupError::Bundle {
            path: config.bundle_dir.display().to_string(),
            message: format!(
                "content_hash không phải SHA-256 hex: {}",
                bundle.content_hash
            ),
        })?;

    Ok(LoadedContent {
        rules_hash: profile.identity_hash(),
        rules_profile_id: profile.profile_id,
        content_sha256,
        map: bundle.map,
        terrain,
        stats,
        rules,
    })
}

/// Nạp bundle từ thư mục (kiểm `content_hash` và SHA-256 từng file atlas).
fn load_bundle(dir: &Path) -> Result<Bundle, StartupError> {
    boom_content::bundle::load(dir).map_err(|err| StartupError::Bundle {
        path: dir.display().to_string(),
        message: err.to_string(),
    })
}

/// Lưới ô theo build từ bundle: ID tile/obj giữ nguyên như file nguồn.
fn map_grid(bundle: &Bundle) -> MapGrid {
    let width = usize::from(bundle.width);
    let height = usize::from(bundle.height);
    let mut cells = vec![
        CellIds {
            tile: None,
            obj: None,
        };
        width * height
    ];
    for cell in &bundle.cells {
        let index = usize::from(cell.y) * width + usize::from(cell.x);
        if let Some(slot) = cells.get_mut(index) {
            *slot = CellIds {
                tile: cell.tile,
                obj: cell.object,
            };
        }
    }
    MapGrid {
        width: bundle.width,
        height: bundle.height,
        cells,
        spawns: bundle.spawns.clone(),
    }
}

/// Hex SHA-256 (64 ký tự) → 32 byte.
fn parse_sha256_hex(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (index, slot) in out.iter_mut().enumerate() {
        let text = value.get(index * 2..index * 2 + 2)?;
        *slot = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

/// Kết nối game đã được chấp nhận, chờ vòng tick nhận vào `Server`.
struct Accepted {
    player: PlayerId,
    slot: u8,
    stream: TcpStream,
    peer: SocketAddr,
}

/// Server đang chạy: content + DB + listener + vòng tick.
pub struct ServeRuntime {
    config: OperationalConfig,
    server: Server,
    local_addr: SocketAddr,
    accept_rx: Receiver<Accepted>,
    accept_handle: Option<JoinHandle<()>>,
    stop: StopFlag,
    state: Arc<crate::admin::HealthState>,
    logger: Arc<Logger>,
    admin: AdminServer,
    heartbeat: Heartbeat,
    /// Mốc epoch + đồng hồ đơn điệu: `now_ms` = epoch lúc boot + thời gian đã trôi.
    epoch_base_ms: u64,
    origin: Instant,
    /// Chu kỳ một vòng tick (1/tick_rate của rules profile).
    tick_period: Duration,
}

impl ServeRuntime {
    /// Khởi động server: nạp content, mở DB, bind listener, bật admin + heartbeat.
    pub fn start(config: OperationalConfig, stop: StopFlag) -> Result<Self, StartupError> {
        let content = load_content(&config)?;
        let instance_id = new_instance_id();

        // Cấu hình hiệu lực sau khi nạp content: fingerprint/health phải khớp content thật.
        let mut config = config;
        config.rules_profile_id = content.rules_profile_id.clone();
        config.rules_hash = content.rules_hash;
        config.content_sha256 = content.content_sha256;
        config.content_epoch = CONTENT_EPOCH;

        // DB identity: giữ khóa instance ⇒ chỉ một server chạy trên một file DB (ADR 0011 §4).
        let store =
            SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).map_err(|err| {
                StartupError::Database {
                    message: err.to_string(),
                }
            })?;
        let auth = AuthService::new(
            config.auth_config(),
            instance_id.clone(),
            Box::new(OsRandomSource),
        )
        .map_err(|err| StartupError::Database {
            message: err.to_string(),
        })?;

        // World: địa hình từ bundle + rules từ profile + mode FFA.
        let Some(mode) = mode_by_id(MODE_FFA) else {
            return Err(StartupError::Characters {
                message: format!("không có mode {MODE_FFA}"),
            });
        };
        let world =
            World::new(content.terrain.clone(), content.rules.clone(), mode).map_err(|err| {
                StartupError::Characters {
                    message: format!("không dựng được thế giới: {err}"),
                }
            })?;

        let replay = config.replay_dir.as_ref().map(|_| ReplaySetup {
            rules_path: config.rules_path.clone(),
            profile_id: content.rules_profile_id.clone(),
            checkpoints: config.replay_checkpoints.clone(),
        });
        let setup = MatchSetup {
            world,
            stats: content.stats,
            map: content.map.clone(),
            replay,
        };
        let mut server_config = ServerConfig::v1();
        server_config.replay_dir = config.replay_dir.clone();

        let handshake = HandshakeConfig {
            rules_profile_id: content.rules_profile_id.clone(),
            rules_hash: content.rules_hash,
            content_sha256: content.content_sha256,
            content_epoch: CONTENT_EPOCH,
            snapshot_interval_ms: snapshot_interval_ms(&content.rules),
        };
        let epoch_base_ms = now_ms();
        // Logger dùng chung cho admin, heartbeat và authority (lỗi ghi replay đi qua log).
        let logger = Arc::new(Logger::stdout(
            config.fingerprint(),
            instance_id.clone(),
            config.content_epoch,
        ));
        let mut server = Server::with_auth(
            Authority::new(server_config, setup).with_logger(logger.clone()),
            handshake,
            auth,
            Box::new(store),
            epoch_base_ms,
        )
        .map_err(|err| StartupError::Database {
            message: err.to_string(),
        })?;
        server.set_auth_deadline_ms(config.auth_deadline_ms);

        // Game listener: một kết nối TCP mỗi client (ADR 0006).
        let listen_addr =
            config
                .listen_addrs
                .first()
                .copied()
                .ok_or_else(|| StartupError::Listen {
                    addr: "<rỗng>".to_owned(),
                    message: "cấu hình không có `listen_addrs`".to_owned(),
                })?;
        let listener = TcpListener::bind(listen_addr).map_err(|err| StartupError::Listen {
            addr: listen_addr.to_string(),
            message: err.to_string(),
        })?;
        listener
            .set_nonblocking(true)
            .map_err(|err| StartupError::Listen {
                addr: listen_addr.to_string(),
                message: err.to_string(),
            })?;
        let local_addr = listener.local_addr().map_err(|err| StartupError::Listen {
            addr: listen_addr.to_string(),
            message: err.to_string(),
        })?;
        config.listen_addrs = vec![local_addr];

        // Admin + heartbeat: health runtime có nguồn sự thật là tiến trình (ADR 0011 §8.2).
        let state = Arc::new(crate::admin::HealthState::new(
            config.clone(),
            instance_id.clone(),
            epoch_base_ms,
        ));
        // Sẵn sàng trước khi mở admin: log `server.start` phải có địa chỉ game thật.
        state.set_db(probe_database(&config));
        state.mark_ready(vec![local_addr.to_string()]);
        let admin = AdminServer::start(&config, state.clone(), logger.clone()).map_err(|err| {
            StartupError::Admin {
                message: err.to_string(),
            }
        })?;
        let heartbeat =
            Heartbeat::start(state.clone(), logger.clone(), config.heartbeat_interval_ms).map_err(
                |err| StartupError::Heartbeat {
                    message: err.to_string(),
                },
            )?;
        // Ghi artifact ngay: `boom-server health` đọc `health.json` để tìm admin listener.
        if let Err(err) = write_health_json(&state, now_ms()) {
            return Err(StartupError::Heartbeat {
                message: err.to_string(),
            });
        }

        // Vòng accept trên luồng riêng; mỗi kết nối mới được cấp `PlayerId` mới.
        let (accept_tx, accept_rx) = mpsc::channel();
        let accept_handle = spawn_accept_loop(
            listener.try_clone().map_err(|err| StartupError::Listen {
                addr: local_addr.to_string(),
                message: err.to_string(),
            })?,
            accept_tx,
            stop.clone(),
        );

        let tick_period = tick_period(&content.rules);
        Ok(Self {
            config,
            server,
            local_addr,
            accept_rx,
            accept_handle: Some(accept_handle),
            stop,
            state,
            logger,
            admin,
            heartbeat,
            epoch_base_ms,
            origin: Instant::now(),
            tick_period,
        })
    }

    /// Địa chỉ game listener thật (cổng 0 ⇒ cổng thật sau khi bind).
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Địa chỉ admin listener thật (loopback).
    pub fn admin_addr(&self) -> SocketAddr {
        self.admin.local_addr()
    }

    /// Heartbeat đang chạy (ghi `health.json` định kỳ).
    pub fn heartbeat(&self) -> &Heartbeat {
        &self.heartbeat
    }

    /// Cấu hình hiệu lực sau khi nạp content.
    pub fn config(&self) -> &OperationalConfig {
        &self.config
    }

    /// Đồng hồ server: epoch lúc boot + thời gian đã trôi (đơn điệu).
    fn now_ms(&self) -> u64 {
        self.epoch_base_ms
            .saturating_add(u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    /// Chạy vòng tick cho tới khi có yêu cầu dừng; sau đó dừng sạch (xem [`Drop`]).
    pub fn run(mut self) -> Result<(), ServeError> {
        while !self.stop.is_requested() {
            self.accept_pending()?;
            let now = self.now_ms();
            self.server.poll(now, self.tick_period)?;
            self.state
                .set_active_sessions(self.server.session_count() as u64);
        }
        Ok(())
    }

    /// Nhận các kết nối đã được chấp nhận vào `Server` (mỗi kết nối một `PlayerId`).
    fn accept_pending(&mut self) -> Result<(), ServeError> {
        while let Ok(accepted) = self.accept_rx.try_recv() {
            let Accepted {
                player,
                slot,
                stream,
                peer,
            } = accepted;
            let writer_stream = stream.try_clone()?;
            let shutdown_stream = writer_stream.try_clone()?;
            let reader = TcpTransport::from_stream(stream)?;
            let writer = TcpTransport::from_stream(writer_stream)?;
            let mut shutdown = TcpTransport::from_stream(shutdown_stream)?;
            self.server.accept_with_shutdown(
                player,
                slot,
                peer.ip(),
                reader,
                writer,
                move || {
                    let _ = shutdown.close();
                },
            )?;
        }
        Ok(())
    }

    /// Dừng sạch: ngừng nhận kết nối, thu hồi phiên của instance, ghi heartbeat cuối.
    ///
    /// Việc dọn dẹp nằm trong [`Drop`] nên chạy dù vòng tick dừng vì lỗi hay vì tín hiệu.
    fn shutdown(&mut self) {
        self.stop.request();
        if let Some(handle) = self.accept_handle.take() {
            let _ = handle.join();
        }
        // Thu hồi phiên của instance này (ADR 0011 §6.1): người chơi đăng nhập lại được.
        self.server.shutdown_sessions(self.now_ms());
        if let Err(err) = write_health_json(&self.state, now_ms()) {
            let mut fields = LogFields::new();
            fields.insert("reason", serde_json::json!("health_json_write_failed"));
            fields.insert("error_message", serde_json::json!(err.to_string()));
            self.logger.warn("server.stop", fields);
        }
    }
}

impl Drop for ServeRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Chu kỳ một vòng tick: 1/tick_rate của rules profile (không hard-code).
fn tick_period(rules: &Rules) -> Duration {
    let hz = rules.ticks_per_second.max(1);
    Duration::from_millis((1_000 / u64::from(hz)).max(1))
}

/// Chu kỳ gửi snapshot: cùng nhịp với tick của rules profile.
fn snapshot_interval_ms(rules: &Rules) -> u32 {
    let hz = rules.ticks_per_second.max(1);
    u32::try_from(1_000 / u64::from(hz)).unwrap_or(1).max(1)
}

/// Vòng accept: chấp nhận kết nối trên luồng riêng, cấp `PlayerId` mới cho mỗi kết nối.
fn spawn_accept_loop(
    listener: TcpListener,
    tx: Sender<Accepted>,
    stop: StopFlag,
) -> JoinHandle<()> {
    let next_player = Arc::new(AtomicU64::new(1));
    let next_slot = Arc::new(AtomicU64::new(0));
    thread::spawn(move || {
        while !stop.is_requested() {
            match listener.accept() {
                Ok((stream, peer)) => {
                    let id = next_player.fetch_add(1, Ordering::SeqCst);
                    let slot =
                        u8::try_from(next_slot.fetch_add(1, Ordering::SeqCst) % 256).unwrap_or(0);
                    let accepted = Accepted {
                        player: PlayerId(id),
                        slot,
                        stream,
                        peer,
                    };
                    if tx.send(accepted).is_err() {
                        break;
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
            }
        }
    })
}

/// Chạy `serve` tới khi có tín hiệu dừng; trả `Ok(())` khi dừng sạch.
pub fn serve(config: OperationalConfig, stop: StopFlag) -> Result<(), ServeError> {
    let runtime = ServeRuntime::start(config, stop).map_err(ServeError::Startup)?;
    runtime.run()
}

#[cfg(test)]
mod tests {
    //! Test tích hợp của `serve`: server thật, client TCP thật, DB thật.
    //!
    //! Fixture tự dựng: bundle nhỏ (5×5, mọi ô là nền) + bảng phân loại ô + rules profile
    //! `fixtures/rules/skeleton-test.toml` + DB SQLite trong thư mục tạm.

    use super::*;

    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use boom_content::bundle::Cell;
    use boom_proto::dto::auth::{Auth, AuthOk, AuthOp};
    use boom_proto::{
        Hello, LoadingAck, Message, MessageKind, RoomAction as RoomActionDto, RoomCharacter,
        RoomCommand as RoomCommandDto, RoomState, RoundStart, Snapshot, Welcome, decode_frame,
        encode_frame, limits::ProtocolLimits, read_envelope,
    };

    use crate::admin::run_health_cli;
    use crate::identity::{AccountId, SessionStore};

    /// Bảng phân loại ô cho map test: mọi ô trống là nền đi được.
    const TEST_MAP_PROFILE: &str = r#"schema = 1
profile_id = "test-map"
map = "TestMap"
build = "kr"
width = 5
height = 5
source = "design"
status = "design"
empty = "floor"

[classes]

[objects]
"#;

    /// Dừng server khi luồng test kết thúc (kể cả khi panic) ⇒ vòng tick không treo.
    struct StopOnDrop(StopFlag);

    impl Drop for StopOnDrop {
        fn drop(&mut self) {
            self.0.request();
        }
    }

    /// Một client TCP thật nói chuyện với server qua DTO của `boom-proto`.
    struct Client {
        transport: TcpTransport,
    }

    impl Client {
        fn connect(addr: SocketAddr) -> Result<Self, String> {
            let stream = TcpStream::connect(addr).map_err(|err| err.to_string())?;
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|err| err.to_string())?;
            let transport = TcpTransport::from_stream(stream).map_err(|err| err.to_string())?;
            Ok(Self { transport })
        }

        fn send<M: Message>(&mut self, message: &M) -> Result<(), String> {
            let frame =
                encode_frame(message, &ProtocolLimits::v1()).map_err(|err| err.to_string())?;
            self.transport
                .send_frame(&frame)
                .map_err(|err| err.to_string())
        }

        /// Nhận message đầu tiên có `kind` (bỏ qua message khác).
        fn recv_of<M: Message>(&mut self, kind: MessageKind) -> Result<M, String> {
            loop {
                let frame = self
                    .transport
                    .recv_frame()
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| "server đóng kết nối".to_owned())?;
                let (envelope, _) = read_envelope(&frame).map_err(|err| err.to_string())?;
                if envelope.kind == kind {
                    return decode_frame(&frame, &ProtocolLimits::v1())
                        .map_err(|err| err.to_string());
                }
            }
        }
    }

    /// Đường dẫn trong file TOML: escape dấu `\` (Windows).
    fn toml_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "\\\\")
    }

    /// Ghi bundle nhỏ tự dựng (5×5, mọi ô nền, 4 spawn).
    fn write_test_bundle(dir: &Path) {
        let mut bundle = Bundle {
            schema: boom_content::bundle::SCHEMA,
            build: "kr".to_owned(),
            map: "TestMap".to_owned(),
            tileset: "Tile00".to_owned(),
            width: 5,
            height: 5,
            spawns: vec![(1, 1), (3, 1), (1, 3), (3, 3)],
            content_hash: String::new(),
            atlas: Vec::new(),
            sprites: Vec::new(),
            assets: Vec::new(),
            cells: Vec::new(),
            unknown: Vec::new(),
            clips: Vec::new(),
            ui: Vec::new(),
            audio: Vec::new(),
        };
        for y in 0..bundle.height {
            for x in 0..bundle.width {
                bundle.cells.push(Cell {
                    x,
                    y,
                    tile: None,
                    object: None,
                });
            }
        }
        bundle.content_hash = bundle.compute_content_hash();
        std::fs::write(
            dir.join(boom_content::bundle::METADATA_FILE),
            bundle.to_toml().expect("bundle hợp lệ"),
        )
        .expect("ghi bundle");
    }

    /// Dựng môi trường test: bundle + bảng phân loại ô + file cấu hình.
    /// Trả về đường dẫn file cấu hình.
    fn write_test_env(root: &Path) -> PathBuf {
        let bundle_dir = root.join("bundle");
        std::fs::create_dir_all(&bundle_dir).expect("tạo thư mục bundle");
        write_test_bundle(&bundle_dir);

        let map_path = root.join("map.toml");
        std::fs::write(&map_path, TEST_MAP_PROFILE).expect("ghi bảng phân loại ô");

        let rules =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rules/skeleton-test.toml");
        let rules = rules.canonicalize().expect("đường dẫn rules profile");
        let data_dir = root.join("data");
        let config_path = root.join("server.toml");
        let text = format!(
            "[server]\n\
             admin_addr = \"127.0.0.1:0\"\n\
             data_dir = \"{data_dir}\"\n\
             listen_addrs = [\"127.0.0.1:0\"]\n\
             heartbeat_interval_ms = 50\n\
             health_timeout_ms = 2000\n\
             bundle_dir = \"{bundle_dir}\"\n\
             rules_path = \"{rules}\"\n\
             map_profile_path = \"{map_path}\"\n\
             \n\
             [identity]\n\
             db_path = \"{db_path}\"\n\
             auth_deadline_ms = 30000\n\
             \n\
             [auth]\n\
             registration_open = true\n\
             \n\
             [characters.c00]\n\
             speed_value = 5\n\
             speed_max = 7\n\
             bombs_value = 1\n\
             bombs_max = 10\n\
             range_value = 1\n\
             range_max = 7\n\
             \n\
             [characters.c01]\n\
             speed_value = 4\n\
             speed_max = 8\n\
             bombs_value = 2\n\
             bombs_max = 7\n\
             range_value = 1\n\
             range_max = 9\n",
            data_dir = toml_path(&data_dir),
            bundle_dir = toml_path(&bundle_dir),
            rules = toml_path(&rules),
            map_path = toml_path(&map_path),
            db_path = toml_path(&data_dir.join("boom-server.sqlite3")),
        );
        std::fs::write(&config_path, text).expect("ghi file cấu hình");
        config_path
    }

    /// Chờ `RoomState` có đúng số thành viên.
    fn recv_room_state(client: &mut Client, members: usize) -> Result<RoomState, String> {
        loop {
            let state: RoomState = client.recv_of(MessageKind::RoomState)?;
            if state.members.len() == members {
                return Ok(state);
            }
        }
    }

    /// Kết quả luồng client: ID tài khoản đã đăng ký.
    struct ClientOutcome {
        account_id: i64,
    }

    /// Luồng client đầy đủ: handshake → đăng ký → tạo/join phòng → ready → start → snapshot.
    fn drive_two_clients(
        addr: SocketAddr,
        config: &OperationalConfig,
    ) -> Result<ClientOutcome, String> {
        let mut host = Client::connect(addr)?;
        let mut guest = Client::connect(addr)?;
        for client in [&mut host, &mut guest] {
            client.send(&Hello {
                wire_schema: 1,
                state_schema: 1,
                rules_profile_id: config.rules_profile_id.clone(),
                rules_hash: config.rules_hash,
                content_sha256: config.content_sha256,
                client_build: "test".to_owned(),
                capabilities: Vec::new(),
            })?;
            let welcome: Welcome = client.recv_of(MessageKind::Welcome)?;
            if welcome.rules_hash != config.rules_hash {
                return Err("Welcome mang rules hash khác server".to_owned());
            }
        }

        let mut account_id = 0i64;
        for (index, client) in [&mut host, &mut guest].into_iter().enumerate() {
            client.send(&Auth {
                op: AuthOp::Register,
                username: format!("player{index}"),
                password: "password123".to_owned(),
            })?;
            let ok: AuthOk = client.recv_of(MessageKind::AuthOk)?;
            if index == 0 {
                account_id = i64::try_from(ok.account_id).unwrap_or_default();
            }
        }

        host.send(&RoomCommandDto {
            request: 1,
            action: RoomActionDto::Create {
                character: RoomCharacter::C00,
            },
        })?;
        let state = recv_room_state(&mut host, 1)?;
        let room = state.room_id;

        guest.send(&RoomCommandDto {
            request: 1,
            action: RoomActionDto::Join {
                room,
                character: RoomCharacter::C01,
            },
        })?;
        recv_room_state(&mut guest, 2)?;
        recv_room_state(&mut host, 2)?;

        for (index, client) in [&mut host, &mut guest].into_iter().enumerate() {
            client.send(&RoomCommandDto {
                request: 2,
                action: RoomActionDto::SetReady { ready: true },
            })?;
            let _ = index;
        }
        recv_room_state(&mut host, 2)?;
        recv_room_state(&mut guest, 2)?;

        host.send(&RoomCommandDto {
            request: 3,
            action: RoomActionDto::Start,
        })?;
        let start: RoundStart = host.recv_of(MessageKind::RoundStart)?;
        let round = start.round_id;
        if start.roster.len() != 2 {
            return Err("roster phải có 2 người".to_owned());
        }

        for client in [&mut host, &mut guest] {
            client.send(&LoadingAck { round_id: round })?;
        }
        let _: Snapshot = host.recv_of(MessageKind::Snapshot)?;
        let _: Snapshot = guest.recv_of(MessageKind::Snapshot)?;

        Ok(ClientOutcome { account_id })
    }

    #[test]
    fn serve_runs_two_tcp_clients_through_a_round_and_revokes_sessions_on_stop() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        let runtime = ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
        let addr = runtime.local_addr();
        let effective = runtime.config().clone();

        // Client chạy ở luồng riêng; vòng tick chạy ở luồng chính của test.
        let client_stop = stop.clone();
        let client_config = effective.clone();
        let handle = thread::spawn(move || {
            let _guard = StopOnDrop(client_stop);
            drive_two_clients(addr, &client_config)
        });

        runtime.run().expect("vòng tick dừng sạch");
        let outcome = handle
            .join()
            .expect("luồng client không panic")
            .expect("client đăng ký → phòng → start → snapshot");

        // Health runtime trả 0 khi server đang chạy (đã kiểm trong luồng client).
        // Dừng sạch ⇒ DB không còn phiên hiệu lực của instance này.
        let store =
            SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).expect("mở DB");
        let active = store
            .active_session(AccountId(outcome.account_id))
            .expect("đọc phiên");
        assert!(active.is_none(), "phiên phải được thu hồi khi dừng");
    }

    #[test]
    fn serve_health_cli_reports_zero_while_running() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        let runtime = ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
        let effective = runtime.config().clone();

        let health_stop = stop.clone();
        let health_config = effective.clone();
        let handle = thread::spawn(move || {
            let _guard = StopOnDrop(health_stop);
            // Đợi server sẵn sàng rồi hỏi health.
            for _ in 0..100 {
                let outcome = run_health_cli(&health_config, false);
                if outcome.code == 0 {
                    return Ok(outcome);
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err("health không bao giờ healthy".to_owned())
        });

        runtime.run().expect("vòng tick dừng sạch");
        let outcome = handle
            .join()
            .expect("luồng health không panic")
            .expect("health healthy");
        assert_eq!(outcome.body["status"], "healthy");
        assert_eq!(outcome.body["scope"], "runtime");
    }

    #[cfg(unix)]
    #[test]
    fn serve_binary_exits_zero_on_sigterm_and_leaves_no_active_session() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");
        let Some(binary) = server_binary() else {
            eprintln!(
                "bỏ qua kiểm tra tín hiệu: không tìm thấy binary boom-server \
                 (chạy `cargo test --workspace` để build bin)"
            );
            return;
        };

        let mut child = Command::new(&binary)
            .arg("serve")
            .arg("--config")
            .arg(&config_path)
            .current_dir(dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("chạy boom-server serve");

        let addr = wait_for_listen_addr(&config.data_dir.join("health.json"));
        // `boom-server health` phải trả 0 khi server đang chạy (đọc artifact của tiến trình con).
        let health = run_health_cli(&config, false);
        assert_eq!(health.code, 0, "health runtime: {:?}", health.body);
        let content = load_content(&config).expect("nạp content");
        let mut client = Client::connect(addr).expect("kết nối server");
        client
            .send(&Hello {
                wire_schema: 1,
                state_schema: 1,
                rules_profile_id: content.rules_profile_id.clone(),
                rules_hash: content.rules_hash,
                content_sha256: content.content_sha256,
                client_build: "test".to_owned(),
                capabilities: Vec::new(),
            })
            .expect("gửi Hello");
        // Server đã nạp content nên fingerprint khớp; `Welcome` là bằng chứng server sống.
        let _: Welcome = client.recv_of(MessageKind::Welcome).expect("nhận Welcome");
        client
            .send(&Auth {
                op: AuthOp::Register,
                username: "player0".to_owned(),
                password: "password123".to_owned(),
            })
            .expect("gửi Auth");
        let ok: AuthOk = client.recv_of(MessageKind::AuthOk).expect("nhận AuthOk");
        let account_id = i64::try_from(ok.account_id).unwrap_or_default();

        // Gửi SIGTERM: server phải dừng sạch và thoát 0.
        let killed = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .expect("gửi SIGTERM");
        assert!(killed.success(), "không gửi được SIGTERM");
        let status = child.wait().expect("chờ server thoát");
        assert!(status.success(), "server phải thoát 0, thực tế: {status:?}");

        let store =
            SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).expect("mở DB");
        let active = store
            .active_session(AccountId(account_id))
            .expect("đọc phiên");
        assert!(active.is_none(), "không còn phiên hiệu lực của instance");
    }

    /// Đường dẫn binary `boom-server` (test chạy trong `target/<profile>/deps`).
    fn server_binary() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_boom-server") {
            return Some(PathBuf::from(path));
        }
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?.parent()?;
        let name = if cfg!(windows) {
            "boom-server.exe"
        } else {
            "boom-server"
        };
        let path = dir.join(name);
        path.is_file().then_some(path)
    }

    /// Chờ `health.json` có địa chỉ game thật (cổng 0 ⇒ cổng thật sau khi bind).
    fn wait_for_listen_addr(artifact: &Path) -> SocketAddr {
        for _ in 0..300 {
            if let Ok(text) = std::fs::read_to_string(artifact)
                && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
                && let Some(list) = value.get("listen_addrs").and_then(|v| v.as_array())
                && let Some(first) = list.first()
                && let Some(addr) = first.as_str()
                && let Ok(addr) = addr.parse::<SocketAddr>()
            {
                return addr;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("server không ghi địa chỉ game vào health.json");
    }
}
