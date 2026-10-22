//! Backend giả lập cho chế độ offline (`--ui-demo`).
//!
//! Nguồn dữ liệu giả: phòng, thành viên, chat và kết quả lượt đều do backend này
//! bịa ra để chạy trọn luồng V1 **không cần server** (issue #103). Bản mạng nối vào
//! sau khi #89/#99 merge (task riêng) — hợp đồng [`crate::ui::model::UiBackend`] không
//! đổi.
//!
//! Số người tối đa và số người tối thiểu để bắt đầu theo policy V1 (SCOPE_V1 §1:
//! 2–4 người); đây là **dữ liệu giả của bản demo**, không phải luật trận — luật
//! trận nằm ở `boom-core` và rules profile.

use crate::anim::Character;
use crate::ui::model::{
    ActionId, ChatLine, FieldId, MatchEntry, MatchOutcome, MatchProgress, MatchResult, RoomInfo,
    RoomMember, RoomPhase, RoomView, UiBackend, UiError, UiModel,
};

/// Tài khoản demo mặc định (đã đăng ký sẵn nên nút "đăng ký" báo lỗi).
pub const DEMO_ACCOUNT: &str = "demo";
/// Mật khẩu demo mặc định.
pub const DEMO_PASSWORD: &str = "demo-pass";
/// Tên phòng demo mặc định.
pub const DEMO_ROOM_NAME: &str = "room-1";
/// Số người tối đa của một phòng V1 (SCOPE_V1 §1).
const MAX_PLAYERS: u32 = 4;
/// Số người tối thiểu để bắt đầu lượt (SCOPE_V1 §1).
const MIN_PLAYERS_TO_START: usize = 2;
/// Số lần hỏi backend trước khi lượt chuyển từ "đang tải" sang "đang chơi".
const LOADING_POLLS: u32 = 2;
/// Số lần hỏi backend trước khi lượt kết thúc.
const PLAYING_POLLS: u32 = 4;
/// Số giây của loading barrier V1 (SCOPE_V1 §3) — chỉ để hiển thị đếm ngược.
const LOADING_SECONDS: u32 = 15;

/// Giai đoạn của lượt trong bản demo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DemoPhase {
    /// Chưa có lượt nào chạy.
    Idle,
    /// Đang chờ người chơi xác nhận tải.
    Loading {
        /// Số lần đã hỏi backend.
        polls: u32,
    },
    /// Đang chơi.
    Playing {
        /// Số lần đã hỏi backend.
        polls: u32,
    },
}

/// Backend giả lập: dữ liệu do chính nó sinh ra.
#[derive(Clone, Debug)]
pub struct DemoBackend {
    /// Tài khoản đang đăng nhập.
    account: Option<String>,
    /// Tài khoản đã đăng ký trong phiên demo kèm mật khẩu.
    registered: Vec<(String, String)>,
    /// Danh sách phòng trong sảnh.
    rooms: Vec<RoomInfo>,
    /// Phòng đang ở.
    room: Option<RoomView>,
    /// Giai đoạn lượt hiện tại.
    phase: DemoPhase,
    /// ID phòng kế tiếp khi tạo mới.
    next_room_id: u32,
}

impl Default for DemoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoBackend {
    /// Dựng backend demo với vài phòng giả.
    pub fn new() -> Self {
        let rooms = vec![
            RoomInfo {
                id: 1,
                name: "room-1".to_owned(),
                host: "demo-2".to_owned(),
                players: 2,
                max_players: MAX_PLAYERS,
                phase: RoomPhase::Waiting,
            },
            RoomInfo {
                id: 2,
                name: "room-2".to_owned(),
                host: "demo-3".to_owned(),
                players: 3,
                max_players: MAX_PLAYERS,
                phase: RoomPhase::Waiting,
            },
            RoomInfo {
                id: 3,
                name: "room-3".to_owned(),
                host: "demo-2".to_owned(),
                players: 4,
                max_players: MAX_PLAYERS,
                phase: RoomPhase::Playing,
            },
        ];
        Self {
            account: None,
            registered: vec![(DEMO_ACCOUNT.to_owned(), DEMO_PASSWORD.to_owned())],
            rooms,
            room: None,
            phase: DemoPhase::Idle,
            next_room_id: 4,
        }
    }

