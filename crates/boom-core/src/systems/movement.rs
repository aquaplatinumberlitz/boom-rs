//! Di chuyển và va chạm ô — V1-011, V1-012.
//!
//! Luật V1 chọn theo BnB (ADR 0012, claim R-107): người chơi là **hộp 26×26 px**
//! (`player.collision_half_px`), không còn là điểm tâm phải căn về tâm ô. Một ô chặn người chơi
//! khi hộp chồm ô đó; hộp chồm ô bom thì người đó phải có `grace` (pass) mới đi qua.
//!
//! - Di chuyển theo đúng một trục mỗi tick theo hướng đang giữ (V1-011). Bị chặn thì dừng ở mép hộp.
//! - Bám góc (BnB `server/game.js:241–272`): khi bị chặn theo trục ngang thì kéo người chơi về
//!   tâm hàng đang đứng rồi thử lại, chỉ khi lệch không quá `player.corner_assist_px`.
//! - `grace` cấp cho **mọi** người có hộp chồng ô bom lúc đặt, và bị xóa khi hộp rời hẳn ô
//!   (R-107; BnB `server/game.js:150–153,392–399`). Không ai thêm lại sau khi mất.
//! - Người bị nhốt đi được theo `player.trapped_can_move` (KR R-127/R-132); người chết
//!   không đi. Tốc độ riêng khi bị nhốt chưa biết: V1 dùng bảng tốc độ hiện hành (R-133 design).
//!
//! Thứ tự xóa `grace`: BnB xóa ở đầu tick (`updateBombPass`) trước khi người chơi di chuyển,
//! V1 xóa ở cuối bước movement. Hai cách cho cùng kết quả: người chơi rời hẳn ô bom trong
//! tick N thì từ tick N+1 bom đã chặn người đó ở cả hai cách.
//!
//! Mọi giá trị luật lấy từ `Rules`; file này không có hằng số luật. Kịch bản nghiệm thu:
//! `fixtures/scenarios/movement/`. Luật và nguồn: RULES_V1.

use boom_content::map::CellKind;

use crate::event::{EventKind, EventSink};
use crate::num::{Cell, Mpx};
use crate::scratch::TickScratch;
use crate::sim::{Dir, PlayerInput, World};
use crate::state::{Bomb, Field, Life, Player, SimulationState};
use crate::system::System;

#[derive(Debug)]
pub struct Movement;

pub static MOVEMENT: Movement = Movement;

impl System for Movement {
    fn name(&self) -> &'static str {
        "movement"
    }

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
        _scratch: &mut TickScratch,
    ) {
        let SimulationState {
            players,
            bombs,
            field,
            ..
        } = state;
        for (player, input) in players.iter_mut().zip(inputs) {
            let can_move = match player.life {
                Life::Alive => true,
                Life::Trapped { .. } => world.rules().trapped_can_move,
                Life::Dead => false,
            };
            if !can_move || input.dir == Dir::None {
                continue;
            }
            if player.facing != input.dir {
                player.facing = input.dir;
                events.emit(EventKind::PlayerTurned { player: player.id });
            }
            move_player(world, player, input.dir, bombs, field);
        }
        clear_finished_grace(world, players, bombs);
    }
}

/// `grace` (pass) chỉ còn khi hộp người chơi còn chồng ô bom; rời hẳn ô thì mất quyền quay lại
/// (R-107). Người đã chết thì mất luôn. Danh sách giữ nguyên thứ tự id tăng dần.
fn clear_finished_grace(world: &World, players: &[Player], bombs: &mut [Bomb]) {
    for bomb in bombs {
        bomb.grace.retain(|&id| {
            players.iter().any(|p| {
                p.id == id && p.life != Life::Dead && box_overlaps_cell(world, p.x, p.y, bomb.cell)
            })
        });
    }
}

/// Hộp tại tâm `(x, y)` có chồng ô `(cell.x, cell.y)` không (BnB `overlapsTile`).
///
/// Dùng chung cho `grace` của bom: người chơi được pass khi hộp còn chồng ô bom.
pub(super) fn box_overlaps_cell(world: &World, x: Mpx, y: Mpx, cell: Cell) -> bool {
    let half = world.rules().collision_half.0;
    let tile = world.rules().tile.0;
    let left = cell.x * tile;
    let top = cell.y * tile;
    x.0 + half > left && x.0 - half < left + tile && y.0 + half > top && y.0 - half < top + tile
}

/// Ô mà hộp tại tâm `pos` chạm tới trên một trục (chỉ số ô đầu và ô cuối).
///
/// Mép hộp vừa chạm ranh giới ô thì ô kề chưa bị tính, đúng như BnB dùng `x + HALF - 0.01`.
fn span(world: &World, pos: Mpx) -> (i32, i32) {
    let tile = world.rules().tile.0;
    let half = world.rules().collision_half.0;
    let lo = (pos.0 - half).div_euclid(tile);
    let hi = (pos.0 + half - 1).div_euclid(tile);
    (lo, hi)
}

