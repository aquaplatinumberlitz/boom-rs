//! Tác vụ repo, chạy bằng `cargo xtask <lệnh>`:
//!
//! - `ci`: cổng bàn giao (AGENTS §7) — fmt, clippy `-D warnings`, test, replay golden, docs/catalog.
//! - `bless`: ghi lại mọi golden replay sau khi đổi hành vi core có chủ đích.
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
}

fn main() -> ExitCode {
    let ok = match std::env::args().nth(1).as_deref() {
        Some("ci") => ci(),
        Some("bless") => replay("bless"),
        Some("setup") => run(
            "git config core.hooksPath .githooks",
            Command::new("git").args(["config", "core.hooksPath", ".githooks"]),
        ),
        _ => {
            eprintln!("dùng: cargo xtask ci|bless|setup");
            return ExitCode::from(2);
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
