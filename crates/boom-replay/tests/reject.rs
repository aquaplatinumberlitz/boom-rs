//! Từ chối replay khi header/content/rules lệch; kiểm cả exit code CLI, không chỉ hàm nội bộ.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/replays/full-ffa.toml");
    let text = std::fs::read_to_string(path).expect("đọc fixture full-ffa");
    let rules = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rules/skeleton-test.toml")
        .display()
        .to_string();
    text.replace(
        "rules = \"../rules/skeleton-test.toml\"",
        &format!("rules = '{rules}'"),
    )
}

fn temp(name: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "boom-replay-reject-{}-{name}.toml",
        std::process::id()
    ));
    std::fs::write(&path, text).expect("ghi replay tạm");
    path
}

fn reject(name: &str, text: &str, expected: &str) {
    let path = temp(name, text);
    let output = Command::new(env!("CARGO_BIN_EXE_boom-replay"))
        .args(["verify", path.to_str().expect("đường dẫn UTF-8")])
        .output()
        .expect("chạy boom-replay");
    let _ = std::fs::remove_file(&path);
    assert!(!output.status.success(), "{name}: phải exit != 0");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(expected), "{name}: stderr {stderr}");
}

#[test]
fn rejects_unknown_schema() {
    reject(
        "schema",
        &fixture().replace("schema = 1", "schema = 2"),
        "schema",
    );
}

#[test]
fn rejects_unknown_hash_algo() {
    reject(
        "hash",
        &fixture().replace("fnv1a64-v0", "sha256-other"),
        "hash_algo",
    );
}

#[test]
fn rejects_content_mismatch() {
    let zeros = "0".repeat(64);
    let text = fixture().replace(
        &format!("content_sha256 = \"{}\"", content_hash()),
        &format!("content_sha256 = \"{zeros}\""),
    );
    reject("content", &text, "content");
}

#[test]
fn rejects_rules_mismatch() {
    let text = break_profile_hash(&fixture());
    reject("rules", &text, "định danh");
}

/// Thay giá trị `profile_hash` trong header replay bằng giá trị sai, không hard-code hash
/// để không stale mỗi lần profile đổi định danh.
fn break_profile_hash(text: &str) -> String {
    const KEY: &str = "profile_hash = \"";
    let start = text.find(KEY).expect("replay có profile_hash") + KEY.len();
    let end = start + text[start..].find('"').expect("profile_hash có dấu nháy");
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..start]);
    out.push_str("0x0000000000000000");
    out.push_str(&text[end..]);
    out
}

fn content_hash() -> &'static str {
    "278af9a907d0be57360206094c46a9ff80bcf2889b9bdc9497382cbdcb874f48"
}
