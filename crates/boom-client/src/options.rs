//! Tùy chọn người dùng của client: `options.toml` (volume/mute/keybind), đọc–ghi an toàn.
//!
//! Nguồn thiết kế: [ADR 0011 §7](../../docs/decisions/0011-identity-persistence-log-v1.md) (tùy chọn
//! client lưu phía client, không lưu ở server) và §11 T6. Đây là **design** của dự án, không phải
//! claim `confirmed` từ client KR/VN, và **không** chứa giá trị luật chơi nào — nên không sinh mã
//! claim `R-xxx` trong [RULES_V1](../../docs/specification/gameplay/RULES_V1.md). Keybind chỉ là
//! ánh xạ hành động → phím; policy thứ tự phím vẫn là `R-080` (không đổi).
//!
//! Định dạng file (schema 1):
//!
//! ```toml
//! schema_version = 1
//! master_volume = 80
//! bgm_volume = 70
//! sfx_volume = 100
//! mute = false
//!
//! [keybinds]
//! up = "Up"
//! down = "Down"
//! left = "Left"
//! right = "Right"
//! bomb = "Space"
//! ```
//!
//! Quy tắc nạp — **không bao giờ crash**:
//!
//! - Chưa có file → dùng mặc định ([`LoadStatus::Missing`]), không phải lỗi.
//! - File hỏng (không phải TOML) hoặc sai `schema_version` → **toàn bộ** dùng mặc định
//!   ([`LoadStatus::Defaults`]) + cảnh báo.
//! - Thiếu trường / sai kiểu / giá trị ngoài miền → **riêng trường đó** dùng mặc định + cảnh báo.
//! - Bảng keybind là **ánh xạ hành động → phím** và phải là ánh xạ 1–1: không hành động nào
//!   được dùng chung một phím. Bảng **hoán đổi/hoán vị** hợp lệ (ví dụ `up = "Down"`,
//!   `down = "Up"` — đúng thứ `--rebind` ghi ra) được giữ nguyên; chỉ khi không gán được
//!   thì hành động đó giữ mặc định kèm cảnh báo (xem [`resolve_keybinds`]).
//!
//! Ghi **atomic**: ghi file tạm rồi đổi tên. Đổi tên là thao tác nguyên tử trên POSIX (`rename(2)`)
//! và dùng `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` trên Windows, nên file cũ không bao giờ bị
//! ghi đè dở dang; khi có lỗi thì file cũ giữ nguyên và file tạm được xoá.

use std::path::{Path, PathBuf};

use macroquad::prelude::KeyCode;
use serde::Serialize;

use crate::input::{InputAction, Keybinds};

/// Phiên bản định dạng `options.toml` hiện tại.
pub const OPTIONS_SCHEMA: u32 = 1;

/// Biến môi trường ghi đè đường dẫn file options (chạy thử tay; test dùng đường dẫn tường minh).
pub const OPTIONS_PATH_ENV: &str = "BOOM_CLIENT_OPTIONS_PATH";

/// Tên file options.
pub const OPTIONS_FILE_NAME: &str = "options.toml";

/// Thư mục con trong thư mục cấu hình của OS.
const APP_DIR: &str = "boom-rs";

/// Miền giá trị volume: 0–100 (ADR 0011 §7).
pub const VOLUME_MIN: u8 = 0;
pub const VOLUME_MAX: u8 = 100;

/// Tùy chọn người dùng đã nạp và chuẩn hoá.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientOptions {
    /// Volume chính (0–100).
    pub master_volume: u8,
    /// Volume nhạc nền (0–100).
    pub bgm_volume: u8,
    /// Volume hiệu ứng (0–100).
    pub sfx_volume: u8,
    /// Tắt toàn bộ âm thanh.
    pub mute: bool,
    /// Ánh xạ hành động → phím.
    pub keybinds: Keybinds,
}

impl Default for ClientOptions {
    /// Mặc định: ba kênh ở mức tối đa, không tắt tiếng, keybind mặc định của V1-033
    /// (mũi tên + `Space`).
    fn default() -> Self {
        Self {
            master_volume: VOLUME_MAX,
            bgm_volume: VOLUME_MAX,
            sfx_volume: VOLUME_MAX,
            mute: false,
            keybinds: Keybinds::default(),
        }
    }
}

impl ClientOptions {
    /// Volume chính sau khi áp `mute`.
    pub fn effective_master(&self) -> u8 {
        if self.mute { 0 } else { self.master_volume }
    }

    /// Volume nhạc nền sau khi áp `mute`.
    pub fn effective_bgm(&self) -> u8 {
        if self.mute { 0 } else { self.bgm_volume }
    }

    /// Volume hiệu ứng sau khi áp `mute`.
    pub fn effective_sfx(&self) -> u8 {
        if self.mute { 0 } else { self.sfx_volume }
    }
}

/// Kết quả nạp options: giá trị đã dùng + trạng thái + cảnh báo từng trường.
#[derive(Clone, Debug)]
pub struct LoadedOptions {
    /// Giá trị dùng được (luôn hợp lệ).
    pub options: ClientOptions,
    /// Trạng thái nạp.
    pub status: LoadStatus,
    /// Cảnh báo từng trường (rỗng nếu file sạch).
    pub warnings: Vec<String>,
}

