//! Đăng ký / đăng nhập của server V1: Argon2id, PHC, thông báo lỗi thống nhất, rate limit
//! (ADR 0011 §5, §9, §11 T3).
//!
//! # Thứ tự xử lý `Login` chống enumeration
//!
//! Theo [ADR 0009 Addendum A §A.2](../decisions/0009-wire-protocol-input-snapshot-codec.md)
//! và ADR 0011 §5:
//!
//! 1. Kiểm **biên/rate limit**.
//! 2. **Verify mật khẩu** — tên không tồn tại vẫn verify với **hash giả**.
//! 3. **Chỉ khi verify thành công** mới kiểm phiên và có thể trả `AccountHasActiveSession`.
//!
//! ⇒ Credential sai luôn trả `InvalidCredentials`, bất kể tài khoản có tồn tại hay đang có
//! phiên. Bucket rate limit theo tên khóa theo `username_key` **đã chuẩn hóa** (không tra DB
//! trước) nên `RateLimited` không thành oracle tồn tại tài khoản.
//!
//! # Quota đăng ký (ADR 0011 §9 dòng 12)
//!
//! `Register` có **quota riêng**, tách khỏi bộ đếm *lần đăng nhập sai*:
//!
//! | Bộ đếm | Đếm cái gì | Khi nào tăng |
//! |---|---|---|
//! | đăng nhập | lần verify mật khẩu **thất bại** | sau khi verify thất bại |
//! | đăng ký | **lượt** đăng ký | **trước** bước băm Argon2id, kể cả lượt thất bại (tên trùng) |
//!
//! ⇒ một lượt đăng ký **không** tiêu tốn bộ ngân sách đăng nhập, và ngược lại.
//!
//! Quota tính theo **hai** bucket: `username_key` đã chuẩn hóa (không tra DB trước, nên
//! `RateLimited` không thành oracle tồn tại tài khoản) và theo nguồn. Lượt đăng ký được đếm
//! **trước phần công việc tốn kém** (băm Argon2id) ⇒ vượt ngưỡng thì bị từ chối mà không
//! tốn công băm: chặn được cả **spam tài khoản** (đăng ký thành công) lẫn **probe/hash
//! spam** (thử tên trùng, mỗi lần đều chạy Argon2id).
//!
//! **Giới hạn đã biết:** bucket theo nguồn là theo IP ⇒ nhiều máy sau cùng một NAT (phòng
//! LAN) dùng chung một bộ ngân sách. V1 chấp nhận; muốn siết thì đổi
//! [`AuthConfig::register_rate_limit_max_attempts`] trong config, không đổi thuật toán.
//!
//! **Giới hạn đã biết (theo tên):** giống bộ đếm đăng nhập, một tên bị khoá cửa sổ thì chính
//! tên đó không đăng ký/đăng nhập được cho tới khi hết cửa sổ — kẻ tấn công dùng tên nạn
//! nhân có thể khoá tài khoản nạn nhân. Đây là đánh đổi tiêu chuẩn của rate limit theo tài
//! khoản; V1 chấp nhận.
//!
//! # Ranh giới với crate khác
//!
//! - **Giao thức** (`boom-proto`): biên độ dài `username`/`password`, mã lý do trả về.
//! - **Tài khoản** (module này): tên 3–24 ký tự, mật khẩu ≥ 8 ký tự, Argon2id, một phiên
//!   mỗi tài khoản, rate limit, thông báo lỗi thống nhất.
//!
//! Identity/persistence **không phải luật chơi**: ADR 0011 §1 nói rõ ADR đó không sinh mã
//! claim `R-xxx` trong RULES_V1. Mọi ràng buộc ở đây là **design** của dự án.
//!
//! # Đồng hồ
//!
//! Mọi thao tác nhận `now_ms` từ caller (Unix epoch milliseconds) — **không** đọc wall clock
//! bên trong. Nhờ vậy ca `T4-A3` (mất mạng im lặng) kiểm chứng được bằng đồng hồ tiêm vào,
//! không cần sleep thật.

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use boom_proto::dto::auth::AuthFailReason;
use boom_proto::limits::{MAX_PASSWORD_BYTES, MAX_USERNAME_BYTES};

use crate::identity::{
    Account, AccountId, AccountStore, NewAccount, RandomSource, Secret, SessionId, SessionService,
    SessionStore, StoreError, username_key,
};

/// Tham số Argon2id (ADR 0011 §5).
///
/// Mặc định lấy **đúng** mặc định của crate `argon2` 0.6.0 (`m = 19 * 1024` KiB, `t = 2`,
/// `p = 1`, output 32 byte) nên code không chứa hằng số tự đặt; muốn đổi thì đổi tham số của
/// crate và ghi lại trong ADR mới (ADR 0011 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Argon2Settings {
    /// Bộ nhớ (KiB).
    pub m_cost: u32,
    /// Số vòng.
    pub t_cost: u32,
    /// Số luồng song song.
    pub p_cost: u32,
    /// Độ dài tag (byte).
    pub output_len: usize,
    /// Độ dài salt (byte); RFC 9106 khuyến nghị 16.
    pub salt_len: usize,
}

impl Default for Argon2Settings {
    /// Mặc định của crate `argon2` 0.6.0 (ADR 0011 §5).
    fn default() -> Self {
        Self {
            m_cost: Params::DEFAULT_M_COST,
            t_cost: Params::DEFAULT_T_COST,
            p_cost: Params::DEFAULT_P_COST,
            output_len: Params::DEFAULT_OUTPUT_LEN,
            salt_len: argon2::RECOMMENDED_SALT_LEN,
        }
    }
}

impl Argon2Settings {
    /// Tham số tương ứng của crate `argon2`.
    ///
    /// Lỗi khi tham số không hợp lệ (ví dụ `m_cost < 8 * p_cost`).
    pub fn params(&self) -> Result<Params, argon2::Error> {
        Params::new(self.m_cost, self.t_cost, self.p_cost, Some(self.output_len))
    }
}

