//! Layout của màn UI: toạ độ logic của từng phần tử (hàm thuần, không GPU).
//!
//! Module này biến [`UiModel`] thành danh sách phần tử đã **định vị** để phần vẽ
//! ([`crate::ui::draw`]) chỉ còn đọc toạ độ và màu. Vì vậy toạ độ, thứ tự và nội dung
//! hiển thị đều kiểm được **không cần GPU** — phần nghiệm thu chính của V1-032.
//!
//! ## Quy ước
//!
//! - Màn có kích thước logic **800×600** như gốc asset (UI_STRINGS_V1 §5.2).
//! - Chữ lấy từ file chuỗi V1; khóa chưa có text hiện placeholder `⟨khóa⟩`.
//! - Ảnh UI chỉ hiện khi phần tử ở trạng thái `selected` (đã duyệt, không có chữ
//!   cần duyệt). Phần tử `needs_review_text`/`excluded` được thay bằng **khung màu +
//!   text catalog** (UI_STRINGS_V1 §3) — không hiện ảnh chưa duyệt.

use boom_content::bundle::{Bundle, UiScreen, UiState};

use crate::anim::Character;
use crate::scene::Scene;
use crate::ui::model::{
    ActionId, FieldId, Focus, MatchProgress, MatchResult, RoomPhase, UiBackend, UiModel,
};
use crate::ui::text::Texts;

/// Bề rộng màn UI (px) — khớp gốc asset 800×600 (UI_STRINGS_V1 §5.2).
pub const WIDTH: u32 = 800;
/// Cao màn UI (px).
pub const HEIGHT: u32 = 600;

/// Lề trái/phải của màn.
const MARGIN: i32 = 24;
/// Vùng tiêu đề.
const TITLE_RECT: Rect = Rect {
    x: MARGIN,
    y: 12,
    width: 752,
    height: 36,
};
/// Vùng dòng thông báo (lỗi hoặc trạng thái).
const NOTICE_RECT: Rect = Rect {
    x: MARGIN,
    y: 52,
    width: 752,
    height: 26,
};
/// Vùng nội dung chính (bảng, chat).
const CONTENT_RECT: Rect = Rect {
    x: MARGIN,
    y: 88,
    width: 580,
    height: 340,
};
/// Vùng cột phải: ảnh UI đã duyệt và khung thay ảnh chưa duyệt.
const SIDE_RECT: Rect = Rect {
    x: 636,
    y: 88,
    width: 156,
    height: 392,
};
/// Cao một ô thay ảnh (khung màu + text catalog) — tối đa.
const FRAME_HEIGHT: u32 = 64;
/// Cao một ô thay ảnh — tối thiểu (đủ một dòng chữ nhỏ).
const FRAME_MIN_HEIGHT: u32 = 28;
/// Cao một ô ảnh UI đã duyệt.
const ART_HEIGHT: u32 = 160;
/// Vùng ô nhập.
const FIELD_RECT: Rect = Rect {
    x: MARGIN,
    y: 440,
    width: 400,
    height: 40,
};
/// Vùng hàng nút.
const BUTTON_Y: i32 = 496;
/// Vùng hàng nút của màn trận: nút nằm ở góc trên-phải vì bản đồ chiếm gần hết màn.
const MATCH_BUTTON_X: i32 = 636;
/// Toạ độ Y của hàng nút màn trận.
const MATCH_BUTTON_Y: i32 = 4;
/// Toạ độ Y của bản đồ ở màn trận (chừa chỗ cho tiêu đề và cho vật cản vươn lên).
const MATCH_MAP_Y: i32 = 70;
/// Cao nút.
const BUTTON_HEIGHT: u32 = 40;
/// Rộng nút.
const BUTTON_WIDTH: u32 = 140;
/// Khoảng cách giữa hai nút.
const BUTTON_GAP: i32 = 8;

/// Hình chữ nhật trong không gian logic của UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    /// Toạ độ X góc trên-trái.
    pub x: i32,
    /// Toạ độ Y góc trên-trái.
    pub y: i32,
    /// Chiều rộng (px).
    pub width: u32,
    /// Chiều cao (px).
    pub height: u32,
}

