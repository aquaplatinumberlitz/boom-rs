//! Vẽ layout của UI lên màn hình (macroquad, ADR 0010).
//!
//! Module này **mỏng**: chỉ đọc [`crate::ui::layout::Layout`] rồi vẽ. Mọi quyết định
//! về nội dung, thứ tự và toạ độ nằm ở module thuần [`crate::ui::layout`] — nhờ vậy
//! phần logic kiểm được không cần GPU, còn phần này chỉ cần một con mắt.

use std::path::PathBuf;

use boom_content::bundle::Bundle;
use boom_content::map::CellKind;
use macroquad::prelude::*;
use macroquad::text::{Font, TextParams};

use crate::render::Atlas;
use crate::scene::{DrawItem, PixelRect, Scene};
use crate::ui::layout::{Layout, Rect, Tone, Widget};

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
/// Màu khung thay ảnh chưa duyệt.
const FRAME_PENDING: Color = Color::from_rgba(92, 74, 36, 255);
/// Màu khung thay ảnh bị loại.
const FRAME_EXCLUDED: Color = Color::from_rgba(58, 60, 66, 255);
/// Màu chữ trong khung thay ảnh.
const FRAME_TEXT: Color = Color::from_rgba(226, 214, 180, 255);

/// Cỡ chữ tiêu đề.
const TITLE_SIZE: u16 = 24;
/// Cỡ chữ thông báo.
const NOTICE_SIZE: u16 = 16;
/// Cỡ chữ nhãn ô nhập.
const LABEL_SIZE: u16 = 14;
/// Cỡ chữ giá trị ô nhập.
const VALUE_SIZE: u16 = 18;
/// Cỡ chữ đầu bảng.
const HEADER_SIZE: u16 = 13;
/// Cỡ chữ dòng bảng.
const ROW_SIZE: u16 = 14;
/// Cỡ chữ nút.
const BUTTON_SIZE: u16 = 16;
/// Cỡ chữ nhỏ (khung thay ảnh, chat).
const SMALL_SIZE: u16 = 13;
/// Chiều cao một dòng bảng (px).
const ROW_HEIGHT: i32 = 26;
/// Khoảng cách giữa hai cột của bảng (px).
const COLUMN_GAP: f32 = 18.0;

