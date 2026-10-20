//! Sảnh/phòng/chat/loading/result phía client (ADR 0009 Addendum B).
//!
//! Module này giữ **trạng thái** luồng phòng của client và **kiểm** fingerprint của round.
//! Việc đóng gói byte thuộc [`super::session`]: module này không biết transport, không đọc
//! đồng hồ và không biết luật chơi.
//!
//! Ranh giới quyền quyết (ADR 0005, SCOPE_V1 §3):
//!
//! - Client **không** mang định danh người chơi trong lệnh phòng: server bind `player` từ
//!   phiên đã `AuthOk` (ADR 0009 Addendum B §B.2).
//! - Client **không** tự kết luận thắng/thua: kết quả do server quyết trong `RoundResult`.
//! - Client **không** chơi trận lệch rules/content: `RoundStart` mang fingerprint, client kiểm
//!   trước khi gửi `LoadingAck`; lệch thì báo lỗi rõ ràng và **không** ack.
//! - Request ID của lệnh phòng tăng dần; gửi lại **cùng** ID thì server dedup, không áp hai lần.

use std::collections::VecDeque;
use std::fmt;

use boom_content::map::Terrain;
use boom_core::{Mode, Rules, World};
use boom_proto::{
    LobbyList, RoomAction, RoomCharacter, RoomChat, RoomChatMessage, RoomCommand, RoomPhase,
    RoomRejected, RoomState, RoundMember, RoundResult, RoundStart,
};

/// Số tin chat giữ lại phía client.
///
/// **Tham số vận hành** (design), không phải luật chơi: khung chat chỉ cần vài tin gần nhất;
/// V1 không lưu chat trên server (ADR 0011 §7).
const CHAT_HISTORY: usize = 64;

/// Lỗi của luồng phòng phía client.
///
/// Lỗi này **không** phải lỗi giao thức: nó là "client không chơi được trận này" (lệch rules,
/// lệch content, thiếu map). Client báo lỗi rõ ràng thay vì chơi sai (AGENTS §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    /// Profile rules của round khác profile client đang dùng.
    ProfileMismatch {
        /// Profile client đang dùng.
        expected: String,
        /// Profile server khai trong `RoundStart`.
        got: String,
    },
    /// Hash rules của round khác rules client đang dùng.
    RulesMismatch {
        /// Hash rules client đang dùng.
        expected: u64,
        /// Hash rules server khai trong `RoundStart`.
        got: u64,
    },
    /// Hash content của round khác content client đã khai ở handshake.
    ContentMismatch {
        /// Hash content client đã khai.
        expected: [u8; 32],
        /// Hash content server khai trong `RoundStart`.
        got: [u8; 32],
    },
    /// Mode của round khác mode client đang chạy.
    ModeMismatch {
        /// Mode client đang chạy.
        expected: String,
        /// Mode server khai trong `RoundStart`.
        got: String,
    },
    /// Client không có trong roster của round.
    NotInRoster {
        /// ID round.
        round_id: u64,
        /// Slot của client.
        slot: u8,
    },
    /// Client chưa có nội dung map mà round yêu cầu.
    MapMissing {
        /// Tên map của round.
        map: String,
    },
    /// Dựng `World` cho round thất bại.
    World {
        /// Chi tiết lỗi.
        detail: String,
    },
}

impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileMismatch { expected, got } => {
                write!(f, "profile rules của round khác: client {expected}, server {got}")
            }
            Self::RulesMismatch { expected, got } => {
                write!(f, "hash rules của round khác: client {expected}, server {got}")
            }
            Self::ContentMismatch { expected, got } => {
                write!(
                    f,
                    "hash content của round khác: client {}, server {}",
                    hex(expected),
                    hex(got)
                )
            }
            Self::ModeMismatch { expected, got } => {
                write!(f, "mode của round khác: client {expected}, server {got}")
            }
            Self::NotInRoster { round_id, slot } => {
                write!(f, "client (slot {slot}) không có trong roster của round {round_id}")
            }
            Self::MapMissing { map } => {
                write!(f, "client chưa có nội dung map {map} của round")
            }
            Self::World { detail } => write!(f, "không dựng được world cho round: {detail}"),
        }
    }
}

