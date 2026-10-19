//! Giới hạn giao thức (ADR 0009 §2, §4.2).
//!
//! Đây là **tham số giao thức**, không phải luật chơi: không nằm trong `Rules`, không có claim
//! RULES_V1, không đổi theo mode. Mỗi giá trị mang revision; đổi giá trị là tăng revision
//! (ADR 0008), server báo `protocol_limits_version` trong `Welcome`.
//!
//! Hai lớp giới hạn:
//!
//! - **Trần định dạng** (`WIRE_*`): hằng số của *wire format*, dùng lúc decode để chặn
//!   length prefix độc hại **trước khi cấp phát** (ADR 0009 §5.2). Chúng không phụ thuộc map
//!   hay số người chơi của phiên.
//! - **Miền V1** (`ProtocolLimits`): giá trị *phiên* của V1 (ADR 0009 §4.2) — số người chơi, số
//!   bom/lửa/item, số ô. Kiểm sau decode, trả lỗi rõ ràng; server/client lấy từ `Welcome`.

/// Bảng kind đã định nghĩa ở ADR 0009 §2; kind khác bị từ chối, không đoán.
pub const WIRE_SCHEMA: u16 = 1;

/// Envelope dài 16 byte (ADR 0009 §2).
pub const ENVELOPE_BYTES: usize = 16;

/// Trần frame: 64 KiB **gồm cả envelope** (ADR 0009 §2).
pub const MAX_FRAME_BYTES: usize = 65_536;

/// Trần payload của message `Snapshot`: 16 KiB (ADR 0009 §2).
pub const MAX_SNAPSHOT_BYTES: usize = 16_384;

/// Trần payload của message `InputBatch`: 1 KiB (ADR 0009 §2).
pub const MAX_INPUT_BATCH_BYTES: usize = 1_024;

/// Số người chơi tối đa trong một snapshot/ack list (miền V1, ADR 0009 §4.2).
pub const MAX_PLAYERS: usize = 4;

/// Số mục ack tối đa (một mục/người chơi).
pub const MAX_ACKS: usize = MAX_PLAYERS;

/// Số lệnh tối đa trong một batch (ADR 0009 §3.1: 1..8).
pub const MAX_COMMANDS: usize = 8;

/// Số người tối đa được ghi trong `grace` của một bom.
pub const MAX_GRACE: usize = MAX_PLAYERS;

/// Số bom tối đa trong một snapshot (4 × C00 max bombs 10, làm tròn xuống 40 — ADR 0009 §4.2).
pub const MAX_BOMBS: usize = 40;

/// Số ô lửa tối đa trong một snapshot (một ô nhiều nhất — ADR 0009 §4.2).
pub const MAX_FLAMES: usize = 195;

/// Số item trên nền tối đa trong một snapshot (một ô nhiều nhất — ADR 0009 §4.2).
pub const MAX_ITEMS: usize = 195;

/// Số item giấu tối đa trong một snapshot (mỗi ô khối mềm nhiều nhất — ADR 0009 §4.2).
pub const MAX_HIDDEN: usize = 195;

/// Số ô lưới tối đa của một map trong miền V1 (Village00 15×13 = 195 — ADR 0009 §4.2).
pub const MAX_CELLS: usize = 195;

/// Trần độ dài `username` trong `Auth` (ADR 0009 Addendum A §A.2).
///
/// Đây là **biên giao thức**: server kiểm sau decode, trước khi đụng DB, và trả
/// `AuthFail { InvalidInput }` khi vượt (không hash). Không phải luật tài khoản của
/// ADR 0011 §4 (tên 3–24 ký tự): đó là tầng account store, kiểm ở server.
pub const MAX_USERNAME_BYTES: usize = 64;

/// Trần độ dài `password` trong `Auth` (ADR 0009 Addendum A §A.2).
///
/// Biên giao thức, cùng ý với [`MAX_USERNAME_BYTES`]: quá giới hạn → `AuthFail { InvalidInput }`,
/// không hash.
pub const MAX_PASSWORD_BYTES: usize = 256;

/// Số mục `problems` tối đa trong `HealthResponse`.
///
/// **Design**: ADR 0011 §8.2.1 liệt kê `problems` nhưng không chốt con số; 32 đủ cho mọi kiểm
/// sức khoẻ hiện biết và chặn danh sách độc hại.
pub const MAX_PROBLEMS: usize = 32;

// --- Trần định dạng (chặn length prefix độc hại lúc decode) ---------------------------

