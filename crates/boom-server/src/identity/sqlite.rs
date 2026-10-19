//! Hiện thực `rusqlite` của tầng lưu trữ identity (ADR 0011 §3–§4, §10).
//!
//! Cơ sở lưu trữ: **một file SQLite** cho server V1 (ADR 0011 §3). Mỗi connection đặt
//! `journal_mode = WAL`, `foreign_keys = ON`, `synchronous = FULL`, `busy_timeout` đúng như
//! ADR 0011 §3 ghi.
//!
//! Migration forward-only theo `PRAGMA user_version` (ADR 0011 §4): danh sách migration là
//! hằng tuple `(version, sql)`, mỗi bước chạy trong **một** transaction và bump
//! `user_version` trong cùng transaction đó. DB có `user_version` cao hơn version binary hỗ
//! trợ thì **từ chối khởi động**, không đoán và không downgrade.
//!
//! **Quyền file (POSIX, ADR 0011 §3):** file DB *và* mọi file bản sao (backup thủ công,
//! backup trước migration) đều owner-only `0600`, và quyền đó được đặt **ngay từ lúc tạo
//! file** — trước khi SQLite ghi bất kỳ byte dữ liệu nào vào. Bản sao chứa
//! `accounts.password_hash` và `sessions.token_hash`, nên để `VACUUM INTO` tự tạo file
//! (quyền theo umask của tiến trình ⇒ `0644` dưới umask 022) là lỗ hổng: user khác trên
//! cùng máy đọc được bản sao trong khi file DB gốc đã được bảo vệ.
//!
//! **Giới hạn Windows:** Windows không có quyền POSIX `0600`. File được tạo với quyền
//! mặc định của tiến trình; việc bảo vệ dựa vào **ACL của thư mục dữ liệu** (thư mục đặt
//! owner-only) — biện pháp `0600` của ADR 0011 §3 chỉ áp dụng cho POSIX.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use fs4::{FileExt, TryLockError};
use rusqlite::{Connection, OptionalExtension, params};

use super::{
    Account, AccountId, AccountStore, NewAccount, NewSession, RevokeReason, Session, SessionId,
    SessionStore, StoreError, username_key,
};

/// Tên file khóa instance trong thư mục dữ liệu (ADR 0011 §4).
pub const LOCK_FILE_NAME: &str = "boom-server.lock";

/// Quyền owner-only (`rw-------`) cho file DB và file bản sao trên POSIX (ADR 0011 §3).
///
/// Đặt **ngay từ lúc tạo file** (không phải chmod sau khi ghi) để không có khoảnh khắc
/// nào dữ liệu nằm trong file đọc được bởi group/other.
#[cfg(unix)]
const OWNER_ONLY_MODE: u32 = 0o600;

/// Schema v1 (ADR 0011 §4). Chỉ hai bảng — identity và session; **không** có bảng
/// economy/inventory/ledger (V1 không thương mại, SCOPE_V1 §1).
///
/// Unique partial index `sessions_one_active_per_account` là nơi *thực thi* policy "một
/// tài khoản một phiên hiệu lực" (SCOPE_V1 §3) ở tầng DB, không chỉ ở code.
const SCHEMA_V1: &str = "\
CREATE TABLE accounts (
    id             INTEGER PRIMARY KEY,
    username       TEXT    NOT NULL,          -- tên hiển thị, giữ nguyên dấu gõ
    username_key   TEXT    NOT NULL UNIQUE,   -- chuẩn hóa: trim + lowercase, khóa so khớp
    password_hash  TEXT    NOT NULL,          -- PHC string; không bao giờ log
    created_at_ms  INTEGER NOT NULL,
    updated_at_ms  INTEGER NOT NULL,
    disabled       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE sessions (
    id              INTEGER PRIMARY KEY,
    account_id      INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash      BLOB    NOT NULL UNIQUE,   -- SHA-256(token thô), không lưu token
    instance_id     TEXT    NOT NULL,          -- boot id của tiến trình sở hữu phiên
    connection_id   TEXT,                     -- connection đã auth; NULL sau khi connection chết
    created_at_ms   INTEGER NOT NULL,
    expires_at_ms   INTEGER NOT NULL,
    last_seen_at_ms INTEGER NOT NULL,
    revoked_at_ms   INTEGER,                  -- NULL = phiên còn giữ chỗ
    revoke_reason   TEXT                      -- mã lý do giải phóng
);

CREATE UNIQUE INDEX sessions_one_active_per_account
    ON sessions(account_id) WHERE revoked_at_ms IS NULL;
";

/// Danh sách migration forward-only: `(version, sql)` (ADR 0011 §4).
const MIGRATIONS: &[(i64, &str)] = &[(1, SCHEMA_V1)];

/// Version schema cao nhất binary này hỗ trợ. Phải bằng version cuối của [`MIGRATIONS`]
/// (có test kiểm).
const SUPPORTED_SCHEMA_VERSION: i64 = 1;

const INSERT_ACCOUNT: &str = "\
INSERT INTO accounts (username, username_key, password_hash, created_at_ms, updated_at_ms, disabled)
VALUES (?1, ?2, ?3, ?4, ?4, 0)";

const SELECT_ACCOUNT_BY_KEY: &str = "\
SELECT id, username, username_key, password_hash, created_at_ms, updated_at_ms, disabled
FROM accounts WHERE username_key = ?1";

const SELECT_ACCOUNT_BY_ID: &str = "\
SELECT id, username, username_key, password_hash, created_at_ms, updated_at_ms, disabled
FROM accounts WHERE id = ?1";

const UPDATE_ACCOUNT_DISABLED: &str = "\
UPDATE accounts SET disabled = ?2, updated_at_ms = ?3 WHERE id = ?1";

const UPDATE_PASSWORD_HASH: &str = "\
UPDATE accounts SET password_hash = ?2, updated_at_ms = ?3 WHERE id = ?1";

const INSERT_SESSION: &str = "\
INSERT INTO sessions (
    account_id, token_hash, instance_id, connection_id,
    created_at_ms, expires_at_ms, last_seen_at_ms, revoked_at_ms, revoke_reason
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL)";

/// Dọn phiên đã hết TTL **của một tài khoản** (ADR 0011 §6.1 bước 2a).
///
/// Chạy trong **cùng** transaction với `INSERT` phiên mới ⇒ không có cửa sổ mà phiên cũ
/// chặn phiên mới, và không có cửa sổ mà cleanup của phiên cũ chạm phiên vừa tạo.
const REVOKE_EXPIRED_FOR_ACCOUNT: &str = "\
UPDATE sessions SET revoked_at_ms = ?2, revoke_reason = ?3, connection_id = NULL
WHERE account_id = ?1 AND revoked_at_ms IS NULL AND expires_at_ms <= ?2";