/// Trạng thái nạp options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadStatus {
    /// Chưa có file: dùng mặc định, chưa có gì hỏng.
    Missing,
    /// Đọc được file (có thể vẫn có cảnh báo cho riêng trường).
    Loaded,
    /// File hỏng hoặc sai `schema_version`: dùng mặc định cho **toàn bộ** file.
    Defaults,
}

/// Lỗi khi ghi options.
#[derive(Debug)]
pub enum OptionsError {
    /// Lỗi hệ thống file.
    Io(std::io::Error),
    /// Không tìm được thư mục cấu hình của OS (thiếu `HOME`/`APPDATA`).
    NoConfigDir,
    /// Không dựng được nội dung TOML để ghi.
    Serialize(String),
}

impl std::fmt::Display for OptionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "lỗi file: {error}"),
            Self::NoConfigDir => write!(f, "không tìm được thư mục cấu hình của hệ điều hành"),
            Self::Serialize(message) => write!(f, "không dựng được file options: {message}"),
        }
    }
}

impl From<std::io::Error> for OptionsError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::error::Error for OptionsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Đường dẫn file options: ưu tiên đường dẫn chỉ định, rồi biến môi trường, rồi quy ước OS.
pub fn resolve_path(explicit: Option<&Path>) -> Result<PathBuf, OptionsError> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = env_override() {
        return Ok(path);
    }
    default_path()
}

/// Đường dẫn file options theo quy ước cấu hình của từng OS.
pub fn default_path() -> Result<PathBuf, OptionsError> {
    let dir = config_dir().ok_or(OptionsError::NoConfigDir)?;
    Ok(dir.join(APP_DIR).join(OPTIONS_FILE_NAME))
}

fn env_override() -> Option<PathBuf> {
    let raw = std::env::var_os(OPTIONS_PATH_ENV)?;
    if raw.is_empty() {
        return None;
    }
    Some(PathBuf::from(raw))
}

#[cfg(target_os = "windows")]
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
        })
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn config_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(xdg));
    }
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(".config"))
}

/// Nạp options từ file.
///
/// Không bao giờ trả lỗi: mọi hợp đồng hỏng đều rơi về mặc định kèm cảnh báo, để client khởi
/// động được với file options hỏng.
pub fn load(path: &Path) -> LoadedOptions {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return LoadedOptions {
                options: ClientOptions::default(),
                status: LoadStatus::Missing,
                warnings: Vec::new(),
            };
        }
        Err(error) => {
            return LoadedOptions {
                options: ClientOptions::default(),
                status: LoadStatus::Defaults,
                warnings: vec![format!("không đọc được {}: {error}", path.display())],
            };
        }
    };
    let table = match toml::from_str::<toml::Table>(&text) {
        Ok(table) => table,
        Err(error) => {
            return LoadedOptions {
                options: ClientOptions::default(),
                status: LoadStatus::Defaults,
                warnings: vec![format!("file options hỏng: {error}")],
            };
        }
    };
    from_table(&table)
}

/// Ghi options **atomic**: ghi file tạm cùng thư mục rồi đổi tên.
///
/// Khi lỗi, file cũ giữ nguyên và file tạm bị xoá; không để lại file rác.
pub fn save(options: &ClientOptions, path: &Path) -> Result<(), OptionsError> {
    let text = toml::to_string_pretty(&OptionsFile::from(options))
        .map_err(|error| OptionsError::Serialize(error.to_string()))?;
    let dir = parent_dir(path);
    std::fs::create_dir_all(&dir)?;
    let temp = temp_path(path);
    if let Err(error) = std::fs::write(&temp, text.as_bytes()) {
        let _ = std::fs::remove_file(&temp);
        return Err(OptionsError::Io(error));
    }
    if let Err(error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(OptionsError::Io(error));
    }
    Ok(())
}

fn parent_dir(path: &Path) -> PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// File tạm nằm **cùng thư mục** với file đích để `rename` là thao tác nguyên tử (cùng hệ thống
/// file) và không đụng tới file cũ cho tới lúc đổi tên thành công.
fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| OPTIONS_FILE_NAME.to_owned());
    parent_dir(path).join(format!(".{name}.tmp{}", std::process::id()))
}

