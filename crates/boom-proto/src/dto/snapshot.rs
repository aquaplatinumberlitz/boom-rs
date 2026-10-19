//! `Snapshot` đầy đủ và `SnapshotRequest` (ADR 0009 §4).
//!
//! V1 gửi **toàn bộ state** trong mỗi snapshot: một snapshot bất kỳ là điểm resync độc lập,
//! không có baseline/delta. Snapshot **không** chứa event; event đi trong `RoundEvents`.
//!
//! Thứ tự field dưới đây là thứ tự encode (postcard struct); đổi thứ tự/kiểu field là tăng
//! `state_schema`.

use serde::{Deserialize, Serialize};

use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::{
    ProtocolLimits, WIRE_MAX_ACKS, WIRE_MAX_BOMBS, WIRE_MAX_FLAMES, WIRE_MAX_GRACE,
    WIRE_MAX_HIDDEN, WIRE_MAX_ITEMS, WIRE_MAX_KINDS_BYTES, WIRE_MAX_PLAYERS,
};
use crate::message::Message;

/// Kết quả lượt (`phase` của snapshot).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Hòa.
    Draw,
    /// Có người thắng, mã người chơi.
    Winner(u8),
}

/// Pha của lượt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Đang chơi.
    Playing,
    /// Lượt đã kết thúc.
    Ended(Outcome),
}

/// Một chỉ số có trần.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stat {
    /// Giá trị hiện tại.
    pub value: u8,
    /// Trần.
    pub max: u8,
}

/// Sống của một người chơi.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Life {
    /// Còn sống.
    Alive,
    /// Bị nhốt trong bong bóng từ tick `since`; vẫn tính là chưa chết.
    Trapped {
        /// Tick bị nhốt.
        since: u32,
    },
    /// Đã chết.
    Dead,
}

/// Trạng thái input đã giải quyết của một người chơi (ack trong snapshot).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputAck {
    /// Người chơi.
    pub player: u8,
    /// Sequence đã **giải quyết** (đã áp dụng hoặc đã retire), không phải ack TCP.
    pub last_seq: u32,
    /// Tick áp dụng/retire lệnh cuối.
    pub applied_tick: u32,
}

/// Trạng thái một người chơi trong snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    /// ID người chơi.
    pub id: u8,
    /// Toạ độ `Mpx` (1/1000 px) của tâm nhân vật; **không** phải chỉ số ô.
    pub x: i32,
    /// Toạ độ `Mpx` của tâm nhân vật.
    pub y: i32,
    /// Mã hướng đang nhìn (0 = None, 1 = Up, 2 = Down, 3 = Left, 4 = Right).
    pub facing: u8,
    /// Cấp tốc độ.
    pub speed: Stat,
    /// Số bom đặt được cùng lúc.
    pub bombs: Stat,
    /// Tầm nổ của bom đặt tiếp theo.
    pub range: Stat,
    /// Sống.
    pub life: Life,
}

/// Loại ô trong `kinds_bits` (ADR 0009 §4.1: 0 = Floor, 1 = Solid, 2 = Breakable, 3 = không hợp lệ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    /// Nền.
    Floor,
    /// Khối cứng.
    Solid,
    /// Khối mềm (phá được, có thể giấu item).
    Breakable,
    /// Không hợp lệ: decoder từ chối.
    Invalid,
}

impl CellKind {
    /// Mã 2 bit của loại ô.
    pub const fn code(self) -> u8 {
        match self {
            Self::Floor => 0,
            Self::Solid => 1,
            Self::Breakable => 2,
            Self::Invalid => 3,
        }
    }

    /// Loại ô từ mã 2 bit.
    pub const fn from_code(code: u8) -> Self {
        match code & 0b11 {
            0 => Self::Floor,
            1 => Self::Solid,
            2 => Self::Breakable,
            _ => Self::Invalid,
        }
    }
}

/// Một item giấu trong khối mềm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenItem {
    /// Chỉ số ô tuyến tính (`y * width + x`).
    pub cell: u16,
    /// Chỉ số loại item trong `Rules::items` của profile.
    pub kind: u8,
}

/// Lưới của lượt trong snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldState {
    /// Số cột.
    pub width: u16,
    /// Số hàng.
    pub height: u16,
    /// Loại ô đóng gói 2 bit/ô: bit thấp trước, ô 0 ở bit 0–1, hàng rồi cột.
    #[serde(deserialize_with = "de_kinds_bits")]
    pub kinds_bits: Vec<u8>,
    /// Item giấu, sắp theo `cell`; chỉ trên ô Breakable.
    #[serde(deserialize_with = "de_hidden")]
    pub hidden: Vec<HiddenItem>,
}

