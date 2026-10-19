//! Chuyển `SimulationState` ↔ DTO `Snapshot` (ADR 0009 §4).
//!
//! Snapshot V1 là **toàn bộ state**: dựng lại được `SimulationState` và giữ **cùng state hash**.
//! Việc dựng lại dùng getter contracts-r4 (`Rng::state`/`from_state`, `IdAllocator::peek_next`/
//! `from_next`) và API công khai sẵn có của `boom-core`; không sửa core.
//!
//! Miền toạ độ `Mpx` (`0 <= x < width·tile_mpx`) được kiểm khi khôi phục: đó là miền của *phiên*
//! nên phải dùng `Rules` đang chạy, không hard-code 15/13 hay 600 000/520 000.

use boom_content::map::CellKind;
use boom_core::{
    Cell, EntityId, Field, Flame, FloorItem, IdAllocator, ItemKind, Life, Phase, Player, Rng,
    SimulationState, Stat, World,
};
use boom_proto::dto::snapshot::InputAck;
use boom_proto::{
    BombState, CellKind as CellKindDto, FieldState, FlameState, HiddenItem, ItemState, Outcome,
    Phase as PhaseDto, PlayerState, Snapshot, Stat as StatDto,
};

use crate::error::SyncError;

/// Thông tin ngoài state mà server gắn vào snapshot.
///
/// `acks` là trạng thái scheduler (đã áp dụng/retire tới sequence nào), không nằm trong
/// `SimulationState`; client cần nó để biết lệnh nào đã được giải quyết (ADR 0009 §3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotContext {
    /// Số snapshot trong round, tăng từ 1.
    pub snapshot_seq: u32,
    /// Round mà snapshot thuộc về.
    pub round_id: u32,
    /// Bố cục DTO `Snapshot`; V1 là `1`.
    pub state_schema: u16,
    /// Hash profile rules của phiên.
    pub rules_hash: u64,
    /// Content epoch của phiên.
    pub content_epoch: u32,
    /// Ack input của từng người chơi.
    pub acks: Vec<InputAck>,
}

/// Dựng DTO `Snapshot` từ state của core.
pub fn to_snapshot(state: &SimulationState, ctx: &SnapshotContext) -> Result<Snapshot, SyncError> {
    let width = state.field.width();
    let height = state.field.height();
    let cell_index =
        |c: Cell| -> u16 { u16::try_from(c.y * width + c.x).expect("ô nằm trong lưới") };

    let mut kinds = Vec::with_capacity(usize::try_from(width * height).unwrap_or(0));
    let mut hidden = Vec::new();
    for c in state.field.cells() {
        let kind = state
            .field
            .kind(c)
            .ok_or_else(|| SyncError::CellOutOfRange {
                cell: u16::try_from(c.y * width + c.x).unwrap_or(u16::MAX),
                cells: usize::try_from(width * height).unwrap_or(0),
            })?;
        kinds.push(match kind {
            CellKind::Floor => CellKindDto::Floor,
            CellKind::Solid => CellKindDto::Solid,
            CellKind::Breakable => CellKindDto::Breakable,
            CellKind::Unknown => CellKindDto::Invalid,
        });
        if let Some(item) = state.field.hidden(c) {
            hidden.push(HiddenItem {
                cell: cell_index(c),
                kind: item_kind_to_u8(item)?,
            });
        }
    }

    let players = state
        .players
        .iter()
        .map(|p| PlayerState {
            id: p.id,
            x: p.x.0,
            y: p.y.0,
            facing: dir_to_code(p.facing),
            speed: stat_to_dto(p.speed),
            bombs: stat_to_dto(p.bombs),
            range: stat_to_dto(p.range),
            life: match p.life {
                Life::Alive => boom_proto::Life::Alive,
                Life::Trapped { since } => boom_proto::Life::Trapped { since },
                Life::Dead => boom_proto::Life::Dead,
            },
        })
        .collect();

    let bombs = state
        .bombs
        .iter()
        .map(|b| BombState {
            id: b.id.0,
            owner: b.owner,
            cell: cell_index(b.cell),
            range: b.range,
            explodes_at: b.explodes_at,
            grace: b.grace.clone(),
        })
        .collect();

    let flames = state
        .flames
        .iter()
        .map(|f| FlameState {
            cell: cell_index(f.cell),
            until: f.until,
            source: f.source.0,
            owner: f.owner,
        })
        .collect();

    let mut items = Vec::with_capacity(state.items.len());
    for i in &state.items {
        items.push(ItemState {
            id: i.id.0,
            kind: item_kind_to_u8(i.kind)?,
            cell: cell_index(i.cell),
        });
    }

    Ok(Snapshot {
        snapshot_seq: ctx.snapshot_seq,
        round_id: ctx.round_id,
        tick: state.tick,
        state_schema: ctx.state_schema,
        rules_hash: ctx.rules_hash,
        content_epoch: ctx.content_epoch,
        phase: match state.phase {
            Phase::Playing => PhaseDto::Playing,
            Phase::Ended(boom_core::Outcome::Draw) => PhaseDto::Ended(Outcome::Draw),
            Phase::Ended(boom_core::Outcome::Winner(id)) => PhaseDto::Ended(Outcome::Winner(id)),
        },
        rng_state: state.rng.state(),
        next_entity_id: state.ids.peek_next(),
        acks: ctx.acks.clone(),
        players,
        field: FieldState {
            width: u16::try_from(width).expect("chiều lưới vừa u16"),
            height: u16::try_from(height).expect("chiều lưới vừa u16"),
            kinds_bits: FieldState::pack_kinds(&kinds),
            hidden,
        },
        bombs,
        flames,
        items,
    })
}