impl Rect {
    /// Dựng hình chữ nhật.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Toạ độ Y mép dưới.
    pub fn bottom(&self) -> i32 {
        self.y + i32::try_from(self.height).unwrap_or(i32::MAX)
    }
}

/// Một dòng của bảng.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRow {
    /// Các ô của dòng, theo thứ tự cột.
    pub cells: Vec<String>,
    /// Dòng đang chọn.
    pub selected: bool,
}

/// Màu của khung vẽ thay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Nền màn.
    Panel,
    /// Ảnh có chữ chưa duyệt → khung màu + text catalog.
    Pending,
    /// Ảnh bị loại khỏi V1 → khung màu + text catalog.
    Excluded,
}

/// Một phần tử của layout.
#[derive(Clone, Debug, PartialEq)]
pub enum Widget {
    /// Tiêu đề màn.
    Title {
        /// Vùng vẽ.
        rect: Rect,
        /// Nội dung tiêu đề.
        text: String,
    },
    /// Dòng thông báo: lỗi (`error`) hoặc trạng thái.
    Notice {
        /// Vùng vẽ.
        rect: Rect,
        /// Nội dung thông báo.
        text: String,
        /// Là thông báo lỗi không.
        error: bool,
    },
    /// Ô nhập: nhãn ở trên, giá trị trong khung.
    Field {
        /// Vùng vẽ khung ô nhập.
        rect: Rect,
        /// Nhãn của ô.
        label: String,
        /// Giá trị hiện tại.
        value: String,
        /// Ô đang có focus.
        focused: bool,
        /// Giấu nội dung (mật khẩu).
        masked: bool,
    },
    /// Bảng dữ liệu (phòng, thành viên, kết quả).
    Table {
        /// Vùng vẽ.
        rect: Rect,
        /// Tên cột.
        columns: Vec<String>,
        /// Các dòng.
        rows: Vec<TableRow>,
    },
    /// Khung chat chờ.
    Chat {
        /// Vùng vẽ.
        rect: Rect,
        /// Các dòng chat.
        lines: Vec<String>,
    },
    /// Nút bấm.
    Button {
        /// Vùng vẽ.
        rect: Rect,
        /// Nhãn nút.
        label: String,
        /// Hành động khi bấm.
        action: ActionId,
        /// Nút dùng được không.
        enabled: bool,
        /// Nút chính của màn.
        primary: bool,
    },
    /// Khung màu thay ảnh chưa duyệt hoặc ảnh bị loại.
    Frame {
        /// Vùng vẽ.
        rect: Rect,
        /// Text catalog thay cho ảnh.
        label: String,
        /// Loại khung.
        tone: Tone,
    },
    /// Ảnh UI đã duyệt (`selected`).
    Art {
        /// Vùng vẽ.
        rect: Rect,
        /// ID sprite chứa ảnh trong bundle.
        sprite: String,
        /// Nhãn (text catalog) đi kèm ảnh.
        label: String,
    },
    /// Vùng bản đồ của màn trận.
    MapArea {
        /// Vùng vẽ.
        rect: Rect,
    },
}

/// Layout của một màn: danh sách phần tử đã định vị.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Màn mà layout này thuộc về.
    pub screen: UiScreen,
    /// Bề rộng màn (px).
    pub width: u32,
    /// Cao màn (px).
    pub height: u32,
    /// Các phần tử, theo thứ tự vẽ.
    pub widgets: Vec<Widget>,
}

/// Dựng layout của màn hiện tại.
///
/// `bundle` và `scene` là tuỳ chọn: có bundle thì ảnh UI đã duyệt (`selected`) và
/// bản đồ ở màn trận được đưa vào layout; không có thì màn vẫn dựng được (khung màu
/// + text catalog).
pub fn layout<B: UiBackend>(
    model: &UiModel<B>,
    bundle: Option<&Bundle>,
    scene: Option<&Scene>,
) -> Layout {
    let mut widgets = vec![Widget::Frame {
        rect: Rect::new(0, 0, WIDTH, HEIGHT),
        label: String::new(),
        tone: Tone::Panel,
    }];
    match model.screen() {
        UiScreen::Login => login_layout(model, &mut widgets),
        UiScreen::Lobby => lobby_layout(model, &mut widgets),
        UiScreen::Room => room_layout(model, &mut widgets),
        UiScreen::Match => match_layout(model, scene, &mut widgets),
        UiScreen::Result => result_layout(model, &mut widgets),
    }
    ui_elements_layout(model, bundle, &mut widgets);
    Layout {
        screen: model.screen(),
        width: WIDTH,
        height: HEIGHT,
        widgets,
    }
}