/// Một bom trong snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BombState {
    /// ID entity.
    pub id: u32,
    /// Người đặt.
    pub owner: u8,
    /// Chỉ số ô tuyến tính.
    pub cell: u16,
    /// Tầm nổ chụp lúc đặt.
    pub range: u8,
    /// Tick bom nổ.
    pub explodes_at: u32,
    /// Người được phép đi ra khỏi bom; sắp tăng dần.
    #[serde(deserialize_with = "de_grace")]
    pub grace: Vec<u8>,
}

/// Một ô đang có lửa.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlameState {
    /// Chỉ số ô tuyến tính.
    pub cell: u16,
    /// Tick cuối còn lửa (bao gồm).
    pub until: u32,
    /// Bom sinh lửa.
    pub source: u32,
    /// Người đặt bom.
    pub owner: u8,
}

/// Một item đang nằm trên nền.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemState {
    /// ID entity.
    pub id: u32,
    /// Chỉ số loại item trong `Rules::items` của profile.
    pub kind: u8,
    /// Chỉ số ô tuyến tính.
    pub cell: u16,
}

/// `Snapshot` (kind `0x0020`, S→C): toàn bộ state của lượt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Số snapshot trong round, tăng từ 1.
    pub snapshot_seq: u32,
    /// Round mà snapshot thuộc về.
    pub round_id: u32,
    /// Tick mà state đang ở.
    pub tick: u32,
    /// Bố cục DTO `Snapshot`; V1 là `1`.
    pub state_schema: u16,
    /// Hash profile rules của phiên.
    pub rules_hash: u64,
    /// Content epoch của phiên.
    pub content_epoch: u32,
    /// Pha lượt.
    pub phase: Phase,
    /// State của RNG SplitMix64 hiện tại (không phải seed).
    pub rng_state: u64,
    /// ID entity sẽ cấp tiếp theo.
    pub next_entity_id: u32,
    /// Ack input của từng người chơi, sắp theo `player`.
    #[serde(deserialize_with = "de_acks")]
    pub acks: Vec<InputAck>,
    /// Người chơi, sắp theo `id`.
    #[serde(deserialize_with = "de_players")]
    pub players: Vec<PlayerState>,
    /// Lưới của lượt.
    pub field: FieldState,
    /// Bom, sắp theo `id`.
    #[serde(deserialize_with = "de_bombs")]
    pub bombs: Vec<BombState>,
    /// Lửa, sắp theo `cell`.
    #[serde(deserialize_with = "de_flames")]
    pub flames: Vec<FlameState>,
    /// Item trên nền, sắp theo `id`.
    #[serde(deserialize_with = "de_items")]
    pub items: Vec<ItemState>,
}

/// `SnapshotRequest` (kind `0x0021`, C→S): xin snapshot đầy đủ để resync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRequest {
    /// Round cần resync.
    pub round_id: u32,
    /// `snapshot_seq` cuối cùng client đã thấy.
    pub last_snapshot_seq: u32,
}

fn de_acks<'de, D>(d: D) -> Result<Vec<InputAck>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, InputAck, WIRE_MAX_ACKS>(d)
}

fn de_players<'de, D>(d: D) -> Result<Vec<PlayerState>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, PlayerState, WIRE_MAX_PLAYERS>(d)
}

fn de_bombs<'de, D>(d: D) -> Result<Vec<BombState>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, BombState, WIRE_MAX_BOMBS>(d)
}

fn de_flames<'de, D>(d: D) -> Result<Vec<FlameState>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, FlameState, WIRE_MAX_FLAMES>(d)
}

fn de_items<'de, D>(d: D) -> Result<Vec<ItemState>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, ItemState, WIRE_MAX_ITEMS>(d)
}

fn de_hidden<'de, D>(d: D) -> Result<Vec<HiddenItem>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, HiddenItem, WIRE_MAX_HIDDEN>(d)
}

fn de_grace<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, u8, WIRE_MAX_GRACE>(d)
}

fn de_kinds_bits<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, u8, WIRE_MAX_KINDS_BYTES>(d)
}

impl FieldState {
    /// Số ô của lưới.
    pub fn cells(&self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }

    /// Số byte cần để đóng gói loại ô (`ceil(cells / 4)`).
    pub fn kinds_bytes(&self) -> usize {
        self.cells().div_ceil(4)
    }

