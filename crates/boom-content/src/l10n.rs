//! Chuỗi UI V1 đã Việt hóa: nạp và kiểm tra `data/l10n/vi-v1.toml`.
//!
//! Nguồn: `docs/specification/localization/UI_STRINGS_V1.md` (khóa V1, trạng thái nguồn) và
//! catalog Việt hóa của `boom-re-l10n` (`l10n/catalog.csv`, cột `vi`). File trong repo là
//! **output của tool** `tools/l10n_export.py` — sửa bằng tool, không sửa tay.
//!
//! ## Quy tắc
//!
//! - `schema` phải là [`SCHEMA`]; `catalog_sha256` phải là SHA-256 hex 64 ký tự.
//! - Khóa `catalog-translated`/`catalog-reused` **có** `text` và `catalog_id`.
//!   Khóa `review`/`image-pending`/`unknown` **không có** `text`, chỉ có `status` + `reason`.
//! - Text lấy nguyên văn `catalog.vi` (chỉ trim biên), nên giữ nguyên placeholder
//!   (`%s`, `{0}`, `<font …>`, CRLF, `|`). Hai khóa dùng chung một `catalog_id` phải có
//!   **cùng số lượng placeholder** — lệch là lỗi nạp.
//! - Loader **không** tự viết chữ cho khóa thiếu: [`Text::Missing`] mang lý do để client hiện
//!   placeholder rõ ràng `«khóa»` (xem [`placeholder`]).
//!
//! ## NFC
//!
//! Loader từ chối chuỗi có **dấu tổ hợp** (ký tự Mn/Me/Mc) — đây là kiểm tra *bảo thủ*, bảo đảm
//! không nhận chuỗi đã phân rã. Kiểm tra NFC đầy đủ (chuẩn hóa Unicode) nằm ở
//! `tools/l10n_export.py` (`unicodedata`), là cổng chặn lúc xuất file.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

/// Schema hiện hành của file chuỗi UI V1.
pub const SCHEMA: u32 = 1;

/// Trạng thái nguồn của một khóa (UI_STRINGS_V1 §1).
///
/// `catalog-translated`/`catalog-reused` là hai trạng thái **đã có text chốt**; ba
/// trạng thái còn lại là khóa chưa có text, client không được tự soạn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Có `id` catalog status `translated`; dùng `catalog.vi` nguyên văn.
    CatalogTranslated,
    /// Có `id` catalog status `reused`; dùng `catalog.vi` nguyên văn.
    CatalogReused,
    /// Có `id` catalog nhưng ngữ cảnh/format cần reviewer chọn ở `boom-re-l10n`.
    Review,
    /// Chữ nằm trong ảnh; ảnh đang chờ duyệt trong quy trình `IMAGE_REVIEW`.
    ImagePending,
    /// Chưa có locator catalog/ảnh.
    Unknown,
}

impl Status {
    /// Trạng thái này đã có text hiển thị được.
    pub fn has_text(self) -> bool {
        matches!(self, Self::CatalogTranslated | Self::CatalogReused)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::CatalogTranslated => "catalog-translated",
            Self::CatalogReused => "catalog-reused",
            Self::Review => "review",
            Self::ImagePending => "image-pending",
            Self::Unknown => "unknown",
        };
        f.write_str(name)
    }
}

/// Một khóa trong file chuỗi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Trạng thái nguồn của khóa.
    pub status: Status,
    /// `id` trong catalog Việt hóa (`boom-re-l10n`), nếu khóa có locator catalog.
    pub catalog_id: Option<String>,
    /// Text đã chốt (nguyên văn `catalog.vi`, chỉ trim biên); `None` với khóa chưa có text.
    pub text: Option<String>,
    /// Mã lý do khóa chưa có text (vd. `needs-review`).
    pub reason: Option<String>,
    /// Yêu cầu hiển thị lấy từ UI_STRINGS_V1 §2 (wrap, giữ placeholder…).
    pub note: Option<String>,
    /// Ảnh chờ duyệt (`pack:entry`), chỉ với khóa `image-pending`.
    pub image: Option<String>,
}

