//! Ghi replay của round thật theo định dạng `boom-replay` schema 1 (V1-026).
//!
//! Module này chỉ ghi dữ liệu đã được authority chốt: input **sau scheduler** theo từng tick
//! (kể cả neutral), setup bất biến của round, checkpoint hash và golden do server tính. Lỗi ghi
//! file không được làm hỏng trận; caller chỉ log rồi bỏ qua.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use boom_content::bundle::sha256_hex;
use boom_content::map::{CellKind, Terrain};
use boom_core::{Dir, PlayerInput};
use boom_room::RoundMember;

use crate::authority::{MatchOutcome, MatchSetup};
use crate::config::ReplaySetup;

/// Ghi một round vào `<replay_dir>/round-<id>.toml` + `.golden`.
pub(crate) struct ReplayLog {
    dir: PathBuf,
    round: u64,
    setup: String,
    players: String,
    inputs: String,
    checkpoints: BTreeSet<u32>,
    golden: String,
}

impl ReplayLog {
    pub(crate) fn new(
        dir: &Path,
        round: u64,
        seed: u64,
        setup: &ReplaySetup,
        match_setup: &MatchSetup,
        members: &[RoundMember],
        terrain: &Terrain,
    ) -> io::Result<Self> {
        let content_sha256 = terrain_sha256(terrain);
        let profile_hash = format!("{:#018x}", match_setup.world.rules().profile_hash);
        let mode = match_setup.world.mode().id();
        let mut setup_text = String::from("# replay do boom-server ghi\n");
        setup_text.push_str("schema = 1\n");
        setup_text.push_str("hash_algo = \"fnv1a64-v0\"\n");
        setup_text.push_str(&format!("content_sha256 = \"{content_sha256}\"\n"));
        setup_text.push_str(&format!(
            "rules = {}\n",
            toml_string(&setup.rules_path.to_string_lossy())
        ));
        setup_text.push_str(&format!("profile_hash = \"{profile_hash}\"\n"));
        setup_text.push_str(&format!("mode = {}\n", toml_string(mode)));
        setup_text.push_str(&format!("seed = {seed}\n"));
        setup_text.push_str("map = [\n");
        for row in terrain_rows(terrain) {
            setup_text.push_str(&format!("    {},\n", toml_string(&row)));
        }
        setup_text.push_str("]\n");
        let mut players_text = String::new();
        for member in members {
            let stats = match member.character {
                boom_room::CharacterId::C00 => match_setup.stats.c00,
                boom_room::CharacterId::C01 => match_setup.stats.c01,
            };
            players_text.push_str("\n[[player]]\n");
            players_text.push_str(&format!("id = {}\n", member.slot.0));
            players_text.push_str(&format!(
                "speed = [{}, {}]\nbombs = [{}, {}]\nrange = [{}, {}]\n",
                stats.speed.value,
                stats.speed.max,
                stats.bombs.value,
                stats.bombs.max,
                stats.range.value,
                stats.range.max
            ));
        }
        let mut checkpoints = setup.checkpoints.clone();
        checkpoints.sort_unstable();
        checkpoints.dedup();
        let golden = format!(
            "# boom-replay golden v1 fnv1a64-v0 profile {} mode {} content {}\n",
            setup.profile_id, mode, content_sha256
        );
        fs::create_dir_all(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            round,
            setup: setup_text,
            players: players_text,
            inputs: String::new(),
            checkpoints: checkpoints.into_iter().collect(),
            golden,
        })
    }

    /// Ghi input đã áp cho tick này, theo đúng thứ tự slot của `state.players`.
    pub(crate) fn record_inputs(&mut self, tick: u32, inputs: &[PlayerInput], slots: &[u8]) {
        for (&input, &slot) in inputs.iter().zip(slots) {
            self.inputs.push_str("\n[[input]]\n");
            self.inputs.push_str(&format!("tick = {tick}\n"));
            self.inputs.push_str(&format!("player = {slot}\n"));
            self.inputs
                .push_str(&format!("dir = \"{}\"\n", dir_name(input.dir)));
            if input.place_bomb {
                self.inputs.push_str("bomb = true\n");
            }
        }
    }

    /// Ghi checkpoint nếu tick này nằm trong danh sách cấu hình.
    pub(crate) fn record_checkpoint(&mut self, tick: u32, hash: u64) {
        if self.checkpoints.contains(&tick) {
            self.golden
                .push_str(&format!("tick {tick} state {hash:#018x}\n"));
        }
    }

    /// Hoàn tất và ghi hai file atomic; trả đường dẫn file replay nếu thành công.
    pub(crate) fn finish(
        self,
        final_tick: u32,
        final_hash: u64,
        events_digest: u64,
        event_count: usize,
        outcome: &str,
    ) -> io::Result<PathBuf> {
        let mut replay = self.setup;
        replay.push_str(&format!("ticks = {final_tick}\n"));
        replay.push_str(&format!(
            "checkpoints = [{}]\n",
            self.checkpoints
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
        replay.push_str(&self.players);
        replay.push_str(&self.inputs);
        let mut golden = self.golden;
        golden.push_str(&format!(
            "final tick {final_tick} state {final_hash:#018x}\n"
        ));
        golden.push_str(&format!(
            "events {event_count} digest {events_digest:#018x}\n"
        ));
        golden.push_str(&format!("outcome {outcome}\n"));

        let replay_path = self.dir.join(format!("round-{}.toml", self.round));
        let golden_path = self.dir.join(format!("round-{}.golden", self.round));
        write_atomic(&replay_path, replay.as_bytes())?;
        write_atomic(&golden_path, golden.as_bytes())?;
        Ok(replay_path)
    }
}

fn dir_name(dir: Dir) -> &'static str {
    match dir {
        Dir::None => "none",
        Dir::Up => "up",
        Dir::Down => "down",
        Dir::Left => "left",
        Dir::Right => "right",
    }
}

fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn terrain_rows(terrain: &Terrain) -> Vec<String> {
    let mut cells: Vec<CellKind> = terrain.kinds.clone();
    for (index, &(x, y)) in terrain.spawns.iter().enumerate() {
        if let Some(cell) =
            cells.get_mut(usize::from(y) * usize::from(terrain.width) + usize::from(x))
        {
            *cell = CellKind::Floor;
            // Spawn là chữ số; nếu vượt 9 thì replay format ASCII không biểu diễn được.
            let _ = index;
        }
    }
    let mut rows = Vec::new();
    for y in 0..terrain.height {
        let mut row = String::new();
        for x in 0..terrain.width {
            let index = usize::from(y) * usize::from(terrain.width) + usize::from(x);
            if let Some(spawn) = terrain
                .spawns
                .iter()
                .position(|&(sx, sy)| sx == x && sy == y)
            {
                row.push(char::from_digit(u32::try_from(spawn).unwrap_or(9), 10).unwrap_or('9'));
                continue;
            }
            row.push(match terrain.kinds[index] {
                CellKind::Floor => '.',
                CellKind::Solid => '#',
                CellKind::Breakable => '+',
                CellKind::Unknown => '?',
            });
        }
        rows.push(row);
    }
    rows
}

fn terrain_sha256(terrain: &Terrain) -> String {
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

pub(crate) fn outcome_label(outcome: MatchOutcome, members: &[RoundMember]) -> String {
    match outcome {
        MatchOutcome::Winner(player) => members
            .iter()
            .find(|member| member.player == player)
            .map_or_else(
                || "draw".to_owned(),
                |member| format!("winner:{}", member.slot.0),
            ),
        MatchOutcome::Draw | MatchOutcome::Aborted => "draw".to_owned(),
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    fs::write(&tmp, data)?;
    #[cfg(not(windows))]
    fs::rename(&tmp, path)?;
    #[cfg(windows)]
    {
        let _ = fs::remove_file(path);
        fs::rename(&tmp, path)?;
    }
    Ok(())
}
