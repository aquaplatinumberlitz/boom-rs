//! Tầng lưu trữ identity của `boom-server` (ADR 0011 §3–§4, §10).
//!
//! Ranh giới module theo ADR 0011 §10:
//!
//! | Module | Nội dung |
//! |---|---|
//! | `identity/mod.rs` (đây) | trait [`AccountStore`] + [`SessionStore`] + kiểu dữ liệu domain; không rò SQL ra ngoài |
//! | `identity/sqlite.rs` | hiện thực `rusqlite`: schema v1, migration runner, backup, `integrity_check`, khóa instance |
//! | `identity/secret.rs` | [`Secret<T>`] (Debug ẩn) cho token phiên và PHC string |
//! | `identity/session.rs` | vòng đời phiên: phát token, bind `instance_id`/`connection_id`, revoke, quét TTL, cleanup lúc boot (ADR 0011 §6, §6.1) |
//!
//! **Phạm vi task #61 (T2 theo ADR 0011 §11)**: schema v1 + migration runner + trait
//! [`AccountStore`] + backup + `integrity_check` + khóa instance `fs4`.
//!
//! **Phạm vi task #76 (T3+T4 theo ADR 0011 §11)**: trait [`SessionStore`] (vòng đời phiên ở
//! tầng lưu trữ) + module `identity/session.rs` (logic vòng đời: phát token 256 bit, bind
//! instance/connection, revoke theo `session_id`, quét TTL, cleanup lúc boot theo
//! `instance_id`) + `auth.rs` (đăng ký/đăng nhập Argon2id, PHC, rate limit, thông báo lỗi
//! thống nhất).
//!
//! Identity/persistence **không phải luật chơi**: ADR 0011 §1 nói rõ ADR đó không sinh mã
//! claim `R-xxx` trong RULES_V1. Mọi ràng buộc ở đây là **design** của dự án, không phải
//! thuật toán của server gốc.
//!
//! **Quyền file (POSIX, ADR 0011 §3):** file DB *và* mọi file bản sao (backup thủ công,
//! backup trước migration) đều owner-only `0600`, quyền đặt **ngay từ lúc tạo file** —
//! trước khi SQLite ghi dữ liệu vào. **Giới hạn Windows:** không có `0600` tương đương,
//! bảo vệ dựa vào ACL của thư mục dữ liệu.

mod secret;
mod session;
mod sqlite;

pub use secret::Secret;
pub use session::{
    IssuedSession, OsRandomSource, RandomSource, RevokeReason, RngError, SessionError,
    SessionService, SessionStore, TOKEN_BYTES, issue_token, token_hash,
};
pub use sqlite::{LOCK_FILE_NAME, SqliteAccountStore, SqliteOptions};

/// Định danh tài khoản (khóa chính `accounts.id`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId(pub i64);

/// Định danh phiên (khóa chính `sessions.id`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub i64);

/// Tài khoản đã lưu. Kiểu miền: không có SQL nào lọt ra khỏi module.
#[derive(Debug, PartialEq, Eq)]
pub struct Account {
    /// Định danh do DB cấp.
    pub id: AccountId,
    /// Tên hiển thị, giữ nguyên dấu gõ (ADR 0011 §4).
    pub username: String,
    /// Khóa so khớp: trim + lowercase (ADR 0011 §4).
    pub username_key: String,
    /// PHC string của mật khẩu; không bao giờ log (ADR 0011 §4–§5).
    pub password_hash: Secret<String>,
    /// Unix epoch milliseconds.
    pub created_at_ms: i64,
    /// Unix epoch milliseconds.
    pub updated_at_ms: i64,
    /// Tài khoản bị vô hiệu hoá thì không được đăng nhập.
    pub disabled: bool,
}

/// Dữ liệu tạo tài khoản.
///
/// `password_hash` là **PHC string đã băm** (T3 sinh), không phải mật khẩu thô; tầng lưu trữ
/// không băm và không hiểu thuật toán (ADR 0011 §5).
#[derive(Debug, PartialEq, Eq)]
pub struct NewAccount {
    /// Tên hiển thị do người chơi gõ; sẽ được chuẩn hóa thành `username_key`.
    pub username: String,
    /// PHC string của mật khẩu.
    pub password_hash: Secret<String>,
    /// Unix epoch milliseconds lúc tạo.
    pub created_at_ms: i64,
}

