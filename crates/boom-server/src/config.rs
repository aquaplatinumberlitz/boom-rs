//! Cấu hình authority. Không chứa luật chơi; các timeout vận hành là policy SCOPE_V1 §3
//! và các ngưỡng input là tham số giao thức ADR 0009 (design), có tên và nguồn.

use boom_core::{CharacterStats, Rules, Tick, ms_to_ticks};
use boom_room::RoomPolicy;

/// Policy vòng đời phòng/trận của V1. Giá trị mặc định lấy từ SCOPE_V1 §3; caller có thể
/// thay bằng cấu hình khác khi test hoặc khi scope revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyConfig {
    /// Loading barrier tối đa trước khi hủy start (SCOPE_V1 §3).
    pub loading_timeout_ms: u32,
    /// Thời gian mất kết nối trước khi loại một lần (SCOPE_V1 §3).
    pub disconnect_timeout_ms: u32,
    /// Thời gian chờ ack kết quả rồi về phòng (SCOPE_V1 §3).
    pub results_timeout_ms: u32,
    /// Số slot và ngưỡng start; policy phòng do server nạp, không phải hằng số của boom-room.
    pub room_policy: RoomPolicy,
}

impl PolicyConfig {
    /// Policy V1 theo SCOPE_V1 §3. Đây là policy dự án (DESIGN), không phải luật client gốc.
    pub fn scope_v1() -> Self {
        Self {
            loading_timeout_ms: 15_000,
            disconnect_timeout_ms: 10_000,
            results_timeout_ms: 5_000,
            room_policy: RoomPolicy::new(4, 2).expect("policy SCOPE_V1 hợp lệ"),
        }
    }
}

/// Giới hạn scheduler input theo ADR 0009 §3.5. Đây là tham số giao thức, không phải luật chơi;
/// đổi giá trị phải kèm revision ADR/ProtocolLimits, không sửa lén trong code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputLimits {
    /// Số lệnh tối đa trong một batch (ADR 0009).
    pub max_batch_commands: u8,
    /// Số sequence tối đa được buffer vượt `applied_seq` (ADR 0009).
    pub max_gap: u32,
    /// Số batch tối đa mỗi giây cho một người (ADR 0009).
    pub max_batches_per_second: u32,
    /// Lệnh cũ hơn `server_tick - late` bị retire, không áp dụng (ADR 0009).
    pub late_ms: u32,
    /// Lệnh xa hơn `server_tick + lead` bị từ chối (ADR 0009).
    pub lead_ms: u32,
    /// Gap timeout chỉ retire sequence thiếu (ADR 0009).
    pub gap_timeout_ms: u32,
    /// Không có input mới trong khoảng này thì hướng về neutral (ADR 0009).
    pub stale_ms: u32,
}

impl InputLimits {
    /// Giá trị V1 trong ADR 0009 §3.5 (design, chưa có trace server gốc).
    pub fn adr_0009() -> Self {
        Self {
            max_batch_commands: 8,
            max_gap: 16,
            max_batches_per_second: 120,
            late_ms: 100,
            lead_ms: 200,
            gap_timeout_ms: 100,
            stale_ms: 150,
        }
    }
}

/// Cấu hình đầy đủ của authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub policy: PolicyConfig,
    pub input: InputLimits,
}

impl ServerConfig {
    pub fn v1() -> Self {
        Self {
            policy: PolicyConfig::scope_v1(),
            input: InputLimits::adr_0009(),
        }
    }
}

/// Ngưỡng input đã đổi sang tick theo `Rules::ticks_per_second` (ADR 0009 §3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputTicks {
    pub(crate) max_gap: u32,
    pub(crate) max_batch_commands: u8,
    pub(crate) max_batches_per_second: u32,
    pub(crate) late: Tick,
    pub(crate) lead: Tick,
    pub(crate) gap_timeout: Tick,
    pub(crate) stale: Tick,
}

impl InputTicks {
    pub(crate) fn from_limits(limits: InputLimits, rules: &Rules) -> Self {
        Self {
            max_gap: limits.max_gap,
            max_batch_commands: limits.max_batch_commands,
            max_batches_per_second: limits.max_batches_per_second,
            late: ms_to_ticks(limits.late_ms, rules.ticks_per_second),
            lead: ms_to_ticks(limits.lead_ms, rules.ticks_per_second),
            gap_timeout: ms_to_ticks(limits.gap_timeout_ms, rules.ticks_per_second),
            stale: ms_to_ticks(limits.stale_ms, rules.ticks_per_second),
        }
    }
}

/// Chỉ số khởi đầu của hai template C00/C01, do caller nạp từ content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterStatsTable {
    pub c00: CharacterStats,
    pub c01: CharacterStats,
}
