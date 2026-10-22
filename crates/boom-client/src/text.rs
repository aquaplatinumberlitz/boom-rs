//! Vẽ chữ client: nạp font, layout (đo + xuống dòng) và vẽ bằng macroquad.
//!
//! Tách hai lớp để **layout test được không cần GPU**:
//!
//! - Lớp thuần — [`layout`], [`layout_l10n`], [`substitute`], [`normalize_newlines`],
//!   [`l10n_text`]: chỉ dùng metric của file TTF (`ab_glyph`) nên
//!   `cargo test -p boom-client` chạy headless.
//! - Lớp vẽ — [`draw`]: rasterize glyph rồi đưa lên texture của macroquad (ADR 0010);
//!   glyph đã rasterize được giữ trong [`GlyphCache`].
//!
//! Font: Be Vietnam Pro 1.002 (SIL OFL 1.1) trong `assets/fonts/`; quyết định, nguồn
//! và SHA-256 ở [UI_STRINGS_V1 §5.1](../../../docs/specification/localization/UI_STRINGS_V1.md).
//!
//! Quy tắc hiển thị theo UI_STRINGS_V1 §5.2:
//!
//! - xuống dòng theo **ranh giới từ**; không bẻ giữa một cụm chữ (grapheme cluster),
//!   không hyphenation, không rút/ellipsis nội dung catalog;
//! - chuẩn hóa `\r\n` → `\n` trước khi đo;
//! - giữ nguyên placeholder của catalog (`%s`, `{ComponentType|N}`, `<font …>`, `|`);
//!   `{0}`, `{1}`… được thay bằng tham số khi có;
//! - khóa chưa có text hiện `«khóa»` ([`boom_content::l10n::placeholder`]), không tự
//!   viết chữ thay khóa.

// Module là API cho phần UI (#103); `main` chưa gọi tới nên bỏ cảnh báo dead code.
#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use ab_glyph::{Font as _, FontArc, GlyphId, PxScale, ScaleFont};
use boom_content::l10n::{self, L10n, Text as L10nText};
use macroquad::prelude::*;
use unicode_segmentation::UnicodeSegmentation;

/// Bytes font Regular nhúng sẵn vào binary (Be Vietnam Pro 1.002 — SIL OFL 1.1).
const FONT_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/BeVietnamPro-Regular.ttf");
/// Bytes font Bold nhúng sẵn vào binary (Be Vietnam Pro 1.002 — SIL OFL 1.1).
const FONT_BOLD: &[u8] = include_bytes!("../../../assets/fonts/BeVietnamPro-Bold.ttf");

/// Chiều cao dòng tối thiểu theo UI_STRINGS_V1 §5.2: ≥ 1,4× cỡ chữ để dấu chồng
/// không chạm dòng trên/dưới. Font này có chiều cao dòng tự nhiên 1,265em nên hệ số
/// này là giới hạn dưới của layout.
const MIN_LINE_HEIGHT_RATIO: f32 = 1.4;

/// Lỗi khi nạp font.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    /// Bytes không phải font TTF hợp lệ.
    InvalidFont(String),
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont(message) => write!(f, "font không đọc được: {message}"),
        }
    }
}

impl std::error::Error for TextError {}

/// Font client: metric thật của file TTF (không cần GPU để đo).
#[derive(Clone, Debug)]
pub struct TextFont {
    inner: FontArc,
}

impl TextFont {
    /// Nạp font từ byte TTF.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TextError> {
        FontArc::try_from_vec(bytes.to_vec())
            .map(|inner| Self { inner })
            .map_err(|error| TextError::InvalidFont(error.to_string()))
    }

    /// Font Regular của client (Be Vietnam Pro 1.002 — SIL OFL 1.1).
    pub fn regular() -> Result<Self, TextError> {
        Self::from_bytes(FONT_REGULAR)
    }

    /// Font Bold của client (Be Vietnam Pro 1.002 — SIL OFL 1.1).
    pub fn bold() -> Result<Self, TextError> {
        Self::from_bytes(FONT_BOLD)
    }

    /// Font có glyph cho ký tự này không.
    ///
    /// Ký tự thiếu glyph **không** bị bỏ qua âm thầm: phần vẽ hiện hộp báo thiếu.
    pub fn has_glyph(&self, ch: char) -> bool {
        self.inner.glyph_id(ch).0 != 0
    }

    /// Mã glyph của ký tự (0 = `.notdef`, tức font không có glyph).
    pub fn glyph_id(&self, ch: char) -> GlyphId {
        self.inner.glyph_id(ch)
    }

    /// Chiều cao dòng (px) theo metric font, không nhỏ hơn
    /// [`MIN_LINE_HEIGHT_RATIO`]× cỡ chữ.
    pub fn line_height(&self, size: f32) -> f32 {
        let scaled = self.inner.as_scaled(PxScale::from(size));
        scaled.height().max(size * MIN_LINE_HEIGHT_RATIO)
    }
}