const SELECT_SESSION_BY_ID: &str = "\
SELECT id, account_id, instance_id, connection_id,
       created_at_ms, expires_at_ms, last_seen_at_ms, revoked_at_ms, revoke_reason
FROM sessions WHERE id = ?1";

const SELECT_SESSION_BY_TOKEN_HASH: &str = "\
SELECT id, account_id, instance_id, connection_id,
       created_at_ms, expires_at_ms, last_seen_at_ms, revoked_at_ms, revoke_reason
FROM sessions WHERE token_hash = ?1 AND revoked_at_ms IS NULL";

const SELECT_ACTIVE_SESSION: &str = "\
SELECT id, account_id, instance_id, connection_id,
       created_at_ms, expires_at_ms, last_seen_at_ms, revoked_at_ms, revoke_reason
FROM sessions WHERE account_id = ?1 AND revoked_at_ms IS NULL";

/// Thu hồi đúng `session_id` còn giữ chỗ (ADR 0011 §6.1 bước 1).
///
/// `revoked_at_ms IS NULL` làm thao tác idempotent và **không** chạm phiên mới (id khác) —
/// đây là hàng chống race "connection cũ chết sau khi connection mới đã đăng nhập" (`T4-A5`).
const REVOKE_SESSION: &str = "\
UPDATE sessions SET revoked_at_ms = ?2, revoke_reason = ?3, connection_id = NULL
WHERE id = ?1 AND revoked_at_ms IS NULL";

const REVOKE_ACCOUNT_SESSIONS: &str = "\
UPDATE sessions SET revoked_at_ms = ?2, revoke_reason = ?3, connection_id = NULL
WHERE account_id = ?1 AND revoked_at_ms IS NULL";

/// Cleanup lúc boot: lọc theo `instance_id`, **không** lọc theo `account_id`
/// (ADR 0011 §6.1 bước 3).
const REVOKE_OTHER_INSTANCE_SESSIONS: &str = "\
UPDATE sessions SET revoked_at_ms = ?2, revoke_reason = ?3, connection_id = NULL
WHERE revoked_at_ms IS NULL AND instance_id <> ?1";

const REVOKE_INSTANCE_SESSIONS: &str = "\
UPDATE sessions SET revoked_at_ms = ?2, revoke_reason = ?3, connection_id = NULL
WHERE revoked_at_ms IS NULL AND instance_id = ?1";

const SWEEP_EXPIRED_SESSIONS: &str = "\
UPDATE sessions SET revoked_at_ms = ?1, revoke_reason = ?2, connection_id = NULL
WHERE revoked_at_ms IS NULL AND expires_at_ms <= ?1";

const SWEEP_STALE_SESSIONS: &str = "\
UPDATE sessions SET revoked_at_ms = ?2, revoke_reason = ?3, connection_id = NULL
WHERE revoked_at_ms IS NULL AND instance_id = ?1 AND last_seen_at_ms <= ?4";

const TOUCH_SESSION: &str = "\
UPDATE sessions SET last_seen_at_ms = ?2 WHERE id = ?1 AND revoked_at_ms IS NULL";

/// Tuỳ chọn mở DB.
///
/// Giá trị ở đây là **tham số vận hành** (design), không phải luật chơi; T1 (config) sẽ đọc
/// từ file cấu hình thay vì dùng mặc định.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SqliteOptions {
    /// `PRAGMA busy_timeout` (ms): chờ khi có ghi đồng thời thay vì lỗi `SQLITE_BUSY` ngay
    /// (ADR 0011 §3).
    pub busy_timeout_ms: u32,
}

impl Default for SqliteOptions {
    fn default() -> Self {
        Self {
            busy_timeout_ms: 5_000,
        }
    }
}

/// Store SQLite cho identity.
///
/// Giữ **khóa instance** suốt vòng đời store: một file DB chỉ một tiến trình server
/// (ADR 0011 §4). Không có khóa này thì bước "giải phóng phiên của instance cũ lúc boot"
/// (ADR 0011 §6.1) sẽ giải phóng nhầm phiên của instance đang chạy.
pub struct SqliteAccountStore {
    conn: Connection,
    db_path: PathBuf,
    /// Giữ file khóa mở ⇒ khóa advisory được giữ (fs4 nhả khóa khi file đóng).
    _instance_lock: File,
}

impl SqliteAccountStore {
    /// Mở store: giữ khóa instance, mở file SQLite, chạy migration.
    ///
    /// Từ chối (không mở được) khi: instance khác đang giữ khóa file DB, hoặc file DB có
    /// `PRAGMA user_version` nằm ngoài phạm vi binary này hỗ trợ (ADR 0011 §4).
    pub fn open(db_path: &Path, options: SqliteOptions) -> Result<Self, StoreError> {
        let dir = db_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        std::fs::create_dir_all(dir).map_err(io_err)?;
        let instance_lock = acquire_instance_lock(dir)?;
        // Tạo file DB (nếu chưa có) với quyền owner-only ngay từ đầu trên POSIX, trước khi
        // SQLite ghi bất kỳ dữ liệu nào vào (ADR 0011 §3).
        ensure_db_file(db_path)?;
        // Chốt quyền owner-only **trước khi** mở connection: SQLite kế thừa quyền file
        // chính cho các file `-wal`/`-shm` tạo kèm, nên phải siết quyền trước lúc mở
        // (ADR 0011 §3, threat §9 dòng 7).
        restrict_permissions(db_path)?;

        let mut conn = Connection::open(db_path).map_err(sqlite_err)?;
        // Kiểm tra version TRƯỚC khi đụng pragma: DB lạ thì bị từ chối mà không sửa file.
        let version = user_version(&conn)?;
        if !(0..=SUPPORTED_SCHEMA_VERSION).contains(&version) {
            return Err(StoreError::UnsupportedSchema {
                found: version,
                supported: SUPPORTED_SCHEMA_VERSION,
            });
        }

        // ADR 0011 §3: cấu hình connection.
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(sqlite_err)?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(sqlite_err)?;
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(sqlite_err)?;
        conn.pragma_update(None, "busy_timeout", options.busy_timeout_ms)
            .map_err(sqlite_err)?;

        run_migrations(&mut conn, db_path, version)?;

        Ok(Self {
            conn,
            db_path: db_path.to_path_buf(),
            _instance_lock: instance_lock,
        })
    }

