//! Vòng đời phiên của `boom-server` (ADR 0011 §6, §6.1, §10).
//!
//! Module này chứa **logic** vòng đời phiên; SQL nằm ở `identity/sqlite.rs` và được gọi qua
//! trait [`SessionStore`] — ranh giới đúng như ADR 0011 §10 ("`identity/session.rs`: vòng đời
//! phiên …", "`identity/sqlite.rs`: hiện thực `rusqlite`").
//!
//! # Quy tắc bắt buộc (ADR 0011 §6.1)
//!
//! 1. **Mọi** đường teardown của connection chỉ revoke đúng `session_id` mà connection đó đã
//!    auth, với điều kiện `revoked_at_ms IS NULL`. **Không** dùng `WHERE account_id = ?`:
//!    nếu không, race "connection cũ chết *sau* khi connection mới đã đăng nhập" sẽ revoke
//!    nhầm phiên mới (ca `T4-A5`).
//! 2. Đăng nhập chạy trong **một** transaction (`BEGIN IMMEDIATE`): dọn phiên đã hết TTL
//!    **của tài khoản đó** rồi `INSERT` phiên mới. Nếu `INSERT` vi phạm unique partial index
//!    ⇒ rollback ⇒ `account_has_active_session`.
//! 3. Cleanup lúc boot lọc theo `instance_id` (`instance_id <> <instance hiện tại>`), **không**
//!    lọc theo `account_id`.
//! 4. Job quét định kỳ dọn theo `expires_at_ms` và theo `instance_id` đã chết; không bao giờ
//!    quét theo `account_id` trần.
//!
//! Token phiên là **bearer credential**: 32 byte (256 bit) từ RNG của hệ điều hành, mã hóa
//! base64url không padding (43 ký tự). DB chỉ lưu SHA-256(token thô) — không phục hồi được
//! token từ DB, và token thô không nằm ở đâu ngoài RAM của tiến trình (ADR 0011 §6).

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use super::AccountStore;
use super::{AccountId, NewSession, Secret, Session, SessionId, StoreError};

/// Số byte token thô (256 bit) — ADR 0011 §6.
pub const TOKEN_BYTES: usize = 32;

/// Mã lý do giải phóng phiên (ADR 0011 §6.1, §8.1).
///
/// Tập giá trị này là **design** của dự án, không phải luật chơi; ADR 0011 §8.1 liệt kê đúng
/// các mã này cho trường `revoke_reason` trong log.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RevokeReason {
    /// Client đăng xuất (`Logout`).
    Logout,
    /// Connection chết **đã xác nhận** (EOF, reset, lỗi ghi).
    Disconnect,
    /// Connection im lặng: quá `SESSION_LIVENESS_TIMEOUT_MS` không có `Pong`.
    TimeoutNoInput,
    /// Hết TTL.
    Expired,
    /// Server khởi động (boot mới).
    ServerRestart,
    /// Server tắt có trật tự (SIGTERM).
    ServerShutdown,
    /// Lệnh quản trị `revoke-sessions <username>`.
    AdminRevoke,
}

impl RevokeReason {
    /// Mã lý do lưu trong DB và ghi vào log (ADR 0011 §6.1, §8.1).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Logout => "logout",
            Self::Disconnect => "disconnect",
            Self::TimeoutNoInput => "timeout_no_input",
            Self::Expired => "expired",
            Self::ServerRestart => "server_restart",
            Self::ServerShutdown => "server_shutdown",
            Self::AdminRevoke => "admin_revoke",
        }
    }
}

impl fmt::Display for RevokeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Lỗi nguồn entropy (ADR 0011 §6).
///
/// Lỗi entropy ⇒ **không** fallback RNG yếu, **không** phát token; server trả thông báo lỗi
/// thống nhất và log `auth.rng_unavailable` kèm mã lỗi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RngError {
    /// Mô tả lỗi để ghi log; **không** trả cho client.
    pub message: String,
}

