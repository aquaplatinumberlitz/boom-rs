//! Âm thanh V1: entry `FxSound`, kiểm định dạng thô và SHA-256 (không decode).
//!
//! **Bảng mapping** (event → entry) là **data**: [`boom_content::assets`] nạp từ
//! `data/assets/audio-v1.toml`, mỗi dòng mang nhãn bằng chứng và claim RULES_V1. Module
//! này chỉ đọc pack theo bảng đó, không khai báo tên entry.
//!
//! Không có bảng ánh xạ event→entry nào được xác nhận bởi consumer client gốc; các mapping
//! trong bảng data là **design** của V1-034 (xem `AUDIO_V1.md`/R-082), còn sự tồn tại
//! entry là `kr_static`. Entry/mapping BGM Village đã xác nhận bởi R-070/R-081; policy
//! loop/một voice là design R-083.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::io;

use boom_content::assets::{AssetError, AudioMapping, AudioTable};

use crate::closure::sha256_hex;
use crate::pack::{Pack, PackError};

/// Một entry âm thanh đọc từ pack: metadata + hash của payload **thô** (không decode).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioInfo {
    pub entry: String,
    pub kind: AudioKind,
    pub format: AudioFormat,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioKind {
    Bgm,
    Sfx,
}

impl AudioKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bgm => "bgm",
            Self::Sfx => "sfx",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFormat {
    Ogg,
    Mp3,
    Wav,
    Unknown,
}

impl AudioFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ogg => "ogg",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Unknown => "unknown",
        }
    }
}

/// Bảng event→entry V1 của V1-034, nạp từ `data/assets/audio-v1.toml`.
///
/// Kiểu này là iterable để CLI in bảng giữ nguyên lệnh cũ:
/// `for usage in audio::V1_AUDIO_USES { … }`.
#[derive(Clone, Copy, Debug, Default)]
pub struct V1AudioUses;

impl IntoIterator for V1AudioUses {
    type Item = AudioMapping;
    type IntoIter = std::vec::IntoIter<AudioMapping>;

    fn into_iter(self) -> Self::IntoIter {
        // Bảng nhúng trong crate được test chặn (xem `boom_content::assets`), nên lỗi ở
        // đây là build hỏng, không phải lỗi lúc chạy.
        v1_audio_uses()
            .expect("bảng audio V1 trong data/assets/ phải hợp lệ")
            .into_iter()
    }
}

/// Bảng event→entry V1 (nguồn chuẩn: `data/assets/audio-v1.toml`).
pub static V1_AUDIO_USES: V1AudioUses = V1AudioUses;

/// Bảng event→entry V1, nạp từ `data/assets/audio-v1.toml`.
///
/// # Errors
///
/// Trả [`AssetError`] khi bảng data sai quy tắc (dữ liệu nhúng trong crate được test
/// chặn nên trên build thường lỗi này không xảy ra).
pub fn v1_audio_uses() -> Result<Vec<AudioMapping>, AssetError> {
    Ok(AudioTable::from_toml(boom_content::assets::AUDIO_V1)?.cues)
}

#[derive(Debug)]
pub enum AudioError {
    Pack(PackError),
    Io(io::Error),
    NonAudioEntry(String),
    /// Bảng mapping audio trong `data/assets/` sai quy tắc.
    Assets(AssetError),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pack(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::NonAudioEntry(name) => write!(f, "entry không phải âm thanh V1: {name}"),
            Self::Assets(e) => write!(f, "bảng mapping âm thanh sai quy tắc: {e}"),
        }
    }
}

impl Error for AudioError {}

impl From<PackError> for AudioError {
    fn from(error: PackError) -> Self {
        Self::Pack(error)
    }
}

impl From<io::Error> for AudioError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<AssetError> for AudioError {
    fn from(error: AssetError) -> Self {
        Self::Assets(error)
    }
}

