//! Kiểm tra l10n của UI V1 (V1-035): glyph, khóa, chuỗi cứng và tràn hộp.
//!
//! Bốn điều được kiểm ở đây — **không cần GPU**:
//!
//! 1. **Glyph**: mọi chữ trong file chuỗi V1 và cả placeholder đều có glyph trong
//!    font OFL đã bundle (Be Vietnam Pro 1.002) — không có hộp báo thiếu nào lọt
//!    ra màn hình (UI_STRINGS_V1 §5.1).
//! 2. **Khóa**: mọi khóa client dùng (kể cả khóa của phần tử UI trong bundle) đều
//!    có trong file chuỗi; không có chuỗi hiển thị nào là placeholder của một khóa
//!    **không có** trong file (đó là lỗi gõ sai, không phải chờ duyệt).
//! 3. **Chuỗi cứng**: trong mã đường chạy của UI không có câu chữ viết tay — mọi
//!    chữ người chơi thấy lấy từ file chuỗi (lint đơn giản quét mã nguồn).
//! 4. **Tràn hộp**: chữ trong layout V1 xuống dòng đúng quy tắc §5.2.1 và **vừa**
//!    hộp của nó (không tràn ra ngoài, không mất chữ).

use boom_content::bundle::UiScreen;
use boom_content::l10n::{self, L10n};

use crate::text::TextFont;
use crate::ui::demo::DemoBackend;
use crate::ui::layout::{self, Widget};
use crate::ui::model::{ActionId, ActionOutcome, FieldId, UiModel};
use crate::ui::text::Texts;

/// File chuỗi V1 đã xuất (output của `tools/l10n_export.py`).
const L10N_FILE: &str = include_str!("../../../../data/l10n/vi-v1.toml");

/// Mã nguồn đường chạy của UI — nơi có thể xuất hiện chữ cho người chơi.
const UI_SOURCES: &[(&str, &str)] = &[
    ("ui/app.rs", include_str!("app.rs")),
    ("ui/demo.rs", include_str!("demo.rs")),
    ("ui/draw.rs", include_str!("draw.rs")),
    ("ui/layout.rs", include_str!("layout.rs")),
    ("ui/model.rs", include_str!("model.rs")),
    ("ui/text.rs", include_str!("text.rs")),
];

/// Font OFL đã bundle (Be Vietnam Pro 1.002).
fn font() -> TextFont {
    TextFont::regular().expect("font regular")
}

/// Bộ chuỗi V1 nạp từ file trong repo.
fn l10n() -> L10n {
    L10n::from_toml(L10N_FILE).expect("file chuỗi V1")
}

/// Model ở màn đăng nhập.
fn model() -> UiModel<DemoBackend> {
    UiModel::new(DemoBackend::new(), Texts::embedded())
}

/// Đăng nhập và sang màn sảnh.
fn logged_in() -> UiModel<DemoBackend> {
    let mut model = model();
    model.set_field_text(FieldId::LoginAccount, "demo");
    model.set_field_text(FieldId::LoginPassword, "demo-pass");
    assert_eq!(model.activate(ActionId::LoginSubmit), ActionOutcome::Done);
    model
}

/// Tạo phòng và sang màn phòng chờ.
fn in_room() -> UiModel<DemoBackend> {
    let mut model = logged_in();
    model.set_field_text(FieldId::LobbyRoomName, "room-test");
    assert_eq!(model.activate(ActionId::LobbyCreate), ActionOutcome::Done);
    model
}

/// Chạy hết lượt để sang màn kết quả.
fn finish_match(model: &mut UiModel<DemoBackend>) {
    for _ in 0..2_000 {
        model.tick();
        if model.screen() == UiScreen::Result {
            return;
        }
    }
    panic!("lượt không kết thúc");
}

/// Mọi màn của luồng V1, dựng layout kèm bảng UI của bundle V1.
fn v1_layouts() -> Vec<(UiScreen, layout::Layout)> {
    let bundle = bundle_ui();
    let mut layouts = Vec::new();

    let login = model();
    layouts.push((login.screen(), layout::layout(&login, Some(&bundle), None)));

    let lobby = logged_in();
    layouts.push((lobby.screen(), layout::layout(&lobby, Some(&bundle), None)));

    let room = in_room();
    layouts.push((room.screen(), layout::layout(&room, Some(&bundle), None)));

    let mut playing = in_room();
    playing.activate(ActionId::RoomReady);
    playing.activate(ActionId::RoomStart);
    layouts.push((
        playing.screen(),
        layout::layout(&playing, Some(&bundle), None),
    ));

    let mut finished = in_room();
    finished.activate(ActionId::RoomReady);
    finished.activate(ActionId::RoomStart);
    finish_match(&mut finished);
    layouts.push((
        finished.screen(),
        layout::layout(&finished, Some(&bundle), None),
    ));

    layouts
}