impl fmt::Display for RngError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Nguồn byte ngẫu nhiên.
///
/// Mặc định là RNG của hệ điều hành ([`OsRandomSource`]). Trait này cho phép **tiêm** lỗi
/// entropy để kiểm chứng "lỗi OS RNG ⇒ không phát token" (ADR 0011 §6, §11 T3).
pub trait RandomSource: Send {
    /// Điền `dest` bằng byte ngẫu nhiên.
    fn fill(&mut self, dest: &mut [u8]) -> Result<(), RngError>;
}

/// RNG của hệ điều hành (`getrandom`) — ADR 0011 §6.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsRandomSource;

impl RandomSource for OsRandomSource {
    fn fill(&mut self, dest: &mut [u8]) -> Result<(), RngError> {
        getrandom::fill(dest).map_err(|err| RngError {
            message: err.to_string(),
        })
    }
}

/// Lỗi của thao tác phiên.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// Lỗi entropy: không phát token, không fallback (ADR 0011 §6).
    Rng(RngError),
    /// Lỗi tầng lưu trữ.
    Store(StoreError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rng(err) => write!(f, "lỗi entropy: {err}"),
            Self::Store(err) => write!(f, "lỗi tầng lưu trữ: {err}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<RngError> for SessionError {
    fn from(value: RngError) -> Self {
        Self::Rng(value)
    }
}

impl From<StoreError> for SessionError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

/// Cổng lưu phiên (ADR 0011 §4, §6.1). Hiện thực miền, không rò SQL ra ngoài module.
pub trait SessionStore {
    /// Tạo phiên mới cho tài khoản trong **một** transaction (ADR 0011 §6.1 bước 2).
    ///
    /// Transaction gồm: (a) revoke các phiên đã hết TTL **của tài khoản đó** (reason
    /// `expired`), (b) `INSERT` phiên mới. Nếu (b) vi phạm unique partial index
    /// `sessions_one_active_per_account` ⇒ rollback ⇒ [`StoreError::AccountHasActiveSession`].
    ///
    /// `session.created_at_ms` là mốc "hiện tại" dùng để dọn TTL — nhất quán với thời điểm
    /// tạo phiên nên không có cửa sổ cleanup chạm phiên vừa tạo.
    fn create_session(&mut self, session: NewSession) -> Result<Session, StoreError>;

    /// Đọc phiên theo định danh.
    fn session_by_id(&self, id: SessionId) -> Result<Option<Session>, StoreError>;

    /// Đọc phiên theo SHA-256(token thô).
    ///
    /// Chỉ trả phiên **còn giữ chỗ** (`revoked_at_ms IS NULL`): token đã thu hồi (logout,
    /// disconnect, timeout…) không tra được phiên nào nữa (ADR 0011 §6).
    fn session_by_token_hash(&self, token_hash: &[u8]) -> Result<Option<Session>, StoreError>;

    /// Phiên đang **giữ chỗ** (`revoked_at_ms IS NULL`) của tài khoản.
    fn active_session(&self, account_id: AccountId) -> Result<Option<Session>, StoreError>;

    /// Thu hồi đúng `session_id` còn giữ chỗ (ADR 0011 §6.1 bước 1).
    ///
    /// Trả `true` nếu có hàng bị đổi. Điều kiện `revoked_at_ms IS NULL` làm thao tác
    /// idempotent và **không** chạm phiên mới (id khác).
    fn revoke_session(
        &mut self,
        id: SessionId,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<bool, StoreError>;

    /// Thu hồi mọi phiên đang giữ chỗ của một tài khoản (đường admin, ADR 0011 §6).
    fn revoke_account_sessions(
        &mut self,
        account_id: AccountId,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<usize, StoreError>;

    /// Thu hồi phiên đang giữ chỗ của **instance khác** (cleanup lúc boot, ADR 0011 §6.1 bước 3).
    fn revoke_other_instance_sessions(
        &mut self,
        current_instance_id: &str,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<usize, StoreError>;

    /// Thu hồi phiên đang giữ chỗ của **instance này** (server tắt có trật tự, ADR 0011 §6.1).
    fn revoke_instance_sessions(
        &mut self,
        instance_id: &str,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<usize, StoreError>;

    /// Quét và thu hồi phiên đã hết TTL (ADR 0011 §6.1 bước 4).
    fn sweep_expired_sessions(&mut self, now_ms: i64) -> Result<usize, StoreError>;

    /// Quét và thu hồi phiên của `instance_id` quá hạn liveness (ADR 0011 §6.1).
    ///
    /// Phiên bị coi là mất im lặng khi `now_ms - last_seen_at_ms >= liveness_timeout_ms`
    /// (tức là quá `SESSION_LIVENESS_TIMEOUT_MS` không có `Pong`).
    fn sweep_stale_sessions(
        &mut self,
        instance_id: &str,
        liveness_timeout_ms: i64,
        now_ms: i64,
    ) -> Result<usize, StoreError>;

    /// Cập nhật `last_seen_at_ms` (khi nhận `Pong`).
    fn touch_session(&mut self, id: SessionId, now_ms: i64) -> Result<bool, StoreError>;
}

/// SHA-256 của token thô — thứ DB lưu, **không** phải token (ADR 0011 §6).
pub fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Phát token phiên: 32 byte ngẫu nhiên mã hóa base64url không padding (ADR 0011 §6).
///
/// Lỗi entropy ⇒ lỗi, **không** fallback RNG yếu.
pub fn issue_token(random: &mut dyn RandomSource) -> Result<String, RngError> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    if let Err(err) = random.fill(&mut bytes) {
        // Xoá phần byte đã điền trước khi trả lỗi: không để lại entropy nào ngoài RAM.
        bytes.fill(0);
        return Err(err);
    }
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// Phiên vừa cấp: token thô chỉ nằm trong RAM của tiến trình (ADR 0011 §6).
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedSession {
    /// Hàng phiên đã lưu.
    pub session: Session,
    /// Token thô để trả cho client; server không lưu ở đâu khác.
    pub token: Secret<String>,
}

/// Dịch vụ vòng đời phiên của một instance tiến trình (ADR 0011 §6.1).
///
/// `instance_id` là boot id của tiến trình: sinh **một lần** lúc khởi động, ghi trong log
/// `server.start` và artifact `health.json`.
pub struct SessionService {
    instance_id: String,
}

impl SessionService {
    /// Tạo dịch vụ phiên cho instance `instance_id`.
    pub fn new(instance_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
        }
    }

    /// Boot id của instance đang chạy.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Đăng nhập: phát token mới rồi tạo phiên trong **một** transaction (ADR 0011 §6, §6.1).
    ///
    /// Trả [`StoreError::AccountHasActiveSession`] nếu tài khoản đang có phiên còn giữ chỗ —
    /// server **không** tự động đuổi phiên cũ (SCOPE_V1 §3).
    pub fn login<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        random: &mut dyn RandomSource,
        account_id: AccountId,
        connection_id: &str,
        ttl_ms: i64,
        now_ms: i64,
    ) -> Result<IssuedSession, SessionError> {
        let token = issue_token(random)?;
        let session = store.create_session(NewSession {
            account_id,
            token_hash: token_hash(&token),
            instance_id: self.instance_id.clone(),
            connection_id: Some(connection_id.to_string()),
            created_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
            last_seen_at_ms: now_ms,
        })?;
        Ok(IssuedSession {
            session,
            token: Secret::new(token),
        })
    }

    /// Client đăng xuất (ADR 0011 §6): token bị từ chối ngay lập tức.
    pub fn logout<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        session_id: SessionId,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        store.revoke_session(session_id, RevokeReason::Logout, now_ms)
    }

    /// Connection chết **đã xác nhận** (EOF, reset, lỗi ghi) — ADR 0011 §6.1.
    pub fn on_disconnect<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        session_id: SessionId,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        store.revoke_session(session_id, RevokeReason::Disconnect, now_ms)
    }

    /// Mất mạng im lặng: quá `liveness_timeout_ms` không có `Pong` — ADR 0011 §6.1.
    pub fn on_liveness_timeout<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        session_id: SessionId,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        store.revoke_session(session_id, RevokeReason::TimeoutNoInput, now_ms)
    }

    /// Quét phiên hết TTL (ADR 0011 §6.1 bước 4).
    pub fn sweep_expired<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        store.sweep_expired_sessions(now_ms)
    }

    /// Quét phiên của instance này quá hạn liveness (ADR 0011 §6.1).
    pub fn sweep_liveness<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        liveness_timeout_ms: i64,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        store.sweep_stale_sessions(&self.instance_id, liveness_timeout_ms, now_ms)
    }

    /// Cleanup lúc boot: revoke phiên của instance khác (ADR 0011 §6.1 bước 3).
    ///
    /// Bước này chạy **trước** khi server nhận auth, và chỉ chạy được vì đã có khóa file DB
    /// (ADR 0011 §4) — nếu không, nó sẽ giải phóng nhầm phiên của instance đang chạy.
    pub fn boot_cleanup<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        store.revoke_other_instance_sessions(&self.instance_id, RevokeReason::ServerRestart, now_ms)
    }

    /// Server tắt có trật tự (SIGTERM): revoke phiên của instance này (ADR 0011 §6.1).
    pub fn shutdown<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        store.revoke_instance_sessions(&self.instance_id, RevokeReason::ServerShutdown, now_ms)
    }

    /// Lệnh quản trị `boom-server account revoke-sessions <username>` (ADR 0011 §6).
    ///
    /// Đường **dự phòng**: người chơi bị đứt mạng hoặc server restart tự vào lại được mà không
    /// cần admin.
    pub fn revoke_account_sessions<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        account_id: AccountId,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        store.revoke_account_sessions(account_id, RevokeReason::AdminRevoke, now_ms)
    }

    /// Lệnh quản trị `boom-server account revoke-sessions <username>` (ADR 0011 §6).
    ///
    /// Đường **dự phòng**: người chơi bị đứt mạng hoặc server restart tự vào lại được mà không
    /// cần admin. Tài khoản không tồn tại ⇒ không thu hồi gì (không phải lỗi).
    pub fn revoke_sessions_by_username<S: AccountStore + SessionStore + ?Sized>(
        &self,
        store: &mut S,
        username: &str,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        let account = store.account_by_username(username)?;
        let Some(account) = account else {
            return Ok(0);
        };
        store.revoke_account_sessions(account.id, RevokeReason::AdminRevoke, now_ms)
    }

    /// Xác thực token thô: server bind identity từ token, **không** tin `account_id` do client
    /// khai (ADR 0005, ADR 0011 §6).
    ///
    /// Token đã thu hồi **hoặc** hết hạn không xác thực được: client phải đăng nhập lại
    /// (ADR 0011 §6 — "hết hạn thì client đăng nhập lại").
    pub fn authenticate<S: SessionStore + ?Sized>(
        &self,
        store: &S,
        token: &str,
        now_ms: i64,
    ) -> Result<Option<Session>, StoreError> {
        let session = store.session_by_token_hash(&token_hash(token))?;
        Ok(session.filter(|session| session.expires_at_ms > now_ms))
    }

    /// Cập nhật `last_seen_at_ms` khi nhận `Pong` (ADR 0011 §6.1).
    pub fn touch<S: SessionStore + ?Sized>(
        &self,
        store: &mut S,
        session_id: SessionId,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        store.touch_session(session_id, now_ms)
    }
}

