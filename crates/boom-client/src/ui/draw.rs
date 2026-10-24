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
//! rộng hộp, bị cắt theo chiều cao hộp và không bao giờ vẽ ra ngoài hộp
//! (UI_STRINGS_V1 §5.2.1). Ký tự không có glyph trong font được vẽ thành **hộp báo
//! thiếu**, không bỏ qua âm thầm (AGENTS §4).

use boom_content::bundle::Bundle;
use boom_content::map::CellKind;
use macroquad::prelude::*;

use crate::render::Atlas;
use crate::scene::{DrawItem, PixelRect, Scene};
use crate::text::{Clip, GlyphCache, TextFont};
use crate::ui::layout::{
    Align, FIELD_BOTTOM_GAP, FIELD_INSET, Layout, Rect, TABLE_HEADER_HEIGHT, TABLE_ROW_HEIGHT,
    TextBox, TextRole, Tone, Widget, text_size,
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
            Widget::Table {
                rect,
                columns,
                rows,
            } => {
                // Chiều cao dòng theo nội dung thật (tô dòng đang chọn khớp dòng
                // chữ) — cùng hàm với phần layout.
                let widths = crate::ui::layout::table_column_widths_fitted(
                    columns,
                    rows,
                    &font.font,
                    rect.width as f32,
                );
                let heights = crate::ui::layout::table_row_heights(
                    rows,
                    &font.font,
                    &widths,
                    rect.height as f32 - TABLE_HEADER_HEIGHT,
                );
                draw_table_background(rect, rows, &heights);
            }
            _ => {}
        }
        for text_box in widget.text_boxes(&font.font) {
            draw_text_box(font, &text_box);
        }
    }
}

/// Vẽ một hộp chữ: xuống dòng theo bề rộng hộp, cắt theo chiều cao hộp.
///
/// Chỉ phần chữ **vừa hộp** được vẽ ([`TextBox::laid_out`], UI_STRINGS_V1
/// §5.2.1): chữ dài hơn hộp bị cắt theo [`crate::text::Clip`], không tràn ra ngoài.
fn draw_text_box(font: &mut UiFont, text_box: &TextBox) {
    if text_box.text.is_empty() {
        return;
    }
    let laid = text_box.laid_out(&font.font);
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

/// Vẽ lớp chữ của màn trận lên trên scene đã vẽ (dải trên cùng của cửa sổ trận).
///
/// Chỉ chữ **cho người chơi** — tên map và trạng thái lượt, lấy từ file chuỗi V1
/// (xem [`crate::ui::layout::match_status_texts`]) và vẽ bằng font đã bundle nên
/// chữ Việt có glyph đầy đủ.
///
/// HUD chẩn đoán (số đo tick/âm thanh/phím) là việc QA, **không** vẽ ở đây: đường
/// chơi thật không có chữ viết tay (issue #179).
///
/// `width` là bề rộng cửa sổ màn trận (px) và `band_height` là chiều cao dải trên
/// cùng; chữ được xuống dòng và cắt theo hộp nên không tràn ra ngoài dải.
pub fn draw_match_overlay(
    title: &str,
    status: &str,
    font: &mut UiFont,
    width: u32,
    band_height: f32,
) {
    let half = width as f32 / 2.0;
    let boxes = [
        TextBox {
            text: title.to_owned(),
            size: text_size::TITLE,
            role: TextRole::Title,
            x: 8.0,
            y: 4.0,
            width: half - 16.0,
            height: band_height - 8.0,
            align: Align::Left,
            clip: Clip::Head,
        },
        TextBox {
            text: status.to_owned(),
            size: text_size::NOTICE,
            role: TextRole::Notice,
            x: half,
            y: 4.0,
            width: half - 8.0,
            height: band_height - 8.0,
            align: Align::Left,
            clip: Clip::Head,
        },
    ];
    for text_box in &boxes {
        draw_text_box(font, text_box);
    }
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
///
/// `heights` là chiều cao từng dòng (theo nội dung thật) nên tô đúng dòng chữ.
fn draw_table_background(rect: &Rect, rows: &[crate::ui::layout::TableRow], heights: &[f32]) {
    let mut y = rect.y as f32 + TABLE_HEADER_HEIGHT;
    for (index, row) in rows.iter().enumerate() {
        let height = heights.get(index).copied().unwrap_or(TABLE_ROW_HEIGHT);
        if y + height > rect.bottom() as f32 {
            break;
        }
        if row.selected {
            draw_rectangle(
                (rect.x - 4) as f32,
                y - 4.0,
                (rect.width + 8) as f32,
                height - 2.0,
                ROW_SELECTED,
            );
        }
        y += height;
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
/// Ảnh canh giữa, thu nhỏ cùng tỷ lệ khi lớn hơn ô; không cắt chữ/ảnh.
fn draw_art(rect: &Rect, sprite: &str, atlas: Option<&Atlas>, bundle: Option<&Bundle>) {
    let Some(item) = art_item(bundle, sprite, rect) else {
        draw_frame(rect, Tone::Excluded);
        return;
    };
    let Some(atlas) = atlas else {
        draw_frame(rect, Tone::Excluded);
        return;
    };
    let image_height = rect
        .height
        .saturating_sub(crate::ui::layout::CHAT_LINE_HEIGHT as u32);
    let scale = art_scale(
        item.source.width,
        item.source.height,
        rect.width,
        image_height,
    );
    if scale <= 0.0 {
        draw_frame(rect, Tone::Excluded);
        return;
    }
    let offset_x = (rect.width as f32 - item.source.width as f32 * scale) / 2.0;
    let offset_y = (image_height as f32 - item.source.height as f32 * scale) / 2.0;
    atlas.draw_scaled(&item, (offset_x, offset_y), scale);
}

/// Thu nhỏ ảnh theo cùng tỷ lệ, giữ đủ pixel nguồn và không phóng to.
fn art_scale(width: u32, height: u32, box_width: u32, box_height: u32) -> f32 {
    if width == 0 || height == 0 {
        return 0.0;
    }
    (box_width as f32 / width as f32)
        .min(box_height as f32 / height as f32)
        .min(1.0)
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

#[cfg(test)]
mod localized_art_tests {
    use super::*;
    #[test]
    fn merged_draw_fits_the_sidebar_without_cropping_or_stretching() {
        let scale = art_scale(216, 71, 156, 160);
        assert!((216.0 * scale - 156.0).abs() < 0.01);
        assert!(71.0 * scale < 160.0);
        assert_eq!(art_scale(80, 40, 156, 160), 1.0);
        assert_eq!(art_scale(0, 40, 156, 160), 0.0);
    }
}