/// Các entry âm thanh V1, đọc thô và hash; sắp theo tên entry để output ổn định.
pub fn v1_entries(pack: &Pack) -> Result<Vec<AudioInfo>, AudioError> {
    let mut names = BTreeSet::new();
    for usage in v1_audio_uses()? {
        names.insert(usage.entry);
    }
    names
        .into_iter()
        .map(|name| read_entry(pack, &name))
        .collect()
}

/// Toàn bộ entry trong `FxSound` (`MP3/` + `Wave/`), đọc thô và hash.
pub fn all_entries(pack: &Pack) -> Result<Vec<AudioInfo>, AudioError> {
    let mut names: Vec<&str> = pack
        .index()
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .filter(|name| name.starts_with("MP3/") || name.starts_with("Wave/"))
        .collect();
    names.sort_unstable();
    names
        .into_iter()
        .map(|name| read_entry(pack, name))
        .collect()
}

/// Đọc một entry, giữ payload thô và tính SHA-256. Không decode Ogg/MP3/WAV.
pub fn read_entry(pack: &Pack, name: &str) -> Result<AudioInfo, AudioError> {
    if !(name.starts_with("MP3/") || name.starts_with("Wave/")) {
        return Err(AudioError::NonAudioEntry(name.to_owned()));
    }
    let bytes = pack.read_named(name)?;
    Ok(AudioInfo {
        entry: name.to_owned(),
        kind: if name.starts_with("MP3/") {
            AudioKind::Bgm
        } else {
            AudioKind::Sfx
        },
        format: detect_format(&bytes),
        bytes: bytes.len(),
        sha256: sha256_hex(&bytes),
    })
}

/// Xuất payload thô, giữ cấu trúc thư mục entry dưới `root`; không sửa/giải mã.
pub fn extract(pack: &Pack, infos: &[AudioInfo], root: &std::path::Path) -> Result<(), AudioError> {
    for info in infos {
        let bytes = pack.read_named(&info.entry)?;
        let path = root.join(&info.entry);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
    }
    Ok(())
}

pub fn detect_format(bytes: &[u8]) -> AudioFormat {
    if bytes.starts_with(b"OggS") {
        return AudioFormat::Ogg;
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return AudioFormat::Wav;
    }
    if bytes.starts_with(b"ID3") || bytes.first().is_some_and(|byte| *byte == 0xff) {
        return AudioFormat::Mp3;
    }
    AudioFormat::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_raw_formats_without_decoding() {
        assert_eq!(detect_format(b"OggS...."), AudioFormat::Ogg);
        assert_eq!(detect_format(b"RIFF....WAVE"), AudioFormat::Wav);
        assert_eq!(detect_format(b"ID3...."), AudioFormat::Mp3);
        assert_eq!(detect_format(b"\xff\xfb...."), AudioFormat::Mp3);
        assert_eq!(detect_format(b"not audio"), AudioFormat::Unknown);
    }

    #[test]
    fn v1_uses_are_unique_and_audio_entries() {
        let uses = v1_audio_uses().expect("bảng audio V1 trong data/assets/ phải hợp lệ");
        for usage in &uses {
            assert!(
                usage.entry.starts_with("MP3/") || usage.entry.starts_with("Wave/"),
                "{}",
                usage.entry
            );
        }
        let mut events = BTreeSet::new();
        let mut entries = BTreeSet::new();
        for usage in &uses {
            assert!(
                events.insert(usage.event.clone()),
                "event trùng {}",
                usage.event
            );
            assert!(
                entries.insert(usage.entry.clone()),
                "entry trùng {}",
                usage.entry
            );
        }
        assert_eq!(entries.len(), 15);
        assert!(uses.iter().any(|u| u.entry == "MP3/Village"));
        assert_eq!(
            uses.iter()
                .find(|u| u.event == "bgm_loop")
                .expect("có cue bgm_loop")
                .status,
            boom_content::rules::Status::Design
        );
    }
}