/// Lý do một khóa chưa có text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Missing<'a> {
    /// Trạng thái nguồn của khóa.
    pub status: Status,
    /// Mã lý do ổn định (vd. `needs-review`).
    pub reason: &'a str,
    /// Yêu cầu hiển thị/bước tiếp lấy từ spec.
    pub note: Option<&'a str>,
    /// Ảnh chờ duyệt, nếu khóa là `image-pending`.
    pub image: Option<&'a str>,
}

/// Text của một khóa: có chuỗi để hiển thị, hoặc lý do để client hiện placeholder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text<'a> {
    /// Chuỗi đã chốt, hiển thị nguyên văn.
    Some(&'a str),
    /// Chưa có chuỗi; client hiện placeholder `«khóa»` và không tự viết chữ.
    Missing(Missing<'a>),
}

impl<'a> Text<'a> {
    /// Chuỗi hiển thị nếu có, ngược lại `None`.
    pub fn as_str(self) -> Option<&'a str> {
        match self {
            Self::Some(text) => Some(text),
            Self::Missing(_) => None,
        }
    }

    /// Lý do nếu khóa chưa có text.
    pub fn missing(self) -> Option<Missing<'a>> {
        match self {
            Self::Some(_) => None,
            Self::Missing(missing) => Some(missing),
        }
    }
}

/// Dấu mở của placeholder cho khóa chưa có text: `«` (U+00AB).
///
/// Chọn ký tự **có glyph** trong font OFL đã bundle (Be Vietnam Pro 1.002): `⟨`
/// (U+27E8) không có trong font nên placeholder cũ hiện hộp báo thiếu ở hai dấu
/// ngoặc (UI_STRINGS_V1 §5.1.1).
pub const PLACEHOLDER_OPEN: char = '«';

/// Dấu đóng của placeholder cho khóa chưa có text: `»` (U+00BB).
pub const PLACEHOLDER_CLOSE: char = '»';

/// Placeholder rõ ràng cho khóa chưa có text: `«app.loading»`.
///
/// Client dùng chuỗi này khi [`Text::Missing`]; không tự viết chữ thay khóa.
pub fn placeholder(key: &str) -> String {
    format!("{PLACEHOLDER_OPEN}{key}{PLACEHOLDER_CLOSE}")
}

/// Khóa nằm trong một placeholder, nếu chuỗi đó là placeholder.
///
/// Dùng để kiểm chuỗi hiển thị: chuỗi nào là placeholder của một khóa **không có
/// trong file chuỗi** thì là lỗi (client gõ sai khóa), còn placeholder của khóa có
/// trong file nhưng chưa có text thì là trạng thái chờ người duyệt.
#[must_use]
pub fn placeholder_key(text: &str) -> Option<&str> {
    let body = text.strip_prefix(PLACEHOLDER_OPEN)?;
    let body = body.strip_suffix(PLACEHOLDER_CLOSE)?;
    if body.is_empty() || body.contains(PLACEHOLDER_OPEN) || body.contains(PLACEHOLDER_CLOSE) {
        return None;
    }
    Some(body)
}

/// Bộ chuỗi UI V1 đã Việt hóa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L10n {
    locale: String,
    catalog_sha256: String,
    entries: BTreeMap<String, Entry>,
}

