//! Rules profile: mỗi giá trị luật mang đơn vị, nguồn, trạng thái và claim (RULES_V1).
//!
//! Định dạng TOML: khóa gốc `schema`, `profile_id`, `release`; mỗi bảng con là một nhóm rule,
//! mỗi rule là `{ value, unit, source, status, claims, tolerance, note }`.
//! Giá trị luôn là số nguyên; float bị từ chối ngay khi parse (ADR 0004).
//!
//! [`RulesOverlay`] là cách mở rộng cho event/season/mode biến thể: một file chỉ chứa các rule
//! bị thay, áp lên profile gốc tại thời điểm bắt đầu lượt; định danh profile đổi theo (ADR 0008).

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::hash::Fnv1a64;

pub const SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    KrStatic,
    KrRuntime,
    VnStatic,
    VnRuntime,
    Web,
    Oss,
    EmuChoice,
    Design,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Confirmed,
    Candidate,
    Design,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub value: Option<i64>,
    pub unit: String,
    pub source: Source,
    pub status: Status,
    #[serde(default)]
    pub claims: Vec<String>,
    #[serde(default)]
    pub tolerance: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
}

// Không dùng deny_unknown_fields: serde không hỗ trợ cùng `flatten`; khóa lạ ở gốc vẫn bị
// từ chối vì phải parse được thành bảng rule.
#[derive(Deserialize)]
struct RawProfile {
    schema: u32,
    profile_id: String,
    release: bool,
    #[serde(flatten)]
    groups: BTreeMap<String, BTreeMap<String, Rule>>,
}

// Không dùng deny_unknown_fields: serde không hỗ trợ cùng `flatten`; khóa lạ ở gốc vẫn bị
// từ chối vì phải parse được thành bảng rule.
#[derive(Deserialize)]
struct RawOverlay {
    schema: u32,
    overlay_id: String,
    base: String,
    #[serde(flatten)]
    groups: BTreeMap<String, BTreeMap<String, Rule>>,
}

/// Tập rule thay thế cho một profile gốc (event, season, biến thể mode).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesOverlay {
    pub overlay_id: String,
    /// `profile_id` mà overlay được viết cho; áp lên profile khác bị từ chối.
    pub base: String,
    pub rules: BTreeMap<String, Rule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesProfile {
    pub profile_id: String,
    pub release: bool,
    /// Khóa dạng `nhóm.tên`, sắp xếp ổn định.
    pub rules: BTreeMap<String, Rule>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProfileError {
    Parse(String),
    Schema(u32),
    UnknownWithValue(String),
    MissingValue(String),
    MissingClaims(String),
    NotReleasable(String, Status),
    Missing(String),
    Unknown(String),
    UnitMismatch {
        key: String,
        expected: String,
        found: String,
    },
    OutOfRange(String, i64),
    OverlayBase {
        overlay: String,
        base: String,
        profile: String,
    },
    OverlayNewKey(String),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "không đọc được profile: {e}"),
            Self::Schema(s) => write!(f, "schema profile {s} không hỗ trợ (cần {SCHEMA})"),
            Self::UnknownWithValue(k) => write!(f, "{k}: status unknown không được mang value"),
            Self::MissingValue(k) => write!(f, "{k}: thiếu value"),
            Self::MissingClaims(k) => write!(f, "{k}: rule không phải design thuần phải trỏ claim"),
            Self::NotReleasable(k, s) => {
                write!(f, "{k}: profile release không chấp nhận status {s:?}")
            }
            Self::Missing(k) => write!(f, "{k}: rule không có trong profile"),
            Self::Unknown(k) => write!(f, "{k}: giá trị unknown, engine không tự đặt mặc định"),
            Self::UnitMismatch {
                key,
                expected,
                found,
            } => write!(f, "{key}: đơn vị {found}, cần {expected}"),
            Self::OutOfRange(k, v) => write!(f, "{k}: giá trị {v} ngoài miền cho phép"),
            Self::OverlayBase {
                overlay,
                base,
                profile,
            } => write!(
                f,
                "overlay {overlay} viết cho {base}, không áp được lên {profile}"
            ),
            Self::OverlayNewKey(k) => {
                write!(
                    f,
                    "{k}: overlay chỉ được thay rule có sẵn (tránh gõ sai khóa)"
                )
            }
        }
    }
}

impl std::error::Error for ProfileError {}

impl RulesProfile {
    pub fn from_toml(text: &str) -> Result<Self, ProfileError> {
        let raw: RawProfile =
            toml::from_str(text).map_err(|e| ProfileError::Parse(e.to_string()))?;
        if raw.schema != SCHEMA {
            return Err(ProfileError::Schema(raw.schema));
        }
        let profile = Self {
            profile_id: raw.profile_id,
            release: raw.release,
            rules: flatten(raw.groups),
        };
        profile.validate()?;
        Ok(profile)
    }