/// Trần định dạng cho danh sách người chơi.
pub const WIRE_MAX_PLAYERS: usize = u8::MAX as usize;
/// Trần định dạng cho danh sách ack.
pub const WIRE_MAX_ACKS: usize = u8::MAX as usize;
/// Trần định dạng cho danh sách lệnh trong một batch.
pub const WIRE_MAX_COMMANDS: usize = u8::MAX as usize;
/// Trần định dạng cho danh sách `grace` của một bom.
pub const WIRE_MAX_GRACE: usize = u8::MAX as usize;
/// Trần định dạng cho danh sách bom.
pub const WIRE_MAX_BOMBS: usize = 4_096;
/// Trần định dạng cho danh sách lửa.
pub const WIRE_MAX_FLAMES: usize = 4_096;
/// Trần định dạng cho danh sách item.
pub const WIRE_MAX_ITEMS: usize = 4_096;
/// Trần định dạng cho danh sách item giấu.
pub const WIRE_MAX_HIDDEN: usize = 4_096;
/// Trần định dạng cho danh sách capability trong `Hello`.
pub const WIRE_MAX_CAPABILITIES: usize = 64;
/// Trần định dạng cho byte đóng gói loại ô (2 bit/ô, tối đa 65 536 ô).
pub const WIRE_MAX_KINDS_BYTES: usize = 16_384;
/// Trần định dạng cho độ dài `username` trong `Auth`.
///
/// **Rộng hơn** biên V1 ([`MAX_USERNAME_BYTES`]) để chuỗi vượt biên vẫn decode được rồi bị
/// `check` từ chối bằng lỗi có kiểu — server cần phân biệt "vượt biên" để trả
/// `AuthFail { InvalidInput }` (ADR 0009 Addendum A §A.2). Trần này chỉ chặn length prefix độc hại.
pub const WIRE_MAX_USERNAME_BYTES: usize = 1_024;
/// Trần định dạng cho độ dài `password` trong `Auth`; cùng ý với [`WIRE_MAX_USERNAME_BYTES`].
pub const WIRE_MAX_PASSWORD_BYTES: usize = 4_096;
/// Trần định dạng cho danh sách `problems` trong `HealthResponse`.
pub const WIRE_MAX_PROBLEMS: usize = 64;
/// Trần định dạng cho danh sách `listen_addrs` trong `HealthResponse`.
pub const WIRE_MAX_LISTEN_ADDRS: usize = 16;

/// Miền giá trị của một phiên V1 (ADR 0009 §2, §4.2).
///
/// Giá trị mặc định là miền V1; server gửi `protocol_limits_version` cùng các định danh khác
/// trong `Welcome` để client biết mình đang nói chuyện với phiên nào.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolLimits {
    /// Revision của bộ giới hạn; đổi giá trị nào thì tăng revision (ADR 0008).
    pub revision: u32,
    /// Trần frame gồm envelope.
    pub max_frame_bytes: usize,
    /// Trần payload `Snapshot`.
    pub max_snapshot_bytes: usize,
    /// Trần payload `InputBatch`.
    pub max_input_batch_bytes: usize,
    /// Số người chơi tối đa.
    pub max_players: usize,
    /// Số mục ack tối đa.
    pub max_acks: usize,
    /// Số lệnh tối đa trong một batch.
    pub max_commands: usize,
    /// Số người tối đa trong `grace` của một bom.
    pub max_grace: usize,
    /// Số bom tối đa.
    pub max_bombs: usize,
    /// Số ô lửa tối đa.
    pub max_flames: usize,
    /// Số item trên nền tối đa.
    pub max_items: usize,
    /// Số item giấu tối đa.
    pub max_hidden: usize,
    /// Số ô lưới tối đa.
    pub max_cells: usize,
    /// Trần độ dài `username` trong `Auth` (ADR 0009 Addendum A §A.2).
    pub max_username_bytes: usize,
    /// Trần độ dài `password` trong `Auth` (ADR 0009 Addendum A §A.2).
    pub max_password_bytes: usize,
    /// Số mục `problems` tối đa trong `HealthResponse`.
    pub max_problems: usize,
}

impl Default for ProtocolLimits {
    /// Miền V1 (ADR 0009 §2, §4.2).
    fn default() -> Self {
        Self::v1()
    }
}

