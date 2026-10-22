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
    RoomRejectReason, RoomRejected, RoomState, RoundMember, RoundResult, RoundStart,
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
        /// Định danh người chơi của client (`AuthOk.account_id`).
        ///
        /// `None` = client chưa `AuthOk` nên chưa biết mình là ai ⇒ chưa thể tra roster.
        player: Option<u64>,
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
                write!(
                    f,
                    "profile rules của round khác: client {expected}, server {got}"
                )
            }
            Self::RulesMismatch { expected, got } => {
                write!(
                    f,
                    "hash rules của round khác: client {expected}, server {got}"
                )
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
            Self::NotInRoster { round_id, player } => match player {
                Some(player) => write!(
                    f,
                    "client (player {player}) không có trong roster của round {round_id}"
                ),
                None => write!(
                    f,
                    "client chưa đăng nhập nên chưa biết mình có trong roster của round {round_id}"
                ),
            },
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

/// Kết quả xử lý `RoundStart`.
///
/// `RoundStart` là **lifecycle message** của round: nó mang slot trận của client và mở
/// lượt. Mạng có thể đưa tới trễ hoặc lặp lại, nên client phải phân biệt ba trường hợp
/// **trước mọi side effect** (đổi `my_slot`, đổi kế hoạch round, gửi `LoadingAck`):
///
/// - [`RoundStartOutcome::Started`]: round mới ⇒ dựng kế hoạch và ack loading.
/// - [`RoundStartOutcome::Duplicate`]: lặp lại round đang chơi ⇒ **idempotent**, trạng thái
///   không đổi (ack đã gửi rồi; nếu ack bị mất thì nhịp gửi lại ack lo).
/// - [`RoundStartOutcome::Stale`]: thuộc round **cũ** ⇒ bỏ, không đụng gì. Bỏ qua thì
///   kế hoạch round không bị lùi (nếu lùi thì snapshot của round mới bị bỏ theo điều kiện
///   so với kế hoạch, và input đi kèm slot của round cũ).
#[derive(Clone, Debug)]
pub enum RoundStartOutcome {
    /// Round mới: kế hoạch đã dựng, client phải ack loading.
    Started(RoundPlan),
    /// `RoundStart` lặp lại của round đang chơi: không đổi trạng thái.
    Duplicate,
    /// `RoundStart` của round cũ: bỏ, không có side effect.
    Stale,
}

/// Ack mà client đã gửi nhưng chưa thấy bằng chứng server đã nhận.
///
/// Mạng có thể mất frame, mà protocol V1 **không** có kind riêng để xác nhận ack
/// (ADR 0009 Addendum B §B.2: `LoadingAck` và `ResultAck` là idempotent, gửi lại không đổi
/// ý nghĩa). Vì vậy client gửi lại ack theo nhịp cho tới khi có bằng chứng:
///
/// - `LoadingAck`: bằng chứng là **snapshot đầu tiên** của round đó (server chỉ gửi snapshot
///   khi lượt đã bắt đầu, tức là barrier đã qua).
/// - `ResultAck`: bằng chứng là `RoomState` pha `Waiting` (server đã đưa phòng về chờ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingAck {
    /// `LoadingAck` của round; chờ snapshot đầu tiên của round đó.
    Loading(u64),
    /// `ResultAck` của round; chờ phòng quay lại `Waiting`.
    Result(u64),
}

