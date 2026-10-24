//! Cửa sổ và phép phóng nguyên dùng chung cho UI và trận (V1-033/G5).

use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Viewport {
    pub x: u32,
    pub y: u32,
    pub scale: u32,
}

impl Viewport {
    /// Giữ đủ canvas; màn lớn hơn dùng viền, không kéo giãn theo từng trục.
    fn fit(logical: [u32; 2], physical: [u32; 2], preferred: u32) -> Option<Self> {
        if logical.contains(&0) {
            return None;
        }
        let available = (physical[0] / logical[0]).min(physical[1] / logical[1]);
        if available == 0 {
            return None;
        }
        let scale = available.min(preferred.max(1));
        Some(Self {
            x: (physical[0] - logical[0] * scale) / 2,
            y: (physical[1] - logical[1] * scale) / 2,
            scale,
        })
    }
}

/// Trình diễn cùng canvas logic trên cửa sổ và toàn màn hình.
pub(crate) fn present_target(
    target: &RenderTarget,
    material: &Material,
    logical: [u32; 2],
    preferred: u32,
) {
    set_default_camera();
    clear_background(Color::new(0.01, 0.01, 0.02, 1.0));
    let Some(viewport) = Viewport::fit(
        logical,
        [screen_width() as u32, screen_height() as u32],
        preferred,
    ) else {
        return; // WindowControls yêu cầu lại kích thước tối thiểu ở frame này.
    };
    gl_use_material(material);
    draw_texture_ex(
        &target.texture,
        viewport.x as f32,
        viewport.y as f32,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(
                (logical[0] * viewport.scale) as f32,
                (logical[1] * viewport.scale) as f32,
            )),
            flip_y: true,
            ..Default::default()
        },
    );
    gl_use_default_material();
}

/// Canvas UI cố định: cả chữ và art được phóng từ cùng một ảnh logic.
pub(crate) struct UiCanvas {
    target: RenderTarget,
    material: Material,
}

impl UiCanvas {
    pub fn new() -> Self {
        let target = render_target(crate::ui::WIDTH, crate::ui::HEIGHT);
        target.texture.set_filter(FilterMode::Nearest);
        Self {
            target,
            material: crate::load_copy_material(),
        }
    }

    pub fn begin(&self) {
        let mut camera = Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            crate::ui::WIDTH as f32,
            crate::ui::HEIGHT as f32,
        ));
        camera.render_target = Some(self.target.clone());
        set_camera(&camera);
    }

    pub fn present(&self, scale: u32) {
        present_target(
            &self.target,
            &self.material,
            [crate::ui::WIDTH, crate::ui::HEIGHT],
            scale,
        );
    }
}

pub(crate) struct WindowControls {
    pub scale: u32,
    fullscreen: bool,
    logical: [u32; 2],
}

impl WindowControls {
    pub fn new(scale: u32, fullscreen: bool, logical: [u32; 2]) -> Self {
        Self {
            scale,
            fullscreen,
            logical,
        }
    }

    /// Trả true khi đổi cửa sổ để caller nhả các phím gameplay đang giữ.
    pub fn update(
        &mut self,
        logical: [u32; 2],
        binds: Option<crate::input::Keybinds>,
        legacy_s: bool,
    ) -> bool {
        let resized = self.logical != logical;
        // Phím đã gán cho gameplay luôn thắng shortcut cửa sổ.
        let available = |key| binds.is_none_or(|binds| binds.action_for(key).is_none());
        let scale_changed = (available(KeyCode::F10) && is_key_pressed(KeyCode::F10))
            || (legacy_s && available(KeyCode::S) && is_key_pressed(KeyCode::S));
        let fullscreen_changed = available(KeyCode::F11) && is_key_pressed(KeyCode::F11);
        self.logical = logical;
        if scale_changed {
            self.scale = if self.scale == 1 { 2 } else { 1 };
        }
        if fullscreen_changed {
            self.fullscreen = !self.fullscreen;
            macroquad::miniquad::window::set_fullscreen(self.fullscreen);
        }
        let too_small = Viewport::fit(
            logical,
            [screen_width() as u32, screen_height() as u32],
            self.scale,
        )
        .is_none();
        if too_small && self.fullscreen {
            self.fullscreen = false;
            macroquad::miniquad::window::set_fullscreen(false);
        }
        if !self.fullscreen && (resized || scale_changed || fullscreen_changed || too_small) {
            macroquad::miniquad::window::set_window_size(
                logical[0] * self.scale,
                logical[1] * self.scale,
            );
        }
        resized || scale_changed || fullscreen_changed || too_small
    }
}

#[cfg(test)]
mod tests {
    use super::Viewport;

    #[test]
    fn viewport_preserves_integer_pixels_and_the_entire_canvas() {
        for (logical, physical, preferred, expected) in [
            (
                [800, 600],
                [800, 600],
                1,
                Viewport {
                    x: 0,
                    y: 0,
                    scale: 1,
                },
            ),
            (
                [800, 600],
                [1600, 1200],
                2,
                Viewport {
                    x: 0,
                    y: 0,
                    scale: 2,
                },
            ),
            (
                [800, 600],
                [1920, 1080],
                2,
                Viewport {
                    x: 560,
                    y: 240,
                    scale: 1,
                },
            ),
            (
                [600, 560],
                [1920, 1080],
                2,
                Viewport {
                    x: 660,
                    y: 260,
                    scale: 1,
                },
            ),
            (
                [800, 600],
                [1921, 1441],
                2,
                Viewport {
                    x: 160,
                    y: 120,
                    scale: 2,
                },
            ),
        ] {
            assert_eq!(Viewport::fit(logical, physical, preferred), Some(expected));
            assert!(expected.x + logical[0] * expected.scale <= physical[0]);
            assert!(expected.y + logical[1] * expected.scale <= physical[1]);
        }
    }

    #[test]
    fn undersized_or_empty_canvas_cannot_silently_crop() {
        assert_eq!(Viewport::fit([800, 600], [799, 600], 2), None);
        assert_eq!(Viewport::fit([800, 600], [1920, 599], 2), None);
        assert_eq!(Viewport::fit([0, 600], [1920, 1080], 2), None);
    }
}
