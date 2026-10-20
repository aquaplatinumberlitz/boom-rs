//! Chạy replay xác định và so với golden.
//!
//! ```text
//! boom-replay run <replay.toml>           in kết quả (định dạng golden) ra stdout
//! boom-replay verify <replay.toml>        so với <replay>.golden; khác thì exit 1
//! boom-replay bless <replay.toml>         ghi lại <replay>.golden
//! boom-replay content-hash <replay.toml>  in SHA-256 của content map đã dựng (để điền `content_sha256`)
//! boom-replay profile-hash <replay.toml>  in định danh profile (để điền `profile_hash`)
//! ```
//!
//! File replay = khóa dựng trận của `boom_scenario::setup` (bắt buộc có `rules`, `profile_hash` và
//! `content_sha256`) cộng `schema`, `hash_algo`, `ticks`, `checkpoints`. Map có thể là ASCII
//! (`map = [...]`) hoặc `[map_pack]` trỏ `FxMap`/`Map/Village00` trong pack KR; replay từ chối khi
//! profile hoặc content tính lại khác hash trong file. Golden là regression baseline, không phải
//! oracle bản gốc; scenario mới là nơi kiểm luật.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use boom_assets::closure::sha256_hex;
use boom_assets::legacy_map::{Build, parse as parse_legacy_map};
use boom_assets::pack::Pack;
use boom_content::hash::HASH_ALGO;
use boom_content::map::{CellKind, MapProfile, Terrain};
use boom_core::{Outcome, Phase, event_digest};
use boom_scenario::Error;
use boom_scenario::setup::{Match, Setup, load_profile, parse_checked};
use serde::Deserialize;

const SCHEMA: u32 = 1;

#[derive(Debug, Deserialize)]
struct Replay {
    schema: u32,
    hash_algo: String,
    content_sha256: String,
    ticks: u32,
    checkpoints: Vec<u32>,
    #[serde(flatten)]
    setup: Setup,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn load(path: &Path) -> Result<Replay, Error> {
    let text = std::fs::read_to_string(path)?;
    let replay: Replay = parse_checked(
        &text,
        &[
            "schema",
            "hash_algo",
            "content_sha256",
            "ticks",
            "checkpoints",
        ],
    )
    .map_err(|e| format!("{}: {e}", path.display()))?;
    if replay.schema != SCHEMA {
        return Err(format!(
            "replay schema {} không hỗ trợ (cần {SCHEMA})",
            replay.schema
        )
        .into());
    }
    if replay.hash_algo != HASH_ALGO {
        return Err(format!(
            "hash_algo {} không hỗ trợ (cần {HASH_ALGO})",
            replay.hash_algo
        )
        .into());
    }
    if !is_sha256(&replay.content_sha256) {
        return Err(format!(
            "content_sha256 {:?} không phải SHA-256 hex 64 ký tự",
            replay.content_sha256
        )
        .into());
    }
    if replay.setup.rules.is_none() || replay.setup.profile_hash.is_none() {
        return Err("replay phải ghi rõ `rules` và `profile_hash`".into());
    }
    if replay.setup.map.is_empty() && replay.setup.map_pack.is_none() {
        return Err("replay phải có `map` ASCII hoặc `[map_pack]`".into());
    }
    if !replay.setup.map.is_empty() && replay.setup.map_pack.is_some() {
        return Err("replay không được ghi đồng thời `map` và `[map_pack]`".into());
    }
    if let Some(pack) = &replay.setup.map_pack
        && pack.env.trim().is_empty()
    {
        return Err("`[map_pack].env` không được rỗng".into());
    }
    Ok(replay)
}

fn base(path: &Path) -> &Path {
    path.parent().unwrap_or(Path::new("."))
}

enum Content {
    Ready(Terrain),
    Skipped(String),
}

fn build_from_name(name: &str) -> Result<Build, Error> {
    match name {
        "kr" => Ok(Build::Kr),
        "vn" => Ok(Build::Vn),
        other => Err(format!("map_pack.build {other:?} không hợp lệ (kr/vn)").into()),
    }
}

/// Nạp map ASCII hoặc map pack. Pack thiếu biến môi trường là skip, không phải lỗi: CI không có
/// dữ liệu game. Biến có nhưng pack/file hỏng vẫn là lỗi để không che cấu hình sai.
fn load_content(setup: &Setup, base: &Path) -> Result<Content, Error> {
    let Some(pack) = &setup.map_pack else {
        return Ok(Content::Ready(Terrain::from_ascii(&setup.map)?));
    };
    let Some(dir) = std::env::var_os(&pack.env).filter(|value| !value.is_empty()) else {
        return Ok(Content::Skipped(format!(
            "bỏ qua: chưa đặt {} cho map pack {}",
            pack.env, pack.entry
        )));
    };
    let dir = PathBuf::from(dir);
    let build = build_from_name(&pack.build)?;
    let pack_file =
        Pack::open(&dir, "FxMap", build).map_err(|e| format!("{} (FxMap): {e}", dir.display()))?;
    let raw = pack_file
        .read_named(&pack.entry)
        .map_err(|e| format!("{}:{}: {e}", dir.display(), pack.entry))?;
    let legacy = parse_legacy_map(&raw, build)
        .map_err(|e| format!("{}:{}: {e}", dir.display(), pack.entry))?;
    let profile_path = base.join(&pack.profile);
    let text = std::fs::read_to_string(&profile_path)
        .map_err(|e| format!("{}: {e}", profile_path.display()))?;
    let profile =
        MapProfile::from_toml(&text).map_err(|e| format!("{}: {e}", profile_path.display()))?;
    let terrain = profile.classify(&legacy.grid)?;
    Ok(Content::Ready(terrain))
}

/// SHA-256 của địa hình đã phân loại theo encoding chuẩn, không phụ thuộc file nguồn/kiểu map.
fn content_sha256(terrain: &Terrain) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"boom-terrain-v1");
    bytes.extend_from_slice(&terrain.width.to_le_bytes());
    bytes.extend_from_slice(&terrain.height.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(terrain.kinds.len())
            .expect("lưới vừa u32")
            .to_le_bytes(),
    );
    for kind in &terrain.kinds {
        bytes.push(match kind {
            CellKind::Floor => 0,
            CellKind::Solid => 1,
            CellKind::Breakable => 2,
            CellKind::Unknown => 3,
        });
    }
    bytes.extend_from_slice(
        &u32::try_from(terrain.spawns.len())
            .expect("spawn vừa u32")
            .to_le_bytes(),
    );
    for (x, y) in &terrain.spawns {
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
    }
    sha256_hex(&bytes)
}

