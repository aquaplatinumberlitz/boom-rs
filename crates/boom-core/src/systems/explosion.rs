//! Fuse, nổ, lan, chặn và nổ dây chuyền — V1-014, V1-015.
//!
//! Mỗi tick: xóa lửa đã hết hạn (`until < tick`), rồi nổ mọi bom có `explodes_at <= tick`.
//! Lửa phủ ô đặt bom và lan tối đa `Bomb.range` ô mỗi hướng; khối cứng và mép lưới chặn,
//! khối mềm đầu tiên có lửa rồi chặn. Bom nằm trong lửa nổ dây chuyền ngay trong tick đó.
//!
//! Tập bom nổ trong tick là bao đóng của bom đến hạn và bom bị lửa chạm; sau đó xử lý theo `id`
//! tăng dần, mỗi bom đúng một lần (RULES_V1 R-033). Nhiều lửa cùng một ô giữ `Flame` có `until`
//! lớn nhất, hòa thì chọn `source` có `id` nhỏ hơn; danh sách vẫn sắp theo ô, mỗi ô một flame.
//! Không phá khối và không nhốt người: việc của `blocks`/`life`.
//!
//! **Chuỗi nổ (R-109, BnB `server/game.js:401–431`):** lửa tính trên **lưới chưa đổi**. BnB gom
//! ô bị phá vào tập `destroyed` rồi mới áp dụng **sau khi hàng đợi nổ rỗng**, nên gạch vẫn chắn
//! lửa của các bom sau trong cùng tick. V1 giữ nguyên: `explosion` chạy trước `blocks` nên toàn
//! bộ lửa của tick tính trên lưới trước khi gạch vỡ.
//!
//! **Thứ tự nổ:** V1 xử lý theo `id` tăng dần (R-033) thay hàng đợi FIFO theo thứ tự chèn của
//! `Map` JS — khác biệt có chủ ý (ADR 0012). *Tập* bom nổ thì giống hệt: cùng là bao đóng dây
//! chuyền, chỉ khác thứ tự phát event và nguồn lửa khi nhiều bom chồng một ô.
//!
//! **Tuổi lửa (R-109, BnB `server/game.js:448–451`):** vụ nổ sinh với `ttl = flame_ticks` rồi
//! bị giảm ngay trong tick nổ, nên lửa có mặt ở `flame_ticks - 1` tick, tức `until` =
//! `tick + flame_ticks - 2`. `flame_ticks = 1` ⇒ **không còn tick nào có lửa**: BnB xóa vụ
//! nổ ngay trong tick nổ (ttl 1 → 0) trước khi gây sát thương, nên V1 **không tạo lửa** —
//! state không có ô lửa, không ai bị nhốt, không có hình lửa.
//!
//! **Blast ≠ lửa:** BnB phá gạch ngay lúc nổ (`server/game.js:401–431`), tức **trước** bước
//! giảm TTL, nên `flame_ticks = 1` vẫn phá gạch. V1 tách hai ý: `flames` là lửa còn sống
//! (sát thương + hình), `state.blasts` là ô vừa bị vụ nổ phủ (kênh để `blocks` phá gạch).
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/explosion/` (`active`). Giá trị luật lấy từ `World::rules()`.

use boom_content::map::CellKind;

use crate::event::{EventKind, EventSink};
use crate::num::Cell;
use crate::sim::{PlayerInput, World};
use crate::state::{Bomb, EntityId, Field, Flame, SimulationState};
use crate::system::System;

/// Bốn hướng lan của lửa; thứ tự cố định để kết quả không phụ thuộc duyệt lưới.
const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

#[derive(Debug)]
pub struct Explosion;

pub static EXPLOSION: Explosion = Explosion;

impl System for Explosion {
    fn name(&self) -> &'static str {
        "explosion"
    }

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        _inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
    ) {
        let tick = state.tick;
        let flame_ticks = world.rules().flame_ticks;

        // `until` là tick cuối còn lửa (bao gồm), nên chỉ bỏ khi đã nhỏ hơn tick hiện tại.
        state.flames.retain(|f| f.until >= tick);

        let mut exploding: Vec<EntityId> = state
            .bombs
            .iter()
            .filter(|b| b.explodes_at <= tick)
            .map(|b| b.id)
            .collect();
        if exploding.is_empty() {
            return;
        }
        exploding.sort_unstable();

        // Bao đóng dây chuyền: thêm bom bị lửa của bom đã nổ trong tick này chạm tới. Lặp tới
        // khi không thêm được, nên mỗi bom vào tập đúng một lần.
        loop {
            let mut added = false;
            for bomb in &state.bombs {
                if exploding.contains(&bomb.id) {
                    continue;
                }
                let reached = state
                    .bombs
                    .iter()
                    .filter(|src| exploding.contains(&src.id))
                    .any(|src| blast_cells(&state.field, src).contains(&bomb.cell));
                if reached {
                    exploding.push(bomb.id);
                    added = true;
                }
            }
            if !added {
                break;
            }
        }
        exploding.sort_unstable();

        // BnB `server/game.js:448–451`: vụ nổ sinh với `ttl = flame_ticks` rồi bị giảm ngay
        // trong tick nổ, nên lửa chỉ còn `flame_ticks - 1` tick — `until` là tick cuối còn lửa.
        // `flame_ticks = 1` ⇒ ttl về 0 ngay trong tick nổ ⇒ BnB **xoá vụ nổ trước khi gây
        // sát thương**: không có tick nào có lửa, nên V1 không tạo `Flame` nào (state không có
        // ô lửa ⇒ không nhốt ai, không có hình lửa). Không dựa vào phép trừ bão hòa để biểu
        // diễn "lửa đã chết": `until` là tick cuối còn lửa, nên lửa chết thì không có `Flame`.
        let until = match flame_ticks {
            0 | 1 => None,
            ticks => Some(tick.saturating_add(ticks).saturating_sub(2)),
        };
        let mut new_flames = Vec::new();
        let mut blasted: Vec<Cell> = Vec::new();
        for id in &exploding {
            let bomb = state
                .bombs
                .iter()
                .find(|b| b.id == *id)
                .expect("bom trong tập nổ phải còn trong state");
            events.emit(EventKind::BombExploded {
                bomb: bomb.id,
                owner: bomb.owner,
                cell: bomb.cell,
                chained: bomb.explodes_at > tick,
            });
            for cell in blast_cells(&state.field, bomb) {
                // Ô bị vụ nổ phủ: BnB phá gạch ngay lúc nổ, không phụ thuộc tuổi lửa.
                blasted.push(cell);
                let Some(until) = until else {
                    continue;
                };
                new_flames.push(Flame {
                    cell,
                    until,
                    source: bomb.id,
                    owner: bomb.owner,
                });
            }
        }
        state.bombs.retain(|b| !exploding.contains(&b.id));
        // Kênh blast: `blocks` đọc để phá khối mềm rồi xóa. Không có bom nổ thì kênh giữ
        // nguyên (hàng chờ rỗng sau tick trước).
        blasted.sort_unstable();
        blasted.dedup();
        state.blasts = blasted;
        state.flames.append(&mut new_flames);
        state.flames.sort_by_key(|f| f.cell);
        state.flames.dedup_by(|later, earlier| {
            if later.cell != earlier.cell {
                return false;
            }
            if preferred_flame(later, earlier) {
                std::mem::swap(later, earlier);
            }
            true
        });
    }
}

/// Các ô lửa của một bom: ô đặt bom, rồi từng hướng tới `range`; khối mềm tính một ô rồi dừng.
fn blast_cells(field: &Field, bomb: &Bomb) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(1 + usize::from(bomb.range) * DIRECTIONS.len());
    cells.push(bomb.cell);
    for (dx, dy) in DIRECTIONS {
        let mut cell = bomb.cell;
        for _ in 0..bomb.range {
            cell = cell.offset(dx, dy);
            match field.kind(cell) {
                Some(CellKind::Floor) => cells.push(cell),
                Some(CellKind::Breakable) => {
                    cells.push(cell);
                    break;
                }
                // Khối cứng, ô Unknown (state sai contract) và ngoài lưới đều chặn lửa.
                _ => break,
            }
        }
    }
    cells
}

/// Lửa được giữ khi trùng ô: `until` xa hơn, hòa thì `source` nhỏ hơn (RULES_V1 R-033).
fn preferred_flame(a: &Flame, b: &Flame) -> bool {
    a.until > b.until || (a.until == b.until && a.source.0 < b.source.0)
}

#[cfg(test)]
mod tests {
    use boom_content::map::{CellKind, Terrain};
    use boom_content::rules::RulesProfile;

    use crate::event::{EventKind, SimulationEvent};
    use crate::mode::mode_by_id;
    use crate::num::Cell;
    use crate::rules::Rules;
    use crate::sim::{PlayerInput, PlayerSetup, World};
    use crate::state::SimulationState;
    use crate::state::{Bomb, CharacterStats, EntityId, Flame, Stat};

    const PROFILE: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

    fn rules() -> Rules {
        Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap()
    }

    fn world(rows: &[&str]) -> World {
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(rows).unwrap(), rules(), ffa).unwrap()
    }

    fn world_with(rules: Rules, rows: &[&str]) -> World {
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(rows).unwrap(), rules, ffa).unwrap()
    }

    fn solo(w: &World) -> SimulationState {
        let setup = PlayerSetup {
            id: 0,
            stats: CharacterStats {
                speed: Stat::new(5, 7),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        };
        SimulationState::new_round(w, &[setup], 1).unwrap()
    }

    fn add_bomb(s: &mut SimulationState, cell: Cell, range: u8, explodes_at: u32) -> EntityId {
        let id = s.ids.next_id();
        s.bombs.push(Bomb {
            id,
            owner: 0,
            cell,
            range,
            explodes_at,
            grace: vec![],
        });
        s.bombs.sort_by_key(|b| b.id);
        id
    }

    fn step(w: &World, s: &mut SimulationState) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        s.step(w, &[PlayerInput::default()], &mut events).unwrap();
        s.check_invariants(w).unwrap();
        events
    }

    #[test]
    fn blast_includes_first_breakable_and_stops() {
        let w = world(&["0.+.", "...."]);
        let mut s = solo(&w);
        add_bomb(&mut s, Cell::new(1, 0), 3, 0);
        step(&w, &mut s);
        let flamed = |x, y| s.flames.iter().any(|f| f.cell == Cell::new(x, y));
        assert!(flamed(2, 0), "khối mềm đầu tiên phải có lửa");
        assert!(!flamed(3, 0), "lửa không được xuyên qua khối mềm");
        assert!(flamed(1, 0), "ô đặt bom có lửa");
        assert!(flamed(0, 0), "lan trái trong tầm");
        assert!(flamed(1, 1), "lan xuống trong tầm");
    }

    #[test]
    fn overlapping_flames_keep_one_cell_and_smallest_source() {
        let w = world(&["0.....", "......"]);
        let mut s = solo(&w);
        let first = add_bomb(&mut s, Cell::new(1, 1), 2, 0);
        let second = add_bomb(&mut s, Cell::new(3, 1), 2, 0);
        let events = step(&w, &mut s);

        assert_eq!(events.len(), 2);
        for x in 1..=3 {
            let cell = Cell::new(x, 1);
            let flames: Vec<_> = s.flames.iter().filter(|f| f.cell == cell).collect();
            assert_eq!(flames.len(), 1, "mỗi ô chỉ giữ một flame");
            assert_eq!(flames[0].source, first, "hòa until thì source id nhỏ thắng");
            assert_ne!(flames[0].source, second);
            assert_eq!(
                flames[0].until,
                w.rules().flame_ticks - 2,
                "until = tick nổ (0) + flame_ticks - 2: lửa mới bị giảm ngay trong tick nổ"
            );
        }
    }

    #[test]
    fn three_bombs_chain_once_in_id_order() {
        let w = world(&["#######", "#.....#", "#...0.#", "#######"]);
        let mut s = solo(&w);
        let first = add_bomb(&mut s, Cell::new(1, 1), 1, 0);
        let second = add_bomb(&mut s, Cell::new(2, 1), 1, 100);
        let third = add_bomb(&mut s, Cell::new(3, 1), 1, 100);
        let events = step(&w, &mut s);

        let exploded: Vec<_> = events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::BombExploded {
                    bomb,
                    cell,
                    chained,
                    ..
                } => Some((bomb, cell, chained)),
                _ => None,
            })
            .collect();
        assert_eq!(
            exploded,
            vec![
                (first, Cell::new(1, 1), false),
                (second, Cell::new(2, 1), true),
                (third, Cell::new(3, 1), true),
            ],
            "xử lý theo id tăng dần, cùng tick"
        );
        assert!(s.bombs.is_empty());

        let later = step(&w, &mut s);
        assert!(
            !later
                .iter()
                .any(|e| matches!(e.kind, EventKind::BombExploded { .. })),
            "mỗi bom chỉ nổ một lần"
        );
    }

    #[test]
    fn expired_flame_is_removed_before_new_flames() {
        let w = world(&["0...", "...."]);
        let mut s = solo(&w);
        s.tick = 10;
        s.flames.push(Flame {
            cell: Cell::new(2, 2),
            until: 9,
            source: EntityId(0),
            owner: 0,
        });
        add_bomb(&mut s, Cell::new(1, 1), 1, 10);
        step(&w, &mut s);

        assert!(
            !s.flames.iter().any(|f| f.cell == Cell::new(2, 2)),
            "lửa until < tick bị xóa"
        );
        let flame = s
            .flames
            .iter()
            .find(|f| f.cell == Cell::new(1, 1))
            .expect("bom vừa nổ phải có lửa");
        assert_eq!(flame.until, 10 + w.rules().flame_ticks - 2);
        assert_eq!(flame.cell, Cell::new(1, 1));
        assert_eq!(s.field.kind(Cell::new(1, 1)), Some(CellKind::Floor));
    }

    /// Rules với `flame_ticks` cho trước: `flame_ticks = 1` là biên không còn tick nào lửa.
    fn rules_with_flame_ticks(flame_ticks: u32) -> Rules {
        let mut r = rules();
        r.flame_ticks = flame_ticks;
        r
    }

    #[test]
    fn flame_ticks_one_leaves_no_flame_at_tick_zero() {
        // BnB `server/game.js:448–451`: ttl 1 → 0 ngay trong tick nổ nên vụ nổ bị xóa trước
        // khi áp sát thương — không có ô lửa nào, không ai bị nhốt.
        let w = world_with(rules_with_flame_ticks(1), &["0...", "...."]);
        let mut s = solo(&w);
        add_bomb(&mut s, Cell::new(1, 0), 1, 0);
        let events = step(&w, &mut s);

        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::BombExploded { .. })),
            "bom vẫn nổ"
        );
        assert!(s.flames.is_empty(), "flame_ticks = 1: state không có ô lửa");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "không có lửa thì không ai bị nhốt"
        );
        assert_eq!(s.players[0].life, crate::state::Life::Alive);
    }

    #[test]
    fn flame_ticks_one_leaves_no_flame_at_later_tick() {
        // Cùng biên ở tick khác 0: tick nổ không phải tick đầu, kết luận phải y hệt.
        let w = world_with(rules_with_flame_ticks(1), &["0...", "...."]);
        let mut s = solo(&w);
        s.tick = 10;
        add_bomb(&mut s, Cell::new(1, 0), 1, 10);
        let events = step(&w, &mut s);

        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::BombExploded { .. })),
            "bom vẫn nổ"
        );
        assert!(s.flames.is_empty(), "flame_ticks = 1: state không có ô lửa");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "không có lửa thì không ai bị nhốt"
        );
    }

    #[test]
    fn flame_ticks_two_damages_exactly_one_tick() {
        // Biên kế bên: ttl 2 → 1 nên lửa còn ở đúng tick nổ và hết ở tick sau.
        let w = world_with(rules_with_flame_ticks(2), &["0...", "...."]);
        let mut s = solo(&w);
        add_bomb(&mut s, Cell::new(1, 0), 1, 0);
        let events = step(&w, &mut s);

        assert!(
            s.flames.iter().any(|f| f.cell == Cell::new(0, 0)),
            "tick nổ vẫn còn lửa ở ô người chơi"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "lửa còn sống ở tick nổ nên người chơi bị nhốt"
        );

        let events = step(&w, &mut s);
        assert!(s.flames.is_empty(), "tick sau lửa hết hạn");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "lửa hết hạn thì không nhốt thêm"
        );
    }

    #[test]
    fn blast_breaks_soft_block_even_when_no_flame_survives() {
        // BnB phá gạch lúc nổ (`server/game.js:401–431`), tức trước bước giảm TTL: `flame_ticks`
        // = 1 vẫn phá gạch dù không còn ô lửa nào.
        let w = world_with(rules_with_flame_ticks(1), &["0+..", "...."]);
        let mut s = solo(&w);
        add_bomb(&mut s, Cell::new(0, 0), 2, 0);
        step(&w, &mut s);

        assert!(s.flames.is_empty(), "flame_ticks = 1: không còn ô lửa");
        assert_eq!(
            s.field.kind(Cell::new(1, 0)),
            Some(CellKind::Floor),
            "gạch trong blast vẫn vỡ"
        );
        assert!(
            s.blasts.is_empty(),
            "kênh blast rỗng ở mốc tick: `blocks` đã đọc và xóa"
        );
    }
}