    /// Đường dẫn file DB của store.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Version schema đang có trong file DB (`PRAGMA user_version`).
    pub fn schema_version(&self) -> Result<i64, StoreError> {
        user_version(&self.conn)
    }

    /// Sao lưu file DB sang đường dẫn khác bằng `VACUUM INTO` (ADR 0011 §4).
    ///
    /// `VACUUM INTO` tạo bản sao tối giản (loại bỏ phần đã xoá) và **không** ghi đè file đã
    /// có. Ở đây việc "không ghi đè" do [`StoreError::BackupTargetExists`] đảm nhiệm: file
    /// đích được tạo trước (xem [`prepare_backup_target`]) nên SQLite không bao giờ phải tự
    /// từ chối ghi đè.
    ///
    /// File bản sao chứa `accounts.password_hash` và `sessions.token_hash` ⇒ trên POSIX
    /// nó được tạo với quyền owner-only `0600` **trước khi** SQLite ghi dữ liệu vào
    /// (ADR 0011 §3). Trên Windows không có `0600` tương đương (xem giới hạn ở đầu
    /// module).
    pub fn backup_to(&self, target: &Path) -> Result<(), StoreError> {
        vacuum_into(&self.conn, target)
    }

    /// Kiểm tra tính toàn vẹn file DB (`PRAGMA integrity_check`, ADR 0011 §4).
    pub fn integrity_check(&self) -> Result<(), StoreError> {
        integrity_check(&self.conn)
    }

    /// Số phiên đang **giữ chỗ** (`revoked_at_ms IS NULL`) của tài khoản.
    ///
    /// Unique partial index `sessions_one_active_per_account` đảm bảo giá trị này **≤ 1**
    /// (ADR 0011 §4); hàm tồn tại để kiểm chứng và chẩn đoán, không phải để dựa vào nó cho
    /// policy.
    pub fn active_session_count(&self, account_id: AccountId) -> Result<usize, StoreError> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT count(*) FROM sessions WHERE account_id = ?1 AND revoked_at_ms IS NULL",
                [account_id.0],
                |row| row.get(0),
            )
            .map_err(sqlite_err)?;
        Ok(count as usize)
    }
}

impl SessionStore for SqliteAccountStore {
    /// Tạo phiên mới trong **một** transaction (ADR 0011 §6.1 bước 2).
    ///
    /// Transaction gồm: (a) revoke các phiên đã hết TTL **của tài khoản đó** (reason
    /// `expired`), (b) `INSERT` phiên mới kèm `instance_id`/`connection_id`. Vì (a) và (b)
    /// cùng transaction, không có cửa sổ mà phiên cũ chặn phiên mới, và không có cửa sổ mà
    /// cleanup của phiên cũ chạm phiên vừa tạo.
    ///
    /// `BEGIN IMMEDIATE` lấy khóa ghi ngay ⇒ hai lần đăng nhập đồng thời bị tuần tự hóa: một
    /// thắng, một còn lại vi phạm unique partial index ⇒ rollback ⇒
    /// [`StoreError::AccountHasActiveSession`] (ADR 0011 §9 dòng 8, ca `T4-A4`).
    fn create_session(&mut self, session: NewSession) -> Result<Session, StoreError> {
        let now_ms = session.created_at_ms;
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sqlite_err)?;

        tx.execute(
            REVOKE_EXPIRED_FOR_ACCOUNT,
            params![session.account_id.0, now_ms, RevokeReason::Expired.as_str()],
        )
        .map_err(sqlite_err)?;

        tx.execute(
            INSERT_SESSION,
            params![
                session.account_id.0,
                session.token_hash,
                session.instance_id,
                session.connection_id,
                session.created_at_ms,
                session.expires_at_ms,
                session.last_seen_at_ms,
            ],
        )
        .map_err(map_constraint)?;

        let id = tx.last_insert_rowid();
        tx.commit().map_err(sqlite_err)?;

        Ok(Session {
            id: SessionId(id),
            account_id: session.account_id,
            instance_id: session.instance_id,
            connection_id: session.connection_id,
            created_at_ms: session.created_at_ms,
            expires_at_ms: session.expires_at_ms,
            last_seen_at_ms: session.last_seen_at_ms,
            revoked_at_ms: None,
            revoke_reason: None,
        })
    }

    /// Đọc phiên theo định danh.
    fn session_by_id(&self, id: SessionId) -> Result<Option<Session>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached(SELECT_SESSION_BY_ID)
            .map_err(sqlite_err)?;
        stmt.query_row([id.0], session_from_row)
            .optional()
            .map_err(sqlite_err)
    }

    /// Đọc phiên theo SHA-256(token thô).
    fn session_by_token_hash(&self, token_hash: &[u8]) -> Result<Option<Session>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached(SELECT_SESSION_BY_TOKEN_HASH)
            .map_err(sqlite_err)?;
        stmt.query_row([token_hash], session_from_row)
            .optional()
            .map_err(sqlite_err)
    }

    /// Phiên đang **giữ chỗ** (`revoked_at_ms IS NULL`) của tài khoản.
    fn active_session(&self, account_id: AccountId) -> Result<Option<Session>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached(SELECT_ACTIVE_SESSION)
            .map_err(sqlite_err)?;
        stmt.query_row([account_id.0], session_from_row)
            .optional()
            .map_err(sqlite_err)
    }

    /// Thu hồi đúng `session_id` còn giữ chỗ (ADR 0011 §6.1 bước 1).
    fn revoke_session(
        &mut self,
        id: SessionId,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute(REVOKE_SESSION, params![id.0, now_ms, reason.as_str()])
            .map_err(sqlite_err)?;
        Ok(changed > 0)
    }

    /// Thu hồi mọi phiên đang giữ chỗ của tài khoản (đường admin, ADR 0011 §6).
    fn revoke_account_sessions(
        &mut self,
        account_id: AccountId,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        let changed = self
            .conn
            .execute(
                REVOKE_ACCOUNT_SESSIONS,
                params![account_id.0, now_ms, reason.as_str()],
            )
            .map_err(sqlite_err)?;
        Ok(changed)
    }

    /// Thu hồi phiên của **instance khác** (cleanup lúc boot, ADR 0011 §6.1 bước 3).
    fn revoke_other_instance_sessions(
        &mut self,
        current_instance_id: &str,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        let changed = self
            .conn
            .execute(
                REVOKE_OTHER_INSTANCE_SESSIONS,
                params![current_instance_id, now_ms, reason.as_str()],
            )
            .map_err(sqlite_err)?;
        Ok(changed)
    }

    /// Thu hồi phiên của **instance này** (server tắt có trật tự, ADR 0011 §6.1).
    fn revoke_instance_sessions(
        &mut self,
        instance_id: &str,
        reason: RevokeReason,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        let changed = self
            .conn
            .execute(
                REVOKE_INSTANCE_SESSIONS,
                params![instance_id, now_ms, reason.as_str()],
            )
            .map_err(sqlite_err)?;
        Ok(changed)
    }

    /// Quét và thu hồi phiên đã hết TTL (ADR 0011 §6.1 bước 4).
    fn sweep_expired_sessions(&mut self, now_ms: i64) -> Result<usize, StoreError> {
        let changed = self
            .conn
            .execute(
                SWEEP_EXPIRED_SESSIONS,
                params![now_ms, RevokeReason::Expired.as_str()],
            )
            .map_err(sqlite_err)?;
        Ok(changed)
    }

    /// Quét và thu hồi phiên quá hạn liveness của `instance_id` (ADR 0011 §6.1).
    fn sweep_stale_sessions(
        &mut self,
        instance_id: &str,
        liveness_timeout_ms: i64,
        now_ms: i64,
    ) -> Result<usize, StoreError> {
        let threshold = now_ms.saturating_sub(liveness_timeout_ms);
        let changed = self
            .conn
            .execute(
                SWEEP_STALE_SESSIONS,
                params![
                    instance_id,
                    now_ms,
                    RevokeReason::TimeoutNoInput.as_str(),
                    threshold
                ],
            )
            .map_err(sqlite_err)?;
        Ok(changed)
    }

    /// Cập nhật `last_seen_at_ms`.
    fn touch_session(&mut self, id: SessionId, now_ms: i64) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute(TOUCH_SESSION, params![id.0, now_ms])
            .map_err(sqlite_err)?;
        Ok(changed > 0)
    }
}

