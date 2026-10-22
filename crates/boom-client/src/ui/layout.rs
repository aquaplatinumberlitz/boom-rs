//! Layout của màn UI: toạ độ logic của từng phần tử (hàm thuần, không GPU).
//!
//! Module này biến [`UiModel`] thành danh sách phần tử đã **định vị** để phần vẽ
//! ([`crate::ui::draw`]) chỉ còn đọc toạ độ và màu. Vì vậy toạ độ, thứ tự và nội dung
//! hiển thị đều kiểm được **không cần GPU** — phần nghiệm thu chính của V1-032.
//!
//! ## Quy ước
//!
//! - Màn có kích thước logic **800×600** như gốc asset (UI_STRINGS_V1 §5.2).
//! - Chữ lấy từ file chuỗi V1; khóa chưa có text hiện placeholder `«khóa»`.
//! - Ảnh UI chỉ hiện khi phần tử ở trạng thái `selected` (đã duyệt, không có chữ
//!   cần duyệt). Phần tử `needs_review_text`/`excluded` được thay bằng **khung màu +
//!   text catalog** (UI_STRINGS_V1 §3) — không hiện ảnh chưa duyệt.
//! - Mỗi đoạn chữ có **một hộp** ([`TextBox`]): phần vẽ và test tràn hộp dùng chung
//!   hàm [`Widget::text_boxes`] nên đo khớp vẽ (UI_STRINGS_V1 §5.2.1).
//! - Hộp chữ là **giới hạn cả hai chiều**: chữ xuống dòng theo bề rộng hộp và bị
//!   cắt theo chiều cao hộp ([`crate::text::Clip`]). Nội dung động (chat dài,
//!   tên phòng dài, ô nhập dài) không tràn ra ngoài hộp.

use boom_content::bundle::{Bundle, UiScreen, UiState};

use crate::anim::Character;
use crate::scene::Scene;
use crate::text::{Clip, TextFont};
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
/// Vùng dòng thông báo (lỗi hoặc trạng thái) — cao đủ hai dòng chữ 16 px.
const NOTICE_RECT: Rect = Rect {
    x: MARGIN,
    y: 52,
    width: 752,
    height: 48,
};
/// Vùng nội dung chính (bảng, chat).
const CONTENT_RECT: Rect = Rect {
    x: MARGIN,
    y: 104,
    width: 580,
    height: 324,
};
/// Vùng cột phải: ảnh UI đã duyệt và khung thay ảnh chưa duyệt.
const SIDE_RECT: Rect = Rect {
    x: 636,
    y: 104,
    width: 156,
    height: 376,
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

/// Cỡ chữ theo vai trò hiển thị (px).
///
/// Nguồn **chung** cho phần vẽ ([`crate::ui::draw`]) và test tràn hộp: đo và vẽ phải
/// dùng cùng cỡ chữ, không thì test không kiểm được điều người chơi thấy.
pub mod text_size {
    /// Tiêu đề màn.
    pub const TITLE: f32 = 24.0;
    /// Dòng thông báo.
    pub const NOTICE: f32 = 16.0;
    /// Nhãn ô nhập.
    pub const LABEL: f32 = 14.0;
    /// Giá trị trong ô nhập.
    pub const VALUE: f32 = 18.0;
    /// Đầu bảng.
    pub const HEADER: f32 = 13.0;
    /// Dòng bảng.
    pub const ROW: f32 = 14.0;
    /// Nhãn nút.
    pub const BUTTON: f32 = 16.0;
    /// Chữ nhỏ (khung thay ảnh, chat).
    pub const SMALL: f32 = 13.0;
}

/// Lề trong của khung thay ảnh (px) — nhãn nằm trong khung.
pub const FRAME_INSET: f32 = 4.0;
/// Lề trong của ô nhập (px) — giá trị nằm trong khung.
pub const FIELD_INSET: f32 = 8.0;
/// Khoảng cách giữa mép trên khung ô nhập và nhãn của ô (px).
pub const FIELD_LABEL_GAP: f32 = 22.0;
/// Lề dưới của ô nhập (px) — chừa chỗ cho đường viền.
pub const FIELD_BOTTOM_GAP: f32 = 10.0;
/// Chiều cao vùng đầu bảng (px) — dòng đầu nằm dưới vùng này.
pub const TABLE_HEADER_HEIGHT: f32 = 22.0;
/// Chiều cao một dòng bảng (px).
pub const TABLE_ROW_HEIGHT: f32 = 26.0;
/// Khoảng cách giữa hai cột của bảng (px).
pub const TABLE_COLUMN_GAP: f32 = 18.0;
/// Chiều cao một dòng chat (px).
pub const CHAT_LINE_HEIGHT: f32 = 20.0;

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

/// Vai trò hiển thị của một đoạn chữ (quyết định màu vẽ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    /// Tiêu đề màn.
    Title,
    /// Thông báo thường.
    Notice,
    /// Thông báo lỗi.
    Error,
    /// Nhãn ô nhập.
    Label,
    /// Giá trị trong ô nhập.
    Value,
    /// Đầu bảng.
    Header,
    /// Dòng bảng.
    Row,
    /// Nhãn nút.
    Button,
    /// Nhãn nút không dùng được.
    ButtonDisabled,
    /// Chữ nhỏ (khung thay ảnh, chat).
    Small,
}