fn from_table(table: &toml::Table) -> LoadedOptions {
    let defaults = ClientOptions::default();
    let mut warnings: Vec<String> = Vec::new();

    match table.get("schema_version") {
        None => {
            warnings.push("thiếu `schema_version`: dùng mặc định cho toàn bộ file".to_owned());
            return LoadedOptions {
                options: defaults,
                status: LoadStatus::Defaults,
                warnings,
            };
        }
        Some(value) => {
            let Some(version) = value.as_integer() else {
                warnings.push(
                    "`schema_version` không phải số nguyên: dùng mặc định cho toàn bộ file"
                        .to_owned(),
                );
                return LoadedOptions {
                    options: defaults,
                    status: LoadStatus::Defaults,
                    warnings,
                };
            };
            if version != i64::from(OPTIONS_SCHEMA) {
                warnings.push(format!(
                    "`schema_version` = {version} khác {OPTIONS_SCHEMA}: dùng mặc định cho toàn bộ file"
                ));
                return LoadedOptions {
                    options: defaults,
                    status: LoadStatus::Defaults,
                    warnings,
                };
            }
        }
    }

    let mut options = defaults;
    options.master_volume = read_volume(
        table,
        "master_volume",
        defaults.master_volume,
        &mut warnings,
    );
    options.bgm_volume = read_volume(table, "bgm_volume", defaults.bgm_volume, &mut warnings);
    options.sfx_volume = read_volume(table, "sfx_volume", defaults.sfx_volume, &mut warnings);
    options.mute = read_bool(table, "mute", defaults.mute, &mut warnings);
    options.keybinds = read_keybinds(table.get("keybinds"), &mut warnings);

    for key in table.keys() {
        if !matches!(
            key.as_str(),
            "schema_version" | "master_volume" | "bgm_volume" | "sfx_volume" | "mute" | "keybinds"
        ) {
            warnings.push(format!("khóa lạ `{key}` bị bỏ qua"));
        }
    }

    LoadedOptions {
        options,
        status: LoadStatus::Loaded,
        warnings,
    }
}

fn read_volume(table: &toml::Table, key: &str, fallback: u8, warnings: &mut Vec<String>) -> u8 {
    let Some(value) = table.get(key) else {
        warnings.push(format!("thiếu `{key}`: dùng mặc định {fallback}"));
        return fallback;
    };
    let Some(raw) = value.as_integer() else {
        warnings.push(format!(
            "`{key}` không phải số nguyên: dùng mặc định {fallback}"
        ));
        return fallback;
    };
    if !(i64::from(VOLUME_MIN)..=i64::from(VOLUME_MAX)).contains(&raw) {
        warnings.push(format!(
            "`{key}` = {raw} ngoài miền {VOLUME_MIN}–{VOLUME_MAX}: dùng mặc định {fallback}"
        ));
        return fallback;
    }
    u8::try_from(raw).unwrap_or(fallback)
}

fn read_bool(table: &toml::Table, key: &str, fallback: bool, warnings: &mut Vec<String>) -> bool {
    let Some(value) = table.get(key) else {
        warnings.push(format!("thiếu `{key}`: dùng mặc định {fallback}"));
        return fallback;
    };
    match value.as_bool() {
        Some(flag) => flag,
        None => {
            warnings.push(format!("`{key}` không phải bool: dùng mặc định {fallback}"));
            fallback
        }
    }
}

/// Nạp bảng keybind; bảng phải là ánh xạ 1–1 (không hai hành động nào dùng chung phím).
///
/// Giá trị trong file được ưu tiên; hành động không gán được thì giữ mặc định kèm cảnh báo.
/// Bảng **hoán đổi/hoán vị** hợp lệ (đúng thứ `--rebind` ghi ra) được giữ nguyên — xem
/// [`resolve_keybinds`].
fn read_keybinds(value: Option<&toml::Value>, warnings: &mut Vec<String>) -> Keybinds {
    let defaults = Keybinds::default();
    let Some(value) = value else {
        warnings.push("thiếu bảng `keybinds`: dùng mặc định".to_owned());
        return defaults;
    };
    let Some(table) = value.as_table() else {
        warnings.push("`keybinds` không phải bảng: dùng mặc định".to_owned());
        return defaults;
    };
    for key in table.keys() {
        if !InputAction::ALL.iter().any(|action| action.label() == key) {
            warnings.push(format!("keybind lạ `{key}` bị bỏ qua"));
        }
    }
    // Giá trị trong file theo thứ tự hành động; `None` = thiếu hoặc tên phím không hợp lệ.
    let mut wanted: Vec<(InputAction, Option<KeyCode>)> = Vec::new();
    for action in InputAction::ALL {
        let file_key = match table.get(action.label()) {
            None => {
                warnings.push(format!("thiếu keybind `{}`: giữ mặc định", action.label()));
                None
            }
            Some(value) => match value.as_str() {
                None => {
                    warnings.push(format!(
                        "keybind `{}` không phải chuỗi: giữ mặc định",
                        action.label()
                    ));
                    None
                }
                Some(raw) => match parse_key_name(raw) {
                    Some(key) => Some(key),
                    None => {
                        warnings.push(format!(
                            "tên phím lạ `{raw}` cho `{}`: giữ mặc định",
                            action.label()
                        ));
                        None
                    }
                },
            },
        };
        wanted.push((action, file_key));
    }
    let Some(assignment) = resolve_keybinds(&wanted, &defaults) else {
        warnings.push("bảng `keybinds` không gán được phím: dùng mặc định".to_owned());
        return defaults;
    };
    // Hành động bị từ chối: giá trị trong file trùng phím đã dùng nên giữ mặc định.
    for &(action, file_key) in &wanted {
        let Some(file_key) = file_key else { continue };
        let assigned = assignment
            .iter()
            .find(|(assigned_action, _)| *assigned_action == action)
            .map(|(_, key)| *key);
        if assigned != Some(file_key) {
            warnings.push(format!(
                "keybind `{}` = `{}` bị từ chối (trùng phím): giữ mặc định",
                action.label(),
                key_name(file_key).unwrap_or("?")
            ));
        }
    }
    Keybinds::from_entries(&assignment)
}

