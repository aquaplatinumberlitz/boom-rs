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
//! **Tuổi lửa (R-156/R-158, đo client VN):** lửa gây sát thương trong một cửa sổ tồn tại,
//! kể cả khi người chơi đi vào sau khi hàm nổ đã trả về. `flame_ticks` là số tick có lửa,
//! tính cả tick tạo: `until = tick + flame_ticks - 1`. Một tick vẫn gây sát thương ở tick tạo;
//! không trừ thêm tuổi ngay lúc nổ như lựa chọn BnB cũ của R-109.
//!
//! **Blast ≠ lửa:** phá gạch ngay lúc nổ (`server/game.js:401–431`). V1 tách hai ý: `flames` là lửa còn sống
//! (sát thương + hình), kênh blast của tick (`TickScratch`) là ô vừa bị vụ nổ phủ (phá gạch).
//! Kênh này nằm **ngoài state**: nó chỉ sống trong tick, nên không vào state hash và không
//! vào snapshot — state là nguồn duy nhất của tick (xem `scratch.rs`).
//!
//! **Mỗi ô một lần:** kênh blast sắp theo ô và khử trùng, nên một khối mềm bị nhiều bom
//! trong cùng chuỗi nổ phủ vẫn chỉ vào kênh **một** lần ⇒ `blocks` chỉ vỡ khối đó một lần và
//! chỉ rơi **một** item (không roll lặp theo số bom, R-109).
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/explosion/` (`active`). Giá trị luật lấy từ `World::rules()`.

use boom_content::map::CellKind;

use crate::event::{EventKind, EventSink};
use crate::num::Cell;
use crate::scratch::TickScratch;
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
        scratch: &mut TickScratch,
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

        // `until` bao gồm tick cuối; R-158 tính đủ số tick đã cấu hình, kể cả tick nổ.
        // Trừ trước khi cộng để tick sát trần vẫn giữ được lửa một tick.
        let until = flame_ticks
            .checked_sub(1)
            .map(|last_offset| tick.saturating_add(last_offset));
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
        // Kênh blast của tick: `blocks` đọc để phá khối mềm rồi xóa. Kênh nằm ở scratch của
        // tick chứ không ở state, nên không vào state hash và không vào snapshot. Không có bom
        // nổ thì kênh giữ nguyên (hàng chờ rỗng sau tick trước, hoặc phần fixture dựng sẵn).
        // `merge_blasts` sắp theo ô và mỗi ô một lần.
        scratch.merge_blasts(blasted);
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
    use crate::num::{Cell, Mpx};
    use crate::rules::Rules;
    use crate::scratch::TickScratch;
    use crate::sim::{Dir, PlayerInput, PlayerSetup, World};
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
                w.rules().flame_ticks - 1,
                "until bao gồm tick cuối của toàn bộ tuổi lửa đã cấu hình"
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
    fn chain_covering_the_same_block_rolls_the_drop_once() {
        // Kênh blast của tick khử trùng ô, nên một khối mềm bị nhiều bom trong chuỗi nổ phủ
        // chỉ vỡ **một** lần và chỉ rơi **một** item — không roll lặp (R-109).
        let w = world(&["#########", "#0...+..#", "#########"]);
        let mut s = solo(&w);
        // Người chơi đứng ngoài vùng lửa để bước `life` không xen vào kết quả của `blocks`.
        let (x, y) = w.center_of(Cell::new(7, 1));
        s.players[0].x = x;
        s.players[0].y = y;
        // RNG của lượt do test kiểm soát: lần rút đầu trúng nên khối vỡ rơi item.
        s.rng = crate::rng::Rng::new(2);
        // Bom thứ nhất nổ ở tick 0 và kéo theo bom thứ hai trong cùng tick (chain).
        let first = add_bomb(&mut s, Cell::new(2, 1), 3, 0);
        let second = add_bomb(&mut s, Cell::new(4, 1), 3, 100);
        let events = step(&w, &mut s);

        assert!(
            events.iter().any(
                |e| matches!(&e.kind, EventKind::BombExploded { bomb, chained, .. }
                    if *bomb == first && !*chained)
            ),
            "bom thứ nhất nổ"
        );
        assert!(
            events.iter().any(
                |e| matches!(&e.kind, EventKind::BombExploded { bomb, chained, .. }
                    if *bomb == second && *chained)
            ),
            "bom thứ hai nổ dây chuyền cùng tick"
        );
        let destroyed = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::BlockDestroyed { .. }))
            .count();
        assert_eq!(destroyed, 1, "khối mềm chỉ vỡ một lần");
        assert_eq!(s.items.len(), 1, "chỉ rơi một item, không roll lặp");
        assert_eq!(s.items[0].cell, Cell::new(5, 1));
        assert_eq!(s.field.kind(Cell::new(5, 1)), Some(CellKind::Floor));
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
        assert_eq!(flame.until, 10 + w.rules().flame_ticks - 1);
        assert_eq!(flame.cell, Cell::new(1, 1));
        assert_eq!(s.field.kind(Cell::new(1, 1)), Some(CellKind::Floor));
    }

    /// Rules với tuổi lửa theo tick, tính cả tick tạo.
    fn rules_with_flame_ticks(flame_ticks: u32) -> Rules {
        let mut r = rules();
        r.flame_ticks = flame_ticks;
        r
    }

    #[test]
    fn one_tick_flame_traps_at_creation_and_expires_next_tick() {
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
        assert!(s.flames.iter().any(|f| f.cell == Cell::new(0, 0)));
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "lửa một tick vẫn gây sát thương lúc tạo"
        );
        assert!(matches!(
            s.players[0].life,
            crate::state::Life::Trapped { .. }
        ));
        let events = step(&w, &mut s);
        assert!(s.flames.is_empty());
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. }))
        );
    }

    #[test]
    fn one_tick_flame_at_later_tick_still_traps() {
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
        assert!(s.flames.iter().any(|f| f.until == 10));
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "lửa một tick vẫn gây sát thương khi nổ ở tick khác không"
        );
    }

    #[test]
    fn two_tick_flame_lasts_both_ticks_then_expires() {
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
        assert!(!s.flames.is_empty(), "tick thứ hai vẫn còn lửa");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "người đã bị nhốt không phát thêm sự kiện nhốt"
        );
        step(&w, &mut s);
        assert!(s.flames.is_empty(), "hết đủ hai tick mới xóa lửa");
    }

    #[test]
    fn late_entry_traps_on_last_active_tick_but_not_after_expiry() {
        // Ca VN: cùng vị trí vào ở frame18 còn bị nhốt, frame19 đã hết lửa.
        // 19 ở đây là tuổi fixture theo tick; không đổi 19 update VN thành 19 tick V1.
        for entry_tick in [18, 19] {
            let w = world_with(rules_with_flame_ticks(19), &[".....0."]);
            let mut s = solo(&w);
            add_bomb(&mut s, Cell::new(1, 0), 1, 0);
            for _ in 0..entry_tick {
                step(&w, &mut s);
            }
            // Đứng sát bên ngoài ô lửa, rồi đi vào bằng pipeline movement bình thường.
            s.players[0].x = Mpx(w.rules().tile.0 * 3 + 1);
            let mut events = Vec::new();
            s.step(
                &w,
                &[PlayerInput {
                    dir: Dir::Left,
                    ..PlayerInput::default()
                }],
                &mut events,
            )
            .unwrap();
            s.check_invariants(&w).unwrap();
            assert_eq!(w.cell_at(s.players[0].x, s.players[0].y), Cell::new(2, 0));
            assert_eq!(
                events
                    .iter()
                    .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
                entry_tick == 18,
                "vào ở tick{entry_tick}"
            );
        }
    }

    #[test]
    fn one_tick_flame_still_breaks_soft_block_once() {
        let w = world_with(rules_with_flame_ticks(1), &["0+..", "...."]);
        let mut s = solo(&w);
        add_bomb(&mut s, Cell::new(0, 0), 2, 0);
        let mut scratch = TickScratch::new();
        step_with_scratch(&w, &mut s, &mut scratch);

        assert!(!s.flames.is_empty(), "tick tạo còn lửa");
        assert_eq!(
            s.field.kind(Cell::new(1, 0)),
            Some(CellKind::Floor),
            "gạch trong blast vẫn vỡ"
        );
        assert!(
            scratch.blasts().is_empty(),
            "kênh blast rỗng ở mốc tick: `blocks` đã đọc và xóa"
        );
    }

    /// Chạy một tick với scratch do test giữ, để kiểm kênh blast ở biên tick.
    fn step_with_scratch(
        w: &World,
        s: &mut SimulationState,
        scratch: &mut TickScratch,
    ) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        s.step_with_scratch(w, &[PlayerInput::default()], &mut events, scratch)
            .unwrap();
        s.check_invariants(w).unwrap();
        scratch.check(w).unwrap();
        events
    }

    #[test]
    fn blast_channel_is_empty_at_tick_boundary() {
        // Kênh blast là dữ liệu **của tick**, không phải của state: sau tick nó rỗng (`blocks` đã
        // đọc và xóa), nên không có gì để mang qua mốc tick và không gì để bỏ khi đóng gói.
        let w = world(&["0+..", "...."]);
        let mut s = solo(&w);
        add_bomb(&mut s, Cell::new(0, 0), 2, 0);
        let mut scratch = TickScratch::new();
        step_with_scratch(&w, &mut s, &mut scratch);

        assert_eq!(
            s.field.kind(Cell::new(1, 0)),
            Some(CellKind::Floor),
            "gạch trong blast vỡ"
        );
        assert!(
            scratch.blasts().is_empty(),
            "kênh blast phải rỗng ở mốc tick"
        );
    }

    #[test]
    fn blast_channel_stays_empty_during_warmup() {
        // Warmup (R-100): không system nào chạy nên kênh blast không có gì, state chỉ tăng tick.
        let mut r = rules();
        r.warmup_ticks = 3;
        let w = world_with(r, &["0+..", "...."]);
        let mut s = solo(&w);
        add_bomb(&mut s, Cell::new(0, 0), 2, 0);
        let mut scratch = TickScratch::new();
        step_with_scratch(&w, &mut s, &mut scratch);

        assert_eq!(s.tick, 1, "warmup vẫn tăng tick engine");
        assert!(
            scratch.blasts().is_empty(),
            "warmup không chạy system nào nên kênh blast rỗng"
        );
        assert_eq!(
            s.field.kind(Cell::new(1, 0)),
            Some(CellKind::Breakable),
            "gạch chưa vỡ trong warmup"
        );
    }

    #[test]
    fn gameplay_step_matches_the_same_step_with_an_empty_scratch() {
        // Đường gameplay (`step`) tự tạo scratch rỗng: kết quả tick phải y hệt đường fixture
        // với scratch rỗng, tức state là nguồn duy nhất của tick.
        let w = world(&["0+..", "...."]);
        let mut a = solo(&w);
        add_bomb(&mut a, Cell::new(0, 0), 2, 0);
        let mut b = a.clone();
        let events_a = step(&w, &mut a);

        let mut scratch = TickScratch::new();
        let mut events_b = Vec::new();
        b.step_with_scratch(&w, &[PlayerInput::default()], &mut events_b, &mut scratch)
            .unwrap();

        assert_eq!(events_a, events_b, "cùng state + cùng input ⇒ cùng event");
        assert_eq!(
            a.hash(),
            b.hash(),
            "cùng state + cùng input ⇒ cùng state hash"
        );
    }
}