impl std::fmt::Debug for SqliteAccountStore {
    /// Chỉ in đường dẫn DB — không in nội dung connection (có thể chứa dữ liệu tài khoản).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteAccountStore")
            .field("db_path", &self.db_path)
            .finish_non_exhaustive()
    }
}

impl Drop for SqliteAccountStore {
    /// Nhả khoá instance **tường minh** trước khi đóng file khoá.
    ///
    /// `flock` gắn với *open file description* (không phải với fd), và mô tả đó bị **kế
    /// thừa** khi tiến trình `fork` — trong khoảng giữa `fork` và `exec`, `O_CLOEXEC` chưa
    /// có tác dụng. Nếu chỉ đóng fd thì khoá **vẫn còn** tới khi đứa trẻ `exec` xong ⇒
    /// instance mới thấy DB "đang bị khoá" và từ chối khởi động với `db_locked`
    /// (ADR 0011 §4) dù instance cũ đã thoát.
    ///
    /// `flock(LOCK_UN)` tường minh nhả khoá của **cả** mô tả ⇒ việc giải phóng là tức thời
    /// và không phụ thuộc đứa trẻ. Lỗi `unlock` bị bỏ qua: khoá vẫn được nhả khi fd đóng.
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self._instance_lock);
    }
}

impl AccountStore for SqliteAccountStore {
    fn create_account(&mut self, account: NewAccount) -> Result<Account, StoreError> {
        let key = username_key(&account.username)?;
        let display = account.username.trim().to_string();
        let now = account.created_at_ms;

        self.conn
            .execute(
                INSERT_ACCOUNT,
                params![display, key, account.password_hash.expose(), now],
            )
            .map_err(map_constraint)?;
        let id = self.conn.last_insert_rowid();

        Ok(Account {
            id: AccountId(id),
            username: display,
            username_key: key,
            password_hash: account.password_hash,
            created_at_ms: now,
            updated_at_ms: now,
            disabled: false,
        })
    }

    fn account_by_key(&self, username_key: &str) -> Result<Option<Account>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached(SELECT_ACCOUNT_BY_KEY)
            .map_err(sqlite_err)?;
        stmt.query_row([username_key], account_from_row)
            .optional()
            .map_err(sqlite_err)
    }

    fn account_by_username(&self, username: &str) -> Result<Option<Account>, StoreError> {
        let key = username_key(username)?;
        self.account_by_key(&key)
    }

    fn account_by_id(&self, id: AccountId) -> Result<Option<Account>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached(SELECT_ACCOUNT_BY_ID)
            .map_err(sqlite_err)?;
        stmt.query_row([id.0], account_from_row)
            .optional()
            .map_err(sqlite_err)
    }

    fn set_account_disabled(
        &mut self,
        id: AccountId,
        disabled: bool,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute(
                UPDATE_ACCOUNT_DISABLED,
                params![id.0, i64::from(disabled), now_ms],
            )
            .map_err(sqlite_err)?;
        Ok(changed > 0)
    }

    /// Ghi lại PHC string (rehash khi tham số Argon2 đổi, ADR 0011 §5).
    fn update_password_hash(
        &mut self,
        id: AccountId,
        password_hash: &str,
        now_ms: i64,
    ) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute(UPDATE_PASSWORD_HASH, params![id.0, password_hash, now_ms])
            .map_err(sqlite_err)?;
        Ok(changed > 0)
    }
}

fn account_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: AccountId(row.get(0)?),
        username: row.get(1)?,
        username_key: row.get(2)?,
        password_hash: super::Secret::new(row.get(3)?),
        created_at_ms: row.get(4)?,
        updated_at_ms: row.get(5)?,
        disabled: row.get::<_, i64>(6)? != 0,
    })
}

fn session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: SessionId(row.get(0)?),
        account_id: AccountId(row.get(1)?),
        instance_id: row.get(2)?,
        connection_id: row.get(3)?,
        created_at_ms: row.get(4)?,
        expires_at_ms: row.get(5)?,
        last_seen_at_ms: row.get(6)?,
        revoked_at_ms: row.get(7)?,
        revoke_reason: row.get(8)?,
    })
}

/// Đọc `PRAGMA user_version` — version schema của app (ADR 0011 §4).
fn user_version(conn: &Connection) -> Result<i64, StoreError> {
    conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(sqlite_err)
}

