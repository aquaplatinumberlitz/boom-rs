//! Admin listener loopback + `HealthResponse`, artifact heartbeat và CLI `health`
//! (ADR 0011 §8.2, ADR 0009 Addendum A §A.1).
//!
//! Admin nói cùng envelope/frame của game, chỉ bind loopback, chỉ nhận `HealthRequest`; kind
//! khác/peer không loopback bị đóng và log `server.health_rejected`. Không mở HTTP.

use std::io::{self, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use boom_net::{FrameLimits, read_frame, write_frame};
use boom_proto::dto::health::{HealthRequest, HealthResponse, HealthStatus};
use boom_proto::kind::{Channel, Role};
use boom_proto::message::decode_frame_on;
use boom_proto::{ProtocolLimits, encode_frame};
use serde_json::{Value, json};

use crate::config::OperationalConfig;
use crate::observability::{LogFields, Logger, hex, now_ms};

/// Số liệu DB mà health phản ánh; T1 chưa có identity nên mặc định `configured = false`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbHealth {
    pub configured: bool,
    pub ok: bool,
    pub schema_version: u32,
    pub expected_schema_version: u32,
    pub integrity_check: String,
    pub problems: Vec<String>,
}

impl Default for DbHealth {
    fn default() -> Self {
        Self {
            configured: false,
            ok: true,
            schema_version: 0,
            expected_schema_version: 0,
            integrity_check: "not_configured".to_owned(),
            problems: Vec::new(),
        }
    }
}

/// Trạng thái runtime chia sẻ giữa admin listener, heartbeat và CLI test.
pub struct HealthState {
    config: OperationalConfig,
    instance_id: String,
    started_at_ms: u64,
    admin_addr: Mutex<String>,
    last_error_ts: Mutex<Option<u64>>,
    db: Mutex<DbHealth>,
    accounts: AtomicU64,
    active_sessions: AtomicU64,
}

impl HealthState {
    pub fn new(config: OperationalConfig, instance_id: String, started_at_ms: u64) -> Self {
        let admin_addr = config.admin_addr.to_string();
        Self {
            config,
            instance_id,
            started_at_ms,
            admin_addr: Mutex::new(admin_addr),
            last_error_ts: Mutex::new(None),
            db: Mutex::new(DbHealth::default()),
            accounts: AtomicU64::new(0),
            active_sessions: AtomicU64::new(0),
        }
    }