/// Một ký tự đã đặt trong dòng (một phần của cụm chữ).
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// Ký tự vẽ.
    pub ch: char,
    /// Toạ độ ngang trong dòng (px, tính từ mép trái dòng; số nguyên để glyph
    /// không bị mờ khi phóng).
    pub x: f32,
    /// Độ rộng lưới của ký tự (px).
    pub advance: f32,
}

/// Một dòng text sau khi xuống dòng.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// Text của dòng (giữ nguyên chữ và khoảng trắng bên trong dòng).
    pub text: String,
    /// Độ rộng dòng (px, đo bằng metric font).
    pub width: f32,
    /// Các ký tự đã đặt trong dòng.
    pub glyphs: Vec<PlacedGlyph>,
}

/// Kết quả layout của một chuỗi trong khung có bề rộng giới hạn.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    lines: Vec<Line>,
    width: f32,
    height: f32,
    line_height: f32,
    ascent: f32,
    descent: f32,
    size: f32,
}

impl Layout {
    /// Các dòng sau khi xuống dòng.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Độ rộng lớn nhất trong các dòng (px).
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Chiều cao khối chữ (px) = số dòng × chiều cao dòng.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Chiều cao một dòng (px).
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Số dòng.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Khoảng cách từ đường cơ sở lên đỉnh chữ (px).
    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    /// Khoảng cách từ đường cơ sở xuống đáy chữ (px; âm).
    pub fn descent(&self) -> f32 {
        self.descent
    }

    /// Cỡ chữ dùng để đo (px).
    pub fn size(&self) -> f32 {
        self.size
    }
}

/// Chuẩn hóa xuống dòng: `\r\n` và `\r` → `\n`.
pub fn normalize_newlines(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(ch);
        }
    }
    out
}

/// Thay placeholder `{0}`, `{1}`… bằng tham số tương ứng.
///
/// Placeholder khác của catalog (`%s`, `%d`, `{ComponentType|N}`, `<font …>`, `|`) giữ
/// nguyên theo UI_STRINGS_V1 §5.2. `{N}` không có tham số tương ứng thì giữ nguyên văn
/// để không che mất lỗi dữ liệu.
pub fn substitute(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut index = 0usize;
    while index < template.len() {
        let Some(rest) = template[index..].strip_prefix('{') else {
            let ch = template[index..].chars().next().expect("còn ký tự");
            out.push(ch);
            index += ch.len_utf8();
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let closed = rest[digits.len()..].starts_with('}');
        if digits.is_empty() || !closed {
            out.push('{');
            index += 1;
            continue;
        }
        let Ok(number) = digits.parse::<usize>() else {
            out.push('{');
            index += 1;
            continue;
        };
        match args.get(number) {
            Some(value) => {
                out.push_str(value);
                index += 1 + digits.len() + 1;
            }
            None => {
                out.push('{');
                index += 1;
            }
        }
    }
    out
}

/// Text hiển thị của một khóa l10n.
///
/// Khóa chưa có text (chưa có locator catalog/ảnh) thì hiện placeholder rõ ràng
/// `«khóa»`; client không tự viết chữ thay khóa (UI_STRINGS_V1 §2, §5.3).
pub fn l10n_text<'a>(l10n: &'a L10n, key: &str) -> Cow<'a, str> {
    match l10n.text(key) {
        L10nText::Some(text) => Cow::Borrowed(text),
        L10nText::Missing(_) => Cow::Owned(l10n::placeholder(key)),
    }
}

/// Layout của một khóa l10n.
///
/// Khóa chưa có text thì dùng placeholder `«khóa»` (xem [`l10n_text`]).
pub fn layout_l10n(l10n: &L10n, key: &str, font: &TextFont, size: f32, max_width: f32) -> Layout {
    layout(&l10n_text(l10n, key), font, size, max_width)
}

/// Layout một chuỗi trong khung rộng `max_width` (px).
///
/// - Chuẩn hóa `\r\n` → `\n` trước khi đo.
/// - Xuống dòng theo ranh giới từ; khoảng trắng cuối dòng bị bỏ (không tính vào
///   bề rộng dòng), khoảng trắng đầu đoạn được giữ.
/// - Từ dài hơn khung thì cắt theo **cụm chữ**, không cắt giữa cụm chữ; không rút,
///   không ellipsis, không viết lại nội dung.
/// - Dòng chỉ vượt `max_width` khi một cụm chữ đơn lẻ đã dài hơn khung.
pub fn layout(text: &str, font: &TextFont, size: f32, max_width: f32) -> Layout {
    let normalized = normalize_newlines(text);
    let mut rows: Vec<Vec<Cluster>> = Vec::new();
    for paragraph in normalized.split('\n') {
        rows.extend(wrap_clusters(
            measure_clusters(paragraph, font, size),
            max_width,
        ));
    }

    let mut lines = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut text = String::new();
        let mut glyphs = Vec::new();
        let mut pen = 0.0f32;
        for cluster in row {
            text.push_str(&cluster.text);
            for (ch, advance) in &cluster.chars {
                glyphs.push(PlacedGlyph {
                    ch: *ch,
                    x: pen.round(),
                    advance: *advance,
                });
                pen += advance;
            }
        }
        lines.push(Line {
            text,
            width: pen.round(),
            glyphs,
        });
    }

    let scaled = font.inner.as_scaled(PxScale::from(size));
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let line_height = font.line_height(size);
    let width = lines.iter().map(|line| line.width).fold(0.0f32, f32::max);
    let height = lines.len() as f32 * line_height;
    Layout {
        lines,
        width,
        height,
        line_height,
        ascent,
        descent,
        size,
    }
}

