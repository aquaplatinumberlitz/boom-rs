//! Input map và bộ theo dõi phím cho client V1 — không phụ thuộc cửa sổ.
//!
//! Policy input V1-033 được chốt ở `RULES_V1` **R-080** (DESIGN, không phải confirmed KR):
//! hướng = **last-pressed theo thứ tự sự kiện** trong các phím còn giữ; bỏ autorepeat; release rồi
//! repress giữa hai frame vẫn tạo cạnh bom mới; mất focus xóa toàn bộ phím đang giữ và cạnh chưa
//! tiêu thụ. Adapter nhận event từ backend theo đúng thứ tự, không suy từ trạng thái cuối frame.

use macroquad::miniquad::{EventHandler, KeyMods};
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

    /// Dựng bộ keybind từ danh sách (hành động, phím) — dùng khi nạp `options.toml`.
    ///
    /// Không hoán đổi phím như [`Keybinds::rebind`]: nơi nạp options đã gán phím cho **cả bảng**
    /// và bảo đảm kết quả là ánh xạ 1–1 (mỗi hành động một phím, không hành động nào trùng phím),
    /// nên danh sách này đã là ánh xạ hoàn chỉnh — hành động không có trong danh sách giữ mặc
    /// định. Bảng **hoán đổi/hoán vị** hợp lệ (đúng thứ `--rebind` ghi ra) vẫn được giữ nguyên.
    pub fn from_entries(entries: &[(InputAction, KeyCode)]) -> Self {
        let mut binds = Self::default();
        for &(action, key) in entries {
            binds.set(action, key);
        }
        binds
    }

    pub fn action_for(self, key: KeyCode) -> Option<InputAction> {
        InputAction::ALL
            .into_iter()
            .find(|&action| self.key(action) == key)
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

/// Một sự kiện bàn phím/focus được adapter đưa vào tracker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    FocusLost,
}

/// Adapter nối event handler của macroquad với [`InputTracker`].
///
/// `macroquad::input::utils::repeat_all_miniquad_input` gọi các hàm này theo đúng thứ tự event
/// trong frame. Adapter bỏ `repeat=true` (autorepeat), nên release rồi press lại vẫn sinh hai
/// event khác nhau.
#[derive(Default, Debug)]
pub struct InputAdapter {
    events: Vec<InputEvent>,
}

impl InputAdapter {
    /// Đẩy toàn bộ event đã nhận vào tracker theo thứ tự, rồi xóa hàng đợi.
    pub fn drain_into(&mut self, input: &mut InputTracker) {
        for event in self.events.drain(..) {
            input.apply(event);
        }
    }
}

impl EventHandler for InputAdapter {
    fn update(&mut self) {}

    fn draw(&mut self) {}

    fn key_down_event(&mut self, keycode: KeyCode, _keymods: KeyMods, repeat: bool) {
        if !repeat {
            self.events.push(InputEvent::KeyDown(keycode));
        }
    }

    fn key_up_event(&mut self, keycode: KeyCode, _keymods: KeyMods) {
        self.events.push(InputEvent::KeyUp(keycode));
    }

