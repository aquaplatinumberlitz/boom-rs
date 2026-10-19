use crate::event::EventSink;
use crate::sim::{PlayerInput, SimulationState, World};

/// Một bước xử lý trong tick. Mode khai báo danh sách và thứ tự system; thứ tự là một phần
/// của luật (ARCHITECTURE §5), đổi thứ tự là đổi hành vi và phải cập nhật golden replay.
///
/// System chỉ đọc `World` (bất biến) và sửa `SimulationState`; không giữ trạng thái riêng,
/// để state hash luôn bao trọn mọi thứ ảnh hưởng tick sau.
pub trait System: Sync {
    /// Tên ổn định, dùng trong log và báo lỗi.
    fn name(&self) -> &'static str;

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
    );
}