impl std::error::Error for RoomError {}

/// In hash content dạng hex để báo lỗi (32 byte).
fn hex(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Nội dung cục bộ của client cho một round: map + địa hình.
///
/// V1 chỉ có **một** map (`FxMap/Map/Village00`, SCOPE_V1 §1) nên client mang địa hình của map
/// nó có. `RoundStart.map` lệch thì client không chơi (xem [`RoomError::MapMissing`]).
#[derive(Clone, Debug)]
pub struct LocalContent {
    /// Tên map (khớp `RoundStart.map`).
    pub map: String,
    /// Địa hình để dựng `World` cho round.
    pub terrain: Terrain,
}

/// Fingerprint của client, dùng để kiểm `RoundStart` trước khi ack.
///
/// Đây là giá trị **của client**: rules client đang chạy và content client đã khai ở
/// handshake (`Hello`). Handshake đã chứng minh chúng khớp server (ADR 0009 §1), nên kiểm ở
/// `RoundStart` là kiểm lại trước khi vào trận, không phải kiểm mới.
#[derive(Clone, Debug)]
pub struct RoundExpectation {
    /// Tên profile rules của client.
    pub profile_id: String,
    /// Hash định danh profile rules của client.
    pub rules_hash: u64,
    /// Hash manifest content của client.
    pub content_sha256: [u8; 32],
    /// Tên mode client đang chạy.
    pub mode: String,
}

/// Thông tin round đã kiểm fingerprint.
///
/// `#[allow(dead_code)]`: đây là **dữ liệu hợp đồng** cho tầng UI (#103) và cho QA (đối chiếu
/// mọi client cùng seed/map/mode/fingerprint); một số trường hiện chỉ đọc ở test.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RoundPlan {
    /// ID round (public): khớp `Snapshot.round_id` của lượt đó.
    pub round_id: u64,
    /// Seed của round.
    pub seed: u64,
    /// Tên map của round.
    pub map: String,
    /// Tên mode của round.
    pub mode: String,
    /// Tên profile rules của round.
    pub rules_profile_id: String,
    /// Hash rules của round.
    pub rules_hash: u64,
    /// Hash content của round.
    pub content_sha256: [u8; 32],
    /// Roster đã chốt lúc start.
    pub roster: Vec<RoundMember>,
    /// Tick bắt đầu.
    pub start_tick: u32,
}

impl RoundPlan {
    /// Thành viên của round có slot `slot`.
    #[allow(dead_code)]
    pub fn member(&self, slot: u8) -> Option<&RoundMember> {
        self.roster.iter().find(|member| member.slot == slot)
    }
}

/// Trạng thái luồng phòng phía client.
///
/// Chỉ giữ **dữ liệu server đã xác nhận** (hoặc lỗi kiểm fingerprint); client không tự suy
/// diễn trạng thái phòng, không tự kết luận thắng/thua.
pub struct RoomFlow {
    /// Slot của chính client: id người chơi trong state trận.
    ///
    /// Server bind slot khi người chơi vào phòng và dùng chính slot đó làm `id` của người chơi
    /// trong state trận (`boom-server` dựng roster từ `member.slot`), nên slot là định danh
    /// mà client dùng được ở cả phòng lẫn trận.
    my_slot: u8,
    /// Request ID tăng dần cho lệnh phòng và chat.
    next_request: u64,
    /// Danh sách phòng sảnh gần nhất.
    lobby: Option<LobbyList>,
    /// Trạng thái phòng gần nhất.
    room: Option<RoomState>,
    /// Chat đã nhận (mới nhất ở cuối).
    chat: VecDeque<RoomChatMessage>,
    /// Round đã bắt đầu (đã kiểm fingerprint).
    round: Option<RoundPlan>,
    /// Kết quả round gần nhất.
    result: Option<RoundResult>,
    /// Lệnh bị từ chối gần nhất.
    rejection: Option<RoomRejected>,
    /// Lỗi kiểm fingerprint gần nhất.
    error: Option<RoomError>,
    /// Round đã gửi `LoadingAck`.
    loading_acked: Option<u64>,
    /// Round đã gửi `ResultAck`.
    result_acked: Option<u64>,
}