impl fmt::Debug for SessionService {
    /// Chỉ in `instance_id` — không in token hay PHC.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionService")
            .field("instance_id", &self.instance_id)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{RevokeReason, TOKEN_BYTES, issue_token, token_hash};
    use crate::identity::{
        AccountStore, NewAccount, OsRandomSource, SessionService, SessionStore, SqliteAccountStore,
        SqliteOptions, StoreError,
    };
    use tempfile::TempDir;

    const NOW: i64 = 1_700_000_000_000;
    const TTL_MS: i64 = 24 * 60 * 60 * 1_000;
    const LIVENESS_TIMEOUT_MS: i64 = 30_000;

    /// Nguồn entropy lỗi — dùng để kiểm chứng "lỗi OS RNG ⇒ không phát token" (ADR 0011 §6).
    struct FailingRandom;

    impl super::RandomSource for FailingRandom {
        fn fill(&mut self, _dest: &mut [u8]) -> Result<(), super::RngError> {
            Err(super::RngError {
                message: String::from("entropy hỏng (test)"),
            })
        }
    }

    /// Nguồn entropy giả: byte tăng dần, đủ xác định cho test.
    struct CountingRandom {
        counter: u8,
    }

    impl super::RandomSource for CountingRandom {
        fn fill(&mut self, dest: &mut [u8]) -> Result<(), super::RngError> {
            for (index, byte) in dest.iter_mut().enumerate() {
                *byte = self.counter.wrapping_add(index as u8);
            }
            self.counter = self.counter.wrapping_add(1);
            Ok(())
        }
    }

