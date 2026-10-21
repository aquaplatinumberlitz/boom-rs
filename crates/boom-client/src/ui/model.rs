//! Model của UI luồng V1: trạng thái màn hình, hành động và nguồn dữ liệu.
//!
//! Module **thuần** (không đụng macroquad): toàn bộ luồng màn hình — đăng nhập →
//! sảnh → phòng chờ → trận → kết quả → phòng — được điều khiển bằng [`UiModel`]
//! và kiểm được **không cần GPU** (acceptance V1-032).
//!
//! ## Nguồn dữ liệu
//!
//! [`UiModel`] không tự biết phòng nào đang mở, ai đã sẵn sàng hay kết quả lượt ra
//! sao: nó hỏi [`UiBackend`]. Bản offline giả lập là
//! [`crate::ui::demo::DemoBackend`] (chế độ `--ui-demo`); bản mạng là
//! [`crate::net::ui_backend::NetBackend`] (server thật) — model và layout không đổi.
//!
//! ## Không có dead end
//!
//! Mỗi màn đều có ít nhất một hành động đưa người chơi ra khỏi màn đó (xem
//! [`UiModel::actions`]). Màn trận (`UiScreen::Match`) **không** có nút rời trận:
//! theo [SCOPE_V1 §3](../../../../docs/planning/SCOPE_V1.md) rời trận chủ động là bị
//! loại, nên V1 chỉ cho thoát ứng dụng; lượt luôn kết thúc bằng màn kết quả vì
//! server chốt kết quả.

use crate::anim::Character;
use crate::ui::text::Texts;
use boom_content::bundle::UiScreen;

/// Lỗi do nguồn dữ liệu (backend) báo cho người chơi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiError {
    /// Sai tài khoản hoặc mật khẩu.
    InvalidCredentials,
    /// Tài khoản đã tồn tại.
    AccountExists,
    /// Thiếu trường bắt buộc.
    MissingField,
    /// Phòng không còn tồn tại.
    RoomGone,
    /// Phòng đã đủ người.
    RoomFull,
    /// Chưa đủ người hoặc chưa ai sẵn sàng.
    NotReady,
    /// Người ra lệnh không phải chủ phòng.
    NotHost,
    /// Lỗi khác (mạng, server).
    Other,
}

impl UiError {
    /// Khóa chuỗi hiển thị cho lỗi này.
    ///
    /// Khóa lấy từ file chuỗi V1; client không tự soạn câu thông báo lỗi.
    pub fn message_key(self) -> &'static str {
        match self {
            Self::InvalidCredentials | Self::AccountExists => "login.failed",
            Self::MissingField => "login.empty",
            Self::RoomGone => "lobby.room_removed",
            Self::RoomFull => "lobby.room_full",
            Self::NotReady => "room.all_ready",
            Self::NotHost => "room.non_host_denied",
            Self::Other => "app.network_error",
        }
    }
}

/// Trạng thái phòng trong sảnh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomPhase {
    /// Phòng đang chờ người chơi.
    Waiting,
    /// Phòng đang chơi một lượt.
    Playing,
}

/// Một phòng hiện có trong sảnh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomInfo {
    /// ID phòng (server cấp).
    pub id: u32,
    /// Tên phòng.
    pub name: String,
    /// Tên chủ phòng.
    pub host: String,
    /// Số người đang trong phòng.
    pub players: u32,
    /// Số người tối đa của phòng.
    pub max_players: u32,
    /// Phòng đang chờ hay đang chơi.
    pub phase: RoomPhase,
}

/// Thành viên trong phòng chờ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomMember {
    /// Tên hiển thị.
    pub name: String,
    /// Đã bấm sẵn sàng chưa.
    pub ready: bool,
    /// Nhân vật đã chọn.
    pub character: Character,
    /// Là chủ phòng không.
    pub host: bool,
    /// Là người chơi cục bộ không.
    pub is_self: bool,
}