    pub fn config(&self) -> &OperationalConfig {
        &self.config
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn started_at_ms(&self) -> u64 {
        self.started_at_ms
    }

    pub fn admin_addr(&self) -> String {
        self.admin_addr.lock().unwrap().clone()
    }

    pub fn set_admin_addr(&self, addr: SocketAddr) {
        *self.admin_addr.lock().unwrap() = addr.to_string();
    }

    pub fn set_db(&self, db: DbHealth) {
        *self.db.lock().unwrap() = db;
    }

    pub fn db(&self) -> DbHealth {
        self.db.lock().unwrap().clone()
    }

    pub fn mark_error(&self, at_ms: u64) {
        *self.last_error_ts.lock().unwrap() = Some(at_ms);
    }

    pub fn last_error_ts(&self) -> Option<u64> {
        *self.last_error_ts.lock().unwrap()
    }

    pub fn set_accounts(&self, accounts: u64) {
        self.accounts.store(accounts, Ordering::Relaxed);
    }

    pub fn set_active_sessions(&self, sessions: u64) {
        self.active_sessions.store(sessions, Ordering::Relaxed);
    }

    fn problems(&self) -> Vec<String> {
        let db = self.db.lock().unwrap();
        let mut problems = db.problems.clone();
        if !db.ok {
            problems.push("db_not_ok".to_owned());
        }
        if db.configured && db.schema_version != db.expected_schema_version {
            problems.push(format!(
                "schema_version_mismatch:{}!={}",
                db.schema_version, db.expected_schema_version
            ));
        }
        if db.configured && db.integrity_check != "ok" {
            problems.push("integrity_check_failed".to_owned());
        }
        problems
    }

    /// `HealthResponse` đầy đủ từ tiến trình server hiện tại.
    pub fn response(&self, at_ms: u64) -> HealthResponse {
        let db = self.db.lock().unwrap().clone();
        let problems = self.problems();
        let status = if problems.is_empty() && db.ok {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };
        HealthResponse {
            status,
            uptime_ms: at_ms.saturating_sub(self.started_at_ms).max(1),
            started_at_ms: self.started_at_ms,
            instance_id: self.instance_id.clone(),
            listen_addrs: self
                .config
                .listen_addrs
                .iter()
                .map(ToString::to_string)
                .collect(),
            admin_addr: self.admin_addr(),
            last_error_ts: self.last_error_ts(),
            db_ok: db.ok,
            schema_version: db.schema_version,
            expected_schema_version: db.expected_schema_version,
            integrity_check: db.integrity_check,
            rules_profile_id: self.config.rules_profile_id.clone(),
            rules_hash: self.config.rules_hash,
            content_sha256: self.config.content_sha256,
            accounts: self.accounts.load(Ordering::Relaxed),
            active_sessions: self.active_sessions.load(Ordering::Relaxed),
            problems,
        }
    }

    /// Nội dung artifact `health.json`; chỉ để chẩn đoán, không phải nguồn sự thật.
    pub fn artifact(&self, at_ms: u64) -> Value {
        let response = self.response(at_ms);
        json!({
            "instance_id": response.instance_id,
            "pid": std::process::id(),
            "started_at_ms": response.started_at_ms,
            "updated_at_ms": at_ms,
            "listen_addrs": response.listen_addrs,
            "admin_addr": response.admin_addr,
            "status": response.status.name(),
        })
    }
}

/// Ghi `<data_dir>/health.json` atomic: file tạm cùng thư mục rồi rename.
pub fn write_health_json(state: &HealthState, at_ms: u64) -> io::Result<()> {
    let data_dir = &state.config().data_dir;
    std::fs::create_dir_all(data_dir)?;
    let body = serde_json::to_vec(&state.artifact(at_ms)).map_err(io::Error::other)?;
    let tmp = data_dir.join(format!(".health.json.{}.tmp", std::process::id()));
    {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(&body)?;
        file.sync_all()?;
    }
    let dest = data_dir.join("health.json");
    #[cfg(not(windows))]
    std::fs::rename(&tmp, &dest)?;
    #[cfg(windows)]
    {
        // Windows không thay thế file đích bằng `rename`; fallback vẫn đi qua file tạm.
        let _ = std::fs::remove_file(&dest);
        std::fs::rename(&tmp, &dest)?;
    }
    Ok(())
}

/// Heartbeat định kỳ: ghi artifact + log `server.heartbeat`.
pub struct Heartbeat {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Heartbeat {
    pub fn start(
        state: Arc<HealthState>,
        logger: Arc<Logger>,
        interval_ms: u64,
    ) -> io::Result<Self> {
        write_health_json(&state, now_ms())?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let handle = thread::spawn(move || {
            let interval = Duration::from_millis(interval_ms.max(1));
            while !thread_stop.load(Ordering::Relaxed) {
                let mut slept = Duration::ZERO;
                while slept < interval {
                    if thread_stop.load(Ordering::Relaxed) {
                        return;
                    }
                    let step = (interval - slept).min(Duration::from_millis(50));
                    thread::sleep(step);
                    slept += step;
                }
                let at = now_ms();
                if write_health_json(&state, at).is_ok() {
                    logger.heartbeat(at.saturating_sub(state.started_at_ms()));
                } else {
                    let mut fields = LogFields::new();
                    fields.insert("reason", json!("health_json_write_failed"));
                    logger.warn("server.heartbeat_fail", fields);
                }
            }
        });
        Ok(Self {
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for Heartbeat {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Admin listener đã bind loopback; `local_addr` là địa chỉ thật (có thể cổng trống khi bận).
pub struct AdminServer {
    local_addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    logger: Arc<Logger>,
}

impl AdminServer {
    /// Bind admin listener. Chỉ loopback; nếu cổng bận thì dùng cổng trống cùng IP.
    pub fn start(
        config: &OperationalConfig,
        state: Arc<HealthState>,
        logger: Arc<Logger>,
    ) -> io::Result<Self> {
        if !config.admin_addr.ip().is_loopback() {
            let mut fields = LogFields::new();
            fields.insert("reason", json!("admin_not_loopback"));
            logger.error("server.start_blocked", fields);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "admin listener phải bind loopback",
            ));
        }
        let listener = match TcpListener::bind(config.admin_addr) {
            Ok(listener) => listener,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                let fallback = SocketAddr::new(config.admin_addr.ip(), 0);
                TcpListener::bind(fallback)?
            }
            Err(err) => {
                let mut fields = LogFields::new();
                fields.insert("reason", json!("admin_bind_failed"));
                fields.insert("error_message", json!(err.to_string()));
                logger.error("server.start_blocked", fields);
                return Err(err);
            }
        };
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        state.set_admin_addr(local_addr);
        logger.server_start(
            &config
                .listen_addrs
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            &local_addr.to_string(),
        );
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread_logger = logger.clone();
        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, peer)) => {
                        let state = state.clone();
                        let logger = thread_logger.clone();
                        thread::spawn(move || {
                            handle_admin_connection(stream, peer, &state, &logger);
                        });
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });
        Ok(Self {
            local_addr,
            stop,
            handle: Some(handle),
            logger,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for AdminServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        self.logger.server_stop("shutdown");
    }
}

/// Xử lý một kết nối admin. Public để test peer không loopback (listener thật luôn loopback).
pub fn handle_admin_connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    state: &HealthState,
    logger: &Logger,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(2_000)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(2_000)));
    if !peer.ip().is_loopback() {
        let mut fields = LogFields::new();
        fields.insert("reason", json!("non_loopback"));
        fields.insert("peer", json!(peer.to_string()));
        logger.warn("server.health_rejected", fields);
        return;
    }
    let frame = match read_frame(&mut stream, FrameLimits::V1) {
        Ok(Some(frame)) => frame,
        _ => {
            let mut fields = LogFields::new();
            fields.insert("reason", json!("bad_frame"));
            logger.warn("server.health_rejected", fields);
            return;
        }
    };
    let request: HealthRequest = match decode_frame_on::<HealthRequest>(
        &frame,
        &ProtocolLimits::v1(),
        Channel::Admin,
        Role::Server,
    ) {
        Ok(request) => request,
        Err(err) => {
            let mut fields = LogFields::new();
            fields.insert("reason", json!(format!("{err}")));
            logger.warn("server.health_rejected", fields);
            return;
        }
    };
    let _ = request;
    let response = state.response(now_ms());
    let Ok(frame) = encode_frame(&response, &ProtocolLimits::v1()) else {
        return;
    };
    let _ = write_frame(&mut stream, &frame, FrameLimits::V1);
    let mut fields = LogFields::new();
    fields.insert("status", json!(response.status.name()));
    logger.info("server.health", fields);
}