/// Căn ngang của đoạn chữ trong hộp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// Căn trái.
    Left,
    /// Căn giữa.
    Center,
}

/// Một hộp chữ: nội dung, cỡ chữ và vùng được vẽ.
///
/// Chữ **xuống dòng theo bề rộng hộp** và **bị cắt theo chiều cao hộp**
/// (UI_STRINGS_V1 §5.2.1): hộp là giới hạn mà phần vẽ và test tràn hộp cùng
/// dùng, nên test kiểm được đúng thứ người chơi thấy.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBox {
    /// Nội dung chữ (ô mật khẩu đã thay bằng ký tự che).
    pub text: String,
    /// Cỡ chữ (px).
    pub size: f32,
    /// Vai trò hiển thị → màu vẽ.
    pub role: TextRole,
    /// Góc trên-trái của hộp (px, không gian logic UI).
    pub x: f32,
    /// Góc trên-trái của hộp (px, không gian logic UI).
    pub y: f32,
    /// Bề rộng khung chữ (px).
    pub width: f32,
    /// Bề cao khung chữ (px).
    pub height: f32,
    /// Căn ngang trong hộp.
    pub align: Align,
    /// Giữ phần đầu hay phần cuối khi chữ không vừa chiều cao hộp.
    pub clip: Clip,
}

impl TextBox {
    /// Chữ **được vẽ** trong hộp này.
    ///
    /// Dùng chung cho phần vẽ ([`crate::ui::draw`]) và test tràn hộp nên đo
    /// khớp vẽ: hộp là giới hạn cả bề rộng lẫn chiều cao, chữ vừa hộp mới được
    /// vẽ (UI_STRINGS_V1 §5.2.1).
    pub fn laid_out(&self, font: &TextFont) -> crate::text::Layout {
        crate::text::layout_in_box(
            &self.text,
            font,
            self.size,
            self.width,
            self.height,
            self.clip,
        )
    }
}

