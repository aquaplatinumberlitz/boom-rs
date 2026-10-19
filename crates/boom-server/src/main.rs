//! CLI server V1: `health` runtime/offline (ADR 0011 §8.2), `account revoke-sessions`
//! (ADR 0011 §6), `config` (đọc/ghi cấu hình vận hành), `content` (kiểm bộ nội dung
//! rules/bundle/l10n của gói phát hành — V1-036) và `serve` (chạy trận thật).
//!
//! `serve` nạp rules profile + bundle map, mở DB identity, bind game listener, nhận client
//! TCP và chạy vòng tick; Ctrl-C/SIGTERM dừng sạch (thu hồi phiên của instance, ghi
//! heartbeat cuối, thoát 0). Chi tiết ở [`boom_server::runtime`].

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use boom_content::bundle;
use boom_content::l10n::L10n;
use boom_content::rules::RulesProfile;
use boom_server::identity::{
    AccountId, AccountStore, RevokeReason, SessionService, SqliteAccountStore, SqliteOptions,
    StoreError,
};
use boom_server::runtime::StopFlag;
use boom_server::{
    LogFields, Logger, OperationalConfig, events, new_instance_id, now_ms, run_health_cli,
};
use serde_json::json;

/// Cách dùng CLI, in ra khi `--help` hoặc khi tham số sai.
const USAGE: &str = "boom-server health [--offline] | account revoke-sessions <username> | \
                     config [--out <file>] | content --rules <file> [--bundle <dir>] \
                     [--l10n <file>] | serve";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("health") => health_command(&args),
        Some("account") => account_command(&args[1..]),
        Some("config") => config_command(&args[1..]),
        Some("content") => content_command(&args[1..]),
        Some("-h") | Some("--help") => {
            print_usage();
            ExitCode::SUCCESS
        }
        None => serve_command(&[]),
        Some("serve") => serve_command(&args[1..]),
        Some(other) => {
            eprintln!("lệnh không hỗ trợ: {other}; dùng: {USAGE}");
            ExitCode::from(3)
        }
    }
}

/// In cách dùng đầy đủ (lệnh `--help`).
fn print_usage() {
    println!("{USAGE}");
    println!();
    println!("lệnh:");
    println!("  health [--offline]              kiểm runtime: tiến trình, DB, log");
    println!("  account revoke-sessions <user>   thu hồi phiên đang hiệu lực của tài khoản");
    println!("  config [--out <file>]           xuất/ghi cấu hình vận hành hiệu lực");
    println!("  content --rules <file>          kiểm bộ nội dung của gói phát hành");
    println!("                                  (--bundle <dir> và --l10n <file> là tuỳ chọn)");
    println!("  serve                           chạy server (admin listener + heartbeat)");
    println!();
    println!(
        "mã thoát: 0 thành công · 2 không dùng được (file nội dung thiếu/sai) · \
              3 sai cú pháp hoặc lỗi cấu hình"
    );
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

/// Tham số của `boom-server serve`.
fn parse_serve_args(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut config_path: Option<PathBuf> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--config" => match rest.next() {
                Some(path) if !path.trim().is_empty() => {
                    config_path = Some(PathBuf::from(path.trim()))
                }
                _ => {
                    return Err(
                        "--config cần đường dẫn file; dùng: boom-server serve [--config <file>]"
                            .to_owned(),
                    );
                }
            },
            other => {
                return Err(format!(
                    "tham số serve không hỗ trợ: {other}; dùng: boom-server serve [--config <file>]"
                ));
            }
        }
    }
    Ok(config_path)
}

/// Mã thoát của lệnh `content`; `content_command` bọc thành [`ExitCode`].
fn content_command(args: &[String]) -> ExitCode {
    ExitCode::from(content_exit(args))
}