    #[test]
    fn token_is_256_bit_base64url_without_padding() {
        let mut random = CountingRandom { counter: 0 };
        let token = issue_token(&mut random).unwrap();

        // 32 byte ⇒ 43 ký tự base64url không padding (ADR 0011 §6).
        assert_eq!(token.len(), 43);
        assert!(!token.contains('='));
        assert!(
            token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn token_differs_between_calls() {
        let mut random = CountingRandom { counter: 0 };
        let first = issue_token(&mut random).unwrap();
        let second = issue_token(&mut random).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn rng_failure_gives_no_token() {
        let mut random = FailingRandom;
        let err = issue_token(&mut random).unwrap_err();
        assert_eq!(err.message, "entropy hỏng (test)");
    }

    #[test]
    fn token_hash_is_sha256_of_raw_token() {
        let mut random = CountingRandom { counter: 0 };
        let token = issue_token(&mut random).unwrap();
        let hash = token_hash(&token);
        assert_eq!(hash.len(), 32);
        // Hash không chứa token thô.
        assert!(!hash.windows(token.len()).any(|w| w == token.as_bytes()));
        // Cùng token ⇒ cùng hash.
        assert_eq!(hash, token_hash(&token));
    }

    #[test]
    fn revoke_reason_codes_match_adr_0011() {
        assert_eq!(RevokeReason::Logout.as_str(), "logout");
        assert_eq!(RevokeReason::Disconnect.as_str(), "disconnect");
        assert_eq!(RevokeReason::TimeoutNoInput.as_str(), "timeout_no_input");
        assert_eq!(RevokeReason::Expired.as_str(), "expired");
        assert_eq!(RevokeReason::ServerRestart.as_str(), "server_restart");
        assert_eq!(RevokeReason::ServerShutdown.as_str(), "server_shutdown");
        assert_eq!(RevokeReason::AdminRevoke.as_str(), "admin_revoke");
    }

    #[test]
    fn token_byte_count_is_256_bit() {
        assert_eq!(TOKEN_BYTES, 32);
    }

    // --- Ca nghiệm thu ADR 0011 §11 (T4-A1…T4-A7) ----------------------------------------

    fn db_path(dir: &TempDir) -> std::path::PathBuf {
        dir.path().join("boom-server.sqlite")
    }

    fn open(dir: &TempDir) -> SqliteAccountStore {
        SqliteAccountStore::open(&db_path(dir), SqliteOptions::default()).expect("mở store")
    }

    fn account(store: &mut SqliteAccountStore, username: &str) -> crate::identity::AccountId {
        store
            .create_account(NewAccount {
                username: username.to_string(),
                password_hash: crate::identity::Secret::new(String::from(
                    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA",
                )),
                created_at_ms: NOW,
            })
            .expect("tạo tài khoản")
            .id
    }

    fn login(
        service: &SessionService,
        store: &mut SqliteAccountStore,
        account_id: crate::identity::AccountId,
        connection_id: &str,
        now_ms: i64,
    ) -> super::IssuedSession {
        service
            .login(
                store,
                &mut OsRandomSource,
                account_id,
                connection_id,
                TTL_MS,
                now_ms,
            )
            .expect("đăng nhập")
    }

    /// `T4-A1` — disconnect **đã xác nhận**: phiên bị thu hồi với reason `disconnect`, và
    /// đăng nhập lại **thành công**, không cần admin.
    #[test]
    fn t4_a1_confirmed_disconnect_releases_session() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");

        let issued = login(&service, &mut store, account_id, "conn-1", NOW);
        assert_eq!(
            store.active_session_count(account_id).unwrap(),
            1,
            "phiên mới phải giữ chỗ"
        );

        assert!(
            service
                .on_disconnect(&mut store, issued.session.id, NOW)
                .unwrap(),
            "phải thu hồi được phiên"
        );
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoked_at_ms, Some(NOW));
        assert_eq!(session.revoke_reason.as_deref(), Some("disconnect"));
        // Connection đã chết ⇒ `connection_id` được NULL (ADR 0011 §4).
        assert_eq!(session.connection_id, None);
        // Không còn giữ chỗ ⇒ đăng nhập lại được, không cần admin.
        assert_eq!(store.active_session_count(account_id).unwrap(), 0);
        login(&service, &mut store, account_id, "conn-2", NOW);
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
    }

    /// `T4-A2` — **restart server**: lúc boot, mọi phiên của instance cũ bị thu hồi với
    /// reason `server_restart`; đăng nhập lại **thành công**; `accounts`/`password_hash` còn
    /// nguyên (restart server giữ identity).
    #[test]
    fn t4_a2_server_restart_releases_old_instance_sessions() {
        let dir = TempDir::new().unwrap();
        let account_id;
        let password_hash;
        let old_session_id;
        {
            let mut store = open(&dir);
            let service = SessionService::new("instance-1");
            account_id = account(&mut store, "bomber");
            password_hash = store
                .account_by_id(account_id)
                .unwrap()
                .unwrap()
                .password_hash
                .expose()
                .to_string();
            let issued = login(&service, &mut store, account_id, "conn-1", NOW);
            old_session_id = issued.session.id;
        }
        // Instance cũ chết ⇒ instance mới mở DB (đã có khóa file ⇒ không tranh chấp).
        let mut store = open(&dir);
        let service = SessionService::new("instance-2");
        let revoked = service.boot_cleanup(&mut store, NOW).unwrap();
        assert_eq!(revoked, 1, "phải thu hồi phiên của instance cũ");

        let session = store.session_by_id(old_session_id).unwrap().unwrap();
        assert_eq!(session.revoked_at_ms, Some(NOW));
        assert_eq!(session.revoke_reason.as_deref(), Some("server_restart"));
        assert_eq!(store.active_session_count(account_id).unwrap(), 0);

        // Identity còn nguyên sau restart.
        assert_eq!(
            store
                .account_by_id(account_id)
                .unwrap()
                .unwrap()
                .password_hash
                .expose(),
            password_hash.as_str()
        );
        // Đăng nhập lại thành công.
        login(&service, &mut store, account_id, "conn-2", NOW);
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
    }

    /// `T4-A3` — **mất mạng im lặng**: quá `SESSION_LIVENESS_TIMEOUT_MS` không có `Pong` ⇒
    /// phiên bị giải phóng với reason `timeout_no_input`; đăng nhập lại được.
    ///
    /// Đồng hồ **tiêm vào**: không sleep thật.
    #[test]
    fn t4_a3_silent_network_loss_times_out_session() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        // Chưa quá ngưỡng ⇒ chưa thu hồi.
        let revoked = service
            .sweep_liveness(
                &mut store,
                LIVENESS_TIMEOUT_MS,
                NOW + LIVENESS_TIMEOUT_MS - 1,
            )
            .unwrap();
        assert_eq!(revoked, 0);
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);

