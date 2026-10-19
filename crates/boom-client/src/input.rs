//! Input map và bộ theo dõi phím cho client V1 — không phụ thuộc cửa sổ.
//!
//! Quy ước thiết kế V1-033: hướng đang dùng là **phím hướng được nhấn gần nhất trong các phím
//! còn giữ** (last-pressed wins); nhả phím đó thì quay về phím còn giữ gần nhất kế tiếp. Khi mất
//! focus, toàn bộ phím đang giữ bị xóa nên input về neutral. Đây là policy input/UI, không phải
//! giá trị luật trong `Rules`; RULES/SCOPE hiện chưa có claim ưu tiên (V1-011 còn mở), nên policy
//! được ghi thành DESIGN ở đây và phải được chốt khi V1-011 cập nhật.

use macroquad::prelude::KeyCode;

use boom_core::Dir;

/// Hành động input mà người chơi có thể gán phím.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAction {
    Up,
    Down,
    Left,
    Right,
    Bomb,
}

impl InputAction {
    pub const ALL: [Self; 5] = [Self::Up, Self::Down, Self::Left, Self::Right, Self::Bomb];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Bomb => "bomb",
        }
    }
}

/// Phím mặc định: mũi tên + Space (V1-033).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keybinds {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub bomb: KeyCode,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            up: KeyCode::Up,
            down: KeyCode::Down,
            left: KeyCode::Left,
            right: KeyCode::Right,
            bomb: KeyCode::Space,
        }
    }
}

impl Keybinds {
    pub fn key(self, action: InputAction) -> KeyCode {
        match action {
            InputAction::Up => self.up,
            InputAction::Down => self.down,
            InputAction::Left => self.left,
            InputAction::Right => self.right,
            InputAction::Bomb => self.bomb,
        }
    }

    pub fn action_for(self, key: KeyCode) -> Option<InputAction> {
        InputAction::ALL
            .into_iter()
            .find(|&action| self.key(action) == key)
    }

    pub fn keys(self) -> [KeyCode; 5] {
        [self.up, self.down, self.left, self.right, self.bomb]
    }

    /// Đổi phím cho một hành động. Nếu phím đã thuộc hành động khác thì hoán đổi phím cũ của
    /// hành động này sang hành động kia, để không có hai hành động cùng một phím.
    pub fn rebind(&mut self, action: InputAction, key: KeyCode) {
        if let Some(other) = self.action_for(key)
            && other != action
        {
            let old = self.key(action);
            self.set(other, old);
        }
        self.set(action, key);
    }

    fn set(&mut self, action: InputAction, key: KeyCode) {
        match action {
            InputAction::Up => self.up = key,
            InputAction::Down => self.down = key,
            InputAction::Left => self.left = key,
            InputAction::Right => self.right = key,
            InputAction::Bomb => self.bomb = key,
        }
    }
}

/// Trạng thái phím đang giữ và cạnh nhấn bom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputTracker {
    binds: Keybinds,
    /// Các phím đang giữ, theo thứ tự nhấn tăng dần.
    held: Vec<KeyCode>,
    bomb_edge: bool,
}

impl Default for InputTracker {
    fn default() -> Self {
        Self::new(Keybinds::default())
    }
}

impl InputTracker {
    pub fn new(binds: Keybinds) -> Self {
        Self {
            binds,
            held: Vec::new(),
            bomb_edge: false,
        }
    }

    pub fn binds(&self) -> &Keybinds {
        &self.binds
    }

    /// Đổi phím và xóa trạng thái đang giữ để tránh kẹt phím theo binding cũ.
    pub fn rebind(&mut self, action: InputAction, key: KeyCode) {
        self.binds.rebind(action, key);
        self.held.clear();
        self.bomb_edge = false;
    }

    pub fn held(&self) -> &[KeyCode] {
        &self.held
    }

    pub fn key_down(&mut self, key: KeyCode) {
        if self.held.contains(&key) {
            return;
        }
        self.held.push(key);
        if key == self.binds.bomb {
            self.bomb_edge = true;
        }
    }

    pub fn key_up(&mut self, key: KeyCode) {
        self.held.retain(|&held| held != key);
    }

    /// Mất focus: nhả mọi phím và bỏ cạnh bom chưa tiêu thụ.
    pub fn focus_lost(&mut self) {
        self.held.clear();
        self.bomb_edge = false;
    }