/// Một dòng chat trong phòng chờ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    /// Người gửi.
    pub author: String,
    /// Nội dung tin nhắn.
    pub text: String,
}

/// Trạng thái phòng chờ mà UI hiển thị.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomView {
    /// ID phòng.
    pub id: u32,
    /// Tên phòng.
    pub name: String,
    /// Tên chủ phòng.
    pub host: String,
    /// Danh sách thành viên.
    pub members: Vec<RoomMember>,
    /// Người chơi cục bộ đã sẵn sàng chưa.
    pub self_ready: bool,
    /// Nhân vật người chơi cục bộ đã chọn.
    pub self_character: Character,
    /// Chủ phòng có thể bắt đầu lượt không.
    pub can_start: bool,
    /// Nội dung chat chờ.
    pub chat: Vec<ChatLine>,
}

/// Kết quả của một người chơi trong lượt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchResult {
    /// Thắng.
    Win,
    /// Thua.
    Lose,
    /// Hòa.
    Draw,
}

/// Một dòng của bảng kết quả.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchEntry {
    /// Hạng (1 = cao nhất).
    pub rank: u32,
    /// Tên người chơi.
    pub name: String,
    /// Kết quả của người chơi đó.
    pub result: MatchResult,
}

/// Kết quả một lượt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchOutcome {
    /// Bảng kết quả, theo hạng.
    pub entries: Vec<MatchEntry>,
    /// Kết quả của người chơi cục bộ.
    pub self_result: MatchResult,
}

/// Tiến trình của lượt đang chạy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchProgress {
    /// Đang chờ người chơi xác nhận tải (loading barrier).
    Loading {
        /// Số giây còn lại của barrier.
        remaining: u32,
    },
    /// Đang chơi.
    Playing,
    /// Lượt đã kết thúc, có kết quả.
    Finished(MatchOutcome),
}

/// Nguồn dữ liệu cho UI.
///
/// Bản offline giả lập: [`crate::ui::demo::DemoBackend`]. Bản mạng:
/// [`crate::net::ui_backend::NetBackend`] (đăng nhập → sảnh → phòng → trận → kết quả với
/// server thật). Hợp đồng của model không đổi.
pub trait UiBackend {
    /// Đăng nhập bằng tài khoản/mật khẩu đã có.
    fn login(&mut self, account: &str, password: &str) -> Result<(), UiError>;
    /// Đăng ký tài khoản mới.
    fn register(&mut self, account: &str, password: &str) -> Result<(), UiError>;
    /// Thoát phiên (về màn đăng nhập).
    fn logout(&mut self) -> Result<(), UiError>;
    /// Danh sách phòng của sảnh.
    fn rooms(&mut self) -> Result<Vec<RoomInfo>, UiError>;
    /// Tạo phòng mới với tên cho trước.
    fn create_room(&mut self, name: &str) -> Result<(), UiError>;
    /// Vào phòng đã có.
    fn join_room(&mut self, room: u32) -> Result<(), UiError>;
    /// Rời phòng, về sảnh.
    fn leave_room(&mut self) -> Result<(), UiError>;
    /// Bật/tắt trạng thái sẵn sàng của người chơi cục bộ.
    fn set_ready(&mut self, ready: bool) -> Result<(), UiError>;
    /// Chọn nhân vật cho người chơi cục bộ.
    fn choose_character(&mut self, character: Character) -> Result<(), UiError>;
    /// Chủ phòng bắt đầu lượt.
    fn start(&mut self) -> Result<(), UiError>;
    /// Gửi tin nhắn chat trong phòng chờ.
    fn send_chat(&mut self, message: &str) -> Result<(), UiError>;
    /// Xác nhận đã xem kết quả (để về phòng).
    fn ack_result(&mut self) -> Result<(), UiError>;
    /// Trạng thái phòng chờ hiện tại; `None` = chưa ở trong phòng nào.
    fn room(&self) -> Option<RoomView>;
    /// Tiến trình lượt đang chạy; `None` = chưa có lượt.
    fn match_progress(&mut self) -> Option<MatchProgress>;