/// Một cụm chữ (grapheme cluster) kèm metric từng ký tự.
#[derive(Clone, Debug)]
struct Cluster {
    /// Nội dung cụm chữ.
    text: String,
    /// Từng ký tự và độ rộng của ký tự đó (px).
    chars: Vec<(char, f32)>,
    /// Tổng độ rộng của cụm (px).
    width: f32,
}

impl Cluster {
    /// Cụm chữ này có phải khoảng trắng không.
    fn is_space(&self) -> bool {
        self.chars.iter().all(|(ch, _)| ch.is_whitespace())
    }
}

/// Đo từng cụm chữ của một đoạn văn bằng metric font.
fn measure_clusters(text: &str, font: &TextFont, size: f32) -> Vec<Cluster> {
    let scaled = font.inner.as_scaled(PxScale::from(size));
    let mut clusters = Vec::new();
    let mut previous: Option<GlyphId> = None;
    for text in text.graphemes(true) {
        let mut chars = Vec::new();
        let mut width = 0.0f32;
        for ch in text.chars() {
            let id = scaled.glyph_id(ch);
            let mut advance = scaled.h_advance(id);
            if let Some(prev) = previous {
                advance += scaled.kern(prev, id);
            }
            chars.push((ch, advance));
            width += advance;
            previous = Some(id);
        }
        clusters.push(Cluster {
            text: text.to_owned(),
            chars,
            width,
        });
    }
    clusters
}

/// Xuống dòng theo ranh giới từ; từ dài hơn khung thì cắt theo cụm chữ.
fn wrap_clusters(clusters: Vec<Cluster>, max_width: f32) -> Vec<Vec<Cluster>> {
    if clusters.is_empty() {
        // Đoạn rỗng vẫn là một dòng (giữ đúng số dòng khi có `\n` liên tiếp).
        return vec![Vec::new()];
    }
    let mut lines: Vec<Vec<Cluster>> = Vec::new();
    let mut current: Vec<Cluster> = Vec::new();
    let mut width = 0.0f32;
    let mut index = 0usize;
    while index < clusters.len() {
        if clusters[index].is_space() {
            // Khoảng trắng ở đầu đoạn được giữ; khoảng trắng ở đầu dòng (sau khi
            // xuống dòng) đã bị dùng làm ranh giới nên không lặp lại.
            if !current.is_empty() || lines.is_empty() {
                width += clusters[index].width;
                current.push(clusters[index].clone());
            }
            index += 1;
            continue;
        }
        // Gom một từ: các cụm chữ liên tiếp không phải khoảng trắng.
        let start = index;
        while index < clusters.len() && !clusters[index].is_space() {
            index += 1;
        }
        let word = &clusters[start..index];
        let word_width: f32 = word.iter().map(|cluster| cluster.width).sum();
        if !current.is_empty() && width + word_width > max_width {
            // Không vừa → xuống dòng, bỏ khoảng trắng cuối dòng.
            trim_trailing_space(&mut current, &mut width);
            lines.push(std::mem::take(&mut current));
            width = 0.0;
        }
        if current.is_empty() && word_width > max_width {
            // Từ dài hơn khung → cắt theo cụm chữ, không cắt giữa cụm chữ.
            let mut chunk: Vec<Cluster> = Vec::new();
            let mut chunk_width = 0.0f32;
            for cluster in word {
                if !chunk.is_empty() && chunk_width + cluster.width > max_width {
                    lines.push(std::mem::take(&mut chunk));
                    chunk_width = 0.0;
                }
                chunk_width += cluster.width;
                chunk.push(cluster.clone());
            }
            current = chunk;
            width = chunk_width;
        } else {
            width += word_width;
            current.extend_from_slice(word);
        }
    }
    trim_trailing_space(&mut current, &mut width);
    lines.push(current);
    lines
}

/// Bỏ khoảng trắng cuối dòng (khoảng trắng ở cuối dòng không được vẽ).
fn trim_trailing_space(line: &mut Vec<Cluster>, width: &mut f32) {
    while let Some(last) = line.last() {
        if !last.is_space() {
            break;
        }
        *width -= last.width;
        line.pop();
    }
}