impl RoomFlow {
    /// Dựng luồng phòng cho client ở slot `my_slot`.
    pub fn new(my_slot: u8) -> Self {
        Self {
            my_slot,
            next_request: 1,
            lobby: None,
            room: None,
            chat: VecDeque::new(),
            round: None,
            result: None,
            rejection: None,
            error: None,
            loading_acked: None,
            result_acked: None,
        }
    }

    /// Slot của chính client.
    pub fn my_slot(&self) -> u8 {
        self.my_slot
    }

    /// Đặt slot của chính client (server cấp trong `Welcome`).
    pub fn set_my_slot(&mut self, slot: u8) {
        self.my_slot = slot;
    }

    /// Ghi lỗi của luồng phòng để caller hiện ra ngoài.
    pub fn set_error(&mut self, error: RoomError) {
        self.error = Some(error);
    }

    /// Cấp request ID tăng dần cho lệnh phòng/chat kế tiếp.
    ///
    /// ID tăng để server phân biệt lệnh mới với lệnh gửi lại; gửi lại **cùng** ID thì server
    /// dedup (không áp hai lần) — xem [`RoomFlow::command_with_request`].
    pub fn next_request(&mut self) -> u64 {
        let request = self.next_request;
        self.next_request = self.next_request.saturating_add(1);
        request
    }

    /// Dựng lệnh phòng với request ID tăng dần.
    pub fn command(&mut self, action: RoomAction) -> RoomCommand {
        let request = self.next_request();
        RoomCommand { request, action }
    }

    /// Dựng lệnh phòng với request ID **do caller chọn**.
    ///
    /// Dùng để gửi lại đúng lệnh đã gửi (mạng mất frame): server dedup theo request ID nên
    /// lệnh không bị áp hai lần (ADR 0009 Addendum B §B.2).
    pub fn command_with_request(&self, request: u64, action: RoomAction) -> RoomCommand {
        RoomCommand { request, action }
    }

    /// Dựng tin chat với request ID tăng dần.
    ///
    /// `#[allow(dead_code)]`: API gửi chat cho tầng UI (#103 chưa merge nên chưa có caller
    /// trong client này); test end-to-end dùng để kiểm chat chỉ tới thành viên phòng.
    #[allow(dead_code)]
    pub fn chat_message(&mut self, text: &str) -> RoomChat {
        let request = self.next_request();
        RoomChat {
            request,
            text: text.to_owned(),
        }
    }

    /// Nhận `LobbyList` từ server.
    pub fn on_lobby_list(&mut self, list: LobbyList) {
        self.lobby = Some(list);
    }

    /// Nhận `RoomState` từ server.
    pub fn on_room_state(&mut self, state: RoomState) {
        self.room = Some(state);
    }

    /// Nhận chat phòng.
    pub fn on_chat(&mut self, message: RoomChatMessage) {
        if self.chat.len() >= CHAT_HISTORY {
            self.chat.pop_front();
        }
        self.chat.push_back(message);
    }

    /// Nhận `RoomRejected`: lệnh phòng bị từ chối.
    pub fn on_rejected(&mut self, rejected: RoomRejected) {
        self.rejection = Some(rejected);
    }

    /// Nhận `RoundStart`: kiểm fingerprint rồi lưu kế hoạch round.
    ///
    /// Trả về `Err` khi fingerprint lệch: client **không** được ack loading nữa (để loading
    /// barrier của server hủy start thay vì chơi sai nội dung).
    pub fn on_round_start(
        &mut self,
        start: RoundStart,
        expected: &RoundExpectation,
    ) -> Result<RoundPlan, RoomError> {
        if start.rules_profile_id != expected.profile_id {
            return Err(self.fail(RoomError::ProfileMismatch {
                expected: expected.profile_id.clone(),
                got: start.rules_profile_id.clone(),
            }));
        }
        if start.rules_hash != expected.rules_hash {
            return Err(self.fail(RoomError::RulesMismatch {
                expected: expected.rules_hash,
                got: start.rules_hash,
            }));
        }
        if start.content_sha256 != expected.content_sha256 {
            return Err(self.fail(RoomError::ContentMismatch {
                expected: expected.content_sha256,
                got: start.content_sha256,
            }));
        }
        if start.mode != expected.mode {
            return Err(self.fail(RoomError::ModeMismatch {
                expected: expected.mode.clone(),
                got: start.mode.clone(),
            }));
        }
        if start.roster.iter().all(|member| member.slot != self.my_slot) {
            return Err(self.fail(RoomError::NotInRoster {
                round_id: start.round_id,
                slot: self.my_slot,
            }));
        }
        let plan = RoundPlan {
            round_id: start.round_id,
            seed: start.seed,
            map: start.map,
            mode: start.mode,
            rules_profile_id: start.rules_profile_id,
            rules_hash: start.rules_hash,
            content_sha256: start.content_sha256,
            roster: start.roster,
            start_tick: start.start_tick,
        };
        self.error = None;
        self.round = Some(plan.clone());
        Ok(plan)
    }