/// Trạng thái luồng phòng phía client.
///
/// Chỉ giữ **dữ liệu server đã xác nhận** (hoặc lỗi kiểm fingerprint); client không tự suy
/// diễn trạng thái phòng, không tự kết luận thắng/thua.
pub struct RoomFlow {
    /// Định danh người chơi trong phòng/round: `AuthOk.account_id`.
    ///
    /// Mọi trường `player` trên dây (`RoomState`, `RoundStart.roster`, `RoomChatMessage`,
    /// `RoundResult`) mang giá trị này: server bind từ phiên đã `AuthOk`, không tin client khai
    /// (ADR 0009 Addendum B §B.2).
    my_player: Option<u64>,
    /// Slot **trận** của chính client.
    ///
    /// Slot trận là `slot` của mục `RoundStart.roster` có `player == my_player`
    /// (ADR 0009 Addendum B §B.2): server bind slot khi người chơi vào phòng và lấy chính
    /// slot đó làm `id` của người chơi trong state trận (`boom-server` dựng roster từ
    /// `member.slot`).
    ///
    /// **Không** lấy từ `Welcome.player_id`: đó là định danh **kết nối**, không phải slot
    /// trận. Cũng không lấy từ `RoomState.members`: đó là slot **phòng**, còn slot trận là
    /// slot trong roster đã chốt lúc `Start`.
    my_slot: Option<u8>,
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
    /// Ack đang chờ bằng chứng server đã nhận (client gửi lại theo nhịp).
    pending_ack: Option<PendingAck>,
}

impl RoomFlow {
    /// Dựng luồng phòng cho client (chưa biết định danh người chơi cho tới `AuthOk`).
    pub fn new() -> Self {
        Self {
            my_player: None,
            my_slot: None,
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
            pending_ack: None,
        }
    }

    /// Đặt định danh người chơi trong phòng/round (`AuthOk.account_id`).
    ///
    /// Mọi thứ client biết về "mình là ai" đều tra theo giá trị này: thành viên trong
    /// `RoomState`, mục trong `RoundStart.roster`, kết quả trong `RoundResult`.
    pub fn set_my_player(&mut self, player: u64) {
        self.my_player = Some(player);
    }

