//! Di chuyển và va chạm ô — V1-011, V1-012 (đang ở mức skeleton).
//!
//! Đã có: đi theo hướng giữ, dừng ở tâm ô trước khối cứng/mềm hoặc mép lưới, căn về tâm ô
//! đang đứng trước khi rẽ (design tạm, RULES_V1 §6). Còn thiếu cho V1-012: bom chặn đường và
//! grace khi vừa đặt bom (`Bomb.grace`), tốc độ khi bị nhốt (R-029), bám góc (R-046, Q-005).
//! Kịch bản: `fixtures/scenarios/movement/`.

use boom_content::map::CellKind;

use crate::event::{EventKind, EventSink};
use crate::num::{Cell, Mpx};
use crate::sim::{Dir, PlayerInput, World};
use crate::state::{Life, Player, SimulationState};
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
    ) {
        let field = &state.field;
        let walkable = |c: Cell| field.kind(c) == Some(CellKind::Floor);
        for (player, input) in state.players.iter_mut().zip(inputs) {
            if player.life != Life::Alive || input.dir == Dir::None {
                continue;
            }
            if player.facing != input.dir {
                player.facing = input.dir;
                events.emit(EventKind::PlayerTurned { player: player.id });
            }
            move_player(world, player, input.dir, &walkable);
        }
    }
}

fn move_player(world: &World, p: &mut Player, dir: Dir, walkable: &impl Fn(Cell) -> bool) {
    let (dx, dy) = dir.delta();
    let speed = world.speed(p.speed.value);
    if dx != 0 {
        let row = world.cell_index(p.y);
        let center = world.center(row);
        if p.y != center {
            p.y = approach(p.y, center, speed);
            return;
        }
        p.x = advance(world, p.x, dx, speed, |c| walkable(Cell::new(c, row)));
    } else {
        let col = world.cell_index(p.x);
        let center = world.center(col);
        if p.x != center {
            p.x = approach(p.x, center, speed);
            return;
        }
        p.y = advance(world, p.y, dy, speed, |c| walkable(Cell::new(col, c)));
    }
}

/// Tiến theo một trục; không vượt tâm ô khi ô phía trước bị chặn.
fn advance(world: &World, pos: Mpx, sign: i32, speed: Mpx, open: impl Fn(i32) -> bool) -> Mpx {
    let target = Mpx(pos.0 + sign * speed.0);
    let here = world.cell_index(pos);
    let there = world.cell_index(target);
    for cell in [here, there] {
        let center = world.center(cell);
        if !open(cell + sign) && (target.0 - center.0) * sign > 0 {
            return if (pos.0 - center.0) * sign > 0 {
                pos
            } else {
                center
            };
        }
        if here == there {
            break;
        }
    }
    target
}

fn approach(from: Mpx, to: Mpx, step: Mpx) -> Mpx {
    if (to.0 - from.0).abs() <= step.0 {
        to
    } else {
        Mpx(from.0 + step.0 * (to.0 - from.0).signum())
    }
}