/// Khôi phục `SimulationState` từ DTO `Snapshot`.
///
/// Kiểm trước khi dựng: kích thước lưới khớp `World`, mọi ô trong lưới, miền toạ độ `Mpx`
/// theo `Rules` của phiên. Sau khi dựng xong gọi `check_invariants` của core; state sai bất biến
/// bị từ chối, không áp dụng một phần (ADR 0009 §4.3).
pub fn from_snapshot(snapshot: &Snapshot, world: &World) -> Result<SimulationState, SyncError> {
    let terrain = world.terrain();
    if (snapshot.field.width, snapshot.field.height) != (terrain.width, terrain.height) {
        return Err(SyncError::GridMismatch {
            snapshot: (snapshot.field.width, snapshot.field.height),
            world: (terrain.width, terrain.height),
        });
    }
    let width = i32::from(snapshot.field.width);
    let height = i32::from(snapshot.field.height);
    let cells = usize::try_from(width * height).expect("lưới vừa usize");
    let cell_of = |index: u16| -> Cell {
        let idx = i32::from(index);
        Cell::new(idx % width, idx / width)
    };

    let kinds = snapshot
        .field
        .kinds()
        .ok_or_else(|| SyncError::KindsBitsLength {
            expected: snapshot.field.kinds_bytes(),
            got: snapshot.field.kinds_bits.len(),
        })?;
    for (i, kind) in kinds.iter().enumerate() {
        if *kind == CellKindDto::Invalid {
            return Err(SyncError::InvalidCellKind {
                cell: u16::try_from(i).unwrap_or(u16::MAX),
            });
        }
    }

    // Miền toạ độ theo Rules của phiên: 0 <= x < width·tile_mpx (ADR 0009 §4.2).
    let tile_mpx = world.rules().tile.0;
    let max_x = width * tile_mpx;
    let max_y = height * tile_mpx;

    let mut field = Field::from_terrain(terrain);
    for (i, kind) in kinds.iter().enumerate() {
        let c = cell_of(u16::try_from(i).expect("ô vừa u16"));
        let core_kind = match kind {
            CellKindDto::Floor => CellKind::Floor,
            CellKindDto::Solid => CellKind::Solid,
            CellKindDto::Breakable => CellKind::Breakable,
            CellKindDto::Invalid => CellKind::Unknown,
        };
        field.set_kind(c, core_kind);
    }
    for h in &snapshot.field.hidden {
        if usize::from(h.cell) >= cells {
            return Err(SyncError::CellOutOfRange {
                cell: h.cell,
                cells,
            });
        }
        field.set_hidden(cell_of(h.cell), Some(ItemKind(u16::from(h.kind))));
    }

    let mut players = Vec::with_capacity(snapshot.players.len());
    for p in &snapshot.players {
        if p.x < 0 || p.x >= max_x || p.y < 0 || p.y >= max_y {
            return Err(SyncError::PositionOutOfRange {
                player: p.id,
                x: p.x,
                y: p.y,
                max_x,
                max_y,
            });
        }
        players.push(Player {
            id: p.id,
            x: boom_core::Mpx(p.x),
            y: boom_core::Mpx(p.y),
            facing: dir_from_code(p.facing).ok_or(SyncError::BadDirection(p.facing))?,
            speed: stat_from_dto(p.speed),
            bombs: stat_from_dto(p.bombs),
            range: stat_from_dto(p.range),
            life: match p.life {
                boom_proto::Life::Alive => Life::Alive,
                boom_proto::Life::Trapped { since } => Life::Trapped { since },
                boom_proto::Life::Dead => Life::Dead,
            },
        });
    }

    let mut bombs = Vec::with_capacity(snapshot.bombs.len());
    for b in &snapshot.bombs {
        if usize::from(b.cell) >= cells {
            return Err(SyncError::CellOutOfRange {
                cell: b.cell,
                cells,
            });
        }
        bombs.push(boom_core::Bomb {
            id: EntityId(b.id),
            owner: b.owner,
            cell: cell_of(b.cell),
            range: b.range,
            explodes_at: b.explodes_at,
            grace: b.grace.clone(),
        });
    }

    let mut flames = Vec::with_capacity(snapshot.flames.len());
    for f in &snapshot.flames {
        if usize::from(f.cell) >= cells {
            return Err(SyncError::CellOutOfRange {
                cell: f.cell,
                cells,
            });
        }
        flames.push(Flame {
            cell: cell_of(f.cell),
            until: f.until,
            source: EntityId(f.source),
            owner: f.owner,
        });
    }

    let mut items = Vec::with_capacity(snapshot.items.len());
    for i in &snapshot.items {
        if usize::from(i.cell) >= cells {
            return Err(SyncError::CellOutOfRange {
                cell: i.cell,
                cells,
            });
        }
        items.push(FloorItem {
            id: EntityId(i.id),
            kind: ItemKind(u16::from(i.kind)),
            cell: cell_of(i.cell),
        });
    }

    let ids = IdAllocator::from_next(snapshot.next_entity_id)
        .ok_or(SyncError::Allocator(snapshot.next_entity_id))?;

    let state = SimulationState {
        tick: snapshot.tick,
        phase: match snapshot.phase {
            PhaseDto::Playing => Phase::Playing,
            PhaseDto::Ended(Outcome::Draw) => Phase::Ended(boom_core::Outcome::Draw),
            PhaseDto::Ended(Outcome::Winner(id)) => Phase::Ended(boom_core::Outcome::Winner(id)),
        },
        rng: Rng::from_state(snapshot.rng_state),
        ids,
        players,
        field,
        bombs,
        flames,
        items,
    };
    state
        .check_invariants(world)
        .map_err(SyncError::Invariant)?;
    Ok(state)
}