enum RunOutput {
    Golden(String),
    Skipped(String),
}

/// Chạy replay, trả về nội dung golden hoặc lý do bỏ qua khi thiếu pack.
fn run(path: &Path) -> Result<RunOutput, Error> {
    let replay = load(path)?;
    let terrain = match load_content(&replay.setup, base(path))? {
        Content::Ready(terrain) => terrain,
        Content::Skipped(reason) => return Ok(RunOutput::Skipped(reason)),
    };
    let actual_content = content_sha256(&terrain);
    if actual_content != replay.content_sha256 {
        return Err(format!(
            "content có SHA-256 {actual_content}, file ghi {}; từ chối chạy với map khác",
            replay.content_sha256
        )
        .into());
    }
    let mut m = Match::from_terrain(&replay.setup, base(path), terrain)?;
    let profile = load_profile(&replay.setup, base(path))?;
    let mut out = format!(
        "# boom-replay golden v{SCHEMA} {HASH_ALGO} profile {} mode {} content {}\n",
        profile.profile_id,
        m.world.mode().id(),
        actual_content
    );
    let mut checkpoints = std::collections::BTreeSet::new();
    while m.state.tick < replay.ticks && m.state.phase == Phase::Playing {
        m.step()?;
        if replay.checkpoints.contains(&m.state.tick) {
            out += &format!("tick {} state {:#018x}\n", m.state.tick, m.state.hash());
            checkpoints.insert(m.state.tick);
        }
    }
    // Authority có thể kết thúc round mà không có `step` cuối (tất cả disconnect): áp phần còn lại
    // tại đúng tick hiện tại, cùng mutation với server.
    m.apply_authority_for_tick()?;
    if replay.checkpoints.contains(&m.state.tick) && !checkpoints.contains(&m.state.tick) {
        out += &format!("tick {} state {:#018x}\n", m.state.tick, m.state.hash());
    }
    if let Some(i) = m.unused_input() {
        return Err(format!("input ở tick {} sau khi replay đã dừng", i.tick).into());
    }
    let outcome = match m.state.phase {
        Phase::Playing => "playing".to_owned(),
        Phase::Ended(Outcome::Draw) => "draw".to_owned(),
        Phase::Ended(Outcome::Winner(id)) => format!("winner:{id}"),
    };
    out += &format!(
        "final tick {} state {:#018x}\nevents {} digest {:#018x}\noutcome {outcome}\n",
        m.state.tick,
        m.state.hash(),
        m.events.len(),
        event_digest(&m.events)
    );
    Ok(RunOutput::Golden(out))
}

fn golden_path(path: &Path) -> PathBuf {
    path.with_extension("golden")
}

