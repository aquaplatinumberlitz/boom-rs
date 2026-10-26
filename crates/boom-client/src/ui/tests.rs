//! Test của UI luồng V1 — **không cần GPU** (acceptance V1-032).
//!
//! Ba điều được kiểm ở đây:
//!
//! 1. Chuyển màn đủ luồng Login → Sảnh → Phòng → Trận → Kết quả → Phòng, và
//!    **không màn nào là dead end** (mỗi màn có hành động đưa người chơi ra khỏi màn).
//! 2. Text hiển thị lấy **đúng khóa l10n** (khóa chưa có text hiện placeholder).
//! 3. Phần tử `needs_review_text`/`excluded` **không hiện ảnh** — chỉ phần tử
//!    `selected` mới được vẽ bằng ảnh.

use boom_content::bundle::{Bundle, EvidenceStatus, UiElement, UiRole, UiScreen, UiState};
use boom_content::rules::Source;

use super::demo::{DEMO_ACCOUNT, DEMO_PASSWORD, DemoBackend};
use super::layout::{self, Widget};
use super::model::{ActionId, ActionOutcome, FieldId, UiModel};
use super::text::Texts;

/// Model ở màn đăng nhập.
fn model() -> UiModel<DemoBackend> {
    UiModel::new(DemoBackend::new(), Texts::embedded())
}

/// Đăng nhập và sang màn sảnh.
fn logged_in() -> UiModel<DemoBackend> {
    let mut model = model();
    model.set_field_text(FieldId::LoginAccount, DEMO_ACCOUNT);
    model.set_field_text(FieldId::LoginPassword, DEMO_PASSWORD);
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
    panic!("lượt không kết thúc: màn vẫn là {:?}", model.screen());
}