impl ProtocolLimits {
    /// Miền V1: `revision = 1`.
    ///
    /// Revision giữ nguyên `1` khi thêm giá trị mới theo ADR 0009 Addendum A: đây là thay đổi
    /// **additive** (ADR 0008), không giá trị cũ nào bị đổi, và bộ giới hạn V1 chưa phát hành.
    /// Đổi một giá trị đang có thì tăng revision.
    pub const fn v1() -> Self {
        Self {
            revision: 1,
            max_frame_bytes: MAX_FRAME_BYTES,
            max_snapshot_bytes: MAX_SNAPSHOT_BYTES,
            max_input_batch_bytes: MAX_INPUT_BATCH_BYTES,
            max_players: MAX_PLAYERS,
            max_acks: MAX_ACKS,
            max_commands: MAX_COMMANDS,
            max_grace: MAX_GRACE,
            max_bombs: MAX_BOMBS,
            max_flames: MAX_FLAMES,
            max_items: MAX_ITEMS,
            max_hidden: MAX_HIDDEN,
            max_cells: MAX_CELLS,
            max_username_bytes: MAX_USERNAME_BYTES,
            max_password_bytes: MAX_PASSWORD_BYTES,
            max_problems: MAX_PROBLEMS,
        }
    }

    /// Trần payload của một loại message.
    pub fn max_payload_bytes(&self, kind: crate::kind::MessageKind) -> usize {
        use crate::kind::MessageKind;
        match kind {
            MessageKind::Snapshot => self.max_snapshot_bytes,
            MessageKind::InputBatch => self.max_input_batch_bytes,
            _ => self.max_frame_bytes.saturating_sub(ENVELOPE_BYTES),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_limits_match_adr_table() {
        let l = ProtocolLimits::v1();
        assert_eq!(l.revision, 1);
        assert_eq!(l.max_frame_bytes, 65_536);
        assert_eq!(l.max_snapshot_bytes, 16_384);
        assert_eq!(l.max_input_batch_bytes, 1_024);
        assert_eq!(l.max_players, 4);
        assert_eq!(l.max_acks, 4);
        assert_eq!(l.max_commands, 8);
        assert_eq!(l.max_grace, 4);
        assert_eq!(l.max_bombs, 40);
        assert_eq!(l.max_flames, 195);
        assert_eq!(l.max_items, 195);
        assert_eq!(l.max_hidden, 195);
        assert_eq!(l.max_cells, 195);
        assert_eq!(l.max_username_bytes, 64);
        assert_eq!(l.max_password_bytes, 256);
        assert_eq!(l.max_problems, 32);
    }

    #[test]
    fn frame_limit_leaves_room_for_envelope() {
        let l = ProtocolLimits::v1();
        assert_eq!(l.max_payload_bytes(crate::kind::MessageKind::Hello), 65_520);
        assert_eq!(
            l.max_payload_bytes(crate::kind::MessageKind::Snapshot),
            16_384
        );
        assert_eq!(
            l.max_payload_bytes(crate::kind::MessageKind::InputBatch),
            1_024
        );
    }

    #[test]
    fn wire_ceilings_are_not_below_session_limits() {
        // Trần định dạng phải chặt hơn miền phiên, nếu không miền phiên không bao giờ đạt.
        let l = ProtocolLimits::v1();
        assert!(WIRE_MAX_PLAYERS >= l.max_players);
        assert!(WIRE_MAX_ACKS >= l.max_acks);
        assert!(WIRE_MAX_COMMANDS >= l.max_commands);
        assert!(WIRE_MAX_GRACE >= l.max_grace);
        assert!(WIRE_MAX_BOMBS >= l.max_bombs);
        assert!(WIRE_MAX_FLAMES >= l.max_flames);
        assert!(WIRE_MAX_ITEMS >= l.max_items);
        assert!(WIRE_MAX_HIDDEN >= l.max_hidden);
        assert!(WIRE_MAX_PROBLEMS >= l.max_problems);
    }

    /// Trần định dạng của chuỗi phải **rộng hơn** biên V1: chuỗi vượt biên vẫn phải decode được
    /// rồi bị `check` từ chối bằng lỗi có kiểu, để server trả `AuthFail { InvalidInput }`
    /// (ADR 0009 Addendum A §A.2) thay vì lỗi decode chung chung.
    #[test]
    fn string_wire_ceilings_are_wider_than_session_limits() {
        let l = ProtocolLimits::v1();
        assert!(WIRE_MAX_USERNAME_BYTES > l.max_username_bytes);
        assert!(WIRE_MAX_PASSWORD_BYTES > l.max_password_bytes);
    }
}