impl Widget {
    /// Các hộp chữ của phần tử: nội dung, cỡ chữ và vùng vẽ.
    ///
    /// Phần vẽ ([`crate::ui::draw`]) và test tràn hộp dùng **cùng** hàm này nên đo
    /// khớp vẽ; `font` cần cho bảng vì bề rộng cột là ô rộng nhất của cột.
    ///
    /// Mọi hộp đều **giới hạn chiều cao**: chữ dài hơn hộp bị cắt theo
    /// [`Clip`], không tràn ra ngoài (UI_STRINGS_V1 §5.2.1).
    pub fn text_boxes(&self, font: &TextFont) -> Vec<TextBox> {
        match self {
            Self::Title { rect, text } => vec![TextBox {
                text: text.clone(),
                size: text_size::TITLE,
                role: TextRole::Title,
                x: rect.x as f32,
                y: rect.y as f32,
                width: rect.width as f32,
                height: rect.height as f32,
                align: Align::Left,
                clip: Clip::Head,
            }],
            Self::Notice { rect, text, error } => vec![TextBox {
                text: text.clone(),
                size: text_size::NOTICE,
                role: if *error {
                    TextRole::Error
                } else {
                    TextRole::Notice
                },
                x: rect.x as f32,
                y: rect.y as f32,
                width: rect.width as f32,
                height: rect.height as f32,
                align: Align::Left,
                clip: Clip::Head,
            }],
            Self::Field {
                rect,
                label,
                value,
                masked,
                ..
            } => {
                let shown = if *masked {
                    MASK_CHAR.to_string().repeat(value.chars().count())
                } else {
                    value.clone()
                };
                vec![
                    TextBox {
                        text: label.clone(),
                        size: text_size::LABEL,
                        role: TextRole::Label,
                        x: rect.x as f32,
                        y: rect.y as f32 - FIELD_LABEL_GAP,
                        width: rect.width as f32,
                        height: FIELD_LABEL_GAP,
                        align: Align::Left,
                        clip: Clip::Head,
                    },
                    TextBox {
                        text: shown,
                        size: text_size::VALUE,
                        role: TextRole::Value,
                        x: rect.x as f32 + FIELD_INSET,
                        y: rect.y as f32 + FIELD_BOTTOM_GAP,
                        width: rect.width as f32 - 2.0 * FIELD_INSET,
                        height: rect.height as f32 - FIELD_BOTTOM_GAP,
                        align: Align::Left,
                        // Ô nhập: giữ **cuối** để thấy phần vừa gõ.
                        clip: Clip::Tail,
                    },
                ]
            }
            Self::Table {
                rect,
                columns,
                rows,
            } => {
                // Bề rộng cột được giới hạn cho vừa khung: bảng không tràn ra
                // ngoài dù dữ liệu động (tên phòng/tên người) dài.
                let widths = table_column_widths_fitted(columns, rows, font, rect.width as f32);
                // Dòng cao theo nội dung thật: ô xuống nhiều dòng thì dòng cao hơn,
                // tên dài **không bị cắt** (UI_STRINGS_V1 §5.2).
                let heights = table_row_heights(
                    rows,
                    font,
                    &widths,
                    rect.height as f32 - TABLE_HEADER_HEIGHT,
                );
                let mut boxes = Vec::with_capacity(columns.len() + rows.len() * columns.len());
                let mut x = rect.x as f32;
                for (index, column) in columns.iter().enumerate() {
                    let width = widths.get(index).copied().unwrap_or(0.0);
                    boxes.push(TextBox {
                        text: column.clone(),
                        size: text_size::HEADER,
                        role: TextRole::Header,
                        x,
                        y: rect.y as f32,
                        width,
                        height: TABLE_HEADER_HEIGHT,
                        align: Align::Left,
                        clip: Clip::Head,
                    });
                    x += width + TABLE_COLUMN_GAP;
                }
                let mut y = rect.y as f32 + TABLE_HEADER_HEIGHT;
                for (row_index, row) in rows.iter().enumerate() {
                    let height = heights.get(row_index).copied().unwrap_or(TABLE_ROW_HEIGHT);
                    // Dòng vượt khung không được vẽ (tràn xuống dưới).
                    if y + height > rect.bottom() as f32 {
                        break;
                    }
                    for (index, cell) in row.cells.iter().enumerate() {
                        let width = widths.get(index).copied().unwrap_or(0.0);
                        boxes.push(TextBox {
                            text: cell.clone(),
                            size: text_size::ROW,
                            role: TextRole::Row,
                            x: x_of_column(&widths, rect.x as f32, index),
                            y,
                            width,
                            height,
                            align: Align::Left,
                            clip: Clip::Head,
                        });
                    }
                    y += height;
                }
                boxes
            }
            Self::Chat { rect, lines } => chat_text_boxes(rect, lines, font),
            Self::Button {
                rect,
                label,
                enabled,
                ..
            } => vec![TextBox {
                text: label.clone(),
                size: text_size::BUTTON,
                role: if *enabled {
                    TextRole::Button
                } else {
                    TextRole::ButtonDisabled
                },
                x: rect.x as f32,
                y: rect.y as f32,
                width: rect.width as f32,
                height: rect.height as f32,
                align: Align::Center,
                clip: Clip::Head,
            }],
            Self::Frame { rect, label, .. } => {
                if label.is_empty() {
                    Vec::new()
                } else {
                    vec![TextBox {
                        text: label.clone(),
                        size: text_size::SMALL,
                        role: TextRole::Small,
                        x: rect.x as f32 + FRAME_INSET,
                        y: rect.y as f32 + FRAME_INSET,
                        width: rect.width as f32 - 2.0 * FRAME_INSET,
                        height: rect.height as f32 - 2.0 * FRAME_INSET,
                        align: Align::Left,
                        clip: Clip::Head,
                    }]
                }
            }
            Self::Art { rect, label, .. } => {
                if label.is_empty() {
                    Vec::new()
                } else {
                    vec![TextBox {
                        text: label.clone(),
                        size: text_size::SMALL,
                        role: TextRole::Small,
                        x: rect.x as f32 + FRAME_INSET,
                        y: rect.bottom() as f32 - CHAT_LINE_HEIGHT,
                        width: rect.width as f32 - 2.0 * FRAME_INSET,
                        height: CHAT_LINE_HEIGHT,
                        align: Align::Left,
                        clip: Clip::Head,
                    }]
                }
            }
            Self::MapArea { .. } => Vec::new(),
        }
    }
}