/// Màn đăng nhập: hai ô nhập và các nút.
fn login_layout<B: UiBackend>(model: &UiModel<B>, widgets: &mut Vec<Widget>) {
    let texts = model.texts();
    title(texts, "login.title", widgets);
    notice(model, widgets);
    widgets.push(Widget::Field {
        rect: Rect::new(MARGIN, 120, 400, 44),
        label: texts.get("login.account"),
        value: model.field_value(FieldId::LoginAccount).to_owned(),
        focused: model.focus() == Focus::Field(FieldId::LoginAccount),
        masked: false,
    });
    widgets.push(Widget::Field {
        rect: Rect::new(MARGIN, 196, 400, 44),
        label: texts.get("login.password"),
        value: model.field_value(FieldId::LoginPassword).to_owned(),
        focused: model.focus() == Focus::Field(FieldId::LoginPassword),
        masked: true,
    });
    buttons(model, widgets, MARGIN, BUTTON_Y);
}

/// Màn sảnh: danh sách phòng và ô đặt tên phòng mới.
fn lobby_layout<B: UiBackend>(model: &UiModel<B>, widgets: &mut Vec<Widget>) {
    let texts = model.texts();
    title(texts, "lobby.title", widgets);
    notice(model, widgets);
    let columns = vec![
        texts.get("lobby.room_name"),
        texts.get("lobby.room_host"),
        texts.get("lobby.room_count"),
        texts.get("lobby.room_status"),
    ];
    let rows = model
        .rooms()
        .iter()
        .map(|room| TableRow {
            cells: vec![
                room.name.clone(),
                room.host.clone(),
                format!("{}/{}", room.players, room.max_players),
                match room.phase {
                    RoomPhase::Waiting => texts.get("lobby.state_waiting"),
                    RoomPhase::Playing => texts.get("lobby.state_playing"),
                },
            ],
            selected: model.selected_room().is_some_and(|item| item.id == room.id),
        })
        .collect();
    widgets.push(Widget::Table {
        rect: CONTENT_RECT,
        columns,
        rows,
    });
    widgets.push(Widget::Field {
        rect: FIELD_RECT,
        label: texts.get("room.title_label"),
        value: model.field_value(FieldId::LobbyRoomName).to_owned(),
        focused: model.focus() == Focus::Field(FieldId::LobbyRoomName),
        masked: false,
    });
    buttons(model, widgets, MARGIN, BUTTON_Y);
}

/// Màn phòng chờ: thành viên, chat và các nút.
fn room_layout<B: UiBackend>(model: &UiModel<B>, widgets: &mut Vec<Widget>) {
    let texts = model.texts();
    let room = model.room();
    let title_text = room
        .as_ref()
        .map_or_else(|| texts.get("room.title_label"), |room| room.name.clone());
    widgets.push(Widget::Title {
        rect: TITLE_RECT,
        text: title_text,
    });
    room_notice(model, widgets);
    if let Some(room) = &room {
        let columns = vec![
            texts.get("room.players"),
            texts.get("room.host"),
            texts.get("room.status"),
            texts.get("room.character"),
        ];
        let rows = room
            .members
            .iter()
            .enumerate()
            .map(|(index, member)| TableRow {
                cells: vec![
                    member.name.clone(),
                    if member.host {
                        member.name.clone()
                    } else {
                        String::new()
                    },
                    if member.ready {
                        texts.get("room.ready")
                    } else {
                        texts.get("room.ready_cancel")
                    },
                    character_label(texts, member.character),
                ],
                selected: model.selected_member_index() == Some(index),
            })
            .collect();
        widgets.push(Widget::Table {
            rect: Rect::new(MARGIN, 88, 600, 200),
            columns,
            rows,
        });
        let lines = room
            .chat
            .iter()
            .map(|line| format!("{}: {}", line.author, line.text))
            .collect();
        widgets.push(Widget::Chat {
            rect: Rect::new(MARGIN, 300, 580, 130),
            lines,
        });
    }
    widgets.push(Widget::Field {
        rect: FIELD_RECT,
        label: texts.get("room.chat_to_all"),
        value: model.field_value(FieldId::RoomChat).to_owned(),
        focused: model.focus() == Focus::Field(FieldId::RoomChat),
        masked: false,
    });
    buttons(model, widgets, MARGIN, BUTTON_Y);
}

