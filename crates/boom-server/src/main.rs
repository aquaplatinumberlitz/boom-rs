//! CLI server V1: `health` runtime/offline (ADR 0011 §8.2), `account revoke-sessions`
//! (ADR 0011 §6) và `config` (đọc/ghi cấu hình vận hành).
//!
//! Game listener/content startup thuộc lớp triển khai P7; `serve` hiện mở admin listener + heartbeat
//! để health runtime có nguồn sự thật là tiến trình, không chỉ đọc file.

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use boom_server::identity::{
    AccountId, AccountStore, RevokeReason, SessionService, SqliteAccountStore, SqliteOptions,
    StoreError,
};
use boom_server::{
    AdminServer, HealthState, Heartbeat, LogFields, Logger, OperationalConfig, events,
    new_instance_id, now_ms, run_health_cli,
};
use serde_json::json;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("health") => health_command(&args),
        Some("account") => account_command(&args[1..]),
        Some("config") => config_command(&args[1..]),
        None => serve_command(&[]),
        Some("serve") => serve_command(&args[1..]),
        Some(other) => {
            eprintln!(
                "lệnh không hỗ trợ: {other}; dùng: boom-server health [--offline] | \
                 account revoke-sessions <username> | config [--out <file>] | serve"
            );
            ExitCode::from(3)
        }
    }
}

fn parse_health_args(args: &[String]) -> Result<bool, String> {
    let mut offline = false;
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--offline" => offline = true,
            other => return Err(format!("tham số health không hỗ trợ: {other}")),
        }
    }
    Ok(offline)
}

fn health_command(args: &[String]) -> ExitCode {
    let offline = match parse_health_args(args) {
        Ok(offline) => offline,
        Err(message) => {
            eprintln!("{message}; dùng: boom-server health [--offline]");
            return ExitCode::from(3);
        }
    };
    let config = match OperationalConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("lỗi cấu hình: {err}");
            return ExitCode::from(3);
        }
    };
    let outcome = run_health_cli(&config, offline);
    if outcome.code == 3 {
        let message = outcome
            .body
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or("lỗi CLI health");
        eprintln!("{message}");
        return ExitCode::from(3);
    }
    match serde_json::to_string(&outcome.body) {
        Ok(body) => println!("{body}"),
        Err(err) => {
            eprintln!("lỗi serialize health: {err}");
            return ExitCode::from(3);
        }
    }
    ExitCode::from(u8::try_from(outcome.code).unwrap_or(3))
}

/// Lệnh quản trị `account` (ADR 0011 §6).
fn account_command(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("revoke-sessions") => revoke_sessions_command(&args[1..]),
        Some(other) => {
            eprintln!(
                "lệnh account không hỗ trợ: {other}; dùng: boom-server account revoke-sessions <username>"
            );
            ExitCode::from(3)
        }
        None => {
            eprintln!("thiếu lệnh con; dùng: boom-server account revoke-sessions <username>");
            ExitCode::from(3)
        }
    }
}

/// `boom-server account revoke-sessions <username>` (ADR 0011 §6).
///
/// Đường **dự phòng**: người chơi bị đứt mạng hoặc server restart tự vào lại được mà không
/// cần admin (§6.1); lệnh này chỉ dùng khi cần thu hồi thủ công.
///
/// Mã thoát: `0` thành công · `1` không tìm thấy tài khoản · `2` DB đang bị khoá (server
/// đang chạy) hoặc lỗi DB · `3` sai cú pháp/lỗi cấu hình.
fn revoke_sessions_command(args: &[String]) -> ExitCode {
    let username = match args {
        [username] if !username.trim().is_empty() => username.trim().to_owned(),
        _ => {
            eprintln!("dùng: boom-server account revoke-sessions <username>");
            return ExitCode::from(3);
        }
    };
    let config = match OperationalConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("lỗi cấu hình: {err}");
            return ExitCode::from(3);
        }
    };
    match revoke_sessions(&config, &username) {
        Ok(outcome) => {
            println!(
                "{}",
                json!({
                    "account_id": outcome.account_id,
                    "username": outcome.username,
                    "revoked_sessions": outcome.revoked,
                })
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", account_error_message(&error, &username));
            ExitCode::from(account_exit_code(&error))
        }
    }
}

/// Mã thoát của lệnh `account` (ADR 0011 §4: instance khác giữ khoá ⇒ exit 2).
fn account_exit_code(error: &AccountCliError) -> u8 {
    match error {
        AccountCliError::AccountNotFound => 1,
        AccountCliError::DatabaseLocked | AccountCliError::Store(_) => 2,
    }
}