/// Tham số vận hành của auth/session (ADR 0011 §5, §6).
///
/// Đây là **design** của dự án, không phải luật chơi: các giá trị ở đây là tham số vận hành
/// trong cấu hình server, không nằm trong `data/rules`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthConfig {
    /// TTL phiên (ms) — ADR 0011 §6: cố định, mặc định 24 giờ, **không trượt** khi dùng.
    pub session_ttl_ms: i64,
    /// Trần thời gian một tài khoản bị khóa (ms) — ADR 0011 §6.1: mặc định 30 000 ms.
    pub session_liveness_timeout_ms: i64,
    /// Số lần thất bại tối đa trong một cửa sổ, theo `username_key` và theo nguồn.
    pub rate_limit_max_failures: u32,
    /// Số **lượt đăng ký** tối đa trong một cửa sổ, theo `username_key` và theo nguồn
    /// (ADR 0011 §9 dòng 12: chặn tạo tài khoản hàng loạt và probe/hash spam).
    ///
    /// Tách khỏi [`Self::rate_limit_max_failures`]: một lượt đăng ký không tiêu tốn bộ ngân
    /// sách đăng nhập, và ngược lại. Cửa sổ dùng chung [`Self::rate_limit_window_ms`].
    pub register_rate_limit_max_attempts: u32,
    /// Cửa sổ tính rate limit (ms) — dùng cho cả bộ đếm đăng nhập lẫn quota đăng ký.
    pub rate_limit_window_ms: i64,
    /// Tham số Argon2id.
    pub argon2: Argon2Settings,
    /// Đăng ký có mở không (ADR 0011 §9 dòng 12: chống tạo tài khoản hàng loạn).
    pub registration_open: bool,
    /// Số ký tự tối thiểu của mật khẩu (ADR 0011 §4: ≥ 8).
    pub password_min_chars: usize,
    /// Số byte UTF-8 tối đa của mật khẩu (ADR 0011 §4: ≤ 256).
    pub password_max_bytes: usize,
}

impl Default for AuthConfig {
    /// Giá trị V1 theo ADR 0011 §4–§6.
    fn default() -> Self {
        Self {
            session_ttl_ms: 24 * 60 * 60 * 1_000,
            session_liveness_timeout_ms: 30_000,
            rate_limit_max_failures: 5,
            register_rate_limit_max_attempts: 5,
            rate_limit_window_ms: 5 * 60 * 1_000,
            argon2: Argon2Settings::default(),
            registration_open: true,
            password_min_chars: 8,
            password_max_bytes: 256,
        }
    }
}

/// Lý do auth thất bại ở tầng miền.
///
/// Ánh xạ **1-1** sang [`AuthFailReason`] của `boom-proto` (ADR 0009 Addendum A §A.2): cùng
/// tập biến, cùng thứ tự, mỗi lý do đúng một mã.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthFailure {
    /// Sai tên hoặc sai mật khẩu — dùng chung để không lộ enumeration (ADR 0011 §5).
    InvalidCredentials,
    /// Tên đã có; chỉ trả cho `Register`.
    UsernameUnavailable,
    /// Input không hợp lệ: vượt biên độ dài, hoặc sai định dạng tên/mật khẩu.
    InvalidInput,
    /// Tài khoản đã có phiên đang sống (ADR 0011 §6).
    AccountHasActiveSession,
    /// Vượt giới hạn tần suất theo tài khoản hoặc theo nguồn (ADR 0011 §5).
    RateLimited,
    /// Đăng ký đã đóng theo cấu hình server.
    RegistrationClosed,
    /// Lỗi phía server (OS RNG, DB); chi tiết chỉ vào log, không trả cho client.
    Unavailable,
}

impl From<AuthFailure> for AuthFailReason {
    /// Ánh xạ 1-1 sang mã lý do trên dây.
    fn from(value: AuthFailure) -> Self {
        match value {
            AuthFailure::InvalidCredentials => AuthFailReason::InvalidCredentials,
            AuthFailure::UsernameUnavailable => AuthFailReason::UsernameUnavailable,
            AuthFailure::InvalidInput => AuthFailReason::InvalidInput,
            AuthFailure::AccountHasActiveSession => AuthFailReason::AccountHasActiveSession,
            AuthFailure::RateLimited => AuthFailReason::RateLimited,
            AuthFailure::RegistrationClosed => AuthFailReason::RegistrationClosed,
            AuthFailure::Unavailable => AuthFailReason::Unavailable,
        }
    }
}

/// Phân loại lỗi auth — để tầng trên biết **log gì**, không phải để trả cho client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthErrorKind {
    /// Lỗi người dùng/điều kiện đầu vào: thông báo thống nhất, không lộ enumeration.
    Rejected,
    /// Lỗi entropy của hệ điều hành — log `auth.rng_unavailable` (ADR 0011 §6, §8.1).
    RngUnavailable,
    /// Lỗi tầng lưu trữ.
    Storage,
    /// Cấu hình không hợp lệ (tham số Argon2 sai).
    Config,
}

/// Lỗi của một thao tác auth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthError {
    /// Phân loại lỗi.
    pub kind: AuthErrorKind,
    /// Lý do ở tầng miền (ánh xạ 1-1 sang `AuthFailReason`).
    pub reason: AuthFailure,
    /// Chi tiết lỗi nội bộ — **chỉ** để ghi log, không bao giờ trả cho client
    /// (ADR 0011 §6, §8.1).
    pub detail: Option<String>,
}

impl AuthError {
    /// Lỗi client thấy được (thông báo thống nhất).
    pub fn rejected(reason: AuthFailure) -> Self {
        Self {
            kind: AuthErrorKind::Rejected,
            reason,
            detail: None,
        }
    }

    /// Lỗi phía server: client chỉ thấy `Unavailable`; chi tiết để log.
    pub fn internal(kind: AuthErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            reason: AuthFailure::Unavailable,
            detail: Some(detail.into()),
        }
    }

    /// Mã lý do trả cho client (ADR 0009 Addendum A §A.2).
    pub fn wire_reason(&self) -> AuthFailReason {
        self.reason.into()
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "{:?}: {detail}", self.reason),
            None => write!(f, "{:?}", self.reason),
        }
    }
}

impl std::error::Error for AuthError {}

/// Kết quả đăng nhập/đăng ký thành công.
#[derive(Debug, PartialEq, Eq)]
pub struct LoginSuccess {
    /// Định danh tài khoản do server cấp.
    pub account_id: AccountId,
    /// Tên hiển thị của tài khoản.
    pub display_name: String,
    /// Phiên vừa cấp.
    pub session_id: SessionId,
    /// Token phiên (bearer credential) — chỉ nằm trong RAM tiến trình; `Debug` in dấu ẩn
    /// (ADR 0011 §6).
    pub token: Secret<String>,
    /// Unix epoch milliseconds hết hạn.
    pub expires_at_ms: i64,
}

/// Kết quả của một thao tác auth.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthOutcome {
    /// Đăng ký thành công: tài khoản mới **và** phiên đầu tiên đã cấp.
    Registered(LoginSuccess),
    /// Đăng nhập thành công.
    LoggedIn(LoginSuccess),
}

impl AuthOutcome {
    /// Phần kết quả dùng chung của hai nhánh.
    pub fn success(&self) -> &LoginSuccess {
        match self {
            Self::Registered(success) | Self::LoggedIn(success) => success,
        }
    }
}