/// Màn trận: bản đồ và trạng thái lượt.
fn match_layout<B: UiBackend>(
    model: &UiModel<B>,
    scene: Option<&Scene>,
    widgets: &mut Vec<Widget>,
) {
    let texts = model.texts();
    widgets.push(Widget::Title {
        rect: Rect::new(MARGIN, 4, 400, 26),
        text: texts.get("room.map_name"),
    });
    let status = match model.match_progress() {
        Some(MatchProgress::Loading { remaining }) => {
            format!("{} {remaining}", texts.get("room.loading"))
        }
        Some(MatchProgress::Playing) | Some(MatchProgress::Finished(_)) => {
            texts.get("match.result_pending")
        }
        None => texts.get("app.loading"),
    };
    widgets.push(Widget::Notice {
        rect: Rect::new(440, 4, 180, 26),
        text: status,
        error: false,
    });
    // Bản đồ vẽ ở kích thước gốc (15×13 ô × 40 px = 600×520 cho Village00).
    let rect = scene.map_or(Rect::new(MARGIN, MATCH_MAP_Y, 600, 400), |scene| {
        let size = scene.map_size();
        Rect::new(MARGIN, MATCH_MAP_Y, size[0], size[1])
    });
    widgets.push(Widget::MapArea { rect });
    buttons(model, widgets, MATCH_BUTTON_X, MATCH_BUTTON_Y);
}

/// Màn kết quả: bảng hạng và kết quả.
fn result_layout<B: UiBackend>(model: &UiModel<B>, widgets: &mut Vec<Widget>) {
    let texts = model.texts();
    title(texts, "result.title", widgets);
    let outcome = model.outcome();
    let summary = outcome.map_or_else(
        || texts.get("match.result_pending"),
        |outcome| match outcome.self_result {
            MatchResult::Win => texts.get("result.win"),
            MatchResult::Lose => texts.get("result.lose"),
            MatchResult::Draw => texts.get("result.draw_all"),
        },
    );
    widgets.push(Widget::Notice {
        rect: NOTICE_RECT,
        text: summary,
        error: false,
    });
    let columns = vec![
        texts.get("result.rank"),
        texts.get("result.player"),
        texts.get("result.title"),
    ];
    let rows = outcome
        .map(|outcome| {
            outcome
                .entries
                .iter()
                .map(|entry| TableRow {
                    cells: vec![
                        entry.rank.to_string(),
                        entry.name.clone(),
                        match entry.result {
                            MatchResult::Win => texts.get("result.win"),
                            MatchResult::Lose => texts.get("result.lose"),
                            MatchResult::Draw => texts.get("result.draw"),
                        },
                    ],
                    selected: false,
                })
                .collect()
        })
        .unwrap_or_default();
    widgets.push(Widget::Table {
        rect: CONTENT_RECT,
        columns,
        rows,
    });
    buttons(model, widgets, MARGIN, BUTTON_Y);
}

/// Hàng nút của màn hiện tại.
fn buttons<B: UiBackend>(model: &UiModel<B>, widgets: &mut Vec<Widget>, x: i32, y: i32) {
    for (index, action) in model.actions().iter().enumerate() {
        let offset = i32::try_from(index).unwrap_or(0)
            * (i32::try_from(BUTTON_WIDTH).unwrap_or(0) + BUTTON_GAP);
        widgets.push(Widget::Button {
            rect: Rect::new(x + offset, y, BUTTON_WIDTH, BUTTON_HEIGHT),
            label: model.texts().get(action.key),
            action: action.id,
            enabled: action.enabled,
            primary: action.primary,
        });
    }
}

/// Dòng tiêu đề theo khóa chuỗi.
fn title(texts: &Texts, key: &str, widgets: &mut Vec<Widget>) {
    widgets.push(Widget::Title {
        rect: TITLE_RECT,
        text: texts.get(key),
    });
}

