//! Input mạng: `InputBatch`, `InputCommand`, `InputRejected` (ADR 0009 §3).
//!
//! Input mạng chỉ là **đề nghị**; server kiểm round, sequence, cửa sổ và rate limit rồi mới
//! chuyển thành `PlayerInput` của core. Client không gửi tọa độ, máu, item, kill hay kết quả.

use serde::{Deserialize, Serialize};

use crate::dto::DIR_MAX;
use crate::error::ProtoError;
use crate::kind::MessageKind;
use crate::limits::{ProtocolLimits, WIRE_MAX_COMMANDS};
use crate::message::Message;

/// Byte `bits` của một lệnh (ADR 0009 §3.1).
///
/// Bit 0–2 là hướng (0 = None, 1 = Up, 2 = Down, 3 = Left, 4 = Right), bit 3 là cạnh nhấn bom,
/// bit 4–7 phải bằng 0. Hướng là **đang giữ**, không phải delta.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputBits(pub u8);

impl InputBits {
    /// Lệnh không đổi hướng, không nhấn bom.
    pub const NONE: Self = Self(0);
    /// Mask bit cờ bom.
    pub const BOMB: u8 = 0b1000;
    /// Mask bit hướng.
    pub const DIRECTION: u8 = 0b0111;
    /// Mask bit phải bằng 0.
    pub const RESERVED: u8 = 0b1111_0000;

    /// Mã hướng (0..=4).
    pub fn direction(self) -> u8 {
        self.0 & Self::DIRECTION
    }

    /// Có cạnh nhấn bom trong tick đầu tiên lệnh được áp dụng.
    pub fn place_bomb(self) -> bool {
        self.0 & Self::BOMB != 0
    }

    /// Bit dành riêng có bằng 0 không.
    pub fn reserved_is_zero(self) -> bool {
        self.0 & Self::RESERVED == 0
    }

    /// Hướng có hợp lệ không (0..=4).
    pub fn direction_is_valid(self) -> bool {
        self.direction() <= DIR_MAX
    }

    /// Dựng từ hướng và cạnh bom; `None` khi mã hướng ngoài miền.
    pub fn new(direction: u8, place_bomb: bool) -> Option<Self> {
        if direction > DIR_MAX {
            return None;
        }
        let mut bits = direction & Self::DIRECTION;
        if place_bomb {
            bits |= Self::BOMB;
        }
        Some(Self(bits))
    }
}

/// Một lệnh input trong batch (ADR 0009 §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputCommand {
    /// Sequence của lệnh; bắt đầu từ 1 cho mỗi `(round_id, player)`.
    pub seq: u32,
    /// Tick authority mà client dự đoán lệnh có hiệu lực.
    pub target_tick: u32,
    /// Byte hướng + cạnh bom.
    pub bits: InputBits,
}

/// `InputBatch` (kind `0x0010`, C→S): cửa sổ lệnh chưa được ack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputBatch {
    /// Round server đã chốt; lệch round bị từ chối.
    pub round_id: u32,
    /// Người chơi; chỉ để đối chiếu, server **bind** người chơi theo session.
    pub player: u8,
    /// `snapshot_seq` lớn nhất client đã áp dụng đủ.
    pub ack_snapshot: u32,
    /// 1..8 lệnh; `seq` bắt đầu từ 1 và tăng liên tục **trong batch**.
    #[serde(deserialize_with = "de_commands")]
    pub commands: Vec<InputCommand>,
}

/// Lý do server từ chối một batch (ADR 0009 §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputRejectReason {
    /// Batch sai cấu trúc: `seq` không tăng liên tục, trùng, hoặc payload khác bản đã buffer.
    BadSequence = 0,
    /// `target_tick` quá tương lai (vượt `INPUT_LEAD`).
    OutOfWindow = 1,
    /// `seq` vượt `MAX_INPUT_GAP` so với `applied_seq`.
    GapTooLarge = 2,
    /// Vượt rate limit.
    RateLimited = 3,
}