/// `boom-server content --rules <file> [--bundle <dir>] [--l10n <file>]`.
///
/// Kiểm bộ nội dung của gói phát hành (V1-036): rules profile, bundle content và file
/// chuỗi UI. Lỗi luôn kèm **đường dẫn** và **cách sửa** (V1-036: thiếu file hoặc sai
/// hash → báo rõ đường dẫn + cách sửa, exit code khác 0, không panic).
///
/// Mã thoát: `0` nội dung dùng được · `2` nội dung thiếu/sai · `3` sai cú pháp.
fn content_exit(args: &[String]) -> u8 {
    let mut rules: Option<PathBuf> = None;
    let mut bundle_dir: Option<PathBuf> = None;
    let mut l10n: Option<PathBuf> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let target = match arg.as_str() {
            "--rules" => &mut rules,
            "--bundle" => &mut bundle_dir,
            "--l10n" => &mut l10n,
            other => {
                eprintln!("tham số content không hỗ trợ: {other}; dùng: {USAGE}");
                return 3;
            }
        };
        let Some(value) = iter.next() else {
            eprintln!("{arg} cần đường dẫn; dùng: {USAGE}");
            return 3;
        };
        *target = Some(PathBuf::from(value));
    }
    let Some(rules_path) = rules else {
        eprintln!("content cần --rules <file>; dùng: {USAGE}");
        return 3;
    };

    let profile = match load_rules_file(&rules_path) {
        Ok(profile) => profile,
        Err(message) => {
            eprintln!("{message}");
            return 2;
        }
    };
    println!(
        "rules: {} · hash {:#018x} · {} rule",
        profile.profile_id,
        profile.identity_hash(),
        profile.rules.len()
    );

    if let Some(dir) = bundle_dir.as_deref() {
        let loaded = match bundle::load(dir) {
            Ok(loaded) => loaded,
            Err(error) => {
                eprintln!("{}", bundle_error(dir, &error));
                return 2;
            }
        };
        println!(
            "bundle: {} (build {}) · content_hash {}",
            loaded.map, loaded.build, loaded.content_hash
        );
    }

    if let Some(path) = l10n.as_deref() {
        let loaded = match load_l10n_file(path) {
            Ok(loaded) => loaded,
            Err(message) => {
                eprintln!("{message}");
                return 2;
            }
        };
        println!("chuỗi: {} · {} khóa", loaded.locale(), loaded.len());
    }

    0
}

/// Nạp rules profile từ file; thông điệp lỗi kèm đường dẫn và cách sửa.
fn load_rules_file(path: &Path) -> Result<RulesProfile, String> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "không đọc được file rules: {} ({error})\n  \
             sửa: file phải tồn tại và đọc được; gói phát hành đặt rules ở data/rules/ \
             (ví dụ data/rules/v1-dev.toml)",
            path.display()
        )
    })?;
    RulesProfile::from_toml(&text).map_err(|error| {
        format!(
            "file rules không dùng được: {} ({error})\n  \
             sửa: file phải là rules profile TOML hợp lệ \
             (xem docs/specification/gameplay/RULES_V1.md)",
            path.display()
        )
    })
}

/// Nạp bộ chuỗi UI từ file; thông điệp lỗi kèm đường dẫn và cách sửa.
fn load_l10n_file(path: &Path) -> Result<L10n, String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "không đọc được file chuỗi: {} ({error})\n  \
             sửa: file phải tồn tại; gói phát hành đặt chuỗi ở data/l10n/ \
             (ví dụ data/l10n/vi-v1.toml)",
            path.display()
        )
    })?;
    L10n::from_bytes(&bytes).map_err(|error| {
        format!(
            "file chuỗi không dùng được: {} ({error})\n  \
             sửa: file phải là file chuỗi UI V1 hợp lệ (sinh bằng tools/l10n_export.py, \
             xem docs/specification/localization/UI_STRINGS_V1.md)",
            path.display()
        )
    })
}

/// Lỗi nạp bundle: kèm thư mục bundle và cách dựng lại từ pack.
fn bundle_error(dir: &Path, error: &dyn std::fmt::Display) -> String {
    format!(
        "không nạp được bundle: {}\n  lỗi: {error}\n  \
         sửa: dựng lại bundle từ pack của bạn: \
         `boom-assets bundle --map Village00 --out <dir> --pack-dir <thư mục pack>`",
        dir.display()
    )
}