/// Gán phím cho từng hành động sao cho **không hành động nào trùng phím**.
///
/// Mỗi hành động có thứ tự ưu tiên: giá trị trong file trước, mặc định sau. Bảng keybind là
/// *ánh xạ* nên phải xét cả bảng: một bảng hoán đổi/hoán vị hợp lệ (ví dụ `up = "Down"`,
/// `down = "Up"` — đúng thứ `--rebind` ghi ra) phải được giữ nguyên, không được loại từng
/// dòng độc lập.
///
/// Vì vậy dùng quay lui theo thứ tự hành động: thử giá trị trong file trước, rồi mặc định.
/// Lời giải luôn tồn tại (mọi hành động dùng mặc định) nên hàm không bao giờ thất bại; khi
/// không gán được thì hành động đó giữ mặc định.
fn resolve_keybinds(
    wanted: &[(InputAction, Option<KeyCode>)],
    defaults: &Keybinds,
) -> Option<Vec<(InputAction, KeyCode)>> {
    let mut assignment: Vec<(InputAction, KeyCode)> = Vec::new();
    let mut used: Vec<KeyCode> = Vec::new();
    if assign_keys(wanted, defaults, 0, &mut assignment, &mut used) {
        Some(assignment)
    } else {
        None
    }
}

fn assign_keys(
    wanted: &[(InputAction, Option<KeyCode>)],
    defaults: &Keybinds,
    index: usize,
    assignment: &mut Vec<(InputAction, KeyCode)>,
    used: &mut Vec<KeyCode>,
) -> bool {
    let Some(&(action, file_key)) = wanted.get(index) else {
        return true;
    };
    let mut candidates: Vec<KeyCode> = Vec::new();
    if let Some(key) = file_key {
        candidates.push(key);
    }
    let default = defaults.key(action);
    if !candidates.contains(&default) {
        candidates.push(default);
    }
    for key in candidates {
        if used.contains(&key) {
            continue;
        }
        assignment.push((action, key));
        used.push(key);
        if assign_keys(wanted, defaults, index + 1, assignment, used) {
            return true;
        }
        assignment.pop();
        used.pop();
    }
    false
}

/// Phím có tên chuẩn trong `options.toml`.
const NAMED_KEYS: [(KeyCode, &str); 24] = [
    (KeyCode::Space, "Space"),
    (KeyCode::Up, "Up"),
    (KeyCode::Down, "Down"),
    (KeyCode::Left, "Left"),
    (KeyCode::Right, "Right"),
    (KeyCode::Enter, "Enter"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::Escape, "Escape"),
    (KeyCode::Backspace, "Backspace"),
    (KeyCode::LeftShift, "LeftShift"),
    (KeyCode::LeftControl, "LeftControl"),
    (KeyCode::LeftAlt, "LeftAlt"),
    (KeyCode::F1, "F1"),
    (KeyCode::F2, "F2"),
    (KeyCode::F3, "F3"),
    (KeyCode::F4, "F4"),
    (KeyCode::F5, "F5"),
    (KeyCode::F6, "F6"),
    (KeyCode::F7, "F7"),
    (KeyCode::F8, "F8"),
    (KeyCode::F9, "F9"),
    (KeyCode::F10, "F10"),
    (KeyCode::F11, "F11"),
    (KeyCode::F12, "F12"),
];

/// Chữ cái, theo thứ tự A–Z.
const LETTERS: [KeyCode; 26] = [
    KeyCode::A,
    KeyCode::B,
    KeyCode::C,
    KeyCode::D,
    KeyCode::E,
    KeyCode::F,
    KeyCode::G,
    KeyCode::H,
    KeyCode::I,
    KeyCode::J,
    KeyCode::K,
    KeyCode::L,
    KeyCode::M,
    KeyCode::N,
    KeyCode::O,
    KeyCode::P,
    KeyCode::Q,
    KeyCode::R,
    KeyCode::S,
    KeyCode::T,
    KeyCode::U,
    KeyCode::V,
    KeyCode::W,
    KeyCode::X,
    KeyCode::Y,
    KeyCode::Z,
];

const LETTER_NAMES: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

/// Phím số trên hàng trên cùng.
const DIGITS: [KeyCode; 10] = [
    KeyCode::Key0,
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
];

const DIGIT_NAMES: [&str; 10] = [
    "Key0", "Key1", "Key2", "Key3", "Key4", "Key5", "Key6", "Key7", "Key8", "Key9",
];