/// Chạy migration forward-only (ADR 0011 §4).
///
/// Mỗi bước: một transaction gồm DDL **và** bump `user_version` ⇒ không nửa vời. Lỗi bước
/// nào ⇒ rollback transaction đó và store không mở được.
fn run_migrations(
    conn: &mut Connection,
    db_path: &Path,
    current_version: i64,
) -> Result<(), StoreError> {
    let pending: Vec<(i64, &str)> = MIGRATIONS
        .iter()
        .filter(|(version, _)| *version > current_version)
        .copied()
        .collect();
    if pending.is_empty() {
        return Ok(());
    }

    // ADR 0011 §4: sao lưu trước khi chạy migration.
    backup_before_migration(conn, db_path, current_version)?;

    for (version, sql) in pending {
        let tx = conn.transaction().map_err(sqlite_err)?;
        tx.execute_batch(sql).map_err(sqlite_err)?;
        tx.pragma_update(None, "user_version", version)
            .map_err(sqlite_err)?;
        tx.commit().map_err(sqlite_err)?;
    }
    Ok(())
}

/// Sao lưu DB trước migration (ADR 0011 §4).
///
/// DB mới (chưa có bảng nào của app) thì không có gì để sao lưu. Trả `None` nghĩa là không
/// tạo bản sao.
fn backup_before_migration(
    conn: &Connection,
    db_path: &Path,
    from_version: i64,
) -> Result<Option<PathBuf>, StoreError> {
    if !has_user_tables(conn)? {
        return Ok(None);
    }
    let target = next_backup_path(db_path, from_version);
    vacuum_into(conn, &target)?;
    Ok(Some(target))
}

/// DB đã có bảng của app hay chưa (`sqlite_master`, bỏ qua bảng hệ thống `sqlite_*`).
fn has_user_tables(conn: &Connection) -> Result<bool, StoreError> {
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .map_err(sqlite_err)?;
    Ok(count > 0)
}

/// Chọn đường dẫn bản sao chưa tồn tại: `<db>.backup-v<from>-<n>.sqlite`.
///
/// Instance lock bảo đảm chỉ một tiến trình chạy trên một file DB, nên không có race khi
/// chọn tên.
fn next_backup_path(db_path: &Path, from_version: i64) -> PathBuf {
    let dir = db_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stem = db_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("boom-server.sqlite"));
    for index in 0..1_000_u32 {
        let candidate = dir.join(format!("{stem}.backup-v{from_version}-{index}.sqlite"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}.backup-v{from_version}.sqlite"))
}

/// `VACUUM INTO 'file'` — bản sao tối giản của DB (ADR 0011 §4).
///
/// File đích được chuẩn bị trước: xem [`prepare_backup_target`]. Nếu `VACUUM INTO` lỗi
/// thì file đích dở dang bị xoá (SQLite không tự dọn) để không để lại bản sao nửa vời.
fn vacuum_into(conn: &Connection, target: &Path) -> Result<(), StoreError> {
    let path = target
        .to_str()
        .ok_or_else(|| StoreError::Io(String::from("đường dẫn bản sao không phải UTF-8")))?;
    // Tạo file đích owner-only **trước khi** SQLite ghi dữ liệu vào (ADR 0011 §3).
    prepare_backup_target(target)?;
    match conn.execute("VACUUM INTO ?1", [path]) {
        Ok(_) => Ok(()),
        Err(err) => {
            // File này do `prepare_backup_target` tạo ⇒ xoá được, không đụng file có sẵn.
            let _ = std::fs::remove_file(target);
            Err(sqlite_err(err))
        }
    }
}

/// Chuẩn bị file đích cho `VACUUM INTO`: tạo file **rỗng** với quyền owner-only
/// **trước khi** SQLite ghi dữ liệu vào (ADR 0011 §3, threat §9 dòng 7).
///
/// Vì sao không để `VACUUM INTO` tự tạo file: SQLite tạo file mới theo quyền mặc định
/// `0666 & ~umask` ⇒ dưới umask 022 (thông thường) file bản sao là `0644`, tức user khác
/// trên cùng máy đọc được `accounts.password_hash` và `sessions.token_hash` trong khi
/// file DB gốc đã ở `0600`. Ở đây file đích được tạo trước (rỗng, `0600`) rồi mới cho
/// SQLite ghi vào: SQLite chỉ từ chối khi file đích **không rỗng**, nên file rỗng
/// owner-only vẫn là đích hợp lệ và giữ nguyên quyền `0600` sau khi ghi.
///
/// File đích đã tồn tại ⇒ **không ghi đè**, trả [`StoreError::BackupTargetExists`]
/// (bản sao cũ là dữ liệu, không phải thư mục rác).
fn prepare_backup_target(target: &Path) -> Result<(), StoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(OWNER_ONLY_MODE);
    }
    match options.open(target) {
        Ok(_) => {
            // Chốt đúng `0600` bất kể umask: umask lạ (ví dụ `0177`) có thể cắt quyền ghi
            // của owner ⇒ SQLite không ghi được. Vẫn làm **trước** khi SQLite ghi dữ liệu.
            restrict_permissions(target)?;
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(StoreError::BackupTargetExists {
                path: target.display().to_string(),
            })
        }
        Err(err) => Err(io_err(err)),
    }
}

/// `PRAGMA integrity_check` — `ok` nghĩa là file DB toàn vẹn (ADR 0011 §4).
fn integrity_check(conn: &Connection) -> Result<(), StoreError> {
    let mut stmt = conn.prepare("PRAGMA integrity_check").map_err(sqlite_err)?;
    let mut rows = stmt.query([]).map_err(sqlite_err)?;
    let mut report = Vec::new();
    while let Some(row) = rows.next().map_err(sqlite_err)? {
        report.push(row.get::<_, String>(0).map_err(sqlite_err)?);
    }
    if report.iter().any(|line| line != "ok") {
        return Err(StoreError::IntegrityCheckFailed {
            report: report.join("; "),
        });
    }
    Ok(())
}

/// Giữ khóa advisory độc quyền trên file `<data_dir>/boom-server.lock` (ADR 0011 §4).
///
/// Không giữ được ⇒ instance khác đang chạy trên file DB này ⇒ từ chối khởi động.
fn acquire_instance_lock(dir: &Path) -> Result<File, StoreError> {
    let path = dir.join(LOCK_FILE_NAME);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(io_err)?;
    match FileExt::try_lock(&file) {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(StoreError::DatabaseLocked),
        Err(TryLockError::Error(err)) => Err(StoreError::Io(err.to_string())),
    }
}

/// Tạo file DB nếu chưa tồn tại; trên POSIX tạo với quyền `0600` ngay từ đầu (ADR 0011 §3).
fn ensure_db_file(path: &Path) -> Result<(), StoreError> {
    if path.exists() {
        return Ok(());
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(OWNER_ONLY_MODE);
    }
    options.open(path).map_err(io_err)?;
    Ok(())
}