    /// Slot **trận** của chính client (lấy từ `RoundStart.roster` theo `account_id`).
    ///
    /// `None` = client chưa biết slot của mình: chưa `AuthOk`, hoặc chưa nhận `RoundStart`
    /// mà mình có mặt trong roster.
    pub fn my_slot(&self) -> Option<u8> {
        self.my_slot
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
    ///
    /// `RoomState` pha `Waiting` là bằng chứng server đã đưa phòng về chờ: ack kết quả của
    /// lượt trước (nếu có) đã tới, và lượt đang chờ barrier thì đã bị hủy.
    pub fn on_room_state(&mut self, state: RoomState) {
        if state.phase == RoomPhase::Waiting {
            self.pending_ack = None;
        }
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
    ///
    /// ## Thứ tự kiểm: round cũ/trùng **trước mọi side effect**
    ///
    /// `RoundStart` mang ID round của server; server cấp ID round **tăng dần** (`boom-room`
    /// `alloc_round`), nên ID nhỏ hơn ⇒ thuộc round cũ. Mạng có thể đưa `RoundStart` cũ tới
    /// trễ *sau* `RoundStart` mới: nếu áp nó thì kế hoạch round bị lùi, `my_slot` bị ghi đè
    /// bằng slot của round cũ, và mọi snapshot của round mới bị bỏ (client chỉ nhận
    /// snapshot khớp kế hoạch) — predictor không tự cứu được vì snapshot mới bị chặn trước
    /// nó. Vì vậy:
    ///
    /// - `round_id` nhỏ hơn round đang chơi ⇒ [`RoundStartOutcome::Stale`]: bỏ, **không**
    ///   đụng `my_slot`/kế hoạch/lỗi.
    /// - `round_id` bằng round đang chơi ⇒ [`RoundStartOutcome::Duplicate`]: xử lý
    ///   idempotent, trạng thái không đổi.
    pub fn on_round_start(
        &mut self,
        start: RoundStart,
        expected: &RoundExpectation,
    ) -> Result<RoundStartOutcome, RoomError> {
        // Bỏ message thuộc round cũ/trùng **trước mọi side effect** (kể cả side effect
        // của việc kiểm fingerprint: ghi đè lỗi đang hiển thị).
        if let Some(current) = self.round.as_ref() {
            if start.round_id < current.round_id {
                return Ok(RoundStartOutcome::Stale);
            }
            if start.round_id == current.round_id {
                return Ok(RoundStartOutcome::Duplicate);
            }
        }
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
        let Some(player) = self.my_player else {
            // Chưa `AuthOk` thì client chưa biết mình là ai ⇒ không có mục nào trong roster
            // là của mình.
            return Err(self.fail(RoomError::NotInRoster {
                round_id: start.round_id,
                player: None,
            }));
        };
        // Slot trận của client: mục roster có `player == account_id`.
        let Some(member) = start.roster.iter().find(|member| member.player == player) else {
            return Err(self.fail(RoomError::NotInRoster {
                round_id: start.round_id,
                player: Some(player),
            }));
        };
        self.my_slot = Some(member.slot);
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
        Ok(RoundStartOutcome::Started(plan))
    }

    /// Nhận `RoundResult`: kết quả do server quyết, client chỉ giữ lại.
    pub fn on_round_result(&mut self, result: RoundResult) {
        self.result = Some(result);
    }

    /// Ghi nhận đã gửi `LoadingAck` cho round `round_id`.
    ///
    /// Ack này **chưa** được xác nhận: client gửi lại cho tới khi nhận snapshot đầu tiên của
    /// round (xem [`PendingAck`]).
    pub fn mark_loading_acked(&mut self, round_id: u64) {
        self.loading_acked = Some(round_id);
        self.pending_ack = Some(PendingAck::Loading(round_id));
    }

    /// Ghi nhận đã gửi `ResultAck` cho round `round_id`.
    ///
    /// Ack này **chưa** được xác nhận: client gửi lại cho tới khi phòng quay lại `Waiting`
    /// (xem [`PendingAck`]).
    pub fn mark_result_acked(&mut self, round_id: u64) {
        self.result_acked = Some(round_id);
        self.pending_ack = Some(PendingAck::Result(round_id));
    }

    /// Ack đang chờ bằng chứng server đã nhận (client gửi lại theo nhịp).
    pub fn pending_ack(&self) -> Option<PendingAck> {
        self.pending_ack
    }

    /// Bằng chứng server đã nhận `LoadingAck`: snapshot cho thấy lượt **đã chạy**.
    ///
    /// `tick > 0` ⇒ server đã chạy ít nhất một tick của round ⇒ loading barrier đã qua ⇒
    /// server đã nhận `LoadingAck` của mọi thành viên.
    ///
    /// Snapshot **trước lượt** (`tick == 0`, server gửi khi phòng còn ở loading) *không*
    /// phải bằng chứng: nếu coi nó là bằng chứng thì client ngừng gửi lại `LoadingAck`
    /// ngay cả khi frame đầu bị mất ⇒ lượt không bao giờ bắt đầu (loading barrier chờ
    /// mãi). `LoadingAck` là idempotent nên gửi lại không đổi ý nghĩa.
    pub fn confirm_loading(&mut self, round_id: u64, tick: u32) {
        if tick == 0 {
            return;
        }
        if self.pending_ack == Some(PendingAck::Loading(round_id)) {
            self.pending_ack = None;
        }
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

    /// Thành viên phòng tương ứng với chính client (theo `account_id`).
    pub fn my_member(&self) -> Option<&boom_proto::RoomMember> {
        let state = self.room.as_ref()?;
        let player = self.my_player?;
        state.members.iter().find(|member| member.player == player)
    }

    /// `PlayerId` của chính client trong phòng (server bind, ADR 0009 Addendum B).
    ///
    /// Đây là `AuthOk.account_id` — giá trị mà server ghi vào mọi trường `player` trên dây.
    pub fn my_player(&self) -> Option<u64> {
        self.my_player
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
///
/// ## Mô hình: hành động **mong muốn** + hành động **đang chờ xác nhận**
///
/// Luồng giữ hai khái niệm tách biệt:
///
/// - **Hành động mong muốn** ([`AutoRoomFlow::desired`]): client muốn làm gì với trạng thái
///   server đã biết. Hàm thuần, chỉ đọc trạng thái.
/// - **Hành động đang chờ** ([`AutoRoomFlow::pending`]): hành động đã gửi mà server **chưa**
///   xác nhận. Client gửi lại đúng hành động này (cùng request ID → server dedup) theo nhịp,
///   nên mất frame không làm kẹt client.
///
/// Hành động được xác nhận khi trạng thái server cho thấy đã xong: vào phòng (có mặt trong
/// `RoomState`), ready (mình ready trong `RoomState`), start (nhận `RoundStart`), xin sảnh
/// (nhận `LobbyList`).
///
/// Vì protocol V1 **không** có kind riêng để xin trạng thái phòng, gửi lại lệnh phòng là cách
/// **duy nhất** để bắt server gửi lại `RoomState` (ADR 0009 Addendum B §B.2: request lặp →
/// `Outcome::Duplicate`, không áp hai lần).
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
    /// Hành động đã gửi mà server chưa xác nhận.
    pending: Option<AutoAction>,
    /// Đã nhận `RoundStart` của lượt đang chơi.
    ///
    /// Khi còn `false`, luồng giữ hành động `Start` để **resync**: server gửi lại `RoundStart`
    /// khi nhận lệnh `Start` lặp cùng request ID (ADR 0009 Addendum B §B.2).
    round_started: bool,
    /// Đã nhận `RoundResult` của lượt vừa xong: đang chờ phòng quay lại `Waiting`.
    result_seen: bool,
    /// Chữ ký lần `Start` bị server từ chối vì **chưa đủ điều kiện**: `(phòng, số thành viên,
    /// tất cả ready)`.
    ///
    /// Client không thử lại `Start` với cùng chữ ký cho tới khi trạng thái phòng đổi (thêm
    /// người hoặc thêm người ready). Lý do "sai pha" (`NotWaiting`) **không** chặn: server còn
    /// trong lượt thì thử lại được ngay.
    start_blocked: Option<(u64, usize, bool)>,
    /// ID phòng đã thử vào bị từ chối: luồng thử phòng khác thay vì lặp lại.
    failed_rooms: Vec<u64>,
    /// Phòng đang thử vào (dùng để đánh dấu phòng bị từ chối).
    last_join: Option<u64>,
}

impl AutoRoomFlow {
    /// Dựng luồng tự động chờ tối thiểu `min_players` người trong phòng rồi mới start.
    pub fn with_min_players(character: RoomCharacter, min_players: usize) -> Self {
        Self {
            character,
            min_players,
            pending: None,
            round_started: false,
            result_seen: false,
            start_blocked: None,
            failed_rooms: Vec::new(),
            last_join: None,
        }
    }

    /// Nhân vật luồng dùng khi tạo/vào phòng.
    pub fn character(&self) -> RoomCharacter {
        self.character
    }

    /// Xử lý sự kiện phòng vừa tới và trả về hành động **cần gửi** (nếu có).
    ///
    /// Trả `None` khi hành động đang chờ chưa được xác nhận: caller **không** gửi lại ngay mà
    /// để tầng mạng gửi lại theo nhịp (cùng request ID → server dedup).
    pub fn on_event(&mut self, event: RoomEvent, flow: &RoomFlow) -> Option<AutoAction> {
        match event {
            RoomEvent::RoundStart => {
                self.round_started = true;
                self.result_seen = false;
                self.confirm(&AutoAction::Start);
            }
            RoomEvent::RoundResult => {
                // Lượt vừa xong: server sẽ gửi `RoomState` (Waiting, ready reset). Client
                // sẵn sàng lại cho vòng sau; `SetReady` cũng là cách bắt server gửi lại
                // `RoomState` nếu frame đó bị mất trên đường.
                self.result_seen = true;
                self.round_started = false;
            }
            RoomEvent::RoomState => {
                if flow
                    .room()
                    .is_some_and(|state| state.phase == RoomPhase::Waiting)
                {
                    // Đã về phòng chờ: server reset ready, vòng sau cần ready lại.
                    self.result_seen = false;
                }
                self.confirm_by_state(flow);
            }
            RoomEvent::Rejected => {
                if flow.room().is_none()
                    && let Some(room) = self.last_join
                {
                    // Vào phòng bị từ chối (phòng đầy/đã bắt đầu): thử phòng khác.
                    self.failed_rooms.push(room);
                    self.last_join = None;
                }
                if flow.rejection().is_some_and(|rejected| {
                    matches!(
                        rejected.reason,
                        RoomRejectReason::NotEnoughPlayers | RoomRejectReason::NotAllReady
                    )
                }) && let Some(state) = flow.room()
                {
                    // `Start` bị từ chối vì chưa đủ điều kiện: chờ trạng thái phòng đổi.
                    let all_ready = state.members.iter().all(|member| member.ready);
                    self.start_blocked = Some((state.room_id, state.members.len(), all_ready));
                }
            }
            RoomEvent::LobbyList => {
                self.confirm(&AutoAction::RequestLobby);
            }
        }
        let desired = self.desired(flow);
        if desired == self.pending {
            // Hành động đang chờ chưa được xác nhận: để tầng mạng gửi lại theo nhịp.
            return None;
        }
        self.pending = desired.clone();
        if let Some(AutoAction::JoinRoom { room }) = &desired {
            self.last_join = Some(*room);
        }
        desired
    }

    /// Hành động đang chờ server xác nhận (chưa có phản hồi).
    ///
    /// Caller dùng để gửi lại **cùng** lệnh khi mạng mất frame: server dedup theo request ID
    /// nên lệnh không bị áp hai lần.
    pub fn pending(&self) -> Option<&AutoAction> {
        self.pending.as_ref()
    }

    /// Hành động client **muốn** làm với trạng thái server đã biết.
    ///
    /// Hàm thuần: chỉ đọc trạng thái phòng đã nhận, nên gọi lại bao nhiêu lần cũng cho cùng
    /// kết quả. `None` = client không cần làm gì lúc này (server sẽ đẩy `RoomState` khi
    /// trạng thái phòng đổi, ví dụ khi người khác bật ready).
    pub fn desired(&self, flow: &RoomFlow) -> Option<AutoAction> {
        if self.result_seen {
            // Lượt vừa xong: chờ phòng quay lại `Waiting`. `SetReady` vừa sẵn sàng cho vòng
            // sau, vừa bắt server gửi lại `RoomState` nếu frame đó bị mất.
            return Some(AutoAction::SetReady);
        }
        let Some(state) = flow.room() else {
            // Chưa ở trong phòng nào: vào phòng trong sảnh hoặc tạo phòng mới.
            return Some(self.enter_action(flow));
        };
        if state.phase != RoomPhase::Waiting {
            // Đang trong lượt: giữ `Start` để resync `RoundStart` nếu frame đó bị mất.
            return if self.round_started {
                None
            } else {
                Some(AutoAction::Start)
            };
        }
        let Some(me) = flow.my_member() else {
            // Không thấy mình trong phòng (server vừa đổi phòng): xin lại danh sách.
            return Some(self.enter_action(flow));
        };
        if !me.ready {
            return Some(AutoAction::SetReady);
        }
        let all_ready = state.members.iter().all(|member| member.ready);
        // Chỉ host mới start; server kiểm lại quyền (RoomRejectReason::NotHost).
        if flow.is_host() && all_ready && state.members.len() >= self.min_players {
            let signature = (state.room_id, state.members.len(), all_ready);
            if self.start_blocked != Some(signature) {
                return Some(AutoAction::Start);
            }
            // Server đã từ chối `Start` vì chưa đủ điều kiện: chờ trạng thái phòng đổi.
            return None;
        }
        // Đã sẵn sàng nhưng phòng chưa đủ điều kiện start: server tự đẩy `RoomState` khi
        // trạng thái phòng đổi nên client không cần hỏi thêm.
        None
    }

    /// Hành động vào phòng khi client chưa ở trong phòng nào.
    fn enter_action(&self, flow: &RoomFlow) -> AutoAction {
        let Some(list) = flow.lobby() else {
            // Chưa có danh sách phòng: xin sảnh.
            return AutoAction::RequestLobby;
        };
        match list
            .rooms
            .iter()
            .find(|room| !self.failed_rooms.contains(&room.room_id))
        {
            Some(room) => AutoAction::JoinRoom { room: room.room_id },
            // Không còn phòng nào để thử (đều đầy hoặc đã bắt đầu): tạo phòng mới.
            None => AutoAction::CreateRoom,
        }
    }

    /// Xác nhận hành động đang chờ khi trạng thái phòng cho thấy đã xong.
    fn confirm_by_state(&mut self, flow: &RoomFlow) {
        let Some(me) = flow.my_member() else {
            return;
        };
        // Đã có mặt trong phòng: lệnh vào phòng đã được chấp nhận.
        self.confirm_enter();
        if me.ready {
            self.confirm(&AutoAction::SetReady);
        }
    }

    /// Xác nhận lệnh vào phòng (tạo hoặc vào phòng có sẵn).
    fn confirm_enter(&mut self) {
        if matches!(
            self.pending,
            Some(AutoAction::CreateRoom) | Some(AutoAction::JoinRoom { .. })
        ) {
            self.pending = None;
        }
    }

    /// Xác nhận hành động đang chờ nếu nó đúng bằng `action`.
    fn confirm(&mut self, action: &AutoAction) {
        if self.pending.as_ref() == Some(action) {
            self.pending = None;
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
                name: "a".into(),
            }],
            start_tick: 0,
        }
    }

    /// Luồng phòng của client đã `AuthOk` với `account_id` = 1.
    fn flow_of_player() -> RoomFlow {
        let mut flow = RoomFlow::new();
        flow.set_my_player(1);
        flow
    }

    #[test]
    fn snapshot_truoc_luot_khong_phai_bang_chung_loading_ack() {
        // Server gửi snapshot *trước lượt* (tick 0) khi phòng còn ở loading: nó **không**
        // phải bằng chứng server đã nhận `LoadingAck` — client phải gửi lại `LoadingAck` tới
        // khi thấy lượt đã chạy, nếu không thì frame `LoadingAck` đầu bị mất là lượt không
        // bao giờ bắt đầu (loading barrier chờ mãi).
        let mut flow = RoomFlow::new();
        flow.mark_loading_acked(3);
        assert_eq!(flow.pending_ack(), Some(PendingAck::Loading(3)));
        flow.confirm_loading(3, 0);
        assert_eq!(
            flow.pending_ack(),
            Some(PendingAck::Loading(3)),
            "snapshot trước lượt không được bỏ việc gửi lại `LoadingAck`"
        );
        flow.confirm_loading(3, 1);
        assert_eq!(
            flow.pending_ack(),
            None,
            "snapshot cho thấy lượt đã chạy là bằng chứng server đã nhận `LoadingAck`"
        );
    }

    #[test]
    fn round_start_khop_fingerprint_thi_nhan_round() {
        let mut flow = flow_of_player();
        let outcome = flow
            .on_round_start(round_start(), &expectation())
            .expect("fingerprint phải khớp");
        let RoundStartOutcome::Started(plan) = outcome else {
            panic!("round mới phải được nhận");
        };
        assert_eq!(plan.round_id, 3);
        assert_eq!(plan.seed, 99);
        assert_eq!(plan.map, "Village00");
        assert_eq!(plan.mode, "battle_ffa");
        assert_eq!(flow.round().map(|plan| plan.round_id), Some(3));
        assert!(flow.error().is_none());
    }

    #[test]
    fn round_start_trung_cung_round_thi_idempotent() {
        // `RoundStart` lặp lại của round đang chơi (server gửi lại khi nhận lệnh `Start`
        // trùng, ADR 0009 Addendum B §B.2): xử lý **idempotent** — trạng thái không đổi và
        // không báo lỗi.
        let mut flow = flow_of_player();
        let start = round_start();
        let first = flow
            .on_round_start(start.clone(), &expectation())
            .expect("round mới phải được nhận");
        assert!(matches!(first, RoundStartOutcome::Started(_)));
        let again = flow
            .on_round_start(start, &expectation())
            .expect("round trùng không phải lỗi");
        assert!(
            matches!(again, RoundStartOutcome::Duplicate),
            "round trùng phải idempotent"
        );
        assert_eq!(flow.round().map(|plan| plan.round_id), Some(3));
        assert_eq!(flow.my_slot(), Some(0));
        assert!(flow.error().is_none());
    }

    #[test]
    fn round_start_cu_khong_lam_lui_round_moi() {
        // `RoundStart` của round cũ tới trễ sau `RoundStart` của round mới: bỏ **trước mọi
        // side effect** — không đổi kế hoạch round, không ghi lùi slot trận (kể cả khi
        // roster của round cũ đặt slot khác), không báo lỗi.
        let mut flow = flow_of_player();
        let mut old = round_start();
        old.round_id = 1;
        old.roster = vec![RoundMember {
            player: 1,
            slot: 5,
            character: RoomCharacter::C00,
            name: "a".into(),
        }];
        let mut new = round_start();
        new.round_id = 2;
        new.roster = vec![RoundMember {
            player: 1,
            slot: 1,
            character: RoomCharacter::C01,
            name: "a".into(),
        }];
        flow.on_round_start(new, &expectation())
            .expect("round mới phải được nhận");
        assert_eq!(flow.my_slot(), Some(1));

        let outcome = flow
            .on_round_start(old, &expectation())
            .expect("round cũ không phải lỗi");
        assert!(
            matches!(outcome, RoundStartOutcome::Stale),
            "round cũ phải bị bỏ"
        );
        assert_eq!(
            flow.round().map(|plan| plan.round_id),
            Some(2),
            "round cũ không được làm lùi kế hoạch round"
        );
        assert_eq!(
            flow.my_slot(),
            Some(1),
            "round cũ không được ghi lùi slot trận"
        );
    }

    #[test]
    fn round_start_cu_khong_xoa_loi_dang_co() {
        // `RoundStart` cũ không được đụng lỗi đang hiển thị: bỏ message cũ **trước** mọi
        // side effect, kể cả side effect của việc kiểm fingerprint (ghi đè `error`).
        let mut flow = flow_of_player();
        flow.on_round_start(round_start(), &expectation())
            .expect("round mới phải được nhận");
        flow.set_error(RoomError::MapMissing {
            map: "Village00".into(),
        });
        let mut old = round_start();
        old.round_id = 1;
        let outcome = flow
            .on_round_start(old, &expectation())
            .expect("round cũ không phải lỗi");
        assert!(matches!(outcome, RoundStartOutcome::Stale));
        assert!(
            flow.error().is_some(),
            "round cũ không được xóa lỗi đang có"
        );
    }

    #[test]
    fn round_start_lech_rules_thi_bao_lo_va_khong_nhan_round() {
        let mut flow = flow_of_player();
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
        let mut flow = flow_of_player();
        let mut start = round_start();
        start.content_sha256 = [2; 32];
        let error = flow
            .on_round_start(start, &expectation())
            .expect_err("hash content lệch phải bị từ chối");
        assert!(matches!(error, RoomError::ContentMismatch { .. }));
    }

    #[test]
    fn round_start_lech_mode_thi_bao_lo() {
        let mut flow = flow_of_player();
        let mut start = round_start();
        start.mode = "battle_team".into();
        let error = flow
            .on_round_start(start, &expectation())
            .expect_err("mode lệch phải bị từ chối");
        assert!(matches!(error, RoomError::ModeMismatch { .. }));
    }

    #[test]
    fn client_khong_trong_roster_thi_bao_lo() {
        // Client có `account_id` = 9 nhưng roster chỉ có player 1 ⇒ không có mục nào của mình.
        let mut flow = RoomFlow::new();
        flow.set_my_player(9);
        let error = flow
            .on_round_start(round_start(), &expectation())
            .expect_err("client không có trong roster phải bị từ chối");
        assert_eq!(
            error,
            RoomError::NotInRoster {
                round_id: 3,
                player: Some(9)
            }
        );
    }

    #[test]
    fn client_chua_dang_nhap_thi_bao_lo() {
        // Chưa `AuthOk` thì client chưa biết mình là ai ⇒ không tra được roster.
        let mut flow = RoomFlow::new();
        let error = flow
            .on_round_start(round_start(), &expectation())
            .expect_err("client chưa đăng nhập phải bị từ chối");
        assert_eq!(
            error,
            RoomError::NotInRoster {
                round_id: 3,
                player: None
            }
        );
    }

    #[test]
    fn slot_tran_lay_tu_roster_theo_account_id() {
        // Roster có hai người; client là player 2 ⇒ slot của client là slot 1 (không phải
        // `Welcome.player_id`, vốn chỉ là định danh kết nối).
        let mut flow = RoomFlow::new();
        flow.set_my_player(2);
        let mut start = round_start();
        start.roster = vec![
            RoundMember {
                player: 1,
                slot: 0,
                character: RoomCharacter::C00,
                name: "a".into(),
            },
            RoundMember {
                player: 2,
                slot: 1,
                character: RoomCharacter::C01,
                name: "a".into(),
            },
        ];
        flow.on_round_start(start, &expectation())
            .expect("phải nhận round");
        assert_eq!(flow.my_slot(), Some(1));
        assert_eq!(flow.my_player(), Some(2));
    }

    #[test]
    fn room_state_chi_dung_de_tim_thanh_vien_theo_account_id() {
        // Slot **trận** chỉ lấy từ `RoundStart.roster`; `RoomState.members` chỉ cho biết
        // thành viên nào là mình (theo `account_id`).
        let mut flow = RoomFlow::new();
        flow.set_my_player(7);
        flow.on_room_state(RoomState {
            room_id: 1,
            host: 3,
            phase: RoomPhase::Waiting,
            members: vec![
                RoomMember {
                    player: 3,
                    slot: 0,
                    character: RoomCharacter::C00,
                    ready: true,
                    name: "a".into(),
                },
                RoomMember {
                    player: 7,
                    slot: 1,
                    character: RoomCharacter::C01,
                    ready: false,
                    name: "a".into(),
                },
            ],
            round: None,
        });
        assert_eq!(flow.my_member().map(|member| member.player), Some(7));
        assert_eq!(
            flow.my_slot(),
            None,
            "chưa có `RoundStart` thì client chưa biết slot trận của mình"
        );
    }

    #[test]
    fn request_id_tang_dan_va_gui_lai_duoc_dung_id() {
        let mut flow = RoomFlow::new();
        let first = flow.command(RoomAction::Start);
        let second = flow.command(RoomAction::SetReady { ready: true });
        assert!(second.request > first.request, "request ID phải tăng dần");
        // Gửi lại lệnh đã gửi: cùng request ID để server dedup.
        let again = flow.command_with_request(first.request, RoomAction::Start);
        assert_eq!(again, first);
    }

    #[test]
    fn chat_giu_toi_phan_tram() {
        let mut flow = RoomFlow::new();
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
        let mut flow = RoomFlow::new();
        flow.set_my_player(9);
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
                name: "a".into(),
            }],
            round: None,
        });
        assert!(flow.is_host(), "thành viên slot 0 là host theo server");
        assert_eq!(flow.my_player(), Some(9));
    }
}
