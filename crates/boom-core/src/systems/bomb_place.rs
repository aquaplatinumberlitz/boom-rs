//! Đặt bom và hạn mức — V1-013.
//!
//! Khi input có cạnh nhấn `place_bomb` và người chơi còn sống (`Life::Alive`), đặt một bom ở ô tâm
//! nhân vật nếu ô đó chưa có bom và số bom người đó đang có nhỏ hơn `bombs.value` (R-025). Bom chụp
//! `range.value` lúc đặt (R-032) và nổ sau `Rules::fuse_ticks`. `grace` (pass) cấp cho **mọi**
//! người chơi còn sống mà hộp va chạm chồng ô bom lúc đặt, để họ đi ra được; người không có
//! `grace` thì bom chặn ô (R-107). Người bị nhốt không đặt được (R-029).
//!
//! Người chơi được duyệt theo thứ tự id (`SimulationState::players` đã sắp), nên hai người cùng ô
//! cùng nhấn trong một tick thì người id nhỏ đặt trước và bom thứ hai bị chặn bởi ô đã có bom.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/bomb_place/`. Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`/chỉ số.

use crate::event::{EventKind, EventSink};
use crate::num::Cell;
use crate::scratch::TickScratch;
use crate::sim::{PlayerInput, World};
use crate::state::{Bomb, Life, SimulationState};
use crate::system::System;

use super::movement::box_overlaps_cell;

#[derive(Debug)]
pub struct BombPlace;

pub static BOMB_PLACE: BombPlace = BombPlace;

/// Bom đã quyết định đặt trong tick này, chờ cấp ID và ghi state theo đúng thứ tự duyệt.
struct PendingBomb {
    owner: u8,
    cell: Cell,
    range: u8,
    grace: Vec<u8>,
}

impl System for BombPlace {
    fn name(&self) -> &'static str {
        "bomb_place"
    }

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
        _scratch: &mut TickScratch,
    ) {
        // Quyết định trước rồi mới ghi state: bom vừa đặt trong tick này cũng phải chặn người sau
        // ở cùng ô và tính vào hạn mức của chủ. Thứ tự duyệt là thứ tự id của người chơi.
        let mut queue: Vec<PendingBomb> = Vec::new();
        for (player, input) in state.players.iter().zip(inputs) {
            if !input.place_bomb || player.life != Life::Alive {
                continue;
            }
            let cell = world.cell_at(player.x, player.y);
            if state.bombs.iter().any(|b| b.cell == cell) || queue.iter().any(|b| b.cell == cell) {
                continue;
            }
            let owned = state.bombs.iter().filter(|b| b.owner == player.id).count()
                + queue.iter().filter(|b| b.owner == player.id).count();
            if owned >= usize::from(player.bombs.value) {
                continue;
            }
            let grace = state
                .players
                .iter()
                .filter(|p| p.life != Life::Dead && box_overlaps_cell(world, p.x, p.y, cell))
                .map(|p| p.id)
                .collect();
            queue.push(PendingBomb {
                owner: player.id,
                cell,
                range: player.range.value,
                grace,
            });
        }
        if queue.is_empty() {
            return;
        }

        let explodes_at = state
            .tick
            .checked_add(world.rules().fuse_ticks)
            .expect("tick nổ vượt miền Tick");
        for pending in queue {
            let id = state.ids.next_id();
            state.bombs.push(Bomb {
                id,
                owner: pending.owner,
                cell: pending.cell,
                range: pending.range,
                explodes_at,
                grace: pending.grace,
            });
            events.emit(EventKind::BombPlaced {
                bomb: id,
                player: pending.owner,
                cell: pending.cell,
                range: pending.range,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use boom_content::map::Terrain;
    use boom_content::rules::RulesProfile;

    use crate::mode::mode_by_id;
    use crate::rules::Rules;
    use crate::sim::{PlayerInput, PlayerSetup, World};
    use crate::state::{CharacterStats, SimulationState, Stat};

    const PROFILE: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

    fn rules() -> Rules {
        Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap()
    }

    fn world(rows: &[&str]) -> World {
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(rows).unwrap(), rules(), ffa).unwrap()
    }

    fn setup(id: u8, range: u8) -> PlayerSetup {
        PlayerSetup {
            id,
            stats: CharacterStats {
                speed: Stat::new(5, 7),
                bombs: Stat::new(2, 10),
                range: Stat::new(range, 7),
            },
        }
    }

    fn press() -> PlayerInput {
        PlayerInput {
            place_bomb: true,
            ..PlayerInput::default()
        }
    }

    #[test]
    fn bomb_snapshots_range_and_fuse_and_graces_everyone_on_the_cell() {
        let w = world(&["0.."]);
        let mut s = SimulationState::new_round(&w, &[setup(0, 2)], 1).unwrap();
        let mut events = Vec::new();
        s.step(&w, &[press()], &mut events).unwrap();
        assert_eq!(s.bombs.len(), 1);
        let b = &s.bombs[0];
        assert_eq!(b.owner, 0);
        assert_eq!(b.range, 2);
        assert_eq!(b.explodes_at, w.rules().fuse_ticks);
        assert_eq!(b.grace, vec![0]);
        s.check_invariants(&w).unwrap();
    }

    #[test]
    fn same_cell_same_tick_lower_id_places_and_both_get_grace() {
        let w = world(&["01."]);
        let mut s = SimulationState::new_round(&w, &[setup(0, 2), setup(1, 3)], 1).unwrap();
        let (x, y) = w.center_of(crate::num::Cell::new(0, 0));
        s.players[1].x = x;
        s.players[1].y = y;
        let mut events = Vec::new();
        s.step(&w, &[press(), press()], &mut events).unwrap();
        assert_eq!(s.bombs.len(), 1, "bom thứ hai cùng ô phải bị chặn");
        assert_eq!(
            s.bombs[0].owner, 0,
            "duyệt theo id, không theo thứ tự input"
        );
        assert_eq!(s.bombs[0].range, 2);
        assert_eq!(s.bombs[0].grace, vec![0, 1]);
        s.check_invariants(&w).unwrap();
    }

    #[test]
    fn grace_covers_every_player_whose_box_overlaps_the_bomb_cell() {
        // R-107: `grace` theo hộp va chạm, không theo ô của tâm. Người 1 đứng ở x = 50 px
        // (hộp 26×26 chồm cả ô (0,0) lẫn ô (1,0)) nên cũng được pass để đi ra khỏi bom.
        let w = world(&["01."]);
        let mut s = SimulationState::new_round(&w, &[setup(0, 2), setup(1, 3)], 1).unwrap();
        s.players[1].x = crate::num::Mpx(50_000);
        let mut events = Vec::new();
        s.step(&w, &[press(), PlayerInput::default()], &mut events)
            .unwrap();
        assert_eq!(s.bombs.len(), 1);
        assert_eq!(s.bombs[0].cell, crate::num::Cell::new(0, 0));
        assert_eq!(
            s.bombs[0].grace,
            vec![0, 1],
            "mọi người có hộp chồng ô bom đều có pass"
        );
        s.check_invariants(&w).unwrap();
    }
}