    /// Nhận `RoundResult`: kết quả do server quyết, client chỉ giữ lại.
    pub fn on_round_result(&mut self, result: RoundResult) {
        self.result = Some(result);
    }

    /// Ghi nhận đã gửi `LoadingAck` cho round `round_id`.
    pub fn mark_loading_acked(&mut self, round_id: u64) {
        self.loading_acked = Some(round_id);
    }

    /// Ghi nhận đã gửi `ResultAck` cho round `round_id`.
    pub fn mark_result_acked(&mut self, round_id: u64) {
        self.result_acked = Some(round_id);
    }

    /// Danh sách phòng sảnh gần nhất.
    pub fn lobby(&self) -> Option<&LobbyList> {
        self.lobby.as_ref()
    }

    /// Trạng thái phòng gần nhất.
    pub fn room(&self) -> Option<&RoomState> {
        self.room.as_ref()
    }

    /// Chat đã nhận (mới nhất ở cuối).
    pub fn chat(&self) -> &VecDeque<RoomChatMessage> {
        &self.chat
    }

    /// Round đã bắt đầu (đã kiểm fingerprint).
    pub fn round(&self) -> Option<&RoundPlan> {
        self.round.as_ref()
    }

    /// Kết quả round gần nhất.
    pub fn result(&self) -> Option<&RoundResult> {
        self.result.as_ref()
    }

    /// Lệnh phòng bị từ chối gần nhất.
    pub fn rejection(&self) -> Option<&RoomRejected> {
        self.rejection.as_ref()
    }

    /// Lỗi kiểm fingerprint gần nhất.
    pub fn error(&self) -> Option<&RoomError> {
        self.error.as_ref()
    }

    /// Round đã gửi `LoadingAck` (nếu có).
    pub fn loading_acked(&self) -> Option<u64> {
        self.loading_acked
    }

    /// Round đã gửi `ResultAck` (nếu có).
    pub fn result_acked(&self) -> Option<u64> {
        self.result_acked
    }

    /// Thành viên phòng tương ứng với chính client (theo slot).
    pub fn my_member(&self) -> Option<&boom_proto::RoomMember> {
        let state = self.room.as_ref()?;
        state
            .members
            .iter()
            .find(|member| member.slot == self.my_slot)
    }

    /// `PlayerId` của chính client trong phòng (server bind, ADR 0009 Addendum B).
    ///
    /// Chỉ có được khi client đã nhận `RoomState` và tìm thấy thành viên có slot của mình.
    pub fn my_player(&self) -> Option<u64> {
        self.my_member().map(|member| member.player)
    }

    /// Client có phải host của phòng hiện tại không.
    ///
    /// Host là `RoomState.host`; client so với `PlayerId` của chính mình (xem [`RoomFlow::my_player`]).
    pub fn is_host(&self) -> bool {
        match (self.room.as_ref(), self.my_player()) {
            (Some(state), Some(player)) => state.host == player,
            _ => false,
        }
    }

    /// Ghi lỗi và trả về lỗi đó (để caller dùng `?`).
    fn fail(&mut self, error: RoomError) -> RoomError {
        self.error = Some(error.clone());
        error
    }
}