    /// Đóng gói loại ô thành 2 bit/ô: bit thấp trước, ô 0 ở bit 0–1, hàng rồi cột.
    pub fn pack_kinds(kinds: &[CellKind]) -> Vec<u8> {
        let mut bits = vec![0u8; kinds.len().div_ceil(4)];
        for (i, kind) in kinds.iter().enumerate() {
            bits[i / 4] |= kind.code() << ((i % 4) * 2);
        }
        bits
    }

    /// Giải nén loại ô từ `kinds_bits`; `None` khi độ dài byte không khớp số ô.
    pub fn unpack_kinds(bits: &[u8], cells: usize) -> Option<Vec<CellKind>> {
        if bits.len() != cells.div_ceil(4) {
            return None;
        }
        let mut kinds = Vec::with_capacity(cells);
        for i in 0..cells {
            kinds.push(CellKind::from_code(bits[i / 4] >> ((i % 4) * 2)));
        }
        Some(kinds)
    }

    /// Loại ô của mọi ô, hàng rồi cột.
    pub fn kinds(&self) -> Option<Vec<CellKind>> {
        Self::unpack_kinds(&self.kinds_bits, self.cells())
    }

    /// Kiểm miền của lưới: kích thước, độ dài `kinds_bits`, ô hợp lệ, item giấu chỉ trên khối mềm.
    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        let cells = self.cells();
        if cells == 0 {
            return Err(ProtoError::Invalid {
                field: "field",
                detail: "lưới rỗng".into(),
            });
        }
        if cells > limits.max_cells {
            return Err(ProtoError::CountLimit {
                field: "field.cells",
                len: cells,
                max: limits.max_cells,
            });
        }
        if self.kinds_bits.len() != self.kinds_bytes() {
            return Err(ProtoError::Invalid {
                field: "field.kinds_bits",
                detail: format!(
                    "cần {} byte cho {} ô, có {}",
                    self.kinds_bytes(),
                    cells,
                    self.kinds_bits.len()
                ),
            });
        }
        if self.hidden.len() > limits.max_hidden {
            return Err(ProtoError::CountLimit {
                field: "field.hidden",
                len: self.hidden.len(),
                max: limits.max_hidden,
            });
        }
        let kinds = self.kinds().ok_or_else(|| ProtoError::Invalid {
            field: "field.kinds_bits",
            detail: "không giải nén được loại ô".into(),
        })?;
        let mut prev: Option<u16> = None;
        for h in &self.hidden {
            let cell = usize::from(h.cell);
            if cell >= cells {
                return Err(ProtoError::Invalid {
                    field: "field.hidden.cell",
                    detail: format!("ô {cell} ngoài lưới {cells} ô"),
                });
            }
            if kinds[cell] != CellKind::Breakable {
                return Err(ProtoError::Invalid {
                    field: "field.hidden",
                    detail: format!("ô {cell} giấu item nhưng không phải khối mềm"),
                });
            }
            if let Some(p) = prev
                && h.cell <= p
            {
                return Err(ProtoError::Invalid {
                    field: "field.hidden",
                    detail: "item giấu phải sắp theo ô".into(),
                });
            }
            prev = Some(h.cell);
        }
        Ok(())
    }
}

impl Snapshot {
    /// Kiểm miền của snapshot sau khi decode (ADR 0009 §4.2, §4.3).
    ///
    /// Kiểm *cấu trúc* và *miền phiên*; miền toạ độ `Mpx` cần `Rules` của phiên nên do adapter
    /// (`boom-sync`) kiểm khi dựng lại `SimulationState`.
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.acks.len() > limits.max_acks {
            return Err(ProtoError::CountLimit {
                field: "acks",
                len: self.acks.len(),
                max: limits.max_acks,
            });
        }
        if self.players.len() > limits.max_players {
            return Err(ProtoError::CountLimit {
                field: "players",
                len: self.players.len(),
                max: limits.max_players,
            });
        }
        if self.bombs.len() > limits.max_bombs {
            return Err(ProtoError::CountLimit {
                field: "bombs",
                len: self.bombs.len(),
                max: limits.max_bombs,
            });
        }
        if self.flames.len() > limits.max_flames {
            return Err(ProtoError::CountLimit {
                field: "flames",
                len: self.flames.len(),
                max: limits.max_flames,
            });
        }
        if self.items.len() > limits.max_items {
            return Err(ProtoError::CountLimit {
                field: "items",
                len: self.items.len(),
                max: limits.max_items,
            });
        }
        self.field.check(limits)?;