/// Thông báo lỗi cho người vận hành (không chứa bí mật).
fn account_error_message(error: &AccountCliError, username: &str) -> String {
    match error {
        AccountCliError::DatabaseLocked => String::from(
            "không thu hồi được: file DB đang bị instance khác giữ khóa \
             (server đang chạy) — dừng server trước khi thu hồi phiên",
        ),
        AccountCliError::AccountNotFound => format!("không tìm thấy tài khoản: {username}"),
        AccountCliError::Store(err) => format!("lỗi khi thu hồi phiên: {err}"),
    }
}

/// Lỗi của lệnh `account revoke-sessions`.
#[derive(Clone, Debug, PartialEq, Eq)]
enum AccountCliError {
    /// File DB đang bị instance khác giữ khóa ⇒ server đang chạy (ADR 0011 §4).
    DatabaseLocked,
    /// Không tìm thấy tài khoản.
    AccountNotFound,
    /// Lỗi tầng lưu trữ khác.
    Store(StoreError),
}

/// Kết quả thu hồi phiên của một tài khoản.
#[derive(Clone, Debug, PartialEq, Eq)]
struct RevokeOutcome {
    /// Định danh tài khoản.
    account_id: i64,
    /// Tên hiển thị của tài khoản.
    username: String,
    /// Số phiên đã thu hồi.
    revoked: usize,
}

/// Thu hồi mọi phiên còn hiệu lực của một tài khoản (ADR 0011 §6).
///
/// Mở DB **có giữ khoá instance** (ADR 0011 §4): nếu server đang chạy thì mở không được ⇒
/// báo lỗi rõ ràng thay vì sửa DB khi tiến trình server còn giữ phiên đó.
fn revoke_sessions(
    config: &OperationalConfig,
    username: &str,
) -> Result<RevokeOutcome, AccountCliError> {
    let mut store = SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).map_err(
        |err| match err {
            StoreError::DatabaseLocked => AccountCliError::DatabaseLocked,
            other => AccountCliError::Store(other),
        },
    )?;
    let account = store
        .account_by_username(username)
        .map_err(|err| match err {
            // Tên không hợp lệ ⇒ không tài khoản nào mang tên đó (ADR 0011 §4).
            StoreError::InvalidUsername => AccountCliError::AccountNotFound,
            other => AccountCliError::Store(other),
        })?
        .ok_or(AccountCliError::AccountNotFound)?;
    let now = i64::try_from(now_ms()).unwrap_or(i64::MAX);
    let revoked = SessionService::new(new_instance_id())
        .revoke_account_sessions(&mut store, account.id, now)
        .map_err(AccountCliError::Store)?;
    log_session_revoked(config, account.id, revoked);
    Ok(RevokeOutcome {
        account_id: account.id.0,
        username: account.username.clone(),
        revoked,
    })
}

/// Ghi sự kiện `session.revoked` với lý do `admin_revoke` (ADR 0011 §6.1, §8.1).
///
/// Log của lệnh một lần chạy đi ra **stderr**: stdout là kết quả của lệnh (JSON), để
/// script đọc được mà không phải lọc log.
fn log_session_revoked(config: &OperationalConfig, account_id: AccountId, revoked: usize) {
    let logger = Logger::new(
        config.fingerprint(),
        new_instance_id(),
        config.content_epoch,
        Box::new(std::io::stderr()),
    );
    let mut fields = LogFields::new();
    fields.insert("account_id", json!(account_id.0));
    fields.insert("revoke_reason", json!(RevokeReason::AdminRevoke.as_str()));
    fields.insert("revoked_sessions", json!(revoked));
    logger.info(events::SESSION_REVOKED, fields);
}

/// `boom-server config [--out <file>]` — xuất/ghi cấu hình hiệu lực (ADR 0011 §5, §8).
fn config_command(args: &[String]) -> ExitCode {
    let mut out: Option<String> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--out" => match rest.next() {
                Some(path) if !path.trim().is_empty() => out = Some(path.trim().to_owned()),
                _ => {
                    eprintln!("--out cần đường dẫn file; dùng: boom-server config [--out <file>]");
                    return ExitCode::from(3);
                }
            },
            other => {
                eprintln!(
                    "tham số config không hỗ trợ: {other}; dùng: boom-server config [--out <file>]"
                );
                return ExitCode::from(3);
            }
        }
    }
    let config = match OperationalConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("lỗi cấu hình: {err}");
            return ExitCode::from(3);
        }
    };
    match out {
        Some(path) => match config.write_to_file(Path::new(&path)) {
            Ok(()) => {
                println!("{}", json!({"written": path}));
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("lỗi ghi file cấu hình: {err}");
                ExitCode::from(3)
            }
        },
        None => match config.to_toml_string() {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("lỗi xuất cấu hình: {err}");
                ExitCode::from(3)
            }
        },
    }
}

