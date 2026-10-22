//! Nhặt item Bomb/Fire/Roller — V1-017…V1-019.
//!
//! Luật V1 (design, RULES_V1 R-056…R-059, R-109):
//! 1. Người còn sống (`Life::Alive`) có tâm trong ô có item trên nền thì nhặt. Người `Trapped`
//!    hoặc `Dead` không nhặt (R-059): người bị nhốt đứng yên trong V1 (R-036).
//! 2. Người chơi được duyệt theo id tăng dần (`SimulationState::players` đã sắp), nên hai người
//!    cùng đứng trong một ô trong cùng tick chỉ người id nhỏ nhận: item bị xóa khỏi nền ngay khi
//!    được nhặt (R-056).
//! 3. Hiệu ứng theo `ItemDef.effect` và `ItemDef.amount` của profile: Bomb → số bom đặt được,
//!    Fire → tầm nổ, Roller → cấp tốc độ; giá trị bị kẹp theo trần của chỉ số (R-057). Bom đã
//!    đặt giữ tầm chụp lúc đặt (R-032), nên nhặt Fire không đổi bom đã đặt.
//! 4. **Đã chạm trần thì item vẫn mất** (R-109; BnB `server/game.js:569–589`): BnB gỡ item
//!    khỏi nền **trước** khi áp hiệu ứng, nên ở trần hiệu ứng không tăng nhưng item đã mất.
//!    V1 giữ đúng thứ tự đó: item rời nền rồi mới áp hiệu ứng, kẹp theo trần chỉ số. Event
//!    `item_picked` phát khi item rời nền (kể cả lúc ở trần) vì item **đã** bị nhặt.
//!
//! Item giấu trong khối mềm không nằm trên nền nên không nhặt được; item rơi khi gạch vỡ
//! (V1-016) nằm trên nền nên được nhặt ở bước sau.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/pickup/`. Luật và nguồn: RULES_V1; giá trị lấy từ `Rules`.

use crate::event::{EventKind, EventSink};
use crate::rules::{ItemDef, ItemEffect};
use crate::scratch::TickScratch;
use crate::sim::{PlayerInput, World};
use crate::state::{Life, Player, SimulationState, Stat};
use crate::system::System;

#[derive(Debug)]
pub struct Pickup;

pub static PICKUP: Pickup = Pickup;

impl System for Pickup {
    fn name(&self) -> &'static str {
        "pickup"
    }

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        _inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
        _scratch: &mut TickScratch,
    ) {
        // Duyệt theo thứ tự id của người chơi: hai người cùng ô trong một tick thì người id nhỏ
        // nhặt trước, item biến mất khỏi nền nên người sau không còn gì để nhặt (R-056).
        for i in 0..state.players.len() {
            if state.players[i].life != Life::Alive {
                continue;
            }
            let cell = world.cell_at(state.players[i].x, state.players[i].y);
            let Some(index) = state.items.iter().position(|it| it.cell == cell) else {
                continue;
            };
            let item = state.items[index].clone();
            let Some(def) = world.rules().items.get(usize::from(item.kind.0)) else {
                continue;
            };
            let player_id = state.players[i].id;
            // BnB `server/game.js:577–581`: item được gỡ khỏi nền **trước** khi áp hiệu ứng,
            // nên ở trần hiệu ứng không tăng nhưng item vẫn mất (R-109).
            state.items.remove(index);
            apply_effect(&mut state.players[i], def);
            events.emit(EventKind::ItemPicked {
                item: item.id,
                kind: item.kind,
                player: player_id,
            });
        }
    }
}

/// Áp hiệu ứng item lên chỉ số tương ứng, kẹp theo trần của chỉ số.
///
/// Ở trần thì chỉ số không đổi (`Stat::add` kẹp theo trần) — item vẫn mất khỏi nền vì đã bị
/// nhặt (R-109).
fn apply_effect(player: &mut Player, def: &ItemDef) {
    let stat: &mut Stat = match def.effect {
        ItemEffect::BombCapacity => &mut player.bombs,
        ItemEffect::BlastRange => &mut player.range,
        ItemEffect::SpeedLevel => &mut player.speed,
    };
    stat.add(def.amount);
}

#[cfg(test)]
mod tests {
    use boom_content::map::Terrain;
    use boom_content::rules::RulesProfile;

    use crate::mode::mode_by_id;
    use crate::num::Cell;
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

    fn setup(id: u8, bombs: Stat, range: Stat, speed: Stat) -> PlayerSetup {
        PlayerSetup {
            id,
            stats: CharacterStats {
                speed,
                bombs,
                range,
            },
        }
    }

    fn stat(value: u8, max: u8) -> Stat {
        Stat::new(value, max)
    }

    /// Đặt người chơi vào ô và thêm item loại `kind` (theo tên trong `Rules::items`).
    fn place_item(state: &mut SimulationState, world: &World, cell: Cell, kind: &str) -> u32 {
        let index = world
            .rules()
            .items
            .iter()
            .position(|i| i.key == kind)
            .expect("item trong rules");
        let id = state.ids.next_id();
        state.items.push(crate::state::FloorItem {
            id,
            kind: crate::state::ItemKind(u16::try_from(index).unwrap()),
            cell,
        });
        id.0
    }

    fn put_player(state: &mut SimulationState, world: &World, index: usize, cell: Cell) {
        let (x, y) = world.center_of(cell);
        state.players[index].x = x;
        state.players[index].y = y;
    }

