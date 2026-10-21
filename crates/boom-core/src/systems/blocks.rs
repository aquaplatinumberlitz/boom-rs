//! Giấu item đầu lượt, phá khối mềm, lộ item — V1-016.
//!
//! **Đầu lượt** (`on_round_start`): duyệt các khối mềm theo thứ tự hàng rồi cột, mỗi khối
//! rút đúng một số phần nghìn; số nhỏ hơn `Rules::brick_item_chance_permille` thì khối mềm đó
//! giấu một item, loại chọn theo trọng số `Rules::items` (lấy mẫu không lệch bằng `Rng::below`).
//! Item được quyết trước trận (R-050), nên cùng seed cho cùng bố cục item giấu (R-067).
//!
//! **Trong lượt** (`run`): ô nào bị vụ nổ phủ (kênh blast của tick) và là khối mềm thì khối vỡ
//! thành nền (`block_destroyed`); nếu khối vừa vỡ giấu item thì item lộ ra, nằm trên nền
//! (`item_revealed`). Lửa không phá item đang nằm trên nền (R-068): item chỉ rời nền khi người
//! chơi nhặt (R-054).
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/blocks/` (`active`). Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use boom_content::map::CellKind;

use crate::event::{EventKind, EventSink};
use crate::num::Cell;
use crate::rng::Rng;
use crate::rules::ItemDef;
use crate::scratch::TickScratch;
use crate::sim::{PlayerInput, World};
use crate::state::{FloorItem, ItemKind, SimulationState};
use crate::system::System;

/// Mẫu số của đơn vị "phần nghìn" (`drop.brick_item_chance`): quy đổi đơn vị, không phải
/// hằng số luật chơi. Giá trị luật nằm ở `Rules::brick_item_chance_permille`.
const PERMILLE: u32 = 1000;

#[derive(Debug)]
pub struct Blocks;

pub static BLOCKS: Blocks = Blocks;

impl System for Blocks {
    fn name(&self) -> &'static str {
        "blocks"
    }

    fn on_round_start(&self, world: &World, state: &mut SimulationState) {
        let rules = world.rules();
        let total_weight: u32 = rules.items.iter().map(|i| i.weight).sum();
        // Ô khối mềm, thứ tự hàng rồi cột: thứ tự duyệt là thứ tự rút RNG.
        let cells: Vec<Cell> = {
            let field = &state.field;
            field
                .cells()
                .filter(|c| field.kind(*c) == Some(CellKind::Breakable))
                .collect()
        };
        for cell in cells {
            // Mỗi khối mềm rút đúng một số phần nghìn, kể cả khi không giấu item, để dòng RNG
            // chỉ phụ thuộc số khối mềm chứ không phụ thuộc giá trị xác suất của profile.
            if state.rng.below(PERMILLE) >= rules.brick_item_chance_permille {
                continue;
            }
            let Some(kind) = pick_by_weight(&mut state.rng, &rules.items, total_weight) else {
                // Không có loại item nào có trọng số: khối mềm không giấu được gì.
                continue;
            };
            state.field.set_hidden(cell, Some(kind));
        }
    }

    fn run(
        &self,
        _world: &World,
        state: &mut SimulationState,
        _inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
        scratch: &mut TickScratch,
    ) {
        // Ô bị vụ nổ phủ: khối mềm vỡ thành nền. BnB phá gạch **lúc nổ** (`game.js:401–431`),
        // tức trước bước giảm TTL, nên việc phá gạch không phụ thuộc tuổi lửa: `flame_ticks = 1`
        // vẫn phá gạch. Lửa không phá item đang nằm trên nền (R-068).
        // Kênh blast là hàng chờ trong tick: đọc xong thì xóa (như `movement` xóa `grace`).
        let blasted = scratch.take_blasts();
        for cell in blasted {
            if state.field.kind(cell) != Some(CellKind::Breakable) {
                continue;
            }
            state.field.set_kind(cell, CellKind::Floor);
            events.emit(EventKind::BlockDestroyed { cell });
            let Some(kind) = state.field.hidden(cell) else {
                continue;
            };
            state.field.set_hidden(cell, None);
            let id = state.ids.next_id();
            state.items.push(FloorItem { id, kind, cell });
            state.items.sort_by_key(|i| i.id);
            events.emit(EventKind::ItemRevealed {
                item: id,
                kind,
                cell,
            });
        }
    }
}

/// Chọn loại item theo trọng số: rút số trong `[0, tổng trọng số)` rồi đi theo bảng trọng số
/// cộng dồn. Loại có trọng số 0 không bao giờ được chọn.
fn pick_by_weight(rng: &mut Rng, items: &[ItemDef], total_weight: u32) -> Option<ItemKind> {
    if total_weight == 0 {
        return None;
    }
    let mut pick = rng.below(total_weight);
    for (index, item) in items.iter().enumerate() {
        if pick < item.weight {
            return u16::try_from(index).ok().map(ItemKind);
        }
        pick -= item.weight;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::map::Terrain;
    use boom_content::rules::RulesProfile;

    use crate::event::{EventId, EventSink, SimulationEvent};
    use crate::mode::mode_by_id;
    use crate::rules::Rules;
    use crate::sim::PlayerSetup;
    use crate::state::{Bomb, CharacterStats, EntityId, Stat};
    use crate::system::System;

    const PROFILE: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

    fn rules() -> Rules {
        Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap()
    }

    fn world<S: AsRef<str>>(rows: &[S]) -> World {
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(rows).unwrap(), rules(), ffa).unwrap()
    }

    fn setup() -> PlayerSetup {
        PlayerSetup {
            id: 0,
            stats: CharacterStats {
                speed: Stat::new(5, 7),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        }
    }

    /// Bản đồ toàn khối mềm: ô (0,0) là nền có spawn, các ô còn lại là khối mềm.
    fn soft_map(width: usize, height: usize) -> Vec<String> {
        let mut rows = Vec::with_capacity(height);
        for y in 0..height {
            let mut row = String::with_capacity(width);
            for x in 0..width {
                row.push(if x == 0 && y == 0 { '0' } else { '+' });
            }
            rows.push(row);
        }
        rows
    }

    fn hidden_items(state: &SimulationState) -> Vec<(Cell, ItemKind)> {
        state
            .field
            .cells()
            .filter_map(|c| state.field.hidden(c).map(|k| (c, k)))
            .collect()
    }

    fn new_round<S: AsRef<str>>(rows: &[S], seed: u64) -> SimulationState {
        let w = world(rows);
        SimulationState::new_round(&w, &[setup()], seed).unwrap()
    }

    /// Chạy **riêng** system `blocks`: kênh blast do `explosion` ghi, `blocks` đọc rồi xóa,
    /// nên test phần này phải dựng sẵn kênh rồi gọi thẳng system.
    fn run_blocks(
        w: &World,
        state: &mut SimulationState,
        scratch: &mut TickScratch,
    ) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        let mut sink = EventSink::new(state.tick, &mut events);
        BLOCKS.run(w, state, &[PlayerInput::default()], &mut sink, scratch);
        state.check_invariants(w).unwrap();
        events
    }

    /// Ô vừa bị vụ nổ phủ: kênh mà `explosion` ghi, `blocks` đọc để phá khối mềm.
    fn blast(cell: Cell) -> Vec<Cell> {
        vec![cell]
    }

    #[test]
    fn hidden_items_are_decided_before_the_round_starts() {
        let rows = soft_map(6, 4);
        let state = new_round(&rows, 7);
        // Item giấu chỉ nằm trong khối mềm.
        for cell in state.field.cells() {
            if state.field.hidden(cell).is_some() {
                assert_eq!(state.field.kind(cell), Some(CellKind::Breakable));
            }
        }
        // Chưa có item nào nằm trên nền lúc đầu lượt: item chỉ lộ ra khi khối vỡ.
        assert!(state.items.is_empty());
    }

    #[test]
    fn same_seed_gives_same_hidden_items_and_other_seed_differs() {
        let rows = soft_map(8, 6);
        let a = new_round(&rows, 7);
        let b = new_round(&rows, 7);
        assert_eq!(hidden_items(&a), hidden_items(&b));
        let differs = (1..40).any(|seed| hidden_items(&new_round(&rows, seed)) != hidden_items(&a));
        assert!(differs, "đổi seed phải đổi được bố cục item giấu");
    }

    #[test]
    fn hidden_kind_follows_item_weights() {
        // Mẫu lớn: tỉ lệ loại item giấu phải bám trọng số trong `Rules::items`.
        let rows = soft_map(40, 40);
        let mut counts = [0usize; 3];
        let mut total = 0usize;
        for seed in 0..40u64 {
            let state = new_round(&rows, seed);
            for (_, kind) in hidden_items(&state) {
                counts[usize::from(kind.0)] += 1;
                total += 1;
            }
        }
        assert!(total > 1000, "mẫu phải đủ lớn, có {total} item");
        let weights: Vec<u32> = rules().items.iter().map(|i| i.weight).collect();
        let sum: u32 = weights.iter().sum();
        for (i, weight) in weights.iter().enumerate() {
            // So phần nghìn bằng số nguyên: crate cấm phép số thực.
            let expected = weight * 1000 / sum;
            let got = (counts[i] as u32 * 1000) / total as u32;
            assert!(
                got.abs_diff(expected) <= 20,
                "loại {i}: {got}‰ khác {expected}‰ quá 20‰"
            );
        }
    }

    #[test]
    fn pick_by_weight_is_unbiased_on_a_large_sample() {
        let r = rules();
        let items = r.items.clone();
        let total: u32 = items.iter().map(|i| i.weight).sum();
        let mut rng = Rng::new(12345);
        let mut counts = [0usize; 3];
        let draws = 100_000u32;
        for _ in 0..draws {
            let kind = pick_by_weight(&mut rng, &items, total).expect("luôn chọn được loại");
            counts[usize::from(kind.0)] += 1;
        }
        let sum: u32 = items.iter().map(|i| i.weight).sum();
        for (i, item) in items.iter().enumerate() {
            let expected = item.weight * 1000 / sum;
            let got = (counts[i] as u32 * 1000) / draws;
            assert!(
                got.abs_diff(expected) <= 10,
                "loại {i}: {got}‰ khác {expected}‰ quá 10‰"
            );
        }
    }

    #[test]
    fn blast_breaks_block_once_and_reveals_item() {
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        // Giấu sẵn item trong khối mềm như scenario dựng sẵn.
        state.field.set_hidden(Cell::new(2, 1), Some(ItemKind(2)));
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert_eq!(
            events,
            vec![
                SimulationEvent {
                    id: EventId { tick: 0, seq: 0 },
                    kind: EventKind::BlockDestroyed {
                        cell: Cell::new(2, 1)
                    },
                },
                SimulationEvent {
                    id: EventId { tick: 0, seq: 1 },
                    kind: EventKind::ItemRevealed {
                        item: EntityId(1),
                        kind: ItemKind(2),
                        cell: Cell::new(2, 1),
                    },
                },
            ]
        );
        assert_eq!(state.field.kind(Cell::new(2, 1)), Some(CellKind::Floor));
        assert_eq!(state.field.hidden(Cell::new(2, 1)), None);
        assert_eq!(
            state.items,
            vec![FloorItem {
                id: EntityId(1),
                kind: ItemKind(2),
                cell: Cell::new(2, 1),
            }]
        );

        // Khối đã vỡ: không phá thêm lần nữa.
        let later = run_blocks(&w, &mut state, &mut scratch);
        assert!(later.is_empty(), "khối mềm chỉ vỡ một lần");
        assert_eq!(state.items.len(), 1);
    }

    #[test]
    fn blast_keeps_item_lying_on_the_floor() {
        // R-068: lửa không phá item đang nằm trên nền.
        let rows = ["#####", "#0..#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        let id = state.ids.next_id();
        state.items.push(FloorItem {
            id,
            kind: ItemKind(1),
            cell: Cell::new(2, 1),
        });
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::ItemDestroyed { .. })),
            "lửa không được phá item trên nền"
        );
        assert_eq!(state.items.len(), 1, "item trên nền còn nguyên");
    }

    #[test]
    fn block_without_hidden_item_reveals_nothing() {
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        // Bỏ item giấu do đầu lượt tự chọn, để khối mềm rỗng.
        state.field.set_hidden(Cell::new(2, 1), None);
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].kind,
            EventKind::BlockDestroyed {
                cell: Cell::new(2, 1)
            }
        );
        assert!(state.items.is_empty(), "khối rỗng không lộ item nào");
    }

    #[test]
    fn fixture_blast_channel_gives_the_same_result_as_a_real_explosion() {
        // Đường fixture (kênh blast dựng sẵn, dùng để test riêng `blocks`) phải cho cùng kết
        // quả với đường gameplay (vụ nổ thật của `explosion`): cùng ô bị phủ ⇒ cùng khối vỡ,
        // cùng item lộ ra.
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);

        // (a) đường fixture: dựng sẵn kênh blast.
        let mut fixture = new_round(&rows, 1);
        fixture.field.set_hidden(Cell::new(2, 1), Some(ItemKind(2)));
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        run_blocks(&w, &mut fixture, &mut scratch);

        // (b) đường gameplay: bom nổ thật ở tick 0, tầm 1 ⇒ vụ nổ phủ ô (1,1) và (2,1).
        let mut gameplay = new_round(&rows, 1);
        gameplay
            .field
            .set_hidden(Cell::new(2, 1), Some(ItemKind(2)));
        let bomb_id = gameplay.ids.next_id();
        gameplay.bombs.push(Bomb {
            id: bomb_id,
            owner: 0,
            cell: Cell::new(1, 1),
            range: 1,
            explodes_at: 0,
            grace: vec![],
        });
        let mut events = Vec::new();
        gameplay
            .step(&w, &[PlayerInput::default()], &mut events)
            .expect("step");

        assert_eq!(
            fixture.field.kind(Cell::new(2, 1)),
            gameplay.field.kind(Cell::new(2, 1)),
            "cả hai đường phải vỡ cùng khối mềm"
        );
        let kinds = |s: &SimulationState| -> Vec<(Cell, ItemKind)> {
            s.items.iter().map(|i| (i.cell, i.kind)).collect()
        };
        assert_eq!(
            kinds(&fixture),
            kinds(&gameplay),
            "cả hai đường phải lộ cùng item ở cùng ô"
        );
    }
}