/// Kết quả CLI health: mã thoát + object JSON stdout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthCliOutcome {
    pub code: i32,
    pub body: Value,
}

/// Chạy `boom-server health` runtime (offline=false) hoặc `--offline` (offline=true).
pub fn run_health_cli(config: &OperationalConfig, offline: bool) -> HealthCliOutcome {
    if offline {
        return offline_health(config);
    }
    let timeout = Duration::from_millis(config.health_timeout_ms.max(1));
    let addr = match admin_addr_for_cli(config) {
        Ok(addr) => addr,
        Err(message) => {
            return HealthCliOutcome {
                code: 3,
                body: json!({"status": "unreachable", "scope": "runtime", "error": message}),
            };
        }
    };
    let stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(stream) => stream,
        Err(err) => {
            return cli_failure(
                config,
                addr,
                HealthStatus::Unreachable,
                &format!("không kết nối được: {err}"),
            );
        }
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let result = query_admin(stream);
    match result {
        Ok(response) => {
            let body = runtime_json(config, &response);
            let code = if response.status == HealthStatus::Healthy && response.problems.is_empty() {
                0
            } else {
                1
            };
            HealthCliOutcome { code, body }
        }
        Err(HealthStatus::Timeout) => cli_failure(
            config,
            addr,
            HealthStatus::Timeout,
            "server không trả lời trong timeout",
        ),
        Err(_) => cli_failure(
            config,
            addr,
            HealthStatus::Unreachable,
            "admin port không trả lời hợp lệ",
        ),
    }
}

