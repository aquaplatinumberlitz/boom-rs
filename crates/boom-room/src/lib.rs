//! Phòng chờ và vòng đời lượt — V1-004…V1-008 (phần server), thuần như `boom-core`.
//!
//! Crate nhận lệnh đã xác thực danh tính và bind người chơi từ server, trả state/event; không socket,
//! không đồng hồ thật, không lưu trữ. Server P7 bọc crate này và tuần tự hoá request.
//!
//! Phạm vi task #13: create/join/leave/kick, host chuyển cho người vào sớm nhất còn lại, chọn C00/C01,
//! ready (host cũng ready), start khi đủ 2–4 người và mọi người ready, đổi roster/lựa chọn hủy ready,
//! lệnh start lặp chỉ tạo một lượt. Lệnh mang request ID để dedup.
//!
//! Policy nguồn: SCOPE_V1 §3 (DESIGN của dự án); khác client gốc có chủ ý ở RULES_V1 R-063 (host V1
//! cũng phải ready). Crate không đặt giá trị policy mặc định: [`RoomPolicy`] do server nạp từ
//! profile/scope. `Loading`/`Playing`/`Results`, loading barrier, disconnect, chat chờ và spawn thuộc
//! V1-009/010/022/023 và các task sau; API này cố ý chỉ mở waiting → started.
//!
//! ```
//! use boom_room::{
//!     CharacterId, Lobby, PlayerId, RequestId, RoomAction, RoomCommand, RoomPolicy,
//! };
//!
//! // Policy 2–4 người là fixture của SCOPE_V1 §3; server thật nạp từ profile.
//! let mut lobby = Lobby::new(RoomPolicy::new(4, 2).unwrap());
//! let host = PlayerId(1);
//! let room = lobby
//!     .apply(RoomCommand {
//!         player: host,
//!         request: RequestId(1),
//!         action: RoomAction::Create {
//!             character: CharacterId::C00,
//!         },
//!     })
//!     .unwrap();
//! assert!(!room.is_duplicate());
//! assert_eq!(lobby.rooms().len(), 1);
//! ```

mod lobby;
mod model;

pub use lobby::Lobby;
pub use model::{
    CharacterId, MemberView, Outcome, PlayerId, PolicyError, ReadyResetReason, RequestId,
    RoomAction, RoomCommand, RoomError, RoomEvent, RoomId, RoomPhase, RoomPolicy, RoomView,
    RoundId, RoundMember, RoundView, SlotId,
};
