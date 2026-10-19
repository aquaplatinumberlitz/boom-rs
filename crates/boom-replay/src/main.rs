//! Chạy replay xác định và so với golden.
//!
//! ```text
//! boom-replay run <replay.toml>           in kết quả (định dạng golden) ra stdout
//! boom-replay verify <replay.toml>        so với <replay>.golden; khác thì exit 1
//! boom-replay bless <replay.toml>         ghi lại <replay>.golden
//! boom-replay profile-hash <replay.toml>  in định danh profile (để điền `profile_hash`)
//! ```
//!
//! File replay = khóa dựng trận của `boom_scenario::setup` (bắt buộc có `rules` và `profile_hash`)
//! cộng `schema`, `hash_algo`, `ticks`, `checkpoints`. Golden là regression baseline, không phải
//! oracle bản gốc; scenario mới là nơi kiểm luật.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use boom_content::hash::HASH_ALGO;
use boom_core::{Outcome, Phase, event_digest};
use boom_scenario::Error;
use boom_scenario::setup::{Match, Setup, load_profile, parse_checked};
use serde::Deserialize;

const SCHEMA: u32 = 1;

#[derive(Deserialize)]
struct Replay {
    schema: u32,
    hash_algo: String,
    ticks: u32,
    checkpoints: Vec<u32>,
    #[serde(flatten)]
    setup: Setup,
}

fn load(path: &Path) -> Result<Replay, Error> {
    let text = std::fs::read_to_string(path)?;
    let replay: Replay = parse_checked(&text, &["schema", "hash_algo", "ticks", "checkpoints"])
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
    if replay.setup.rules.is_none() || replay.setup.profile_hash.is_none() {
        return Err("replay phải ghi rõ `rules` và `profile_hash`".into());
    }
    Ok(replay)
}

fn base(path: &Path) -> &Path {
    path.parent().unwrap_or(Path::new("."))
}

/// Chạy replay, trả về nội dung golden.
fn run(path: &Path) -> Result<String, Error> {
    let replay = load(path)?;
    let mut m = Match::new(&replay.setup, base(path))?;
    let profile = load_profile(&replay.setup, base(path))?;
    let mut out = format!(
        "# boom-replay golden v{SCHEMA} {HASH_ALGO} profile {} mode {}\n",
        profile.profile_id,
        m.world.mode().id()
    );
    while m.state.tick < replay.ticks && m.state.phase == Phase::Playing {
        m.step()?;
        if replay.checkpoints.contains(&m.state.tick) {
            out += &format!("tick {} state {:#018x}\n", m.state.tick, m.state.hash());
        }
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
    Ok(out)
}

fn golden_path(path: &Path) -> PathBuf {
    path.with_extension("golden")
}

fn verify(path: &Path) -> Result<(), Error> {
    let actual = run(path)?;
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(file)) = (args.first(), args.get(1)) else {
        eprintln!("dùng: boom-replay run|verify|bless|profile-hash <replay.toml>");
        return ExitCode::from(2);
    };
    let path = Path::new(file);
    let result = match cmd.as_str() {
        "run" => run(path).map(|out| print!("{out}")),
        "verify" => verify(path),
        "bless" => run(path).and_then(|out| Ok(std::fs::write(golden_path(path), out)?)),
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