/// Dựng `World` cho round từ nội dung cục bộ của client.
///
/// `Ok(None)` = client chưa có nội dung map đó: caller dùng đường dựng world từ snapshot
/// (server là authority về lưới ô) và **phải** ghi nhận việc này, không được coi là đã kiểm
/// content. `Err` = client có nội dung nhưng không khớp map của round, hoặc dựng world lỗi.
pub fn world_for_round(
    plan: &RoundPlan,
    local: Option<&LocalContent>,
    rules: &Rules,
    mode: &'static dyn Mode,
) -> Result<Option<World>, RoomError> {
    let Some(local) = local else {
        return Ok(None);
    };
    if local.map != plan.map {
        return Err(RoomError::MapMissing {
            map: plan.map.clone(),
        });
    }
    let world = World::new(local.terrain.clone(), rules.clone(), mode).map_err(|error| {
        RoomError::World {
            detail: error.to_string(),
        }
    })?;
    Ok(Some(world))
}

/// Loại sự kiện phòng mà luồng tự động phản ứng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomEvent {
    /// Vừa nhận `LobbyList`.
    LobbyList,
    /// Vừa nhận `RoomState`.
    RoomState,
    /// Vừa nhận `RoundStart`.
    RoundStart,
    /// Vừa nhận `RoundResult`.
    RoundResult,
    /// Vừa nhận `RoomRejected`.
    Rejected,
}

/// Hành động mà luồng tự động đề nghị.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutoAction {
    /// Xin danh sách phòng sảnh.
    RequestLobby,
    /// Tạo phòng mới.
    CreateRoom,
    /// Vào phòng có sẵn.
    JoinRoom {
        /// ID phòng.
        room: u64,
    },
    /// Bật ready.
    SetReady,
    /// Bắt đầu lượt (chỉ host được phép; server kiểm lại).
    Start,
}

impl AutoAction {
    /// Lệnh phòng tương ứng trên dây; `None` với [`AutoAction::RequestLobby`] vì đó là
    /// message riêng (`LobbyListRequest`), không phải `RoomCommand`.
    pub fn room_action(&self, character: RoomCharacter) -> Option<RoomAction> {
        match self {
            Self::RequestLobby => None,
            Self::CreateRoom => Some(RoomAction::Create { character }),
            Self::JoinRoom { room } => Some(RoomAction::Join {
                room: *room,
                character,
            }),
            Self::SetReady => Some(RoomAction::SetReady { ready: true }),
            Self::Start => Some(RoomAction::Start),
        }
    }
}

/// Luồng tự động của client: vào phòng, ready, start, ack loading, ack result.
///
/// Dùng cho QA/test (`--auto-ready`): **mọi** bước là lệnh phòng thật trên dây, không giả lập
/// state phía client. Luồng không tự quyết kết quả: `RoundResult` của server mới là kết quả.
#[derive(Clone, Debug)]
pub struct AutoRoomFlow {
    /// Nhân vật chọn khi tạo/vào phòng.
    character: RoomCharacter,
    /// Số người tối thiểu trong phòng để host start.
    ///
    /// Đây là **tham số kịch bản QA**, không phải luật chơi: server vẫn kiểm policy thật
    /// (`min_players_to_start`) và từ chối `Start` khi chưa đủ người. Client dùng tham số này
    /// để *chờ* đủ người rồi mới start — ví dụ QA 4 client thì chờ 4 người vào phòng.
    min_players: usize,
    /// Hành động đang chờ server xác nhận.
    pending: Option<AutoAction>,
    /// Chữ ký của lần start gần nhất: `(phòng, số thành viên, tất cả ready)`.
    ///
    /// Dùng để **không** gửi lại `Start` khi server đã từ chối và trạng thái phòng không đổi
    /// (ví dụ `NotEnoughPlayers`): mỗi trạng thái mới mới thử lại.
    last_start: Option<(u64, usize, bool)>,
    /// Số lần xin lại danh sách phòng sau khi bị từ chối.
    ///
    /// **Tham số vận hành** (design): tránh lặp vô hạn khi lệnh phòng bị từ chối liên tục.
    lobby_retries: u32,
}

/// Số lần xin lại danh sách phòng tối đa (tham số vận hành, design).
const LOBBY_RETRIES_MAX: u32 = 8;