fn query_admin(mut stream: TcpStream) -> Result<HealthResponse, HealthStatus> {
    let payload = encode_frame(&HealthRequest {}, &ProtocolLimits::v1())
        .map_err(|_| HealthStatus::Unreachable)?;
    write_frame(&mut stream, &payload, FrameLimits::V1).map_err(|_| HealthStatus::Unreachable)?;
    let frame = match read_frame(&mut stream, FrameLimits::V1) {
        Ok(Some(frame)) => frame,
        Ok(None) => return Err(HealthStatus::Unreachable),
        Err(boom_net::NetError::Io(io))
            if matches!(
                io.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) =>
        {
            return Err(HealthStatus::Timeout);
        }
        Err(_) => return Err(HealthStatus::Unreachable),
    };
    decode_frame_on::<HealthResponse>(&frame, &ProtocolLimits::v1(), Channel::Admin, Role::Client)
        .map_err(|_| HealthStatus::Unreachable)
}

fn cli_failure(
    config: &OperationalConfig,
    addr: SocketAddr,
    status: HealthStatus,
    error: &str,
) -> HealthCliOutcome {
    HealthCliOutcome {
        code: 2,
        body: json!({
            "status": status.name(),
            "scope": "runtime",
            "server_version": config.server_version,
            "wire_schema": config.wire_schema,
            "state_schema": config.state_schema,
            "rules_profile_id": config.rules_profile_id,
            "rules_hash": config.rules_hash,
            "content_sha256": hex(&config.content_sha256),
            "protocol_limits_version": config.protocol_limits_version,
            "admin_addr": addr.to_string(),
            "error": error,
            "diagnosis": if status == HealthStatus::Timeout { "timeout" } else { "unreachable" },
            "artifact": artifact_diagnostic(config),
            "config": fingerprint_json(config),
        }),
    }
}

fn artifact_diagnostic(config: &OperationalConfig) -> Value {
    let path = health_artifact_path(&config.data_dir);
    let Ok(body) = std::fs::read_to_string(&path) else {
        return json!({"path": path, "present": false});
    };
    let Ok(value) = serde_json::from_str::<Value>(&body) else {
        return json!({"path": path, "present": true, "parse_error": true});
    };
    let updated = value.get("updated_at_ms").and_then(Value::as_u64);
    let age = updated.map(|updated| now_ms().saturating_sub(updated));
    let stale = age.is_some_and(|age| age > config.heartbeat_interval_ms.saturating_mul(2));
    json!({
        "path": path,
        "present": true,
        "instance_id": value.get("instance_id"),
        "pid": value.get("pid"),
        "updated_at_ms": updated,
        "age_ms": age,
        "stale": stale,
    })
}

fn offline_health(config: &OperationalConfig) -> HealthCliOutcome {
    let db_configured = config.data_dir.join("boom-server.sqlite3").exists();
    let problems: Vec<String> = Vec::new();
    let status = "healthy";
    let code = 0;
    let fp = config.fingerprint();
    HealthCliOutcome {
        code,
        body: json!({
            "status": status,
            "scope": "offline",
            "runtime": "not_checked",
            "server_version": fp.server_version,
            "wire_schema": fp.wire_schema,
            "state_schema": fp.state_schema,
            "rules_profile_id": fp.rules_profile_id,
            "rules_hash": fp.rules_hash,
            "content_sha256": fp.content_sha256_hex(),
            "protocol_limits_version": fp.protocol_limits_version,
            "db": if db_configured { "configured" } else { "not_configured" },
            "db_ok": db_configured,
            "config": fingerprint_json(config),
            "problems": problems,
        }),
    }
}