        let cells = self.field.cells();
        let mut prev_player = None;
        for p in &self.players {
            if let Some(prev) = prev_player
                && p.id <= prev
            {
                return Err(ProtoError::Invalid {
                    field: "players.id",
                    detail: "người chơi phải sắp theo id, không trùng".into(),
                });
            }
            prev_player = Some(p.id);
            for (name, stat) in [("speed", p.speed), ("bombs", p.bombs), ("range", p.range)] {
                if stat.value > stat.max {
                    return Err(ProtoError::Invalid {
                        field: "players.stat",
                        detail: format!("người chơi {}: {name} vượt trần", p.id),
                    });
                }
            }
            if p.facing > crate::dto::DIR_MAX {
                return Err(ProtoError::Invalid {
                    field: "players.facing",
                    detail: format!("người chơi {}: mã hướng không hợp lệ", p.id),
                });
            }
        }

        let mut prev_ack = None;
        for a in &self.acks {
            if let Some(prev) = prev_ack
                && a.player <= prev
            {
                return Err(ProtoError::Invalid {
                    field: "acks.player",
                    detail: "ack phải sắp theo người chơi, không trùng".into(),
                });
            }
            prev_ack = Some(a.player);
        }

        let mut prev_bomb = None;
        for b in &self.bombs {
            if let Some(prev) = prev_bomb
                && b.id <= prev
            {
                return Err(ProtoError::Invalid {
                    field: "bombs.id",
                    detail: "bom phải sắp theo id".into(),
                });
            }
            prev_bomb = Some(b.id);
            if usize::from(b.cell) >= cells {
                return Err(ProtoError::Invalid {
                    field: "bombs.cell",
                    detail: format!("ô {} ngoài lưới {cells} ô", b.cell),
                });
            }
            if b.grace.len() > limits.max_grace {
                return Err(ProtoError::CountLimit {
                    field: "bombs.grace",
                    len: b.grace.len(),
                    max: limits.max_grace,
                });
            }
            if !b.grace.windows(2).all(|w| w[0] < w[1]) {
                return Err(ProtoError::Invalid {
                    field: "bombs.grace",
                    detail: "grace phải sắp tăng, không trùng".into(),
                });
            }
        }

        let mut prev_flame = None;
        for f in &self.flames {
            if let Some(prev) = prev_flame
                && f.cell <= prev
            {
                return Err(ProtoError::Invalid {
                    field: "flames.cell",
                    detail: "lửa phải sắp theo ô, mỗi ô một lửa".into(),
                });
            }
            prev_flame = Some(f.cell);
            if usize::from(f.cell) >= cells {
                return Err(ProtoError::Invalid {
                    field: "flames.cell",
                    detail: format!("ô {} ngoài lưới {cells} ô", f.cell),
                });
            }
        }

        let mut prev_item = None;
        for i in &self.items {
            if let Some(prev) = prev_item
                && i.id <= prev
            {
                return Err(ProtoError::Invalid {
                    field: "items.id",
                    detail: "item phải sắp theo id".into(),
                });
            }
            prev_item = Some(i.id);
            if usize::from(i.cell) >= cells {
                return Err(ProtoError::Invalid {
                    field: "items.cell",
                    detail: format!("ô {} ngoài lưới {cells} ô", i.cell),
                });
            }
        }
        Ok(())
    }
}

impl SnapshotRequest {
    /// Kiểm DTO sau khi decode.
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl Message for Snapshot {
    const KIND: MessageKind = MessageKind::Snapshot;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Snapshot::check(self, limits)
    }
}

impl Message for SnapshotRequest {
    const KIND: MessageKind = MessageKind::SnapshotRequest;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        SnapshotRequest::check(self, limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_pack_two_bits_per_cell() {
        let kinds = vec![
            CellKind::Floor,
            CellKind::Solid,
            CellKind::Breakable,
            CellKind::Invalid,
            CellKind::Floor,
        ];
        let bits = FieldState::pack_kinds(&kinds);
        assert_eq!(bits.len(), 2);
        // Ô 0 = Floor (00), ô 1 = Solid (01), ô 2 = Breakable (10), ô 3 = Invalid (11).
        assert_eq!(bits[0], 0b1110_0100);
        assert_eq!(bits[1], 0);
        assert_eq!(FieldState::unpack_kinds(&bits, 5).unwrap(), kinds);
    }

    #[test]
    fn kinds_length_must_match_cell_count() {
        // 5 ô cần 2 byte; 1 byte là thiếu, 3 byte là thừa.
        assert!(FieldState::unpack_kinds(&[0], 5).is_none());
        assert!(FieldState::unpack_kinds(&[0, 0], 5).is_some());
        assert!(FieldState::unpack_kinds(&[0, 0, 0], 5).is_none());
    }
}
