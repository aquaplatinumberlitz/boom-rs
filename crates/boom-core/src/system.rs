use crate::event::EventSink;
use crate::scratch::TickScratch;
use crate::sim::{PlayerInput, World};
use crate::state::SimulationState;

/// Một bước xử lý trong tick. Mode khai báo danh sách và thứ tự system; thứ tự là một phần
/// của luật (ARCHITECTURE §5), đổi thứ tự là đổi hành vi và phải cập nhật golden replay.
///
/// System chỉ đọc `World` (bất biến) và sửa `SimulationState` theo ma trận quyền ghi của
/// contracts-r1; dữ liệu chỉ sống trong tick (kênh blast) truyền qua [`TickScratch`], không
/// lưu vào state, nên không vào state hash và không vào snapshot.
pub trait System: Sync {
    /// Tên ổn định, dùng trong log và báo lỗi.
    fn name(&self) -> &'static str;

    /// Gọi một lần khi tạo lượt, sau khi đặt người chơi, theo thứ tự mode.
    fn on_round_start(&self, _world: &World, _state: &mut SimulationState) {}

    /// Chạy system trong tick hiện tại.
    ///
    /// `scratch` là dữ liệu tạm của tick: `explosion` ghi kênh blast, `blocks` đọc rồi xóa.
    /// System không sở hữu trạng thái riêng ngoài state và scratch (ADR 0008).
    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
        scratch: &mut TickScratch,
    );
}