    /// Bơm nguồn dữ liệu để trạng thái mới tới UI.
    ///
    /// Mặc định **không làm gì**: bản offline ([`crate::ui::demo::DemoBackend`]) không có nguồn
    /// ngoài. Bản mạng ([`crate::net::ui_backend::NetBackend`]) đọc message từ server ở đây —
    /// [`UiModel::tick`] gọi hàm này mỗi vòng, nên UI trong phòng chờ thấy được thay đổi của
    /// người khác mà không cần bấm làm mới.
    fn pump(&mut self) {}
}

/// Hành động của UI (mỗi hành động gắn một khóa chuỗi đã có trong file chuỗi V1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    /// Đăng nhập.
    LoginSubmit,
    /// Đăng ký tài khoản mới.
    LoginRegister,
    /// Tạo phòng.
    LobbyCreate,
    /// Vào phòng đã chọn.
    LobbyJoin,
    /// Làm mới danh sách phòng.
    LobbyRefresh,
    /// Quay lại màn đăng nhập (thoát phiên).
    LobbyBack,
    /// Sẵn sàng / hủy sẵn sàng.
    RoomReady,
    /// Bắt đầu lượt (chủ phòng).
    RoomStart,
    /// Gửi chat.
    RoomChatSend,
    /// Rời phòng, về sảnh.
    RoomLeave,
    /// Xem xong kết quả, về phòng.
    ResultContinue,
    /// Thoát ứng dụng.
    Quit,
}

impl ActionId {
    /// Khóa chuỗi của nhãn nút.
    ///
    /// Mọi khóa đều có trong `data/l10n/vi-v1.toml`; khóa chưa có text thì UI hiện
    /// placeholder `⟨khóa⟩` (UI_STRINGS_V1 §6.1).
    pub fn key(self) -> &'static str {
        match self {
            Self::LoginSubmit => "login.submit",
            Self::LoginRegister => "login.register",
            Self::LobbyCreate => "lobby.create",
            Self::LobbyJoin => "lobby.join",
            Self::LobbyRefresh => "lobby.refresh",
            Self::LobbyBack => "lobby.back",
            Self::RoomReady => "room.ready",
            Self::RoomStart => "room.start",
            Self::RoomChatSend => "lobby.chat_send",
            Self::RoomLeave => "room.leave",
            Self::ResultContinue => "result.continue",
            Self::Quit => "lobby.exit",
        }
    }
}

/// Ô nhập của UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldId {
    /// Ô tài khoản.
    LoginAccount,
    /// Ô mật khẩu.
    LoginPassword,
    /// Ô đặt tên phòng mới.
    LobbyRoomName,
    /// Ô gõ chat trong phòng chờ.
    RoomChat,
}

/// Mục tiêu focus của UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// Một ô nhập.
    Field(FieldId),
    /// Một nút.
    Action(ActionId),
}

/// Một nút trên màn hiện tại.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAction {
    /// ID hành động.
    pub id: ActionId,
    /// Khóa chuỗi của nhãn nút.
    pub key: &'static str,
    /// Nút chính của màn (focus mặc định).
    pub primary: bool,
    /// Nút dùng được không (ví dụ "vào phòng" khi chưa chọn phòng).
    pub enabled: bool,
}

/// Kết quả xử lý một hành động.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionOutcome {
    /// Hành động đã chạy.
    Done,
    /// Hành động bị từ chối; kèm khóa chuỗi để hiện thông báo.
    Rejected(&'static str),
    /// Người chơi muốn thoát ứng dụng.
    Quit,
}