/// Dòng thông báo: lỗi đang có thì hiện lỗi, không thì bỏ trống.
fn notice<B: UiBackend>(model: &UiModel<B>, widgets: &mut Vec<Widget>) {
    let Some(key) = model.notice() else {
        return;
    };
    widgets.push(Widget::Notice {
        rect: NOTICE_RECT,
        text: model.texts().get(key),
        error: true,
    });
}

/// Dòng thông báo của màn phòng chờ: lỗi, hoặc nhân vật đang chọn.
fn room_notice<B: UiBackend>(model: &UiModel<B>, widgets: &mut Vec<Widget>) {
    let texts = model.texts();
    if let Some(key) = model.notice() {
        widgets.push(Widget::Notice {
            rect: NOTICE_RECT,
            text: texts.get(key),
            error: true,
        });
        return;
    }
    let Some(room) = model.room() else {
        return;
    };
    widgets.push(Widget::Notice {
        rect: NOTICE_RECT,
        text: format!(
            "{}: {}",
            texts.get("room.character"),
            character_label(texts, room.self_character)
        ),
        error: false,
    });
}

/// Tên hiển thị của nhân vật theo khóa chuỗi.
fn character_label(texts: &Texts, character: Character) -> String {
    match character {
        Character::C00 => texts.get("room.character_c00"),
        Character::C01 => texts.get("room.character_c01"),
    }
}

/// Ảnh và khung thay ảnh của bảng `[[ui]]` trong bundle.
///
/// - Phần tử `selected` (đã duyệt, không có chữ cần duyệt) → vẽ **ảnh**.
/// - Phần tử `needs_review_text`/`excluded` → **khung màu + text catalog**, không
///   hiện ảnh (UI_STRINGS_V1 §3).
fn ui_elements_layout<B: UiBackend>(
    model: &UiModel<B>,
    bundle: Option<&Bundle>,
    widgets: &mut Vec<Widget>,
) {
    let Some(bundle) = bundle else {
        return;
    };
    let texts = model.texts();
    let elements = bundle.ui_by_screen(model.screen());
    let art_count = u32::try_from(
        elements
            .iter()
            .filter(|element| element.state == UiState::Selected)
            .count(),
    )
    .unwrap_or(0);
    let frame_count = u32::try_from(elements.len())
        .unwrap_or(0)
        .saturating_sub(art_count);
    // Chia chiều cao cột phải cho các ô: ảnh trước, khung thay ảnh sau. Chiều cao
    // ô khung được tính lại để mọi phần tử đều vừa cột (không tràn xuống nút).
    let available = SIDE_RECT.height;
    let art_height = available
        .checked_div(art_count)
        .unwrap_or(0)
        .min(ART_HEIGHT);
    let frame_available = available.saturating_sub(art_height.saturating_mul(art_count));
    let frame_height = frame_available
        .checked_div(frame_count)
        .unwrap_or(0)
        .clamp(FRAME_MIN_HEIGHT, FRAME_HEIGHT);

    let mut y = SIDE_RECT.y;
    for element in elements {
        match element.state {
            UiState::Selected => {
                let Some(sprite) = element.sprite.as_deref() else {
                    continue;
                };
                let rect = Rect::new(SIDE_RECT.x, y, SIDE_RECT.width, art_height);
                y += i32::try_from(art_height).unwrap_or(0);
                widgets.push(Widget::Art {
                    rect,
                    sprite: sprite.to_owned(),
                    label: texts.get(&element.id),
                });
            }
            UiState::NeedsReviewText => {
                let rect = Rect::new(SIDE_RECT.x, y, SIDE_RECT.width, frame_height);
                y += i32::try_from(frame_height).unwrap_or(0);
                widgets.push(Widget::Frame {
                    rect,
                    label: texts.get(&element.id),
                    tone: Tone::Pending,
                });
            }
            UiState::Excluded => {
                let rect = Rect::new(SIDE_RECT.x, y, SIDE_RECT.width, frame_height);
                y += i32::try_from(frame_height).unwrap_or(0);
                widgets.push(Widget::Frame {
                    rect,
                    label: texts.get(&element.id),
                    tone: Tone::Excluded,
                });
            }
        }
    }
}