impl L10n {
    /// Nạp từ nội dung TOML của file chuỗi.
    pub fn from_toml(text: &str) -> Result<Self, L10nError> {
        check_duplicate_keys(text)?;
        let raw: RawFile =
            toml::from_str(text).map_err(|error| L10nError::Parse(error.to_string()))?;
        if raw.schema != SCHEMA {
            return Err(L10nError::Schema { found: raw.schema });
        }
        if raw.locale.trim().is_empty() {
            return Err(L10nError::Header("locale"));
        }
        if raw.generated_by.trim().is_empty() {
            return Err(L10nError::Header("generated_by"));
        }
        if raw.catalog_source.trim().is_empty() {
            return Err(L10nError::Header("catalog_source"));
        }
        if raw.spec_source.trim().is_empty() {
            return Err(L10nError::Header("spec_source"));
        }
        if !is_sha256_hex(&raw.catalog_sha256) {
            return Err(L10nError::CatalogSha256(raw.catalog_sha256));
        }

        let mut entries = BTreeMap::new();
        for (key, raw_entry) in raw.strings {
            if !is_valid_key(&key) {
                return Err(L10nError::InvalidKey(key));
            }
            let entry = raw_entry.into_entry(&key)?;
            entries.insert(key, entry);
        }
        if entries.is_empty() {
            return Err(L10nError::Empty);
        }

        let l10n = Self {
            locale: raw.locale,
            catalog_sha256: raw.catalog_sha256,
            entries,
        };
        l10n.check_placeholder_groups()?;
        Ok(l10n)
    }