/// Nạp font hỗ trợ tiếng Việt.
///
/// Ưu tiên font hệ thống có dấu (DejaVu/Noto/Segoe/Arial/Helvetica), ghi đè bằng
/// biến môi trường `BOOM_FONT`. Nếu không có font nào thì dùng font mặc định của
/// macroquad và **báo cảnh báo** — font mặc định thiếu glyph tiếng Việt nên ảnh
/// chụp sẽ không đầy đủ dấu.
///
/// Font chuẩn của V1 (Be Vietnam Pro) chưa có trong repo: xem
/// [UI_STRINGS_V1 §6](../../../../docs/specification/localization/UI_STRINGS_V1.md).
pub fn load_font() -> Font {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(path) = std::env::var("BOOM_FONT") {
        candidates.push(PathBuf::from(path));
    }
    candidates.extend(
        [
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
        .into_iter()
        .map(PathBuf::from),
    );
    for path in candidates {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        match macroquad::text::load_ttf_font_from_bytes(&bytes) {
            Ok(font) => return font,
            Err(error) => eprintln!("không nạp được font {}: {error:?}", path.display()),
        }
    }
    eprintln!(
        "cảnh báo UI: không tìm thấy font TTF có dấu tiếng Việt (đặt BOOM_FONT); \
         dùng font mặc định của macroquad — ảnh sẽ thiếu dấu"
    );
    macroquad::text::get_default_font()
}

/// Vẽ một layout lên màn hình.
///
/// `atlas`/`bundle`/`scene` là tuỳ chọn: có thì ảnh UI đã duyệt và bản đồ ở màn
/// trận vẽ được; không có thì phần tử đó vẽ khung màu + text catalog.
pub fn draw(
    layout: &Layout,
    font: &Font,
    atlas: Option<&Atlas>,
    bundle: Option<&Bundle>,
    scene: Option<&Scene>,
) {
    clear_background(BACKGROUND);
    for widget in &layout.widgets {
        match widget {
            Widget::Frame { rect, label, tone } => draw_frame(rect, label, *tone, font),
            Widget::Title { rect, text } => draw_text_at(text, rect, TITLE_SIZE, TITLE_TEXT, font),
            Widget::Notice { rect, text, error } => {
                let color = if *error { ERROR_TEXT } else { NOTICE_TEXT };
                draw_text_at(text, rect, NOTICE_SIZE, color, font);
            }
            Widget::Field {
                rect,
                label,
                value,
                focused,
                masked,
            } => draw_field(rect, label, value, *focused, *masked, font),
            Widget::Table {
                rect,
                columns,
                rows,
            } => draw_table(rect, columns, rows, font),
            Widget::Chat { rect, lines } => draw_chat(rect, lines, font),
            Widget::Button {
                rect,
                label,
                enabled,
                primary,
                ..
            } => draw_button(rect, label, *enabled, *primary, font),
            Widget::Art {
                rect,
                sprite,
                label,
            } => {
                draw_art(rect, sprite, label, atlas, bundle, font);
            }
            Widget::MapArea { rect } => draw_map(rect, atlas, scene),
        }
    }
}

/// Vẽ chữ tại góc trên-trái của vùng.
///
/// `rect.y` là **đỉnh** của dòng chữ: macroquad vẽ theo đường chân nên phải cộng
/// chiều cao chữ xuống, nếu không chữ bị cắt ở mép trên.
fn draw_text_at(text: &str, rect: &Rect, size: u16, color: Color, font: &Font) {
    if text.is_empty() {
        return;
    }
    draw_text_ex(
        text,
        rect.x as f32,
        baseline(text, rect.y as f32, size, font),
        TextParams {
            font: Some(font),
            font_size: size,
            color,
            ..Default::default()
        },
    );
}

/// Toạ độ Y để vẽ chữ có **đỉnh** tại `top` (macroquad vẽ theo đường chân).
fn baseline(text: &str, top: f32, size: u16, font: &Font) -> f32 {
    top + macroquad::text::measure_text(text, Some(font), size, 1.0).offset_y
}

/// Vẽ khung màu (nền màn hoặc khung thay ảnh chưa duyệt).
fn draw_frame(rect: &Rect, label: &str, tone: Tone, font: &Font) {
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
    if label.is_empty() {
        return;
    }
    draw_text_ex(
        label,
        rect.x as f32 + 4.0,
        baseline(label, rect.y as f32 + 4.0, SMALL_SIZE, font),
        TextParams {
            font: Some(font),
            font_size: SMALL_SIZE,
            color: FRAME_TEXT,
            ..Default::default()
        },
    );
}

/// Vẽ ô nhập: nhãn ở trên, khung và giá trị bên trong.
fn draw_field(rect: &Rect, label: &str, value: &str, focused: bool, masked: bool, font: &Font) {
    draw_text_ex(
        label,
        rect.x as f32,
        baseline(label, (rect.y - 18) as f32, LABEL_SIZE, font),
        TextParams {
            font: Some(font),
            font_size: LABEL_SIZE,
            color: HEADER_TEXT,
            ..Default::default()
        },
    );
    let border = if focused { FIELD_FOCUS } else { FIELD_BORDER };
    draw_rectangle_lines(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
        2.0,
        border,
    );
    let shown = if masked {
        "•".repeat(value.chars().count())
    } else {
        value.to_owned()
    };
    draw_text_ex(
        &shown,
        (rect.x + 8) as f32,
        baseline(&shown, (rect.y + 10) as f32, VALUE_SIZE, font),
        TextParams {
            font: Some(font),
            font_size: VALUE_SIZE,
            color: ROW_TEXT,
            ..Default::default()
        },
    );
    if focused {
        draw_rectangle(
            (rect.x + 8) as f32,
            (rect.y + 10) as f32,
            2.0,
            20.0,
            FIELD_FOCUS,
        );
    }
}

/// Vẽ bảng: đầu cột rồi tới các dòng.
///
/// Chiều rộng mỗi cột là ô rộng nhất của cột đó (đo theo font đang dùng) nên các
/// cột không dính nhau dù chuỗi là placeholder dài.
fn draw_table(rect: &Rect, columns: &[String], rows: &[crate::ui::layout::TableRow], font: &Font) {
    let mut widths: Vec<f32> = columns
        .iter()
        .map(|column| macroquad::text::measure_text(column, Some(font), HEADER_SIZE, 1.0).width)
        .collect();
    for row in rows {
        for (index, cell) in row.cells.iter().enumerate() {
            let width = macroquad::text::measure_text(cell, Some(font), ROW_SIZE, 1.0).width;
            if let Some(slot) = widths.get_mut(index) {
                *slot = slot.max(width);
            }
        }
    }
    let mut columns_x: Vec<f32> = Vec::with_capacity(columns.len());
    let mut x = rect.x as f32;
    for width in &widths {
        columns_x.push(x);
        x += width + COLUMN_GAP;
    }
    for (index, column) in columns.iter().enumerate() {
        draw_text_ex(
            column,
            columns_x[index],
            baseline(column, rect.y as f32, HEADER_SIZE, font),
            TextParams {
                font: Some(font),
                font_size: HEADER_SIZE,
                color: HEADER_TEXT,
                ..Default::default()
            },
        );
    }
    for (index, row) in rows.iter().enumerate() {
        let y = rect.y + 22 + i32::try_from(index).unwrap_or(0) * ROW_HEIGHT;
        if y >= rect.bottom() {
            break;
        }
        if row.selected {
            draw_rectangle(
                rect.x as f32 - 4.0,
                (y - 4) as f32,
                (rect.width + 8) as f32,
                (ROW_HEIGHT - 2) as f32,
                ROW_SELECTED,
            );
        }
        for (cell_index, cell) in row.cells.iter().enumerate() {
            let Some(x) = columns_x.get(cell_index) else {
                continue;
            };
            draw_text_ex(
                cell,
                *x,
                baseline(cell, y as f32, ROW_SIZE, font),
                TextParams {
                    font: Some(font),
                    font_size: ROW_SIZE,
                    color: ROW_TEXT,
                    ..Default::default()
                },
            );
        }
    }
}

/// Vẽ khung chat (các dòng mới nhất ở dưới).
fn draw_chat(rect: &Rect, lines: &[String], font: &Font) {
    let max_lines = usize::try_from((rect.height / 18).max(1)).unwrap_or(1);
    let start = lines.len().saturating_sub(max_lines);
    for (index, line) in lines[start..].iter().enumerate() {
        let y = rect.y + i32::try_from(index).unwrap_or(0) * 18;
        draw_text_ex(
            line,
            rect.x as f32,
            baseline(line, y as f32, SMALL_SIZE, font),
            TextParams {
                font: Some(font),
                font_size: SMALL_SIZE,
                color: ROW_TEXT,
                ..Default::default()
            },
        );
    }
}

/// Vẽ nút: nền theo trạng thái, nhãn căn giữa.
fn draw_button(rect: &Rect, label: &str, enabled: bool, primary: bool, font: &Font) {
    let background = if !enabled {
        BUTTON_DISABLED
    } else if primary {
        BUTTON_PRIMARY
    } else {
        BUTTON
    };
    let color = if enabled {
        BUTTON_TEXT
    } else {
        BUTTON_TEXT_DISABLED
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
        if enabled {
            FIELD_BORDER
        } else {
            BUTTON_DISABLED
        },
    );
    let measured = macroquad::text::measure_text(label, Some(font), BUTTON_SIZE, 1.0);
    let x = rect.x as f32 + ((rect.width as f32) - measured.width) / 2.0;
    let y = rect.y as f32 + ((rect.height as f32) - measured.height) / 2.0;
    draw_text_ex(
        label,
        x,
        baseline(label, y, BUTTON_SIZE, font),
        TextParams {
            font: Some(font),
            font_size: BUTTON_SIZE,
            color,
            ..Default::default()
        },
    );
}

/// Vẽ ảnh UI đã duyệt (`selected`).
///
/// Ảnh vẽ ở kích thước gốc (bản đồ ảnh của bundle) trong ô đã cấp. Nếu ảnh
/// không vừa ô hoặc bundle không có sprite thì vẽ khung màu + text catalog — không
/// hiện ảnh khác, không bỏ qua âm thầm.
fn draw_art(
    rect: &Rect,
    sprite: &str,
    label: &str,
    atlas: Option<&Atlas>,
    bundle: Option<&Bundle>,
    font: &Font,
) {
    let Some(item) = art_item(bundle, sprite, rect) else {
        draw_frame(rect, label, Tone::Excluded, font);
        return;
    };
    let Some(atlas) = atlas else {
        draw_frame(rect, label, Tone::Excluded, font);
        return;
    };
    if item.source.width > rect.width || item.source.height > rect.height {
        // Ảnh không vừa ô: vẽ khung thay, không cắt ảnh.
        draw_frame(rect, label, Tone::Excluded, font);
        return;
    }
    // Ảnh vẽ ở kích thước gốc, canh giữa trong ô.
    let offset_x = (rect.width.saturating_sub(item.source.width)) as f32 / 2.0;
    let offset_y = (rect.height.saturating_sub(item.source.height)) as f32 / 2.0;
    atlas.draw(&item, (offset_x, offset_y));
    draw_text_ex(
        label,
        rect.x as f32 + 4.0,
        baseline(label, (rect.bottom() - 16) as f32, SMALL_SIZE, font),
        TextParams {
            font: Some(font),
            font_size: SMALL_SIZE,
            color: HEADER_TEXT,
            ..Default::default()
        },
    );
}

/// Lệnh vẽ ảnh UI: cắt vùng của sprite trong atlas rồi đặt vào ô đã cấp.
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
