//! Boundary scenario: dựng trận + input + kỳ vọng. Kiểm hành vi theo luật, độc lập với hash.
//!
//! ```toml
//! name = "Đi tới tường thì dừng ở tâm ô"
//! features = ["V1-011", "V1-012"]
//! claims = ["R-001"]
//! status = "active"        # active: phải qua; pending: feature chưa làm, chạy và báo nhưng không fail
//! ticks = 100              # chạy tối đa N tick (dừng sớm nếu lượt kết thúc)
//! # ... các khóa dựng trận của setup.rs ...
//!
//! [[expect]]               # kiểm sau khi chạy xong `tick` tick
//! tick = 40
//! player = 0
//! cell = [3, 1]            # tùy chọn: pos_mpx, life, bombs, range, speed, facing
//!
//! [[expect]]               # event phát ra trong tick thứ `tick` (EventId.tick = tick - 1)
//! tick = 151
//! event = "bomb_exploded"
//! where = { x = 2, y = 1 } # các trường phải khớp (EventKind::fields)
//! count = 1                # mặc định: ít nhất một
//!
//! [[expect]]               # trạng thái một ô
//! tick = 151
//! at = [3, 1]
//! kind = "floor"           # floor | solid | breakable; tùy chọn: flame, bomb (bool), item ("Fire" | "none")
//!
//! [[expect]]
//! tick = 151
//! bomb_count = 0           # hoặc item_count
//!
//! [end]
//! outcome = "winner:0"     # draw | winner:<id> | playing
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use boom_content::map::CellKind;
use boom_core::{Cell, Dir, Mpx, Outcome, Phase};
use serde::Deserialize;

use crate::Error;
use crate::setup::{DirName, LifeName, Match, Setup, parse_checked};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Active,
    Pending,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    tick: u32,
    player: Option<u8>,
    cell: Option<[i32; 2]>,
    pos_mpx: Option<[i32; 2]>,
    life: Option<LifeName>,
    bombs: Option<u8>,
    range: Option<u8>,
    speed: Option<u8>,
    facing: Option<DirName>,
    event: Option<String>,
    #[serde(rename = "where", default)]
    fields: BTreeMap<String, i64>,
    count: Option<usize>,
    at: Option<[i32; 2]>,
    kind: Option<String>,
    flame: Option<bool>,
    bomb: Option<bool>,
    item: Option<String>,
    bomb_count: Option<usize>,
    item_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct End {
    outcome: String,
}

#[derive(Debug, Deserialize)]
struct ScenarioFile {
    name: String,
    features: Vec<String>,
    #[serde(default)]
    claims: Vec<String>,
    status: Status,
    ticks: u32,
    #[serde(default)]
    expect: Vec<Expect>,
    end: Option<End>,
    #[serde(flatten)]
    setup: Setup,
}

const SCENARIO_KEYS: &[&str] = &[
    "name", "features", "claims", "status", "ticks", "expect", "end",
];

#[derive(Debug)]
pub struct Report {
    pub name: String,
    pub status: Status,
    pub features: Vec<String>,
    /// Rỗng là qua.
    pub failures: Vec<String>,
}

/// Đọc phần dựng trận của một file scenario mà **không chạy**.
///
/// Oracle dùng hàm này để đo trên đúng cấu hình của kịch bản (bản đồ, người chơi, input và
/// profile luật mà file đó dùng).
pub fn load_setup(path: &Path) -> Result<(Setup, u32), Error> {
    let text = std::fs::read_to_string(path)?;
    let file: ScenarioFile = parse_checked(&text, SCENARIO_KEYS)?;
    validate(&file)?;
    Ok((file.setup, file.ticks))
}

pub fn run_file(path: &Path) -> Result<Report, Error> {
    let text = std::fs::read_to_string(path)?;
    let file: ScenarioFile = parse_checked(&text, SCENARIO_KEYS)?;
    validate(&file)?;
    let base = path.parent().unwrap_or(Path::new("."));
    let mut m = Match::new(&file.setup, base)?;
    let mut failures = Vec::new();
    let mut expects: Vec<&Expect> = file.expect.iter().collect();
    expects.sort_by_key(|e| e.tick);
    let mut pending = expects.into_iter().peekable();
    while let Some(e) = pending.next_if(|e| e.tick == 0) {
        check(&m, e, &mut failures);
    }
    while m.state.tick < file.ticks && m.state.phase == Phase::Playing {
        if let Err(e) = m.step() {
            failures.push(e.to_string());
            break;
        }
        while let Some(e) = pending.next_if(|e| e.tick == m.state.tick) {
            check(&m, e, &mut failures);
        }
    }
    for e in pending {
        failures.push(format!(
            "tick {}: lượt dừng ở tick {} trước khi kiểm",
            e.tick, m.state.tick
        ));
    }
    if let Some(i) = m.unused_input() {
        failures.push(format!("input ở tick {} không được dùng", i.tick));
    }
    if let Some(end) = &file.end {
        let actual = match m.state.phase {
            Phase::Playing => "playing".to_owned(),
            Phase::Ended(Outcome::Draw) => "draw".to_owned(),
            Phase::Ended(Outcome::Winner(id)) => format!("winner:{id}"),
        };
        if actual != end.outcome {
            failures.push(format!("kết quả {actual}, cần {}", end.outcome));
        }
    }
    Ok(Report {
        name: file.name,
        status: file.status,
        features: file.features,
        failures,
    })
}

