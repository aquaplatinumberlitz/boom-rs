//! Vẽ layout của UI lên màn hình (macroquad, ADR 0010).
//!
//! Module này **mỏng**: chỉ đọc [`crate::ui::layout::Layout`] rồi vẽ. Mọi quyết định
//! về nội dung, thứ tự và toạ độ nằm ở module thuần [`crate::ui::layout`] — nhờ vậy
//! phần logic kiểm được không cần GPU, còn phần này chỉ cần một con mắt.
//!
//! ## Font
//!
//! Chữ vẽ bằng **Be Vietnam Pro 1.002** (SIL OFL 1.1) nhúng trong binary
//! ([`crate::text`]), không phải font của hệ điều hành: cùng một bộ chữ trên mọi
//! máy, và ảnh chụp của client khớp với golden layout (UI_STRINGS_V1 §5.1.1).
//!
//! ## Wrap
//!
//! Mỗi đoạn chữ có một hộp ([`crate::ui::layout::TextBox`]); chữ xuống dòng theo bề
//! rộng hộp và không bao giờ vẽ ra ngoài hộp (UI_STRINGS_V1 §5.2.1). Ký tự không có
//! glyph trong font được vẽ thành **hộp báo thiếu**, không bỏ qua âm thầm (AGENTS §4).

use boom_content::bundle::Bundle;
use boom_content::map::CellKind;
use macroquad::prelude::*;

use crate::render::Atlas;
use crate::scene::{DrawItem, PixelRect, Scene};
use crate::text::{GlyphCache, TextFont};
use crate::ui::layout::{
    Align, FIELD_BOTTOM_GAP, FIELD_INSET, Layout, Rect, TABLE_HEADER_HEIGHT, TABLE_ROW_HEIGHT,
    TextBox, TextRole, Tone, Widget,
};

/// Nền màn.
const BACKGROUND: Color = Color::from_rgba(14, 16, 22, 255);
/// Nền khung.
const PANEL: Color = Color::from_rgba(24, 28, 36, 255);
/// Màu chữ tiêu đề.
const TITLE_TEXT: Color = Color::from_rgba(235, 240, 250, 255);
/// Màu chữ thông báo thường.
const NOTICE_TEXT: Color = Color::from_rgba(150, 160, 175, 255);
/// Màu chữ thông báo lỗi.
const ERROR_TEXT: Color = Color::from_rgba(245, 130, 90, 255);
/// Màu viền ô nhập.
const FIELD_BORDER: Color = Color::from_rgba(70, 80, 96, 255);
/// Màu viền ô nhập đang focus.
const FIELD_FOCUS: Color = Color::from_rgba(120, 180, 250, 255);
/// Nền nút thường.
const BUTTON: Color = Color::from_rgba(40, 48, 62, 255);
/// Nền nút chính.
const BUTTON_PRIMARY: Color = Color::from_rgba(46, 92, 152, 255);
/// Nền nút không dùng được.
const BUTTON_DISABLED: Color = Color::from_rgba(30, 33, 40, 255);
/// Màu chữ nút.
const BUTTON_TEXT: Color = Color::from_rgba(225, 232, 240, 255);
/// Màu chữ nút không dùng được.
const BUTTON_TEXT_DISABLED: Color = Color::from_rgba(105, 112, 124, 255);
/// Màu chữ đầu bảng.
const HEADER_TEXT: Color = Color::from_rgba(140, 150, 165, 255);
/// Màu chữ dòng bảng.
const ROW_TEXT: Color = Color::from_rgba(215, 222, 232, 255);
/// Nền dòng bảng đang chọn.
const ROW_SELECTED: Color = Color::from_rgba(40, 60, 92, 255);
/// Nền khung thay ảnh chưa duyệt.
const FRAME_PENDING: Color = Color::from_rgba(92, 74, 36, 255);
/// Nền khung thay ảnh bị loại.
const FRAME_EXCLUDED: Color = Color::from_rgba(58, 60, 66, 255);
/// Màu chữ trong khung thay ảnh.
const FRAME_TEXT: Color = Color::from_rgba(226, 214, 180, 255);

/// Font của UI: font OFL đã bundle + cache glyph.
pub struct UiFont {
    font: TextFont,
    cache: GlyphCache,
}