/// Nhãn của widget `Title` đầu tiên.
fn title_of(model: &UiModel<DemoBackend>) -> String {
    layout::layout(model, None, None)
        .widgets
        .iter()
        .find_map(|widget| match widget {
            Widget::Title { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("màn phải có tiêu đề")
}

/// Nhãn của các nút theo thứ tự.
fn button_labels(model: &UiModel<DemoBackend>) -> Vec<String> {
    layout::layout(model, None, None)
        .widgets
        .iter()
        .filter_map(|widget| match widget {
            Widget::Button { label, .. } => Some(label.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn full_flow_login_lobby_room_match_result_room() {
    let mut model = model();
    assert_eq!(model.screen(), UiScreen::Login);

    // Đăng nhập → sảnh.
    model.set_field_text(FieldId::LoginAccount, DEMO_ACCOUNT);
    model.set_field_text(FieldId::LoginPassword, DEMO_PASSWORD);
    assert_eq!(model.activate(ActionId::LoginSubmit), ActionOutcome::Done);
    assert_eq!(model.screen(), UiScreen::Lobby);
    assert!(!model.rooms().is_empty());

    // Sảnh → phòng chờ.
    model.set_field_text(FieldId::LobbyRoomName, "room-test");
    assert_eq!(model.activate(ActionId::LobbyCreate), ActionOutcome::Done);
    assert_eq!(model.screen(), UiScreen::Room);
    assert!(model.room().is_some());

    // Phòng chờ → trận (sẵn sàng rồi bắt đầu).
    assert_eq!(model.activate(ActionId::RoomReady), ActionOutcome::Done);
    assert_eq!(model.activate(ActionId::RoomStart), ActionOutcome::Done);
    assert_eq!(model.screen(), UiScreen::Match);

    // Trận → kết quả (server chốt, client tự chuyển màn).
    finish_match(&mut model);
    assert_eq!(model.screen(), UiScreen::Result);
    let outcome = model.outcome().expect("phải có kết quả lượt");
    assert!(!outcome.entries.is_empty());

    // Kết quả → phòng.
    assert_eq!(
        model.activate(ActionId::ResultContinue),
        ActionOutcome::Done
    );
    assert_eq!(model.screen(), UiScreen::Room);
}

#[test]
fn no_screen_is_a_dead_end() {
    // Màn đăng nhập: lối ra là thoát ứng dụng.
    let mut model = model();
    assert!(!model.actions().is_empty());
    assert_eq!(model.back(), ActionOutcome::Quit);

    // Màn sảnh: lối ra là về đăng nhập (thoát phiên).
    let mut model = logged_in();
    assert!(!model.actions().is_empty());
    assert_eq!(model.back(), ActionOutcome::Done);
    assert_eq!(model.screen(), UiScreen::Login);

    // Màn phòng: lối ra là về sảnh.
    let mut model = in_room();
    assert!(!model.actions().is_empty());
    assert_eq!(model.back(), ActionOutcome::Done);
    assert_eq!(model.screen(), UiScreen::Lobby);

    // Màn trận: không có nút rời trận (rời trận là bị loại) nhưng vẫn có lối ra.
    let mut model = in_room();
    model.activate(ActionId::RoomReady);
    model.activate(ActionId::RoomStart);
    assert_eq!(model.screen(), UiScreen::Match);
    let actions = model.actions();
    assert!(!actions.is_empty());
    assert!(
        !actions
            .iter()
            .any(|action| action.id == ActionId::RoomLeave),
        "màn trận không được có nút rời phòng"
    );
    assert_eq!(model.back(), ActionOutcome::Quit);

    // Màn kết quả: lối ra là về phòng.
    let mut model = in_room();
    model.activate(ActionId::RoomReady);
    model.activate(ActionId::RoomStart);
    finish_match(&mut model);
    assert!(!model.actions().is_empty());
    assert_eq!(
        model.activate(ActionId::ResultContinue),
        ActionOutcome::Done
    );
    assert_eq!(model.screen(), UiScreen::Room);
}

#[test]
fn login_is_required_before_lobby() {
    let mut model = model();
    // Bỏ trống tài khoản thì không sang sảnh, có thông báo lỗi theo khóa l10n.
    model.set_field_text(FieldId::LoginPassword, DEMO_PASSWORD);
    assert_eq!(
        model.activate(ActionId::LoginSubmit),
        ActionOutcome::Rejected("login.empty")
    );
    assert_eq!(model.screen(), UiScreen::Login);
    assert_eq!(model.notice(), Some("login.empty"));
}

#[test]
fn wrong_password_shows_the_login_failed_key() {
    let mut model = model();
    model.set_field_text(FieldId::LoginAccount, DEMO_ACCOUNT);
    model.set_field_text(FieldId::LoginPassword, "sai-mat-khau");
    assert_eq!(
        model.activate(ActionId::LoginSubmit),
        ActionOutcome::Rejected("login.failed")
    );
    assert_eq!(model.screen(), UiScreen::Login);
}

#[test]
fn texts_come_from_the_l10n_keys() {
    let model = model();
    // Khóa đã được chốt tại catalog: kiểm đúng nội dung người chơi thấy.
    assert_eq!(title_of(&model), "Đăng nhập");
    let labels = button_labels(&model);
    assert_eq!(labels, vec!["Đăng ký", "Đăng nhập", "Thoát game"]);

    // Khóa có text chốt → lấy nguyên văn từ file chuỗi.
    let layout = layout::layout(&model, None, None);
    let password = layout.widgets.iter().find_map(|widget| match widget {
        Widget::Field { label, .. } if label == "Mật khẩu" => Some(label.clone()),
        _ => None,
    });
    assert_eq!(password.as_deref(), Some("Mật khẩu"));
}

#[test]
fn lobby_cards_show_backend_rooms_and_empty_slots() {
    let model = logged_in();
    let layout = layout::layout(&model, None, None);
    let cards: Vec<_> = layout
        .widgets
        .iter()
        .filter_map(|widget| match widget {
            Widget::Card { rect, lines, .. } if rect.x >= 227 && rect.y >= 77 && rect.y < 320 => {
                Some(lines)
            }
            _ => None,
        })
        .collect();
    // R-228: sáu thẻ, dữ liệu backend giữ đủ tên/host/số người.
    assert_eq!(cards.len(), 6);
    for room in model.rooms() {
        assert!(
            cards
                .iter()
                .any(|lines| lines.contains(&room.name) && lines.contains(&room.host))
        );
    }
}

/// Bundle giả với bảng `[[ui]]` của màn kết quả: một phần tử `selected` (được vẽ
/// bằng ảnh), một `needs_review_text` và một `excluded`.
fn ui_bundle() -> Bundle {
    Bundle {
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
        ui: vec![
            UiElement {
                id: "result.info_good".to_owned(),
                screen: UiScreen::Result,
                role: UiRole::Info,
                entry: Some("FxStage/Game/ResultInfoGood".to_owned()),
                state: UiState::Selected,
                reason: "chỉ có icon, không có chữ".to_owned(),
                sprite: Some("FxStage/Game/ResultInfoGood".to_owned()),
                source: Source::KrStatic,
                status: EvidenceStatus::Confirmed,
            },
            UiElement {
                id: "result.win".to_owned(),
                screen: UiScreen::Result,
                role: UiRole::ResultArt,
                entry: Some("FxStage/Game/Win".to_owned()),
                state: UiState::NeedsReviewText,
                reason: "ảnh có chữ, chưa duyệt".to_owned(),
                sprite: Some("FxStage/Game/Win".to_owned()),
                source: Source::KrStatic,
                status: EvidenceStatus::Confirmed,
            },
            UiElement {
                id: "result.background".to_owned(),
                screen: UiScreen::Result,
                role: UiRole::Background,
                entry: Some("FxStage/Game/CAStage".to_owned()),
                state: UiState::Excluded,
                reason: "nền có chữ, V1 dựng nền bằng màu".to_owned(),
                sprite: None,
                source: Source::KrStatic,
                status: EvidenceStatus::Confirmed,
            },
        ],
        audio: Vec::new(),
    }
}

#[test]
fn only_selected_ui_elements_are_drawn_as_images() {
    let mut model = in_room();
    model.activate(ActionId::RoomReady);
    model.activate(ActionId::RoomStart);
    finish_match(&mut model);
    assert_eq!(model.screen(), UiScreen::Result);

    let bundle = ui_bundle();
    let layout = layout::layout(&model, Some(&bundle), None);
    let images: Vec<&str> = layout
        .widgets
        .iter()
        .filter_map(|widget| match widget {
            Widget::Art { sprite, .. } => Some(sprite.as_str()),
            _ => None,
        })
        .collect();
    // Chỉ phần tử `selected` được vẽ bằng ảnh.
    assert_eq!(images, vec!["FxStage/Game/ResultInfoGood"]);

    // Phần tử `needs_review_text`/`excluded` thay bằng khung màu + text catalog.
    let frames: Vec<&str> = layout
        .widgets
        .iter()
        .filter_map(|widget| match widget {
            Widget::Frame { label, .. } if !label.is_empty() => Some(label.as_str()),
            _ => None,
        })
        .collect();
    // `result.win` có text chốt trong file chuỗi → khung hiện text catalog.
    assert!(frames.contains(&"Thắng"), "khung: {frames:?}");
    // Nhãn nền đã có text trong catalog; fixture vẫn không được bật ảnh excluded.
    assert!(frames.contains(&"Kết quả"), "khung: {frames:?}");
    // Và không được vẽ ảnh của hai phần tử đó.
    assert!(!images.contains(&"FxStage/Game/Win"));
}

#[test]
fn room_lists_members_and_chat() {
    let mut model = in_room();
    model.set_field_text(FieldId::RoomChat, "xin chào");
    assert_eq!(model.activate(ActionId::RoomChatSend), ActionOutcome::Done);
    let room = model.room().expect("đang ở phòng");
    assert_eq!(room.chat.len(), 1);
    assert_eq!(room.chat[0].text, "xin chào");

    let layout = layout::layout(&model, None, None);
    let cards: Vec<_> = layout
        .widgets
        .iter()
        .filter_map(|widget| match widget {
            Widget::Card { lines, rect, .. } if rect.y == 196 => Some(lines),
            _ => None,
        })
        .collect();
    assert_eq!(cards.len(), 4);
    for member in &room.members {
        assert!(cards.iter().any(|lines| lines.contains(&member.name)));
    }
}

#[test]
fn character_can_be_switched_in_the_room() {
    let mut model = in_room();
    let before = model.room().expect("đang ở phòng").self_character;
    model.cycle_character();
    let after = model.room().expect("đang ở phòng").self_character;
    assert_ne!(before, after);
    // Ngoài phòng chờ thì lệnh đổi nhân vật không có tác dụng.
    let mut model = logged_in();
    model.cycle_character();
    assert_eq!(model.screen(), UiScreen::Lobby);
}

#[test]
fn focus_cycles_through_fields_and_buttons() {
    let mut model = model();
    let focusables = model.focusables();
    assert!(focusables.len() >= 3);
    let first = model.focus();
    model.focus_step(false);
    assert_ne!(model.focus(), first);
    model.focus_step(true);
    assert_eq!(model.focus(), first);
}

#[test]
fn typing_goes_into_the_focused_field() {
    let mut model = model();
    model.type_char('a');
    model.type_char('b');
    assert_eq!(model.field_value(FieldId::LoginAccount), "ab");
    model.backspace();
    assert_eq!(model.field_value(FieldId::LoginAccount), "a");
    // Mật khẩu không hiện ra: layout đánh dấu ô là masked.
    model.focus_step(false);
    model.type_char('x');
    let layout = layout::layout(&model, None, None);
    let masked = layout
        .widgets
        .iter()
        .any(|widget| matches!(widget, Widget::Field { masked: true, value, .. } if value == "x"));
    assert!(masked, "mật khẩu phải được đánh dấu để che");
}

/// Ảnh Hòa được duyệt vẫn chỉ dùng cho kết quả hòa, không hiện cạnh nhãn Thắng.
#[test]
fn approved_draw_art_is_not_shown_for_a_winner() {
    let mut model = in_room();
    model.activate(ActionId::RoomReady);
    model.activate(ActionId::RoomStart);
    finish_match(&mut model);
    assert_eq!(
        model.outcome().unwrap().self_result,
        crate::ui::model::MatchResult::Win
    );
    let mut bundle = ui_bundle();
    let mut draw = bundle.ui[1].clone();
    draw.id = "result.draw".to_owned();
    draw.state = UiState::Selected;
    draw.sprite = Some("FxStage/Game/Draw".to_owned());
    bundle.ui.push(draw);
    let layout = layout::layout(&model, Some(&bundle), None);
    assert!(!layout.widgets.iter().any(|widget| matches!(widget,
        Widget::Art { sprite, .. } if sprite == "FxStage/Game/Draw")));
}

/// Lỗi đăng ký trùng tài khoản phải khác thông báo sai mật khẩu.
#[test]
fn account_exists_has_its_own_resolved_message() {
    use crate::ui::model::UiError;
    let texts = Texts::embedded();
    assert_eq!(UiError::AccountExists.message_key(), "login.account_exists");
    assert_eq!(UiError::InvalidCredentials.message_key(), "login.failed");
    assert_ne!(
        texts.get(UiError::AccountExists.message_key()),
        texts.get(UiError::InvalidCredentials.message_key())
    );
}

// Regression theo visual rect R-227; không thay bằng chứng fidelity local.
#[test]
fn login_visual_rects_and_password_enabled_state() {
    let mut model = model();
    let layout = layout::layout(&model, None, None);
    let fields: Vec<_> = layout
        .widgets
        .iter()
        .filter_map(|widget| match widget {
            Widget::Field { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(
        fields,
        vec![
            layout::Rect::new(402, 461, 92, 20),
            layout::Rect::new(402, 486, 92, 20)
        ]
    );
    assert!(layout.widgets.iter().any(|widget| matches!(
        widget,
        Widget::Button {
            action: ActionId::LoginSubmit,
            enabled: false,
            ..
        }
    )));
    model.set_field_text(FieldId::LoginPassword, "x");
    let layout = layout::layout(&model, None, None);
    assert!(layout.widgets.iter().any(|widget| matches!(
        widget,
        Widget::Button {
            action: ActionId::LoginSubmit,
            enabled: true,
            ..
        }
    )));
}

#[test]
fn lobby_selection_keeps_rooms_beyond_first_six_visible() {
    let mut model = logged_in();
    for index in 0..7 {
        model.set_field_text(FieldId::LobbyRoomName, &format!("page-room-{index}"));
        assert_eq!(model.activate(ActionId::LobbyCreate), ActionOutcome::Done);
        assert_eq!(model.activate(ActionId::RoomLeave), ActionOutcome::Done);
    }
    for _ in 1..model.rooms().len() {
        model.move_selection(1);
    }
    let selected = model.selected_room().expect("room selection");
    let layout = layout::layout(&model, None, None);
    assert!(layout.widgets.iter().any(|widget| matches!(widget,
        Widget::Card { lines, selected: true, .. } if lines.contains(&selected.name))));
}