    /// Profile mới = profile này + overlay. `profile_id` thành `gốc+overlay`, nên định danh
    /// trong replay ghi rõ overlay đã áp. Overlay áp theo thứ tự gọi.
    pub fn with_overlay(&self, overlay: &RulesOverlay) -> Result<Self, ProfileError> {
        let current_base = self.profile_id.split('+').next().unwrap_or_default();
        if overlay.base != current_base {
            return Err(ProfileError::OverlayBase {
                overlay: overlay.overlay_id.clone(),
                base: overlay.base.clone(),
                profile: self.profile_id.clone(),
            });
        }
        let mut next = self.clone();
        for (key, rule) in &overlay.rules {
            if !next.rules.contains_key(key) {
                return Err(ProfileError::OverlayNewKey(key.clone()));
            }
            next.rules.insert(key.clone(), rule.clone());
        }
        next.profile_id = format!("{}+{}", self.profile_id, overlay.overlay_id);
        next.validate()?;
        Ok(next)
    }

    fn validate(&self) -> Result<(), ProfileError> {
        for (key, rule) in &self.rules {
            match (rule.status, rule.value) {
                (Status::Unknown, Some(_)) => {
                    return Err(ProfileError::UnknownWithValue(key.clone()));
                }
                (Status::Unknown, None) => {}
                (_, None) => return Err(ProfileError::MissingValue(key.clone())),
                (_, Some(_)) => {}
            }
            if rule.claims.is_empty() && rule.source != Source::Design {
                return Err(ProfileError::MissingClaims(key.clone()));
            }
            if self.release && matches!(rule.status, Status::Candidate | Status::Unknown) {
                return Err(ProfileError::NotReleasable(key.clone(), rule.status));
            }
        }
        Ok(())
    }

    /// Giá trị số nguyên của rule; kiểm đơn vị để tránh đọc nhầm (vd. ms và tick).
    pub fn int(&self, key: &str, unit: &str) -> Result<i64, ProfileError> {
        let rule = self
            .rules
            .get(key)
            .ok_or_else(|| ProfileError::Missing(key.to_owned()))?;
        if rule.unit != unit {
            return Err(ProfileError::UnitMismatch {
                key: key.to_owned(),
                expected: unit.to_owned(),
                found: rule.unit.clone(),
            });
        }
        rule.value
            .ok_or_else(|| ProfileError::Unknown(key.to_owned()))
    }

    /// Như [`Self::int`] nhưng ép vào miền `[min, max]`.
    pub fn int_in(&self, key: &str, unit: &str, min: i64, max: i64) -> Result<i64, ProfileError> {
        let v = self.int(key, unit)?;
        if (min..=max).contains(&v) {
            Ok(v)
        } else {
            Err(ProfileError::OutOfRange(key.to_owned(), v))
        }
    }

    /// Định danh profile trên encoding chuẩn của các rule, không phụ thuộc định dạng file
    /// (khoảng trắng, CRLF, thứ tự khóa). `note` là chú thích nên không vào hash.
    pub fn identity_hash(&self) -> u64 {
        let mut h = Fnv1a64::default();
        h.str(&self.profile_id).u8(u8::from(self.release));
        h.u32(u32::try_from(self.rules.len()).expect("quá nhiều rule"));
        for (key, rule) in &self.rules {
            h.str(key).str(&rule.unit);
            match rule.value {
                Some(v) => h.u8(1).i64(v),
                None => h.u8(0),
            };
            h.u8(rule.source as u8).u8(rule.status as u8);
            match rule.tolerance {
                Some(t) => h.u8(1).i64(t),
                None => h.u8(0),
            };
            h.u32(u32::try_from(rule.claims.len()).expect("quá nhiều claim"));
            for claim in &rule.claims {
                h.str(claim);
            }
        }
        h.finish()
    }
}

impl RulesOverlay {
    pub fn from_toml(text: &str) -> Result<Self, ProfileError> {
        let raw: RawOverlay =
            toml::from_str(text).map_err(|e| ProfileError::Parse(e.to_string()))?;
        if raw.schema != SCHEMA {
            return Err(ProfileError::Schema(raw.schema));
        }
        Ok(Self {
            overlay_id: raw.overlay_id,
            base: raw.base,
            rules: flatten(raw.groups),
        })
    }
}

