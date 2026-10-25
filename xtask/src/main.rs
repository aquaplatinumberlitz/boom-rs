//! Tác vụ repo, chạy bằng `cargo xtask <lệnh>`:
//!
//! - `ci`: cổng bàn giao (AGENTS §7) — fmt, clippy `-D warnings`, test, replay golden, docs/catalog,
//!   kiểm ưu tiên nguồn luật theo ADR 0013, đối chiếu oracle, test Python của tool theo dõi tiến độ.
//! - `bless`: ghi lại mọi golden replay sau khi đổi hành vi core có chủ đích.
//! - `oracle`: sinh `docs/research/ORACLE_REPORT.md` từ `fixtures/oracle/*.toml` (V1-037);
//!   `--check` chỉ kiểm (dùng trong `ci`).
//! - `setup`: bật hook pre-push của repo (`.githooks/`), chạy `ci` trước mỗi lần push.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask nằm trong workspace")
        .to_path_buf()
}

fn run(desc: &str, cmd: &mut Command) -> bool {
    eprintln!("==> {desc}");
    match cmd.current_dir(root()).status() {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!("thất bại: {desc} ({s})");
            false
        }
        Err(e) => {
            eprintln!("không chạy được {desc}: {e}");
            false
        }
    }
}

/// Lệnh git chạy trên repo của workspace này, không theo GIT_DIR/GIT_WORK_TREE thừa hưởng.
fn git() -> Command {
    let mut cmd = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

fn cargo() -> Command {
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
}

fn replays() -> Vec<PathBuf> {
    let dir = root().join("fixtures/replays");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|it| {
            it.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn replay(cmd: &str) -> bool {
    replays().iter().all(|file| {
        run(
            &format!("replay {cmd} {}", file.display()),
            cargo()
                .args([
                    "run",
                    "--quiet",
                    "--release",
                    "-p",
                    "boom-replay",
                    "--",
                    cmd,
                ])
                .arg(file),
        )
    })
}

fn python() -> Command {
    let name = if cfg!(windows) { "python" } else { "python3" };
    let mut cmd = Command::new(name);
    // Tool docs đọc/ghi file không ghi rõ encoding; Windows mặc định cp1252 sẽ vỡ tiếng Việt.
    cmd.env("PYTHONUTF8", "1");
    cmd
}

/// Đối chiếu oracle (V1-037): đọc `fixtures/oracle/*.toml`, đo trên engine, sinh báo cáo.
///
/// `check = true`: không ghi file, chỉ so báo cáo trong repo với kết quả đo và báo sai khác
/// chưa ghi nhận ở SCOPE_V1 §10 (sai khác đó là lỗi của `ci`).
fn oracle(check: bool) -> bool {
    let root = root();
    let dir = root.join(boom_scenario::oracle::ORACLE_DIR);
    let oracles = match boom_scenario::oracle::load_dir(&dir, &root) {
        Ok(list) => list,
        Err(e) => {
            eprintln!("oracle: không đọc được metadata oracle: {e}");
            return false;
        }
    };
    let outcomes = match boom_scenario::oracle::run(&oracles, &root) {
        Ok(list) => list,
        Err(e) => {
            eprintln!("oracle: không đo được: {e}");
            return false;
        }
    };
    let gaps = match boom_scenario::oracle::scope_gaps(&root) {
        Ok(gaps) => gaps,
        Err(e) => {
            eprintln!("oracle: không đọc được sổ sai khác: {e}");
            return false;
        }
    };
    let report = boom_scenario::oracle::render(&outcomes, &gaps);
    let path = root.join(boom_scenario::oracle::REPORT_PATH);

    let mut ok = true;
    for error in boom_scenario::oracle::blocking_errors(&outcomes) {
        eprintln!("oracle: {error}");
        ok = false;
    }
    if check {
        // So nội dung đã chuẩn hoá dòng kết thúc: checkout trên Windows có thể để CRLF.
        let current = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if current != report {
            eprintln!(
                "oracle: {} cũ — chạy `cargo xtask oracle` để sinh lại",
                boom_scenario::oracle::REPORT_PATH
            );
            ok = false;
        }
    } else if let Err(e) = std::fs::write(&path, &report) {
        eprintln!("oracle: không ghi được {}: {e}", path.display());
        ok = false;
    }
    for o in &outcomes {
        println!(
            "{} {} — kỳ vọng {} · engine {} · {}",
            o.oracle.id,
            o.oracle.quantity,
            match o.oracle.expected {
                boom_scenario::oracle::Expected::Value(v) =>
                    format!("{v} ± {}", o.oracle.tolerance),
                boom_scenario::oracle::Expected::Unknown(_) => "chưa có số".to_owned(),
            },
            o.measured,
            o.verdict.label()
        );
    }
    println!(
        "oracle: {} oracle, {} lỗi chặn CI",
        outcomes.len(),
        boom_scenario::oracle::blocking_errors(&outcomes).len()
    );
    ok
}

fn ci() -> bool {
    run("fmt", cargo().args(["fmt", "--all", "--check"]))
        && run(
            "clippy",
            cargo().args([
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ]),
        )
        && run("test", cargo().args(["test", "--workspace"]))
        && replay("verify")
        && run("docs_check", python().arg("tools/docs_check.py"))
        && run(
            "kr_catalog --check",
            python().args(["tools/kr_catalog.py", "--check"]),
        )
        && run(
            "rules_precedence",
            python().arg("tools/rules_precedence.py"),
        )
        && run("test_gui_probe", python().args(["tools/test_gui_probe.py"]))
        && run(
            "test_rules_precedence",
            python().args(["tools/test_rules_precedence.py"]),
        )
        && run(
            "test_kr_tracking",
            python().args(["tools/test_kr_tracking.py"]),
        )
        && oracle(true)
}

fn main() -> ExitCode {
    let ok = match std::env::args().nth(1).as_deref() {
        Some("ci") => ci(),
        Some("bless") => replay("bless"),
        Some("oracle") => oracle(std::env::args().nth(2).as_deref() == Some("--check")),
        Some("setup") => run(
            "git config core.hooksPath .githooks",
            git().args(["config", "core.hooksPath", ".githooks"]),
        ),
        _ => {
            eprintln!("dùng: cargo xtask ci|bless|oracle[ --check]|setup");
            return ExitCode::from(2);
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
