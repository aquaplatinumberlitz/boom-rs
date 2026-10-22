//! Phá khối mềm, lửa đốt item và **rơi item khi gạch vỡ** — V1-016.
//!
//! **Roll khi gạch vỡ** (R-106, R-109; BnB `server/game.js:424–430`): V1 **không** giấu item
//! trước lượt nữa. Mỗi khối mềm bị vụ nổ phủ sẽ vỡ thành nền, rồi mới rút một số phần nghìn:
//! nhỏ hơn `Rules::brick_item_chance_permille` (`drop.brick_item_chance`) thì khối vỡ **rơi ra
//! một item**, loại chọn theo trọng số `Rules::items` (bảng trọng số cộng dồn, lấy mẫu không lệch
//! bằng `Rng::below`). Cùng seed cho cùng bố cục item rơi.
//!
//! **Lửa đốt item cũ trước, item mới sinh sau** (R-109; BnB `server/game.js:415–417,424–430`):
//! BnB đốt item nằm trên nền ở ô bị vụ nổ phủ *trong lúc xử lý bom*, rồi mới phá khối và rơi
//! item mới ở ô vừa vỡ. V1 giữ đúng thứ tự đó: `blocks` đốt item cũ ở **mọi** ô bị vụ nổ phủ
//! trước, rồi mới vỡ khối và rơi item mới — nên item vừa rơi không bị đốt bởi chính vụ nổ đó.
//!
//! **Mỗi ô một lần** (R-109): kênh blast của tick (`TickScratch`) đã sắp theo ô và khử trùng,
//! nên một khối mềm bị nhiều bom trong cùng chuỗi nổ phủ vẫn chỉ vỡ **một** lần và chỉ rơi
//! **một** item — không roll lặp theo số bom.
//!
//! **Thứ tự rút RNG:** mỗi khối vỡ rút đúng **một** số phần nghìn (kể cả khi trượt), và chỉ khi
//! trúng mới rút thêm một số để chọn loại item — giống BnB (`game.js:429` rút một số cho xác
//! suất, `rollItem()` rút thêm một số cho loại). Các khối được rút theo thứ tự ô (hàng rồi
//! cột) của kênh blast, **không** theo thứ tự chèn của `Map` JS: khác biệt có chủ ý (ADR 0012).
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/blocks/` (`active`). Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use boom_content::map::CellKind;

use crate::event::{EventKind, EventSink};
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

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        _inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
        scratch: &mut TickScratch,
    ) {
        let rules = world.rules();
        let total_weight: u32 = rules.items.iter().map(|i| i.weight).sum();

        // Ô bị vụ nổ phủ: kênh blast của tick đã sắp theo ô và khử trùng, nên mỗi ô vào
        // đúng một lần dù chuỗi nổ có nhiều bom.
        let blasted = scratch.take_blasts();

        // **Bước 1 — lửa đốt item cũ** (BnB `game.js:415–417`): item nằm trên nền ở ô bị vụ
        // nổ phủ bị đốt. Làm trước khi rơi item mới để item vừa rơi ở ô vừa vỡ không bị đốt bởi
        // chính vụ nổ đó (BnB đốt hết rồi mới rơi).
        for cell in &blasted {
            let Some(index) = state.items.iter().position(|it| it.cell == *cell) else {
                continue;
            };
            let item = state.items.remove(index);
            events.emit(EventKind::ItemDestroyed {
                item: item.id,
                cell: *cell,
            });
        }

        // **Bước 2 — khối mềm vỡ rồi mới rơi item** (BnB `game.js:424–430`): ô bị vụ nổ phủ mà
        // là khối mềm thì vỡ thành nền (`block_destroyed`); xác suất có item lấy từ profile.
        for cell in &blasted {
            if state.field.kind(*cell) != Some(CellKind::Breakable) {
                continue;
            }
            state.field.set_kind(*cell, CellKind::Floor);
            events.emit(EventKind::BlockDestroyed { cell: *cell });
            // Mỗi khối vỡ rút đúng một số phần nghìn, kể cả khi không rơi item, để dòng RNG
            // chỉ phụ thuộc số khối vỡ chứ không phụ thuộc kết quả rút.
            if state.rng.below(PERMILLE) >= rules.brick_item_chance_permille {
                continue;
            }
            let Some(kind) = pick_by_weight(&mut state.rng, &rules.items, total_weight) else {
                // Không có loại item nào có trọng số: khối vỡ không rơi được gì.
                continue;
            };
            let id = state.ids.next_id();
            state.items.push(FloorItem {
                id,
                kind,
                cell: *cell,
            });
            events.emit(EventKind::ItemRevealed {
                item: id,
                kind,
                cell: *cell,
            });
        }
        // Item mới có id lớn hơn mọi item đang có, nên danh sách vẫn sắp theo id (bất biến
        // contract) sau khi thêm; sắp lại một lần cho chắc.
        state.items.sort_by_key(|i| i.id);
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
    use crate::num::Cell;
    use crate::rules::Rules;
    use crate::sim::PlayerSetup;
    use crate::state::{CharacterStats, EntityId, Stat};
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

    /// Item nằm trên nền ở ô `cell` (dựng sẵn như fixture).
    fn floor_item(state: &mut SimulationState, cell: Cell, kind: ItemKind) -> EntityId {
        let id = state.ids.next_id();
        state.items.push(FloorItem { id, kind, cell });
        id
    }

    /// Số phần nghìn mà RNG rút ra ở lần đầu: dùng để chọn seed có xác suất trúng/trượt.
    fn first_permille(seed: u64) -> u32 {
        let mut rng = Rng::new(seed);
        rng.below(PERMILLE)
    }

    /// Seed mà lần rút đầu **trượt** (không rơi item).
    fn missing_seed() -> u64 {
        (1u64..)
            .find(|s| first_permille(*s) >= rules().brick_item_chance_permille)
            .expect("có seed trượt")
    }

    /// Seed mà lần rút đầu **trúng** (rơi item).
    fn hit_seed() -> u64 {
        (1u64..)
            .find(|s| first_permille(*s) < rules().brick_item_chance_permille)
            .expect("có seed trúng")
    }

    /// Loại item mà seed `s` rơi ra: đọc dòng RNG đúng thứ tự luật định (xác suất trước,
    /// rồi mới tới loại). Test này ghim **thứ tự rút**, không ghim thuật toán chọn theo trọng
    /// số — thuật toán đó có test riêng trên mẫu lớn.
    fn rolled_kind(s: u64) -> ItemKind {
        let mut rng = Rng::new(s);
        let _chance = rng.below(PERMILLE);
        let items = rules().items.clone();
        let total: u32 = items.iter().map(|i| i.weight).sum();
        pick_by_weight(&mut rng, &items, total).expect("luôn chọn được loại")
    }

    #[test]
    fn broken_block_drops_one_item() {
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        // RNG của lượt do test kiểm soát: lần rút đầu trúng nên khối vỡ rơi item.
        state.rng = Rng::new(hit_seed());
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
                        kind: rolled_kind(hit_seed()),
                        cell: Cell::new(2, 1),
                    },
                },
            ]
        );
        assert_eq!(state.field.kind(Cell::new(2, 1)), Some(CellKind::Floor));
        assert_eq!(
            state.items,
            vec![FloorItem {
                id: EntityId(1),
                kind: rolled_kind(hit_seed()),
                cell: Cell::new(2, 1),
            }]
        );
    }

    #[test]
    fn broken_block_may_drop_nothing() {
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        state.rng = Rng::new(missing_seed());
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert_eq!(events.len(), 1, "chỉ có `block_destroyed`");
        assert_eq!(
            events[0].kind,
            EventKind::BlockDestroyed {
                cell: Cell::new(2, 1)
            }
        );
        assert!(state.items.is_empty(), "lần rút trượt thì không rơi item");
        assert_eq!(state.field.kind(Cell::new(2, 1)), Some(CellKind::Floor));
    }

    #[test]
    fn same_seed_gives_same_drops_and_other_seed_differs() {
        let rows = soft_map(8, 6);
        let drops = |seed: u64| -> Vec<(Cell, ItemKind)> {
            let w = world(&rows);
            let mut state = new_round(&rows, seed);
            let mut scratch = TickScratch::new();
            // Vụ nổ phủ toàn bộ khối mềm: mọi khối vỡ trong một tick.
            let cells: Vec<Cell> = state
                .field
                .cells()
                .filter(|c| state.field.kind(*c) == Some(CellKind::Breakable))
                .collect();
            scratch.merge_blasts(cells);
            run_blocks(&w, &mut state, &mut scratch);
            state.items.iter().map(|i| (i.cell, i.kind)).collect()
        };
        assert_eq!(drops(7), drops(7), "cùng seed cho cùng item rơi");
        let differs = (1..40).any(|seed| drops(seed) != drops(7));
        assert!(differs, "đổi seed phải đổi được bố cục item rơi");
    }

    #[test]
    fn drop_kind_follows_item_weights() {
        // Mẫu lớn: tỉ lệ loại item rơi phải bám trọng số trong `Rules::items`.
        let rows = soft_map(40, 40);
        let mut counts = [0usize; 3];
        let mut total = 0usize;
        for seed in 0..40u64 {
            let w = world(&rows);
            let mut state = new_round(&rows, seed);
            let mut scratch = TickScratch::new();
            let cells: Vec<Cell> = state
                .field
                .cells()
                .filter(|c| state.field.kind(*c) == Some(CellKind::Breakable))
                .collect();
            scratch.merge_blasts(cells);
            run_blocks(&w, &mut state, &mut scratch);
            for item in &state.items {
                counts[usize::from(item.kind.0)] += 1;
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
    fn drop_chance_matches_the_profile() {
        // Xác suất gạch có item lấy từ profile (`drop.brick_item_chance`).
        let rows = soft_map(40, 40);
        let mut dropped = 0usize;
        let mut broken = 0usize;
        for seed in 0..20u64 {
            let w = world(&rows);
            let mut state = new_round(&rows, seed);
            let mut scratch = TickScratch::new();
            let cells: Vec<Cell> = state
                .field
                .cells()
                .filter(|c| state.field.kind(*c) == Some(CellKind::Breakable))
                .collect();
            broken += cells.len();
            scratch.merge_blasts(cells);
            run_blocks(&w, &mut state, &mut scratch);
            dropped += state.items.len();
        }
        let chance = rules().brick_item_chance_permille;
        let got = (dropped as u32 * 1000) / broken as u32;
        let expected = chance * 1000 / PERMILLE;
        assert!(
            got.abs_diff(expected) <= 20,
            "tỉ lệ rơi {got}‰ khác {expected}‰ quá 20‰"
        );
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
    fn blast_breaks_block_once_and_drops_one_item() {
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        state.rng = Rng::new(hit_seed());
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::BlockDestroyed { .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::ItemRevealed { .. }))
        );
        assert_eq!(state.items.len(), 1);

        // Khối đã vỡ: không phá thêm lần nữa, không rơi thêm item.
        let later = run_blocks(&w, &mut state, &mut scratch);
        assert!(later.is_empty(), "khối mềm chỉ vỡ một lần");
        assert_eq!(state.items.len(), 1);
    }

    #[test]
    fn blast_burns_the_item_lying_on_the_floor() {
        // R-109: lửa đốt item nằm trên nền ở ô bị vụ nổ phủ.
        let rows = ["#####", "#0..#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        let id = floor_item(&mut state, Cell::new(2, 1), ItemKind(1));
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert!(
            events
                .iter()
                .any(|e| matches!(&e.kind, EventKind::ItemDestroyed { item, .. } if *item == id)),
            "lửa phải đốt item trên nền"
        );
        assert!(state.items.is_empty(), "item bị đốt thì rời nền");
    }

    #[test]
    fn new_drop_survives_the_blast_that_made_it() {
        // Lửa đốt item cũ trước, item mới sinh sau: item vừa rơi ở ô vừa vỡ còn sống.
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        state.rng = Rng::new(hit_seed());
        // Item cũ nằm ở ô nền (1,1), khối mềm ở ô (2,1): cùng một vụ nổ phủ.
        let old = floor_item(&mut state, Cell::new(1, 1), ItemKind(1));
        let mut scratch = TickScratch::new();
        scratch.merge_blasts([Cell::new(1, 1), Cell::new(2, 1)]);
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert!(
            events
                .iter()
                .any(|e| matches!(&e.kind, EventKind::ItemDestroyed { item, .. } if *item == old)),
            "item cũ bị đốt"
        );
        assert_eq!(state.items.len(), 1, "chỉ còn item mới rơi ở ô vừa vỡ");
        assert_eq!(state.items[0].cell, Cell::new(2, 1));
    }

    #[test]
    fn blast_channel_is_deduplicated_so_the_drop_rolls_once() {
        // Cùng một ô vào kênh blast nhiều lần (nhiều bom trong chuỗi nổ phủ): khối vỡ một
        // lần, rơi **một** item — không roll lặp.
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);
        let mut state = new_round(&rows, 1);
        state.rng = Rng::new(hit_seed());
        let mut scratch = TickScratch::new();
        scratch.merge_blasts([Cell::new(2, 1), Cell::new(2, 1), Cell::new(2, 1)]);
        let events = run_blocks(&w, &mut state, &mut scratch);

        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::BlockDestroyed { .. }))
                .count(),
            1,
            "khối mềm chỉ vỡ một lần"
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::ItemRevealed { .. }))
                .count(),
            1,
            "chỉ rơi một item, không roll lặp"
        );
        assert_eq!(state.items.len(), 1);
    }

    #[test]
    fn fixture_blast_channel_gives_the_same_result_as_a_real_explosion() {
        // Đường fixture (kênh blast dựng sẵn, dùng để test riêng `blocks`) phải cho cùng kết
        // quả với đường gameplay (vụ nổ thật của `explosion`): cùng ô bị phủ ⇒ cùng khối vỡ,
        // cùng item rơi ra.
        let rows = ["#####", "#0+.#", "#####"];
        let w = world(&rows);

        // (a) đường fixture: dựng sẵn kênh blast.
        let mut fixture = new_round(&rows, 1);
        fixture.rng = Rng::new(hit_seed());
        let mut scratch = TickScratch::new();
        scratch.merge_blasts(blast(Cell::new(2, 1)));
        run_blocks(&w, &mut fixture, &mut scratch);

        // (b) đường gameplay: bom nổ thật ở tick 0, tầm 1 ⇒ vụ nổ phủ ô (1,1) và (2,1).
        let mut gameplay = new_round(&rows, 1);
        gameplay.rng = Rng::new(hit_seed());
        let bomb_id = gameplay.ids.next_id();
        gameplay.bombs.push(crate::state::Bomb {
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
            "cả hai đường phải rơi cùng item ở cùng ô"
        );
    }
}