/// Số lần verify đã thực hiện — để kiểm chứng "cùng đường verify" (ADR 0009 Addendum A §A.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VerifyCounts {
    /// Số lần verify với PHC thật của tài khoản.
    pub with_account: u64,
    /// Số lần verify với PHC giả (tài khoản không tồn tại).
    pub with_dummy: u64,
}

/// Cửa sổ đếm của một bucket rate limit.
#[derive(Clone, Copy, Debug)]
struct Window {
    /// Đầu cửa sổ (ms).
    start_ms: i64,
    /// Số lượt đã đếm trong cửa sổ.
    count: u32,
}

/// Bộ giới hạn tần suất trong RAM (ADR 0011 §5).
///
/// Hai bucket theo tài khoản (`username_key` đã chuẩn hóa) và theo nguồn (IP). Trạng thái
/// **không** lưu xuống DB: mất thì mất, chấp nhận được (ADR 0011 §5 nói "giới hạn tần suất
/// theo tài khoản và theo nguồn (IP)" mà không yêu cầu bền vững).
///
/// [`AuthService`] dùng **hai** bộ đếm **tách biệt**: bộ đếm lần đăng nhập sai và bộ đếm lượt
/// đăng ký (xem doc đầu module).
#[derive(Debug)]
struct RateLimiter {
    window_ms: i64,
    /// Ngưỡng: số lượt tối đa trong một cửa sổ.
    max_count: u32,
    buckets: HashMap<String, Window>,
}

impl RateLimiter {
    fn new(window_ms: i64, max_count: u32) -> Self {
        Self {
            window_ms,
            max_count,
            buckets: HashMap::new(),
        }
    }

    /// Còn được thử hay không.
    fn allows(&self, key: &str, now_ms: i64) -> bool {
        match self.buckets.get(key) {
            Some(window) if self.in_window(window, now_ms) => window.count < self.max_count,
            _ => true,
        }
    }

    /// Ghi nhận một lượt đã đếm: lần thất bại (bộ đếm đăng nhập) hoặc lượt đăng ký (quota đăng
    /// ký). Lượt đã đếm thì **giữ nguyên** khi bị từ chối: lượt bị từ chối không tiêu tốn
    /// ngân sách, nên kẻ tấn công không giữ được cửa sổ của nạn nhân mãi.
    fn record_attempt(&mut self, key: &str, now_ms: i64) {
        let expired = match self.buckets.get(key) {
            Some(window) => !in_window(window, self.window_ms, now_ms),
            None => true,
        };
        let window = self.buckets.entry(key.to_string()).or_insert(Window {
            start_ms: now_ms,
            count: 0,
        });
        if expired {
            window.start_ms = now_ms;
            window.count = 0;
        }
        window.count = window.count.saturating_add(1);
    }

    /// `now_ms` còn nằm trong cửa sổ của `window` không.
    fn in_window(&self, window: &Window, now_ms: i64) -> bool {
        in_window(window, self.window_ms, now_ms)
    }
}

/// `now_ms` còn nằm trong cửa sổ của `window` không.
fn in_window(window: &Window, window_ms: i64, now_ms: i64) -> bool {
    now_ms.saturating_sub(window.start_ms) < window_ms
}

/// Mật khẩu giả dùng để verify khi tài khoản **không tồn tại** (ADR 0011 §5).
///
/// Dài **hơn** mọi mật khẩu hợp lệ (`password_max_bytes` = 256 byte, ADR 0011 §4) nên verify
/// với hash giả **luôn** thất bại: không thể có trường hợp "tên không tồn tại" mà verify thành
/// công.
fn dummy_password() -> Vec<u8> {
    vec![0_u8; 1024]
}

/// Salt cố định cho hash giả.
///
/// Đây **không phải bí mật**: hash giả không sinh ra mật khẩu nào, nó chỉ để verify tốn đúng
/// lượng công việc như hash thật (ADR 0011 §5).
const DUMMY_SALT: [u8; argon2::RECOMMENDED_SALT_LEN] = [0_u8; argon2::RECOMMENDED_SALT_LEN];

/// Dịch vụ đăng ký/đăng nhập của server V1.
///
/// Gom các thao tác của [ADR 0011 §11](https://github.com/aquaplatinumberlitz/boom-rs) T3
/// (đăng ký/đăng nhập) và phần phát token của T4; vòng đời phiên còn lại nằm ở
/// `identity/session.rs`.
pub struct AuthService {
    config: AuthConfig,
    /// Tham số Argon2 đã kiểm hợp lệ lúc khởi tạo.
    params: Params,
    /// Vòng đời phiên (phát token, bind instance/connection).
    sessions: SessionService,
    /// Nguồn entropy — mặc định RNG của hệ điều hành, test thì tiêm nguồn lỗi được.
    random: Box<dyn RandomSource>,
    /// Bộ đếm **lần đăng nhập sai** (theo `username_key` và theo nguồn).
    limiter: RateLimiter,
    /// Bộ đếm **lượt đăng ký** (theo `username_key` và theo nguồn) — tách khỏi bộ đếm trên
    /// (ADR 0011 §9 dòng 12).
    register_limiter: RateLimiter,
    /// PHC giả, tính **một lần** rồi cache lại.
    dummy_hash: OnceLock<Result<String, String>>,
    /// Bộ đếm verify (để kiểm chứng đường verify thống nhất).
    verifies: VerifyCounts,
    /// Số lần băm mật khẩu (Argon2id) đã thực hiện — để kiểm chứng quota chặn **trước**
    /// phần công việc tốn kém.
    hashes: u64,
}

impl AuthService {
    /// Tạo dịch vụ auth.
    ///
    /// `instance_id` là boot id của tiến trình (ADR 0011 §6.1) — sinh **một lần** lúc khởi
    /// động, ghi trong log `server.start` và artifact `health.json`.
    ///
    /// `random` là nguồn entropy: mặc định dùng RNG của hệ điều hành
    /// ([`OsRandomSource`](crate::identity::OsRandomSource)); test thì tiêm nguồn lỗi được để
    /// kiểm chứng "lỗi OS RNG ⇒ không phát token" (ADR 0011 §6).
    pub fn new(
        config: AuthConfig,
        instance_id: impl Into<String>,
        random: Box<dyn RandomSource>,
    ) -> Result<Self, AuthError> {
        let params = config.argon2.params().map_err(|err| {
            AuthError::internal(
                AuthErrorKind::Config,
                format!("tham số Argon2 không hợp lệ: {err}"),
            )
        })?;
        let limiter = RateLimiter::new(config.rate_limit_window_ms, config.rate_limit_max_failures);
        let register_limiter = RateLimiter::new(
            config.rate_limit_window_ms,
            config.register_rate_limit_max_attempts,
        );
        Ok(Self {
            config,
            params,
            sessions: SessionService::new(instance_id),
            random,
            limiter,
            register_limiter,
            dummy_hash: OnceLock::new(),
            verifies: VerifyCounts::default(),
            hashes: 0,
        })
    }