fn verify(path: &Path) -> Result<(), Error> {
    let actual = match run(path)? {
        RunOutput::Golden(actual) => actual,
        RunOutput::Skipped(reason) => {
            eprintln!("{reason}");
            return Ok(());
        }
    };
    let golden_file = golden_path(path);
    let golden = std::fs::read_to_string(&golden_file)
        .map_err(|e| format!("{}: {e}", golden_file.display()))?
        .replace("\r\n", "\n");
    if actual != golden {
        return Err(format!(
            "lệch golden {}\n--- golden\n{golden}--- thực tế\n{actual}",
            golden_file.display()
        )
        .into());
    }
    Ok(())
}

fn profile_hash(path: &Path) -> Result<(), Error> {
    let mut replay = load(path)?;
    replay.setup.profile_hash = None;
    let profile = load_profile(&replay.setup, base(path))?;
    println!("{:#018x}", profile.identity_hash());
    Ok(())
}

fn content_hash(path: &Path) -> Result<(), Error> {
    let replay = load(path)?;
    match load_content(&replay.setup, base(path))? {
        Content::Ready(terrain) => {
            println!("{}", content_sha256(&terrain));
            Ok(())
        }
        Content::Skipped(reason) => Err(reason.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(file)) = (args.first(), args.get(1)) else {
        eprintln!("dùng: boom-replay run|verify|bless|content-hash|profile-hash <replay.toml>");
        return ExitCode::from(2);
    };
    let path = Path::new(file);
    let result = match cmd.as_str() {
        "run" => run(path).map(|out| match out {
            RunOutput::Golden(out) => print!("{out}"),
            RunOutput::Skipped(reason) => eprintln!("{reason}"),
        }),
        "verify" => verify(path),
        "bless" => run(path).and_then(|out| match out {
            RunOutput::Golden(out) => Ok(std::fs::write(golden_path(path), out)?),
            RunOutput::Skipped(reason) => {
                eprintln!("{reason}");
                Ok(())
            }
        }),
        "content-hash" => content_hash(path),
        "profile-hash" => profile_hash(path),
        _ => Err(format!("lệnh lạ {cmd}").into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{file}: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        let rules = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/rules/skeleton-test.toml")
            .display()
            .to_string();
        format!(
            r######"
schema = 1
hash_algo = "fnv1a64-v0"
content_sha256 = "{}"
ticks = 1
checkpoints = [1]
rules = '{rules}'
profile_hash = "0x3f43f71d06488a95"
seed = 1
map = ["#####", "#0..#", "#####"]

[[player]]
id = 0
speed = [5, 7]
bombs = [1, 10]
range = [1, 7]
"######,
            "0".repeat(64)
        )
    }

    fn temp_replay(name: &str, text: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("boom-replay-{}-{name}.toml", std::process::id()));
        std::fs::write(&path, text).expect("ghi replay tạm");
        path
    }

    fn run_text(name: &str, text: &str) -> Result<RunOutput, Error> {
        let path = temp_replay(name, text);
        let result = run(&path);
        let _ = std::fs::remove_file(&path);
        result
    }

    #[test]
    fn content_hash_is_canonical_and_changes_with_map() {
        let a = Terrain::from_ascii(&["###", "#0#", "###"]).unwrap();
        let b = Terrain::from_ascii(&["###", "#0+", "###"]).unwrap();
        let c = Terrain::from_ascii(&["###", "#0#", "###"]).unwrap();
        assert_eq!(content_sha256(&a), content_sha256(&c));
        assert_ne!(content_sha256(&a), content_sha256(&b));
    }

    #[test]
    fn rejects_unknown_schema_and_hash_algo() {
        let path = temp_replay("schema", &fixture().replace("schema = 1", "schema = 2"));
        assert!(load(&path).unwrap_err().to_string().contains("schema"));
        let _ = std::fs::remove_file(&path);

        let path = temp_replay("algo", &fixture().replace("fnv1a64-v0", "other-hash"));
        assert!(load(&path).unwrap_err().to_string().contains("hash_algo"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_content_and_rules_mismatch() {
        // content sai: file ghi toàn số 0, map ASCII thật khác.
        let err = match run_text("content", &fixture()) {
            Ok(_) => panic!("content sai phải bị từ chối"),
            Err(e) => e.to_string(),
        };
        assert!(err.contains("content"), "{err}");

        // rules sai: đổi profile_hash trong khi vẫn dùng đúng content.
        let map = Terrain::from_ascii(&["#####", "#0..#", "#####"]).unwrap();
        let hash = content_sha256(&map);
        let zeros = "0".repeat(64);
        let text = fixture()
            .replace(&zeros, &hash)
            .replace("0x3f43f71d06488a95", "0x0000000000000000");
        let err = match run_text("rules", &text) {
            Ok(_) => panic!("rules sai phải bị từ chối"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("định danh") || err.contains("profile"),
            "{err}"
        );
    }
}