fn validate(file: &ScenarioFile) -> Result<(), Error> {
    if file.features.is_empty() {
        return Err("scenario phải gắn ít nhất một feature ID".into());
    }
    for f in &file.features {
        let ok = f.len() == 6 && f.starts_with("V1-") && f[3..].bytes().all(|b| b.is_ascii_digit());
        if !ok {
            return Err(format!("feature ID `{f}` không đúng dạng V1-NNN").into());
        }
    }
    for c in &file.claims {
        if !c.starts_with("R-") {
            return Err(format!("claim `{c}` không đúng dạng R-NNN").into());
        }
    }
    for e in &file.expect {
        let has_check = e.player.is_some()
            || e.event.is_some()
            || e.at.is_some()
            || e.bomb_count.is_some()
            || e.item_count.is_some();
        if !has_check {
            return Err(format!("expect ở tick {} không kiểm gì", e.tick).into());
        }
        let player_fields = e.cell.is_some()
            || e.pos_mpx.is_some()
            || e.life.is_some()
            || e.bombs.is_some()
            || e.range.is_some()
            || e.speed.is_some()
            || e.facing.is_some();
        if player_fields && e.player.is_none() {
            return Err(format!("expect ở tick {}: kiểm người chơi cần `player`", e.tick).into());
        }
        let cell_fields =
            e.kind.is_some() || e.flame.is_some() || e.bomb.is_some() || e.item.is_some();
        if cell_fields && e.at.is_none() {
            return Err(format!("expect ở tick {}: kiểm ô cần `at`", e.tick).into());
        }
        if (e.count.is_some() || !e.fields.is_empty()) && e.event.is_none() {
            return Err(format!("expect ở tick {}: `count`/`where` cần `event`", e.tick).into());
        }
    }
    Ok(())
}

fn check(m: &Match, e: &Expect, failures: &mut Vec<String>) {
    let mut fail = |msg: String| failures.push(format!("tick {}: {msg}", e.tick));
    let s = &m.state;
    if let Some(id) = e.player {
        let Some(p) = s.player(id) else {
            fail(format!("không có người chơi {id}"));
            return;
        };
        let cell = m.world.cell_at(p.x, p.y);
        if let Some(c) = e.cell
            && Cell::new(c[0], c[1]) != cell
        {
            fail(format!(
                "người chơi {id} ở ô ({},{}), cần ({},{})",
                cell.x, cell.y, c[0], c[1]
            ));
        }
        if let Some([x, y]) = e.pos_mpx
            && (p.x, p.y) != (Mpx(x), Mpx(y))
        {
            fail(format!(
                "người chơi {id} ở mpx ({},{}), cần ({x},{y})",
                p.x.0, p.y.0
            ));
        }
        if let Some(l) = e.life
            && !l.matches(p.life)
        {
            fail(format!("người chơi {id} có life {:?}, cần {l:?}", p.life));
        }
        for (name, want, have) in [
            ("bombs", e.bombs, p.bombs.value),
            ("range", e.range, p.range.value),
            ("speed", e.speed, p.speed.value),
        ] {
            if let Some(w) = want
                && w != have
            {
                fail(format!("người chơi {id}: {name} = {have}, cần {w}"));
            }
        }
        if let Some(d) = e.facing
            && Dir::from(d) != p.facing
        {
            fail(format!("người chơi {id} nhìn {:?}, cần {d:?}", p.facing));
        }
    }
    if let Some(name) = &e.event {
        let hits = m
            .events
            .iter()
            .filter(|ev| ev.id.tick + 1 == e.tick && ev.kind.name() == name)
            .filter(|ev| {
                let fields = ev.kind.fields();
                e.fields
                    .iter()
                    .all(|(k, v)| fields.iter().any(|(fk, fv)| fk == k && fv == v))
            })
            .count();
        let ok = match e.count {
            Some(n) => hits == n,
            None => hits >= 1,
        };
        if !ok {
            let seen: Vec<String> = m
                .events
                .iter()
                .filter(|ev| ev.id.tick + 1 == e.tick)
                .map(|ev| format!("{} {:?}", ev.kind.name(), ev.kind.fields()))
                .collect();
            fail(format!(
                "event `{name}` {:?}: {hits} lần, cần {}; trong tick có: {seen:?}",
                e.fields,
                e.count.map_or("≥1".to_owned(), |n| n.to_string())
            ));
        }
    }
    if let Some([x, y]) = e.at {
        let c = Cell::new(x, y);
        if let Some(k) = &e.kind {
            let have = match s.field.kind(c) {
                Some(CellKind::Floor) => "floor",
                Some(CellKind::Solid) => "solid",
                Some(CellKind::Breakable) => "breakable",
                Some(CellKind::Unknown) => "unknown",
                None => "outside",
            };
            if have != k {
                fail(format!("ô ({x},{y}) là {have}, cần {k}"));
            }
        }
        if let Some(want) = e.flame {
            let have = s.flames.iter().any(|f| f.cell == c);
            if have != want {
                fail(format!("ô ({x},{y}) flame = {have}, cần {want}"));
            }
        }
        if let Some(want) = e.bomb {
            let have = s.bombs.iter().any(|b| b.cell == c);
            if have != want {
                fail(format!("ô ({x},{y}) bomb = {have}, cần {want}"));
            }
        }
        if let Some(want) = &e.item {
            let have = s
                .items
                .iter()
                .find(|i| i.cell == c)
                .map_or("none", |i| m.world.rules().items[usize::from(i.kind.0)].key);
            if have != want {
                fail(format!("ô ({x},{y}) item {have}, cần {want}"));
            }
        }
    }
    if let Some(n) = e.bomb_count
        && s.bombs.len() != n
    {
        fail(format!("{} bom, cần {n}", s.bombs.len()));
    }
    if let Some(n) = e.item_count
        && s.items.len() != n
    {
        fail(format!("{} item, cần {n}", s.items.len()));
    }
}