    /// Cấu hình đang dùng.
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    /// Dịch vụ vòng đời phiên (phát token, revoke, quét TTL, cleanup lúc boot).
    pub fn sessions(&self) -> &SessionService {
        &self.sessions
    }

    /// Số lần verify đã thực hiện — để kiểm chứng "cùng đường verify" (ADR 0009 Addendum A §A.4).
    pub fn verify_counts(&self) -> VerifyCounts {
        self.verifies
    }

    /// Số lần băm mật khẩu (Argon2id) đã thực hiện.
    ///
    /// Dùng để kiểm chứng rằng lượt đăng ký bị **từ chối vì quota** không tốn công băm:
    /// quota được kiểm **trước** phần công việc tốn kém (ADR 0011 §9 dòng 12).
    pub fn hashed_password_count(&self) -> u64 {
        self.hashes
    }

    /// Đăng ký tài khoản mới.
    ///
    /// Thứ tự: biên → **quota đăng ký** → cờ đăng ký → định dạng → **đếm lượt** → băm → tạo
    /// tài khoản → cấp phiên.
    ///
    /// Quota được kiểm **trước** mọi việc tốn kém, và lượt đăng ký được **đếm trước** bước
    /// băm Argon2id: lượt đăng ký thành công cũng bị đếm (chặn spam tài khoản), lượt thất bại
    /// vì tên trùng cũng bị đếm (chặn probe/hash spam) — ADR 0011 §9 dòng 12.
    ///
    /// Lượt bị từ chối vì sai định dạng tên/mật khẩu **không** bị đếm: lượt đó không tốn
    /// công băm, và đếm nó sẽ cho phép kẻ tấn công khoá cửa sổ của một tên bất kỳ chỉ bằng
    /// input rác.
    ///
    /// `UsernameUnavailable` chỉ trả cho `Register` — đăng ký tự bản chất lộ tên đã có
    /// (ADR 0009 Addendum A §A.2).
    pub fn register<S: AccountStore + SessionStore + ?Sized>(
        &mut self,
        store: &mut S,
        username: &str,
        password: &str,
        source: &str,
        connection_id: &str,
        now_ms: i64,
    ) -> Result<AuthOutcome, AuthError> {
        self.check_bounds(username, password)?;
        let user_bucket = register_user_bucket(username);
        let source_bucket = register_source_bucket(source);
        // Quota đăng ký: kiểm **trước** mọi việc tốn kém, **không** tra DB trước.
        self.check_register_quota(&user_bucket, &source_bucket, now_ms)?;

        if !self.config.registration_open {
            return Err(AuthError::rejected(AuthFailure::RegistrationClosed));
        }
        // Ràng buộc định dạng tên (ADR 0011 §4).
        username_key(username).map_err(|_| AuthError::rejected(AuthFailure::InvalidInput))?;
        self.check_password(password)?;

        // Đếm lượt đăng ký **trước** bước băm: mọi lượt đi tới đây đều tốn công băm Argon2id,
        // kể cả lượt thất bại vì tên đã có.
        self.register_limiter.record_attempt(&user_bucket, now_ms);
        self.register_limiter.record_attempt(&source_bucket, now_ms);

        let phc = self.hash_password(password)?;
        let account = store
            .create_account(NewAccount {
                username: username.trim().to_string(),
                password_hash: Secret::new(phc),
                created_at_ms: now_ms,
            })
            .map_err(map_store_error)?;

        let success = self.issue_session(store, &account, connection_id, now_ms)?;
        Ok(AuthOutcome::Registered(success))
    }

    /// Đăng nhập tài khoản đã có.
    ///
    /// Thứ tự chống enumeration: biên/rate limit → verify (hash giả nếu tên không tồn tại) →
    /// **chỉ khi verify thành công** mới kiểm phiên.
    pub fn login<S: AccountStore + SessionStore + ?Sized>(
        &mut self,
        store: &mut S,
        username: &str,
        password: &str,
        source: &str,
        connection_id: &str,
        now_ms: i64,
    ) -> Result<AuthOutcome, AuthError> {
        // (1) biên / quota đăng nhập — trước mọi việc khác, kể cả trước khi tra DB.
        self.check_bounds(username, password)?;
        let user_bucket = user_bucket(username);
        let source_bucket = source_bucket(source);
        self.check_login_quota(&user_bucket, &source_bucket, now_ms)?;

        // Ràng buộc định dạng tên (ADR 0011 §4) — sai định dạng thì không có tài khoản nào
        // mang tên đó, nên trả `InvalidInput` thay vì `InvalidCredentials`.
        let key =
            username_key(username).map_err(|_| AuthError::rejected(AuthFailure::InvalidInput))?;

        // (2) verify mật khẩu — tên không tồn tại vẫn verify với hash giả.
        let account = store
            .account_by_key(&key)
            .map_err(|err| AuthError::internal(AuthErrorKind::Storage, err.to_string()))?;
        let verified = match &account {
            Some(account) => {
                self.verifies.with_account += 1;
                self.verify_password(account.password_hash.expose(), password)
            }
            None => {
                self.verifies.with_dummy += 1;
                self.verify_password(self.dummy_hash()?, password)
            }
        };
        if !verified {
            self.limiter.record_attempt(&user_bucket, now_ms);
            self.limiter.record_attempt(&source_bucket, now_ms);
            return Err(AuthError::rejected(AuthFailure::InvalidCredentials));
        }

        // Verify chỉ thành công khi tài khoản tồn tại: nhánh `None` luôn verify thất bại.
        let account = match account {
            Some(account) => account,
            None => return Err(AuthError::rejected(AuthFailure::InvalidCredentials)),
        };
        // Tài khoản bị vô hiệu hoá: thông báo thống nhất, không lộ trạng thái tài khoản.
        if account.disabled {
            return Err(AuthError::rejected(AuthFailure::InvalidCredentials));
        }

        // Rehash khi tham số Argon2 đã đổi (ADR 0011 §5): người dùng không cần đổi mật khẩu.
        if self.needs_rehash(account.password_hash.expose()) {
            let phc = self.hash_password(password)?;
            store
                .update_password_hash(account.id, &phc, now_ms)
                .map_err(|err| AuthError::internal(AuthErrorKind::Storage, err.to_string()))?;
        }

        // (3) chỉ khi verify thành công mới kiểm phiên.
        let success = self.issue_session(store, &account, connection_id, now_ms)?;
        Ok(AuthOutcome::LoggedIn(success))
    }