fn serve_command(args: &[String]) -> ExitCode {
    if let Some(other) = args.first() {
        eprintln!("tham số serve không hỗ trợ: {other}; dùng: boom-server serve");
        return ExitCode::from(3);
    }
    let config = match OperationalConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("lỗi cấu hình: {err}");
            return ExitCode::from(3);
        }
    };
    let instance_id = new_instance_id();
    let state = Arc::new(HealthState::new(
        config.clone(),
        instance_id.clone(),
        now_ms(),
    ));
    let logger = Arc::new(Logger::stdout(
        config.fingerprint(),
        instance_id,
        config.content_epoch,
    ));
    let admin = match AdminServer::start(&config, state.clone(), logger.clone()) {
        Ok(admin) => admin,
        Err(err) => {
            eprintln!("không mở được admin listener: {err}");
            return ExitCode::from(2);
        }
    };
    let heartbeat = match Heartbeat::start(state, logger, config.heartbeat_interval_ms) {
        Ok(heartbeat) => heartbeat,
        Err(err) => {
            eprintln!("không ghi được health.json: {err}");
            return ExitCode::from(2);
        }
    };
    eprintln!("admin listener {}", admin.local_addr());
    let _admin = admin;
    let _heartbeat = heartbeat;
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use boom_server::auth::{AuthConfig, AuthService};
    use boom_server::identity::OsRandomSource;
    use tempfile::TempDir;

    const NOW: i64 = 1_700_000_000_000;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn config_for(dir: &TempDir) -> OperationalConfig {
        OperationalConfig {
            db_path: dir.path().join("boom-server.sqlite3"),
            ..OperationalConfig::default()
        }
    }

    fn service() -> AuthService {
        AuthService::new(
            AuthConfig::default(),
            "instance-1",
            Box::new(OsRandomSource),
        )
        .expect("tạo dịch vụ auth")
    }

    #[test]
    fn health_args_reject_unknown_before_io() {
        assert_eq!(parse_health_args(&args(&["health"])), Ok(false));
        assert_eq!(parse_health_args(&args(&["health", "--offline"])), Ok(true));
        assert!(parse_health_args(&args(&["health", "--bogus"])).is_err());
        assert!(parse_health_args(&args(&["health", "--offline", "--bogus"])).is_err());
    }

    #[test]
    fn revoke_sessions_revokes_active_session_and_login_works_again() {
        let dir = TempDir::new().unwrap();
        let config = config_for(&dir);
        // Tạo tài khoản + phiên đầu tiên trên DB thật (tempfile).
        {
            let mut store =
                SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).unwrap();
            let mut auth = service();
            auth.register(
                &mut store,
                "bomber",
                "matkhau123",
                "10.0.0.1",
                "conn-1",
                NOW,
            )
            .expect("đăng ký thành công");
        }

        let outcome = revoke_sessions(&config, "bomber").expect("thu hồi phiên");
        assert_eq!(outcome.revoked, 1);
        assert_eq!(outcome.username, "bomber");

        // Phiên đã bị thu hồi ⇒ đăng nhập lại được (ADR 0011 §6/§6.1).
        let mut store =
            SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).unwrap();
        let mut auth = service();
        auth.login(
            &mut store,
            "bomber",
            "matkhau123",
            "10.0.0.1",
            "conn-2",
            NOW + 1,
        )
        .expect("đăng nhập lại được");
    }

    #[test]
    fn revoke_sessions_reports_missing_account() {
        let dir = TempDir::new().unwrap();
        let config = config_for(&dir);
        // Mở store một lần để tạo file DB rồi đóng lại (nhả khoá).
        drop(SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).unwrap());
        let error = revoke_sessions(&config, "khong-ton-tai").unwrap_err();
        assert_eq!(error, AccountCliError::AccountNotFound);
    }

    #[test]
    fn revoke_sessions_fails_when_db_is_locked_by_running_server() {
        let dir = TempDir::new().unwrap();
        let config = config_for(&dir);
        // Giữ khoá instance như một server đang chạy.
        let _store = SqliteAccountStore::open(&config.db_path, SqliteOptions::default()).unwrap();
        let error = revoke_sessions(&config, "bomber").unwrap_err();
        assert_eq!(error, AccountCliError::DatabaseLocked);
    }

    #[test]
    fn account_exit_codes_follow_documented_meaning() {
        assert_eq!(account_exit_code(&AccountCliError::AccountNotFound), 1);
        assert_eq!(account_exit_code(&AccountCliError::DatabaseLocked), 2);
        assert_eq!(
            account_exit_code(&AccountCliError::Store(StoreError::Sqlite(
                "lỗi".to_owned()
            ))),
            2
        );
    }
}