    /// Đọc trạng thái phím từ backend (macroquad) mỗi frame.
    ///
    /// `pressed`/`down` là các hàm `is_key_pressed`/`is_key_down`; gọi `focus_lost` không cần thiết
    /// khi backend đã báo phím nhả, nhưng vẫn giữ API cho test và cho backend khác.
    pub fn sync(&mut self, pressed: impl Fn(KeyCode) -> bool, down: impl Fn(KeyCode) -> bool) {
        for key in self.binds.keys() {
            if pressed(key) {
                self.key_down(key);
            }
        }
        for key in self.binds.keys() {
            if !down(key) {
                self.key_up(key);
            }
        }
    }

    /// Hướng hiện tại: phím hướng còn giữ được nhấn gần nhất.
    pub fn direction(&self) -> Dir {
        for &key in self.held.iter().rev() {
            match self.binds.action_for(key) {
                Some(InputAction::Up) => return Dir::Up,
                Some(InputAction::Down) => return Dir::Down,
                Some(InputAction::Left) => return Dir::Left,
                Some(InputAction::Right) => return Dir::Right,
                Some(InputAction::Bomb) | None => {}
            }
        }
        Dir::None
    }

    /// Cạnh nhấn bom; gọi một lần mỗi tick để không lặp khi giữ phím.
    pub fn take_bomb(&mut self) -> bool {
        let pressed = self.bomb_edge;
        self.bomb_edge = false;
        pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_arrows_and_space() {
        let binds = Keybinds::default();
        assert_eq!(binds.up, KeyCode::Up);
        assert_eq!(binds.down, KeyCode::Down);
        assert_eq!(binds.left, KeyCode::Left);
        assert_eq!(binds.right, KeyCode::Right);
        assert_eq!(binds.bomb, KeyCode::Space);
    }

    #[test]
    fn last_pressed_direction_wins_and_release_falls_back() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.key_down(binds.up);
        assert_eq!(input.direction(), Dir::Up);
        input.key_down(binds.right);
        assert_eq!(input.direction(), Dir::Right);
        input.key_down(binds.left);
        assert_eq!(input.direction(), Dir::Left);
        input.key_up(binds.left);
        assert_eq!(input.direction(), Dir::Right);
        input.key_up(binds.right);
        assert_eq!(input.direction(), Dir::Up);
        input.key_up(binds.up);
        assert_eq!(input.direction(), Dir::None);
    }

    #[test]
    fn opposite_keys_use_last_pressed_not_fixed_priority() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.key_down(binds.up);
        input.key_down(binds.down);
        assert_eq!(input.direction(), Dir::Down);
        input.key_up(binds.down);
        assert_eq!(input.direction(), Dir::Up);
    }

    #[test]
    fn focus_loss_releases_everything_and_bomb_edge() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.key_down(binds.up);
        input.key_down(binds.bomb);
        assert_eq!(input.direction(), Dir::Up);
        assert!(input.take_bomb());
        input.key_down(binds.bomb);
        input.focus_lost();
        assert_eq!(input.direction(), Dir::None);
        assert!(input.held().is_empty());
        assert!(!input.take_bomb());
    }

    #[test]
    fn bomb_is_an_edge_not_a_held_state() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.key_down(binds.bomb);
        assert!(input.take_bomb());
        assert!(!input.take_bomb());
        input.key_down(binds.bomb); // repeat while held: không tạo cạnh mới
        assert!(!input.take_bomb());
        input.key_up(binds.bomb);
        input.key_down(binds.bomb);
        assert!(input.take_bomb());
    }

    #[test]
    fn rebind_moves_key_and_swaps_existing_binding() {
        let mut binds = Keybinds::default();
        binds.rebind(InputAction::Up, KeyCode::W);
        assert_eq!(binds.up, KeyCode::W);
        assert_eq!(binds.action_for(KeyCode::Up), None);

        binds.rebind(InputAction::Down, KeyCode::W);
        assert_eq!(binds.down, KeyCode::W);
        assert_eq!(binds.up, KeyCode::Down);
        assert_eq!(binds.action_for(KeyCode::W), Some(InputAction::Down));
        assert_eq!(binds.action_for(KeyCode::Down), Some(InputAction::Up));
    }

    #[test]
    fn sync_releases_when_backend_loses_focus() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.sync(|key| key == binds.right, |key| key == binds.right);
        assert_eq!(input.direction(), Dir::Right);
        // Focus loss: backend báo không còn phím nào down.
        input.sync(|_| false, |_| false);
        assert_eq!(input.direction(), Dir::None);
        assert!(input.held().is_empty());
    }
}