/// Bảng `[[ui]]` của bundle V1 (nguồn: `data/assets/ui-v1.toml`).
fn bundle_ui() -> boom_content::bundle::Bundle {
    let table = boom_content::assets::UiTable::from_toml(boom_content::assets::UI_V1)
        .expect("bảng UI V1 phải hợp lệ");
    let ui = table
        .elements
        .iter()
        .map(|element| boom_content::bundle::UiElement {
            id: element.id.clone(),
            screen: element.screen,
            role: element.role,
            entry: element.entry.clone(),
            state: element.state,
            reason: element.reason.clone(),
            sprite: element.entry.clone(),
            source: element.source,
            status: element.status.into(),
        })
        .collect();
    boom_content::bundle::Bundle {
        schema: boom_content::bundle::SCHEMA,
        build: "kr".to_owned(),
        map: "Village00".to_owned(),
        tileset: "Tile00".to_owned(),
        width: 1,
        height: 1,
        spawns: Vec::new(),
        content_hash: String::new(),
        atlas: Vec::new(),
        sprites: Vec::new(),
        assets: Vec::new(),
        cells: Vec::new(),
        unknown: Vec::new(),
        clips: Vec::new(),
        ui,
        audio: Vec::new(),
    }
}

/// Mọi chữ trong file chuỗi V1 và cả placeholder đều phải có glyph trong font.
#[test]
fn every_string_and_the_placeholder_have_glyphs() {
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
                "khóa {key}: ký tự {ch:?} (U+{:04X}) không có glyph trong font đã bundle",
                ch as u32
            );
        }
    }
    assert_eq!(checked, 44, "số khóa V1 có text trong file chuỗi");

    // Placeholder của khóa chưa có text cũng phải vẽ được (không hộp báo thiếu).
    let placeholder = l10n::placeholder("app.quit_confirm");
    for ch in placeholder.chars() {
        assert!(
            font.has_glyph(ch),
            "placeholder {placeholder:?}: ký tự {ch:?} không có glyph trong font đã bundle"
        );
    }
}

/// Mọi khóa mà client dùng đều có trong file chuỗi.
#[test]
fn every_key_used_by_the_client_exists_in_the_file() {
    let l10n = l10n();
    for (name, source) in UI_SOURCES {
        for key in key_literals(source) {
            assert!(
                l10n.entry(&key).is_some(),
                "{name}: khóa `{key}` không có trong file chuỗi — thêm vào \
                 docs/specification/localization/UI_STRINGS_V1.md rồi xuất lại file"
            );
        }
    }
}

/// Mọi phần tử UI của bundle đều có khóa trong file chuỗi.
///
/// Client vẽ nhãn của phần tử UI bằng chính ID của nó, nên ID phải là khóa có trong
/// file chuỗi (UI_STRINGS_V1 §3).
#[test]
fn every_bundle_ui_element_has_a_string_key() {
    let l10n = l10n();
    for element in bundle_ui().ui {
        assert!(
            l10n.entry(&element.id).is_some(),
            "phần tử UI `{}` chưa có khóa trong file chuỗi",
            element.id
        );
    }
}

/// Không có chuỗi hiển thị nào là placeholder của khóa **không có** trong file.
///
/// Placeholder của khóa có trong file nhưng chưa có text là trạng thái chờ người
/// duyệt (hợp lệ); placeholder của khóa không có trong file là lỗi gõ sai.
#[test]
fn no_displayed_placeholder_belongs_to_an_unknown_key() {
    let l10n = l10n();
    let font = font();
    for (screen, layout) in v1_layouts() {
        for widget in &layout.widgets {
            for text_box in widget.text_boxes(&font) {
                let Some(key) = l10n::placeholder_key(&text_box.text) else {
                    continue;
                };
                assert!(
                    l10n.entry(key).is_some(),
                    "màn {screen:?}: hiện placeholder của khóa `{key}` không có trong file chuỗi"
                );
            }
        }
    }
}

/// Chữ trong layout V1 vừa hộp của nó và không mất chữ.
#[test]
fn v1_texts_fit_their_boxes_and_keep_every_word() {
    let font = font();
    for (screen, layout) in v1_layouts() {
        for widget in &layout.widgets {
            for text_box in widget.text_boxes(&font) {
                let laid =
                    crate::text::layout(&text_box.text, &font, text_box.size, text_box.width);
                assert!(
                    laid.height() <= text_box.height + 0.5,
                    "màn {screen:?}: chữ {:?} cao {} px vượt hộp {} px",
                    text_box.text,
                    laid.height(),
                    text_box.height
                );
                for line in laid.lines() {
                    assert!(
                        line.width <= text_box.width + 0.5,
                        "màn {screen:?}: dòng {:?} rộng {} px vượt hộp {} px",
                        line.text,
                        line.width,
                        text_box.width
                    );
                }
                // Không được mất chữ: khớp lại các dòng phải cho lại đúng nội dung.
                let joined = laid
                    .lines()
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                assert_eq!(
                    joined.split_whitespace().collect::<Vec<_>>().join(" "),
                    text_box
                        .text
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                    "màn {screen:?}: chữ {:?} bị cắt khi xuống dòng",
                    text_box.text
                );
            }
        }
    }
}