/// Tên chuẩn của phím trong `options.toml`, nếu phím nằm trong tập hỗ trợ.
pub fn key_name(key: KeyCode) -> Option<&'static str> {
    if let Some((_, name)) = NAMED_KEYS.iter().find(|(code, _)| *code == key) {
        return Some(name);
    }
    if let Some(index) = LETTERS.iter().position(|&code| code == key) {
        return LETTER_NAMES.get(index).copied();
    }
    if let Some(index) = DIGITS.iter().position(|&code| code == key) {
        return DIGIT_NAMES.get(index).copied();
    }
    None
}

/// Tên dùng khi ghi file: tên chuẩn nếu có, nếu không thì tên debug của `KeyCode`.
///
/// Phím ngoài tập hỗ trợ vẫn ghi được nhưng khi đọc lại sẽ bị từ chối (tên không nằm trong bảng
/// đọc) và hành động đó giữ mặc định — có cảnh báo.
fn canonical_key_name(key: KeyCode) -> String {
    key_name(key).map_or_else(|| format!("{key:?}"), str::to_owned)
}

/// Đọc tên phím trong `options.toml` (không phân biệt hoa thường, có alias thân thiện).
pub fn parse_key_name(value: &str) -> Option<KeyCode> {
    let lower = value.trim().to_ascii_lowercase();
    if let Some((code, _)) = NAMED_KEYS
        .iter()
        .find(|(_, name)| name.eq_ignore_ascii_case(&lower))
    {
        return Some(*code);
    }
    let alias = match lower.as_str() {
        "arrowup" => Some(KeyCode::Up),
        "arrowdown" => Some(KeyCode::Down),
        "arrowleft" => Some(KeyCode::Left),
        "arrowright" => Some(KeyCode::Right),
        "spacebar" => Some(KeyCode::Space),
        "return" => Some(KeyCode::Enter),
        "esc" => Some(KeyCode::Escape),
        "shift" | "leftshift" => Some(KeyCode::LeftShift),
        "ctrl" | "control" | "leftcontrol" => Some(KeyCode::LeftControl),
        "alt" | "leftalt" => Some(KeyCode::LeftAlt),
        _ => None,
    };
    if alias.is_some() {
        return alias;
    }
    // `Key0`…`Key9` viết tắt có tiền tố "key"; các tên khác không có tiền tố này.
    let body = lower.strip_prefix("key").unwrap_or(lower.as_str());
    let mut chars = body.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    match first {
        'a'..='z' => ascii_offset(first, 'a')
            .and_then(|index| LETTERS.get(index))
            .copied(),
        '0'..='9' => ascii_offset(first, '0')
            .and_then(|index| DIGITS.get(index))
            .copied(),
        _ => None,
    }
}

/// Chỉ số của ký tự ASCII so với ký tự đầu hàng (không dùng cast để tránh mất dấu).
fn ascii_offset(value: char, base: char) -> Option<usize> {
    let value = u32::from(value);
    let base = u32::from(base);
    if value < base || value - base > 25 {
        return None;
    }
    usize::try_from(value - base).ok()
}

/// Dạng ghi ra file.
#[derive(Serialize)]
struct OptionsFile {
    schema_version: u32,
    master_volume: u8,
    bgm_volume: u8,
    sfx_volume: u8,
    mute: bool,
    keybinds: KeybindsFile,
}

#[derive(Serialize)]
struct KeybindsFile {
    up: String,
    down: String,
    left: String,
    right: String,
    bomb: String,
}