impl InputRejectReason {
    /// Mã lý do trên dây.
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// `InputRejected` (kind `0x0011`, S→C): tín hiệu resync sequence, không phải "hủy cửa sổ".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputRejected {
    /// Round bị từ chối.
    pub round_id: u32,
    /// Mã lý do.
    pub reason: InputRejectReason,
    /// Ack hiện tại sau dedup/retire; client bỏ mọi lệnh `seq <=` giá trị này.
    pub applied_seq: u32,
    /// `seq` đầu tiên của batch bị từ chối; `0` khi không xác định được.
    pub first_seq: u32,
    /// Số lệnh của batch bị từ chối; `0` khi không xác định được.
    pub command_count: u8,
    /// Tick server lúc từ chối; client dùng để phân loại quá khứ/tương lai.
    pub server_tick: u32,
}

fn de_commands<'de, D>(d: D) -> Result<Vec<InputCommand>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::bounded::bounded::<D, InputCommand, WIRE_MAX_COMMANDS>(d)
}

impl InputBatch {
    /// Kiểm DTO sau khi decode: số lệnh trong miền 1..8, `seq` bắt đầu từ 1 và tăng liên tục
    /// **trong batch**, `bits` phải có bit dành riêng bằng 0 và hướng hợp lệ
    /// (ADR 0009 §3.1, §3.3 bước 1).
    ///
    /// Lỗ hổng **giữa** các batch là gap hợp lệ, không phải lỗi cấu trúc.
    pub fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        if self.commands.is_empty() {
            return Err(ProtoError::Invalid {
                field: "commands",
                detail: "batch phải có ít nhất 1 lệnh".into(),
            });
        }
        if self.commands.len() > limits.max_commands {
            return Err(ProtoError::CountLimit {
                field: "commands",
                len: self.commands.len(),
                max: limits.max_commands,
            });
        }
        for (i, cmd) in self.commands.iter().enumerate() {
            if cmd.seq == 0 {
                return Err(ProtoError::Invalid {
                    field: "seq",
                    detail: "seq phải bắt đầu từ 1 cho mỗi (round_id, player)".into(),
                });
            }
            if !cmd.bits.reserved_is_zero() {
                return Err(ProtoError::Invalid {
                    field: "bits",
                    detail: format!("lệnh seq {} có bit dành riêng khác 0", cmd.seq),
                });
            }
            if !cmd.bits.direction_is_valid() {
                return Err(ProtoError::Invalid {
                    field: "bits",
                    detail: format!("lệnh seq {} có mã hướng không hợp lệ", cmd.seq),
                });
            }
            if i > 0 {
                let prev = self.commands[i - 1].seq;
                let cur = cmd.seq;
                // `checked_add`, **không** phải `saturating_add`: ở biên `u32::MAX` phải báo
                // lỗi, không được coi như tăng liên tục — nếu không batch `[MAX, MAX]` (trùng
                // sequence) lọt qua (ADR 0009 §3.1: `seq` tăng 1 mỗi lệnh, không trùng).
                let consecutive = prev.checked_add(1) == Some(cur);
                if !consecutive {
                    return Err(ProtoError::Invalid {
                        field: "seq",
                        detail: format!("seq {prev} rồi {cur} không tăng liên tục trong batch"),
                    });
                }
            }
        }
        Ok(())
    }
}

impl InputRejected {
    /// Kiểm DTO sau khi decode.
    ///
    /// `first_seq`/`command_count` bằng 0 nghĩa là không xác định được batch bị từ chối; client
    /// coi mọi lệnh liên quan là **có thể đã buffer** (ADR 0009 §3.2).
    pub fn check(&self, _limits: &ProtocolLimits) -> Result<(), ProtoError> {
        Ok(())
    }
}

impl Message for InputBatch {
    const KIND: MessageKind = MessageKind::InputBatch;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        InputBatch::check(self, limits)
    }
}

impl Message for InputRejected {
    const KIND: MessageKind = MessageKind::InputRejected;

