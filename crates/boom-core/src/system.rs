use crate::event::EventSink;
use crate::sim::{PlayerInput, World};
use crate::state::SimulationState;

/// Một bước xử lý trong tick. Mode khai báo danh sách và thứ tự system; thứ tự là một phần
/// của luật (ARCHITECTURE §5), đổi thứ tự là đổi hành vi và phải cập nhật golden replay.
///
/// System chỉ đọc `World` (bất biến) và sửa `SimulationState` theo ma trận quyền ghi của
/// contracts-r1; không giữ trạng thái riêng ngoài state, để state hash bao trọn mọi thứ.
pub trait System: Sync {
    /// Tên ổn định, dùng trong log và báo lỗi.
    fn name(&self) -> &'static str;

    /// Gọi một lần khi tạo lượt, sau khi đặt người chơi, theo thứ tự mode.
    fn on_round_start(&self, _world: &World, _state: &mut SimulationState) {}

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
    );
}