/// Hộp chữ của khung chat: mỗi tin chiếm **chiều cao thật** sau khi xuống dòng.
///
/// Policy (nội dung động, V1-035):
///
/// - Tin dài xuống nhiều dòng thì chiếm nhiều chiều cao; tin sau **không** bị
///   chồng lên tin trước (mỗi tin là một hộp riêng).
/// - Tin mới nhất nằm sát đáy khung; tin cũ hơn bị **cuộn** ra khung (không vẽ).
/// - Tin cao hơn cả khung thì giữ phần **cuối** của tin (nội dung mới nhất).
fn chat_text_boxes(rect: &Rect, lines: &[String], font: &TextFont) -> Vec<TextBox> {
    let frame_height = rect.height as f32;
    let heights: Vec<f32> = lines
        .iter()
        .map(|line| {
            let laid = crate::text::layout(line, font, text_size::SMALL, rect.width as f32);
            laid.height().max(CHAT_LINE_HEIGHT)
        })
        .collect();
    let mut boxes = Vec::new();
    // Xếp từ dưới lên: tin mới nhất sát đáy khung.
    let mut bottom = rect.bottom() as f32;
    for index in (0..lines.len()).rev() {
        let height = heights[index].min(frame_height);
        let y = bottom - height;
        // Tin không vừa phần trống còn lại → tin này và các tin cũ hơn không vẽ.
        if y < rect.y as f32 - 0.5 {
            break;
        }
        boxes.push(TextBox {
            text: lines[index].clone(),
            size: text_size::SMALL,
            role: TextRole::Small,
            x: rect.x as f32,
            y,
            width: rect.width as f32,
            height,
            align: Align::Left,
            // Tin cao hơn khung: giữ cuối (phần mới nhất của tin đó).
            clip: if heights[index] > frame_height {
                Clip::Tail
            } else {
                Clip::Head
            },
        });
        bottom = y;
    }
    boxes.reverse();
    boxes
}

/// Ký tự che giá trị ô mật khẩu.
const MASK_CHAR: &str = "•";

/// Bề rộng từng cột của bảng: cột rộng bằng ô rộng nhất (đầu bảng hoặc dòng).
///
/// Dùng chung cho phần vẽ và test nên bảng không tràn khung. Cột rộng thêm 1 px
/// để chữ vừa đúng bề rộng đo được không bị xuống dòng do làm tròn.
pub fn table_column_widths(columns: &[String], rows: &[TableRow], font: &TextFont) -> Vec<f32> {
    let mut widths: Vec<f32> = columns
        .iter()
        .map(|column| measure_width(column, font, text_size::HEADER))
        .collect();
    for row in rows {
        for (index, cell) in row.cells.iter().enumerate() {
            let width = measure_width(cell, font, text_size::ROW);
            if let Some(slot) = widths.get_mut(index) {
                *slot = slot.max(width);
            }
        }
    }
    for width in &mut widths {
        *width += 1.0;
    }
    widths
}

/// Bề rộng cột **sau khi giới hạn** cho vừa khung bảng.
///
/// Bảng V1 có dữ liệu động (tên phòng, tên người): ô rộng nhất có thể vượt khung
/// thì bảng tràn ra ngoài màn. Ở đây bề rộng cột được cắt (cột rộng nhất trước)
/// cho tổng bề rộng + khoảng cách vừa `max_width`; chữ trong ô vượt bề rộng cột
/// thì xuống dòng và bị cắt theo chiều cao dòng — không tràn ra ngoài.
pub fn table_column_widths_fitted(
    columns: &[String],
    rows: &[TableRow],
    font: &TextFont,
    max_width: f32,
) -> Vec<f32> {
    let mut widths = table_column_widths(columns, rows, font);
    let gaps = TABLE_COLUMN_GAP * widths.len().saturating_sub(1) as f32;
    let available = (max_width - gaps).max(0.0);
    fit_widths(&mut widths, available);
    widths
}