    #[test]
    fn picks_item_raises_stat_and_removes_item() {
        let w = world(&["0.."]);
        let mut s =
            SimulationState::new_round(&w, &[setup(0, stat(1, 10), stat(1, 7), stat(5, 7))], 1)
                .unwrap();
        let cell = Cell::new(1, 0);
        put_player(&mut s, &w, 0, cell);
        let item_id = place_item(&mut s, &w, cell, "Bomb");
        let mut events = Vec::new();
        s.step(&w, &[PlayerInput::default()], &mut events).unwrap();
        assert_eq!(s.players[0].bombs.value, 2);
        assert!(s.items.is_empty(), "item phải mất khỏi nền");
        let picked: Vec<_> = events
            .iter()
            .filter(|e| e.kind.name() == "item_picked")
            .collect();
        assert_eq!(picked.len(), 1);
        assert_eq!(
            picked[0].kind.fields(),
            vec![("item", i64::from(item_id)), ("kind", 0), ("player", 0),]
        );
        s.check_invariants(&w).unwrap();
    }

    #[test]
    fn two_players_same_cell_only_lower_id_takes_item() {
        let w = world(&["01."]);
        let mut s = SimulationState::new_round(
            &w,
            &[
                setup(0, stat(1, 10), stat(1, 7), stat(5, 7)),
                setup(1, stat(1, 10), stat(1, 7), stat(5, 7)),
            ],
            1,
        )
        .unwrap();
        let cell = Cell::new(1, 0);
        put_player(&mut s, &w, 0, cell);
        put_player(&mut s, &w, 1, cell);
        place_item(&mut s, &w, cell, "Bomb");
        let mut events = Vec::new();
        s.step(
            &w,
            &[PlayerInput::default(), PlayerInput::default()],
            &mut events,
        )
        .unwrap();
        assert_eq!(s.players[0].bombs.value, 2, "người id nhỏ nhận");
        assert_eq!(s.players[1].bombs.value, 1, "người id lớn không nhận");
        assert!(s.items.is_empty());
        s.check_invariants(&w).unwrap();
    }

    #[test]
    fn at_cap_item_is_lost_and_event_is_emitted() {
        // R-109 (BnB `server/game.js:577–581`): item gỡ khỏi nền **trước** khi áp hiệu ứng,
        // nên ở trần chỉ số không tăng nhưng item vẫn mất.
        let w = world(&["0.."]);
        let mut s =
            SimulationState::new_round(&w, &[setup(0, stat(3, 3), stat(1, 7), stat(5, 7))], 1)
                .unwrap();
        let cell = Cell::new(1, 0);
        put_player(&mut s, &w, 0, cell);
        let item_id = place_item(&mut s, &w, cell, "Bomb");
        let mut events = Vec::new();
        s.step(&w, &[PlayerInput::default()], &mut events).unwrap();
        assert_eq!(s.players[0].bombs.value, 3, "đã ở trần thì không tăng");
        assert!(s.items.is_empty(), "item ở cap vẫn mất khỏi nền");
        let picked: Vec<_> = events
            .iter()
            .filter(|e| e.kind.name() == "item_picked")
            .collect();
        assert_eq!(picked.len(), 1, "item đã rời nền thì phát event");
        assert_eq!(
            picked[0].kind.fields(),
            vec![("item", i64::from(item_id)), ("kind", 0), ("player", 0),]
        );
        s.check_invariants(&w).unwrap();
    }

    #[test]
    fn item_at_cap_does_not_stack() {
        // Chỉ số không vượt trần khi nhặt ở cap.
        let w = world(&["0.."]);
        let mut s =
            SimulationState::new_round(&w, &[setup(0, stat(3, 3), stat(1, 7), stat(5, 7))], 1)
                .unwrap();
        let cell = Cell::new(1, 0);
        put_player(&mut s, &w, 0, cell);
        place_item(&mut s, &w, cell, "Bomb");
        let mut events = Vec::new();
        s.step(&w, &[PlayerInput::default()], &mut events).unwrap();
        assert_eq!(s.players[0].bombs.value, 3);
        assert_eq!(s.players[0].bombs.max, 3);
        s.check_invariants(&w).unwrap();
    }

    #[test]
    fn trapped_player_does_not_pick_item() {
        let w = world(&["0.."]);
        let mut s =
            SimulationState::new_round(&w, &[setup(0, stat(1, 10), stat(1, 7), stat(5, 7))], 1)
                .unwrap();
        let cell = Cell::new(1, 0);
        put_player(&mut s, &w, 0, cell);
        place_item(&mut s, &w, cell, "Bomb");
        s.players[0].life = crate::state::Life::Trapped { since: 0 };
        let mut events = Vec::new();
        s.step(&w, &[PlayerInput::default()], &mut events).unwrap();
        assert_eq!(s.players[0].bombs.value, 1, "người bị nhốt không nhặt");
        assert_eq!(s.items.len(), 1, "item phải còn trên nền");
        assert!(!events.iter().any(|e| e.kind.name() == "item_picked"));
        s.check_invariants(&w).unwrap();
    }

    #[test]
    fn roller_raises_speed_level() {
        let w = world(&["0.."]);
        let mut s =
            SimulationState::new_round(&w, &[setup(0, stat(1, 10), stat(1, 7), stat(5, 7))], 1)
                .unwrap();
        let cell = Cell::new(1, 0);
        put_player(&mut s, &w, 0, cell);
        place_item(&mut s, &w, cell, "Roller");
        let mut events = Vec::new();
        s.step(&w, &[PlayerInput::default()], &mut events).unwrap();
        assert_eq!(s.players[0].speed.value, 6);
        s.check_invariants(&w).unwrap();
    }
}