/// Phiên đã lưu.
///
/// Không có `token_hash`: DB chỉ lưu SHA-256(token thô) (ADR 0011 §6) và hash đó không cần
/// lọt ra khỏi tầng lưu trữ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// Định danh do DB cấp.
    pub id: SessionId,
    /// Tài khoản sở hữu phiên.
    pub account_id: AccountId,
    /// Boot id của tiến trình sở hữu phiên (ADR 0011 §6.1).
    pub instance_id: String,
    /// Connection đã auth; `None` sau khi connection chết (ADR 0011 §6.1).
    pub connection_id: Option<String>,
    /// Unix epoch milliseconds.
    pub created_at_ms: i64,
    /// Unix epoch milliseconds hết hạn.
    pub expires_at_ms: i64,
    /// Unix epoch milliseconds lần thấy gần nhất.
    pub last_seen_at_ms: i64,
    /// `None` = phiên còn **giữ chỗ** — đúng thứ unique partial index thực thi
    /// "một tài khoản một phiên hiệu lực" (ADR 0011 §4).
    pub revoked_at_ms: Option<i64>,
    /// Mã lý do giải phóng phiên (ADR 0011 §6.1).
    pub revoke_reason: Option<String>,
}

/// Dữ liệu tạo hàng phiên.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSession {
    /// Tài khoản sở hữu phiên.
    pub account_id: AccountId,
    /// SHA-256(token thô) — DB không lưu token thô (ADR 0011 §6).
    pub token_hash: Vec<u8>,
    /// Boot id của tiến trình sở hữu phiên (ADR 0011 §6.1).
    pub instance_id: String,
    /// Connection đã auth, `None` nếu chưa gắn.
    pub connection_id: Option<String>,
    /// Unix epoch milliseconds lúc tạo.
    pub created_at_ms: i64,
    /// Unix epoch milliseconds hết hạn.
    pub expires_at_ms: i64,
    /// Unix epoch milliseconds lần thấy gần nhất.
    pub last_seen_at_ms: i64,
}

/// Cổng lưu tài khoản. Hiện thực miền, không rò SQL ra ngoài module (ADR 0011 §10).
pub trait AccountStore {
    /// Tạo tài khoản mới.
    ///
    /// `username` được kiểm ràng buộc và chuẩn hóa thành `username_key` (trim + lowercase,
    /// ADR 0011 §4). `username_key` đã tồn tại thì trả [`StoreError::UsernameTaken`].
    fn create_account(&mut self, account: NewAccount) -> Result<Account, StoreError>;

    /// Tìm tài khoản theo `username_key` **đã chuẩn hóa**.
    fn account_by_key(&self, username_key: &str) -> Result<Option<Account>, StoreError>;

    /// Tìm tài khoản theo tên hiển thị; tự chuẩn hóa trước khi so khớp.
    ///
    /// Tên không hợp lệ trả [`StoreError::InvalidUsername`] (không phải `Ok(None)`): không
    /// tài khoản nào có thể mang tên đó.
    fn account_by_username(&self, username: &str) -> Result<Option<Account>, StoreError>;

    /// Tìm tài khoản theo định danh.
    fn account_by_id(&self, id: AccountId) -> Result<Option<Account>, StoreError>;

    /// Bật/tắt tài khoản; trả `false` nếu không có tài khoản tương ứng.
    fn set_account_disabled(
        &mut self,
        id: AccountId,
        disabled: bool,
        now_ms: i64,
    ) -> Result<bool, StoreError>;

    /// Ghi lại PHC string của tài khoản (rehash khi tham số Argon2 đổi, ADR 0011 §5).
    ///
    /// Trả `false` nếu không có tài khoản tương ứng. `password_hash` là **PHC string đã băm**;
    /// tầng lưu trữ không hiểu thuật toán.
    fn update_password_hash(
        &mut self,
        id: AccountId,
        password_hash: &str,
        now_ms: i64,
    ) -> Result<bool, StoreError>;
}

