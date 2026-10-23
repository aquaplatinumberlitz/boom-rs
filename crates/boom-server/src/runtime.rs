//! Lớp chạy thật của `boom-server serve` (V1-002 handshake, V1-029 vận hành, V1-036 nền).
//!
//! `serve` nối các mảnh đã có sẵn thành một tiến trình chạy được cho 2–4 client trong LAN:
//!
//! 1. **Nạp content** — rules profile (+ bảng chỉ số nhân vật trong profile) + bundle map
//!    + bảng phân loại ô. Thiếu/sai ⇒ lỗi rõ ràng, không panic (exit ≠ 0).
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
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use boom_content::bundle::Bundle;
use boom_content::map::{CellIds, MapGrid, MapProfile, Terrain};
use boom_content::rules::RulesProfile;
use boom_core::{CharacterStatsTable, Rules, World, mode_by_id};
use boom_net::{TcpTransport, Transport};
use boom_room::PlayerId;

use crate::admin::{AdminServer, Heartbeat, probe_database, write_health_json};
use crate::auth::AuthService;
use crate::authority::{Authority, MatchSetup, ServerError};
use crate::config::{HandshakeConfig, OperationalConfig, ReplaySetup, ServerConfig};
use crate::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
use crate::observability::{LogFields, Logger, SharedLogSink, events, new_instance_id, now_ms};
use crate::server::Server;

/// ID mode FFA của V1 (SCOPE_V1 §1).
const MODE_FFA: &str = "battle_ffa";

/// Chu kỳ thăm listener khi không có kết nối mới (ms).
const ACCEPT_POLL_MS: u64 = 10;