/// File owner-only trên POSIX (ADR 0011 §3). Trên Windows không làm gì.
///
/// Windows không có `0600` tương đương: bảo vệ dựa vào ACL của thư mục dữ liệu
/// (giới hạn đã biết, xem ADR 0011 §3).
#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(OWNER_ONLY_MODE)).map_err(io_err)
}

/// File DB owner-only trên POSIX (ADR 0011 §3). Trên Windows không làm gì.
#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

fn sqlite_err(err: rusqlite::Error) -> StoreError {
    StoreError::Sqlite(err.to_string())
}

fn io_err(err: std::io::Error) -> StoreError {
    StoreError::Io(err.to_string())
}

/// Dịch lỗi ràng buộc UNIQUE thành lỗi miền.
///
/// SQLite không trả tên ràng buộc qua mã lỗi, chỉ trả trong thông báo. Thông báo của một
/// vi phạm UNIQUE là `UNIQUE constraint failed: <bảng>.<cột>` — kể cả khi ràng buộc do
/// **partial index** đặt (SQLite báo cột của index, không báo tên index). Vì vậy hai ràng
/// buộc được phân biệt bằng cột: `accounts.username_key` (tên đã có) và
/// `sessions.account_id` (đã có phiên hiệu lực theo index
/// `sessions_one_active_per_account`).
fn map_constraint(err: rusqlite::Error) -> StoreError {
    if err.sqlite_extended_error_code() == Some(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE)
        && let rusqlite::Error::SqliteFailure(_, Some(message)) = &err
    {
        if message.contains("sessions_one_active_per_account")
            || message.contains("sessions.account_id")
        {
            return StoreError::AccountHasActiveSession;
        }
        if message.contains("accounts.username_key") || message.contains("username_key") {
            return StoreError::UsernameTaken;
        }
    }
    StoreError::Sqlite(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AccountStore, MIGRATIONS, SUPPORTED_SCHEMA_VERSION, SqliteAccountStore, SqliteOptions,
    };
    use crate::identity::{
        AccountId, NewAccount, NewSession, Secret, SessionStore, StoreError, username_key,
    };
    use rusqlite::params;
    use tempfile::TempDir;

    fn db_path(dir: &TempDir) -> std::path::PathBuf {
        dir.path().join("boom-server.sqlite")
    }

    fn open(dir: &TempDir) -> SqliteAccountStore {
        SqliteAccountStore::open(&db_path(dir), SqliteOptions::default()).expect("mở store")
    }

    fn new_account(username: &str) -> NewAccount {
        NewAccount {
            username: username.to_string(),
            password_hash: Secret::new(String::from("$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFza")),
            created_at_ms: 1_700_000_000_000,
        }
    }

    fn new_session(account_id: AccountId, token: &[u8]) -> NewSession {
        NewSession {
            account_id,
            token_hash: token.to_vec(),
            instance_id: String::from("instance-1"),
            connection_id: Some(String::from("conn-1")),
            created_at_ms: 1_700_000_000_000,
            expires_at_ms: 1_700_000_000_000 + 86_400_000,
            last_seen_at_ms: 1_700_000_000_000,
        }
    }

    /// Tìm đúng một file bản sao trong thư mục dữ liệu (tiền tố tên file).
    fn find_backup_file(dir: &TempDir, prefix: &str) -> std::path::PathBuf {
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
            .collect();
        assert_eq!(backups.len(), 1, "phải có đúng một file bản sao `{prefix}`");
        backups[0].path()
    }

    /// Quyền file (chỉ bit quyền) trên POSIX.
    #[cfg(unix)]
    fn file_mode(path: &std::path::Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    /// Biến môi trường đánh dấu "đang chạy ở tiến trình con" (xem
    /// [`run_test_in_child_with_umask`]).
    #[cfg(unix)]
    const UMASK_CHILD_ENV: &str = "BOOM_IDENTITY_TEST_UMASK_CHILD";

    /// `umask` là trạng thái **của tiến trình**: đổi nó trong tiến trình test đang chạy
    /// song song sẽ làm hỏng test khác. Vì vậy ca cần umask xác định chạy ở **tiến trình
    /// con**: tiến trình con tự đặt umask rồi chạy đúng một test (`--exact`).
    #[cfg(unix)]
    fn is_umask_child() -> bool {
        std::env::var_os(UMASK_CHILD_ENV).is_some()
    }

    /// Tên đầy đủ của test trong test binary (đường dẫn tương đối crate, như libtest báo).
    #[cfg(unix)]
    fn test_path(name: &str) -> String {
        let module = module_path!();
        let crate_name = module.split("::").next().unwrap_or(module);
        let relative = module
            .strip_prefix(crate_name)
            .map(|rest| rest.trim_start_matches("::"))
            .unwrap_or(module);
        format!("{relative}::{name}")
    }

    /// Chạy một test ở tiến trình con với umask chỉ định.
    ///
    /// Tiến trình cha **không** đổi umask của mình (test chạy song song); umask được đặt
    /// trong tiến trình con, ngay trước khi nó chạy test.
    #[cfg(unix)]
    fn run_test_in_child_with_umask(test_name: &str, umask: &str) {
        use std::process::Command;

        let exe = std::env::current_exe().expect("đường dẫn test binary");
        // `sh -c 'umask "$1"; shift; exec "$@"' sh <umask> <exe> <test> --exact --nocapture`
        let output = Command::new("sh")
            .arg("-c")
            .arg("umask \"$1\"; shift; exec \"$@\"")
            .arg("sh")
            .arg(umask)
            .arg(exe)
            .arg(test_name)
            .arg("--exact")
            .arg("--nocapture")
            .env(UMASK_CHILD_ENV, "1")
            .output()
            .expect("chạy tiến trình test con");
        assert!(
            output.status.success(),
            "test con `{test_name}` (umask {umask}) thất bại\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn migration_creates_schema_v1_on_real_file() {
        let dir = TempDir::new().unwrap();
        let store = open(&dir);

        assert_eq!(store.schema_version().unwrap(), SUPPORTED_SCHEMA_VERSION);

        // Bảng và index phải có trong file thật.
        let names: Vec<String> = store
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','index')")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        assert!(names.iter().any(|n| n == "accounts"));
        assert!(names.iter().any(|n| n == "sessions"));
        assert!(names.iter().any(|n| n == "sessions_one_active_per_account"));

        // Pragma theo ADR 0011 §3.
        let journal: String = store
            .conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(journal, "wal");
        let foreign_keys: i64 = store
            .conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
        let synchronous: i64 = store
            .conn
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        assert_eq!(synchronous, 2); // FULL (0 = OFF, 1 = NORMAL, 2 = FULL, 3 = EXTRA)
    }

    #[test]
    fn migration_is_idempotent_across_reopen() {
        let dir = TempDir::new().unwrap();
        {
            let mut store = open(&dir);
            store.create_account(new_account("bomber")).unwrap();
        }
        // Mở lại: không chạy lại migration, dữ liệu còn nguyên.
        let store = open(&dir);
        assert_eq!(store.schema_version().unwrap(), SUPPORTED_SCHEMA_VERSION);
        assert!(store.account_by_username("bomber").unwrap().is_some());
    }

    #[test]
    fn unsupported_user_version_is_rejected() {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", 99_i64).unwrap();
        }

        let err = SqliteAccountStore::open(&path, SqliteOptions::default()).unwrap_err();
        assert_eq!(
            err,
            StoreError::UnsupportedSchema {
                found: 99,
                supported: SUPPORTED_SCHEMA_VERSION
            }
        );
    }

    #[test]
    fn negative_user_version_is_rejected() {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", -1_i64).unwrap();
        }

        let err = SqliteAccountStore::open(&path, SqliteOptions::default()).unwrap_err();
        assert!(matches!(err, StoreError::UnsupportedSchema { .. }));
    }

    #[test]
    fn account_key_is_trimmed_and_lowercased() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);

        let account = store.create_account(new_account("  BoMber  ")).unwrap();
        assert_eq!(account.username, "BoMber");
        assert_eq!(account.username_key, "bomber");
        assert!(!account.disabled);

        // Tra theo key chuẩn hóa, không phân biệt hoa thường.
        assert!(store.account_by_key("bomber").unwrap().is_some());
        assert!(store.account_by_username("BOMBER").unwrap().is_some());
        assert_eq!(
            store.account_by_username("bomber").unwrap().unwrap().id,
            account.id
        );
        assert!(store.account_by_id(account.id).unwrap().is_some());
        assert_eq!(store.account_by_username("other").unwrap(), None);
    }

    #[test]
    fn invalid_username_is_rejected() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        assert_eq!(
            store.create_account(new_account("ab")).unwrap_err(),
            StoreError::InvalidUsername
        );
        assert_eq!(
            store.account_by_username("a b").unwrap_err(),
            StoreError::InvalidUsername
        );
    }

    #[test]
    fn duplicate_username_key_is_rejected() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        store.create_account(new_account("Bob")).unwrap();
        assert_eq!(
            store.create_account(new_account("bob")).unwrap_err(),
            StoreError::UsernameTaken
        );
    }

    #[test]
    fn second_active_session_for_account_is_rejected() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let account = store.create_account(new_account("bomber")).unwrap();

        store
            .create_session(new_session(account.id, b"token-1"))
            .unwrap();
        assert_eq!(
            store
                .create_session(new_session(account.id, b"token-2"))
                .unwrap_err(),
            StoreError::AccountHasActiveSession
        );

        // Phiên đã giải phóng thì không còn giữ chỗ ⇒ phiên mới được nhận.
        store
            .conn
            .execute(
                "UPDATE sessions SET revoked_at_ms = ?1, revoke_reason = 'logout' WHERE account_id = ?2",
                params![1_700_000_001_000_i64, account.id.0],
            )
            .unwrap();
        store
            .create_session(new_session(account.id, b"token-2"))
            .unwrap();
        assert!(store.active_session(account.id).unwrap().is_some());
    }

    #[test]
    fn second_instance_lock_fails() {
        let dir = TempDir::new().unwrap();
        let _first = open(&dir);

        let err = SqliteAccountStore::open(&db_path(&dir), SqliteOptions::default()).unwrap_err();
        assert_eq!(err, StoreError::DatabaseLocked);
    }

    #[test]
    fn backup_is_reopenable_and_passes_integrity_check() {
        let dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        store.create_account(new_account("bomber")).unwrap();

        let target = backup_dir.path().join("backup.sqlite");
        store.backup_to(&target).unwrap();
        assert!(target.exists());

        // Bản sao mở lại được và còn dữ liệu.
        let conn = rusqlite::Connection::open(&target).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");

        // DB gốc cũng phải toàn vẹn.
        store.integrity_check().unwrap();
    }

    /// Bản sao thủ công phải owner-only **ngay từ lúc tạo**, kể cả dưới umask thông thường
    /// (`022`): bản sao chứa `accounts.password_hash` và `sessions.token_hash` (review PR #68
    /// P2 — trước đây file bản sao là `0644` do `VACUUM INTO` tự tạo theo umask).
    #[cfg(unix)]
    #[test]
    fn backup_file_is_owner_only_under_typical_umask() {
        let name = test_path("backup_file_is_owner_only_under_typical_umask");
        if !is_umask_child() {
            // Tiến trình cha: chạy lại đúng test này ở tiến trình con có umask `022`.
            run_test_in_child_with_umask(&name, "022");
            return;
        }

        // Tiến trình con: umask `022` ⇒ file tạo theo mặc định sẽ là `0644`.
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        store.create_account(new_account("bomber")).unwrap();
        let target = dir.path().join("backup.sqlite");
        store.backup_to(&target).unwrap();
        assert_eq!(file_mode(&target), 0o600);
    }

    /// Backup trước migration cũng phải owner-only **ngay từ lúc tạo** (review PR #68 P2).
    #[cfg(unix)]
    #[test]
    fn pre_migration_backup_file_is_owner_only_under_typical_umask() {
        let name = test_path("pre_migration_backup_file_is_owner_only_under_typical_umask");
        if !is_umask_child() {
            run_test_in_child_with_umask(&name, "022");
            return;
        }

        let dir = TempDir::new().unwrap();
        let path = db_path(&dir);
        {
            // DB có sẵn bảng của app ở version 0 ⇒ migration phải sao lưu trước.
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE legacy_notes (id INTEGER PRIMARY KEY)")
                .unwrap();
        }

        let _store = SqliteAccountStore::open(&path, SqliteOptions::default()).unwrap();
        let backup = find_backup_file(&dir, "boom-server.sqlite.backup-v0");
        assert_eq!(file_mode(&backup), 0o600);
    }

    /// Bản sao owner-only bất kể umask của tiến trình (quyền chốt đúng `0600`, không chỉ
    /// `0600 & ~umask`).
    #[cfg(unix)]
    #[test]
    fn backup_file_mode_is_exactly_owner_only() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        store.create_account(new_account("bomber")).unwrap();
        let target = dir.path().join("backup.sqlite");
        store.backup_to(&target).unwrap();
        assert_eq!(file_mode(&target), 0o600);
    }

    /// Bản sao cũ không bao giờ bị ghi đè: file đích đã tồn tại ⇒ lỗi, nội dung cũ còn
    /// nguyên (ADR 0011 §4).
    #[test]
    fn backup_refuses_to_overwrite_existing_file() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        store.create_account(new_account("bomber")).unwrap();

        let target = dir.path().join("backup.sqlite");
        let existed = b"ban sao cu khong duoc ghi de";
        std::fs::write(&target, existed).unwrap();

        let err = store.backup_to(&target).unwrap_err();
        assert!(
            matches!(err, StoreError::BackupTargetExists { .. }),
            "lỗi lạ: {err}"
        );
        assert_eq!(std::fs::read(&target).unwrap(), existed);
    }

    /// File bản sao đã tồn tại nhưng **rỗng** cũng không bị dùng lại: cùng lý do "không ghi
    /// đè" — bản sao là dữ liệu, không phải file rỗng để điền.
    #[test]
    fn backup_refuses_to_reuse_existing_empty_file() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        store.create_account(new_account("bomber")).unwrap();

        let target = dir.path().join("backup.sqlite");
        std::fs::write(&target, b"").unwrap();

        let err = store.backup_to(&target).unwrap_err();
        assert!(
            matches!(err, StoreError::BackupTargetExists { .. }),
            "lỗi lạ: {err}"
        );
        assert_eq!(std::fs::metadata(&target).unwrap().len(), 0);
    }

    /// Windows không có quyền POSIX `0600`: bản sao vẫn phải tạo được và mở lại được
    /// (giới hạn: bảo vệ dựa vào ACL của thư mục, xem ADR 0011 §3).
    #[cfg(not(unix))]
    #[test]
    fn backup_file_is_created_and_reopenable_without_posix_modes() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        store.create_account(new_account("bomber")).unwrap();
        let target = dir.path().join("backup.sqlite");
        store.backup_to(&target).unwrap();
        assert!(target.exists());

        let conn = rusqlite::Connection::open(&target).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_backs_up_existing_database_first() {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir);
        {
            // DB có sẵn bảng của app ở version 0 (trạng thái trước migration).
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE legacy_notes (id INTEGER PRIMARY KEY)")
                .unwrap();
        }

        let store = SqliteAccountStore::open(&path, SqliteOptions::default()).unwrap();
        assert_eq!(store.schema_version().unwrap(), SUPPORTED_SCHEMA_VERSION);

        let backup_path = find_backup_file(&dir, "boom-server.sqlite.backup-v0");
        let conn = rusqlite::Connection::open(&backup_path).unwrap();
        let legacy: i64 = conn
            .query_row("SELECT count(*) FROM legacy_notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(legacy, 0);
    }

    #[test]
    fn supported_version_matches_last_migration() {
        let last = MIGRATIONS.last().map(|(version, _)| *version).unwrap();
        assert_eq!(last, SUPPORTED_SCHEMA_VERSION);
    }

    #[test]
    fn db_file_is_owner_only_on_posix() {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir);
        let _store = SqliteAccountStore::open(&path, SqliteOptions::default()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        #[cfg(not(unix))]
        assert!(path.exists());
    }

    #[test]
    fn account_can_be_disabled() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let account = store.create_account(new_account("bomber")).unwrap();
        assert!(store.set_account_disabled(account.id, true, 42).unwrap());
        assert!(store.account_by_id(account.id).unwrap().unwrap().disabled);
        assert!(
            !store
                .set_account_disabled(AccountId(9999), true, 42)
                .unwrap()
        );
    }

    #[test]
    fn username_key_helper_is_shared_with_store() {
        assert_eq!(username_key(" BoMber ").unwrap(), "bomber");
    }

    /// Khoá instance phải được nhả **tức thời** khi store bị drop — kể cả khi còn fd khác
    /// trỏ vào **cùng** *open file description*.
    ///
    /// `flock` gắn với *open file description*, không phải với fd: chỉ đóng fd thì khoá **còn**
    /// hiệu lực tới khi *mọi* fd của mô tả đó đóng. Đây đúng là tình huống fd khoá bị **kế
    /// thừa** sang tiến trình con trong khoảng giữa `fork` và `exec` — khi đó instance mới thấy
    /// DB "đang bị khoá" và từ chối khởi động với `db_locked` (ADR 0011 §4).
    ///
    /// `SqliteAccountStore::drop` gọi `unlock` tường minh nên khoá được nhả ngay ⇒ instance
    /// mới mở được DB.
    #[test]
    fn instance_lock_is_released_on_drop_even_with_another_fd_on_same_description() {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir);
        let inherited = {
            let store = SqliteAccountStore::open(&path, SqliteOptions::default()).unwrap();
            // `try_clone` ⇒ fd khác nhưng **cùng** open file description.
            store
                ._instance_lock
                .try_clone()
                .expect("clone fd khoá instance")
        };
        // Store đã drop, mô tả vẫn còn fd ⇒ nếu chỉ đóng fd thì khoá còn hiệu lực.
        let store = SqliteAccountStore::open(&path, SqliteOptions::default())
            .expect("instance mới phải mở được DB");
        drop(store);
        drop(inherited);
    }

    /// DB **chỉ** lưu SHA-256(token thô), không lưu token thô (ADR 0011 §6, threat §9 dòng 3).
    #[test]
    fn db_stores_only_token_hash_not_raw_token() {
        let dir = TempDir::new().unwrap();
        let mut store = open(&dir);
        let account = store.create_account(new_account("bomber")).unwrap();

        // Token thô giả, đúng dạng base64url 43 ký tự.
        let raw_token = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let hash = crate::identity::token_hash(raw_token);
        assert_eq!(hash.len(), 32);

        store
            .create_session(NewSession {
                account_id: account.id,
                token_hash: hash.clone(),
                instance_id: String::from("instance-1"),
                connection_id: Some(String::from("conn-1")),
                created_at_ms: 1_700_000_000_000,
                expires_at_ms: 1_700_000_000_000 + 86_400_000,
                last_seen_at_ms: 1_700_000_000_000,
            })
            .unwrap();

        // Tra theo hash thì ra phiên; tra theo token thô thì không ra.
        let found = store.session_by_token_hash(&hash).unwrap();
        assert!(found.is_some());
        assert!(
            store
                .session_by_token_hash(raw_token.as_bytes())
                .unwrap()
                .is_none()
        );

        // Cột `token_hash` trong DB không chứa token thô.
        let stored: Vec<u8> = store
            .conn
            .query_row("SELECT token_hash FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(stored, hash);
        assert!(
            !stored
                .windows(raw_token.len())
                .any(|w| w == raw_token.as_bytes())
        );
    }
}
