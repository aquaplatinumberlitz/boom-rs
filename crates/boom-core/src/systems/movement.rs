//! Di chuyển và va chạm ô — V1-011, V1-012.
//!
//! Đã có: đi theo hướng giữ, dừng ở tâm ô trước khối cứng/mềm/mép lưới, căn về tâm ô đang đứng
//! trước khi rẽ (design tạm, RULES_V1 §6), bom chặn đường (R-048), grace một chiều khi vừa đặt bom
//! (R-049), người bị nhốt đứng yên (R-036). Còn thiếu: bám góc (R-046, Q-005), tốc độ khi bị nhốt
//! nếu đổi quyết định design. Kịch bản: `fixtures/scenarios/movement/`.
//!
//! `Bomb.grace` do bomb_place thêm lúc đặt; movement chỉ xóa khi người đó rời ô. Vì movement chạy
//! trước bomb_place trong một tick, grace của bom đặt trong tick N bắt đầu có hiệu lực từ tick N+1.

use boom_content::map::CellKind;

use crate::event::{EventKind, EventSink};
use crate::num::{Cell, Mpx};
use crate::sim::{Dir, PlayerInput, World};
use crate::state::{Bomb, Life, Player, SimulationState};
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
        let SimulationState {
            players,
            bombs,
            field,
            ..
        } = state;
        let walkable = |c: Cell| field.kind(c) == Some(CellKind::Floor);
        for (player, input) in players.iter_mut().zip(inputs) {
            if player.life != Life::Alive || input.dir == Dir::None {
                continue;
            }
            if player.facing != input.dir {
                player.facing = input.dir;
                events.emit(EventKind::PlayerTurned { player: player.id });
            }
            move_player(world, player, input.dir, bombs, &walkable);
        }
        clear_finished_grace(world, players, bombs);
    }
}

/// Grace chỉ còn khi người chơi vẫn đứng trên ô bom; rời ô là mất quyền quay lại (R-049).
/// Giữ id nếu người đó còn trong ô, kể cả khi đang đi lại trong ô mà chưa vượt biên.
fn clear_finished_grace(world: &World, players: &[Player], bombs: &mut [Bomb]) {
    for bomb in bombs {
        let cell = bomb.cell;
        bomb.grace.retain(|&id| {
            players
                .iter()
                .any(|p| p.id == id && world.cell_at(p.x, p.y) == cell)
        });
    }
}

fn move_player(
    world: &World,
    p: &mut Player,
    dir: Dir,
    bombs: &[Bomb],
    walkable: &impl Fn(Cell) -> bool,
) {
    let (dx, dy) = dir.delta();
    let speed = world.speed(p.speed.value);
    let id = p.id;
    if dx != 0 {
        let row = world.cell_index(p.y);
        let center = world.center(row);
        if p.y != center {
            p.y = approach(p.y, center, speed);
            return;
        }
        p.x = advance(
            world,
            p.x,
            dx,
            speed,
            |c| enterable(bombs, walkable, Cell::new(c, row)),
            |c| may_leave(bombs, Cell::new(c, row), id),
        );
    } else {
        let col = world.cell_index(p.x);
        let center = world.center(col);
        if p.x != center {
            p.x = approach(p.x, center, speed);
            return;
        }
        p.y = advance(
            world,
            p.y,
            dy,
            speed,
            |c| enterable(bombs, walkable, Cell::new(col, c)),
            |c| may_leave(bombs, Cell::new(col, c), id),
        );
    }
}

/// Vào ô chỉ khi là nền và không có bom (R-048). Grace không cho đi vào, chỉ cho đi ra (R-049).
fn enterable(bombs: &[Bomb], walkable: &impl Fn(Cell) -> bool, cell: Cell) -> bool {
    walkable(cell) && !bombs.iter().any(|b| b.cell == cell)
}

/// Rời ô có bom cần grace của chính bom đó (R-049).
fn may_leave(bombs: &[Bomb], cell: Cell, player: u8) -> bool {
    bombs
        .iter()
        .find(|b| b.cell == cell)
        .is_none_or(|b| b.grace.contains(&player))
}

/// Tiến theo một trục; không vượt tâm ô khi ô phía trước bị chặn hoặc không được rời ô hiện tại.
fn advance(
    world: &World,
    pos: Mpx,
    sign: i32,
    speed: Mpx,
    open: impl Fn(i32) -> bool,
    may_leave: impl Fn(i32) -> bool,
) -> Mpx {
    let target = Mpx(pos.0 + sign * speed.0);
    let here = world.cell_index(pos);
    let there = world.cell_index(target);
    for cell in [here, there] {
        let center = world.center(cell);
        if !open(cell + sign) && (target.0 - center.0) * sign > 0 {
            return clamp_at(pos, center, sign);
        }
        if here == there {
            break;
        }
    }
    if there != here && !may_leave(here) {
        return clamp_at(pos, world.center(here), sign);
    }
    target
}

/// Dừng ở tâm ô hiện tại nếu chưa qua; đã qua thì không lùi (tránh nhảy ngược khi bị chặn).
fn clamp_at(pos: Mpx, center: Mpx, sign: i32) -> Mpx {
    if (pos.0 - center.0) * sign > 0 {
        pos
    } else {
        center
    }
}

fn approach(from: Mpx, to: Mpx, step: Mpx) -> Mpx {
    if (to.0 - from.0).abs() <= step.0 {
        to
    } else {
        Mpx(from.0 + step.0 * (to.0 - from.0).signum())
    }
}