    /// Dựng phòng chờ giả: người chơi cục bộ cùng một người giả đã sẵn sàng.
    fn demo_room(&self, id: u32, name: &str, host: &str) -> RoomView {
        let account = self.account.clone().unwrap_or_default();
        let members = vec![
            RoomMember {
                name: account,
                ready: false,
                character: Character::C00,
                host: true,
                is_self: true,
            },
            RoomMember {
                name: "demo-2".to_owned(),
                ready: true,
                character: Character::C01,
                host: false,
                is_self: false,
            },
        ];
        RoomView {
            id,
            name: name.to_owned(),
            host: host.to_owned(),
            members,
            self_ready: false,
            self_character: Character::C00,
            can_start: false,
            chat: Vec::new(),
        }
    }

    /// Cập nhật cờ "đủ điều kiện bắt đầu" sau mỗi thay đổi trạng thái phòng.
    fn refresh_can_start(&mut self) {
        let Some(room) = self.room.as_mut() else {
            return;
        };
        let all_ready = room.members.iter().all(|member| member.ready);
        let enough = room.members.len() >= MIN_PLAYERS_TO_START;
        room.can_start = all_ready && enough;
    }

    /// Kết quả lượt giả: người chơi cục bộ thắng, người khác thua.
    fn demo_outcome(&self) -> MatchOutcome {
        let members = self
            .room
            .as_ref()
            .map(|room| room.members.clone())
            .unwrap_or_default();
        let entries: Vec<MatchEntry> = members
            .iter()
            .enumerate()
            .map(|(index, member)| MatchEntry {
                rank: index as u32 + 1,
                name: member.name.clone(),
                result: if member.is_self {
                    MatchResult::Win
                } else {
                    MatchResult::Lose
                },
            })
            .collect();
        // Không còn ai thì hòa (SCOPE_V1 §3).
        let self_result = if entries.len() > 1 {
            MatchResult::Win
        } else {
            MatchResult::Draw
        };
        MatchOutcome {
            entries,
            self_result,
        }
    }
}

impl UiBackend for DemoBackend {
    fn login(&mut self, account: &str, password: &str) -> Result<(), UiError> {
        if account.trim().is_empty() || password.is_empty() {
            return Err(UiError::MissingField);
        }
        // Tài khoản đã đăng ký thì mật khẩu phải đúng — để bản demo có đường lỗi
        // đăng nhập (client hiện `«login.failed»`).
        if let Some((_, stored)) = self.registered.iter().find(|(name, _)| name == account)
            && stored != password
        {
            return Err(UiError::InvalidCredentials);
        }
        self.account = Some(account.to_owned());
        Ok(())
    }

    fn register(&mut self, account: &str, password: &str) -> Result<(), UiError> {
        if account.trim().is_empty() || password.is_empty() {
            return Err(UiError::MissingField);
        }
        if self.registered.iter().any(|(name, _)| name == account) {
            return Err(UiError::AccountExists);
        }
        self.registered
            .push((account.to_owned(), password.to_owned()));
        Ok(())
    }

    fn logout(&mut self) -> Result<(), UiError> {
        self.account = None;
        self.room = None;
        self.phase = DemoPhase::Idle;
        Ok(())
    }

    fn rooms(&mut self) -> Result<Vec<RoomInfo>, UiError> {
        Ok(self.rooms.clone())
    }

    fn create_room(&mut self, name: &str) -> Result<(), UiError> {
        let id = self.next_room_id;
        self.next_room_id += 1;
        let name = if name.trim().is_empty() {
            format!("room-{id}")
        } else {
            name.trim().to_owned()
        };
        let host = self.account.clone().unwrap_or_default();
        self.rooms.push(RoomInfo {
            id,
            name: name.clone(),
            host: host.clone(),
            players: 1,
            max_players: MAX_PLAYERS,
            phase: RoomPhase::Waiting,
        });
        self.room = Some(self.demo_room(id, &name, &host));
        self.refresh_can_start();
        Ok(())
    }

    fn join_room(&mut self, room: u32) -> Result<(), UiError> {
        let Some(info) = self.rooms.iter().find(|item| item.id == room).cloned() else {
            return Err(UiError::RoomGone);
        };
        if info.players >= info.max_players {
            return Err(UiError::RoomFull);
        }
        let host = info.host.clone();
        self.room = Some(self.demo_room(info.id, &info.name, &host));
        self.refresh_can_start();
        Ok(())
    }

    fn leave_room(&mut self) -> Result<(), UiError> {
        self.room = None;
        self.phase = DemoPhase::Idle;
        Ok(())
    }