/// `boom-server serve [--config <file>]` — chạy server thật cho client LAN.
///
/// Thứ tự cấu hình giữ nguyên ADR 0011 §5: mặc định → file (`--config` hoặc
/// `BOOM_CONFIG_FILE`) → biến môi trường.
///
/// Mã thoát: `0` dừng sạch (Ctrl-C/SIGTERM) · `2` lỗi khởi động hoặc lỗi lúc chạy ·
/// `3` sai cú pháp/lỗi cấu hình.
fn serve_command(args: &[String]) -> ExitCode {
    let config_path = match parse_serve_args(args) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(3);
        }
    };
    let config = match config_path {
        Some(path) => OperationalConfig::from_file_with_env(Path::new(&path)),
        None => OperationalConfig::from_env(),
    };
    let config = match config {
        Ok(config) => config,
        Err(err) => {
            eprintln!("lỗi cấu hình: {err}");
            return ExitCode::from(3);
        }
    };
    // Ctrl-C/SIGTERM ⇒ dừng sạch (ADR 0011 §6.1): handler chỉ đặt cờ, vòng tick lo phần còn lại.
    let stop = StopFlag::default();
    let handler_stop = stop.clone();
    if let Err(err) = ctrlc::set_handler(move || handler_stop.request()) {
        eprintln!("không cài được handler Ctrl-C/SIGTERM: {err}");
        return ExitCode::from(2);
    }
    match boom_server::runtime::serve(config, stop) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("server dừng vì lỗi: {err}");
            ExitCode::from(2)
        }
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

    /// V1-036: thiếu file rules → thông điệp có đường dẫn và cách sửa.
    #[test]
    fn missing_rules_file_reports_path_and_fix() {
        let message = load_rules_file(Path::new("/khong/ton/tai/rules.toml")).unwrap_err();
        assert!(
            message.contains("/khong/ton/tai/rules.toml"),
            "thông điệp phải nêu đường dẫn: {message}"
        );
        assert!(
            message.contains("sửa:"),
            "thông điệp phải nêu cách sửa: {message}"
        );
    }

    /// V1-036: thiếu file chuỗi → thông điệp có đường dẫn và cách sửa.
    #[test]
    fn missing_l10n_file_reports_path_and_fix() {
        let message = load_l10n_file(Path::new("/khong/ton/tai/vi-v1.toml")).unwrap_err();
        assert!(
            message.contains("/khong/ton/tai/vi-v1.toml"),
            "thông điệp phải nêu đường dẫn: {message}"
        );
        assert!(
            message.contains("sửa:"),
            "thông điệp phải nêu cách sửa: {message}"
        );
    }

    /// V1-036: bundle thiếu → thông điệp có thư mục bundle và cách dựng lại.
    #[test]
    fn missing_bundle_reports_path_and_fix() {
        let message = bundle_error(Path::new("/khong/ton/tai/bundle"), &"metadata thiếu");
        assert!(
            message.contains("/khong/ton/tai/bundle"),
            "thông điệp phải nêu thư mục bundle: {message}"
        );
        assert!(
            message.contains("boom-assets bundle"),
            "thông điệp phải nêu cách dựng lại bundle: {message}"
        );
    }

    /// `content` với tham số lạ → mã thoát 3 (sai cú pháp), không phải 2.
    #[test]
    fn content_command_rejects_unknown_flag() {
        assert_eq!(content_exit(&args(&["--bogus", "x"])), 3);
    }

    /// `content` thiếu `--rules` → mã thoát 3 (sai cú pháp).
    #[test]
    fn content_command_requires_rules_flag() {
        assert_eq!(content_exit(&args(&["--bundle", "/tmp"])), 3);
    }

    /// `content` với file rules thiếu → mã thoát 2 (nội dung không dùng được).
    #[test]
    fn content_command_reports_missing_rules_file() {
        assert_eq!(
            content_exit(&args(&["--rules", "/khong/ton/tai/rules.toml"])),
            2
        );
    }
}