/// Hạn chờ vòng accept kết thúc khi dừng server (ms).
///
/// Vượt hạn thì buông luồng accept thay vì chờ vô hạn: `main` vẫn thoát được, còn vòng
/// tick đã dừng nên không còn việc gì cần luồng đó làm.
const ACCEPT_JOIN_TIMEOUT: Duration = Duration::from_secs(2);

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
    /// Không dựng được thế giới (địa hình/mode).
    World { message: String },
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
            Self::World { message } => write!(f, "không dựng được thế giới: {message}"),
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
///
/// Chỉ số nhân vật đọc từ rules profile (`characters.c00_*` / `characters.c01_*`), không từ
/// file cấu hình: mọi người chạy server dùng cùng một nguồn và `rules_hash` phản ánh chỉ số
/// (AGENTS §3).
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

    // 4. Chỉ số nhân vật: đọc từ rules profile (`characters.c00_*` / `characters.c01_*`,
    //    R-040/R-041) — server không tự đặt giá trị luật, và `rules_hash` phản ánh chỉ số.
    let stats = CharacterStatsTable::from_profile(&profile).map_err(|err| StartupError::Rules {
        path: rules_path.display().to_string(),
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

/// Mã lỗi + thông báo cho log `content.load_fail` (ADR 0011 §8.1).
///
/// `error_message` mang **đường dẫn file** (không mang nội dung file — ADR 0011 §8.1); thông
/// báo chi tiết cho người vận hành vẫn do `main` in ra stderr.
fn content_load_failure(err: &StartupError) -> (&'static str, String) {
    match err {
        StartupError::Rules { path, .. } => ("rules_load_failed", path.clone()),
        StartupError::MapProfile { path, .. } => ("map_profile_load_failed", path.clone()),
        StartupError::Bundle { path, .. } => ("bundle_load_failed", path.clone()),
        StartupError::World { message } => ("world_build_failed", message.clone()),
        _ => (
            "content_load_failed",
            "không xác định được nguồn lỗi".to_owned(),
        ),
    }
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
    /// Định danh kết nối (khoá session trong `Server`).
    player: PlayerId,
    /// Định danh kết nối trong phiên (đưa vào `Welcome.player_id`).
    ///
    /// **Không phải** slot trận: slot lấy từ roster round theo `account_id`
    /// (ADR 0009 Addendum B §B.2).
    connection_id: u8,
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
        Self::start_with_log_sink(config, stop, Box::new(io::stdout()))
    }

    /// Như [`Self::start`] nhưng ghi log vào `sink` (test đọc JSON Lines từ buffer).
    ///
    /// `sink` được chia sẻ giữa logger bootstrap (dựng trước khi nạp content) và logger
    /// thật ⇒ mọi dòng log của tiến trình nằm trong cùng một chỗ.
    pub fn start_with_log_sink(
        config: OperationalConfig,
        stop: StopFlag,
        sink: Box<dyn io::Write + Send>,
    ) -> Result<Self, StartupError> {
        let instance_id = new_instance_id();
        let sink = SharedLogSink::new(sink);

        // Logger **trước** khi nạp content: lỗi nạp cũng phải đi qua log JSON
        // (`content.load_fail` + `error_code`/`error_message`, ADR 0011 §8.1) chứ không
        // chỉ in ra stderr. Fingerprint lúc này còn là giá trị tạm của file cấu hình ⇒
        // logger này chỉ dùng cho đường lỗi nạp content.
        let bootstrap = Logger::new(
            config.fingerprint(),
            instance_id.clone(),
            config.content_epoch,
            sink.writer(),
        );
        let content = match load_content(&config) {
            Ok(content) => content,
            Err(err) => {
                let (code, message) = content_load_failure(&err);
                let mut fields = LogFields::new();
                fields.insert("error_code", serde_json::json!(code));
                fields.insert("error_message", serde_json::json!(message));
                bootstrap.error(events::CONTENT_LOAD_FAIL, fields);
                return Err(err);
            }
        };

        // Cấu hình hiệu lực sau khi nạp content: fingerprint/health phải khớp content thật.
        let mut config = config;
        config.rules_profile_id = content.rules_profile_id.clone();
        config.rules_hash = content.rules_hash;
        config.content_sha256 = content.content_sha256;
        config.content_epoch = CONTENT_EPOCH;

        // Logger dùng chung cho auth, admin, heartbeat và authority.
        let logger = Arc::new(Logger::new(
            config.fingerprint(),
            instance_id.clone(),
            config.content_epoch,
            sink.writer(),
        ));
        // Content đã nạp xong: `content.load_ok` mang `content_sha256` thật (ADR 0011 §8.1).
        {
            let mut fields = LogFields::new();
            fields.insert("map", serde_json::json!(content.map.clone()));
            logger.info(events::CONTENT_LOAD_OK, fields);
        }

        // DB identity: giữ khóa instance ⇒ chỉ một server chạy trên một file DB (ADR 0011 §4).
        let store =
            SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).map_err(|err| {
                StartupError::Database {
                    message: err.to_string(),
                }
            })?;
        let mut auth = AuthService::new(
            config.auth_config(),
            instance_id.clone(),
            Box::new(OsRandomSource),
        )
        .map_err(|err| StartupError::Database {
            message: err.to_string(),
        })?;
        // Sự kiện `auth.*` của ADR 0011 §8.1 đi qua log của tiến trình.
        auth.set_logger(logger.clone());

        // World: địa hình từ bundle + rules từ profile + mode FFA.
        let Some(mode) = mode_by_id(MODE_FFA) else {
            return Err(StartupError::World {
                message: format!("không có mode {MODE_FFA}"),
            });
        };
        let world =
            World::new(content.terrain.clone(), content.rules.clone(), mode).map_err(|err| {
                StartupError::World {
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
        // Log JSON Lines của server (sự kiện input bị bỏ, v.v.).
        server.set_logger(logger.clone());

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
    pub fn run(&mut self) -> Result<(), ServeError> {
        while !self.stop.is_requested() {
            self.accept_pending()?;
            let now = self.now_ms();
            self.server.poll(now, self.tick_period)?;
            self.state
                .set_active_sessions(self.server.session_count() as u64);
        }
        Ok(())
    }

    /// Số batch input bị bỏ vì client khai slot khác slot server bind từ phiên.
    pub fn input_slot_mismatch_count(&self) -> u64 {
        self.server.input_slot_mismatch_count()
    }

    /// Nhận các kết nối đã được chấp nhận vào `Server` (mỗi kết nối một `PlayerId`).
    fn accept_pending(&mut self) -> Result<(), ServeError> {
        while let Ok(accepted) = self.accept_rx.try_recv() {
            let Accepted {
                player,
                connection_id,
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
                connection_id,
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
        // Vòng accept đang chờ trong `accept()` (listener blocking) ⇒ kết nối giả tới
        // chính listener để mở khóa nó, rồi mới join.
        self.wake_accept_loop();
        if let Some(handle) = self.accept_handle.take() {
            self.join_accept_loop(handle);
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

    /// Mở khóa vòng accept bằng kết nối giả tới chính listener.
    ///
    /// Thử **cùng họ địa chỉ** với listener trước: socket IPv6-only (môi trường Windows)
    /// không nhận kết nối IPv4, nên `127.0.0.1` sẽ hỏng và vòng accept treo vô hạn.
    /// Thử cả hai họ để không bỏ sót trường hợp wildcard hai họ.
    fn wake_accept_loop(&self) {
        for addr in self.wakeup_addrs() {
            if TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok() {
                return;
            }
        }
    }

    /// Chờ vòng accept kết thúc, có hạn.
    ///
    /// `join` không có hạn ⇒ nếu kết nối giả thất bại (ví dụ listener đã đóng) thì server
    /// treo. Ở đây chờ có giới hạn rồi **buông** luồng: tiến trình vẫn thoát được vì
    /// `main` kết thúc, còn tick đã dừng nên không còn việc gì cần luồng đó làm.
    fn join_accept_loop(&self, handle: JoinHandle<()>) {
        let (tx, rx) = mpsc::channel::<()>();
        thread::spawn(move || {
            let _ = handle.join();
            let _ = tx.send(());
        });
        if rx.recv_timeout(ACCEPT_JOIN_TIMEOUT).is_err() {
            let mut fields = LogFields::new();
            fields.insert("reason", serde_json::json!("accept_loop_join_timeout"));
            self.logger.warn("server.stop", fields);
        }
    }

    /// Địa chỉ loopback dùng để mở khóa vòng accept.
    ///
    /// Listener wildcard (`0.0.0.0`/`[::]`) không có IP cụ thể ⇒ dùng loopback **cùng họ**
    /// với địa chỉ đã bind. Nếu bind một địa chỉ cụ thể thì dùng chính nó.
    fn wakeup_addrs(&self) -> Vec<SocketAddr> {
        let port = self.local_addr.port();
        match self.local_addr.ip() {
            IpAddr::V4(ip) => {
                if ip.is_unspecified() {
                    vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)]
                } else {
                    vec![self.local_addr]
                }
            }
            IpAddr::V6(ip) => {
                if ip.is_unspecified() {
                    // Socket IPv6-only chỉ nhận `::1`; socket hai họ nhận cả hai.
                    vec![
                        SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port),
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
                    ]
                } else {
                    vec![self.local_addr]
                }
            }
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
///
/// Listener **blocking** (không non-blocking): socket trả về bởi `accept()` kế thừa cờ
/// non-blocking của listener trên BSD/macOS, và trên Linux một socket vừa chuyển từ
/// non-blocking về blocking vẫn có thể trả `EAGAIN` cho lần đọc đầu ⇒ luồng đọc của
/// session sẽ đóng kết nối ngay. Vòng accept vì vậy dừng bằng **kết nối giả** tới chính
/// listener (xem [`ServeRuntime::shutdown`]), không bằng cờ non-blocking.
///
/// **Số thứ tự kết nối TCP không phải slot trận**: slot trận lấy từ roster round theo
/// `account_id` (ADR 0009 Addendum B §B.2). Ở đây chỉ cấp định danh kết nối.
fn spawn_accept_loop(
    listener: TcpListener,
    tx: Sender<Accepted>,
    stop: StopFlag,
) -> JoinHandle<()> {
    let next_connection = Arc::new(AtomicU64::new(1));
    thread::spawn(move || {
        while !stop.is_requested() {
            match listener.accept() {
                Ok((stream, peer)) => {
                    if stop.is_requested() {
                        // Kết nối giả dùng để mở khóa accept khi dừng.
                        let _ = stream.shutdown(Shutdown::Both);
                        break;
                    }
                    let id = next_connection.fetch_add(1, Ordering::SeqCst);
                    let connection_id = u8::try_from(id % 256).unwrap_or(0);
                    let accepted = Accepted {
                        player: PlayerId(id),
                        connection_id,
                        stream,
                        peer,
                    };
                    if tx.send(accepted).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    // Lỗi accept (ví dụ hết fd) không được giết vòng accept.
                    thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
            }
        }
    })
}

/// Chạy `serve` tới khi có tín hiệu dừng; trả `Ok(())` khi dừng sạch.
pub fn serve(config: OperationalConfig, stop: StopFlag) -> Result<(), ServeError> {
    let mut runtime = ServeRuntime::start(config, stop).map_err(ServeError::Startup)?;
    runtime.run()
}

#[cfg(test)]
mod tests {
    //! Test tích hợp của `serve`: server thật, client TCP thật, DB thật.
    //!
    //! Fixture tự dựng: bundle nhỏ (5×5, mọi ô là nền) + bảng phân loại ô + rules profile
    //! `fixtures/rules/skeleton-test.toml` + DB SQLite trong thư mục tạm.

    use super::*;

    use std::io::Read as _;
    use std::path::{Path, PathBuf};
    #[cfg(unix)]
    use std::process::{Command, Stdio};

    use boom_content::bundle::Cell;
    use boom_proto::dto::auth::{Auth, AuthOk, AuthOp};
    use boom_proto::{
        Hello, InputBatch, InputBits, InputCommand, LoadingAck, Message, MessageKind, Ping, Pong,
        RoomAction as RoomActionDto, RoomCharacter, RoomCommand as RoomCommandDto, RoomState,
        RoundStart, Snapshot, Welcome, decode_frame, encode_frame, limits::ProtocolLimits,
        read_envelope,
    };

    use crate::admin::run_health_cli;
    use crate::identity::{AccountId, SessionStore};
    use crate::observability::SharedLogBuffer;

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

        /// Nhận message đầu tiên có `kind`; trả `Pong` cho `Ping` để giữ liveness.
        ///
        /// Server gửi `Ping` định kỳ và đóng phiên nếu không nhận `Pong`; fixture không được
        /// bỏ qua frame này chỉ vì test đang chờ `Snapshot`.
        fn recv_of<M: Message>(&mut self, kind: MessageKind) -> Result<M, String> {
            loop {
                let frame = self
                    .transport
                    .recv_frame()
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| "server đóng kết nối".to_owned())?;
                let (envelope, _) = read_envelope(&frame).map_err(|err| err.to_string())?;
                if envelope.kind == MessageKind::Ping {
                    let ping: Ping = decode_frame(&frame, &ProtocolLimits::v1())
                        .map_err(|err| err.to_string())?;
                    self.send(&Pong(ping.0))?;
                    continue;
                }
                if envelope.kind == kind {
                    return decode_frame(&frame, &ProtocolLimits::v1())
                        .map_err(|err| err.to_string());
                }
            }
        }
    }

    #[test]
    fn client_answers_server_ping_while_waiting_for_pong() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fixture");
        let addr = listener.local_addr().expect("địa chỉ fixture");
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture");
            let mut transport = TcpTransport::from_stream(stream).expect("transport fixture");
            let ping = encode_frame(&Ping(7), &ProtocolLimits::v1()).expect("encode Ping");
            transport.send_frame(&ping).expect("gửi Ping");
            let frame = transport
                .recv_frame()
                .expect("đọc Pong")
                .expect("client phải trả Pong");
            let (envelope, _) = read_envelope(&frame).expect("envelope Pong");
            assert_eq!(envelope.kind, MessageKind::Pong);
            let pong: Pong = decode_frame(&frame, &ProtocolLimits::v1()).expect("decode Pong");
            assert_eq!(pong.0, 7);
            let reply = encode_frame(&Pong(7), &ProtocolLimits::v1()).expect("encode Pong");
            transport.send_frame(&reply).expect("gửi Pong");
        });

        let mut client = Client::connect(addr).expect("client fixture");
        let pong: Pong = client.recv_of(MessageKind::Pong).expect("nhận Pong");
        assert_eq!(pong.0, 7);
        server.join().expect("thread fixture không panic");
    }

    /// Client TCP thật với bộ đệm tự quản.
    ///
    /// Khác [`Client`] (dùng `TcpTransport`), client này giữ byte chưa đọc trong bộ đệm và
    /// **không** đóng socket khi hết thời gian chờ: cần quan sát "server im lặng" để kiểm
    /// server không gửi `Snapshot` khi phòng còn ở loading barrier (ADR 0009 §4.3).
    struct RawClient {
        stream: TcpStream,
        buf: Vec<u8>,
    }

    impl RawClient {
        fn connect(addr: SocketAddr) -> Result<Self, String> {
            let stream = TcpStream::connect(addr).map_err(|err| err.to_string())?;
            stream.set_nodelay(true).map_err(|err| err.to_string())?;
            Ok(Self {
                stream,
                buf: Vec::new(),
            })
        }

        fn send<M: Message>(&mut self, message: &M) -> Result<(), String> {
            let frame =
                encode_frame(message, &ProtocolLimits::v1()).map_err(|err| err.to_string())?;
            boom_net::write_frame(&mut self.stream, &frame, boom_net::FrameLimits::V1)
                .map_err(|err| err.to_string())
        }

        /// Đọc thêm byte cho tới khi có dữ liệu hoặc hết thời gian chờ.
        ///
        /// `Ok(false)` = server im lặng trong khoảng chờ.
        fn fill(&mut self, wait: Duration) -> Result<bool, String> {
            let deadline = Instant::now() + wait;
            let mut chunk = [0u8; 4096];
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Ok(false);
                }
                // Sàn 1 ms: timeout dưới 1 ms làm `SO_RCVTIMEO` bằng 0 ⇒ đọc chặn vô hạn.
                self.stream
                    .set_read_timeout(Some(remaining.max(Duration::from_millis(1))))
                    .map_err(|err| err.to_string())?;
                match self.stream.read(&mut chunk) {
                    Ok(0) => return Err("server đóng kết nối".to_owned()),
                    Ok(n) => {
                        self.buf.extend_from_slice(&chunk[..n]);
                        return Ok(true);
                    }
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err)
                        if err.kind() == io::ErrorKind::WouldBlock
                            || err.kind() == io::ErrorKind::TimedOut =>
                    {
                        // Hết thời gian chờ: kiểm tra mốc rồi thử lại (socket có thể trả
                        // timeout sớm hơn hạn một chút).
                        if Instant::now() >= deadline {
                            return Ok(false);
                        }
                    }
                    Err(err) => return Err(format!("đọc socket lỗi: {err}")),
                }
            }
        }

        /// Cắt một frame trọn vẹn khỏi bộ đệm (tiền tố độ dài 4 byte).
        fn take_frame(&mut self) -> Option<Vec<u8>> {
            if self.buf.len() < 4 {
                return None;
            }
            let len =
                u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
            if self.buf.len() < 4 + len {
                return None;
            }
            let frame = self.buf[4..4 + len].to_vec();
            self.buf.drain(..4 + len);
            Some(frame)
        }

        /// Đọc frame tiếp theo; `Ok(None)` = hết thời gian chờ mà chưa có frame trọn vẹn.
        fn next_frame(&mut self, wait: Duration) -> Result<Option<Vec<u8>>, String> {
            let deadline = Instant::now() + wait;
            loop {
                if let Some(frame) = self.take_frame() {
                    return Ok(Some(frame));
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Ok(None);
                }
                if !self.fill(remaining)? {
                    return Ok(None);
                }
            }
        }

        /// Nhận message đầu tiên có `kind` (bỏ qua kind khác).
        fn recv_of<M: Message>(&mut self, kind: MessageKind) -> Result<M, String> {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let Some(frame) = self.next_frame(remaining)? else {
                    return Err(format!("timeout chờ {kind:?}"));
                };
                let (envelope, _) = read_envelope(&frame).map_err(|err| err.to_string())?;
                if envelope.kind == kind {
                    return decode_frame(&frame, &ProtocolLimits::v1())
                        .map_err(|err| err.to_string());
                }
            }
        }

        /// Đọc mọi frame tới khi server im lặng `wait`; trả về các kind đã nhận.
        fn recv_kinds_until_silence(&mut self, wait: Duration) -> Result<Vec<MessageKind>, String> {
            let deadline = Instant::now() + wait;
            let mut kinds = Vec::new();
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let Some(frame) = self.next_frame(remaining)? else {
                    return Ok(kinds);
                };
                let (envelope, _) = read_envelope(&frame).map_err(|err| err.to_string())?;
                kinds.push(envelope.kind);
            }
        }

        /// Nhận `RoomState` có đúng số thành viên.
        fn recv_room_state(&mut self, members: usize) -> Result<RoomState, String> {
            loop {
                let state: RoomState = self.recv_of(MessageKind::RoomState)?;
                if state.members.len() == members {
                    return Ok(state);
                }
            }
        }

        /// Nhận `RoomState` có đúng số thành viên **và tất cả đã ready**.
        fn recv_room_state_all_ready(&mut self, members: usize) -> Result<RoomState, String> {
            loop {
                let state: RoomState = self.recv_of(MessageKind::RoomState)?;
                if state.members.len() == members && state.members.iter().all(|m| m.ready) {
                    return Ok(state);
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
        write_test_env_with_rules(root, None)
    }

    /// Dựng môi trường test với địa chỉ nghe tuỳ chỉnh (ví dụ `[::]:0`).
    fn write_test_env_with_listen(root: &Path, listen: &str) -> PathBuf {
        write_test_env_full(root, None, listen)
    }

    /// Dựng môi trường test với rules profile tuỳ chỉnh.
    ///
    /// `rules_override`: nội dung thay cho profile chuẩn (`skeleton-test`) — dùng để kiểm
    /// lỗi khi profile thiếu/sai chỉ số nhân vật.
    fn write_test_env_with_rules(root: &Path, rules_override: Option<&str>) -> PathBuf {
        write_test_env_full(root, rules_override, "127.0.0.1:0")
    }

    /// Dựng môi trường test đầy đủ: bundle, bảng phân loại ô, rules profile, file cấu hình.
    fn write_test_env_full(root: &Path, rules_override: Option<&str>, listen: &str) -> PathBuf {
        let bundle_dir = root.join("bundle");
        std::fs::create_dir_all(&bundle_dir).expect("tạo thư mục bundle");
        write_test_bundle(&bundle_dir);

        let map_path = root.join("map.toml");
        std::fs::write(&map_path, TEST_MAP_PROFILE).expect("ghi bảng phân loại ô");

        let rules =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rules/skeleton-test.toml");
        let rules = rules.canonicalize().expect("đường dẫn rules profile");
        let rules = match rules_override {
            Some(text) => {
                let path = root.join("rules.toml");
                std::fs::write(&path, text).expect("ghi rules profile");
                path
            }
            None => rules,
        };
        let data_dir = root.join("data");
        let config_path = root.join("server.toml");
        let text = format!(
            "[server]\n\
             admin_addr = \"127.0.0.1:0\"\n\
             data_dir = \"{data_dir}\"\n\
             listen_addrs = [\"{listen}\"]\n\
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
             registration_open = true\n",
            data_dir = toml_path(&data_dir),
            bundle_dir = toml_path(&bundle_dir),
            rules = toml_path(&rules),
            map_path = toml_path(&map_path),
            db_path = toml_path(&data_dir.join("boom-server.sqlite3")),
            listen = listen,
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

    /// Chờ `RoomState` có đúng số thành viên **và tất cả đã ready**.
    ///
    /// Lệnh ready của hai client đi trên hai kết nối khác nhau ⇒ server có thể xử lý lệnh
    /// `Start` trước khi nhận lệnh ready còn lại. Chờ state có mọi người ready bảo đảm
    /// server đã xử lý xong lệnh ready trước khi client gửi `Start`.
    fn recv_room_state_all_ready(client: &mut Client, members: usize) -> Result<RoomState, String> {
        loop {
            let state: RoomState = client.recv_of(MessageKind::RoomState)?;
            if state.members.len() == members && state.members.iter().all(|member| member.ready) {
                return Ok(state);
            }
        }
    }

    /// Kết quả luồng client: ID tài khoản đã đăng ký.
    struct ClientOutcome {
        account_id: i64,
    }

    /// Gắn nhãn bước vào lỗi để biết client dừng ở đâu.
    fn step<T>(name: &str, result: Result<T, String>) -> Result<T, String> {
        result.map_err(|err| format!("{name}: {err}"))
    }

    /// Luồng client đầy đủ: handshake → đăng ký → tạo/join phòng → ready → start → snapshot.
    fn drive_two_clients(
        addr: SocketAddr,
        config: &OperationalConfig,
    ) -> Result<ClientOutcome, String> {
        let (outcome, _clients) = drive_two_clients_keep_open(addr, config)?;
        Ok(outcome)
    }

    /// Như [`drive_two_clients`] nhưng **trả lại** client để caller giữ kết nối mở.
    ///
    /// Test log cần client còn sống khi server dừng: nếu socket đóng trước, lý do disconnect
    /// là `client_closed` chứ không phải `server_shutdown`.
    fn drive_two_clients_keep_open(
        addr: SocketAddr,
        config: &OperationalConfig,
    ) -> Result<(ClientOutcome, Vec<Client>), String> {
        let mut host = Client::connect(addr)?;
        let mut guest = Client::connect(addr)?;
        for (name, client) in [("host", &mut host), ("guest", &mut guest)] {
            step(
                &format!("gửi Hello ({name})"),
                client.send(&Hello {
                    wire_schema: 1,
                    state_schema: 1,
                    rules_profile_id: config.rules_profile_id.clone(),
                    rules_hash: config.rules_hash,
                    content_sha256: config.content_sha256,
                    client_build: "test".to_owned(),
                    capabilities: Vec::new(),
                }),
            )?;
            let welcome: Welcome = step(
                &format!("nhận Welcome ({name})"),
                client.recv_of(MessageKind::Welcome),
            )?;
            if welcome.rules_hash != config.rules_hash {
                return Err("Welcome mang rules hash khác server".to_owned());
            }
        }

        let mut account_id = 0i64;
        for (index, client) in [&mut host, &mut guest].into_iter().enumerate() {
            let name = format!("đăng ký (client {index})");
            step(
                &format!("gửi Auth ({name})"),
                client.send(&Auth {
                    op: AuthOp::Register,
                    username: format!("player{index}"),
                    password: "password123".to_owned(),
                }),
            )?;
            let ok: AuthOk = step(
                &format!("nhận AuthOk ({name})"),
                client.recv_of(MessageKind::AuthOk),
            )?;
            if index == 0 {
                account_id = i64::try_from(ok.account_id).unwrap_or_default();
            }
        }

        step(
            "gửi Create",
            host.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Create {
                    character: RoomCharacter::C00,
                },
            }),
        )?;
        let state = step("nhận RoomState (host)", recv_room_state(&mut host, 1))?;
        let room = state.room_id;

        step(
            "gửi Join",
            guest.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Join {
                    room,
                    character: RoomCharacter::C01,
                },
            }),
        )?;
        step("nhận RoomState (guest)", recv_room_state(&mut guest, 2))?;
        step("nhận RoomState (host)", recv_room_state(&mut host, 2))?;

        for client in [&mut host, &mut guest] {
            step(
                "gửi SetReady",
                client.send(&RoomCommandDto {
                    request: 2,
                    action: RoomActionDto::SetReady { ready: true },
                }),
            )?;
        }
        step(
            "nhận RoomState (host, mọi người ready)",
            recv_room_state_all_ready(&mut host, 2),
        )?;
        step(
            "nhận RoomState (guest, mọi người ready)",
            recv_room_state_all_ready(&mut guest, 2),
        )?;

        step(
            "gửi Start",
            host.send(&RoomCommandDto {
                request: 3,
                action: RoomActionDto::Start,
            }),
        )?;
        let start: RoundStart = step("nhận RoundStart", host.recv_of(MessageKind::RoundStart))?;
        let round = start.round_id;
        if start.roster.len() != 2 {
            return Err("roster phải có 2 người".to_owned());
        }

        for client in [&mut host, &mut guest] {
            step(
                "gửi LoadingAck",
                client.send(&LoadingAck { round_id: round }),
            )?;
        }
        let _: Snapshot = step("nhận Snapshot (host)", host.recv_of(MessageKind::Snapshot))?;
        let _: Snapshot = step(
            "nhận Snapshot (guest)",
            guest.recv_of(MessageKind::Snapshot),
        )?;

        Ok((ClientOutcome { account_id }, vec![host, guest]))
    }

    /// Luồng kiểm thứ tự snapshot: `RoundStart` → (im lặng khi còn ở loading barrier) →
    /// `LoadingAck` → `Snapshot` của lượt (ADR 0009 §4.3).
    fn drive_loading_barrier(addr: SocketAddr, config: &OperationalConfig) -> Result<(), String> {
        let mut host = RawClient::connect(addr)?;
        let mut guest = RawClient::connect(addr)?;
        for (name, client) in [("host", &mut host), ("guest", &mut guest)] {
            step(
                &format!("gửi Hello ({name})"),
                client.send(&Hello {
                    wire_schema: 1,
                    state_schema: 1,
                    rules_profile_id: config.rules_profile_id.clone(),
                    rules_hash: config.rules_hash,
                    content_sha256: config.content_sha256,
                    client_build: "test".to_owned(),
                    capabilities: Vec::new(),
                }),
            )?;
            let welcome: Welcome = step(
                &format!("nhận Welcome ({name})"),
                client.recv_of(MessageKind::Welcome),
            )?;
            if welcome.rules_hash != config.rules_hash {
                return Err("Welcome mang rules hash khác server".to_owned());
            }
            step(
                &format!("gửi Auth ({name})"),
                client.send(&Auth {
                    op: AuthOp::Register,
                    username: format!("player-{name}"),
                    password: "password123".to_owned(),
                }),
            )?;
            let _: AuthOk = step(
                &format!("nhận AuthOk ({name})"),
                client.recv_of(MessageKind::AuthOk),
            )?;
        }

        step(
            "gửi Create",
            host.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Create {
                    character: RoomCharacter::C00,
                },
            }),
        )?;
        let state = step("nhận RoomState (host)", host.recv_room_state(1))?;
        let room = state.room_id;
        step(
            "gửi Join",
            guest.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Join {
                    room,
                    character: RoomCharacter::C01,
                },
            }),
        )?;
        step("nhận RoomState (guest)", guest.recv_room_state(2))?;
        step("nhận RoomState (host)", host.recv_room_state(2))?;
        for client in [&mut host, &mut guest] {
            step(
                "gửi SetReady",
                client.send(&RoomCommandDto {
                    request: 2,
                    action: RoomActionDto::SetReady { ready: true },
                }),
            )?;
        }
        step(
            "nhận RoomState (host, mọi người ready)",
            host.recv_room_state_all_ready(2),
        )?;
        step(
            "nhận RoomState (guest, mọi người ready)",
            guest.recv_room_state_all_ready(2),
        )?;

        step(
            "gửi Start",
            host.send(&RoomCommandDto {
                request: 3,
                action: RoomActionDto::Start,
            }),
        )?;
        let start: RoundStart = step(
            "nhận RoundStart (host)",
            host.recv_of(MessageKind::RoundStart),
        )?;
        let _: RoundStart = step(
            "nhận RoundStart (guest)",
            guest.recv_of(MessageKind::RoundStart),
        )?;
        let round = start.round_id;

        // Phòng còn ở loading barrier: server **không** được gửi `Snapshot` nào.
        for (name, client) in [("host", &mut host), ("guest", &mut guest)] {
            let kinds = step(
                &format!("đọc frame khi còn loading ({name})"),
                client.recv_kinds_until_silence(Duration::from_millis(300)),
            )?;
            if kinds.contains(&MessageKind::Snapshot) {
                return Err(format!(
                    "server gửi Snapshot khi phòng còn ở loading barrier ({name}): {kinds:?}"
                ));
            }
        }

        // Qua barrier: snapshot của lượt phải tới, và lượt đã chạy (`tick > 0`).
        for client in [&mut host, &mut guest] {
            step(
                "gửi LoadingAck",
                client.send(&LoadingAck { round_id: round }),
            )?;
        }
        for (name, client) in [("host", &mut host), ("guest", &mut guest)] {
            let snapshot: Snapshot = step(
                &format!("nhận Snapshot ({name})"),
                client.recv_of(MessageKind::Snapshot),
            )?;
            if u64::from(snapshot.round_id) != round {
                return Err(format!(
                    "snapshot thuộc round {} khác round {round}",
                    snapshot.round_id
                ));
            }
            if snapshot.tick == 0 {
                return Err("snapshot đầu của lượt phải có tick > 0 (lượt đã chạy)".to_owned());
            }
        }
        Ok(())
    }

    // --- Acceptance: slot trận lấy từ roster theo `account_id`, không theo thứ tự TCP ---

    /// Vị trí người chơi trong snapshot (mpx).
    type Position = (i32, i32);

    /// Kết quả acceptance của một client: slot trận và vị trí trước/sau khi gửi input.
    struct MoveOutcome {
        /// Slot trận lấy từ `RoundStart.roster` theo `account_id`.
        slot: u8,
        /// Vị trí của mình trước khi gửi input.
        before: Position,
        /// Vị trí của mình sau khi gửi input.
        after: Position,
    }

    /// Slot trận của một account: `slot` của mục `RoundStart.roster` có `player == account_id`.
    fn slot_in_roster(start: &RoundStart, account_id: u64) -> Result<u8, String> {
        start
            .roster
            .iter()
            .find(|member| member.player == account_id)
            .map(|member| member.slot)
            .ok_or_else(|| format!("roster không có người chơi account {account_id}"))
    }

    /// Vị trí của một người chơi trong snapshot (`id` của snapshot là slot trận).
    fn player_position(snapshot: &Snapshot, slot: u8) -> Option<Position> {
        snapshot
            .players
            .iter()
            .find(|player| player.id == slot)
            .map(|player| (player.x, player.y))
    }

    /// Handshake + đăng ký một client; trả `account_id` (định danh người chơi trong phòng).
    fn handshake_and_register(
        client: &mut Client,
        config: &OperationalConfig,
        username: &str,
    ) -> Result<u64, String> {
        step(
            "gửi Hello",
            client.send(&Hello {
                wire_schema: 1,
                state_schema: 1,
                rules_profile_id: config.rules_profile_id.clone(),
                rules_hash: config.rules_hash,
                content_sha256: config.content_sha256,
                client_build: "test".to_owned(),
                capabilities: Vec::new(),
            }),
        )?;
        let welcome: Welcome = step("nhận Welcome", client.recv_of(MessageKind::Welcome))?;
        if welcome.rules_hash != config.rules_hash {
            return Err("Welcome mang rules hash khác server".to_owned());
        }
        step(
            "gửi Auth",
            client.send(&Auth {
                op: AuthOp::Register,
                username: username.to_owned(),
                password: "password123".to_owned(),
            }),
        )?;
        let ok: AuthOk = step("nhận AuthOk", client.recv_of(MessageKind::AuthOk))?;
        Ok(ok.account_id)
    }

    /// Đọc snapshot tới khi round đã chạy (tick > 0); trả tick và vị trí hai người chơi.
    fn read_playing_snapshot(
        client: &mut Client,
        my_slot: u8,
        other_slot: u8,
    ) -> Result<(u32, Position, Position), String> {
        for _ in 0..500 {
            let snapshot: Snapshot = client.recv_of(MessageKind::Snapshot)?;
            if snapshot.tick == 0 {
                // Vẫn ở loading barrier: tick chưa chạy.
                continue;
            }
            let mine = player_position(&snapshot, my_slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {my_slot}"))?;
            let other = player_position(&snapshot, other_slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {other_slot}"))?;
            return Ok((snapshot.tick, mine, other));
        }
        Err("round không chạy (tick vẫn 0)".to_owned())
    }

    /// Gửi một lệnh đi rồi chờ **nhân vật của mình** đổi vị trí trong snapshot.
    ///
    /// `claimed_slot` là slot client khai trong batch: nếu khác slot server bind thì batch
    /// bị bỏ ⇒ nhân vật của mình **không** đổi vị trí (kiểm bằng `expect_move`).
    fn send_move(
        client: &mut Client,
        round_id: u32,
        claimed_slot: u8,
        seq: u32,
        target_tick: u32,
        dir: u8,
    ) -> Result<(), String> {
        let batch = InputBatch {
            round_id,
            player: claimed_slot,
            ack_snapshot: 0,
            commands: vec![InputCommand {
                seq,
                target_tick,
                bits: InputBits::new(dir, false).expect("hướng hợp lệ"),
            }],
        };
        step("gửi InputBatch", client.send(&batch))
    }

    /// Chờ tới khi vị trí `slot` khác `before`; trả vị trí mới và vị trí người chơi khác.
    fn wait_move(
        client: &mut Client,
        slot: u8,
        other_slot: u8,
        before: Position,
    ) -> Result<(Position, Position), String> {
        for _ in 0..500 {
            let snapshot: Snapshot = client.recv_of(MessageKind::Snapshot)?;
            let mine = player_position(&snapshot, slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {slot}"))?;
            if mine == before {
                continue;
            }
            let other = player_position(&snapshot, other_slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {other_slot}"))?;
            return Ok((mine, other));
        }
        Err(format!("nhân vật slot {slot} không di chuyển"))
    }

    /// Đọc snapshot Playing có tick **lớn hơn** `min_tick`.
    ///
    /// Dùng để chủ động vượt cửa sổ late của input trước khi gửi một lệnh cũ; không sleep.
    fn read_playing_snapshot_after(
        client: &mut Client,
        my_slot: u8,
        other_slot: u8,
        min_tick: u32,
    ) -> Result<(u32, Position, Position), String> {
        for _ in 0..500 {
            let snapshot: Snapshot = client.recv_of(MessageKind::Snapshot)?;
            if snapshot.tick <= min_tick {
                continue;
            }
            let mine = player_position(&snapshot, my_slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {my_slot}"))?;
            let other = player_position(&snapshot, other_slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {other_slot}"))?;
            return Ok((snapshot.tick, mine, other));
        }
        Err(format!("không có snapshot sau tick {min_tick}"))
    }

    /// Đọc thêm `snapshots` frame `Snapshot` và khẳng định vị trí `slot` **không đổi**.
    fn wait_no_move(
        client: &mut Client,
        slot: u8,
        before: Position,
        snapshots: usize,
    ) -> Result<(), String> {
        for _ in 0..snapshots {
            let snapshot: Snapshot = client.recv_of(MessageKind::Snapshot)?;
            let mine = player_position(&snapshot, slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {slot}"))?;
            if mine != before {
                return Err(format!(
                    "lệnh cũ không được di chuyển slot {slot}: {before:?} → {mine:?}"
                ));
            }
        }
        Ok(())
    }

    /// Gửi lệnh mới theo từng snapshot cho tới khi `slot` di chuyển.
    ///
    /// Command cũ có thể tới sau khi server đã vượt `late`; mỗi vòng dùng sequence mới và
    /// target từ snapshot mới, không resend payload cũ. Vòng đọc snapshot cũng trả `Pong`.
    fn move_with_fresh_input(
        client: &mut Client,
        round_id: u32,
        claimed_slot: u8,
        other_slot: u8,
        before: Position,
        mut seq: u32,
        dir: u8,
    ) -> Result<(Position, Position), String> {
        for _ in 0..50 {
            let snapshot: Snapshot = client.recv_of(MessageKind::Snapshot)?;
            let mine = player_position(&snapshot, claimed_slot)
                .ok_or_else(|| format!("snapshot không có người chơi slot {claimed_slot}"))?;
            if mine != before {
                let other = player_position(&snapshot, other_slot)
                    .ok_or_else(|| format!("snapshot không có người chơi slot {other_slot}"))?;
                return Ok((mine, other));
            }
            send_move(client, round_id, claimed_slot, seq, snapshot.tick + 1, dir)?;
            seq += 1;
            for _ in 0..3 {
                let next: Snapshot = client.recv_of(MessageKind::Snapshot)?;
                let mine = player_position(&next, claimed_slot)
                    .ok_or_else(|| format!("snapshot không có người chơi slot {claimed_slot}"))?;
                if mine != before {
                    let other = player_position(&next, other_slot)
                        .ok_or_else(|| format!("snapshot không có người chơi slot {other_slot}"))?;
                    return Ok((mine, other));
                }
            }
        }
        Err(format!("slot {claimed_slot} không di chuyển sau input mới"))
    }

    /// Luồng acceptance: một probe + hai client thật, **join ngược thứ tự kết nối**.
    ///
    /// Mỗi client lấy slot của mình từ `RoundStart.roster` theo `account_id` (không suy từ
    /// `Welcome.player_id`), gửi input và kiểm **nhân vật của mình** di chuyển.
    fn drive_two_clients_reversed(
        addr: SocketAddr,
        config: &OperationalConfig,
    ) -> Result<(MoveOutcome, MoveOutcome), String> {
        // Probe: kết nối rồi đóng trước hai client thật ⇒ thứ tự kết nối TCP lệch với
        // thứ tự slot trong phòng (đúng lỗi của `next_slot` cũ).
        {
            let probe = TcpStream::connect(addr).map_err(|err| format!("probe connect: {err}"))?;
            drop(probe);
        }

        // `first` kết nối trước, `second` kết nối sau; vào phòng thì ngược lại.
        let mut first = Client::connect(addr)?;
        let mut second = Client::connect(addr)?;
        let first_account = handshake_and_register(&mut first, config, "player-first")?;
        let second_account = handshake_and_register(&mut second, config, "player-second")?;

        step(
            "gửi Create (second)",
            second.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Create {
                    character: RoomCharacter::C00,
                },
            }),
        )?;
        let state = step("nhận RoomState (second)", recv_room_state(&mut second, 1))?;
        let room = state.room_id;
        step(
            "gửi Join (first)",
            first.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Join {
                    room,
                    character: RoomCharacter::C01,
                },
            }),
        )?;
        step("nhận RoomState (first)", recv_room_state(&mut first, 2))?;
        step("nhận RoomState (second)", recv_room_state(&mut second, 2))?;

        for (client, name) in [(&mut first, "first"), (&mut second, "second")] {
            step(
                &format!("gửi SetReady ({name})"),
                client.send(&RoomCommandDto {
                    request: 2,
                    action: RoomActionDto::SetReady { ready: true },
                }),
            )?;
        }
        step(
            "nhận RoomState (first, mọi người ready)",
            recv_room_state_all_ready(&mut first, 2),
        )?;
        step(
            "nhận RoomState (second, mọi người ready)",
            recv_room_state_all_ready(&mut second, 2),
        )?;

        step(
            "gửi Start (second)",
            second.send(&RoomCommandDto {
                request: 3,
                action: RoomActionDto::Start,
            }),
        )?;
        let start_first: RoundStart = step(
            "nhận RoundStart (first)",
            first.recv_of(MessageKind::RoundStart),
        )?;
        let start_second: RoundStart = step(
            "nhận RoundStart (second)",
            second.recv_of(MessageKind::RoundStart),
        )?;
        let round =
            u32::try_from(start_first.round_id).map_err(|_| "round id vượt u32".to_owned())?;
        // Slot lấy từ roster theo `account_id` — **không** suy từ `Welcome.player_id`.
        let slot_first = slot_in_roster(&start_first, first_account)?;
        let slot_second = slot_in_roster(&start_second, second_account)?;
        if slot_first == slot_second {
            return Err("hai người chơi phải có slot khác nhau".to_owned());
        }

        for client in [&mut first, &mut second] {
            step(
                "gửi LoadingAck",
                client.send(&LoadingAck {
                    round_id: start_first.round_id,
                }),
            )?;
        }

        // Mỗi client đi theo **hướng khác nhau**: điều này chứng minh đúng nhân vật di
        // chuyển (đi ngang làm `x` đổi, đi dọc làm `y` đổi) chứ không chỉ "có ai đó đi".
        let (_tick_first, before_first, _) =
            read_playing_snapshot(&mut first, slot_first, slot_second)?;
        // Cả hai client dùng producer mới: target/sequence lấy từ snapshot mới, không
        // gửi một lệnh từ snapshot đầu rồi chờ vô ích khi backlog đã vượt late.
        let (after_first, _) = move_with_fresh_input(
            &mut first,
            round,
            slot_first,
            slot_second,
            before_first,
            1,
            4,
        )?;

        let (tick_second, before_second, _) =
            read_playing_snapshot(&mut second, slot_second, slot_first)?;
        // Test clock dùng tick snapshot; protocol hiện có INPUT_LATE = 6 tick. +20 bảo đảm
        // command seq1 bên dưới đã ra ngoài cửa sổ, không phụ thuộc wall-clock/scheduler.
        let (stale_tick, _, _) = read_playing_snapshot_after(
            &mut second,
            slot_second,
            slot_first,
            tick_second.saturating_add(20),
        )?;
        if stale_tick <= tick_second {
            return Err("snapshot chưa vượt cửa sổ late".to_owned());
        }
        send_move(&mut second, round, slot_second, 1, tick_second + 1, 2)?;
        wait_no_move(&mut second, slot_second, before_second, 3)?;
        // Sequence mới, target từ snapshot mới; payload cũ không được resend.
        let (after_second, _) = move_with_fresh_input(
            &mut second,
            round,
            slot_second,
            slot_first,
            before_second,
            2,
            2,
        )?;

        Ok((
            MoveOutcome {
                slot: slot_first,
                before: before_first,
                after: after_first,
            },
            MoveOutcome {
                slot: slot_second,
                before: before_second,
                after: after_second,
            },
        ))
    }

    #[test]
    fn serve_binds_round_slot_from_roster_not_tcp_order() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        let mut runtime =
            ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
        let addr = runtime.local_addr();
        let effective = runtime.config().clone();

        let client_stop = stop.clone();
        let client_config = effective.clone();
        let handle = thread::spawn(move || {
            let _guard = StopOnDrop(client_stop);
            drive_two_clients_reversed(addr, &client_config)
        });

        runtime.run().expect("vòng tick dừng sạch");
        let (first, second) = handle
            .join()
            .expect("luồng client không panic")
            .expect("probe + hai client: input, di chuyển đúng nhân vật");

        // Slot trận lấy từ **phòng** (người tạo phòng trước), không theo thứ tự kết nối TCP:
        // `second` kết nối sau nhưng tạo phòng trước ⇒ slot 0; `first` vào sau ⇒ slot 1.
        assert_eq!(second.slot, 0, "người tạo phòng phải có slot 0");
        assert_eq!(first.slot, 1, "người vào sau phải có slot 1");

        // Mỗi người di chuyển **đúng nhân vật của mình**: đi ngang ⇒ `x` đổi, `y` giữ;
        // đi dọc ⇒ `y` đổi, `x` giữ.
        assert!(
            first.after.0 > first.before.0 && first.after.1 == first.before.1,
            "client thứ nhất phải đi sang phải: {:?} → {:?}",
            first.before,
            first.after
        );
        assert!(
            second.after.1 > second.before.1 && second.after.0 == second.before.0,
            "client thứ hai phải đi xuống: {:?} → {:?}",
            second.before,
            second.after
        );
    }

    #[test]
    fn serve_drops_input_that_claims_another_players_slot() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        let mut runtime =
            ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
        let addr = runtime.local_addr();
        let effective = runtime.config().clone();

        let client_stop = stop.clone();
        let client_config = effective.clone();
        let handle = thread::spawn(move || {
            let _guard = StopOnDrop(client_stop);
            drive_two_clients_claiming_other_slot(addr, &client_config)
        });

        runtime.run().expect("vòng tick dừng sạch");
        let outcome = handle
            .join()
            .expect("luồng client không panic")
            .expect("batch khai slot của người khác phải bị bỏ");
        assert_eq!(outcome, 1, "phải đếm đúng một batch bị bỏ");
        assert_eq!(
            runtime.input_slot_mismatch_count(),
            1,
            "server phải đếm batch input khai sai slot"
        );
    }

    /// Client gửi batch khai **slot của người chơi khác**: server phải bỏ batch, không
    /// áp dụng cho ai (ADR 0009 Addendum B §B.2 — không tin client khai).
    fn drive_two_clients_claiming_other_slot(
        addr: SocketAddr,
        config: &OperationalConfig,
    ) -> Result<u32, String> {
        let mut first = Client::connect(addr)?;
        let mut second = Client::connect(addr)?;
        let first_account = handshake_and_register(&mut first, config, "player-first")?;
        let second_account = handshake_and_register(&mut second, config, "player-second")?;

        step(
            "gửi Create (first)",
            first.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Create {
                    character: RoomCharacter::C00,
                },
            }),
        )?;
        let state = step("nhận RoomState (first)", recv_room_state(&mut first, 1))?;
        let room = state.room_id;
        step(
            "gửi Join (second)",
            second.send(&RoomCommandDto {
                request: 1,
                action: RoomActionDto::Join {
                    room,
                    character: RoomCharacter::C01,
                },
            }),
        )?;
        step("nhận RoomState (second)", recv_room_state(&mut second, 2))?;
        step("nhận RoomState (first)", recv_room_state(&mut first, 2))?;

        for (client, name) in [(&mut first, "first"), (&mut second, "second")] {
            step(
                &format!("gửi SetReady ({name})"),
                client.send(&RoomCommandDto {
                    request: 2,
                    action: RoomActionDto::SetReady { ready: true },
                }),
            )?;
        }
        step(
            "nhận RoomState (first, mọi người ready)",
            recv_room_state_all_ready(&mut first, 2),
        )?;
        step(
            "nhận RoomState (second, mọi người ready)",
            recv_room_state_all_ready(&mut second, 2),
        )?;

        step(
            "gửi Start (first)",
            first.send(&RoomCommandDto {
                request: 3,
                action: RoomActionDto::Start,
            }),
        )?;
        let start_first: RoundStart = step(
            "nhận RoundStart (first)",
            first.recv_of(MessageKind::RoundStart),
        )?;
        let start_second: RoundStart = step(
            "nhận RoundStart (second)",
            second.recv_of(MessageKind::RoundStart),
        )?;
        let round =
            u32::try_from(start_first.round_id).map_err(|_| "round id vượt u32".to_owned())?;
        let slot_first = slot_in_roster(&start_first, first_account)?;
        let slot_second = slot_in_roster(&start_second, second_account)?;

        for client in [&mut first, &mut second] {
            step(
                "gửi LoadingAck",
                client.send(&LoadingAck {
                    round_id: start_first.round_id,
                }),
            )?;
        }

        let (tick_first, before_first, _) =
            read_playing_snapshot(&mut first, slot_first, slot_second)?;
        let (tick_second, before_second, _) =
            read_playing_snapshot(&mut second, slot_second, slot_first)?;

        // `first` khai slot của `second` ⇒ batch phải bị bỏ, không ai di chuyển.
        send_move(&mut first, round, slot_second, 1, tick_first + 1, 4)?;
        // `second` gửi lệnh hợp lệ của mình: nó vẫn phải di chuyển.
        send_move(&mut second, round, slot_second, 1, tick_second + 1, 2)?;
        let (after_second, _) = wait_move(&mut second, slot_second, slot_first, before_second)?;

        // Chờ thêm vài snapshot rồi kiểm `first` **không** di chuyển.
        let mut first_moved = false;
        for _ in 0..20 {
            let snapshot: Snapshot = first.recv_of(MessageKind::Snapshot)?;
            if let Some(position) = player_position(&snapshot, slot_first)
                && position != before_first
            {
                first_moved = true;
                break;
            }
        }
        if first_moved {
            return Err(
                "batch khai slot của người khác không được làm người khai di chuyển".to_owned(),
            );
        }
        if after_second == before_second {
            return Err("người gửi slot đúng phải di chuyển".to_owned());
        }
        Ok(1)
    }

    #[test]
    fn serve_shutdown_finishes_with_ipv6_wildcard_listener() {
        // OS không có IPv6 thì bỏ qua (không phải lỗi của server).
        if TcpListener::bind("[::]:0").is_err() {
            eprintln!("bỏ qua: OS không hỗ trợ IPv6");
            return;
        }
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env_with_listen(dir.path(), "[::]:0");
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        // Luồng riêng yêu cầu dừng sau một nhịp: vòng tick phải thoát, rồi `Drop` dọn dẹp.
        let stopper = stop.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            stopper.request();
        });

        let started = Instant::now();
        {
            let mut runtime =
                ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
            assert!(
                runtime.local_addr().is_ipv6(),
                "listener phải là IPv6: {:?}",
                runtime.local_addr()
            );
            runtime.run().expect("vòng tick dừng sạch");
        }
        // Shutdown (kết nối giả + join có hạn) phải xong trong hạn: socket IPv6-only không
        // nhận kết nối giả IPv4 ⇒ vòng accept phải dừng được theo cả hai họ loopback.
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "server không dừng trong hạn với listener IPv6: {:?}",
            started.elapsed()
        );
    }

    /// Socket của `accept()` phải là blocking trên mọi OS.
    ///
    /// BSD/macOS: socket trả về **kế thừa** cờ non-blocking của listener (khác Linux) ⇒
    /// luồng đọc của session thấy `WouldBlock` và đóng kết nối ngay. Vòng accept của
    /// server dùng listener **blocking** và dừng bằng kết nối giả, nên socket luôn
    /// blocking; test này giữ hành vi đó.
    #[test]
    fn accepted_socket_is_blocking_after_accept() {
        use std::io::{Read, Write};
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let sender = thread::spawn(move || {
            let mut client = TcpStream::connect(addr).expect("connect");
            thread::sleep(Duration::from_millis(100));
            client.write_all(b"x").expect("ghi");
        });
        let (mut stream, _) = listener.accept().expect("accept");
        // Đọc phải **chờ** dữ liệu (socket blocking) chứ không trả `WouldBlock` ngay.
        let mut buf = [0u8; 1];
        let read = stream.read(&mut buf).expect("đọc phải chờ dữ liệu");
        assert_eq!(read, 1, "phải đọc được byte client gửi");
        sender.join().expect("luồng client");
    }

    #[test]
    fn serve_reads_character_stats_from_rules_profile() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");
        let content = load_content(&config).expect("nạp content");

        // Chỉ số đến từ rules profile (`characters.*`), không từ file cấu hình.
        assert_eq!(content.stats.c00.speed.value, 5);
        assert_eq!(content.stats.c00.speed.max, 7);
        assert_eq!(content.stats.c00.bombs.value, 1);
        assert_eq!(content.stats.c00.bombs.max, 10);
        assert_eq!(content.stats.c00.range.value, 1);
        assert_eq!(content.stats.c00.range.max, 7);
        assert_eq!(content.stats.c01.speed.value, 4);
        assert_eq!(content.stats.c01.speed.max, 8);
        assert_eq!(content.stats.c01.bombs.value, 2);
        assert_eq!(content.stats.c01.bombs.max, 7);
        assert_eq!(content.stats.c01.range.value, 1);
        assert_eq!(content.stats.c01.range.max, 9);
    }

    #[test]
    fn serve_reports_clear_error_when_profile_lacks_characters() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        // Profile không có bảng chỉ số nhân vật ⇒ lỗi rõ, không tự đặt mặc định.
        let profile = include_str!("../../../fixtures/rules/skeleton-test.toml")
            .split("[characters.c01_range]")
            .next()
            .expect("còn phần C00")
            .to_owned();
        let config_path = write_test_env_with_rules(dir.path(), Some(&profile));
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");
        let error = load_content(&config).expect_err("profile thiếu nhân vật phải lỗi");
        let message = error.to_string();
        assert!(message.contains("characters.c01"), "{message}");
    }

    #[test]
    fn serve_reports_clear_error_when_profile_character_value_above_max() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        // Cấp hiện tại vượt trần ⇒ lỗi rõ, không tự sửa.
        let profile = include_str!("../../../fixtures/rules/skeleton-test.toml").replace(
            "[characters.c00_speed]\nvalue = 5",
            "[characters.c00_speed]\nvalue = 8",
        );
        let config_path = write_test_env_with_rules(dir.path(), Some(&profile));
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");
        let error = load_content(&config).expect_err("giá trị vượt trần phải lỗi");
        let message = error.to_string();
        assert!(message.contains("vượt trần"), "{message}");
    }

    #[test]
    fn rules_hash_changes_when_character_stats_change() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");
        let base = load_content(&config).expect("nạp content");

        // Đổi chỉ số trong profile ⇒ `rules_hash` đổi ⇒ handshake bắt được lệch.
        let profile = include_str!("../../../fixtures/rules/skeleton-test.toml").replace(
            "[characters.c00_speed]\nvalue = 5",
            "[characters.c00_speed]\nvalue = 6",
        );
        let other_dir = tempfile::tempdir().expect("thư mục tạm");
        let other_path = write_test_env_with_rules(other_dir.path(), Some(&profile));
        let other_config =
            OperationalConfig::from_file_with_env(&other_path).expect("đọc cấu hình");
        let other = load_content(&other_config).expect("nạp content");

        assert_ne!(base.rules_hash, other.rules_hash);
        assert_eq!(other.stats.c00.speed.value, 6);
        assert_eq!(base.stats.c00.speed.value, 5);
    }

    /// Server **không** gửi `Snapshot` khi phòng còn ở loading barrier (ADR 0009 §4.3).
    ///
    /// Sau `RoundStart` nhưng trước khi mọi người gửi `LoadingAck`, trên dây không được có
    /// `Snapshot` nào; ngay khi barrier qua thì snapshot của lượt phải tới (và lượt đã chạy).
    #[test]
    fn serve_khong_gui_snapshot_khi_phong_dang_loading() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        let mut runtime =
            ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
        let addr = runtime.local_addr();
        let effective = runtime.config().clone();

        let client_stop = stop.clone();
        let client_config = effective.clone();
        let handle = thread::spawn(move || {
            let _guard = StopOnDrop(client_stop);
            drive_loading_barrier(addr, &client_config)
        });

        runtime.run().expect("vòng tick dừng sạch");
        handle
            .join()
            .expect("luồng client không panic")
            .expect("phải kiểm được thứ tự snapshot");
    }

    #[test]
    fn serve_runs_two_tcp_clients_through_a_round_and_revokes_sessions_on_stop() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        let outcome = {
            let mut runtime =
                ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
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
            handle
                .join()
                .expect("luồng client không panic")
                .expect("client đăng ký → phòng → start → snapshot")
            // Runtime thả ở đây ⇒ nhả khoá instance DB trước khi mở lại DB kiểm phiên.
        };

        // Health runtime trả 0 khi server đang chạy (đã kiểm trong luồng client).
        // Dừng sạch ⇒ DB không còn phiên hiệu lực của instance này.
        let store =
            SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).expect("mở DB");
        let active = store
            .active_session(AccountId(outcome.account_id))
            .expect("đọc phiên");
        assert!(active.is_none(), "phiên phải được thu hồi khi dừng");
    }

    /// Dòng log đầu tiên có `event` = `name` (không có ⇒ test đỏ, không bỏ qua âm thầm).
    fn first_log_line(buffer: &SharedLogBuffer, name: &str) -> serde_json::Value {
        buffer
            .lines()
            .into_iter()
            .find(|line| line.get("event").and_then(|value| value.as_str()) == Some(name))
            .unwrap_or_else(|| {
                panic!(
                    "không có sự kiện `{name}` trong log: {:?}",
                    buffer
                        .lines()
                        .iter()
                        .map(|line| line.get("event").cloned())
                        .collect::<Vec<_>>()
                )
            })
    }

    /// Log JSON của tiến trình server thật: vòng đời round có `round_id`, disconnect có lý do.
    #[test]
    fn serve_log_json_co_round_id_va_ly_do_disconnect() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let buffer = SharedLogBuffer::new();
        let stop = StopFlag::default();
        {
            let mut runtime = ServeRuntime::start_with_log_sink(
                config.clone(),
                stop.clone(),
                Box::new(buffer.clone()),
            )
            .expect("khởi động server");
            let addr = runtime.local_addr();
            let effective = runtime.config().clone();

            // Client giữ kết nối **mở** tới khi server dừng ⇒ lý do disconnect lúc dừng là
            // `server_shutdown` (không phải `client_closed`).
            let (done_tx, done_rx) = mpsc::channel::<()>();
            let client_stop = stop.clone();
            let client_config = effective.clone();
            let handle = thread::spawn(move || {
                let _guard = StopOnDrop(client_stop.clone());
                let (outcome, clients) = drive_two_clients_keep_open(addr, &client_config)
                    .expect("client đăng ký → phòng → start → snapshot");
                // Dừng server nhưng **giữ kết nối** mở tới khi main báo đã dừng xong.
                client_stop.request();
                done_rx.recv().expect("chờ server dừng");
                drop(clients);
                outcome
            });

            runtime.run().expect("vòng tick dừng sạch");
            // Thả runtime ⇒ `shutdown_sessions` chạy ⇒ log `player.disconnect`.
            drop(runtime);
            done_tx.send(()).expect("báo client đóng");
            handle.join().expect("luồng client không panic");
        }

        // Content nạp xong: `content.load_ok` mang `content_sha256` thật.
        let load_ok = first_log_line(&buffer, "content.load_ok");
        assert!(load_ok["content_sha256"].is_string());

        // Vòng đời round: mọi dòng đều có `round_id` khác null.
        for event in ["round.start", "round.loading", "round.playing"] {
            let line = first_log_line(&buffer, event);
            assert!(
                line["round_id"].is_u64(),
                "{event} phải có round_id: {line}"
            );
        }

        // Dừng server ⇒ mỗi phiên đang sống ghi `player.disconnect` lý do `server_shutdown`.
        let disconnect = first_log_line(&buffer, "player.disconnect");
        assert_eq!(disconnect["reason"], serde_json::json!("server_shutdown"));
        assert_eq!(
            disconnect["revoke_reason"],
            serde_json::json!("server_shutdown")
        );
    }

    /// Lỗi nạp content phải đi qua **log JSON**, không chỉ ra stderr.
    #[test]
    fn content_load_fail_duoc_ghi_qua_json_logger() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let mut config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");
        // Rules profile không tồn tại ⇒ nạp content thất bại.
        config.rules_path = dir.path().join("khong-co-rules.toml");

        let buffer = SharedLogBuffer::new();
        let stop = StopFlag::default();
        let Err(err) = ServeRuntime::start_with_log_sink(config, stop, Box::new(buffer.clone()))
        else {
            panic!("nạp content phải thất bại");
        };
        assert!(matches!(err, StartupError::Rules { .. }));

        let line = first_log_line(&buffer, "content.load_fail");
        assert_eq!(line["error_code"], serde_json::json!("rules_load_failed"));
        // `error_message` mang đường dẫn file, không mang nội dung file (ADR 0011 §8.1).
        let message = line["error_message"].as_str().expect("error_message");
        assert!(message.contains("khong-co-rules.toml"), "{message}");
        // Log không được lộ mật khẩu/token.
        let bytes = buffer.bytes();
        let raw = String::from_utf8_lossy(&bytes);
        assert!(!raw.contains("password"), "log lộ mật khẩu: {raw}");
        assert!(!raw.contains("token"), "log lộ token: {raw}");
    }

    #[test]
    fn serve_health_cli_reports_zero_while_running() {
        let dir = tempfile::tempdir().expect("thư mục tạm");
        let config_path = write_test_env(dir.path());
        let config = OperationalConfig::from_file_with_env(&config_path).expect("đọc cấu hình");

        let stop = StopFlag::default();
        let mut runtime =
            ServeRuntime::start(config.clone(), stop.clone()).expect("khởi động server");
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
    #[cfg(unix)]
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
    #[cfg(unix)]
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