fn fingerprint_json(config: &OperationalConfig) -> Value {
    let fp = config.fingerprint();
    json!({
        "server_version": fp.server_version,
        "wire_schema": fp.wire_schema,
        "state_schema": fp.state_schema,
        "rules_profile_id": fp.rules_profile_id,
        "rules_hash": fp.rules_hash,
        "content_sha256": fp.content_sha256_hex(),
        "protocol_limits_version": fp.protocol_limits_version,
    })
}

fn runtime_json(config: &OperationalConfig, response: &HealthResponse) -> Value {
    let fp = config.fingerprint();
    json!({
        "status": response.status.name(),
        "scope": "runtime",
        "server_version": fp.server_version,
        "wire_schema": fp.wire_schema,
        "state_schema": fp.state_schema,
        "protocol_limits_version": fp.protocol_limits_version,
        "uptime_ms": response.uptime_ms,
        "started_at_ms": response.started_at_ms,
        "instance_id": response.instance_id,
        "listen_addrs": response.listen_addrs,
        "admin_addr": response.admin_addr,
        "last_error_ts": response.last_error_ts,
        "db": if response.integrity_check == "not_configured" { "not_configured" } else { "configured" },
        "db_ok": response.db_ok,
        "schema_version": response.schema_version,
        "expected_schema_version": response.expected_schema_version,
        "integrity_check": response.integrity_check,
        "rules_profile_id": response.rules_profile_id,
        "rules_hash": response.rules_hash,
        "content_sha256": hex(&response.content_sha256),
        "accounts": response.accounts,
        "active_sessions": response.active_sessions,
        "problems": response.problems,
        "config": fingerprint_json(config),
    })
}

fn admin_addr_for_cli(config: &OperationalConfig) -> Result<SocketAddr, String> {
    let artifact = config.data_dir.join("health.json");
    if !artifact.exists() {
        return Ok(config.admin_addr);
    }
    let body = std::fs::read_to_string(&artifact)
        .map_err(|e| format!("không đọc được health.json: {e}"))?;
    let value: Value = serde_json::from_str(&body).map_err(|e| format!("health.json hỏng: {e}"))?;
    let addr = value
        .get("admin_addr")
        .and_then(Value::as_str)
        .ok_or_else(|| "health.json thiếu admin_addr".to_owned())?;
    addr.parse()
        .map_err(|_| format!("health.json có admin_addr không hợp lệ: {addr}"))
}

/// Kiểm peer có loopback; helper công khai để test không cần bind ngoài loopback.
pub fn peer_is_loopback(peer: SocketAddr) -> bool {
    peer.ip().is_loopback()
}