fn flatten(groups: BTreeMap<String, BTreeMap<String, Rule>>) -> BTreeMap<String, Rule> {
    let mut rules = BTreeMap::new();
    for (group, entries) in groups {
        for (name, rule) in entries {
            rules.insert(format!("{group}.{name}"), rule);
        }
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"
schema = 1
profile_id = "t"
release = false

[sim.ticks_per_second]
value = 60
unit = "Hz"
source = "design"
status = "design"
"#;

    #[test]
    fn parses_and_reads_values() {
        let p = RulesProfile::from_toml(BASE).unwrap();
        assert_eq!(p.int("sim.ticks_per_second", "Hz"), Ok(60));
        assert!(matches!(
            p.int("sim.ticks_per_second", "ms"),
            Err(ProfileError::UnitMismatch { .. })
        ));
        assert_eq!(
            p.int("bomb.fuse_ms", "ms"),
            Err(ProfileError::Missing("bomb.fuse_ms".into()))
        );
    }

    #[test]
    fn rejects_float_values() {
        let text = BASE.replace("value = 60", "value = 60.5");
        assert!(matches!(
            RulesProfile::from_toml(&text),
            Err(ProfileError::Parse(_))
        ));
    }

    #[test]
    fn unknown_must_not_carry_value() {
        let text = BASE.replace("status = \"design\"", "status = \"unknown\"");
        assert_eq!(
            RulesProfile::from_toml(&text),
            Err(ProfileError::UnknownWithValue(
                "sim.ticks_per_second".into()
            ))
        );
    }

    #[test]
    fn unknown_without_value_is_reported_on_read() {
        let text = BASE
            .replace("value = 60\n", "")
            .replace("status = \"design\"", "status = \"unknown\"");
        let p = RulesProfile::from_toml(&text).unwrap();
        assert_eq!(
            p.int("sim.ticks_per_second", "Hz"),
            Err(ProfileError::Unknown("sim.ticks_per_second".into()))
        );
    }

    #[test]
    fn non_design_rule_needs_claims() {
        let text = BASE.replace("source = \"design\"", "source = \"oss\"");
        assert_eq!(
            RulesProfile::from_toml(&text),
            Err(ProfileError::MissingClaims("sim.ticks_per_second".into()))
        );
    }

    #[test]
    fn release_profile_rejects_candidates() {
        let text = BASE
            .replace("release = false", "release = true")
            .replace(
                "source = \"design\"",
                "source = \"oss\"\nclaims = [\"R-014\"]",
            )
            .replace("status = \"design\"", "status = \"candidate\"");
        assert_eq!(
            RulesProfile::from_toml(&text),
            Err(ProfileError::NotReleasable(
                "sim.ticks_per_second".into(),
                Status::Candidate
            ))
        );
    }

    #[test]
    fn identity_ignores_formatting_but_not_values() {
        let a = RulesProfile::from_toml(BASE).unwrap();
        let b = RulesProfile::from_toml(&BASE.replace('\n', "\r\n")).unwrap();
        assert_eq!(a.identity_hash(), b.identity_hash());
        let c = RulesProfile::from_toml(&BASE.replace("value = 60", "value = 30")).unwrap();
        assert_ne!(a.identity_hash(), c.identity_hash());
    }

    const OVERLAY: &str = r#"
schema = 1
overlay_id = "double-speed"
base = "t"

[sim.ticks_per_second]
value = 120
unit = "Hz"
source = "design"
status = "design"
"#;

    #[test]
    fn overlay_replaces_rules_and_changes_identity() {
        let base = RulesProfile::from_toml(BASE).unwrap();
        let overlay = RulesOverlay::from_toml(OVERLAY).unwrap();
        let p = base.with_overlay(&overlay).unwrap();
        assert_eq!(p.profile_id, "t+double-speed");
        assert_eq!(p.int("sim.ticks_per_second", "Hz"), Ok(120));
        assert_ne!(p.identity_hash(), base.identity_hash());
    }

    #[test]
    fn overlay_checks_base_and_keys() {
        let base =
            RulesProfile::from_toml(&BASE.replace("profile_id = \"t\"", "profile_id = \"x\""))
                .unwrap();
        let overlay = RulesOverlay::from_toml(OVERLAY).unwrap();
        assert!(matches!(
            base.with_overlay(&overlay),
            Err(ProfileError::OverlayBase { .. })
        ));
        let typo =
            RulesOverlay::from_toml(&OVERLAY.replace("ticks_per_second]", "tick_per_second]"))
                .unwrap();
        let base = RulesProfile::from_toml(BASE).unwrap();
        assert_eq!(
            base.with_overlay(&typo),
            Err(ProfileError::OverlayNewKey("sim.tick_per_second".into()))
        );
    }

    #[test]
    fn dev_profile_in_repo_is_valid() {
        let text = include_str!("../../../data/rules/v1-dev.toml");
        let p = RulesProfile::from_toml(text).unwrap();
        assert!(!p.release);
        assert_eq!(p.int("grid.tile_px", "px"), Ok(40));
    }
}