/// Ô `cell` chặn người chơi `player` không: ngoài lưới, khối cứng/mềm, hoặc bom mà người đó
/// không có `grace` (R-107).
fn solid_for(field: &Field, bombs: &[Bomb], player: u8, cell: Cell) -> bool {
    let blocked_by_field = match field.kind(cell) {
        None => true,
        Some(CellKind::Floor) => false,
        Some(CellKind::Solid | CellKind::Breakable | CellKind::Unknown) => true,
    };
    if blocked_by_field {
        return true;
    }
    bombs
        .iter()
        .any(|b| b.cell == cell && !b.grace.contains(&player))
}

/// Hộp tại tâm `(x, y)` chạm ô chặn nào không.
fn collides(world: &World, field: &Field, bombs: &[Bomb], player: u8, x: Mpx, y: Mpx) -> bool {
    let (x0, x1) = span(world, x);
    let (y0, y1) = span(world, y);
    for cy in y0..=y1 {
        for cx in x0..=x1 {
            if solid_for(field, bombs, player, Cell::new(cx, cy)) {
                return true;
            }
        }
    }
    false
}

/// Một bước di chuyển theo hướng `dir`: thử bước trước, bị chặn thì mới bám góc.
fn move_player(world: &World, p: &mut Player, dir: Dir, bombs: &[Bomb], field: &Field) {
    let (dx, dy) = dir.delta();
    let step = world.speed(p.speed.value);
    let assist = world.rules().corner_assist.0;
    let tile = world.rules().tile.0;
    if dx != 0 {
        let target = Mpx(p.x.0 + dx * step.0);
        if !collides(world, field, bombs, p.id, target, p.y) {
            p.x = target;
            return;
        }
        // Bám góc: kéo về tâm hàng đang đứng (rồi tới tâm hàng trên/dưới kèm theo), chỉ khi lệch
        // trong phạm vi assist và vị trí sau khi kéo không chạm ô chặn.
        let center = world.center(world.cell_index(p.y)).0;
        for candidate in [center, center - tile, center + tile] {
            let delta = candidate - p.y.0;
            if delta == 0 || delta.abs() > assist {
                continue;
            }
            let shift = if delta.abs() <= step.0 {
                delta
            } else {
                delta.signum() * step.0
            };
            let shifted = Mpx(p.y.0 + shift);
            if collides(world, field, bombs, p.id, p.x, shifted) {
                continue;
            }
            p.y = shifted;
            if !collides(world, field, bombs, p.id, target, shifted) {
                p.x = target;
            }
            return;
        }
    } else {
        let target = Mpx(p.y.0 + dy * step.0);
        if !collides(world, field, bombs, p.id, p.x, target) {
            p.y = target;
            return;
        }
        // Bám góc theo trục dọc: kéo về tâm cột đang đứng.
        let center = world.center(world.cell_index(p.x)).0;
        for candidate in [center, center - tile, center + tile] {
            let delta = candidate - p.x.0;
            if delta == 0 || delta.abs() > assist {
                continue;
            }
            let shift = if delta.abs() <= step.0 {
                delta
            } else {
                delta.signum() * step.0
            };
            let shifted = Mpx(p.x.0 + shift);
            if collides(world, field, bombs, p.id, shifted, p.y) {
                continue;
            }
            p.x = shifted;
            if !collides(world, field, bombs, p.id, shifted, target) {
                p.y = target;
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use boom_content::map::Terrain;
    use boom_content::rules::RulesProfile;

    use crate::mode::mode_by_id;
    use crate::num::{Cell, Mpx};
    use crate::rules::Rules;
    use crate::sim::{Dir, PlayerInput, PlayerSetup, World};
    use crate::state::{Bomb, CharacterStats, EntityId, Life, SimulationState, Stat};

    const PROFILE: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

    fn rules() -> Rules {
        Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap()
    }

    fn world(rows: &[&str]) -> World {
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(rows).unwrap(), rules(), ffa).unwrap()
    }

    fn setup(id: u8) -> PlayerSetup {
        PlayerSetup {
            id,
            stats: CharacterStats {
                speed: Stat::new(5, 7),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        }
    }

    fn walk(dir: Dir) -> PlayerInput {
        PlayerInput {
            dir,
            place_bomb: false,
        }
    }

    /// Chạy `ticks` tick với hướng `dir`.
    fn run(w: &World, s: &mut SimulationState, dir: Dir, ticks: u32) {
        for _ in 0..ticks {
            s.step(w, &[walk(dir)], &mut Vec::new()).unwrap();
        }
    }

    #[test]
    fn hitbox_stops_at_the_edge_of_the_wall() {
        // Tường cột 4 bắt đầu ở x = 160 px; hộp nửa 13 px nên người chơi dừng ở x = 146 px
        // (mép hộp chạm đúng mép ô), không phải ở tâm ô (140 px) như mô hình điểm tâm.
        let w = world(&["#####", "#0..#", "#####"]);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        run(&w, &mut s, Dir::Right, 100);
        assert_eq!(s.players[0].x, Mpx(146_000));
    }

    #[test]
    fn corner_assist_applies_at_twelve_px_and_stops_at_thirteen() {
        // Khối cứng ở (2,2) chặn bước sang phải khi hộp chồm hàng 2. Người chơi đứng ở mép
        // hộp chồm cột 2 (x = 66 px) nên bước sang phải bị chặn.
        let rows = ["#####", "#0..#", "#.##.", "#####"];
        // Lệch 12 px so với tâm hàng 1 (60 px): bám góc được, kéo về tâm hàng theo một bước.
        let w = world(&rows);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        s.players[0].x = Mpx(66_000);
        s.players[0].y = Mpx(72_000);
        run(&w, &mut s, Dir::Right, 1);
        assert_eq!(s.players[0].x, Mpx(66_000), "vẫn bị khối cứng chặn");
        assert_eq!(
            s.players[0].y,
            Mpx(70_000),
            "lệch 12 px: bám góc kéo về tâm hàng"
        );

        // Lệch 13 px: vượt phạm vi assist nên đứng yên.
        let w = world(&rows);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        s.players[0].x = Mpx(66_000);
        s.players[0].y = Mpx(73_000);
        run(&w, &mut s, Dir::Right, 1);
        assert_eq!(s.players[0].x, Mpx(66_000));
        assert_eq!(s.players[0].y, Mpx(73_000), "lệch 13 px: không bám góc");
    }

    #[test]
    fn trapped_player_moves_until_blocked() {
        let w = world(&["#####", "#0..#", "#####"]);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        s.players[0].life = Life::Trapped { since: 0 };
        run(&w, &mut s, Dir::Right, 20);
        assert_eq!(s.players[0].x, Mpx(100_000));
        assert_eq!(s.players[0].life, Life::Trapped { since: 0 });
        assert_eq!(s.players[0].facing, Dir::Right);
    }

    #[test]
    fn dead_player_ignores_direction_and_turn_events() {
        let w = world(&["#####", "#0..#", "#####"]);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        s.players[0].life = Life::Dead;
        let before = s.players[0].clone();
        let mut events = Vec::new();
        s.step(&w, &[walk(Dir::Right)], &mut events).unwrap();
        assert_eq!(s.players[0], before);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, crate::event::EventKind::PlayerTurned { .. }))
        );
    }

    #[test]
    fn profile_can_disable_trapped_movement_without_disabling_alive_movement() {
        let mut r = rules();
        r.trapped_can_move = false;
        let w = World::new(
            Terrain::from_ascii(&["#####", "#0..#", "#####"]).unwrap(),
            r,
            mode_by_id("battle_ffa").unwrap(),
        )
        .unwrap();
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        s.players[0].life = Life::Trapped { since: 0 };
        run(&w, &mut s, Dir::Right, 1);
        assert_eq!(s.players[0].x, Mpx(60_000));
        s.players[0].life = Life::Alive;
        run(&w, &mut s, Dir::Right, 1);
        assert!(s.players[0].x > Mpx(60_000));
    }

    #[test]
    fn trapped_player_leaves_grace_but_cannot_reenter_bomb() {
        let w = world(&["#####", "#0..#", "#####"]);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        s.players[0].life = Life::Trapped { since: 0 };
        s.bombs.push(Bomb {
            id: EntityId(1),
            owner: 0,
            cell: Cell::new(1, 1),
            range: 1,
            explodes_at: 1000,
            grace: vec![0],
        });
        run(&w, &mut s, Dir::Right, 20);
        assert_eq!(s.players[0].x, Mpx(100_000));
        assert!(s.bombs[0].grace.is_empty());
        run(&w, &mut s, Dir::Left, 20);
        // Bước 2 px dừng ở x=94: bước tiếp sẽ chồm ô bom kết thúc ở x=80.
        assert_eq!(s.players[0].x, Mpx(94_000));
        assert_eq!(s.players[0].life, Life::Trapped { since: 0 });
    }

    #[test]
    fn grace_is_dropped_when_the_box_leaves_the_bomb_cell() {
        // R-107: pass chỉ còn khi hộp còn chồng ô bom; rời hẳn ô thì mất và bom chặn người đó.
        let w = world(&["#####", "#0..#", "#####"]);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        s.bombs.push(Bomb {
            id: EntityId(1),
            owner: 0,
            cell: Cell::new(1, 1),
            range: 1,
            explodes_at: 1000,
            grace: vec![0],
        });
        run(&w, &mut s, Dir::Right, 20);
        assert_eq!(s.players[0].x, Mpx(100_000));
        assert!(
            s.bombs[0].grace.is_empty(),
            "hộp đã rời hẳn ô bom nên pass bị xóa"
        );
    }
}