        // Quá ngưỡng (đồng hồ tiêm, không sleep) ⇒ thu hồi.
        let revoked = service
            .sweep_liveness(&mut store, LIVENESS_TIMEOUT_MS, NOW + LIVENESS_TIMEOUT_MS)
            .unwrap();
        assert_eq!(revoked, 1);
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoked_at_ms, Some(NOW + LIVENESS_TIMEOUT_MS));
        assert_eq!(session.revoke_reason.as_deref(), Some("timeout_no_input"));
        // Trần thời gian khóa tài khoản là timeout này, không phải TTL 24 h.
        assert_eq!(store.active_session_count(account_id).unwrap(), 0);
        login(
            &service,
            &mut store,
            account_id,
            "conn-2",
            NOW + LIVENESS_TIMEOUT_MS,
        );
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
    }

    /// `T4-A4` — **hai lần đăng nhập**: lần thứ hai bị từ chối với
    /// `account_has_active_session`; trong DB chỉ có một hàng `revoked_at_ms IS NULL` và
    /// phiên thắng vẫn hiệu lực.
    ///
    /// **Giới hạn của ca này:** hai lời gọi ở đây là **tuần tự**, nên đây **không phải** race
    /// thật. Ca `T4-A4` *đồng thời thật* (hai thread + barrier trên hai connection độc lập
    /// tới cùng file DB) nằm ở
    /// `identity::sqlite::tests::t4_a4_concurrent_logins_only_one_wins` — cùng một đường
    /// `INSERT` trong transaction `BEGIN IMMEDIATE` nên ràng buộc được kiểm ở cả hai nơi.
    #[test]
    fn t4_a4_sequential_logins_only_one_wins() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");

        let first = login(&service, &mut store, account_id, "conn-1", NOW);
        let second = service.login(
            &mut store,
            &mut OsRandomSource,
            account_id,
            "conn-2",
            TTL_MS,
            NOW,
        );

        assert_eq!(
            second.unwrap_err(),
            super::SessionError::Store(StoreError::AccountHasActiveSession)
        );
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
        // Phiên thắng vẫn hiệu lực.
        assert_eq!(
            store
                .session_by_id(first.session.id)
                .unwrap()
                .unwrap()
                .revoked_at_ms,
            None
        );
    }

    /// `T4-A5` — **race teardown**: connection cũ chết *sau* khi connection mới đã đăng nhập
    /// thành công ⇒ phiên mới **vẫn** hiệu lực (regression cho lỗi "revoke theo `account_id`").
    #[test]
    fn t4_a5_old_connection_teardown_does_not_revoke_new_session() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");

        let old = login(&service, &mut store, account_id, "conn-1", NOW);
        // Phiên cũ được giải phóng (ví dụ timeout liveness) ⇒ chỗ trống.
        service
            .on_liveness_timeout(&mut store, old.session.id, NOW + 1)
            .unwrap();
        // Connection mới đăng nhập thành công.
        let new = login(&service, &mut store, account_id, "conn-2", NOW + 2);
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);

        // Teardown của connection cũ đến muộn: chỉ chạm `session_id` cũ.
        let changed = service
            .on_disconnect(&mut store, old.session.id, NOW + 3)
            .unwrap();
        assert!(
            !changed,
            "phiên cũ đã bị thu hồi nên thao tác phải idempotent"
        );
        // Phiên mới vẫn hiệu lực.
        let session = store.session_by_id(new.session.id).unwrap().unwrap();
        assert_eq!(session.revoked_at_ms, None);
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
    }

    /// `T4-A6` — phiên còn sống, đăng nhập thứ hai bị từ chối rõ ràng (SCOPE_V1 §3).
    #[test]
    fn t4_a6_second_login_while_session_alive_is_rejected() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        login(&service, &mut store, account_id, "conn-1", NOW);

        let err = service
            .login(
                &mut store,
                &mut OsRandomSource,
                account_id,
                "conn-2",
                TTL_MS,
                NOW,
            )
            .unwrap_err();
        assert_eq!(
            err,
            super::SessionError::Store(StoreError::AccountHasActiveSession)
        );
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
    }

    /// `T4-A7` — khởi động server thứ hai khi instance cũ còn giữ khóa file ⇒ **từ chối
    /// khởi động**; phiên của instance cũ **không** bị đụng.
    #[test]
    fn t4_a7_second_instance_is_refused_and_leaves_sessions_untouched() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        // Instance thứ hai không mở được DB ⇒ từ chối khởi động (exit code 2 ở tầng CLI).
        let err = SqliteAccountStore::open(&db_path(&dir), SqliteOptions::default()).unwrap_err();
        assert_eq!(err, StoreError::DatabaseLocked);

        // Phiên của instance cũ không bị đụng.
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoked_at_ms, None);
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
    }

    /// Đường admin `revoke-sessions <username>`: thu hồi mọi phiên hiệu lực của tài khoản
    /// (ADR 0011 §6).
    #[test]
    fn admin_revoke_sessions_releases_every_active_session() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        let revoked = service
            .revoke_account_sessions(&mut store, account_id, NOW)
            .unwrap();
        assert_eq!(revoked, 1);
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoke_reason.as_deref(), Some("admin_revoke"));
        assert_eq!(store.active_session_count(account_id).unwrap(), 0);
    }

    /// `revoke-sessions <username>`: tra theo **tên hiển thị**, tài khoản không tồn tại ⇒ không
    /// thu hồi gì, không phải lỗi.
    #[test]
    fn admin_revoke_sessions_by_username_releases_active_session() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        let revoked = service
            .revoke_sessions_by_username(&mut store, "bomber", NOW)
            .unwrap();
        assert_eq!(revoked, 1);
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoke_reason.as_deref(), Some("admin_revoke"));

        // Tài khoản không tồn tại ⇒ 0 phiên bị thu hồi, không lỗi.
        assert_eq!(
            service
                .revoke_sessions_by_username(&mut store, "khongcogi", NOW)
                .unwrap(),
            0
        );
    }

    /// Logout: token bị từ chối ngay lập tức (ADR 0011 §6).
    #[test]
    fn logout_revokes_session_and_invalidates_token() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        // Token hợp lệ trước khi logout.
        assert!(
            service
                .authenticate(&store, issued.token.expose(), NOW)
                .unwrap()
                .is_some()
        );

        service.logout(&mut store, issued.session.id, NOW).unwrap();
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoke_reason.as_deref(), Some("logout"));
        // Token không còn tra được phiên nào.
        assert!(
            service
                .authenticate(&store, issued.token.expose(), NOW)
                .unwrap()
                .is_none()
        );
    }

    /// Token hết hạn (TTL) không xác thực được: client phải đăng nhập lại (ADR 0011 §6).
    #[test]
    fn expired_token_does_not_authenticate() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        assert!(
            service
                .authenticate(&store, issued.token.expose(), NOW + TTL_MS - 1)
                .unwrap()
                .is_some()
        );
        assert!(
            service
                .authenticate(&store, issued.token.expose(), NOW + TTL_MS)
                .unwrap()
                .is_none()
        );
    }

    /// Job quét định kỳ dọn phiên hết TTL (ADR 0011 §6.1 bước 4).
    #[test]
    fn expired_sessions_are_swept() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        let revoked = service.sweep_expired(&mut store, NOW + TTL_MS + 1).unwrap();
        assert_eq!(revoked, 1);
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoke_reason.as_deref(), Some("expired"));
        assert_eq!(store.active_session_count(account_id).unwrap(), 0);
    }

    /// Server tắt có trật tự: thu hồi phiên của instance này (ADR 0011 §6.1).
    #[test]
    fn shutdown_revokes_sessions_of_this_instance() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        let revoked = service.shutdown(&mut store, NOW).unwrap();
        assert_eq!(revoked, 1);
        let session = store.session_by_id(issued.session.id).unwrap().unwrap();
        assert_eq!(session.revoke_reason.as_deref(), Some("server_shutdown"));
    }

    /// `Ping`/`Pong` giữ phiên sống: `touch` cập nhật `last_seen_at_ms` nên phiên không bị coi
    /// là mất im lặng (ADR 0011 §6.1).
    #[test]
    fn pong_keeps_session_alive() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let service = SessionService::new("instance-1");
        let account_id = account(&mut store, "bomber");
        let issued = login(&service, &mut store, account_id, "conn-1", NOW);

        service
            .touch(&mut store, issued.session.id, NOW + LIVENESS_TIMEOUT_MS - 1)
            .unwrap();
        let revoked = service
            .sweep_liveness(&mut store, LIVENESS_TIMEOUT_MS, NOW + LIVENESS_TIMEOUT_MS)
            .unwrap();
        assert_eq!(revoked, 0, "phiên đã trả lời Pong thì không bị thu hồi");
        assert_eq!(store.active_session_count(account_id).unwrap(), 1);
    }
}