/// Cắt bề rộng cột (cột rộng nhất trước) cho tổng vừa `available`.
fn fit_widths(widths: &mut [f32], available: f32) {
    loop {
        let total: f32 = widths.iter().sum();
        if total <= available + 0.5 || widths.is_empty() {
            return;
        }
        let widest = widths.iter().copied().fold(0.0f32, f32::max);
        let count = widths
            .iter()
            .filter(|width| **width >= widest - 0.5)
            .count();
        if count == 0 {
            return;
        }
        // Mỗi lượt cắt đủ lượng thừa nên tổng vừa `available` sau một lượt.
        let share = (total - available) / count as f32;
        for width in widths.iter_mut() {
            if *width >= widest - 0.5 {
                *width = (*width - share).max(0.0);
            }
        }
    }
}

/// Chiều cao từng dòng của bảng.
///
/// Dòng cao theo nội dung thật: ô xuống nhiều dòng thì dòng cao hơn, nên tên
/// dài **được xuống dòng** chứ không bị cắt (UI_STRINGS_V1 §5.2). Dòng không có ô
/// nào xuống dòng thì cao bằng [`TABLE_ROW_HEIGHT`].
///
/// `max_height` là chiều cao còn trống của khung bảng: một dòng không được cao
/// hơn cả khung (nếu không thì dòng đó không vẽ và các dòng sau bị đẩy ra).
///
/// Dùng chung cho phần vẽ và layout nên tô dòng đang chọn khớp với dòng chữ.
pub fn table_row_heights(
    rows: &[TableRow],
    font: &TextFont,
    widths: &[f32],
    max_height: f32,
) -> Vec<f32> {
    rows.iter()
        .map(|row| {
            let mut height = TABLE_ROW_HEIGHT;
            for (index, cell) in row.cells.iter().enumerate() {
                let width = widths.get(index).copied().unwrap_or(0.0);
                let laid = crate::text::layout(cell, font, text_size::ROW, width);
                height = height.max(laid.height());
            }
            height.min(max_height.max(TABLE_ROW_HEIGHT))
        })
        .collect()
}

/// Bề rộng một chuỗi (px) theo metric của font, không xuống dòng.
fn measure_width(text: &str, font: &TextFont, size: f32) -> f32 {
    crate::text::layout(text, font, size, f32::MAX).width()
}

/// Toạ độ X của cột thứ `index` trong bảng.
fn x_of_column(widths: &[f32], start: f32, index: usize) -> f32 {
    let mut x = start;
    for width in widths.iter().take(index) {
        x += width + TABLE_COLUMN_GAP;
    }
    x
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
        rect: Rect::new(MARGIN, 130, 400, 44),
        label: texts.get("login.account"),
        value: model.field_value(FieldId::LoginAccount).to_owned(),
        focused: model.focus() == Focus::Field(FieldId::LoginAccount),
        masked: false,
    });
    widgets.push(Widget::Field {
        rect: Rect::new(MARGIN, 206, 400, 44),
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
            rect: Rect::new(MARGIN, 104, 600, 190),
            columns,
            rows,
        });
        let lines = room
            .chat
            .iter()
            .map(|line| format!("{}: {}", line.author, line.text))
            .collect();
        widgets.push(Widget::Chat {
            rect: Rect::new(MARGIN, 300, 580, 128),
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

/// Chữ của màn trận cho người chơi: (tên map, trạng thái lượt).
///
/// Lấy từ file chuỗi V1 theo khóa `UI_STRINGS_V1`; dùng chung cho layout V1 và cho
/// lớp chữ vẽ trên scene (module [`crate::ui::draw`]) nên màn trận nói cùng một
/// chữ, và chữ đó không viết tay trong mã.
pub fn match_status_texts<B: UiBackend>(model: &UiModel<B>) -> (String, String) {
    let texts = model.texts();
    let title = texts.get("room.map_name");
    let status = match model.match_progress() {
        Some(MatchProgress::Loading { remaining }) => {
            format!("{} {remaining}", texts.get("room.loading"))
        }
        Some(MatchProgress::Playing) | Some(MatchProgress::Finished(_)) => {
            texts.get("match.result_pending")
        }
        None => texts.get("app.loading"),
    };
    (title, status)
}

/// Màn trận: bản đồ và trạng thái lượt.
fn match_layout<B: UiBackend>(
    model: &UiModel<B>,
    scene: Option<&Scene>,
    widgets: &mut Vec<Widget>,
) {
    let (title, status) = match_status_texts(model);
    widgets.push(Widget::Title {
        rect: Rect::new(MARGIN, 4, 400, 34),
        text: title,
    });
    widgets.push(Widget::Notice {
        rect: Rect::new(440, 4, 336, 24),
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