    /// Nạp từ byte của file; file phải là UTF-8.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, L10nError> {
        let text = std::str::from_utf8(bytes).map_err(|_| L10nError::Utf8)?;
        Self::from_toml(text)
    }

    /// Text của một khóa; khóa lạ hoặc chưa có text thì trả lý do tương ứng.
    ///
    /// Khóa không có trong file trả lý do `no-key`; khóa có trong file nhưng chưa có text
    /// trả lý do ghi ở `reason` (vd. `needs-review`).
    pub fn text(&self, key: &str) -> Text<'_> {
        match self.entries.get(key) {
            None => Text::Missing(Missing {
                status: Status::Unknown,
                reason: "no-key",
                note: None,
                image: None,
            }),
            Some(entry) => match entry.text.as_deref() {
                Some(text) => Text::Some(text),
                None => Text::Missing(Missing {
                    status: entry.status,
                    reason: entry.reason.as_deref().unwrap_or("unknown"),
                    note: entry.note.as_deref(),
                    image: entry.image.as_deref(),
                }),
            },
        }
    }

    /// Toàn bộ thông tin của một khóa.
    pub fn entry(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key)
    }

    /// Mã ngôn ngữ của bộ chuỗi (`vi-VN`).
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// SHA-256 của file catalog nguồn lúc xuất.
    pub fn catalog_sha256(&self) -> &str {
        &self.catalog_sha256
    }

    /// Số khóa trong bộ chuỗi.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Bộ chuỗi rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Các khóa theo thứ tự tăng dần.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Hai khóa dùng chung một `catalog_id` phải có cùng số lượng placeholder.
    fn check_placeholder_groups(&self) -> Result<(), L10nError> {
        let mut groups: BTreeMap<&str, Vec<(&str, Vec<String>)>> = BTreeMap::new();
        for (key, entry) in &self.entries {
            let (Some(catalog_id), Some(text)) = (&entry.catalog_id, entry.text.as_deref()) else {
                continue;
            };
            groups
                .entry(catalog_id.as_str())
                .or_default()
                .push((key.as_str(), placeholder_tokens(text)));
        }
        for (catalog_id, members) in groups {
            let Some((reference_key, reference)) = members.first() else {
                continue;
            };
            for (key, tokens) in members.iter().skip(1) {
                let mut left = reference.clone();
                let mut right = tokens.clone();
                left.sort();
                right.sort();
                if left != right {
                    return Err(L10nError::PlaceholderMismatch {
                        catalog_id: catalog_id.to_owned(),
                        key: (*key).to_owned(),
                        other: (*reference_key).to_owned(),
                        left: left.join(" "),
                        right: right.join(" "),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum L10nError {
    /// File không phải UTF-8.
    Utf8,
    /// TOML sai cú pháp.
    Parse(String),
    /// Cùng một khóa xuất hiện hai lần.
    DuplicateKey(String),
    /// Schema không hỗ trợ.
    Schema {
        /// Schema đọc được.
        found: u32,
    },
    /// Thiếu trường bắt buộc ở đầu file.
    Header(&'static str),
    /// `catalog_sha256` không phải SHA-256 hex 64 ký tự.
    CatalogSha256(String),
    /// Khóa không đúng dạng `mục.khóa` (chữ thường, số, `_`).
    InvalidKey(String),
    /// File không có khóa nào.
    Empty,
    /// Trạng thái yêu cầu có text nhưng không có `text`.
    MissingText {
        /// Khóa bị lỗi.
        key: String,
        /// Trạng thái của khóa.
        status: Status,
    },
    /// Trạng thái không được có text nhưng lại có `text`.
    UnexpectedText {
        /// Khóa bị lỗi.
        key: String,
        /// Trạng thái của khóa.
        status: Status,
    },
    /// Khóa chưa có text mà thiếu `reason`.
    MissingReason {
        /// Khóa bị lỗi.
        key: String,
    },
    /// Khóa `unknown` mà lại có `catalog_id` (chưa có locator catalog thì không có id).
    UnexpectedCatalogId {
        /// Khóa bị lỗi.
        key: String,
    },
    /// Khóa có text mà thiếu `catalog_id`.
    MissingCatalogId {
        /// Khóa bị lỗi.
        key: String,
    },
    /// Khóa `image-pending` mà thiếu `image`.
    MissingImage {
        /// Khóa bị lỗi.
        key: String,
    },
    /// Hai khóa dùng chung `catalog_id` lệch số lượng placeholder.
    PlaceholderMismatch {
        /// `catalog_id` chung.
        catalog_id: String,
        /// Khóa bị lỗi.
        key: String,
        /// Khóa dùng để so sánh.
        other: String,
        /// Placeholder của khóa so sánh.
        left: String,
        /// Placeholder của khóa bị lỗi.
        right: String,
    },
    /// Text có dấu tổ hợp: chưa chuẩn hóa NFC (kiểm tra đầy đủ ở `tools/l10n_export.py`).
    CombiningMark {
        /// Khóa bị lỗi.
        key: String,
    },
}

impl fmt::Display for L10nError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8 => write!(f, "file chuỗi không phải UTF-8"),
            Self::Parse(e) => write!(f, "TOML chuỗi sai: {e}"),
            Self::DuplicateKey(key) => write!(f, "khóa trùng: `{key}`"),
            Self::Schema { found } => {
                write!(f, "schema chuỗi {found} không hỗ trợ (cần {SCHEMA})")
            }
            Self::Header(field) => write!(f, "thiếu trường `{field}` ở đầu file"),
            Self::CatalogSha256(value) => {
                write!(f, "catalog_sha256 không phải SHA-256 hex: `{value}`")
            }
            Self::InvalidKey(key) => write!(f, "khóa không hợp lệ: `{key}`"),
            Self::Empty => write!(f, "file chuỗi không có khóa nào"),
            Self::MissingText { key, status } => {
                write!(f, "{key}: trạng thái {status} phải có text")
            }
            Self::UnexpectedText { key, status } => {
                write!(f, "{key}: trạng thái {status} không được có text")
            }
            Self::MissingReason { key } => write!(f, "{key}: khóa chưa có text thì phải có reason"),
            Self::UnexpectedCatalogId { key } => {
                write!(f, "{key}: trạng thái unknown không được có catalog_id")
            }
            Self::MissingCatalogId { key } => {
                write!(f, "{key}: khóa có text thì phải có catalog_id")
            }
            Self::MissingImage { key } => {
                write!(f, "{key}: trạng thái image-pending phải có image")
            }
            Self::PlaceholderMismatch {
                catalog_id,
                key,
                other,
                left,
                right,
            } => write!(
                f,
                "catalog `{catalog_id}`: placeholder của `{key}` [{right}] khác `{other}` [{left}]"
            ),
            Self::CombiningMark { key } => write!(
                f,
                "{key}: text có dấu tổ hợp, chưa chuẩn hóa NFC \
                 (kiểm tra đầy đủ ở tools/l10n_export.py)"
            ),
        }
    }
}

impl std::error::Error for L10nError {}

#[derive(Deserialize)]
struct RawFile {
    schema: u32,
    locale: String,
    generated_by: String,
    catalog_source: String,
    catalog_sha256: String,
    spec_source: String,
    strings: BTreeMap<String, RawEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    status: Status,
    #[serde(default)]
    catalog_id: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    image: Option<String>,
}

impl RawEntry {
    fn into_entry(mut self, key: &str) -> Result<Entry, L10nError> {
        let status = self.status;
        let has_text = self.text.is_some();
        if status.has_text() {
            let text = self.text.take().ok_or_else(|| L10nError::MissingText {
                key: key.to_owned(),
                status,
            })?;
            if text.trim().is_empty() || text != text.trim() {
                return Err(L10nError::MissingText {
                    key: key.to_owned(),
                    status,
                });
            }
            if has_combining_mark(&text) {
                return Err(L10nError::CombiningMark {
                    key: key.to_owned(),
                });
            }
            if self
                .catalog_id
                .as_deref()
                .is_none_or(|id| id.trim().is_empty())
            {
                return Err(L10nError::MissingCatalogId {
                    key: key.to_owned(),
                });
            }
            self.text = Some(text);
        } else {
            if has_text {
                return Err(L10nError::UnexpectedText {
                    key: key.to_owned(),
                    status,
                });
            }
            if self
                .reason
                .as_deref()
                .is_none_or(|reason| reason.trim().is_empty())
            {
                return Err(L10nError::MissingReason {
                    key: key.to_owned(),
                });
            }
            if status == Status::Unknown && self.catalog_id.is_some() {
                return Err(L10nError::UnexpectedCatalogId {
                    key: key.to_owned(),
                });
            }
            if status == Status::ImagePending
                && self
                    .image
                    .as_deref()
                    .is_none_or(|image| image.trim().is_empty())
            {
                return Err(L10nError::MissingImage {
                    key: key.to_owned(),
                });
            }
        }
        Ok(Entry {
            status,
            catalog_id: self.catalog_id,
            text: self.text,
            reason: self.reason,
            note: self.note,
            image: self.image,
        })
    }
}

/// Khoảng codepoint chứa dấu tổ hợp (Mn/Me/Mc), dùng cho kiểm tra NFC bảo thủ.
const COMBINING_RANGES: &[(u32, u32)] = &[
    (0x0300, 0x036F), // Combining Diacritical Marks
    (0x0483, 0x0489), // Cyrillic
    (0x0591, 0x05C7), // Hebrew
    (0x0610, 0x061A), // Arabic
    (0x064B, 0x065F), // Arabic
    (0x0670, 0x0670), // Arabic
    (0x06D6, 0x06ED), // Arabic
    (0x1AB0, 0x1AFF), // Combining Diacritical Marks Extended
    (0x1DC0, 0x1DFF), // Combining Diacritical Marks Supplement
    (0x20D0, 0x20F0), // Combining Diacritical Marks for Symbols
    (0xFE20, 0xFE2F), // Combining Half Marks
];

/// Chuỗi có dấu tổ hợp thì chắc chắn chưa ở dạng NFC.
fn has_combining_mark(text: &str) -> bool {
    text.chars().any(|char| {
        let code = char as u32;
        COMBINING_RANGES
            .iter()
            .any(|(start, end)| code >= *start && code <= *end)
    })
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_valid_key(key: &str) -> bool {
    let mut segments = key.split('.');
    let Some(first) = segments.next() else {
        return false;
    };
    if !is_key_segment(first) {
        return false;
    }
    let mut count = 1;
    for segment in segments {
        if !is_key_segment(segment) {
            return false;
        }
        count += 1;
    }
    count >= 2
}

fn is_key_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Bắt khóa trùng trước khi parse: TOML báo lỗi chung, ở đây báo đúng khóa.
fn check_duplicate_keys(text: &str) -> Result<(), L10nError> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("[strings.") else {
            continue;
        };
        let Some(rest) = rest.strip_suffix(']') else {
            continue;
        };
        let key = rest.trim().trim_matches('"').to_owned();
        let count = seen.entry(key.clone()).or_insert(0);
        *count += 1;
        if *count == 2 {
            return Err(L10nError::DuplicateKey(key));
        }
    }
    Ok(())
}

/// Placeholder của một chuỗi: printf (`%s`, `%02d`, `%.1f%%`), `{0}`, `{ComponentType|N}`,
/// thẻ `<font …>`, CRLF thật và `|`.
///
/// Quy tắc này phải giống `tools/l10n_export.py` để hai bên cùng đếm.
pub fn placeholder_tokens(text: &str) -> Vec<String> {
    const FLAGS: &str = "-+ #0123456789.*";
    let chars: Vec<char> = text.chars().collect();
    let mut tokens: Vec<String> = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '%' => {
                let mut end = index + 1;
                while end < chars.len() && FLAGS.contains(chars[end]) {
                    end += 1;
                }
                if end < chars.len() {
                    tokens.push(chars[index..=end].iter().collect());
                    index = end + 1;
                } else {
                    index += 1;
                }
            }
            '{' => {
                let body: String = chars[index + 1..]
                    .iter()
                    .take_while(|char| **char != '}')
                    .collect();
                let closed = body.len() < chars.len() - index - 1;
                let is_index = !body.is_empty() && body.chars().all(|char| char.is_ascii_digit());
                let is_named = body.rsplit('|').next().is_some_and(|tail| {
                    !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
                });
                if closed && (is_index || (body.contains('|') && is_named)) {
                    tokens.push(format!("{{{body}}}"));
                    index += body.len() + 2;
                } else {
                    index += 1;
                }
            }
            '<' => {
                let is_font = chars[index..]
                    .iter()
                    .take(5)
                    .collect::<String>()
                    .eq_ignore_ascii_case("<font");
                if is_font {
                    if let Some(offset) = chars[index..].iter().position(|char| *char == '>') {
                        tokens.push("<font>".to_owned());
                        index += offset + 1;
                    } else {
                        index += 1;
                    }
                } else {
                    index += 1;
                }
            }
            '\r' => {
                if chars.get(index + 1) == Some(&'\n') {
                    tokens.push("\r\n".to_owned());
                    index += 2;
                } else {
                    index += 1;
                }
            }
            '|' => {
                tokens.push("|".to_owned());
                index += 1;
            }
            _ => index += 1,
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"
schema = 1
locale = "vi-VN"
generated_by = "tools/l10n_export.py"
catalog_source = "/home/admin/boom-re-l10n/l10n/catalog.csv"
catalog_sha256 = "79a704b991f749ee46e66fdcafda8dd62ccbe5f6a4c9152e8de3d20e1fe59513"
spec_source = "docs/specification/localization/UI_STRINGS_V1.md"

[strings."app.loading"]
catalog_id = "Loading"
status = "catalog-reused"
text = "Đang tải..."
note = "Lấy `catalog.vi`; giữ dấu `...`."

[strings."lobby.create"]
status = "image-pending"
reason = "image-pending-review"
image = "FxStage:Lobby/LB_Bt_T_Create-NM"

[strings."app.quit_confirm"]
status = "unknown"
reason = "no-locator"
note = "Cần sửa ở boom-re-l10n: thêm key/asset cho `app.quit_confirm`; chưa render."
"#;

    #[test]
    fn loads_and_reads_text() {
        let l10n = L10n::from_toml(FILE).unwrap();
        assert_eq!(l10n.len(), 3);
        assert_eq!(l10n.locale(), "vi-VN");
        assert_eq!(l10n.text("app.loading"), Text::Some("Đang tải..."));
        assert_eq!(l10n.catalog_sha256().len(), 64);
    }

    #[test]
    fn missing_key_carries_reason() {
        let l10n = L10n::from_toml(FILE).unwrap();
        let missing = l10n.text("lobby.create").missing().unwrap();
        assert_eq!(missing.status, Status::ImagePending);
        assert_eq!(missing.reason, "image-pending-review");
        assert_eq!(missing.image, Some("FxStage:Lobby/LB_Bt_T_Create-NM"));

        let unknown = l10n.text("app.quit_confirm").missing().unwrap();
        assert_eq!(unknown.status, Status::Unknown);
        assert_eq!(unknown.reason, "no-locator");
        assert!(unknown.note.is_some());

        let absent = l10n.text("app.khong_co").missing().unwrap();
        assert_eq!(absent.reason, "no-key");
    }

    #[test]
    fn placeholder_is_explicit() {
        assert_eq!(placeholder("app.loading"), "«app.loading»");
        // Placeholder nhận ra được để kiểm "khóa không có trong file".
        assert_eq!(placeholder_key("«app.loading»"), Some("app.loading"));
        assert_eq!(placeholder_key("«a«b»"), None);
        assert_eq!(placeholder_key("«»"), None);
        assert_eq!(placeholder_key("Đang tải..."), None);
    }

    #[test]
    fn rejects_duplicate_key() {
        let text = format!("{FILE}\n[strings.\"app.loading\"]\nstatus = \"unknown\"\n");
        assert_eq!(
            L10n::from_toml(&text),
            Err(L10nError::DuplicateKey("app.loading".into()))
        );
    }

    #[test]
    fn rejects_placeholder_mismatch_between_shared_keys() {
        let text = format!(
            "{}\n[strings.\"room.loading\"]\ncatalog_id = \"Loading\"\n\
             status = \"catalog-reused\"\ntext = \"Đang tải %s\"\n",
            FILE
        );
        let error = L10n::from_toml(&text).unwrap_err();
        assert!(matches!(error, L10nError::PlaceholderMismatch { .. }));
    }

    #[test]
    fn accepts_placeholder_match_between_shared_keys() {
        let text = format!(
            "{}\n[strings.\"room.loading\"]\ncatalog_id = \"Loading\"\n\
             status = \"catalog-reused\"\ntext = \"Đang tải...\"\n",
            FILE
        );
        assert!(L10n::from_toml(&text).is_ok());
    }

    #[test]
    fn rejects_text_for_missing_status() {
        let text = FILE.replace(
            "[strings.\"app.quit_confirm\"]\nstatus = \"unknown\"",
            "[strings.\"app.quit_confirm\"]\nstatus = \"unknown\"\ntext = \"Thoát?\"",
        );
        assert!(matches!(
            L10n::from_toml(&text),
            Err(L10nError::UnexpectedText { .. })
        ));
    }

    #[test]
    fn rejects_missing_text_for_ready_status() {
        let text = FILE.replace("text = \"Đang tải...\"\n", "");
        assert!(matches!(
            L10n::from_toml(&text),
            Err(L10nError::MissingText { .. })
        ));
    }

    #[test]
    fn rejects_bad_schema_and_hash() {
        assert!(matches!(
            L10n::from_toml(&FILE.replace("schema = 1", "schema = 2")),
            Err(L10nError::Schema { found: 2 })
        ));
        assert!(matches!(
            L10n::from_toml(&FILE.replace(
                "79a704b991f749ee46e66fdcafda8dd62ccbe5f6a4c9152e8de3d20e1fe59513",
                "zz"
            )),
            Err(L10nError::CatalogSha256(_))
        ));
    }

    #[test]
    fn rejects_invalid_key() {
        let text = FILE.replace("[strings.\"app.loading\"]", "[strings.\"App.Loading\"]");
        assert!(matches!(
            L10n::from_toml(&text),
            Err(L10nError::InvalidKey(_))
        ));
    }

    #[test]
    fn rejects_combining_marks() {
        let text = FILE.replace("Đang tải...", "Đang ta\u{0300}i...");
        assert!(matches!(
            L10n::from_toml(&text),
            Err(L10nError::CombiningMark { .. })
        ));
    }

    #[test]
    fn rejects_non_utf8_bytes() {
        assert_eq!(L10n::from_bytes(&[0xff, 0xfe]), Err(L10nError::Utf8));
    }

    #[test]
    fn rejects_missing_reason_and_image() {
        let text = FILE.replace("reason = \"image-pending-review\"\n", "");
        assert!(matches!(
            L10n::from_toml(&text),
            Err(L10nError::MissingReason { .. })
        ));
        let text = FILE.replace("image = \"FxStage:Lobby/LB_Bt_T_Create-NM\"\n", "");
        assert!(matches!(
            L10n::from_toml(&text),
            Err(L10nError::MissingImage { .. })
        ));
    }

    #[test]
    fn rejects_catalog_id_for_unknown_key() {
        let text = FILE.replace(
            "[strings.\"app.quit_confirm\"]\nstatus = \"unknown\"",
            "[strings.\"app.quit_confirm\"]\nstatus = \"unknown\"\ncatalog_id = \"X\"",
        );
        assert!(matches!(
            L10n::from_toml(&text),
            Err(L10nError::UnexpectedCatalogId { .. })
        ));
    }

    #[test]
    fn rejects_unknown_field() {
        let text = FILE.replace(
            "status = \"catalog-reused\"",
            "status = \"catalog-reused\"\ntex = \"gõ sai\"",
        );
        assert!(matches!(L10n::from_toml(&text), Err(L10nError::Parse(_))));
    }

    #[test]
    fn counts_placeholders_like_the_tool() {
        assert_eq!(placeholder_tokens("%s đã tham gia."), vec!["%s"]);
        assert_eq!(placeholder_tokens("a\r\nb"), vec!["\r\n"]);
        assert_eq!(placeholder_tokens("{0} và {1}"), vec!["{0}", "{1}"]);
        assert_eq!(
            placeholder_tokens("{ComponentType|3}"),
            vec!["{ComponentType|3}"]
        );
        assert_eq!(
            placeholder_tokens("<font color=\"red\">x</font>"),
            vec!["<font>"]
        );
        assert_eq!(
            placeholder_tokens("%02d và %.1f%%"),
            vec!["%02d", "%.1f", "%%"]
        );
        assert!(placeholder_tokens("không có placeholder").is_empty());
    }

    #[test]
    fn file_in_repo_is_valid() {
        let text = include_str!("../../../data/l10n/vi-v1.toml");
        let l10n = L10n::from_toml(text).unwrap();
        assert_eq!(l10n.len(), 126);
        let with_text = l10n
            .keys()
            .filter(|key| l10n.text(key).as_str().is_some())
            .count();
        assert_eq!(with_text, 126);
        let missing = l10n
            .keys()
            .filter(|key| l10n.text(key).missing().is_some())
            .count();
        assert_eq!(missing, 0);
    }

    #[test]
    fn file_in_repo_keeps_catalog_text_verbatim() {
        // Mẫu đối chiếu byte với `catalog.vi` của boom-re-l10n (cột `vi`, chỉ trim biên).
        let text = include_str!("../../../data/l10n/vi-v1.toml");
        let l10n = L10n::from_toml(text).unwrap();
        let samples = [
            ("app.loading", "Đang tải..."),
            (
                "app.network_error",
                "Do lỗi mạng nên không thể tiếp tục chơi game.",
            ),
            ("lobby.join_session", "%s đã tham gia."),
            (
                "room.kicked",
                "Bạn đã bị chủ phòng mời ra. Hãy tham gia phòng khác.",
            ),
            (
                "room.leave_ready_confirm",
                "Nếu thoát cửa sổ khi đang sẵn sàng\r\nsẽ bị đưa ra khỏi phòng chờ.\r\nBạn có muốn di chuyển hông?",
            ),
            ("options.sfx", "Hiệu ứng âm thanh"),
        ];
        for (key, expected) in samples {
            assert_eq!(l10n.text(key), Text::Some(expected), "khóa {key}");
        }
    }
}