/// Lỗi của tầng lưu trữ identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    /// Tên đăng nhập không khớp ràng buộc ADR 0011 §4: 3–24 ký tự, chữ/số/`_`/`-`.
    InvalidUsername,
    /// `username_key` đã tồn tại.
    UsernameTaken,
    /// Tài khoản đã có phiên hiệu lực (unique partial index, SCOPE_V1 §3).
    AccountHasActiveSession,
    /// `PRAGMA user_version` của file DB không nằm trong phạm vi binary này hỗ trợ
    /// (ADR 0011 §4: version lạ thì từ chối, không đoán và không downgrade).
    UnsupportedSchema {
        /// Version đọc được trong file DB.
        found: i64,
        /// Version cao nhất binary này hỗ trợ.
        supported: i64,
    },
    /// Instance khác đang giữ khóa file DB ⇒ từ chối khởi động (ADR 0011 §4).
    DatabaseLocked,
    /// `PRAGMA integrity_check` trả về khác `ok` (ADR 0011 §4).
    IntegrityCheckFailed {
        /// Báo cáo của SQLite.
        report: String,
    },
    /// File bản sao đích đã tồn tại ⇒ bản sao cũ **không** bị ghi đè (ADR 0011 §4).
    BackupTargetExists {
        /// Đường dẫn file bản sao đích.
        path: String,
    },
    /// Lỗi SQLite.
    Sqlite(String),
    /// Lỗi I/O khi đọc/ghi file.
    Io(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUsername => write!(
                f,
                "tên đăng nhập không hợp lệ (ADR 0011 §4: 3–24 ký tự, chữ/số/`_`/`-`)"
            ),
            Self::UsernameTaken => write!(f, "tên đăng nhập đã tồn tại"),
            Self::AccountHasActiveSession => write!(f, "tài khoản đã có phiên hiệu lực"),
            Self::UnsupportedSchema { found, supported } => write!(
                f,
                "phiên bản schema của DB không được hỗ trợ: đọc được {found}, hỗ trợ tới {supported}"
            ),
            Self::DatabaseLocked => write!(f, "file DB đang bị instance khác giữ khóa"),
            Self::IntegrityCheckFailed { report } => {
                write!(f, "kiểm tra tính toàn vẹn thất bại: {report}")
            }
            Self::BackupTargetExists { path } => {
                write!(f, "file bản sao đã tồn tại, không ghi đè: {path}")
            }
            Self::Sqlite(msg) => write!(f, "lỗi SQLite: {msg}"),
            Self::Io(msg) => write!(f, "lỗi I/O: {msg}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// Số ký tự tối thiểu của tên đăng nhập (ADR 0011 §4).
const USERNAME_MIN_CHARS: usize = 3;

/// Số ký tự tối đa của tên đăng nhập (ADR 0011 §4).
const USERNAME_MAX_CHARS: usize = 24;

/// Kiểm tra ràng buộc tên đăng nhập (ADR 0011 §4): 3–24 ký tự, chữ/số/`_`/`-`.
///
/// ADR 0011 §4 ghi "chữ/số/`_`/`-`" mà không nói ASCII; ở đây đọc là **ASCII alphanumeric**
/// để không có nhập nhằng Unicode (ký tự trông giống nhau) và để so khớp `username_key`
/// là xác định. Chữ Việt có dấu bị từ chối — nếu muốn cho phép thì cần ADR sửa §4.
fn validate_username(name: &str) -> Result<(), StoreError> {
    let chars = name.chars().count();
    if !(USERNAME_MIN_CHARS..=USERNAME_MAX_CHARS).contains(&chars) {
        return Err(StoreError::InvalidUsername);
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(StoreError::InvalidUsername);
    }
    Ok(())
}

/// Chuẩn hóa `username_key`: trim + lowercase (ADR 0011 §4).
///
/// Đây là khóa so khớp duy nhất; tên hiển thị giữ nguyên dấu gõ.
pub fn username_key(username: &str) -> Result<String, StoreError> {
    let trimmed = username.trim();
    validate_username(trimmed)?;
    Ok(trimmed.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{StoreError, username_key};

    #[test]
    fn key_is_trimmed_and_lowercased() {
        assert_eq!(username_key("  BoMber  ").unwrap(), "bomber");
        assert_eq!(username_key("BOMBER").unwrap(), "bomber");
        assert_eq!(username_key("bom_ber-1").unwrap(), "bom_ber-1");
    }

    #[test]
    fn invalid_usernames_are_rejected() {
        // Quá ngắn / quá dài.
        assert_eq!(username_key("ab"), Err(StoreError::InvalidUsername));
        assert_eq!(
            username_key(&"a".repeat(25)),
            Err(StoreError::InvalidUsername)
        );
        // Ký tự ngoài chữ/số/`_`/`-`.
        assert_eq!(username_key("a b"), Err(StoreError::InvalidUsername));
        assert_eq!(username_key("a@b"), Err(StoreError::InvalidUsername));
        assert_eq!(username_key("a.b"), Err(StoreError::InvalidUsername));
        // Rỗng sau trim.
        assert_eq!(username_key("   "), Err(StoreError::InvalidUsername));
    }

    #[test]
    fn boundary_lengths_are_accepted() {
        assert!(username_key("abc").is_ok());
        assert!(username_key(&"a".repeat(24)).is_ok());
    }
}