impl AutoRoomFlow {
    /// Dựng luồng tự động chờ tối thiểu `min_players` người trong phòng rồi mới start.
    pub fn with_min_players(character: RoomCharacter, min_players: usize) -> Self {
        Self {
            character,
            min_players,
            pending: None,
            last_start: None,
            lobby_retries: 0,
        }
    }

    /// Nhân vật luồng dùng khi tạo/vào phòng.
    pub fn character(&self) -> RoomCharacter {
        self.character
    }

    /// Xử lý sự kiện phòng vừa tới và trả về hành động cần làm (nếu có).
    pub fn on_event(&mut self, event: RoomEvent, flow: &RoomFlow) -> Option<AutoAction> {
        // Sự kiện này xác nhận lệnh đang chờ (nếu có) nên không cần gửi lại nữa.
        if matches!(
            event,
            RoomEvent::LobbyList | RoomEvent::RoomState | RoomEvent::RoundStart
        ) {
            self.pending = None;
        }
        let action = self.decide(event, flow);
        if let Some(action) = &action {
            self.pending = Some(action.clone());
        }
        action
    }

    /// Hành động đang chờ server xác nhận (chưa có phản hồi).
    ///
    /// Caller dùng để gửi lại **cùng** lệnh khi mạng mất frame: server dedup theo request ID
    /// nên lệnh không bị áp hai lần.
    pub fn pending(&self) -> Option<&AutoAction> {
        self.pending.as_ref()
    }