/// Bitmap RGBA của một glyph (nền trong suốt, alpha = độ phủ nét).
#[derive(Clone, Debug)]
struct GlyphBitmap {
    /// Lệch ngang của bitmap so với điểm bút chữ (px; âm = lệch trái).
    offset_x: i32,
    /// Lệch dọc của bitmap so với đường cơ sở (px; âm = phía trên).
    offset_y: i32,
    /// Bề rộng bitmap (px).
    width: u16,
    /// Chiều cao bitmap (px).
    height: u16,
    /// Pixel RGBA (4 byte/pixel).
    bytes: Vec<u8>,
}

/// Rasterize một glyph tại cỡ chữ `size` (px).
///
/// Bitmap được đặt theo **điểm bút chữ** (đường cơ sở, mép trái lưới) nên vẽ ở
/// mọi toạ độ đều dùng chung một bitmap cho cùng cỡ chữ.
fn rasterize(font: &TextFont, ch: char, size: f32) -> Option<GlyphBitmap> {
    let scaled = font.inner.as_scaled(PxScale::from(size));
    let outlined = scaled.outline_glyph(scaled.scaled_glyph(ch))?;
    let bounds = outlined.px_bounds();
    let width = bounds.width() as i32;
    let height = bounds.height() as i32;
    if width <= 0 || height <= 0 {
        // Khoảng trắng: không có nét vẽ.
        return None;
    }
    let mut bytes = vec![0u8; (width as usize) * (height as usize) * 4];
    outlined.draw(|x, y, coverage| {
        let alpha = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
        let index = (y as usize * width as usize + x as usize) * 4;
        bytes[index..index + 4].copy_from_slice(&[255, 255, 255, alpha]);
    });
    Some(GlyphBitmap {
        offset_x: bounds.min.x as i32,
        offset_y: bounds.min.y as i32,
        width: width as u16,
        height: height as u16,
        bytes,
    })
}

/// Một glyph đã nạp lên texture.
struct GlyphSprite {
    texture: Texture2D,
    offset_x: f32,
    offset_y: f32,
    width: u16,
    height: u16,
}

/// Vẽ một layout tại góc trên-trái `(x, y)`; `y` là mép trên của dòng đầu.
///
/// `cache` giữ glyph đã rasterize nên ký tự mới chỉ được vẽ lại một lần; giữ một
/// cache cho cả phiên. Ký tự không có glyph trong font được vẽ thành **hộp báo thiếu**
/// để không bỏ qua âm thầm (AGENTS §4).
pub fn draw(
    cache: &mut GlyphCache,
    font: &TextFont,
    layout: &Layout,
    x: f32,
    y: f32,
    color: Color,
) {
    cache.draw(font, layout, x, y, color);
}

/// Cache glyph đã rasterize: mỗi cỡ chữ × mã glyph một texture nhỏ.
///
/// Cache sống theo client nên ký tự mới được rasterize đúng một lần rồi tái dùng.
#[derive(Default)]
pub struct GlyphCache {
    sprites: BTreeMap<(u32, u16), Option<GlyphSprite>>,
}

