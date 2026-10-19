//! Chạy replay xác định và so với golden.
//!
//! ```text
//! boom-replay run <replay.toml>           in kết quả (định dạng golden) ra stdout
//! boom-replay verify <replay.toml>        so với <replay>.golden; khác thì exit 1
//! boom-replay bless <replay.toml>         ghi lại <replay>.golden
//! boom-replay profile-hash <replay.toml>  in định danh profile (để điền `profile_hash`)
//! ```
//!
//! Replay TOML: `schema`, `hash_algo`, `mode`, `rules` (+ `overlays`, đường dẫn tương đối file replay),
//! `profile_hash`, `seed`, `ticks`, `checkpoints`, `map` (ASCII), `[[player]]`, `[[input]]`.
//! Input giữ nguyên từ tick khai báo tới khi đổi — giống phím đang giữ.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use boom_content::hash::HASH_ALGO;
use boom_content::map::Terrain;
use boom_content::rules::{RulesOverlay, RulesProfile};
use boom_core::{
    Dir, Outcome, Phase, PlayerInput, PlayerSetup, Rules, SimulationState, World, event_digest,
    mode_by_id,
};
use serde::Deserialize;

const SCHEMA: u32 = 0;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Replay {
    schema: u32,
    hash_algo: String,
    mode: String,
    rules: PathBuf,
    #[serde(default)]
    overlays: Vec<PathBuf>,
    profile_hash: String,
    seed: u64,
    ticks: u32,
    checkpoints: Vec<u32>,
    map: Vec<String>,
    player: Vec<PlayerEntry>,
    #[serde(default)]
    input: Vec<InputEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerEntry {
    id: u8,
    speed_level: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputEntry {
    tick: u32,
    player: u8,
    dir: DirName,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DirName {
    None,
    Up,
    Down,
    Left,
    Right,
}

impl From<DirName> for Dir {
    fn from(d: DirName) -> Self {
        match d {
            DirName::None => Dir::None,
            DirName::Up => Dir::Up,
            DirName::Down => Dir::Down,
            DirName::Left => Dir::Left,
            DirName::Right => Dir::Right,
        }
    }
}

type Error = Box<dyn std::error::Error>;

fn load(path: &Path) -> Result<(Replay, RulesProfile), Error> {
    let replay: Replay = toml::from_str(&std::fs::read_to_string(path)?)
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
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut profile = RulesProfile::from_toml(&std::fs::read_to_string(dir.join(&replay.rules))?)?;
    for overlay in &replay.overlays {
        let o = RulesOverlay::from_toml(&std::fs::read_to_string(dir.join(overlay))?)?;
        profile = profile.with_overlay(&o)?;
    }
    Ok((replay, profile))
}

fn profile_hash_text(profile: &RulesProfile) -> String {
    format!("{:#018x}", profile.identity_hash())
}

/// Chạy replay, trả về nội dung golden.
fn run(path: &Path) -> Result<String, Error> {
    let (replay, profile) = load(path)?;
    let actual = profile_hash_text(&profile);
    if replay.profile_hash != actual {
        return Err(format!(
            "profile {} có định danh {actual}, replay ghi {}; từ chối phát lại với luật khác",
            profile.profile_id, replay.profile_hash
        )
        .into());
    }
    let mode =
        mode_by_id(&replay.mode).ok_or_else(|| format!("mode {} chưa đăng ký", replay.mode))?;
    let world = World::new(
        Terrain::from_ascii(&replay.map)?,
        Rules::from_profile(&profile)?,
        mode,
    )?;
    let roster: Vec<PlayerSetup> = replay
        .player
        .iter()
        .map(|p| PlayerSetup {
            id: p.id,
            speed_level: p.speed_level,
        })
        .collect();
    let mut state = SimulationState::new_round(&world, &roster, replay.seed)?;
    let ids: Vec<u8> = state.players.iter().map(|p| p.id).collect();
    let mut inputs = replay.input;
    inputs.sort_by_key(|i| i.tick);
    let mut held = vec![PlayerInput::default(); ids.len()];
    let mut pending = inputs.iter().peekable();
    let mut events = Vec::new();
    let mut out = format!(
        "# boom-replay golden v{SCHEMA} {HASH_ALGO} profile {} mode {}\n",
        profile.profile_id, replay.mode
    );
    for tick in 0..replay.ticks {
        if state.phase != Phase::Playing {
            break;
        }
        while let Some(i) = pending.next_if(|i| i.tick == tick) {
            let idx = ids
                .iter()
                .position(|&id| id == i.player)
                .ok_or_else(|| format!("input cho người chơi lạ {}", i.player))?;
            held[idx] = PlayerInput { dir: i.dir.into() };
        }
        state.step(&world, &held, &mut events)?;
        if replay.checkpoints.contains(&state.tick) {
            out += &format!("tick {} state {:#018x}\n", state.tick, state.hash());
        }
    }
    if let Some(i) = pending.next() {
        return Err(format!("input ở tick {} sau khi replay đã dừng", i.tick).into());
    }
    let outcome = match state.phase {
        Phase::Playing => "playing".to_owned(),
        Phase::Ended(Outcome::Draw) => "draw".to_owned(),
        Phase::Ended(Outcome::Winner(id)) => format!("winner:{id}"),
    };
    out += &format!(
        "final tick {} state {:#018x}\nevents {} digest {:#018x}\noutcome {outcome}\n",
        state.tick,
        state.hash(),
        events.len(),
        event_digest(&events)
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
        "profile-hash" => load(path).map(|(_, p)| println!("{}", profile_hash_text(&p))),
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