/// Số lần gọi [`UiModel::tick`] giữa hai lần hỏi backend về tiến trình lượt.
///
/// Đây là **nhịp hiển thị** của client (design), không phải luật chơi: lượt trong
/// trận do server chốt, client chỉ hỏi trạng thái.
pub const MATCH_POLL_TICKS: u32 = 30;

/// Model của UI: màn hiện tại, dữ liệu hiển thị và cách chuyển màn.
pub struct UiModel<B: UiBackend> {
    backend: B,
    texts: Texts,
    screen: UiScreen,
    account: String,
    password: String,
    room_name: String,
    chat_input: String,
    rooms: Vec<RoomInfo>,
    selected_room: Option<usize>,
    selected_member: Option<usize>,
    focus: Focus,
    /// Khóa chuỗi của dòng thông báo đang hiện.
    notice: Option<&'static str>,
    outcome: Option<MatchOutcome>,
    progress: Option<MatchProgress>,
    tick: u32,
}

impl<B: UiBackend> UiModel<B> {
    /// Dựng model ở màn đăng nhập.
    pub fn new(backend: B, texts: Texts) -> Self {
        Self {
            backend,
            texts,
            screen: UiScreen::Login,
            account: String::new(),
            password: String::new(),
            room_name: String::new(),
            chat_input: String::new(),
            rooms: Vec::new(),
            selected_room: None,
            selected_member: None,
            focus: Focus::Field(FieldId::LoginAccount),
            notice: None,
            outcome: None,
            progress: None,
            tick: 0,
        }
    }

    /// Màn hiện tại.
    pub fn screen(&self) -> UiScreen {
        self.screen
    }

    /// Bộ chuỗi hiển thị.
    pub fn texts(&self) -> &Texts {
        &self.texts
    }

    /// Danh sách phòng ở sảnh.
    pub fn rooms(&self) -> &[RoomInfo] {
        &self.rooms
    }

    /// Phòng đang chọn ở sảnh.
    pub fn selected_room(&self) -> Option<&RoomInfo> {
        self.selected_room.and_then(|index| self.rooms.get(index))
    }

    /// Chỉ số thành viên đang chọn ở phòng chờ.
    pub fn selected_member_index(&self) -> Option<usize> {
        self.selected_member
    }

    /// Trạng thái phòng chờ hiện tại.
    pub fn room(&self) -> Option<RoomView> {
        self.backend.room()
    }

    /// Kết quả lượt vừa kết thúc.
    pub fn outcome(&self) -> Option<&MatchOutcome> {
        self.outcome.as_ref()
    }

    /// Tiến trình lượt đang chạy.
    pub fn match_progress(&self) -> Option<&MatchProgress> {
        self.progress.as_ref()
    }