fn stat_to_dto(s: Stat) -> StatDto {
    StatDto {
        value: s.value,
        max: s.max,
    }
}

fn stat_from_dto(s: StatDto) -> Stat {
    Stat {
        value: s.value,
        max: s.max,
    }
}

fn item_kind_to_u8(kind: ItemKind) -> Result<u8, SyncError> {
    u8::try_from(kind.0).map_err(|_| SyncError::ItemKindTooLarge(kind.0))
}

/// Mã hướng trên dây (ADR 0009 §3.1): 0 = None, 1 = Up, 2 = Down, 3 = Left, 4 = Right.
fn dir_to_code(dir: boom_core::Dir) -> u8 {
    match dir {
        boom_core::Dir::None => boom_proto::dto::DIR_NONE,
        boom_core::Dir::Up => boom_proto::dto::DIR_UP,
        boom_core::Dir::Down => boom_proto::dto::DIR_DOWN,
        boom_core::Dir::Left => boom_proto::dto::DIR_LEFT,
        boom_core::Dir::Right => boom_proto::dto::DIR_RIGHT,
    }
}

/// Mã hướng trên dây → hướng của core.
fn dir_from_code(code: u8) -> Option<boom_core::Dir> {
    Some(match code {
        boom_proto::dto::DIR_NONE => boom_core::Dir::None,
        boom_proto::dto::DIR_UP => boom_core::Dir::Up,
        boom_proto::dto::DIR_DOWN => boom_core::Dir::Down,
        boom_proto::dto::DIR_LEFT => boom_core::Dir::Left,
        boom_proto::dto::DIR_RIGHT => boom_core::Dir::Right,
        _ => return None,
    })
}