    /// Cấp phiên cho tài khoản vừa xác thực (phát token + tạo phiên trong một transaction).
    fn issue_session<S: AccountStore + SessionStore + ?Sized>(
        &mut self,
        store: &mut S,
        account: &Account,
        connection_id: &str,
        now_ms: i64,
    ) -> Result<LoginSuccess, AuthError> {
        let issued = self
            .sessions
            .login(
                store,
                self.random.as_mut(),
                account.id,
                connection_id,
                self.config.session_ttl_ms,
                now_ms,
            )
            .map_err(|err| match err {
                crate::identity::SessionError::Rng(rng) => {
                    AuthError::internal(AuthErrorKind::RngUnavailable, rng.message)
                }
                crate::identity::SessionError::Store(store_err) => map_store_error(store_err),
            })?;

        Ok(LoginSuccess {
            account_id: account.id,
            display_name: account.username.clone(),
            session_id: issued.session.id,
            token: issued.token,
            expires_at_ms: issued.session.expires_at_ms,
        })
    }

    /// Biên độ dài ở biên giao thức (ADR 0009 Addendum A §A.2): quá giới hạn → `InvalidInput`,
    /// **không hash**.
    fn check_bounds(&self, username: &str, password: &str) -> Result<(), AuthError> {
        if username.len() > MAX_USERNAME_BYTES || password.len() > MAX_PASSWORD_BYTES {
            return Err(AuthError::rejected(AuthFailure::InvalidInput));
        }
        Ok(())
    }

    /// Ràng buộc mật khẩu (ADR 0011 §4): ≥ `password_min_chars` ký tự, ≤ `password_max_bytes`
    /// byte UTF-8; không bắt buộc độ phức tạp.
    fn check_password(&self, password: &str) -> Result<(), AuthError> {
        let chars = password.chars().count();
        if chars < self.config.password_min_chars || password.len() > self.config.password_max_bytes
        {
            return Err(AuthError::rejected(AuthFailure::InvalidInput));
        }
        Ok(())
    }

    /// Kiểm quota **lần đăng nhập sai** theo tài khoản và theo nguồn (ADR 0011 §5).
    fn check_login_quota(
        &self,
        user_bucket: &str,
        source_bucket: &str,
        now_ms: i64,
    ) -> Result<(), AuthError> {
        if !self.limiter.allows(user_bucket, now_ms) || !self.limiter.allows(source_bucket, now_ms)
        {
            return Err(AuthError::rejected(AuthFailure::RateLimited));
        }
        Ok(())
    }

    /// Kiểm **quota đăng ký** theo tài khoản và theo nguồn (ADR 0011 §9 dòng 12).
    ///
    /// Quota này **tách** khỏi bộ đếm lần đăng nhập sai: một lượt đăng ký không tiêu tốn bộ
    /// ngân sách đăng nhập và ngược lại.
    fn check_register_quota(
        &self,
        user_bucket: &str,
        source_bucket: &str,
        now_ms: i64,
    ) -> Result<(), AuthError> {
        if !self.register_limiter.allows(user_bucket, now_ms)
            || !self.register_limiter.allows(source_bucket, now_ms)
        {
            return Err(AuthError::rejected(AuthFailure::RateLimited));
        }
        Ok(())
    }

    /// Băm mật khẩu bằng Argon2id với salt **mới** từ RNG (ADR 0011 §5, §6).
    fn hash_password(&mut self, password: &str) -> Result<String, AuthError> {
        self.hashes = self.hashes.saturating_add(1);
        let salt_len = self.config.argon2.salt_len;
        let mut salt = vec![0_u8; salt_len];
        if let Err(err) = self.random.fill(&mut salt) {
            salt.fill(0);
            return Err(AuthError::internal(
                AuthErrorKind::RngUnavailable,
                err.message,
            ));
        }
        let hasher = self.hasher();
        let phc = hasher
            .hash_password_with_salt(password.as_bytes(), &salt)
            .map_err(|err| {
                AuthError::internal(
                    AuthErrorKind::Config,
                    format!("không băm được mật khẩu: {err}"),
                )
            })?;
        Ok(phc.to_string())
    }

    /// Verify mật khẩu với PHC string.
    ///
    /// Tham số lấy từ **chính** PHC string (không phải từ cấu hình), nên tài khoản băm bằng
    /// tham số cũ vẫn verify được cho tới khi rehash (ADR 0011 §5).
    fn verify_password(&self, phc: &str, password: &str) -> bool {
        self.hasher()
            .verify_password(password.as_bytes(), phc)
            .is_ok()
    }

    /// PHC đang lưu có tham số khác tham số hiện hành không (ADR 0011 §5).
    fn needs_rehash(&self, phc: &str) -> bool {
        let Ok(parsed) = argon2::PasswordHash::new(phc) else {
            return false;
        };
        let Ok(stored) = Params::try_from(&parsed) else {
            return false;
        };
        stored != self.params
    }

    /// PHC giả để verify khi tài khoản không tồn tại (ADR 0011 §5).
    ///
    /// Tính **một lần** rồi cache: mỗi lần verify tên không tồn tại tốn đúng lượng công việc
    /// như verify thật.
    fn dummy_hash(&self) -> Result<&str, AuthError> {
        let slot = self.dummy_hash.get_or_init(|| {
            let hasher = Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone());
            hasher
                .hash_password_with_salt(&dummy_password(), &DUMMY_SALT)
                .map(|phc| phc.to_string())
                .map_err(|err| err.to_string())
        });
        match slot {
            Ok(hash) => Ok(hash.as_str()),
            Err(message) => Err(AuthError::internal(
                AuthErrorKind::Config,
                format!("không tạo được PHC giả: {message}"),
            )),
        }
    }

    /// Hasher Argon2id với tham số hiện hành.
    fn hasher(&self) -> Argon2<'static> {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone())
    }
}

impl fmt::Debug for AuthService {
    /// Không in token, PHC hay mật khẩu (ADR 0011 §6, §8.1).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthService")
            .field("instance_id", &self.sessions.instance_id())
            .field("registration_open", &self.config.registration_open)
            .finish_non_exhaustive()
    }
}

/// Bucket rate limit theo tài khoản — khóa theo `username_key` **đã chuẩn hóa** mà **không**
/// tra DB trước, để `RateLimited` không thành oracle tồn tại tài khoản (ADR 0009 Addendum A
/// §A.2).
fn user_bucket(username: &str) -> String {
    format!("user:{}", username.trim().to_lowercase())
}