/// Bảng V1 không được tràn khung bên (tổng bề rộng cột + khoảng cách).
#[test]
fn v1_tables_fit_their_width() {
    let font = font();
    for (screen, layout) in v1_layouts() {
        for widget in &layout.widgets {
            let Widget::Table {
                rect,
                columns,
                rows,
            } = widget
            else {
                continue;
            };
            let widths = layout::table_column_widths(columns, rows, &font);
            let total: f32 = widths.iter().sum::<f32>()
                + layout::TABLE_COLUMN_GAP * widths.len().saturating_sub(1) as f32;
            assert!(
                total <= rect.width as f32 + 0.5,
                "màn {screen:?}: bảng rộng {total} px vượt khung {} px",
                rect.width
            );
        }
    }
}

/// Chat dài hơn khung thì chỉ vẽ dòng vừa khung, không tràn xuống dưới.
#[test]
fn chat_longer_than_the_frame_is_cut_not_overflowed() {
    let mut model = in_room();
    for index in 0..12 {
        model.set_field_text(FieldId::RoomChat, &format!("tin {index}"));
        assert_eq!(model.activate(ActionId::RoomChatSend), ActionOutcome::Done);
    }
    let font = font();
    let layout = layout::layout(&model, None, None);
    let lines = layout
        .widgets
        .iter()
        .find_map(|widget| match widget {
            Widget::Chat { lines, .. } => Some(lines.clone()),
            _ => None,
        })
        .expect("phòng chờ phải có khung chat");
    assert_eq!(lines.len(), 12, "mô hình giữ đủ chat");

    // Phần vẽ chỉ lấy các dòng vừa khung.
    let boxes = layout
        .widgets
        .iter()
        .find_map(|widget| match widget {
            Widget::Chat { .. } => Some(widget.text_boxes(&font)),
            _ => None,
        })
        .expect("khung chat");
    let rect_height = 128.0;
    let visible = (rect_height / layout::CHAT_LINE_HEIGHT).floor() as usize;
    assert_eq!(boxes.len(), visible, "chỉ vẽ dòng vừa khung");
    assert!(
        boxes
            .last()
            .is_some_and(|last| last.text.contains("tin 11")),
        "dòng mới nhất phải còn trong khung: {boxes:?}"
    );
    for text_box in &boxes {
        assert!(
            text_box.y + text_box.height <= 300.0 + rect_height + 0.5,
            "dòng chat tràn khung: {text_box:?}"
        );
    }
}

/// Trong mã đường chạy của UI không có câu chữ viết tay.
///
/// Mọi chữ người chơi thấy phải lấy từ file chuỗi; chỉ log/diễn đạt kỹ thuật được
/// miễn (issue #179).
#[test]
fn no_hardcoded_player_text_outside_l10n() {
    let l10n = l10n();
    let texts: Vec<String> = l10n
        .keys()
        .filter_map(|key| l10n.text(key).as_str().map(|text| text.to_owned()))
        .collect();
    for (name, source) in UI_SOURCES {
        for literal in prose_literals(source) {
            assert!(
                texts.contains(&literal),
                "{name}: chữ {literal:?} viết tay trong mã — lấy từ file chuỗi \
                 (khóa trong UI_STRINGS_V1) hoặc bỏ sang log/diễn đạt kỹ thuật"
            );
        }
    }
}

/// Các khóa l10n nằm trong mã nguồn (dạng `mục.khóa`).
fn key_literals(source: &str) -> Vec<String> {
    string_literals(source)
        .into_iter()
        .filter(|literal| {
            let parts: Vec<&str> = literal.split('.').collect();
            parts.len() >= 2
                && parts
                    .iter()
                    .all(|part| !part.is_empty() && part.chars().all(is_key_char))
                && parts[0]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_lowercase())
        })
        .collect()
}

/// Ký tự được phép trong một phần của khóa l10n (chữ thường, số, gạch dưới).
fn is_key_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'
}