impl UiFont {
    /// Font chính của V1 (Be Vietnam Pro 1.002, SIL OFL 1.1) nhúng trong binary.
    pub fn bundled() -> Self {
        Self {
            font: TextFont::regular().expect("font Be Vietnam Pro đã bundle phải nạp được"),
            cache: GlyphCache::new(),
        }
    }
}

/// Font chính của UI V1 (Be Vietnam Pro 1.002, SIL OFL 1.1) nhúng trong binary.
///
/// Không dùng font hệ thiết: cùng một bộ chữ trên mọi máy và ảnh chụp khớp golden
/// layout (UI_STRINGS_V1 §5.1.1).
pub fn bundled_font() -> UiFont {
    UiFont::bundled()
}

/// Màu chữ theo vai trò hiển thị.
fn role_color(role: TextRole) -> Color {
    match role {
        TextRole::Title => TITLE_TEXT,
        TextRole::Notice => NOTICE_TEXT,
        TextRole::Error => ERROR_TEXT,
        TextRole::Label => HEADER_TEXT,
        TextRole::Value => ROW_TEXT,
        TextRole::Header => HEADER_TEXT,
        TextRole::Row => ROW_TEXT,
        TextRole::Button => BUTTON_TEXT,
        TextRole::ButtonDisabled => BUTTON_TEXT_DISABLED,
        TextRole::Small => FRAME_TEXT,
    }
}

/// Vẽ một layout lên màn hình.
///
/// `atlas`/`bundle`/`scene` là tuỳ chọn: có thì ảnh UI đã duyệt và bản đồ ở màn
/// trận vẽ được; không có thì phần tử đó vẽ khung màu + text catalog.
pub fn draw(
    layout: &Layout,
    font: &mut UiFont,
    atlas: Option<&Atlas>,
    bundle: Option<&Bundle>,
    scene: Option<&Scene>,
) {
    clear_background(BACKGROUND);
    for widget in &layout.widgets {
        match widget {
            Widget::Frame { rect, tone, .. } => draw_frame(rect, *tone),
            Widget::Art { rect, sprite, .. } => draw_art(rect, sprite, atlas, bundle),
            Widget::MapArea { rect } => draw_map(rect, atlas, scene),
            Widget::Field { rect, focused, .. } => draw_field(rect, *focused),
            Widget::Button {
                rect,
                enabled,
                primary,
                ..
            } => draw_button(rect, *enabled, *primary),
            Widget::Table { rect, rows, .. } => draw_table_background(rect, rows),
            _ => {}
        }
        for text_box in widget.text_boxes(&font.font) {
            draw_text_box(font, &text_box);
        }
    }
}

/// Vẽ một hộp chữ: xuống dòng theo bề rộng hộp rồi vẽ từng dòng.
fn draw_text_box(font: &mut UiFont, text_box: &TextBox) {
    if text_box.text.is_empty() {
        return;
    }
    let laid = crate::text::layout(&text_box.text, &font.font, text_box.size, text_box.width);
    let x = match text_box.align {
        Align::Left => text_box.x,
        Align::Center => text_box.x + (text_box.width - laid.width()) / 2.0,
    };
    let y = match text_box.align {
        Align::Left => text_box.y,
        Align::Center => text_box.y + (text_box.height - laid.height()) / 2.0,
    };
    crate::text::draw(
        &mut font.cache,
        &font.font,
        &laid,
        x,
        y,
        role_color(text_box.role),
    );
}

/// Vẽ khung màu (nền màn hoặc khung thay ảnh chưa duyệt).
fn draw_frame(rect: &Rect, tone: Tone) {
    let color = match tone {
        Tone::Panel => PANEL,
        Tone::Pending => FRAME_PENDING,
        Tone::Excluded => FRAME_EXCLUDED,
    };
    draw_rectangle(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
        color,
    );
}

/// Vẽ ô nhập: khung và con trỏ focus (nhãn và giá trị do phần chữ vẽ).
fn draw_field(rect: &Rect, focused: bool) {
    let border = if focused { FIELD_FOCUS } else { FIELD_BORDER };
    draw_rectangle_lines(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
        2.0,
        border,
    );
    if focused {
        draw_rectangle(
            (rect.x as f32) + FIELD_INSET,
            (rect.y as f32) + FIELD_BOTTOM_GAP,
            2.0,
            20.0,
            FIELD_FOCUS,
        );
    }
}