    fn window_minimized_event(&mut self) {
        self.events.push(InputEvent::FocusLost);
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

    /// Áp một event theo thứ tự backend. Không dựa vào trạng thái cuối frame.
    pub fn apply(&mut self, event: InputEvent) {
        match event {
            InputEvent::KeyDown(key) => {
                if self.held.contains(&key) {
                    return;
                }
                self.held.push(key);
                if key == self.binds.bomb {
                    self.bomb_edge = true;
                }
            }
            InputEvent::KeyUp(key) => self.held.retain(|&held| held != key),
            InputEvent::FocusLost => self.focus_lost(),
        }
    }

    #[cfg(test)]
    pub fn apply_all(&mut self, events: &[InputEvent]) {
        for &event in events {
            self.apply(event);
        }
    }

    /// Mất focus: nhả mọi phím và bỏ cạnh bom chưa tiêu thụ.
    pub fn focus_lost(&mut self) {
        self.held.clear();
        self.bomb_edge = false;
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

    fn key_mods() -> KeyMods {
        KeyMods::default()
    }

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
    fn event_order_wins_across_same_frame_reverse_bindings() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.apply_all(&[
            InputEvent::KeyDown(binds.right),
            InputEvent::KeyDown(binds.up),
        ]);
        assert_eq!(input.direction(), Dir::Up);

        let mut input = InputTracker::new(binds);
        input.apply_all(&[
            InputEvent::KeyDown(binds.up),
            InputEvent::KeyDown(binds.right),
        ]);
        assert_eq!(input.direction(), Dir::Right);
    }

    #[test]
    fn release_falls_back_to_previous_held_direction() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.apply_all(&[
            InputEvent::KeyDown(binds.up),
            InputEvent::KeyDown(binds.right),
            InputEvent::KeyDown(binds.left),
        ]);
        assert_eq!(input.direction(), Dir::Left);
        input.apply(InputEvent::KeyUp(binds.left));
        assert_eq!(input.direction(), Dir::Right);
        input.apply(InputEvent::KeyUp(binds.right));
        assert_eq!(input.direction(), Dir::Up);
        input.apply(InputEvent::KeyUp(binds.up));
        assert_eq!(input.direction(), Dir::None);
    }

    #[test]
    fn focus_loss_releases_everything_and_bomb_edge() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.apply_all(&[
            InputEvent::KeyDown(binds.up),
            InputEvent::KeyDown(binds.bomb),
        ]);
        assert_eq!(input.direction(), Dir::Up);
        assert!(input.take_bomb());
        input.apply(InputEvent::KeyDown(binds.bomb)); // vẫn held: không tạo cạnh mới
        input.apply(InputEvent::FocusLost);
        assert_eq!(input.direction(), Dir::None);
        assert!(input.held().is_empty());
        assert!(!input.take_bomb());
    }

    #[test]
    fn adapter_repress_between_frames_keeps_bomb_edge() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        let mut adapter = InputAdapter::default();
        adapter.key_down_event(binds.bomb, key_mods(), false);
        adapter.drain_into(&mut input);
        assert!(input.take_bomb());

        // Giữa hai frame: key-up rồi key-down trước khi frame sau đọc trạng thái.
        adapter.key_up_event(binds.bomb, key_mods());
        adapter.key_down_event(binds.bomb, key_mods(), false);
        adapter.drain_into(&mut input);
        assert!(input.take_bomb());
    }

    #[test]
    fn adapter_ignores_autorepeat() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        let mut adapter = InputAdapter::default();
        adapter.key_down_event(binds.up, key_mods(), false);
        adapter.key_down_event(binds.up, key_mods(), true);
        assert_eq!(adapter.events, vec![InputEvent::KeyDown(binds.up)]);
        adapter.drain_into(&mut input);
        assert_eq!(input.direction(), Dir::Up);
    }

    #[test]
    fn adapter_focus_event_clears_held_and_pending_bomb() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        let mut adapter = InputAdapter::default();
        adapter.key_down_event(binds.up, key_mods(), false);
        adapter.key_down_event(binds.bomb, key_mods(), false);
        adapter.window_minimized_event();
        adapter.drain_into(&mut input);
        assert_eq!(input.direction(), Dir::None);
        assert!(input.held().is_empty());
        assert!(!input.take_bomb());
    }

    #[test]
    fn bomb_is_an_edge_not_a_held_state() {
        let binds = Keybinds::default();
        let mut input = InputTracker::new(binds);
        input.apply(InputEvent::KeyDown(binds.bomb));
        assert!(input.take_bomb());
        assert!(!input.take_bomb());
        input.apply(InputEvent::KeyDown(binds.bomb)); // duplicate/repeat
        assert!(!input.take_bomb());
        input.apply(InputEvent::KeyUp(binds.bomb));
        input.apply(InputEvent::KeyDown(binds.bomb));
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
}