    fn set_ready(&mut self, ready: bool) -> Result<(), UiError> {
        let Some(room) = self.room.as_mut() else {
            return Err(UiError::Other);
        };
        room.self_ready = ready;
        for member in &mut room.members {
            if member.is_self {
                member.ready = ready;
            }
        }
        self.refresh_can_start();
        Ok(())
    }

    fn choose_character(&mut self, character: Character) -> Result<(), UiError> {
        let Some(room) = self.room.as_mut() else {
            return Err(UiError::Other);
        };
        room.self_character = character;
        for member in &mut room.members {
            if member.is_self {
                member.character = character;
            }
        }
        Ok(())
    }

    fn start(&mut self) -> Result<(), UiError> {
        let Some(room) = self.room.as_ref() else {
            return Err(UiError::Other);
        };
        if !room
            .members
            .iter()
            .any(|member| member.is_self && member.host)
        {
            return Err(UiError::NotHost);
        }
        if room.members.len() < MIN_PLAYERS_TO_START {
            return Err(UiError::NotReady);
        }
        if !room.members.iter().all(|member| member.ready) {
            return Err(UiError::NotReady);
        }
        self.phase = DemoPhase::Loading { polls: 0 };
        Ok(())
    }

    fn send_chat(&mut self, message: &str) -> Result<(), UiError> {
        let Some(room) = self.room.as_mut() else {
            return Err(UiError::Other);
        };
        let author = self.account.clone().unwrap_or_default();
        room.chat.push(ChatLine {
            author,
            text: message.to_owned(),
        });
        Ok(())
    }

    fn ack_result(&mut self) -> Result<(), UiError> {
        // Về phòng thì reset sẵn sàng (SCOPE_V1 §3).
        if let Some(room) = self.room.as_mut() {
            room.self_ready = false;
            for member in &mut room.members {
                if member.is_self {
                    member.ready = false;
                }
            }
        }
        self.phase = DemoPhase::Idle;
        self.refresh_can_start();
        Ok(())
    }

    fn room(&self) -> Option<RoomView> {
        self.room.clone()
    }

    fn match_progress(&mut self) -> Option<MatchProgress> {
        match self.phase {
            DemoPhase::Idle => None,
            DemoPhase::Loading { polls } => {
                let polls = polls + 1;
                self.phase = if polls >= LOADING_POLLS {
                    DemoPhase::Playing { polls: 0 }
                } else {
                    DemoPhase::Loading { polls }
                };
                Some(MatchProgress::Loading {
                    remaining: LOADING_SECONDS.saturating_sub(polls),
                })
            }
            DemoPhase::Playing { polls } => {
                let polls = polls + 1;
                if polls >= PLAYING_POLLS {
                    self.phase = DemoPhase::Idle;
                    Some(MatchProgress::Finished(self.demo_outcome()))
                } else {
                    self.phase = DemoPhase::Playing { polls };
                    Some(MatchProgress::Playing)
                }
            }
        }
    }
}

/// Màn cần chụp ảnh QA.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotScreen {
    /// Màn đăng nhập.
    Login,
    /// Màn sảnh.
    Lobby,
    /// Màn phòng chờ.
    Room,
    /// Màn trận.
    Match,
    /// Màn kết quả.
    Result,
}

/// Đưa bản demo tới một màn để chụp ảnh QA.
///
/// Đi qua **đúng các hành động thật** của UI (không set state tay) nên ảnh chụp
/// phản ánh trạng thái người chơi sẽ thấy.
pub fn jump_to(model: &mut UiModel<DemoBackend>, screen: ShotScreen) {
    if screen == ShotScreen::Login {
        return;
    }
    model.set_field_text(FieldId::LoginAccount, DEMO_ACCOUNT);
    model.set_field_text(FieldId::LoginPassword, DEMO_PASSWORD);
    model.activate(ActionId::LoginSubmit);
    if screen == ShotScreen::Lobby {
        return;
    }
    model.set_field_text(FieldId::LobbyRoomName, DEMO_ROOM_NAME);
    model.activate(ActionId::LobbyCreate);
    if screen == ShotScreen::Room {
        return;
    }
    model.activate(ActionId::RoomReady);
    model.activate(ActionId::RoomStart);
    if screen == ShotScreen::Match {
        return;
    }
    // Chạy hết lượt để có màn kết quả.
    let steps = (LOADING_POLLS + PLAYING_POLLS + 2) * crate::ui::model::MATCH_POLL_TICKS;
    for _ in 0..steps {
        model.tick();
        if model.screen() == boom_content::bundle::UiScreen::Result {
            break;
        }
    }
}