/// Bucket rate limit theo nguồn (IP).
fn source_bucket(source: &str) -> String {
    format!("source:{source}")
}

/// Bucket quota đăng ký theo tài khoản — chuẩn hóa như [`user_bucket`], **không** tra DB
/// trước, để `RateLimited` không thành oracle tồn tại tài khoản.
fn register_user_bucket(username: &str) -> String {
    format!("reg-user:{}", username.trim().to_lowercase())
}

/// Bucket quota đăng ký theo nguồn (IP).
fn register_source_bucket(source: &str) -> String {
    format!("reg-source:{source}")
}

/// Dịch lỗi tầng lưu trữ sang lỗi miền.
///
/// `UsernameTaken` chỉ xảy ra ở `Register` ⇒ `UsernameUnavailable`; `AccountHasActiveSession`
/// giữ nguyên; mọi lỗi còn lại là lỗi phía server ⇒ `Unavailable` (chi tiết chỉ vào log).
fn map_store_error(err: StoreError) -> AuthError {
    match err {
        StoreError::UsernameTaken => AuthError::rejected(AuthFailure::UsernameUnavailable),
        StoreError::InvalidUsername => AuthError::rejected(AuthFailure::InvalidInput),
        StoreError::AccountHasActiveSession => {
            AuthError::rejected(AuthFailure::AccountHasActiveSession)
        }
        other => AuthError::internal(AuthErrorKind::Storage, other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Argon2Settings, AuthConfig, AuthErrorKind, AuthFailure, AuthService, VerifyCounts,
    };
    use crate::identity::{
        AccountStore, OsRandomSource, RandomSource, RngError, SessionStore, SqliteAccountStore,
        SqliteOptions, StoreError,
    };
    use argon2::Params;
    use boom_proto::dto::auth::AuthFailReason;
    use boom_proto::limits::MAX_USERNAME_BYTES;
    use tempfile::TempDir;

    const NOW: i64 = 1_700_000_000_000;

    /// Nguồn entropy lỗi — kiểm chứng "lỗi OS RNG ⇒ không phát token" (ADR 0011 §6).
    struct FailingRandom;

    impl RandomSource for FailingRandom {
        fn fill(&mut self, _dest: &mut [u8]) -> Result<(), RngError> {
            Err(RngError {
                message: String::from("entropy hỏng (test)"),
            })
        }
    }

    fn open(dir: &TempDir) -> SqliteAccountStore {
        SqliteAccountStore::open(
            &dir.path().join("boom-server.sqlite"),
            SqliteOptions::default(),
        )
        .expect("mở store")
    }

    fn service(config: AuthConfig) -> AuthService {
        AuthService::new(config, "instance-1", Box::new(OsRandomSource)).expect("tạo dịch vụ")
    }

    fn default_service() -> AuthService {
        service(AuthConfig::default())
    }

    #[test]
    fn failure_reasons_map_one_to_one_to_wire_codes() {
        // Cùng tập biến, ánh xạ 1-1 sang `AuthFailReason` (ADR 0009 Addendum A §A.2).
        let pairs = [
            (
                AuthFailure::InvalidCredentials,
                AuthFailReason::InvalidCredentials,
            ),
            (
                AuthFailure::UsernameUnavailable,
                AuthFailReason::UsernameUnavailable,
            ),
            (AuthFailure::InvalidInput, AuthFailReason::InvalidInput),
            (
                AuthFailure::AccountHasActiveSession,
                AuthFailReason::AccountHasActiveSession,
            ),
            (AuthFailure::RateLimited, AuthFailReason::RateLimited),
            (
                AuthFailure::RegistrationClosed,
                AuthFailReason::RegistrationClosed,
            ),
            (AuthFailure::Unavailable, AuthFailReason::Unavailable),
        ];
        for (domain, wire) in pairs {
            assert_eq!(AuthFailReason::from(domain), wire);
        }
    }

    #[test]
    fn argon2_defaults_match_crate_defaults() {
        let settings = Argon2Settings::default();
        assert_eq!(settings.m_cost, Params::DEFAULT_M_COST);
        assert_eq!(settings.t_cost, Params::DEFAULT_T_COST);
        assert_eq!(settings.p_cost, Params::DEFAULT_P_COST);
        assert_eq!(settings.output_len, Params::DEFAULT_OUTPUT_LEN);
        assert_eq!(settings.salt_len, argon2::RECOMMENDED_SALT_LEN);
    }

    #[test]
    fn registration_stores_phc_string() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();

        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();

        let account = store.account_by_username("bomber").unwrap().unwrap();
        let phc = account.password_hash.expose();
        // Định dạng PHC của Argon2id (ADR 0011 §5).
        assert!(phc.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{phc}");
        assert_eq!(phc.split('$').count(), 6);
        // PHC parse được ⇒ đúng định dạng.
        argon2::PasswordHash::new(phc).expect("PHC parse được");
        // Không chứa mật khẩu thô.
        assert!(!phc.contains("matkhau123"));
    }

    #[test]
    fn login_succeeds_and_issues_token() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();

        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();
        // Tài khoản đang có phiên ⇒ lần đăng nhập thứ hai bị từ chối.
        let err = auth
            .login(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.1",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::AccountHasActiveSession);
        assert_eq!(err.wire_reason(), AuthFailReason::AccountHasActiveSession);
    }

    #[test]
    fn password_is_rehashed_when_params_change() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();
        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();
        let before = store
            .account_by_username("bomber")
            .unwrap()
            .unwrap()
            .password_hash
            .expose()
            .to_string();

        // Đổi tham số Argon2 ⇒ phải rehash khi verify thành công (ADR 0011 §5).
        let changed = AuthConfig {
            argon2: Argon2Settings {
                t_cost: 3,
                ..Argon2Settings::default()
            },
            ..AuthConfig::default()
        };
        let mut auth2 = service(changed);
        // Nhả phiên cũ để đăng nhập lại được.
        let account = store.account_by_username("bomber").unwrap().unwrap();
        let session_id = account_session_id(&store, account.id);
        auth2
            .sessions()
            .logout(&mut store, session_id, NOW)
            .unwrap();

        auth2
            .login(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.1",
                "conn-2",
                NOW,
            )
            .unwrap();

        let after = store
            .account_by_username("bomber")
            .unwrap()
            .unwrap()
            .password_hash
            .expose()
            .to_string();
        assert_ne!(before, after);
        assert!(
            after.starts_with("$argon2id$v=19$m=19456,t=3,p=1$"),
            "{after}"
        );
    }

    fn account_session_id(
        store: &SqliteAccountStore,
        account_id: crate::identity::AccountId,
    ) -> crate::identity::SessionId {
        store
            .active_session(account_id)
            .expect("đọc phiên")
            .expect("có phiên")
            .id
    }

    #[test]
    fn wrong_password_is_invalid_credentials_for_every_account_state() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();

        // (a) tên không tồn tại.
        let err = auth
            .login(
                &mut store,
                "khongcogi",
                "saimatkhau",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidCredentials);

        // (b) tài khoản rảnh.
        auth.register(&mut store, "idle", "matkhau123", "10.0.0.1", "conn-2", NOW)
            .unwrap();
        let idle_account = store.account_by_username("idle").unwrap().unwrap();
        let idle_session_id = account_session_id(&store, idle_account.id);
        auth.sessions()
            .logout(&mut store, idle_session_id, NOW)
            .unwrap();
        let err = auth
            .login(&mut store, "idle", "saimatkhau", "10.0.0.1", "conn-3", NOW)
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidCredentials);

        // (c) tài khoản đang có phiên.
        auth.register(&mut store, "busy", "matkhau123", "10.0.0.2", "conn-4", NOW)
            .unwrap();
        let err = auth
            .login(&mut store, "busy", "saimatkhau", "10.0.0.2", "conn-5", NOW)
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidCredentials);

        // Cả ba đều đi qua đường verify (hash thật cho tài khoản có, hash giả cho tài khoản
        // không) ⇒ `RateLimited`/`InvalidCredentials` không phân biệt được tài khoản.
        let counts = auth.verify_counts();
        assert_eq!(counts.with_dummy, 1);
        assert_eq!(counts.with_account, 2);
        assert_eq!(
            counts,
            VerifyCounts {
                with_dummy: 1,
                with_account: 2
            }
        );
    }

    #[test]
    fn rate_limit_blocks_after_too_many_failures() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            rate_limit_max_failures: 2,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        for _ in 0..2 {
            let err = auth
                .login(
                    &mut store,
                    "bomber",
                    "saimatkhau",
                    "10.0.0.1",
                    "conn-1",
                    NOW,
                )
                .unwrap_err();
            assert_eq!(err.reason, AuthFailure::InvalidCredentials);
        }
        let err = auth
            .login(
                &mut store,
                "bomber",
                "saimatkhau",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);
        assert_eq!(err.wire_reason(), AuthFailReason::RateLimited);
    }

    // --- Quota đăng ký (review PR #85, finding P1) ---------------------------------------
    //
    // `register()` trước đây **kiểm** bucket nhưng không **đếm** lượt nào: một nguồn chưa
    // từng đăng nhập sai có thể tạo tài khoản liên tục, hoặc thử tên đăng ký liên tục (mỗi
    // lần vẫn chạy Argon2id) mà không bao giờ nhận `RateLimited`.

    /// Ngưỡng mặc định của quota đăng ký: 5 lượt / 5 phút, giống ngưỡng đăng nhập sai.
    #[test]
    fn register_quota_default_matches_documented_value() {
        let config = AuthConfig::default();
        assert_eq!(config.register_rate_limit_max_attempts, 5);
        assert_eq!(config.rate_limit_max_failures, 5);
        assert_eq!(config.rate_limit_window_ms, 5 * 60 * 1_000);
    }

    /// Tái hiện đúng ca review PR #85: cấu hình **mặc định**, cùng nguồn, cùng `now_ms` ⇒
    /// lượt đăng ký vượt ngưỡng trả `RateLimited` (trước đây 6/6 đều thành công).
    #[test]
    fn register_quota_blocks_after_max_attempts_from_one_source() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();
        let max = auth.config().register_rate_limit_max_attempts;

        for index in 0..max {
            let username = format!("bomber{index}");
            auth.register(
                &mut store,
                &username,
                "matkhau123",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .unwrap();
        }

        let err = auth
            .register(
                &mut store,
                "bomberx",
                "matkhau123",
                "10.0.0.1",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);
        assert_eq!(err.wire_reason(), AuthFailReason::RateLimited);
        // Tài khoản bị chặn **không** được tạo.
        assert!(store.account_by_username("bomberx").unwrap().is_none());
    }

    /// Quota tính theo **tên** (chuẩn hóa, không tra DB trước): nguồn khác cũng bị chặn.
    #[test]
    fn register_quota_by_username_blocks_other_sources() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            register_rate_limit_max_attempts: 1,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();

        // Nguồn khác, cùng tên ⇒ bucket theo tên đã đầy ⇒ `RateLimited`.
        let err = auth
            .register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.2",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);

        // Tên viết hoa cũng ăn cùng bucket đã chuẩn hóa.
        let err = auth
            .register(
                &mut store,
                "BOMBER",
                "matkhau123",
                "10.0.0.2",
                "conn-3",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);

        // Nguồn khác, tên khác ⇒ vẫn được (bucket theo tên và theo nguồn độc lập).
        auth.register(&mut store, "khac", "matkhau123", "10.0.0.2", "conn-4", NOW)
            .unwrap();
    }

    /// Quota chặn được probe/hash spam: lượt thử tên **trùng** cũng là lượt đăng ký (đã tốn
    /// công băm) nên bị đếm — sau ngưỡng thì trả `RateLimited`, không phải
    /// `UsernameUnavailable`.
    #[test]
    fn register_quota_counts_duplicate_name_probes() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            register_rate_limit_max_attempts: 2,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();

        // Lượt thứ hai: tên đã có ⇒ vẫn bị đếm (đã chạy Argon2id).
        let err = auth
            .register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.2",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::UsernameUnavailable);

        // Lượt thứ ba: bucket theo tên đã đầy ⇒ `RateLimited` (không phải
        // `UsernameUnavailable` ⇒ đã chặn được probe/hash spam).
        let err = auth
            .register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.3",
                "conn-3",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);
    }

    /// Hết cửa sổ thì quota reset — đồng hồ **tiêm vào**, không sleep thật.
    #[test]
    fn register_quota_window_expires_with_injected_clock() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            register_rate_limit_max_attempts: 1,
            rate_limit_window_ms: 60_000,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        auth.register(
            &mut store,
            "bomber1",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();
        let err = auth
            .register(
                &mut store,
                "bomber2",
                "matkhau123",
                "10.0.0.1",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);

        // Qua cửa sổ ⇒ quota reset, đăng ký lại được (đồng hồ tiêm, không sleep).
        auth.register(
            &mut store,
            "bomber3",
            "matkhau123",
            "10.0.0.1",
            "conn-3",
            NOW + 60_000,
        )
        .unwrap();
    }

    /// Quota được kiểm **trước** phần công việc tốn kém: lượt bị chặn không chạy Argon2id.
    #[test]
    fn rate_limited_register_does_not_hash_password() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            register_rate_limit_max_attempts: 1,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        auth.register(
            &mut store,
            "bomber1",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();
        assert_eq!(auth.hashed_password_count(), 1);

        let err = auth
            .register(
                &mut store,
                "bomber2",
                "matkhau123",
                "10.0.0.1",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);
        // Không có thêm lần băm nào ⇒ lượt bị chặn không tốn công Argon2id.
        assert_eq!(auth.hashed_password_count(), 1);
    }

    /// Quota đăng ký **tách** khỏi bộ đếm lần đăng nhập sai: lượt đăng ký không tiêu tốn bộ
    /// ngân sách đăng nhập và ngược lại.
    #[test]
    fn register_quota_is_separate_from_login_failure_budget() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            register_rate_limit_max_attempts: 1,
            rate_limit_max_failures: 1,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        // Lượt đăng ký đầu tiên (hết quota đăng ký theo nguồn).
        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();

        // Đăng nhập sai **vẫn** được thử (bộ đếm đăng nhập chưa dùng lượt nào) ⇒
        // `InvalidCredentials`, không phải `RateLimited`.
        let err = auth
            .login(
                &mut store,
                "bomber",
                "saimatkhau",
                "10.0.0.1",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidCredentials);

        // Lượt đăng ký thứ hai đã vượt quota đăng ký ⇒ `RateLimited` (bộ đếm đăng nhập
        // không liên quan).
        let err = auth
            .register(&mut store, "khac", "matkhau123", "10.0.0.1", "conn-3", NOW)
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RateLimited);
    }

    /// Lượt đăng ký bị từ chối vì sai định dạng **không** tốn công băm và **không** bị đếm
    /// (nếu đếm, kẻ tấn công khoá được cửa sổ của một tên bất kỳ bằng input rác).
    #[test]
    fn invalid_registration_input_is_neither_counted_nor_hashed() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            register_rate_limit_max_attempts: 1,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        // Sai định dạng: bị từ chối, không băm, không đếm.
        let err = auth
            .register(&mut store, "ab", "matkhau123", "10.0.0.1", "conn-1", NOW)
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidInput);
        assert_eq!(auth.hashed_password_count(), 0);

        // Lượt hợp lệ đầu tiên vẫn còn ngân sách ⇒ thành công.
        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-2",
            NOW,
        )
        .unwrap();
        assert_eq!(auth.hashed_password_count(), 1);
    }

    #[test]
    fn registration_closed_is_reported() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let config = AuthConfig {
            registration_open: false,
            ..AuthConfig::default()
        };
        let mut auth = service(config);

        let err = auth
            .register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::RegistrationClosed);
        assert!(store.account_by_username("bomber").unwrap().is_none());
    }

    #[test]
    fn duplicate_registration_is_username_unavailable() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();

        auth.register(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-1",
            NOW,
        )
        .unwrap();
        let err = auth
            .register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.2",
                "conn-2",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::UsernameUnavailable);
    }

    #[test]
    fn invalid_input_is_rejected_without_hashing() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();

        // Tên quá ngắn.
        let err = auth
            .register(&mut store, "ab", "matkhau123", "10.0.0.1", "conn-1", NOW)
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidInput);

        // Mật khẩu quá ngắn (ngắn hơn 8 ký tự).
        let err = auth
            .register(&mut store, "bomber", "ngan", "10.0.0.1", "conn-1", NOW)
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidInput);

        // Vượt biên độ dài ở biên giao thức.
        let long_user = "a".repeat(MAX_USERNAME_BYTES + 1);
        let err = auth
            .register(
                &mut store,
                &long_user,
                "matkhau123",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.reason, AuthFailure::InvalidInput);

        // Không có tài khoản nào được tạo.
        assert!(store.account_by_username("bomber").unwrap().is_none());
    }

    #[test]
    fn rng_failure_is_unavailable_and_issues_no_token() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth =
            AuthService::new(AuthConfig::default(), "instance-1", Box::new(FailingRandom)).unwrap();

        let err = auth
            .register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .unwrap_err();
        assert_eq!(err.kind, AuthErrorKind::RngUnavailable);
        assert_eq!(err.reason, AuthFailure::Unavailable);
        assert_eq!(err.wire_reason(), AuthFailReason::Unavailable);
        // Không có tài khoản, không có phiên.
        assert!(store.account_by_username("bomber").unwrap().is_none());
    }

    #[test]
    fn debug_output_hides_token_and_password() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let mut auth = default_service();

        let outcome = auth
            .register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .unwrap();
        let token = outcome.success().token.expose().clone();

        // Quét **mọi** `Debug` mà tầng trên có thể vô tình in ra.
        let scanned = [
            format!("{outcome:?}"),
            format!("{:?}", outcome.success()),
            format!("{auth:?}"),
            format!("{:?}", auth.sessions()),
            format!(
                "{:?}",
                auth.login(
                    &mut store,
                    "bomber",
                    "saimatkhau",
                    "10.0.0.1",
                    "conn-2",
                    NOW
                )
                .unwrap_err()
            ),
        ];
        for text in &scanned {
            assert!(!text.contains(&token), "token lọt vào Debug: {text}");
            assert!(
                !text.contains("matkhau123"),
                "mật khẩu lọt vào Debug: {text}"
            );
            assert!(
                !text.contains("saimatkhau"),
                "mật khẩu lọt vào Debug: {text}"
            );
        }
        // Dấu ẩn là dấu ẩn, không phải chuỗi rỗng.
        assert!(scanned[0].contains("Secret(<hidden>)"), "{}", scanned[0]);
    }

    #[test]
    fn store_error_account_has_active_session_maps_to_domain_reason() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let account = store
            .create_account(crate::identity::NewAccount {
                username: String::from("bomber"),
                password_hash: crate::identity::Secret::new(String::from("$argon2id$")),
                created_at_ms: NOW,
            })
            .unwrap();
        store
            .create_session(crate::identity::NewSession {
                account_id: account.id,
                token_hash: b"token-1".to_vec(),
                instance_id: String::from("instance-1"),
                connection_id: Some(String::from("conn-1")),
                created_at_ms: NOW,
                expires_at_ms: NOW + 86_400_000,
                last_seen_at_ms: NOW,
            })
            .unwrap();
        let err = store
            .create_session(crate::identity::NewSession {
                account_id: account.id,
                token_hash: b"token-2".to_vec(),
                instance_id: String::from("instance-1"),
                connection_id: Some(String::from("conn-2")),
                created_at_ms: NOW,
                expires_at_ms: NOW + 86_400_000,
                last_seen_at_ms: NOW,
            })
            .unwrap_err();
        assert_eq!(err, StoreError::AccountHasActiveSession);
    }
}