/// Vẽ bảng: tô dòng đang chọn (đầu bảng và dòng do phần chữ vẽ).
fn draw_table_background(rect: &Rect, rows: &[crate::ui::layout::TableRow]) {
    for (index, row) in rows.iter().enumerate() {
        if !row.selected {
            continue;
        }
        let y = rect.y
            + TABLE_HEADER_HEIGHT as i32
            + i32::try_from(index).unwrap_or(0) * TABLE_ROW_HEIGHT as i32;
        if y as f32 + TABLE_ROW_HEIGHT > rect.bottom() as f32 {
            break;
        }
        draw_rectangle(
            (rect.x - 4) as f32,
            (y - 4) as f32,
            (rect.width + 8) as f32,
            TABLE_ROW_HEIGHT - 2.0,
            ROW_SELECTED,
        );
    }
}

/// Vẽ nút: nền theo trạng thái (nhãn do phần chữ vẽ).
fn draw_button(rect: &Rect, enabled: bool, primary: bool) {
    let background = if !enabled {
        BUTTON_DISABLED
    } else if primary {
        BUTTON_PRIMARY
    } else {
        BUTTON
    };
    draw_rectangle(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
        background,
    );
    draw_rectangle_lines(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
        1.0,
        if enabled { FIELD_FOCUS } else { FIELD_BORDER },
    );
}

/// Vẽ ảnh UI đã duyệt (`selected`).
///
/// Ảnh vẽ ở kích thước gốc, canh giữa trong ô. Nếu ảnh không vừa ô thì vẽ khung
/// thay — không cắt ảnh, không hiện ảnh khác.
fn draw_art(rect: &Rect, sprite: &str, atlas: Option<&Atlas>, bundle: Option<&Bundle>) {
    let Some(item) = art_item(bundle, sprite, rect) else {
        draw_frame(rect, Tone::Excluded);
        return;
    };
    let Some(atlas) = atlas else {
        draw_frame(rect, Tone::Excluded);
        return;
    };
    if item.source.width > rect.width || item.source.height > rect.height {
        // Ảnh không vừa ô: vẽ khung thay, không cắt ảnh.
        draw_frame(rect, Tone::Excluded);
        return;
    }
    // Ảnh vẽ ở kích thước gốc, canh giữa trong ô.
    let offset_x = (rect.width.saturating_sub(item.source.width)) as f32 / 2.0;
    let offset_y = (rect.height.saturating_sub(item.source.height)) as f32 / 2.0;
    atlas.draw(&item, (offset_x, offset_y));
}

/// Lệnh vẽ ảnh UI: cắt vùng của sprite trong atlas rồi đưa vào ô đã cấp.
fn art_item(bundle: Option<&Bundle>, sprite_id: &str, dest: &Rect) -> Option<DrawItem> {
    let bundle = bundle?;
    let sprite = bundle
        .sprites
        .iter()
        .find(|sprite| sprite.id == sprite_id)?;
    let frame = sprite.frames.iter().find(|frame| {
        sprite
            .images
            .get(frame.image as usize)
            .is_some_and(|image| image.width > 0 && image.height > 0)
    })?;
    let image = sprite.images.get(frame.image as usize)?;
    let page = image.page?;
    Some(DrawItem {
        page,
        source: PixelRect {
            x: i32::try_from(image.x).unwrap_or(i32::MAX) + frame.x0,
            y: i32::try_from(image.y).unwrap_or(i32::MAX) + frame.y0,
            width: u32::try_from((frame.x1 - frame.x0).max(0)).unwrap_or(0),
            height: u32::try_from((frame.y1 - frame.y0).max(0)).unwrap_or(0),
        },
        dest: PixelRect {
            x: dest.x,
            y: dest.y,
            width: dest.width,
            height: dest.height,
        },
        row: 0,
        column: 0,
        kind: CellKind::Floor,
    })
}

/// Vẽ bản đồ ở màn trận (nền và vật cản từ bundle).
///
/// Không có bundle thì vẽ khung trống: màn trận vẫn mở được, không hiện map giả.
fn draw_map(rect: &Rect, atlas: Option<&Atlas>, scene: Option<&Scene>) {
    draw_rectangle(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
        PANEL,
    );
    let (Some(atlas), Some(scene)) = (atlas, scene) else {
        return;
    };
    let origin = (rect.x as f32, rect.y as f32);
    for item in &scene.ground {
        atlas.draw(item, origin);
    }
    for item in &scene.objects {
        atlas.draw(item, origin);
    }
}