    /// Khóa chuỗi của dòng thông báo đang hiện.
    pub fn notice(&self) -> Option<&'static str> {
        self.notice
    }

    /// Nội dung ô đang focus (để hiển thị và kiểm tra).
    pub fn field_value(&self, field: FieldId) -> &str {
        match field {
            FieldId::LoginAccount => &self.account,
            FieldId::LoginPassword => &self.password,
            FieldId::LobbyRoomName => &self.room_name,
            FieldId::RoomChat => &self.chat_input,
        }
    }

    /// Mục tiêu focus hiện tại.
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// Danh sách mục tiêu có thể focus, theo thứ tự Tab.
    pub fn focusables(&self) -> Vec<Focus> {
        let mut out = Vec::new();
        match self.screen {
            UiScreen::Login => {
                out.push(Focus::Field(FieldId::LoginAccount));
                out.push(Focus::Field(FieldId::LoginPassword));
            }
            UiScreen::Lobby => out.push(Focus::Field(FieldId::LobbyRoomName)),
            UiScreen::Room => out.push(Focus::Field(FieldId::RoomChat)),
            UiScreen::Match | UiScreen::Result => {}
        }
        for action in self.actions() {
            out.push(Focus::Action(action.id));
        }
        out
    }

    /// Chuyển focus tới mục tiêu kế tiếp (hoặc trước đó nếu `back`).
    pub fn focus_step(&mut self, back: bool) {
        let focusables = self.focusables();
        if focusables.is_empty() {
            return;
        }
        let current = focusables.iter().position(|item| *item == self.focus);
        let len = focusables.len();
        let next = match (current, back) {
            (Some(index), false) => (index + 1) % len,
            (Some(index), true) => (index + len - 1) % len,
            (None, false) => 0,
            (None, true) => len - 1,
        };
        self.focus = focusables[next];
    }

    /// Nhận ký tự vào ô đang focus.
    pub fn type_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        if let Focus::Field(field) = self.focus {
            let mut value = self.field_value(field).to_owned();
            value.push(character);
            self.set_field_text(field, &value);
        }
    }

    /// Xoá ký tự cuối ở ô đang focus.
    pub fn backspace(&mut self) {
        if let Focus::Field(field) = self.focus {
            let mut value = self.field_value(field).to_owned();
            value.pop();
            self.set_field_text(field, &value);
        }
    }

    /// Đặt nội dung của một ô nhập (dán, IME, hoặc điền sẵn cho bản demo).
    pub fn set_field_text(&mut self, field: FieldId, value: &str) {
        let value = value.to_owned();
        self.set_field(field, value);
    }

    /// Đổi nhân vật của người chơi cục bộ (C00 ⇄ C01).
    ///
    /// V1 có đúng hai nhân vật (SCOPE_V1 §1) nên đổi là chuyển qua lại. Đổi nhân vật
    /// làm hủy sẵn sàng của cả phòng — server xử lý, client chỉ gửi lệnh.
    pub fn cycle_character(&mut self) {
        if self.screen != UiScreen::Room {
            return;
        }
        let Some(room) = self.backend.room() else {
            return;
        };
        let next = match room.self_character {
            Character::C00 => Character::C01,
            Character::C01 => Character::C00,
        };
        if let Err(error) = self.backend.choose_character(next) {
            self.notice = Some(error.message_key());
        }
    }

    /// Di chuyển dòng đang chọn trong danh sách của màn hiện tại.
    pub fn move_selection(&mut self, delta: i32) {
        match self.screen {
            UiScreen::Lobby => {
                if self.rooms.is_empty() {
                    self.selected_room = None;
                    return;
                }
                let len = self.rooms.len() as i32;
                let current = self.selected_room.map_or(0, |index| index as i32);
                let next = (current + delta).rem_euclid(len);
                self.selected_room = Some(next as usize);
            }
            UiScreen::Room => {
                let Some(room) = self.backend.room() else {
                    self.selected_member = None;
                    return;
                };
                if room.members.is_empty() {
                    self.selected_member = None;
                    return;
                }
                let len = room.members.len() as i32;
                let current = self.selected_member.map_or(0, |index| index as i32);
                let next = (current + delta).rem_euclid(len);
                self.selected_member = Some(next as usize);
            }
            UiScreen::Login | UiScreen::Match | UiScreen::Result => {}
        }
    }

    /// Các nút của màn hiện tại.
    ///
    /// Mỗi màn có ít nhất một hành động đưa người chơi ra khỏi màn — không có dead end.
    pub fn actions(&self) -> Vec<UiAction> {
        match self.screen {
            UiScreen::Login => vec![
                UiAction {
                    id: ActionId::LoginSubmit,
                    key: ActionId::LoginSubmit.key(),
                    primary: true,
                    enabled: true,
                },
                UiAction {
                    id: ActionId::LoginRegister,
                    key: ActionId::LoginRegister.key(),
                    primary: false,
                    enabled: true,
                },
                UiAction {
                    id: ActionId::Quit,
                    key: ActionId::Quit.key(),
                    primary: false,
                    enabled: true,
                },
            ],
            UiScreen::Lobby => vec![
                UiAction {
                    id: ActionId::LobbyCreate,
                    key: ActionId::LobbyCreate.key(),
                    primary: true,
                    enabled: true,
                },
                UiAction {
                    id: ActionId::LobbyJoin,
                    key: ActionId::LobbyJoin.key(),
                    primary: false,
                    enabled: self.selected_room.is_some(),
                },
                UiAction {
                    id: ActionId::LobbyRefresh,
                    key: ActionId::LobbyRefresh.key(),
                    primary: false,
                    enabled: true,
                },
                UiAction {
                    id: ActionId::LobbyBack,
                    key: ActionId::LobbyBack.key(),
                    primary: false,
                    enabled: true,
                },
                UiAction {
                    id: ActionId::Quit,
                    key: ActionId::Quit.key(),
                    primary: false,
                    enabled: true,
                },
            ],
            UiScreen::Room => {
                let ready = self.self_ready();
                vec![
                    UiAction {
                        id: ActionId::RoomReady,
                        key: if ready {
                            "room.ready_cancel"
                        } else {
                            ActionId::RoomReady.key()
                        },
                        primary: true,
                        enabled: true,
                    },
                    UiAction {
                        id: ActionId::RoomStart,
                        key: ActionId::RoomStart.key(),
                        primary: false,
                        enabled: self.can_start(),
                    },
                    UiAction {
                        id: ActionId::RoomChatSend,
                        key: ActionId::RoomChatSend.key(),
                        primary: false,
                        enabled: !self.chat_input.trim().is_empty(),
                    },
                    UiAction {
                        id: ActionId::RoomLeave,
                        key: ActionId::RoomLeave.key(),
                        primary: false,
                        enabled: true,
                    },
                    UiAction {
                        id: ActionId::Quit,
                        key: ActionId::Quit.key(),
                        primary: false,
                        enabled: true,
                    },
                ]
            }
            // Trong trận không có nút rời trận: rời trận chủ động là bị loại
            // (SCOPE_V1 §3). Lối thoát duy nhất là thoát ứng dụng; lượt luôn kết
            // thúc bằng màn kết quả vì server chốt kết quả.
            UiScreen::Match => vec![UiAction {
                id: ActionId::Quit,
                key: ActionId::Quit.key(),
                primary: true,
                enabled: true,
            }],
            UiScreen::Result => vec![
                UiAction {
                    id: ActionId::ResultContinue,
                    key: ActionId::ResultContinue.key(),
                    primary: true,
                    enabled: true,
                },
                UiAction {
                    id: ActionId::Quit,
                    key: ActionId::Quit.key(),
                    primary: false,
                    enabled: true,
                },
            ],
        }
    }

    /// Chạy một hành động.
    pub fn activate(&mut self, action: ActionId) -> ActionOutcome {
        self.notice = None;
        match action {
            ActionId::LoginSubmit => self.do_login(false),
            ActionId::LoginRegister => self.do_login(true),
            ActionId::LobbyCreate => self.do_create_room(),
            ActionId::LobbyJoin => self.do_join_room(),
            ActionId::LobbyRefresh => self.do_refresh_rooms(),
            ActionId::LobbyBack => self.do_logout(),
            ActionId::RoomReady => self.do_toggle_ready(),
            ActionId::RoomStart => self.do_start(),
            ActionId::RoomChatSend => self.do_send_chat(),
            ActionId::RoomLeave => self.do_leave_room(),
            ActionId::ResultContinue => self.do_continue(),
            ActionId::Quit => ActionOutcome::Quit,
        }
    }

    /// Quay lại màn trước.
    ///
    /// Màn nào cũng có lối ra: đăng nhập → thoát ứng dụng, sảnh → về đăng nhập,
    /// phòng → về sảnh, kết quả → về phòng. Màn trận không có nút quay lại vì
    /// rời trận chủ động là bị loại (SCOPE_V1 §3).
    pub fn back(&mut self) -> ActionOutcome {
        match self.screen {
            UiScreen::Login => ActionOutcome::Quit,
            UiScreen::Lobby => self.activate(ActionId::LobbyBack),
            UiScreen::Room => self.activate(ActionId::RoomLeave),
            UiScreen::Match => ActionOutcome::Quit,
            UiScreen::Result => self.activate(ActionId::ResultContinue),
        }
    }

    /// Gửi phím Enter: kích hoạt nút đang focus, hoặc gửi nội dung ô đang focus.
    pub fn submit_focused(&mut self) -> ActionOutcome {
        match self.focus {
            Focus::Action(action) => self.activate(action),
            Focus::Field(FieldId::LoginAccount) | Focus::Field(FieldId::LoginPassword) => {
                self.activate(ActionId::LoginSubmit)
            }
            Focus::Field(FieldId::LobbyRoomName) => self.activate(ActionId::LobbyCreate),
            Focus::Field(FieldId::RoomChat) => self.activate(ActionId::RoomChatSend),
        }
    }

    /// Tiến trình client: hỏi backend về lượt và chuyển sang màn kết quả khi xong.
    ///
    /// Mỗi vòng gọi [`UiBackend::pump`] trước để nguồn dữ liệu (bản mạng) có trạng thái mới
    /// nhất; bản offline không làm gì.
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.backend.pump();
        if self.screen != UiScreen::Match {
            return;
        }
        if !self.tick.is_multiple_of(MATCH_POLL_TICKS) {
            return;
        }
        let Some(progress) = self.backend.match_progress() else {
            return;
        };
        if let MatchProgress::Finished(outcome) = &progress {
            self.outcome = Some(outcome.clone());
            self.progress = Some(progress);
            self.screen = UiScreen::Result;
            self.focus = Focus::Action(ActionId::ResultContinue);
            return;
        }
        self.progress = Some(progress);
    }

    /// Người chơi cục bộ đã sẵn sàng chưa.
    fn self_ready(&self) -> bool {
        self.backend.room().is_some_and(|room| room.self_ready)
    }

    /// Chủ phòng có thể bắt đầu lượt không.
    fn can_start(&self) -> bool {
        self.backend.room().is_some_and(|room| room.can_start)
    }

    /// Đặt giá trị của một ô nhập.
    fn set_field(&mut self, field: FieldId, value: String) {
        match field {
            FieldId::LoginAccount => self.account = value,
            FieldId::LoginPassword => self.password = value,
            FieldId::LobbyRoomName => self.room_name = value,
            FieldId::RoomChat => self.chat_input = value,
        }
    }

    /// Ghi thông báo lỗi và trả về kết quả tương ứng.
    fn reject(&mut self, key: &'static str) -> ActionOutcome {
        self.notice = Some(key);
        ActionOutcome::Rejected(key)
    }

    /// Đăng nhập (hoặc đăng ký rồi đăng nhập).
    fn do_login(&mut self, register: bool) -> ActionOutcome {
        if self.account.trim().is_empty() || self.password.is_empty() {
            return self.reject("login.empty");
        }
        if register && let Err(error) = self.backend.register(&self.account, &self.password) {
            return self.reject(error.message_key());
        }
        if let Err(error) = self.backend.login(&self.account, &self.password) {
            return self.reject(error.message_key());
        }
        self.screen = UiScreen::Lobby;
        self.selected_room = None;
        self.refresh_rooms();
        self.focus = Focus::Field(FieldId::LobbyRoomName);
        ActionOutcome::Done
    }

    /// Thoát phiên, về màn đăng nhập.
    fn do_logout(&mut self) -> ActionOutcome {
        if let Err(error) = self.backend.logout() {
            return self.reject(error.message_key());
        }
        self.screen = UiScreen::Login;
        self.password.clear();
        self.rooms.clear();
        self.selected_room = None;
        self.focus = Focus::Field(FieldId::LoginAccount);
        ActionOutcome::Done
    }

    /// Làm mới danh sách phòng.
    fn do_refresh_rooms(&mut self) -> ActionOutcome {
        self.refresh_rooms();
        if let Some(key) = self.notice {
            return ActionOutcome::Rejected(key);
        }
        ActionOutcome::Done
    }

    /// Nạp danh sách phòng từ backend; lỗi thì ghi thông báo.
    fn refresh_rooms(&mut self) {
        match self.backend.rooms() {
            Ok(rooms) => {
                self.rooms = rooms;
                if self
                    .selected_room
                    .is_some_and(|index| index >= self.rooms.len())
                {
                    self.selected_room = None;
                }
            }
            Err(error) => {
                self.notice = Some(error.message_key());
            }
        }
    }

    /// Tạo phòng mới rồi vào phòng đó.
    fn do_create_room(&mut self) -> ActionOutcome {
        if let Err(error) = self.backend.create_room(&self.room_name) {
            return self.reject(error.message_key());
        }
        self.enter_room();
        ActionOutcome::Done
    }

    /// Vào phòng đang chọn ở sảnh.
    fn do_join_room(&mut self) -> ActionOutcome {
        let Some(room) = self.selected_room().cloned() else {
            return self.reject("lobby.empty");
        };
        if let Err(error) = self.backend.join_room(room.id) {
            return self.reject(error.message_key());
        }
        self.enter_room();
        ActionOutcome::Done
    }

    /// Chuyển sang màn phòng chờ sau khi vào phòng thành công.
    fn enter_room(&mut self) {
        self.screen = UiScreen::Room;
        self.selected_member = None;
        self.chat_input.clear();
        self.outcome = None;
        self.progress = None;
        self.focus = Focus::Field(FieldId::RoomChat);
    }

    /// Rời phòng, về sảnh.
    fn do_leave_room(&mut self) -> ActionOutcome {
        if let Err(error) = self.backend.leave_room() {
            return self.reject(error.message_key());
        }
        self.screen = UiScreen::Lobby;
        self.selected_member = None;
        self.chat_input.clear();
        self.refresh_rooms();
        self.focus = Focus::Field(FieldId::LobbyRoomName);
        ActionOutcome::Done
    }

    /// Bật/tắt sẵn sàng.
    fn do_toggle_ready(&mut self) -> ActionOutcome {
        let ready = !self.self_ready();
        if let Err(error) = self.backend.set_ready(ready) {
            return self.reject(error.message_key());
        }
        ActionOutcome::Done
    }

    /// Bắt đầu lượt (chủ phòng).
    fn do_start(&mut self) -> ActionOutcome {
        if let Err(error) = self.backend.start() {
            return self.reject(error.message_key());
        }
        self.screen = UiScreen::Match;
        self.progress = Some(MatchProgress::Loading { remaining: 0 });
        self.focus = Focus::Action(ActionId::Quit);
        ActionOutcome::Done
    }

    /// Gửi chat trong phòng chờ.
    fn do_send_chat(&mut self) -> ActionOutcome {
        let message = self.chat_input.trim();
        if message.is_empty() {
            return self.reject("login.empty");
        }
        if let Err(error) = self.backend.send_chat(message) {
            return self.reject(error.message_key());
        }
        self.chat_input.clear();
        ActionOutcome::Done
    }

    /// Xem xong kết quả, về phòng.
    fn do_continue(&mut self) -> ActionOutcome {
        if let Err(error) = self.backend.ack_result() {
            return self.reject(error.message_key());
        }
        self.screen = UiScreen::Room;
        self.outcome = None;
        self.progress = None;
        self.selected_member = None;
        self.focus = Focus::Field(FieldId::RoomChat);
        ActionOutcome::Done
    }
}
