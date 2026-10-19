//! Vòng lặp app của chế độ demo UI (macroquad).
//!
//! Chế độ `--ui-demo` chạy trọn luồng màn hình V1 với nguồn dữ liệu giả lập
//! ([`crate::ui::demo::DemoBackend`]) — đăng nhập → sảnh → phòng chờ → trận → kết
//! quả → phòng — **không cần server**. Bản mạng nối vào sau khi #89/#99 merge
//! (task riêng), không đổi phần model.

use std::path::{Path, PathBuf};

use boom_content::bundle::Bundle;
use boom_content::map::MapProfile;
use boom_content::rules::RulesProfile;
use boom_core::Rules;
use macroquad::prelude::*;

use crate::render::Atlas;
use crate::scene::{self, Scene};
use crate::ui::demo::{DemoBackend, ShotScreen, jump_to};
use crate::ui::draw;
use crate::ui::layout;
use crate::ui::model::{ActionOutcome, UiModel};

/// Tuỳ chọn của chế độ demo UI.
pub struct UiOptions {
    /// Thư mục bundle (tuỳ chọn): màn trận vẽ map thật, ảnh UI đã duyệt vẽ được.
    pub bundle: Option<PathBuf>,
    /// Ảnh chụp: ghi xong rồi thoát.
    pub snapshot: Option<PathBuf>,
    /// Màn cần chụp (nếu có): nhảy thẳng tới màn đó trước khi chụp.
    pub shot: Option<ShotScreen>,
}

/// Nội dung bundle đã nạp cho chế độ demo UI.
struct Content {
    /// Metadata bundle (bảng `[[ui]]`, atlas).
    bundle: Option<Bundle>,
    /// Scene dựng từ bundle (ô của map, vật cản).
    scene: Option<Scene>,
    /// Trang atlas đã nạp thành texture.
    atlas: Option<Atlas>,
}

impl Content {
    /// Không có bundle: màn trận vẽ khung trống, ảnh UI vẽ khung màu + text.
    fn empty() -> Self {
        Self {
            bundle: None,
            scene: None,
            atlas: None,
        }
    }
}

/// Chạy chế độ demo UI: mở vòng lặp vẽ cho tới khi đóng cửa sổ (hoặc chụp ảnh).
pub async fn run_demo(mut model: UiModel<DemoBackend>, opts: UiOptions) {
    let font = draw::load_font();
    // Atlas cần GL context nên phải nạp sau khi cửa sổ đã mở.
    let content = load_content(opts.bundle.as_deref());
    if let Some(screen) = opts.shot {
        jump_to(&mut model, screen);
    }
    let mut frame = 0u32;
    loop {
        if handle_input(&mut model) {
            return;
        }
        model.tick();
        let layout = layout::layout(&model, content.bundle.as_ref(), content.scene.as_ref());
        draw::draw(
            &layout,
            &font,
            content.atlas.as_ref(),
            content.bundle.as_ref(),
            content.scene.as_ref(),
        );
        frame += 1;
        if let Some(path) = &opts.snapshot
            && frame >= 3
        {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            get_screen_data().export_png(&path.to_string_lossy());
            println!("ảnh chụp màn UI: {}", path.display());
            return;
        }
        next_frame().await;
    }
}

/// Xử lý phím; trả về `true` nếu cần thoát ứng dụng.
fn handle_input(model: &mut UiModel<DemoBackend>) -> bool {
    if is_key_pressed(KeyCode::Escape) {
        return matches!(model.back(), ActionOutcome::Quit);
    }
    if is_key_pressed(KeyCode::Tab) {
        let back = is_key_down(KeyCode::LeftShift);
        model.focus_step(back);
    }
    if is_key_pressed(KeyCode::Up) {
        model.move_selection(-1);
    }
    if is_key_pressed(KeyCode::Down) {
        model.move_selection(1);
    }
    if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Right) {
        // Trong phòng chờ: đổi nhân vật C00 ⇄ C01.
        model.cycle_character();
    }
    if is_key_pressed(KeyCode::Enter) && matches!(model.submit_focused(), ActionOutcome::Quit) {
        return true;
    }
    if is_key_pressed(KeyCode::Backspace) {
        model.backspace();
    }
    while let Some(character) = get_char_pressed() {
        model.type_char(character);
    }
    false
}

/// Nạp bundle content (tuỳ chọn) cho chế độ demo UI.
///
/// Không có bundle thì chế độ demo vẫn chạy: màn trận vẽ khung trống, ảnh UI chưa
/// duyệt vẽ khung màu + text catalog.
fn load_content(dir: Option<&Path>) -> Content {
    let Some(dir) = dir else {
        return Content::empty();
    };
    let profile = match RulesProfile::from_toml(include_str!("../../../../data/rules/v1-dev.toml"))
    {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("không nạp được rules profile: {error}");
            return Content::empty();
        }
    };
    let rules = match Rules::from_profile(&profile) {
        Ok(rules) => rules,
        Err(error) => {
            eprintln!("không dựng được rules: {error}");
            return Content::empty();
        }
    };
    let tile_px = u32::try_from(rules.tile.0 / 1000).unwrap_or(0);
    let map_profile =
        match MapProfile::from_toml(include_str!("../../../../data/maps/village00-kr.toml")) {
            Ok(profile) => profile,
            Err(error) => {
                eprintln!("không nạp được bảng phân loại ô: {error}");
                return Content::empty();
            }
        };
    let loaded = match scene::load(dir, &map_profile, tile_px) {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("không nạp được bundle: {error}");
            return Content::empty();
        }
    };
    let atlas = match Atlas::load(dir, &loaded.scene.pages) {
        Ok(atlas) => Some(atlas),
        Err(error) => {
            eprintln!("không nạp được trang atlas: {error}");
            None
        }
    };
    Content {
        bundle: Some(loaded.bundle),
        scene: Some(loaded.scene),
        atlas,
    }
}