/// Đường dẫn artifact cho test/CLI.
pub fn health_artifact_path(data_dir: &Path) -> PathBuf {
    data_dir.join("health.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;

    use boom_proto::dto::handshake::Hello;
    use boom_proto::kind::Channel;
    use boom_proto::message::decode_frame_on;
    use serde_json::Value;

    use crate::config::OperationalConfig;
    use crate::observability::{Logger, SharedLogBuffer, new_instance_id, now_ms};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "boom-server-{tag}-{}-{}",
            std::process::id(),
            now_ms()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn config(tag: &str) -> OperationalConfig {
        OperationalConfig {
            admin_addr: "127.0.0.1:0".parse().unwrap(),
            data_dir: temp_dir(tag),
            server_version: "test-build".to_owned(),
            rules_profile_id: "test-rules".to_owned(),
            rules_hash: 0x1234,
            content_sha256: [7; 32],
            heartbeat_interval_ms: 20,
            health_timeout_ms: 200,
            ..OperationalConfig::default()
        }
    }

    fn logger(config: &OperationalConfig) -> (Arc<Logger>, SharedLogBuffer) {
        let buffer = SharedLogBuffer::new();
        let logger = Logger::new(
            config.fingerprint(),
            new_instance_id(),
            config.content_epoch,
            Box::new(buffer.clone()),
        );
        (Arc::new(logger), buffer)
    }

    fn read_response(stream: &mut TcpStream) -> HealthResponse {
        let frame = read_frame(stream, FrameLimits::V1).unwrap().unwrap();
        decode_frame_on::<HealthResponse>(
            &frame,
            &ProtocolLimits::v1(),
            Channel::Admin,
            Role::Client,
        )
        .unwrap()
    }

    #[test]
    fn health_state_is_healthy_then_unhealthy_with_db_problem() {
        let config = config("state");
        let state = HealthState::new(config, "boot".into(), 1_000);
        let healthy = state.response(2_500);
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.uptime_ms, 1_500);
        assert!(healthy.problems.is_empty());
        assert!(healthy.db_ok);

        state.set_db(DbHealth {
            configured: true,
            ok: false,
            schema_version: 2,
            expected_schema_version: 1,
            integrity_check: "broken".into(),
            problems: vec!["forced".into()],
        });
        let unhealthy = state.response(2_500);
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert!(!unhealthy.problems.is_empty());
        assert!(unhealthy.uptime_ms > 0);
    }

    #[test]
    fn admin_listener_accepts_health_request_only() {
        let config = config("admin");
        let state = Arc::new(HealthState::new(config.clone(), "boot".into(), now_ms()));
        let (logger, logs) = logger(&config);
        let server = AdminServer::start(&config, state, logger).unwrap();
        std::thread::sleep(Duration::from_millis(20));

        let mut client = TcpStream::connect(server.local_addr()).unwrap();
        let _ = client.set_read_timeout(Some(Duration::from_millis(500)));
        let request = encode_frame(&HealthRequest {}, &ProtocolLimits::v1()).unwrap();
        write_frame(&mut client, &request, FrameLimits::V1).unwrap();
        let response = read_response(&mut client);
        assert_eq!(response.status, HealthStatus::Healthy);
        assert!(response.uptime_ms > 0 || response.started_at_ms > 0);
        assert_eq!(response.rules_profile_id, "test-rules");
        assert_eq!(response.content_sha256, [7; 32]);

        // Kind game trên admin port bị đóng, không trả lời.
        let mut bad = TcpStream::connect(server.local_addr()).unwrap();
        let _ = bad.set_read_timeout(Some(Duration::from_millis(500)));
        let hello = Hello {
            wire_schema: 1,
            state_schema: 1,
            rules_profile_id: "test-rules".into(),
            rules_hash: 7,
            content_sha256: [7; 32],
            client_build: "test".into(),
            capabilities: vec![],
        };
        let frame = encode_frame(&hello, &ProtocolLimits::v1()).unwrap();
        write_frame(&mut bad, &frame, FrameLimits::V1).unwrap();
        assert!(read_frame(&mut bad, FrameLimits::V1).unwrap().is_none());
        std::thread::sleep(Duration::from_millis(30));
        let lines = logs.lines();
        assert!(
            lines
                .iter()
                .any(|line| line["event"] == "server.health_rejected")
        );
        assert!(lines.iter().any(|line| line["event"] == "server.health"));
    }

    #[test]
    fn non_loopback_admin_config_is_rejected_and_peer_is_logged() {
        let mut config = config("nonloop");
        config.admin_addr = "0.0.0.0:0".parse().unwrap();
        let state = Arc::new(HealthState::new(config.clone(), "boot".into(), now_ms()));
        let (logger, logs) = logger(&config);
        assert!(AdminServer::start(&config, state.clone(), logger.clone()).is_err());

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let mut client = TcpStream::connect(addr).unwrap();
        let (stream, _) = listener.accept().unwrap();
        handle_admin_connection(stream, "192.0.2.1:1234".parse().unwrap(), &state, &logger);
        let mut bytes = [0u8; 1];
        let read = client.read(&mut bytes).unwrap_or(0);
        assert_eq!(read, 0);
        std::thread::sleep(Duration::from_millis(20));
        assert!(
            logs.lines()
                .iter()
                .any(|line| line["event"] == "server.health_rejected"
                    && line["reason"] == "non_loopback")
        );
    }

    #[test]
    fn heartbeat_writes_atomic_health_json() {
        let config = config("heartbeat");
        let state = Arc::new(HealthState::new(
            config.clone(),
            "boot-heartbeat".into(),
            now_ms(),
        ));
        let (logger, _) = logger(&config);
        let heartbeat = Heartbeat::start(state.clone(), logger, 20).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        let path = health_artifact_path(&config.data_dir);
        let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["instance_id"], "boot-heartbeat");
        assert_eq!(value["status"], "healthy");
        assert!(value["admin_addr"].is_string());
        assert!(value["pid"].is_u64());
        drop(heartbeat);
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }

    #[test]
    fn cli_runtime_unhealthy_is_exit_one() {
        let config = config("cli-unhealthy");
        let state = Arc::new(HealthState::new(
            config.clone(),
            "boot-unhealthy".into(),
            now_ms(),
        ));
        state.set_db(DbHealth {
            configured: true,
            ok: false,
            schema_version: 2,
            expected_schema_version: 1,
            integrity_check: "broken".into(),
            problems: vec!["forced_db_failure".into()],
        });
        let (logger, _) = logger(&config);
        let server = AdminServer::start(&config, state, logger).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let mut live = config.clone();
        live.admin_addr = server.local_addr();
        let outcome = run_health_cli(&live, false);
        assert_eq!(outcome.code, 1);
        assert_eq!(outcome.body["status"], "unhealthy");
        assert!(!outcome.body["problems"].as_array().unwrap().is_empty());
        assert!(outcome.body["uptime_ms"].as_u64().unwrap() > 0);
        assert_eq!(outcome.body["server_version"], "test-build");
        assert_eq!(outcome.body["wire_schema"], 1);
    }

    #[test]
    fn cli_runtime_healthy_unreachable_timeout_and_offline() {
        let config = config("cli");
        let state = Arc::new(HealthState::new(
            config.clone(),
            "boot-cli".into(),
            now_ms(),
        ));
        let (logger, _) = logger(&config);
        let server = AdminServer::start(&config, state, logger).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let mut live = config.clone();
        live.admin_addr = server.local_addr();
        let healthy = run_health_cli(&live, false);
        assert_eq!(healthy.code, 0);
        assert_eq!(healthy.body["status"], "healthy");
        assert_eq!(healthy.body["scope"], "runtime");
        assert_eq!(healthy.body["config"]["rules_profile_id"], "test-rules");
        drop(server);

        // Cổng trống đã đóng → connection refused, exit 2.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_addr = listener.local_addr().unwrap();
        drop(listener);
        let mut dead = config.clone();
        dead.admin_addr = dead_addr;
        dead.health_timeout_ms = 100;
        let unreachable = run_health_cli(&dead, false);
        assert_eq!(unreachable.code, 2);
        assert_eq!(unreachable.body["status"], "unreachable");

        // Listener nhận nhưng không trả lời → timeout, exit 2.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let timeout_addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(500));
        });
        let mut timeout = config.clone();
        timeout.admin_addr = timeout_addr;
        timeout.health_timeout_ms = 50;
        let begin = std::time::Instant::now();
        let timed = run_health_cli(&timeout, false);
        assert!(begin.elapsed() < Duration::from_secs(1));
        assert_eq!(timed.code, 2);
        assert_eq!(timed.body["status"], "timeout");

        let offline = run_health_cli(&config, true);
        assert_eq!(offline.code, 0);
        assert_eq!(offline.body["scope"], "offline");
        assert_eq!(offline.body["runtime"], "not_checked");
        assert_eq!(offline.body["db"], "not_configured");
    }
    #[test]
    fn admin_falls_back_to_free_port_when_configured_port_is_busy() {
        let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut config = config("busy");
        config.admin_addr = occupied.local_addr().unwrap();
        let state = Arc::new(HealthState::new(
            config.clone(),
            "boot-busy".into(),
            now_ms(),
        ));
        let (logger, _) = logger(&config);
        let server = AdminServer::start(&config, state.clone(), logger).unwrap();
        assert_ne!(server.local_addr(), config.admin_addr);
        assert_eq!(
            server.local_addr().ip(),
            std::net::IpAddr::from([127, 0, 0, 1])
        );
        assert_eq!(state.admin_addr(), server.local_addr().to_string());
        drop(server);
        let _ = std::fs::remove_dir_all(&config.data_dir);
    }
}