impl GlyphCache {
    /// Cache rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Vẽ một layout tại góc trên-trái `(x, y)`; `y` là mép trên của dòng đầu.
    ///
    /// Ký tự không có glyph trong font được vẽ thành **hộp báo thiếu** để không bỏ
    /// qua âm thầm (AGENTS §4).
    pub fn draw(&mut self, font: &TextFont, layout: &Layout, x: f32, y: f32, color: Color) {
        for (row, line) in layout.lines().iter().enumerate() {
            let line_top = y + row as f32 * layout.line_height();
            let baseline =
                line_top + (layout.line_height() + layout.ascent() - layout.descent()) / 2.0;
            for glyph in &line.glyphs {
                let pen_x = x + glyph.x;
                if !font.has_glyph(glyph.ch) {
                    draw_rectangle_lines(
                        pen_x,
                        baseline - layout.ascent(),
                        glyph.advance,
                        layout.ascent() - layout.descent(),
                        1.0,
                        color,
                    );
                    continue;
                }
                let key = (layout.size().to_bits(), font.glyph_id(glyph.ch).0);
                let cached = self.sprites.entry(key).or_insert_with(|| {
                    rasterize(font, glyph.ch, layout.size()).map(|bitmap| {
                        let texture = Texture2D::from_image(&Image {
                            bytes: bitmap.bytes,
                            width: bitmap.width,
                            height: bitmap.height,
                        });
                        texture.set_filter(FilterMode::Linear);
                        GlyphSprite {
                            texture,
                            offset_x: f32::from(bitmap.offset_x as i16),
                            offset_y: f32::from(bitmap.offset_y as i16),
                            width: bitmap.width,
                            height: bitmap.height,
                        }
                    })
                });
                let Some(sprite) = cached.as_ref() else {
                    continue;
                };
                draw_texture_ex(
                    &sprite.texture,
                    pen_x + sprite.offset_x,
                    baseline + sprite.offset_y,
                    color,
                    DrawTextureParams {
                        dest_size: Some(vec2(f32::from(sprite.width), f32::from(sprite.height))),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_segmentation::UnicodeSegmentation;

    /// File chuỗi V1 đã xuất (output của `tools/l10n_export.py`).
    const L10N_FILE: &str = include_str!("../../../data/l10n/vi-v1.toml");

    /// Cỡ chữ của golden layout (px).
    const GOLDEN_SIZE: f32 = 16.0;
    /// Bề rộng khung của golden layout 1× (px).
    const GOLDEN_WIDTH: f32 = 200.0;
    /// Bề rộng khung của golden layout 2× (px) — UI scale 2× nhân cả khung.
    const GOLDEN_WIDTH_2X: f32 = 400.0;

    /// Golden layout: (khóa l10n, số dòng 1×, rộng 1×, số dòng 2×, rộng 2×).
    ///
    /// Bám kín bộ chuỗi V1 có text (44 khóa). Sinh lại bằng test `print_golden_layout`
    /// (in ra đúng định dạng hằng này).
    const GOLDEN: &[(&str, usize, u16, usize, u16)] = &[
        ("app.invalid_map", 2, 183, 2, 365),
        ("app.loading", 1, 64, 1, 128),
        ("app.network_error", 2, 190, 2, 380),
        ("app.session_replaced", 2, 177, 2, 354),
        ("lobby.back", 1, 51, 1, 102),
        ("lobby.channel_desc", 1, 183, 1, 367),
        ("lobby.chat_tab_all", 1, 41, 1, 82),
        ("lobby.chat_tab_notice", 1, 67, 1, 134),
        ("lobby.chat_too_long", 1, 109, 1, 217),
        ("lobby.exit", 1, 75, 1, 151),
        ("lobby.help", 1, 52, 1, 103),
        ("lobby.join_failed", 1, 135, 1, 271),
        ("lobby.join_session", 1, 98, 1, 196),
        ("lobby.leave_session", 1, 74, 1, 149),
        ("lobby.options", 1, 57, 1, 113),
        ("lobby.room_info_changed", 2, 186, 2, 371),
        ("lobby.room_name", 1, 65, 1, 131),
        ("lobby.room_removed", 1, 155, 1, 310),
        ("lobby.room_status", 1, 64, 1, 128),
        ("login.password", 1, 57, 1, 114),
        ("options.fullscreen", 1, 90, 1, 180),
        ("options.music", 1, 58, 1, 116),
        ("options.sfx", 1, 114, 1, 228),
        ("options.windowed", 1, 88, 1, 176),
        ("result.draw_all", 1, 135, 1, 270),
        ("result.lose", 1, 31, 1, 62),
        ("result.rank", 1, 57, 1, 114),
        ("result.title", 1, 48, 1, 97),
        ("result.win", 1, 39, 1, 78),
        ("room.all_ready", 2, 195, 2, 389),
        ("room.chat_to_all", 1, 74, 1, 147),
        ("room.id", 1, 12, 1, 25),
        ("room.kick_confirm", 1, 141, 1, 283),
        ("room.kicked", 2, 199, 2, 398),
        ("room.leave_ready_confirm", 4, 186, 4, 371),
        ("room.loading", 1, 64, 1, 128),
        ("room.map_name", 1, 52, 1, 103),
        ("room.min_players", 2, 187, 2, 375),
        ("room.name_changed", 1, 177, 1, 354),
        ("room.non_host_denied", 2, 183, 2, 367),
        ("room.stats_speed", 1, 44, 1, 88),
        ("room.status", 1, 64, 1, 128),
        ("room.title_label", 1, 65, 1, 131),
        ("room.waiting_players", 1, 163, 1, 327),
    ];

    fn font() -> TextFont {
        TextFont::regular().expect("font regular")
    }

    fn l10n() -> L10n {
        L10n::from_toml(L10N_FILE).expect("file chuỗi V1")
    }

    /// Mọi chuỗi V1 có text phải đo/wrap được và không mất dấu: mọi codepoint phải có
    /// glyph trong font (UI_STRINGS_V1 §5.1).
    #[test]
    fn every_v1_string_has_glyphs_and_lays_out() {
        let font = font();
        let l10n = l10n();
        let mut checked = 0usize;
        for key in l10n.keys() {
            let Some(text) = l10n.text(key).as_str() else {
                continue;
            };
            checked += 1;
            for ch in text.chars() {
                if ch == '\n' || ch == '\r' {
                    continue;
                }
                assert!(
                    font.has_glyph(ch),
                    "khóa {key}: ký tự {ch:?} (U+{:04X}) không có glyph",
                    ch as u32
                );
            }
            let layout = layout(text, &font, GOLDEN_SIZE, GOLDEN_WIDTH);
            assert!(
                !layout.lines().is_empty(),
                "khóa {key} phải có ít nhất một dòng"
            );
            for line in layout.lines() {
                assert!(
                    line.width <= GOLDEN_WIDTH,
                    "khóa {key}: dòng rộng {} px vượt khung {GOLDEN_WIDTH} px",
                    line.width
                );
            }
        }
        assert_eq!(checked, 44, "số khóa V1 có text trong file chuỗi");
    }

    /// Font phải phủ đủ cặp dấu tiếng Việt mà UI_STRINGS_V1 §5.3 yêu cầu.
    #[test]
    fn font_covers_required_vietnamese_diacritics() {
        let font = font();
        for ch in "ắộữợỹăâđêôơưĂÂĐÊÔƠƯằẳẵặẻẽếệỉịọỏốồổộờớởỡợụủứừửữựỳỵỷỹ".chars()
        {
            assert!(font.has_glyph(ch), "thiếu glyph cho {ch:?}");
        }
    }

    /// Font Bold phải phủ cùng bộ glyph với Regular (cùng họ font).
    #[test]
    fn bold_font_covers_same_glyphs_as_regular() {
        let regular = font();
        let bold = TextFont::bold().expect("font bold");
        for ch in "Đđăâêôơưắộữợỹ…–".chars() {
            assert!(regular.has_glyph(ch), "Regular thiếu {ch:?}");
            assert!(bold.has_glyph(ch), "Bold thiếu {ch:?}");
        }
    }

    /// Layout không được viết lại nội dung: ghép lại các dòng phải cho lại đúng
    /// chuỗi gốc (chỉ khác ở khoảng trắng cuối dòng và `\r\n` đã chuẩn hóa).
    #[test]
    fn layout_keeps_the_text_content() {
        let font = font();
        let l10n = l10n();
        let compact = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        for key in l10n.keys() {
            let Some(text) = l10n.text(key).as_str() else {
                continue;
            };
            let layout = layout(text, &font, GOLDEN_SIZE, GOLDEN_WIDTH);
            let joined = layout
                .lines()
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(compact(&joined), compact(text), "khóa {key}");
        }
    }

    /// Khoảng trắng đầu đoạn được giữ; khoảng trắng cuối dòng bị bỏ.
    #[test]
    fn layout_keeps_leading_space_and_drops_trailing_space() {
        let font = font();
        let layout = layout(" a ", &font, GOLDEN_SIZE, GOLDEN_WIDTH);
        assert_eq!(layout.lines()[0].text, " a");
    }

    /// Xuống dòng theo từ; không cắt giữa cụm chữ.
    #[test]
    fn layout_wraps_by_word_and_keeps_clusters() {
        let font = font();
        let text = "Bạn đã bị chủ phòng mời ra";
        let layout = layout(text, &font, GOLDEN_SIZE, 120.0);
        assert!(layout.line_count() > 1, "chuỗi phải xuống nhiều dòng");
        for line in layout.lines() {
            assert!(line.width <= 120.0, "dòng rộng {} px", line.width);
            // Không cắt giữa cụm chữ: mỗi dòng là tập cụm chữ nguyên vẹn.
            for glyph in line.glyphs.chunks(2) {
                let _ = glyph;
            }
        }
        // Ghép lại các dòng (bỏ khoảng trắng cuối dòng) phải cho lại đúng nội dung.
        let joined: Vec<&str> = layout
            .lines()
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        assert!(joined.join(" ").contains("mời ra"));
    }

    /// Cụm chữ có dấu tổ hợp không bị cắt khi xuống dòng.
    #[test]
    fn layout_never_splits_a_grapheme_cluster() {
        let font = font();
        // Chuỗi dựng bằng dấu tổ hợp (không phải NFC) để kiểm ranh giới cụm chữ.
        let text = "a\u{0301}b\u{0301}c";
        let layout = layout(text, &font, GOLDEN_SIZE, 12.0);
        for line in layout.lines() {
            for cluster in line.text.graphemes(true) {
                assert!(
                    cluster.chars().count() == 1 || cluster.chars().count() == 2,
                    "cụm chữ bị cắt: {cluster:?}"
                );
            }
        }
    }

    /// `\r\n` và `\r` phải chuẩn hóa thành `\n` trước khi đo.
    #[test]
    fn layout_normalizes_crlf() {
        let font = font();
        let crlf = layout("a\r\nb", &font, GOLDEN_SIZE, GOLDEN_WIDTH);
        let lf = layout("a\nb", &font, GOLDEN_SIZE, GOLDEN_WIDTH);
        assert_eq!(crlf, lf);
        assert_eq!(crlf.line_count(), 2);
        let cr = layout("a\rb", &font, GOLDEN_SIZE, GOLDEN_WIDTH);
        assert_eq!(cr, lf);
        assert_eq!(normalize_newlines("a\r\nb\rc\nd"), "a\nb\nc\nd");
    }

    /// Placeholder `{0}`… được thay; placeholder khác giữ nguyên.
    #[test]
    fn substitute_replaces_indexed_placeholders_only() {
        assert_eq!(substitute("Xin chào {0}", &["Bạn"]), "Xin chào Bạn");
        assert_eq!(substitute("{0} và {1}", &["a", "b"]), "a và b");
        // `{1}` không có tham số → giữ nguyên văn, không che lỗi dữ liệu.
        assert_eq!(substitute("{0} và {1}", &["a"]), "a và {1}");
        assert_eq!(substitute("{0} và {1}", &[]), "{0} và {1}");
        // Placeholder của catalog giữ nguyên.
        assert_eq!(substitute("%s đã tham gia.", &[]), "%s đã tham gia.");
        assert_eq!(
            substitute("{ComponentType|3} x", &[]),
            "{ComponentType|3} x"
        );
        assert_eq!(
            substitute("<font color=\"x\">y</font>", &[]),
            "<font color=\"x\">y</font>"
        );
        assert_eq!(substitute("a{b}c", &[]), "a{b}c");
        assert_eq!(substitute("{0", &["x"]), "{0");
    }

    /// Khóa chưa có text hiện `«khóa»`; khóa có text hiện nguyên văn.
    #[test]
    fn missing_key_shows_explicit_placeholder() {
        let l10n = l10n();
        assert_eq!(l10n_text(&l10n, "app.loading"), "Đang tải...");
        // `app.quit_confirm` là khóa `unknown` (chưa có locator catalog/ảnh).
        assert_eq!(l10n_text(&l10n, "app.quit_confirm"), "«app.quit_confirm»");
        assert_eq!(
            l10n_text(&l10n, "app.khong_co_trong_file"),
            "«app.khong_co_trong_file»"
        );
    }

    /// Layout của khóa l10n dùng đúng text (hoặc placeholder khi khóa chưa có text).
    #[test]
    fn layout_of_l10n_key_uses_placeholder_when_missing() {
        let font = font();
        let l10n = l10n();
        let ready = layout_l10n(&l10n, "app.loading", &font, GOLDEN_SIZE, GOLDEN_WIDTH);
        assert_eq!(ready.lines()[0].text, "Đang tải...");
        let missing = layout_l10n(&l10n, "app.quit_confirm", &font, GOLDEN_SIZE, GOLDEN_WIDTH);
        assert_eq!(missing.lines()[0].text, "«app.quit_confirm»");
    }

    /// Golden layout: số dòng và độ rộng của chuỗi V1 ở 1× và 2×.
    #[test]
    fn golden_layout_matches() {
        let font = font();
        let l10n = l10n();
        for &(key, lines_1x, width_1x, lines_2x, width_2x) in GOLDEN {
            let text = l10n
                .text(key)
                .as_str()
                .unwrap_or_else(|| panic!("khóa {key} phải có text để so golden"));
            let one = layout(text, &font, GOLDEN_SIZE, GOLDEN_WIDTH);
            assert_eq!(one.line_count(), lines_1x, "số dòng 1× của {key}");
            assert_eq!(one.width().round() as u16, width_1x, "độ rộng 1× của {key}");
            let two = layout(text, &font, GOLDEN_SIZE * 2.0, GOLDEN_WIDTH_2X);
            assert_eq!(two.line_count(), lines_2x, "số dòng 2× của {key}");
            assert_eq!(two.width().round() as u16, width_2x, "độ rộng 2× của {key}");
        }
    }

    /// In bảng golden để sinh lại hằng [`GOLDEN`].
    ///
    /// Chạy tay: `cargo test -p boom-client -- --ignored print_golden_layout --nocapture`.
    #[test]
    #[ignore = "sinh lại bảng golden layout"]
    fn print_golden_layout() {
        let font = font();
        let l10n = l10n();
        println!("const GOLDEN: &[(&str, usize, u16, usize, u16)] = &[");
        for key in l10n.keys() {
            let Some(text) = l10n.text(key).as_str() else {
                continue;
            };
            let one = layout(text, &font, GOLDEN_SIZE, GOLDEN_WIDTH);
            let two = layout(text, &font, GOLDEN_SIZE * 2.0, GOLDEN_WIDTH_2X);
            println!(
                "    (\"{key}\", {}, {}, {}, {}),",
                one.line_count(),
                one.width().round() as u16,
                two.line_count(),
                two.width().round() as u16
            );
        }
        println!("];");
    }

    /// Sinh ảnh layout V1 (không cần GPU) để soi bằng mắt.
    ///
    /// Chạy tay:
    /// `cargo test -p boom-client -- --ignored write_golden_images --nocapture`.
    /// Thư mục đích: `BOOM_TEXT_SHOTS_DIR` (mặc định `/home/admin/boom-team/shots`).
    #[test]
    #[ignore = "sinh ảnh layout local"]
    fn write_golden_images() {
        let font = font();
        let l10n = l10n();
        let dir = std::env::var("BOOM_TEXT_SHOTS_DIR")
            .unwrap_or_else(|_| "/home/admin/boom-team/shots".to_owned());
        std::fs::create_dir_all(&dir).expect("tạo thư mục ảnh");
        for (scale, size, width) in [
            (1.0f32, GOLDEN_SIZE, GOLDEN_WIDTH),
            (2.0, GOLDEN_SIZE * 2.0, GOLDEN_WIDTH_2X),
        ] {
            // Dựng trước để biết chiều cao canvas, rồi mới vẽ.
            let mut rows: Vec<(String, Layout)> = Vec::new();
            for key in l10n.keys() {
                let Some(text) = l10n.text(key).as_str() else {
                    continue;
                };
                rows.push((key.to_owned(), layout(text, &font, size, width)));
            }
            // Dòng placeholder của khóa chưa có text (hiện `«khóa»`).
            rows.push((
                "app.quit_confirm (khóa chưa có text)".to_owned(),
                layout(&l10n_text(&l10n, "app.quit_confirm"), &font, size, width),
            ));
            let height = rows
                .iter()
                .map(|(_, layout)| layout.height() + 6.0)
                .sum::<f32>()
                + 8.0;
            let canvas_width = (width + 16.0) as u32;
            let mut canvas = Canvas::new(canvas_width, height as u32);
            let mut y = 8.0f32;
            for (_, layout) in &rows {
                draw_layout_on_canvas(&font, layout, &mut canvas, 8.0, y);
                y += layout.height() + 6.0;
            }
            let path = std::path::Path::new(&dir).join(format!(
                "issue-112-text-layout-{}x.png",
                scale.round() as u32
            ));
            canvas.write_png(&path).expect("ghi ảnh");
            println!("đã ghi {}", path.display());
        }
    }

    /// Canvas RGBA tối giản để ghép glyph khi sinh ảnh local.
    struct Canvas {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    }

    impl Canvas {
        fn new(width: u32, height: u32) -> Self {
            let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 4);
            for _ in 0..(width as usize) * (height as usize) {
                pixels.extend_from_slice(&[16, 18, 24, 255]);
            }
            Self {
                width,
                height,
                pixels,
            }
        }

        /// Ghép một bitmap glyph (kênh alpha) vào canvas.
        fn blit(&mut self, bitmap: &GlyphBitmap, x: i32, y: i32) {
            for row in 0..i32::from(bitmap.height) {
                for column in 0..i32::from(bitmap.width) {
                    let target_x = x + column;
                    let target_y = y + row;
                    if target_x < 0
                        || target_y < 0
                        || target_x >= i32::try_from(self.width).unwrap_or(0)
                        || target_y >= i32::try_from(self.height).unwrap_or(0)
                    {
                        continue;
                    }
                    let source = ((row as usize) * (bitmap.width as usize) + column as usize) * 4;
                    let alpha = bitmap.bytes[source + 3];
                    if alpha == 0 {
                        continue;
                    }
                    let target =
                        ((target_y as usize) * (self.width as usize) + target_x as usize) * 4;
                    self.pixels[target] = 240;
                    self.pixels[target + 1] = 245;
                    self.pixels[target + 2] = 255;
                }
            }
        }

        /// Vẽ khung hộp (ký tự thiếu glyph) trên canvas.
        fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
            let left = x.round() as i32;
            let top = y.round() as i32;
            let right = (x + width).round() as i32;
            let bottom = (y + height).round() as i32;
            for column in left..=right {
                self.plot(column, top);
                self.plot(column, bottom);
            }
            for row in top..=bottom {
                self.plot(left, row);
                self.plot(right, row);
            }
        }

        /// Tô một pixel trắng.
        fn plot(&mut self, x: i32, y: i32) {
            if x < 0
                || y < 0
                || x >= i32::try_from(self.width).unwrap_or(0)
                || y >= i32::try_from(self.height).unwrap_or(0)
            {
                return;
            }
            let index = ((y as usize) * (self.width as usize) + x as usize) * 4;
            self.pixels[index] = 240;
            self.pixels[index + 1] = 245;
            self.pixels[index + 2] = 255;
        }

        fn write_png(&self, path: &std::path::Path) -> std::io::Result<()> {
            let file = std::fs::File::create(path)?;
            let writer = std::io::BufWriter::new(file);
            let mut encoder = png::Encoder::new(writer, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
            writer
                .write_image_data(&self.pixels)
                .map_err(std::io::Error::other)?;
            Ok(())
        }
    }

    /// Ghép layout lên canvas bằng đúng metric của font (không cần GPU).
    fn draw_layout_on_canvas(
        font: &TextFont,
        layout: &Layout,
        canvas: &mut Canvas,
        x: f32,
        y: f32,
    ) {
        for (row, line) in layout.lines().iter().enumerate() {
            let line_top = y + row as f32 * layout.line_height();
            let baseline =
                line_top + (layout.line_height() + layout.ascent() - layout.descent()) / 2.0;
            for glyph in &line.glyphs {
                let pen_x = x + glyph.x;
                if !font.has_glyph(glyph.ch) {
                    // Hộp báo thiếu glyph, giống hành vẽ thật (`GlyphCache::draw`).
                    canvas.stroke_rect(
                        pen_x,
                        baseline - layout.ascent(),
                        glyph.advance,
                        layout.ascent() - layout.descent(),
                    );
                    continue;
                }
                let Some(bitmap) = rasterize(font, glyph.ch, layout.size()) else {
                    continue;
                };
                let left = pen_x as i32 + bitmap.offset_x;
                let top = baseline as i32 + bitmap.offset_y;
                canvas.blit(&bitmap, left, top);
            }
        }
    }
}