    /// Quyết định hành động theo sự kiện và trạng thái phòng hiện tại.
    fn decide(&mut self, event: RoomEvent, flow: &RoomFlow) -> Option<AutoAction> {
        match event {
            RoomEvent::LobbyList => {
                // Đã ở trong phòng thì không xin danh sách nữa.
                if flow.room().is_some() {
                    return None;
                }
                match flow.lobby().and_then(|list| list.rooms.first()) {
                    Some(room) => Some(AutoAction::JoinRoom {
                        room: room.room_id,
                    }),
                    None => Some(AutoAction::CreateRoom),
                }
            }
            RoomEvent::RoomState => {
                let state = flow.room()?;
                if state.phase != RoomPhase::Waiting {
                    return None;
                }
                let me = state
                    .members
                    .iter()
                    .find(|member| member.slot == flow.my_slot())?;
                if !me.ready {
                    return Some(AutoAction::SetReady);
                }
                let all_ready = state.members.iter().all(|member| member.ready);
                // Chỉ host mới start; server kiểm lại quyền (RoomRejectReason::NotHost).
                if flow.is_host() && all_ready && state.members.len() >= self.min_players {
                    let signature = (state.room_id, state.members.len(), all_ready);
                    if self.last_start != Some(signature) {
                        self.last_start = Some(signature);
                        return Some(AutoAction::Start);
                    }
                }
                None
            }
            RoomEvent::RoundStart => None,
            RoomEvent::RoundResult => {
                // Về phòng: server gửi `RoomState` mới, luồng ready lại cho vòng sau.
                self.last_start = None;
                None
            }
            RoomEvent::Rejected => {
                // Chưa vào phòng: xin lại danh sách để thử phòng khác (có giới hạn số lần).
                if flow.room().is_none() && self.lobby_retries < LOBBY_RETRIES_MAX {
                    self.lobby_retries += 1;
                    return Some(AutoAction::RequestLobby);
                }
                None
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use boom_proto::{RoomMember, RoundMember};

    /// Fingerprint của client trong test.
    fn expectation() -> RoundExpectation {
        RoundExpectation {
            profile_id: "skeleton-test".into(),
            rules_hash: 7,
            content_sha256: [1; 32],
            mode: "battle_ffa".into(),
        }
    }

    /// `RoundStart` hợp lệ (khớp fingerprint của test).
    fn round_start() -> RoundStart {
        RoundStart {
            round_id: 3,
            seed: 99,
            rules_profile_id: "skeleton-test".into(),
            rules_hash: 7,
            content_sha256: [1; 32],
            map: "Village00".into(),
            mode: "battle_ffa".into(),
            roster: vec![RoundMember {
                player: 1,
                slot: 0,
                character: RoomCharacter::C00,
            }],
            start_tick: 0,
        }
    }

    #[test]
    fn round_start_khop_fingerprint_thi_nhan_round() {
        let mut flow = RoomFlow::new(0);
        let plan = flow
            .on_round_start(round_start(), &expectation())
            .expect("fingerprint phải khớp");
        assert_eq!(plan.round_id, 3);
        assert_eq!(plan.seed, 99);
        assert_eq!(plan.map, "Village00");
        assert_eq!(plan.mode, "battle_ffa");
        assert_eq!(flow.round().map(|plan| plan.round_id), Some(3));
        assert!(flow.error().is_none());
    }

    #[test]
    fn round_start_lech_rules_thi_bao_lo_va_khong_nhan_round() {
        let mut flow = RoomFlow::new(0);
        let mut start = round_start();
        start.rules_hash = 8;
        let error = flow
            .on_round_start(start, &expectation())
            .expect_err("hash rules lệch phải bị từ chối");
        assert_eq!(
            error,
            RoomError::RulesMismatch {
                expected: 7,
                got: 8
            }
        );
        assert!(flow.round().is_none(), "lượt lệch rules không được nhận");
        assert!(flow.error().is_some(), "lỗi phải được báo ra ngoài");
    }

    #[test]
    fn round_start_lech_content_thi_bao_lo() {
        let mut flow = RoomFlow::new(0);
        let mut start = round_start();
        start.content_sha256 = [2; 32];
        let error = flow
            .on_round_start(start, &expectation())
            .expect_err("hash content lệch phải bị từ chối");
        assert!(matches!(error, RoomError::ContentMismatch { .. }));
    }

    #[test]
    fn round_start_lech_mode_thi_bao_lo() {
        let mut flow = RoomFlow::new(0);
        let mut start = round_start();
        start.mode = "battle_team".into();
        let error = flow
            .on_round_start(start, &expectation())
            .expect_err("mode lệch phải bị từ chối");
        assert!(matches!(error, RoomError::ModeMismatch { .. }));
    }

    #[test]
    fn client_khong_trong_roster_thi_bao_lo() {
        let mut flow = RoomFlow::new(5);
        let error = flow
            .on_round_start(round_start(), &expectation())
            .expect_err("client không có trong roster phải bị từ chối");
        assert_eq!(
            error,
            RoomError::NotInRoster {
                round_id: 3,
                slot: 5
            }
        );
    }

    #[test]
    fn request_id_tang_dan_va_gui_lai_duoc_dung_id() {
        let mut flow = RoomFlow::new(0);
        let first = flow.command(RoomAction::Start);
        let second = flow.command(RoomAction::SetReady { ready: true });
        assert!(second.request > first.request, "request ID phải tăng dần");
        // Gửi lại lệnh đã gửi: cùng request ID để server dedup.
        let again = flow.command_with_request(first.request, RoomAction::Start);
        assert_eq!(again, first);
    }

    #[test]
    fn chat_giu_toi_phan_tram() {
        let mut flow = RoomFlow::new(0);
        for index in 0..(CHAT_HISTORY + 5) {
            flow.on_chat(RoomChatMessage {
                room_id: 1,
                from: 2,
                from_name: "kia".into(),
                text: format!("tin {index}"),
            });
        }
        let chat = flow.chat();
        assert_eq!(chat.len(), CHAT_HISTORY);
        assert_eq!(
            chat.back().map(|message| message.text.as_str()),
            Some(format!("tin {}", CHAT_HISTORY + 4).as_str())
        );
    }

    #[test]
    fn is_host_dung_theo_host_cua_server() {
        let mut flow = RoomFlow::new(0);
        assert!(!flow.is_host(), "chưa có phòng thì không phải host");
        flow.on_room_state(RoomState {
            room_id: 1,
            host: 9,
            phase: RoomPhase::Waiting,
            members: vec![RoomMember {
                player: 9,
                slot: 0,
                character: RoomCharacter::C00,
                ready: true,
            }],
            round: None,
        });
        assert!(flow.is_host(), "thành viên slot 0 là host theo server");
        assert_eq!(flow.my_player(), Some(9));
    }
}