    fn check(&self, limits: &ProtocolLimits) -> Result<(), ProtoError> {
        InputRejected::check(self, limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::DIR_NONE;

    fn cmd(seq: u32, bits: u8) -> InputCommand {
        InputCommand {
            seq,
            target_tick: 100,
            bits: InputBits(bits),
        }
    }

    #[test]
    fn bits_pack_direction_and_bomb() {
        let b = InputBits::new(4, true).unwrap();
        assert_eq!(b.0, 0b1100);
        assert_eq!(b.direction(), 4);
        assert!(b.place_bomb());
        assert!(b.reserved_is_zero());

        let b = InputBits::new(0, false).unwrap();
        assert_eq!(b.0, 0);
        assert_eq!(b.direction(), DIR_NONE);
        assert!(!b.place_bomb());
    }

    #[test]
    fn invalid_direction_is_rejected() {
        assert!(InputBits::new(5, false).is_none());
        assert!(!InputBits(5).direction_is_valid());
        assert!(!InputBits(0b0001_0000).reserved_is_zero());
    }

    #[test]
    fn batch_requires_consecutive_seq() {
        let limits = ProtocolLimits::v1();
        let ok = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 3,
            commands: vec![cmd(7, 0), cmd(8, 0)],
        };
        assert!(ok.check(&limits).is_ok());

        let bad = InputBatch {
            commands: vec![cmd(7, 0), cmd(9, 0)],
            ..ok.clone()
        };
        assert!(bad.check(&limits).is_err());
    }

    #[test]
    fn batch_rejects_reserved_bits_and_empty_list() {
        let limits = ProtocolLimits::v1();
        let reserved = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: vec![cmd(1, 0b0001_0000)],
        };
        assert!(reserved.check(&limits).is_err());

        let empty = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: vec![],
        };
        assert!(empty.check(&limits).is_err());
    }

    #[test]
    fn batch_over_command_limit_is_rejected() {
        let limits = ProtocolLimits::v1();
        let batch = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: (1..=9).map(|s| cmd(s, 0)).collect(),
        };
        assert!(matches!(
            batch.check(&limits),
            Err(ProtoError::CountLimit {
                field: "commands",
                len: 9,
                max: 8
            })
        ));
    }

    /// Batch chỉ có một lệnh với `seq = 0` phải bị từ chối: sequence bắt đầu từ 1 cho mỗi
    /// `(round_id, player)` (ADR 0009 §3.1).
    #[test]
    fn batch_rejects_zero_seq() {
        let limits = ProtocolLimits::v1();
        let single = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: vec![cmd(0, 0)],
        };
        assert!(
            matches!(
                single.check(&limits),
                Err(ProtoError::Invalid { field: "seq", .. })
            ),
            "lệnh đơn seq 0 phải bị từ chối"
        );

        let zero_then_one = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: vec![cmd(0, 0), cmd(1, 0)],
        };
        assert!(
            matches!(
                zero_then_one.check(&limits),
                Err(ProtoError::Invalid { field: "seq", .. })
            ),
            "batch [0, 1] phải bị từ chối"
        );
    }

    /// Regression: `saturating_add` cho phép batch `[MAX, MAX]` lọt (hai lệnh cùng sequence).
    /// `checked_add` phải từ chối, dù hai lệnh có payload khác nhau.
    #[test]
    fn batch_rejects_duplicate_seq_at_u32_max() {
        let limits = ProtocolLimits::v1();
        let same_bits = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: vec![cmd(u32::MAX, 0), cmd(u32::MAX, 0)],
        };
        assert!(
            matches!(
                same_bits.check(&limits),
                Err(ProtoError::Invalid { field: "seq", .. })
            ),
            "batch [MAX, MAX] phải bị từ chối"
        );

        let different_bits = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: vec![cmd(u32::MAX, 0), cmd(u32::MAX, 0b0000_0100)],
        };
        assert!(
            matches!(
                different_bits.check(&limits),
                Err(ProtoError::Invalid { field: "seq", .. })
            ),
            "batch [MAX, MAX] khác bits phải bị từ chối"
        );
    }

    /// `[MAX-1, MAX]` vẫn hợp lệ: sequence tăng đúng 1 mỗi lệnh.
    #[test]
    fn batch_accepts_consecutive_seq_at_u32_max() {
        let limits = ProtocolLimits::v1();
        let batch = InputBatch {
            round_id: 1,
            player: 0,
            ack_snapshot: 0,
            commands: vec![cmd(u32::MAX - 1, 0), cmd(u32::MAX, 0)],
        };
        assert!(
            batch.check(&limits).is_ok(),
            "batch [MAX-1, MAX] phải hợp lệ: {:?}",
            batch.check(&limits)
        );
    }
}
