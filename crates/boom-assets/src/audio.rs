//! Âm thanh V1: entry `FxSound`, kiểm định dạng thô và SHA-256 (không decode).
//!
//! Không có bảng ánh xạ event→entry nào được xác nhận bởi consumer client gốc; các mapping trong
//! [`V1_AUDIO_USES`] là **design** của V1-034 (xem `AUDIO_V1.md`/R-082), còn sự tồn tại entry là
//! `kr_static`. BGM Village đã xác nhận bởi R-070.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::io;

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

/// Một event V1 dùng entry âm thanh nào. `status` ghi mức bằng chứng của mapping; `note` giữ
/// lý do/nguồn ngắn để CLI và docs không phải suy lại.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct V1AudioUse {
    pub event: &'static str,
    pub entry: &'static str,
    /// `confirmed` (BGM R-070) hoặc `design` (mapping V1 chưa có consumer trace).
    pub status: &'static str,
    pub note: &'static str,
}

/// Bảng event→entry V1 (design, trừ BGM đã xác nhận R-070). Đây là nguồn cho CLI `audio --list`.
pub const V1_AUDIO_USES: &[V1AudioUse] = &[
    V1AudioUse {
        event: "bgm_loop",
        entry: "MP3/Village",
        status: "confirmed",
        note: "R-070; theme Village, entry FxSound/MP3/Village",
    },
    V1AudioUse {
        event: "bomb_placed",
        entry: "Wave/Cha_BombIgnite",
        status: "design",
        note: "tên fuse/ignite; chưa có trace consumer gốc",
    },
    V1AudioUse {
        event: "bomb_exploded",
        entry: "Wave/BombExplosion001",
        status: "design",
        note: "entry nổ chuẩn; pack có nhiều biến thể 001…032, chưa rõ quy tắc",
    },
    V1AudioUse {
        event: "item_revealed",
        entry: "Wave/Itm_ItemPop",
        status: "design",
        note: "item lộ ra khỏi khối; design V1",
    },
    V1AudioUse {
        event: "item_picked",
        entry: "Wave/Itm_ItemEaten",
        status: "design",
        note: "dùng chung Bomb/Fire/Roller; chưa có entry riêng cho V1",
    },
    V1AudioUse {
        event: "player_trapped",
        entry: "Wave/Itm_UseTrap",
        status: "design",
        note: "ứng viên trap/bong bóng; chưa có trace consumer gốc",
    },
    V1AudioUse {
        event: "player_died",
        entry: "Wave/Cha_Die",
        status: "design",
        note: "entry chết chung; chưa có trace consumer gốc",
    },
    V1AudioUse {
        event: "round_ended_win",
        entry: "Wave/Ntc_Win",
        status: "design",
        note: "thắng; MP3/Ntc_Win là bản khác, chưa chọn",
    },
    V1AudioUse {
        event: "round_ended_lose",
        entry: "Wave/Ntc_Lose",
        status: "design",
        note: "thua; chưa có trace consumer gốc",
    },
    V1AudioUse {
        event: "round_ended_draw",
        entry: "Wave/Ntc_Draw",
        status: "design",
        note: "hòa; chưa có trace consumer gốc",
    },
    V1AudioUse {
        event: "start_count_3",
        entry: "Wave/Ntc_Count3",
        status: "design",
        note: "đếm ngược start theo thứ tự 3→2→1→go",
    },
    V1AudioUse {
        event: "start_count_2",
        entry: "Wave/Ntc_Count2",
        status: "design",
        note: "đếm ngược start; design V1",
    },
    V1AudioUse {
        event: "start_count_1",
        entry: "Wave/Ntc_Count1",
        status: "design",
        note: "đếm ngược start; design V1",
    },
    V1AudioUse {
        event: "start_go",
        entry: "Wave/Ntc_GameStart",
        status: "design",
        note: "bắt đầu lượt; design V1",
    },
    V1AudioUse {
        event: "ui_click",
        entry: "Wave/Bt_p_Default",
        status: "design",
        note: "click UI mặc định; chưa có trace consumer gốc",
    },
];

#[derive(Debug)]
pub enum AudioError {
    Pack(PackError),
    Io(io::Error),
    NonAudioEntry(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pack(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::NonAudioEntry(name) => write!(f, "entry không phải âm thanh V1: {name}"),
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

/// Các entry âm thanh V1, đọc thô và hash; sắp theo tên entry để output ổn định.
pub fn v1_entries(pack: &Pack) -> Result<Vec<AudioInfo>, AudioError> {
    let mut names = BTreeSet::new();
    for usage in V1_AUDIO_USES {
        names.insert(usage.entry);
    }
    names
        .into_iter()
        .map(|name| read_entry(pack, name))
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
        for usage in V1_AUDIO_USES {
            assert!(
                usage.entry.starts_with("MP3/") || usage.entry.starts_with("Wave/"),
                "{}",
                usage.entry
            );
        }
        let mut events = BTreeSet::new();
        let mut entries = BTreeSet::new();
        for usage in V1_AUDIO_USES {
            assert!(events.insert(usage.event), "event trùng {}", usage.event);
            assert!(entries.insert(usage.entry), "entry trùng {}", usage.entry);
        }
        assert_eq!(entries.len(), 15);
        assert!(V1_AUDIO_USES.iter().any(|u| u.entry == "MP3/Village"));
    }
}