impl From<&ClientOptions> for OptionsFile {
    fn from(options: &ClientOptions) -> Self {
        Self {
            schema_version: OPTIONS_SCHEMA,
            master_volume: options.master_volume,
            bgm_volume: options.bgm_volume,
            sfx_volume: options.sfx_volume,
            mute: options.mute,
            keybinds: KeybindsFile {
                up: canonical_key_name(options.keybinds.up),
                down: canonical_key_name(options.keybinds.down),
                left: canonical_key_name(options.keybinds.left),
                right: canonical_key_name(options.keybinds.right),
                bomb: canonical_key_name(options.keybinds.bomb),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// Thư mục tạm tự xoá khi test kết thúc (không thêm dependency `tempfile`).
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let unique = format!(
                "boom-client-options-{tag}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("tạo thư mục tạm");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("tạo thư mục cha");
        }
        std::fs::write(path, contents).expect("ghi file test");
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("đọc thư mục")
            .map(|entry| {
                entry
                    .expect("mục")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    fn custom_options() -> ClientOptions {
        ClientOptions {
            master_volume: 40,
            bgm_volume: 30,
            sfx_volume: 20,
            mute: true,
            keybinds: Keybinds {
                up: KeyCode::W,
                bomb: KeyCode::J,
                ..Keybinds::default()
            },
        }
    }

    #[test]
    fn defaults_are_full_volume_and_arrow_keys() {
        let options = ClientOptions::default();
        assert_eq!(options.master_volume, VOLUME_MAX);
        assert_eq!(options.bgm_volume, VOLUME_MAX);
        assert_eq!(options.sfx_volume, VOLUME_MAX);
        assert!(!options.mute);
        assert_eq!(options.keybinds, Keybinds::default());
    }

    #[test]
    fn mute_silences_every_channel() {
        let options = ClientOptions {
            mute: true,
            ..ClientOptions::default()
        };
        assert_eq!(options.effective_master(), 0);
        assert_eq!(options.effective_bgm(), 0);
        assert_eq!(options.effective_sfx(), 0);
        let unmuted = ClientOptions::default();
        assert_eq!(unmuted.effective_master(), VOLUME_MAX);
    }

    #[test]
    fn round_trip_write_then_read_keeps_values() {
        let dir = TempDir::new("round-trip");
        let path = dir.file(OPTIONS_FILE_NAME);
        let options = custom_options();
        save(&options, &path).expect("ghi options");
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Loaded);
        assert!(
            loaded.warnings.is_empty(),
            "cảnh báo: {:#?}",
            loaded.warnings
        );
        assert_eq!(loaded.options, options);
    }

    #[test]
    fn round_trip_creates_missing_parent_directory() {
        let dir = TempDir::new("parent");
        let path = dir.file("nested/deeper/options.toml");
        save(&ClientOptions::default(), &path).expect("ghi options");
        assert!(path.is_file());
    }

    #[test]
    fn restart_keeps_volume_and_keybind() {
        // "Restart client" = nạp lại từ đĩa bằng một lần đọc mới, không dùng state cũ.
        let dir = TempDir::new("restart");
        let path = dir.file(OPTIONS_FILE_NAME);
        let options = custom_options();
        save(&options, &path).expect("ghi options");

        let first = load(&path);
        let second = load(&path);
        assert_eq!(first.options, second.options);
        assert_eq!(second.options.master_volume, 40);
        assert_eq!(second.options.bgm_volume, 30);
        assert_eq!(second.options.sfx_volume, 20);
        assert!(second.options.mute);
        assert_eq!(second.options.keybinds.up, KeyCode::W);
        assert_eq!(second.options.keybinds.bomb, KeyCode::J);
        assert_eq!(second.options.keybinds.down, KeyCode::Down);
    }

    #[test]
    fn missing_file_uses_defaults_without_warning() {
        let dir = TempDir::new("missing");
        let loaded = load(&dir.file(OPTIONS_FILE_NAME));
        assert_eq!(loaded.status, LoadStatus::Missing);
        assert_eq!(loaded.options, ClientOptions::default());
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = TempDir::new("corrupt");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(&path, "schema_version = 1\nmaster_volume = \n[[[");
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Defaults);
        assert_eq!(loaded.options, ClientOptions::default());
        assert!(!loaded.warnings.is_empty());
    }

    #[test]
    fn wrong_schema_version_falls_back_to_defaults() {
        let dir = TempDir::new("schema");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(&path, "schema_version = 99\nmaster_volume = 10\n");
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Defaults);
        assert_eq!(loaded.options, ClientOptions::default());
        assert!(!loaded.warnings.is_empty());
    }

    #[test]
    fn missing_schema_version_falls_back_to_defaults() {
        let dir = TempDir::new("no-schema");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(&path, "master_volume = 10\n");
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Defaults);
        assert_eq!(loaded.options, ClientOptions::default());
    }

    #[test]
    fn out_of_range_volume_falls_back_per_field() {
        let dir = TempDir::new("range");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 150\nbgm_volume = 30\nsfx_volume = -1\nmute = false\n",
        );
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Loaded);
        assert_eq!(loaded.options.master_volume, VOLUME_MAX);
        assert_eq!(loaded.options.bgm_volume, 30);
        assert_eq!(loaded.options.sfx_volume, VOLUME_MAX);
        assert!(
            loaded.warnings.iter().any(|w| w.contains("master_volume")),
            "cảnh báo: {:#?}",
            loaded.warnings
        );
        assert!(
            loaded.warnings.iter().any(|w| w.contains("sfx_volume")),
            "cảnh báo: {:#?}",
            loaded.warnings
        );
    }

    #[test]
    fn missing_field_falls_back_per_field_and_warns() {
        let dir = TempDir::new("field");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(&path, "schema_version = 1\nmaster_volume = 10\n");
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Loaded);
        assert_eq!(loaded.options.master_volume, 10);
        assert_eq!(loaded.options.bgm_volume, VOLUME_MAX);
        assert!(!loaded.warnings.is_empty());
    }

    #[test]
    fn wrong_type_falls_back_per_field_and_warns() {
        let dir = TempDir::new("type");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = \"toi\"\nmute = \"yes\"\n",
        );
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Loaded);
        assert_eq!(loaded.options.master_volume, VOLUME_MAX);
        assert!(!loaded.options.mute);
        assert!(!loaded.warnings.is_empty());
    }

    #[test]
    fn duplicate_keybind_is_rejected_and_keeps_default() {
        let dir = TempDir::new("dup");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"W\"\ndown = \"W\"\nleft = \"A\"\nright = \"D\"\nbomb = \"Space\"\n",
        );
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Loaded);
        assert_eq!(loaded.options.keybinds.up, KeyCode::W);
        // `down` trùng phím với `up` nên bị từ chối, giữ mặc định.
        assert_eq!(loaded.options.keybinds.down, KeyCode::Down);
        assert!(loaded.warnings.iter().any(|w| w.contains("trùng phím")));
    }

    #[test]
    fn keybind_cannot_take_another_actions_default_key() {
        let dir = TempDir::new("reserve");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"Space\"\ndown = \"Down\"\nleft = \"Left\"\nright = \"Right\"\nbomb = \"Space\"\n",
        );
        let loaded = load(&path);
        // `bomb` giữ mặc định `Space` nên `up` không lấy được phím đó và giữ mặc định.
        assert_eq!(loaded.options.keybinds.up, KeyCode::Up);
        assert_eq!(loaded.options.keybinds.bomb, KeyCode::Space);
        assert!(loaded.warnings.iter().any(|w| w.contains("trùng phím")));
    }

    #[test]
    fn keybind_may_take_another_actions_default_when_that_action_moves() {
        let dir = TempDir::new("move");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"Space\"\ndown = \"Down\"\nleft = \"Left\"\nright = \"Right\"\nbomb = \"J\"\n",
        );
        let loaded = load(&path);
        // `bomb` đã đi khỏi `Space` nên `up` được lấy phím đó.
        assert_eq!(loaded.options.keybinds.up, KeyCode::Space);
        assert_eq!(loaded.options.keybinds.bomb, KeyCode::J);
    }

    #[test]
    fn swapped_keybinds_are_preserved() {
        // Regression review #69: `--rebind up=Down` ghi ra up=Down/down=Up; bảng hoán đổi hợp lệ
        // phải được giữ nguyên khi đọc lại, nếu không round-trip/restart mất keybind.
        let dir = TempDir::new("swap");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"Down\"\ndown = \"Up\"\nleft = \"Left\"\nright = \"Right\"\nbomb = \"Space\"\n",
        );
        let loaded = load(&path);
        assert_eq!(loaded.options.keybinds.up, KeyCode::Down);
        assert_eq!(loaded.options.keybinds.down, KeyCode::Up);
        assert_eq!(loaded.options.keybinds.left, KeyCode::Left);
        assert_eq!(loaded.options.keybinds.right, KeyCode::Right);
        assert_eq!(loaded.options.keybinds.bomb, KeyCode::Space);
        assert!(
            !loaded.warnings.iter().any(|w| w.contains("trùng phím")),
            "cảnh báo: {:#?}",
            loaded.warnings
        );
    }

    #[test]
    fn keybind_cycle_is_preserved() {
        let dir = TempDir::new("cycle");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"Down\"\ndown = \"Left\"\nleft = \"Up\"\nright = \"Right\"\nbomb = \"Space\"\n",
        );
        let loaded = load(&path);
        assert_eq!(loaded.options.keybinds.up, KeyCode::Down);
        assert_eq!(loaded.options.keybinds.down, KeyCode::Left);
        assert_eq!(loaded.options.keybinds.left, KeyCode::Up);
    }

    #[test]
    fn swapped_keybinds_survive_write_then_read() {
        // Regression review #69: ghi rồi đọc lại phải giữ nguyên bảng hoán đổi.
        let dir = TempDir::new("swap-round-trip");
        let path = dir.file(OPTIONS_FILE_NAME);
        let options = ClientOptions {
            keybinds: Keybinds {
                up: KeyCode::Down,
                down: KeyCode::Up,
                ..Keybinds::default()
            },
            ..ClientOptions::default()
        };
        save(&options, &path).expect("ghi options");
        let loaded = load(&path);
        assert_eq!(loaded.status, LoadStatus::Loaded);
        assert!(
            loaded.warnings.is_empty(),
            "cảnh báo: {:#?}",
            loaded.warnings
        );
        assert_eq!(loaded.options.keybinds, options.keybinds);

        // Ghi lần hai (giống hành vi client khi lưu lại) không được đổi bảng keybind.
        save(&loaded.options, &path).expect("ghi lại options");
        let again = load(&path);
        assert_eq!(again.options.keybinds, options.keybinds);
    }

    #[test]
    fn duplicate_keybind_on_swapped_pair_is_rejected() {
        // `up` và `down` cùng nhận `Down`: phải có một hành động nhường, hành động đó giữ mặc định.
        let dir = TempDir::new("swap-dup");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"Down\"\ndown = \"Down\"\nleft = \"Left\"\nright = \"Right\"\nbomb = \"Space\"\n",
        );
        let loaded = load(&path);
        // `down` giữ `Down` nên `up` không lấy được phím đó và giữ mặc định `Up`.
        assert_eq!(loaded.options.keybinds.up, KeyCode::Up);
        assert_eq!(loaded.options.keybinds.down, KeyCode::Down);
        assert!(
            loaded.warnings.iter().any(|w| w.contains("trùng phím")),
            "cảnh báo: {:#?}",
            loaded.warnings
        );
    }

    #[test]
    fn unknown_keybind_action_is_ignored_with_warning() {
        let dir = TempDir::new("unknown-action");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"W\"\ndown = \"Down\"\nleft = \"Left\"\nright = \"Right\"\nbomb = \"Space\"\njump = \"X\"\n",
        );
        let loaded = load(&path);
        assert_eq!(loaded.options.keybinds.up, KeyCode::W);
        assert!(
            loaded.warnings.iter().any(|w| w.contains("jump")),
            "cảnh báo: {:#?}",
            loaded.warnings
        );
    }

    #[test]
    fn unknown_key_name_is_rejected_and_keeps_default() {
        let dir = TempDir::new("bad-key");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(
            &path,
            "schema_version = 1\nmaster_volume = 100\nbgm_volume = 100\nsfx_volume = 100\nmute = false\n\
             \n[keybinds]\nup = \"PhímHỏng\"\ndown = \"Down\"\nleft = \"Left\"\nright = \"Right\"\nbomb = \"Space\"\n",
        );
        let loaded = load(&path);
        assert_eq!(loaded.options.keybinds.up, KeyCode::Up);
        assert!(!loaded.warnings.is_empty());
    }

    #[test]
    fn atomic_write_leaves_no_temp_file() {
        let dir = TempDir::new("atomic");
        let path = dir.file(OPTIONS_FILE_NAME);
        save(&ClientOptions::default(), &path).expect("ghi options");
        assert_eq!(entries(dir.path()), vec![OPTIONS_FILE_NAME.to_owned()]);
    }

    #[test]
    fn atomic_write_replaces_old_file_and_leaves_no_temp() {
        let dir = TempDir::new("replace");
        let path = dir.file(OPTIONS_FILE_NAME);
        write_file(&path, "schema_version = 1\nmaster_volume = 1\n");
        save(&custom_options(), &path).expect("ghi options");
        assert_eq!(entries(dir.path()), vec![OPTIONS_FILE_NAME.to_owned()]);
        let loaded = load(&path);
        assert_eq!(loaded.options.master_volume, 40);
    }

    #[test]
    fn failed_write_keeps_old_file_and_leaves_no_temp() {
        // Đích là một thư mục có sẵn: `rename` thất bại sau khi đã tạo file tạm.
        let dir = TempDir::new("fail-rename");
        let target = dir.file(OPTIONS_FILE_NAME);
        std::fs::create_dir_all(&target).expect("tạo thư mục đích");
        write_file(&target.join("keep.txt"), "giữ nguyên");

        let error = save(&custom_options(), &target).expect_err("ghi vào thư mục phải lỗi");
        assert!(matches!(error, OptionsError::Io(_)), "lỗi: {error:?}");
        // File tạm đã bị xoá, không để lại file rác.
        assert_eq!(entries(dir.path()), vec![OPTIONS_FILE_NAME.to_owned()]);
        assert_eq!(
            std::fs::read_to_string(target.join("keep.txt")).expect("đọc file cũ"),
            "giữ nguyên"
        );
    }

    #[test]
    fn failed_write_keeps_existing_file_content() {
        // Thư mục cha là một file: ghi lỗi trước khi chạm vào file options cũ.
        let dir = TempDir::new("fail-parent");
        let blocker = dir.file("blocker");
        write_file(&blocker, "nội dung cũ");
        let target = blocker.join(OPTIONS_FILE_NAME);

        let error = save(&custom_options(), &target).expect_err("ghi phải lỗi");
        assert!(matches!(error, OptionsError::Io(_)), "lỗi: {error:?}");
        assert_eq!(
            std::fs::read_to_string(&blocker).expect("đọc file cũ"),
            "nội dung cũ"
        );
    }

    #[test]
    fn key_names_round_trip_through_parse() {
        for (key, name) in NAMED_KEYS {
            assert_eq!(parse_key_name(name), Some(key), "tên {name}");
        }
        for (index, key) in LETTERS.iter().enumerate() {
            let name = LETTER_NAMES[index];
            assert_eq!(parse_key_name(name), Some(*key), "tên {name}");
        }
        for (index, key) in DIGITS.iter().enumerate() {
            let name = DIGIT_NAMES[index];
            assert_eq!(parse_key_name(name), Some(*key), "tên {name}");
        }
    }

    #[test]
    fn key_name_accepts_aliases() {
        assert_eq!(parse_key_name("arrowleft"), Some(KeyCode::Left));
        assert_eq!(parse_key_name("SPACE"), Some(KeyCode::Space));
        assert_eq!(parse_key_name("w"), Some(KeyCode::W));
        assert_eq!(parse_key_name("7"), Some(KeyCode::Key7));
        assert_eq!(parse_key_name("key7"), Some(KeyCode::Key7));
        assert_eq!(parse_key_name("nope"), None);
        assert_eq!(parse_key_name(""), None);
    }

    #[test]
    fn resolve_path_prefers_explicit_path() {
        let explicit = PathBuf::from("/tmp/boom-client-options-explicit.toml");
        assert_eq!(
            resolve_path(Some(&explicit)).expect("đường dẫn tường minh"),
            explicit
        );
    }

    #[test]
    fn default_path_points_to_options_file() {
        if let Ok(path) = default_path() {
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some(OPTIONS_FILE_NAME)
            );
            assert!(path.parent().is_some_and(|dir| dir.ends_with(APP_DIR)));
        }
    }
}