/// Các literal chuỗi "giống văn bản" trong mã nguồn, bỏ qua log và module test.
///
/// "Giống văn bản" = có khoảng trắng **và** chữ cái. Chuỗi định dạng (`{}`, `%s`, `{0}`) không
/// phải văn bản — nó là khuôn, phần chữ lấy từ file chuỗi được ghép vào.
fn prose_literals(source: &str) -> Vec<String> {
    string_literals(source)
        .into_iter()
        .filter(|literal| {
            literal.contains(char::is_whitespace)
                && literal.chars().any(|ch| ch.is_alphabetic())
                && !literal.contains('{')
                && !literal.contains('%')
        })
        .collect()
}

/// Các literal chuỗi trong mã nguồn, bỏ qua comment, module test và lời gọi log.
///
/// Bỏ qua:
/// - comment (`//`, `/* */`) và khối `#[cfg(test)]`;
/// - lời gọi `eprintln!`/`println!`/`panic!`/`assert*!`/`expect(...)`/`unwrap*`.
///
/// Đây là log/diễn đạt kỹ thuật, không phải chữ cho người chơi (issue #179).
fn string_literals(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut depth = 0i32;
    // Độ sâu ngoặc mà lời gọi đang bỏ qua kết thúc.
    let mut skip_depth: Option<i32> = None;
    while index < chars.len() {
        let ch = chars[index];
        // Bỏ qua khối `#[cfg(test)]` (module test không nằm trên đường chạy).
        if ch == '#' && chars.get(index + 1) == Some(&'[') {
            let end = find_line_end(&chars, index);
            if chars[index..end]
                .iter()
                .collect::<String>()
                .contains("cfg(test)")
            {
                index = skip_braced_block(&chars, end);
                continue;
            }
        }
        // Comment dòng.
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            index = find_line_end(&chars, index);
            continue;
        }
        // Comment khối.
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            let mut end = index + 2;
            while end + 1 < chars.len() && !(chars[end] == '*' && chars[end + 1] == '/') {
                end += 1;
            }
            index = (end + 2).min(chars.len());
            continue;
        }
        // Literal chuỗi.
        if ch == '"' && skip_depth.is_none() {
            let mut literal = String::new();
            let mut cursor = index + 1;
            while cursor < chars.len() && chars[cursor] != '"' {
                if chars[cursor] == '\\' {
                    literal.push(chars[cursor]);
                    cursor += 1;
                    if cursor < chars.len() {
                        literal.push(chars[cursor]);
                        cursor += 1;
                    }
                    continue;
                }
                literal.push(chars[cursor]);
                cursor += 1;
            }
            out.push(literal);
            index = cursor + 1;
            continue;
        }
        // Ngoặc: nhận ra lời gọi cần bỏ qua (log/diễn đạt kỹ thuật).
        if ch == '(' {
            depth += 1;
            if skip_depth.is_none()
                && let Some(name) = call_name(&chars, index)
                && SKIP_CALLS.contains(&name.as_str())
            {
                skip_depth = Some(depth);
            }
            index += 1;
            continue;
        }
        if ch == ')' {
            depth -= 1;
            if skip_depth.is_some_and(|target| depth < target) {
                skip_depth = None;
            }
            index += 1;
            continue;
        }
        index += 1;
    }
    out
}

/// Lời gọi không phải chữ cho người chơi (log/diễn đạt kỹ thuật).
const SKIP_CALLS: &[&str] = &[
    "eprintln",
    "println",
    "eprint",
    "print",
    "panic",
    "todo",
    "unimplemented",
    "unreachable",
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "write",
    "writeln",
    "expect",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_err",
];

/// Tên hàm/macro đứng trước dấu `(` tại `index` (bỏ qua dấu `!` của macro).
fn call_name(chars: &[char], index: usize) -> Option<String> {
    let mut end = index;
    if end > 0 && chars[end - 1] == '!' {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let name: String = chars[start..end].iter().collect();
    if name.is_empty() { None } else { Some(name) }
}

/// Ký tự trong tên định danh.
fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Cuối dòng (không gồm ký tự xuống dòng).
fn find_line_end(chars: &[char], index: usize) -> usize {
    let mut end = index;
    while end < chars.len() && chars[end] != '\n' {
        end += 1;
    }
    end
}

/// Bỏ qua khối `{ ... }` sau `index` (dùng cho module test).
fn skip_braced_block(chars: &[char], index: usize) -> usize {
    let mut cursor = index;
    // Tìm dấu `{` đầu tiên sau chỗ này.
    while cursor < chars.len() && chars[cursor] != '{' {
        cursor += 1;
    }
    if cursor >= chars.len() {
        return chars.len();
    }
    let mut depth = 0i32;
    while cursor < chars.len() {
        match chars[cursor] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return cursor + 1;
                }
            }
            '"' => {
                cursor += 1;
                while cursor < chars.len() && chars[cursor] != '"' {
                    if chars[cursor] == '\\' {
                        cursor += 1;
                    }
                    cursor += 1;
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    chars.len()
}
