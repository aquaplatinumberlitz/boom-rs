//! Oracle — đối chiếu giá trị của engine với nguồn gốc KR/VN (V1-037).
//!
//! # Mục đích
//!
//! Mỗi file trong [`ORACLE_DIR`] là **một oracle**: một kỳ vọng lấy từ nguồn gốc
//! (client KR 00967 hoặc VN VBNB_116) kèm cách đo trên engine hiện tại. Module này:
//!
//! 1. **kiểm metadata** (validator, fail closed): thiếu nguồn/claim/bằng chứng/đơn vị/kỳ vọng
//!    ⇒ lỗi, không đoán;
//! 2. **chạy phép đo** trên engine: đọc giá trị luật từ profile, chạy kịch bản scenario, hoặc
//!    rút trọng số rơi item nhiều lần với seed cố định;
//! 3. **phân loại kết quả**: khớp / lệch đã ghi nhận ở SCOPE_V1 §10 / lệch chưa ghi nhận.
//!
//! Lệch **chưa** ghi nhận ở SCOPE_V1 §10 là lỗi của `cargo xtask ci`; lệch đã ghi nhận và
//! kỳ vọng chưa có số (`unknown`) chỉ được báo trong [`REPORT_PATH`].
//!
//! # Định dạng
//!
//! ```toml
//! # fixtures/oracle/o-001-round-duration.toml
//! [oracle]
//! id = "O-001"
//! name = "Thời hạn vòng 120 s"
//! feature = "V1-037"
//! source = "vn_runtime"          # nguồn gốc của kỳ vọng (AGENTS §4)
//! status = "confirmed"           # trạng thái bằng chứng của nguồn đó
//! claims = ["R-010", "R-110"]   # claim trong RULES_V1
//! evidence = ["docs/specification/gameplay/RULES_V1.md"]   # đường dẫn hoặc địa chỉ KR
//! quantity = "round_duration"    # đại lượng
//! unit = "ms"                    # đơn vị
//! expected = 120000              # kỳ vọng của nguồn gốc (hoặc "unknown")
//! tolerance = 0                  # dung sai
//! profile = "data/rules/v1-dev.toml"
//!
//! [oracle.check]
//! kind = "rule"
//! rule = "round.duration_ms"
//! ```
//!
//! Ba kiểu đo:
//!
//! - `rule`: đọc giá trị mà engine nạp từ profile (`round.duration_ms`, `grid.tile_px`,
//!   `item.*_delta`, `drop.weight_*`);
//! - `scenario`: chạy một file trong `fixtures/scenarios/` và đo (`player_speed`,
//!   `trap_to_death`);
//! - `distribution`: rút trọng số rơi item nhiều lần với seed cố định, so tỷ trọng.
//!
//! **Đơn vị là metadata bắt buộc** (ADR 0013: chỉ so *cùng đại lượng, cùng đơn vị*): mỗi phép đo
//! có một đơn vị cố định — `rule` lấy đơn vị khai trong profile, `scenario` là `px/s` (tốc độ) hoặc
//! `ms` (thời gian nhốt→chết), `distribution` là `permille`. Oracle khai đơn vị khác là metadata
//! sai ⇒ validator báo lỗi, không đo và không quy đổi âm thầm.
//!
//! **Số mẫu phải là số thật**: `distribution` dựng bản đồ với đúng số khối mềm khai trong
//! metadata và đếm số khối engine thực sự vỡ (`block_destroyed`) để ghi vào báo cáo — con số
//! mẫu trong báo cáo là số đo được, không phải số khai.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use boom_content::rules::{RulesProfile, Source, Status};
use boom_core::{ItemEffect, Life, Phase, Rules};
use serde::Deserialize;

use crate::Error;
use crate::setup::{BlastEntry, Match, PlayerEntry, Setup, load_profile};

/// Thư mục chứa các file oracle.
pub const ORACLE_DIR: &str = "fixtures/oracle";
/// File báo cáo sinh từ oracle (generated — đừng sửa tay).
pub const REPORT_PATH: &str = "docs/research/ORACLE_REPORT.md";
/// Nguồn chuẩn claim.
pub const RULES_V1: &str = "docs/specification/gameplay/RULES_V1.md";
/// Nguồn chuẩn sổ sai khác có chủ ý (§10).
pub const SCOPE_V1: &str = "docs/planning/SCOPE_V1.md";
/// Đơn vị của phép đo phân phối: tỷ trọng rơi item tính bằng **phần nghìn** (khớp
/// `drop.brick_item_chance` của profile).
pub const DISTRIBUTION_UNIT: &str = "permille";
/// Số khối mềm **tối thiểu** trong một lần đo phân phối: dưới ngưỡng này mẫu quá nhỏ để nói
/// được gì về phân phối.
pub const MIN_DISTRIBUTION_BLOCKS: u32 = 100;
/// Số khối mềm **tối đa** trong một lần đo phân phối (map tối đa 129×128 ô).
///
/// Trần này do **ngân sách event của engine** quyết định: mỗi khối vỡ phát tối đa 2 event
/// (`block_destroyed` + `item_revealed`) mà engine giới hạn 65535 event mỗi tick, nên 16384 khối
/// (32768 event) vẫn nằm xa trần; map cũng vừa để một lần đo chạy nhanh trong CI.
pub const MAX_DISTRIBUTION_BLOCKS: u32 = 16_384;

/// Kỳ vọng của oracle: số đo được, hoặc `unknown` khi nguồn gốc chưa có giá trị
/// (AGENTS §4: `unknown` không mang giá trị).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Expected {
    /// Kỳ vọng có số.
    Value(i64),
    /// Nguồn gốc chưa có giá trị để so (`expected = "unknown"`).
    Unknown(String),
}

impl Expected {
    /// Số kỳ vọng, nếu có.
    pub fn value(self) -> Option<i64> {
        match self {
            Self::Value(v) => Some(v),
            Self::Unknown(_) => None,
        }
    }
}

/// Đại lượng đo trên kịch bản.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    /// Tốc độ người chơi (px/s) trong lúc chạy kịch bản.
    PlayerSpeed,
    /// Thời gian từ lúc bị nhốt tới lúc chết (ms).
    TrapToDeath,
}

impl Measure {
    /// Đơn vị **bắt buộc** của phép đo này.
    ///
    /// ADR 0013: chỉ so **cùng đại lượng, cùng đơn vị**. Phép đo luôn trả về một đơn vị cố
    /// định, nên oracle khai đơn vị khác là metadata sai — validator chặn, không đoán
    /// (xem [`validate`]).
    pub fn unit(self) -> &'static str {
        match self {
            Self::PlayerSpeed => "px/s",
            Self::TrapToDeath => "ms",
        }
    }

    /// Tên phép đo để in lỗi metadata.
    pub fn name(self) -> &'static str {
        match self {
            Self::PlayerSpeed => "player_speed",
            Self::TrapToDeath => "trap_to_death",
        }
    }
}

/// Cách đo một oracle.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Check {
    /// Giá trị luật mà engine nạp từ profile.
    Rule {
        /// Khoá rule trong profile (vd. `round.duration_ms`).
        rule: String,
    },
    /// Đo trên một kịch bản scenario.
    Scenario {
        /// Đường dẫn file scenario (tương đối repo root).
        scenario: PathBuf,
        /// Đại lượng cần đo.
        measure: Measure,
        /// Id người chơi cần đo (mặc định 0).
        #[serde(default)]
        player: Option<u8>,
    },
    /// Phân phối trọng số rơi item: rút nhiều lần với seed cố định.
    Distribution {
        /// Loại item cần đo (Bomb | Fire | Roller).
        item: String,
        /// Số khối mềm vỡ trong một lần đo (bản đồ dựng đúng số này; trần hợp lệ:
        /// `MIN_DISTRIBUTION_BLOCKS`…`MAX_DISTRIBUTION_BLOCKS`).
        blocks: u32,
        /// Seed cố định của lần đo.
        seed: u64,
    },
}

/// Một oracle: kỳ vọng từ nguồn gốc + cách đo trên engine.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Oracle {
    /// Id ổn định `O-NNN`.
    pub id: String,
    /// Tên ngắn để đọc trong báo cáo.
    pub name: String,
    /// Feature ID của V1 mà oracle phục vụ.
    pub feature: String,
    /// Nguồn gốc của kỳ vọng.
    pub source: Source,
    /// Trạng thái bằng chứng của nguồn gốc.
    pub status: Status,
    /// Claim trong RULES_V1 mà kỳ vọng dựa vào.
    pub claims: Vec<String>,
    /// Bằng chứng: đường dẫn trong repo, hoặc địa chỉ trong client gốc (`0x…`).
    pub evidence: Vec<String>,
    /// Tên đại lượng.
    pub quantity: String,
    /// Đơn vị của đại lượng.
    pub unit: String,
    /// Kỳ vọng của nguồn gốc.
    pub expected: Expected,
    /// Dung sai (cùng đơn vị với `expected`).
    pub tolerance: i64,
    /// Cách đo.
    pub check: Check,
    /// Profile luật dùng cho phép đo (bắt buộc với `rule` và `distribution`).
    #[serde(default)]
    pub profile: Option<PathBuf>,
    /// Ghi chú thêm (nguồn con số, cách quy đổi đơn vị…).
    #[serde(default)]
    pub note: Option<String>,
    /// Mục sai khác đã ghi nhận ở SCOPE_V1 §10 (bắt buộc khi lệch hoặc kỳ vọng `unknown`).
    #[serde(default)]
    pub diff: Option<String>,
}

/// Một file oracle: bảng `[oracle]` duy nhất.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleFile {
    oracle: Oracle,
}

/// Trạng thái đối chiếu của một oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Khớp kỳ vọng trong dung sai.
    Match,
    /// Lệch, đã có mục ở SCOPE_V1 §10 (chỉ báo).
    Recorded(String),
    /// Lệch, chưa ghi nhận ở SCOPE_V1 §10 (lỗi CI).
    Unrecorded,
    /// Nguồn gốc chưa có giá trị để so (chỉ báo).
    NoExpectation(String),
}

impl Verdict {
    /// Nhãn ngắn để in trong báo cáo.
    pub fn label(&self) -> String {
        match self {
            Self::Match => "khớp".to_owned(),
            Self::Recorded(id) => format!("lệch (đã ghi nhận {id})"),
            Self::Unrecorded => "lệch (chưa ghi nhận)".to_owned(),
            Self::NoExpectation(id) => format!("chưa có số (gap {id})"),
        }
    }

    /// Lỗi cần chặn CI (rỗng nếu oracle không chặn).
    pub fn blocking_error(&self, id: &str) -> Option<String> {
        match self {
            Self::Unrecorded => Some(format!(
                "{id}: sai khác chưa được ghi nhận ở SCOPE_V1 §10 — \
                 thêm mục D-xx/G-xx vào sổ sai khác rồi trỏ `diff` trong file oracle"
            )),
            _ => None,
        }
    }
}

/// Kết quả đối chiếu của một oracle.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// Oracle đã đo.
    pub oracle: Oracle,
    /// Kết quả engine, dạng text (kèm đơn vị).
    pub measured: String,
    /// Trạng thái đối chiếu.
    pub verdict: Verdict,
    /// Ghi chú thêm cho báo cáo (profile đã dùng, số mẫu…).
    pub detail: String,
}

/// Sổ sai khác ở SCOPE_V1 §10: mã mục → mô tả.
pub type ScopeGaps = BTreeMap<String, String>;

/// Đọc mọi file oracle trong `dir` và kiểm metadata (fail closed).
pub fn load_dir(dir: &Path, root: &Path) -> Result<Vec<Oracle>, Error> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.extension().is_some_and(|x| x == "toml") {
            files.push(path);
        }
    }
    files.sort();
    if files.is_empty() {
        return Err(format!("{}: không có file oracle nào", dir.display()).into());
    }
    let claims = claim_ids(root)?;
    let gaps = scope_gaps(root)?;
    let mut out = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(&file)?;
        let parsed: OracleFile =
            toml::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
        let oracle = parsed.oracle;
        validate(&oracle, &file, root, &claims, &gaps)?;
        if !seen.insert(oracle.id.clone()) {
            return Err(format!("{}: id oracle `{}` bị lặp", file.display(), oracle.id).into());
        }
        out.push(oracle);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Kiểm metadata của một oracle. Thiếu gì là lỗi — không tự đặt giá trị (AGENTS §4).
fn validate(
    oracle: &Oracle,
    file: &Path,
    root: &Path,
    claims: &BTreeSet<String>,
    gaps: &ScopeGaps,
) -> Result<(), Error> {
    let where_ = file.display().to_string();
    if oracle.id.len() != 5
        || !oracle.id.starts_with("O-")
        || !oracle.id[2..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(format!("{where_}: id `{}` không đúng dạng O-NNN", oracle.id).into());
    }
    if oracle.feature.len() != 6
        || !oracle.feature.starts_with("V1-")
        || !oracle.feature[3..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(format!(
            "{where_}: feature `{}` không đúng dạng V1-NNN",
            oracle.feature
        )
        .into());
    }
    if oracle.name.trim().is_empty() {
        return Err(format!("{where_}: thiếu tên oracle").into());
    }
    if oracle.quantity.trim().is_empty() {
        return Err(format!("{where_}: thiếu đại lượng").into());
    }
    if oracle.unit.trim().is_empty() {
        return Err(format!("{where_}: thiếu đơn vị").into());
    }
    if oracle.tolerance < 0 {
        return Err(format!("{where_}: dung sai phải ≥ 0").into());
    }
    if oracle.claims.is_empty() {
        return Err(format!("{where_}: thiếu claim (`oracle.claims`)").into());
    }
    for claim in &oracle.claims {
        if claim.len() != 5
            || !claim.starts_with("R-")
            || !claim[2..].bytes().all(|b| b.is_ascii_digit())
        {
            return Err(format!("{where_}: claim `{claim}` không đúng dạng R-NNN").into());
        }
        if !claims.contains(claim) {
            return Err(format!("{where_}: claim `{claim}` không có trong RULES_V1").into());
        }
    }
    if oracle.evidence.is_empty() {
        return Err(format!("{where_}: thiếu bằng chứng (`oracle.evidence`)").into());
    }
    for item in &oracle.evidence {
        // Địa chỉ trong client gốc (`0x…`) không phải đường dẫn nên không kiểm tồn tại.
        if item.starts_with("0x") {
            continue;
        }
        if !root.join(item).exists() {
            return Err(format!("{where_}: bằng chứng `{item}` không tồn tại trong repo").into());
        }
    }
    // Cặp nguồn/trạng thái phải hợp lệ theo AGENTS §4: `confirmed` chỉ từ KR/VN,
    // `design` chỉ từ BnB hoặc design của dự án, `candidate` không đè được confirmed.
    let pair_ok = matches!(
        (oracle.source, oracle.status),
        (
            Source::KrStatic | Source::KrRuntime | Source::VnStatic | Source::VnRuntime,
            Status::Confirmed
        ) | (Source::Oss | Source::Design, Status::Design)
            | (
                Source::KrStatic
                    | Source::KrRuntime
                    | Source::VnStatic
                    | Source::VnRuntime
                    | Source::Web
                    | Source::Oss
                    | Source::EmuChoice,
                Status::Candidate
            )
    );
    if !pair_ok {
        return Err(format!(
            "{where_}: cặp nguồn/trạng thái `{:?}`/`{:?}` không hợp lệ (AGENTS §4)",
            oracle.source, oracle.status
        )
        .into());
    }
    match (&oracle.expected, oracle.status) {
        (Expected::Unknown(text), _) if text != "unknown" => {
            return Err(format!("{where_}: expected `\"{text}\"` chỉ nhận \"unknown\"").into());
        }
        (Expected::Unknown(_), Status::Confirmed) => {
            return Err(format!(
                "{where_}: claim confirmed phải có kỳ vọng có số (không dùng \"unknown\")"
            )
            .into());
        }
        (Expected::Unknown(_), _) => {
            if oracle.diff.is_none() {
                return Err(format!(
                    "{where_}: kỳ vọng \"unknown\" phải trỏ mục sai khác đã ghi nhận (`diff`)"
                )
                .into());
            }
        }
        (Expected::Value(_), _) => {}
    }
    match &oracle.check {
        Check::Rule { rule } => {
            if rule.trim().is_empty() {
                return Err(format!("{where_}: check `rule` thiếu khoá rule").into());
            }
            // Đơn vị của phép đo `rule` là đơn vị profile khai cho khoá đó — nguồn chuẩn.
            let profile = require_profile(oracle, &where_)?;
            let unit = rule_unit(root, profile, rule).map_err(|e| format!("{where_}: {e}"))?;
            require_unit(&where_, &oracle.unit, &format!("rule `{rule}`"), &unit)?;
        }
        Check::Scenario {
            scenario,
            measure,
            player,
        } => {
            if !root.join(scenario).exists() {
                return Err(format!(
                    "{where_}: kịch bản `{}` không tồn tại trong repo",
                    scenario.display()
                )
                .into());
            }
            if matches!(measure, Measure::TrapToDeath) && player.is_none() {
                return Err(
                    format!("{where_}: đo `trap_to_death` cần biết người chơi (`player`)").into(),
                );
            }
            // Phép đo trả về một đơn vị cố định: oracle khai khác là metadata sai (ADR 0013).
            require_unit(
                &where_,
                &oracle.unit,
                &format!("phép đo `{}`", measure.name()),
                measure.unit(),
            )?;
        }
        Check::Distribution { item, blocks, .. } => {
            if !matches!(item.as_str(), "Bomb" | "Fire" | "Roller") {
                return Err(format!("{where_}: loại item `{item}` không có trong bảng V1").into());
            }
            if *blocks < MIN_DISTRIBUTION_BLOCKS {
                return Err(format!(
                    "{where_}: cần ít nhất {MIN_DISTRIBUTION_BLOCKS} khối vỡ để phân phối có ý nghĩa"
                )
                .into());
            }
            if *blocks > MAX_DISTRIBUTION_BLOCKS {
                return Err(format!(
                    "{where_}: tối đa {MAX_DISTRIBUTION_BLOCKS} khối vỡ trong một lần đo"
                )
                .into());
            }
            // Tỷ trọng đo ra là phần nghìn; oracle khai đơn vị khác là metadata sai (ADR 0013).
            require_unit(
                &where_,
                &oracle.unit,
                "phép đo `distribution`",
                DISTRIBUTION_UNIT,
            )?;
            require_profile(oracle, &where_)?;
        }
    }
    if let Some(diff) = &oracle.diff
        && !gaps.contains_key(diff)
    {
        return Err(format!("{where_}: mục sai khác `{diff}` không có trong SCOPE_V1 §10").into());
    }
    Ok(())
}

/// Profile luật mà phép đo cần; trả về đường dẫn để dùng tiếp.
fn require_profile<'a>(oracle: &'a Oracle, where_: &str) -> Result<&'a Path, Error> {
    let Some(profile) = &oracle.profile else {
        return Err(format!("{where_}: phép đo cần profile luật (`profile`)").into());
    };
    if profile.as_os_str().is_empty() {
        return Err(format!("{where_}: `profile` rỗng").into());
    }
    Ok(profile.as_path())
}

/// Đơn vị mà profile khai cho một khoá rule — nguồn chuẩn của đơn vị khi phép đo là `rule`.
fn rule_unit(root: &Path, profile_path: &Path, rule: &str) -> Result<String, Error> {
    let text = std::fs::read_to_string(root.join(profile_path))?;
    let profile = RulesProfile::from_toml(&text)?;
    let entry = profile
        .rules
        .get(rule)
        .ok_or_else(|| format!("profile {} không có khoá `{rule}`", profile_path.display()))?;
    Ok(entry.unit.clone())
}

/// Chặn oracle khai sai đơn vị so với phép đo.
///
/// ADR 0013: chỉ so **cùng đại lượng, cùng đơn vị** — `mpx/s` không phải `px/s`, `percent` không
/// phải `permille`. Đơn vị là metadata của oracle, nên sai là **lỗi metadata** (fail closed),
/// không phải chuyện quy đổi âm thầm trong lúc đo.
fn require_unit(where_: &str, declared: &str, what: &str, required: &str) -> Result<(), Error> {
    if declared != required {
        return Err(format!(
            "{where_}: {what} có đơn vị `{required}` nhưng oracle khai `{declared}`"
        )
        .into());
    }
    Ok(())
}

/// Đọc mọi id claim (`R-NNN`) có trong RULES_V1.
fn claim_ids(root: &Path) -> Result<BTreeSet<String>, Error> {
    let text = std::fs::read_to_string(root.join(RULES_V1))?;
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("| R-") else {
            continue;
        };
        let id = rest.split('|').next().unwrap_or_default().trim();
        if id.len() == 3 && id.bytes().all(|b| b.is_ascii_digit()) {
            out.insert(format!("R-{id}"));
        }
    }
    if out.is_empty() {
        return Err(format!("{RULES_V1}: không đọc được claim nào").into());
    }
    Ok(out)
}

/// Đọc sổ sai khác ở SCOPE_V1 §10: mã mục (`D-NN`, `G-NN`) → mô tả.
pub fn scope_gaps(root: &Path) -> Result<ScopeGaps, Error> {
    let text = std::fs::read_to_string(root.join(SCOPE_V1))?;
    let mut out = ScopeGaps::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let id = cells[0];
        let is_gap = (id.starts_with("D-") || id.starts_with("G-"))
            && id.len() == 4
            && id[2..].bytes().all(|b| b.is_ascii_digit());
        if is_gap {
            out.insert(id.to_owned(), cells[1].to_owned());
        }
    }
    if out.is_empty() {
        return Err(format!("{SCOPE_V1}: không đọc được mục sai khác nào ở §10").into());
    }
    Ok(out)
}

/// Chạy mọi oracle và trả về kết quả đối chiếu.
pub fn run(oracles: &[Oracle], root: &Path) -> Result<Vec<Outcome>, Error> {
    let mut out = Vec::new();
    for oracle in oracles {
        out.push(run_one(oracle, root)?);
    }
    Ok(out)
}

fn run_one(oracle: &Oracle, root: &Path) -> Result<Outcome, Error> {
    let (measured, detail) = match &oracle.check {
        Check::Rule { rule } => measure_rule(oracle, root, rule)?,
        Check::Scenario {
            scenario,
            measure,
            player,
        } => measure_scenario(oracle, root, scenario, *measure, player.unwrap_or(0))?,
        Check::Distribution { item, blocks, seed } => {
            measure_distribution(oracle, root, item, *blocks, *seed)?
        }
    };
    let verdict = match oracle.expected {
        Expected::Value(expected) => {
            let value = measured
                .parse::<i64>()
                .map_err(|_| format!("{}: kết quả engine không phải số", oracle.id))?;
            if (value - expected).abs() <= oracle.tolerance {
                Verdict::Match
            } else if let Some(diff) = &oracle.diff {
                Verdict::Recorded(diff.clone())
            } else {
                Verdict::Unrecorded
            }
        }
        Expected::Unknown(_) => Verdict::NoExpectation(oracle.diff.clone().unwrap_or_default()),
    };
    Ok(Outcome {
        oracle: oracle.clone(),
        measured,
        verdict,
        detail,
    })
}

/// Đo giá trị luật mà engine nạp từ profile.
fn measure_rule(oracle: &Oracle, root: &Path, rule: &str) -> Result<(String, String), Error> {
    let profile_path = oracle
        .profile
        .as_ref()
        .ok_or_else(|| format!("{}: thiếu profile", oracle.id))?;
    let text = std::fs::read_to_string(root.join(profile_path))?;
    let profile = RulesProfile::from_toml(&text)?;
    let entry = profile
        .rules
        .get(rule)
        .ok_or_else(|| format!("{}: profile không có khoá `{rule}`", oracle.id))?;
    if entry.unit != oracle.unit {
        let unit = &oracle.unit;
        return Err(format!(
            "{}: đơn vị của `{rule}` là `{}`, oracle khai `{unit}`",
            oracle.id, entry.unit
        )
        .into());
    }
    let rules = Rules::from_profile(&profile)?;
    let value = engine_value(&rules, rule)
        .ok_or_else(|| format!("{}: engine không có giá trị cho `{rule}`", oracle.id))?;
    Ok((
        value.to_string(),
        format!("profile {}", profile_path.display()),
    ))
}

/// Giá trị của một khoá rule trong `Rules` — tức là giá trị engine thật sự dùng.
fn engine_value(rules: &Rules, rule: &str) -> Option<i64> {
    let hz = i64::from(rules.ticks_per_second);
    match rule {
        "round.duration_ms" => Some(i64::from(rules.round_ticks) * 1000 / hz),
        "grid.tile_px" => Some(i64::from(rules.tile.0) / 1000),
        "item.bomb_capacity_delta" => {
            item_of(rules, ItemEffect::BombCapacity).map(|i| i64::from(i.amount))
        }
        "item.fire_range_delta" => {
            item_of(rules, ItemEffect::BlastRange).map(|i| i64::from(i.amount))
        }
        "item.roller_speed_delta" => {
            item_of(rules, ItemEffect::SpeedLevel).map(|i| i64::from(i.amount))
        }
        "drop.weight_bomb" => item_of(rules, ItemEffect::BombCapacity).map(|i| i64::from(i.weight)),
        "drop.weight_fire" => item_of(rules, ItemEffect::BlastRange).map(|i| i64::from(i.weight)),
        "drop.weight_roller" => item_of(rules, ItemEffect::SpeedLevel).map(|i| i64::from(i.weight)),
        _ => None,
    }
}

fn item_of(rules: &Rules, effect: ItemEffect) -> Option<&boom_core::ItemDef> {
    rules.items.iter().find(|i| i.effect == effect)
}

/// Đo trên một kịch bản scenario.
fn measure_scenario(
    oracle: &Oracle,
    root: &Path,
    scenario: &Path,
    measure: Measure,
    player: u8,
) -> Result<(String, String), Error> {
    let path = root.join(scenario);
    let base = path.parent().unwrap_or(Path::new("."));
    let (setup, ticks) = crate::scenario::load_setup(&path)?;
    let profile = load_profile(&setup, base)?;
    let rules = Rules::from_profile(&profile)?;
    let hz = rules.ticks_per_second;

    let mut m = Match::new(&setup, base)?;
    let mut previous = m
        .state
        .player(player)
        .map(|p| (p.x.0, p.y.0))
        .ok_or_else(|| format!("{}: kịch bản không có người chơi {player}", oracle.id))?;
    let mut moved_mpx: i64 = 0;
    let mut steps: u32 = 0;
    let mut trapped_at: Option<u32> = None;
    let mut died_at: Option<u32> = None;

    while m.state.tick < ticks && m.state.phase == Phase::Playing {
        m.step()?;
        steps += 1;
        // `state.tick` đã tăng sau tick: tick nội bộ vừa chạy là `state.tick - 1`.
        let internal = m.state.tick - 1;
        let p = m
            .state
            .player(player)
            .ok_or_else(|| format!("{}: người chơi {player} biến mất", oracle.id))?;
        moved_mpx += i64::from((p.x.0 - previous.0).abs()) + i64::from((p.y.0 - previous.1).abs());
        previous = (p.x.0, p.y.0);
        match p.life {
            Life::Trapped { since } => {
                trapped_at.get_or_insert(since);
            }
            Life::Dead => {
                if trapped_at.is_some() {
                    died_at.get_or_insert(internal);
                }
            }
            Life::Alive => {}
        }
    }

    let detail = format!(
        "kịch bản {} · profile {} · {steps} tick ở {hz} Hz",
        scenario.display(),
        profile.profile_id
    );
    match measure {
        Measure::PlayerSpeed => {
            if steps == 0 {
                return Err(format!("{}: kịch bản không chạy được tick nào", oracle.id).into());
            }
            // mpx/s → px/s: quãng đường (mpx) × Hz ÷ (số tick × 1000).
            let speed = moved_mpx * i64::from(hz) / (i64::from(steps) * 1000);
            Ok((speed.to_string(), detail))
        }
        Measure::TrapToDeath => {
            let (Some(trapped), Some(died)) = (trapped_at, died_at) else {
                return Err(
                    format!("{}: kịch bản không đo được thời gian nhốt→chết", oracle.id).into(),
                );
            };
            // Timer nhốt tính cả tick bị nhốt (R-108): số tick = chết − nhốt + 1.
            let ticks = u64::from(died - trapped) + 1;
            let ms = ticks * 1000 / u64::from(hz);
            Ok((ms.to_string(), detail))
        }
    }
}

/// Đo phân phối trọng số rơi item: vỡ nhiều khối mềm rồi đếm loại item rơi ra.
///
/// Bản đồ dựng **đúng** `blocks` khối mềm (không phải một bản đồ cố định), và số khối vỡ được
/// **đếm từ engine** (`block_destroyed`) nên số mẫu ghi trong báo cáo là số thật, không phải con
/// số khai trong metadata.
fn measure_distribution(
    oracle: &Oracle,
    root: &Path,
    item: &str,
    blocks: u32,
    seed: u64,
) -> Result<(String, String), Error> {
    let profile_path = oracle
        .profile
        .as_ref()
        .ok_or_else(|| format!("{}: thiếu profile", oracle.id))?;
    let sample = distribution_sample(root, profile_path, item, blocks, seed)
        .map_err(|e| format!("{}: {e}", oracle.id))?;
    let share = sample.share_permille();
    let detail = format!(
        "khối khai {blocks} · engine vỡ {} khối → {} item rơi · seed {seed} · profile {}",
        sample.broken_blocks,
        sample.items,
        profile_path.display()
    );
    Ok((share.to_string(), detail))
}

/// Số liệu thô của một lần đo phân phối: số khối engine thực sự vỡ và số item rơi.
///
/// Tách riêng khỏi phép đo để **kiểm chứng được số mẫu**: test đổi `blocks` thì số khối vỡ và
/// số item rơi phải đổi theo, không được đo chung một tập mẫu cố định.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DistributionSample {
    /// Số khối mềm khai trong metadata oracle.
    pub declared_blocks: u32,
    /// Số khối mềm engine thực sự vỡ (đếm từ event `block_destroyed`).
    pub broken_blocks: u32,
    /// Số item rơi ra từ các khối vỡ.
    pub items: u64,
    /// Số item rơi đúng loại đang đo.
    pub picked: u64,
}

impl DistributionSample {
    /// Tỷ trọng item đang đo, tính bằng phần nghìn (làm tròn xuống).
    pub fn share_permille(&self) -> u64 {
        if self.items == 0 {
            return 0;
        }
        self.picked * 1000 / self.items
    }
}

/// Đo phân phối trọng số rơi item với `blocks` khối mềm và seed cố định.
///
/// Bản đồ dựng **đúng** `blocks` khối mềm: ô spawn cho người chơi, còn lại là khối mềm; ô nào
/// thừa ra của hình chữ nhật là nền (không vỡ được) nên không tính vào mẫu. Số khối engine vỡ
/// được đếm từ event `block_destroyed`; lệch với số khai thì báo lỗi — mẫu phải khớp metadata.
pub fn distribution_sample(
    root: &Path,
    profile_path: &Path,
    item: &str,
    blocks: u32,
    seed: u64,
) -> Result<DistributionSample, Error> {
    if !(MIN_DISTRIBUTION_BLOCKS..=MAX_DISTRIBUTION_BLOCKS).contains(&blocks) {
        return Err(format!(
            "số khối {blocks} ngoài khoảng hợp lệ {MIN_DISTRIBUTION_BLOCKS}..={MAX_DISTRIBUTION_BLOCKS}"
        )
        .into());
    }
    let text = std::fs::read_to_string(root.join(profile_path))?;
    let profile = RulesProfile::from_toml(&text)?;
    let rules = Rules::from_profile(&profile)?;
    let Some(index) = rules.items.iter().position(|i| i.key == item) else {
        return Err(format!("item `{item}` không có trong profile").into());
    };

    let (map, blasts) = soft_block_map(blocks);
    let setup = Setup {
        rules: Some(profile_path.to_path_buf()),
        seed: Some(seed),
        map,
        blast: blasts,
        player: vec![PlayerEntry {
            id: 0,
            speed: [5, 7],
            bombs: [1, 10],
            range: [1, 7],
            cell: None,
            life: None,
        }],
        ..Setup::default()
    };
    let mut m = Match::new(&setup, root)?;
    // Warmup (R-100) đóng băng các system nên kênh blast chỉ được xử lý ở tick đầu sau warmup.
    let steps = u64::from(rules.warmup_ticks) + 2;
    for _ in 0..steps {
        m.step()?;
    }

    let mut broken: u32 = 0;
    let mut total: u64 = 0;
    let mut picked: u64 = 0;
    for event in &m.events {
        match event.kind {
            boom_core::EventKind::BlockDestroyed { .. } => broken += 1,
            boom_core::EventKind::ItemRevealed { kind, .. } => {
                total += 1;
                if usize::from(kind.0) == index {
                    picked += 1;
                }
            }
            _ => {}
        }
    }
    if broken != blocks {
        return Err(format!(
            "engine vỡ {broken} khối nhưng oracle khai {blocks} khối — số mẫu lệch metadata"
        )
        .into());
    }
    if total == 0 {
        return Err("không có item nào rơi để đo".into());
    }
    Ok(DistributionSample {
        declared_blocks: blocks,
        broken_blocks: broken,
        items: total,
        picked,
    })
}

/// Dựng bản đồ toàn khối mềm với **đúng** `blocks` khối, kèm kênh blast phủ hết chúng.
///
/// Bản đồ là hình chữ nhật: chiều rộng gần như căn bậc hai của `blocks + 1` (chừa một ô spawn
/// cho người chơi), chiều cao đủ chứa hết. Ô không dùng làm khối mềm thì là nền (`.`) — không vỡ
/// được nên không tính vào mẫu. Nhờ vậy số khối mềm luôn đúng bằng `blocks` khai.
fn soft_block_map(blocks: u32) -> (Vec<String>, Vec<BlastEntry>) {
    let total = blocks as usize + 1; // +1 ô spawn cho người chơi
    let mut width = 1usize;
    while width * width < total {
        width += 1;
    }
    let height = total.div_ceil(width);
    let mut map = Vec::with_capacity(height);
    let mut blasts = Vec::with_capacity(blocks as usize);
    let mut left = blocks as usize;
    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            if x == 0 && y == 0 {
                row.push('0'); // ô spawn: người chơi đứng, không bị blast
            } else if left > 0 {
                row.push('+');
                left -= 1;
                blasts.push(BlastEntry {
                    cell: [
                        i32::try_from(x).expect("lưới nhỏ"),
                        i32::try_from(y).expect("lưới nhỏ"),
                    ],
                });
            } else {
                row.push('.'); // nền dư: không vỡ được, không tính vào mẫu
            }
        }
        map.push(row);
    }
    (map, blasts)
}

/// Sinh báo cáo Markdown (tất định: không có thời gian, không có đường dẫn tuyệt đối).
pub fn render(outcomes: &[Outcome], gaps: &ScopeGaps) -> String {
    let mut out = String::new();
    out.push_str("# Báo cáo đối chiếu oracle KR/VN ↔ engine (V1-037)\n\n");
    out.push_str(
        "<!-- SINH TỘNG BỞI `cargo xtask oracle` — đừng sửa tay; chạy lại lệnh để cập nhật. -->\n\n",
    );
    out.push_str(
        "Báo cáo đối chiếu **kỳ vọng từ nguồn gốc** (client KR 00967 / VN VBNB_116) với **kết quả\n\
         engine V1 đo được**. Mỗi oracle là một file trong [`fixtures/oracle/`](../../fixtures/oracle);\n\
         metadata (nguồn, claim, bằng chứng, đại lượng, đơn vị, kỳ vọng, dung sai) là phần bắt buộc\n\
         của file đó — thiếu thì validator báo lỗi.\n\n",
    );
    out.push_str("## Cách đọc\n\n");
    out.push_str(
        "- **Nguồn / trạng thái**: nguồn gốc của kỳ vọng và mức bằng chứng theo [AGENTS §4](../../AGENTS.md#4-bằng-chứng).\n\
         - **Đại lượng / đơn vị**: tên và đơn vị theo bảng chọn của\n\
           [RULES_V1 §11](../specification/gameplay/RULES_V1.md#11-ưu-tiên-nguồn-hiện-hành--adr-0013) —\n\
           ADR 0013 chỉ so **cùng đại lượng, cùng đơn vị**.\n\
         - **Đơn vị là bắt buộc**: mỗi phép đo có một đơn vị cố định (`rule` lấy đơn vị trong\n\
           profile, `scenario` là `px/s` hoặc `ms`, `distribution` là `permille`); oracle khai khác là\n\
           metadata sai ⇒ validator báo lỗi, không quy đổi âm thầm.\n\
         - **Kỳ vọng**: giá trị của nguồn gốc, kèm dung sai.\n\
         - **Kết quả engine**: giá trị engine đo được trong lần chạy này.\n\
         - **Cỡ mẫu**: với phép đo phân phối, số khối vỡ **do engine đếm** (`block_destroyed`) — đây\n\
           là số mẫu thật mà dung sai thống kê bám theo, không phải con số khai trong metadata.\n\
         - **Trạng thái**: `khớp` (trong dung sai) · `lệch (đã ghi nhận D-xx/G-xx)` (sai khác đã có trong\n\
           [SCOPE_V1 §10](../planning/SCOPE_V1.md#10-sai-khác-có-chủ-ý-so-với-client-gốc)) ·\n\
           `lệch (chưa ghi nhận)` (**lỗi** của `cargo xtask ci`) · `chưa có số` (nguồn gốc chưa có giá trị\n\
           để so, nên chỉ báo khoảng trống).\n\n",
    );
    out.push_str(
        "Quy tắc CI: **sai khác chưa ghi nhận ở SCOPE_V1 §10 là lỗi**; sai khác đã ghi nhận và\n\
         kỳ vọng chưa có số chỉ được báo ở đây. Báo cáo này **không** nâng tầng bằng chứng của\n\
         claim: nó chỉ cho biết engine có khớp nguồn gốc hay không.\n\n",
    );

    out.push_str("## Kết quả\n\n");
    out.push_str(
        "| Oracle | Đại lượng | Nguồn | Kỳ vọng | Kết quả engine | Trạng thái | Ghi chú |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|\n");
    for o in outcomes {
        let oracle = &o.oracle;
        let expectation = match oracle.expected {
            Expected::Value(v) if oracle.tolerance == 0 => v.to_string(),
            Expected::Value(v) => format!("{v} ± {}", oracle.tolerance),
            Expected::Unknown(_) => "chưa có số".to_owned(),
        };
        let source = format!(
            "{} / {} ({})",
            source_text(oracle.source),
            status_text(oracle.status),
            oracle.claims.join(", ")
        );
        let _ = writeln!(
            out,
            "| {} | {} ({}) | {} | {} | {} | {} | {} |",
            oracle.id,
            oracle.quantity,
            oracle.unit,
            source,
            expectation,
            o.measured,
            o.verdict.label(),
            o.detail
        );
    }
    out.push('\n');

    // Danh sách sai khác đã ghi nhận mà oracle chạm tới.
    let mut recorded: Vec<(&str, &Outcome)> = Vec::new();
    for o in outcomes {
        if let Verdict::Recorded(id) = &o.verdict {
            recorded.push((id.as_str(), o));
        }
    }
    out.push_str("## Sai khác đã ghi nhận (SCOPE_V1 §10)\n\n");
    if recorded.is_empty() {
        out.push_str("Không có sai khác nào.\n\n");
    } else {
        for (id, o) in &recorded {
            let description = gaps
                .get(*id)
                .map(String::as_str)
                .unwrap_or("(không tìm thấy)");
            let _ = writeln!(
                out,
                "- **{id}** — {description}. Oracle {}: kỳ vọng `{}`, engine `{}`.",
                o.oracle.id,
                expectation_text(&o.oracle.expected, o.oracle.tolerance),
                o.measured
            );
        }
        out.push('\n');
    }

    // Khoảng trống còn mở: oracle có nguồn chưa có giá trị để so.
    let mut open: Vec<(&str, &Outcome)> = Vec::new();
    for o in outcomes {
        if let Verdict::NoExpectation(id) = &o.verdict {
            open.push((id.as_str(), o));
        }
    }
    out.push_str("## Khoảng trống còn mở\n\n");
    if open.is_empty() {
        out.push_str("Không có khoảng trống nào.\n\n");
    } else {
        out.push_str(
            "Nguồn gốc của các oracle dưới đây **chưa có giá trị** để so (thường là `candidate`),\n\
             nên engine chỉ ghi kết quả đo, không kết luận khớp hay lệch. Khoảng trống đã ghi ở\n\
             SCOPE_V1 §10 và vẫn mở ở [QUESTIONS](QUESTIONS.md).\n\n",
        );
        for (id, o) in &open {
            let description = gaps
                .get(*id)
                .map(String::as_str)
                .unwrap_or("(không tìm thấy)");
            let _ = writeln!(
                out,
                "- **{id}** — {description}. Oracle {}: engine `{}`.",
                o.oracle.id, o.measured
            );
        }
        out.push('\n');
    }

    // Danh sách bằng chứng của từng oracle: người đọc tự mở được.
    out.push_str("## Bằng chứng từng oracle\n\n");
    for o in outcomes {
        let oracle = &o.oracle;
        let evidence = oracle
            .evidence
            .iter()
            .map(|e| {
                if e.starts_with("0x") {
                    format!("`{e}`")
                } else {
                    format!("[`{e}`]({})", evidence_link(e))
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "- **{}** — {}: feature {}, nguồn {} / {}, claim {}. Bằng chứng: {evidence}.",
            oracle.id,
            oracle.name,
            oracle.feature,
            source_text(oracle.source),
            status_text(oracle.status),
            oracle.claims.join(", "),
        );
        if let Some(note) = &oracle.note {
            let _ = writeln!(out, "  - Ghi chú: {note}");
        }
    }
    out.push('\n');

    out.push_str("## Cách sinh lại\n\n");
    out.push_str(
        "- `cargo xtask oracle` — sinh lại file này từ `fixtures/oracle/*.toml`.\n\
         - `cargo xtask oracle --check` — kiểm file này còn khớp kết quả đo và không có sai khác\n\
           chưa ghi nhận; `cargo xtask ci` chạy lệnh này.\n\
         - Oracle là **đối chiếu**, không phải bằng chứng mới: nó không nâng tầng bằng chứng của claim\n\
           và không thay thế kịch bản nghiệm thu trong `fixtures/scenarios/`.\n",
    );
    out
}

fn expectation_text(expected: &Expected, tolerance: i64) -> String {
    match expected {
        Expected::Value(v) if tolerance == 0 => v.to_string(),
        Expected::Value(v) => format!("{v} ± {tolerance}"),
        Expected::Unknown(_) => "chưa có số".to_owned(),
    }
}

/// Đường dẫn tương đối tới file bằng chứng, tính từ thư mục `docs/research/`.
fn evidence_link(path: &str) -> String {
    match path.strip_prefix("docs/") {
        Some(rest) => format!("../{rest}"),
        None => format!("../../{path}"),
    }
}

/// Tên nguồn hiển thị trong báo cáo.
fn source_text(source: Source) -> &'static str {
    match source {
        Source::KrStatic => "kr_static",
        Source::KrRuntime => "kr_runtime",
        Source::VnStatic => "vn_static",
        Source::VnRuntime => "vn_runtime",
        Source::Web => "web",
        Source::Oss => "oss",
        Source::EmuChoice => "emu_choice",
        Source::Design => "design",
    }
}

/// Tên trạng thái hiển thị trong báo cáo.
fn status_text(status: Status) -> &'static str {
    match status {
        Status::Confirmed => "confirmed",
        Status::Candidate => "candidate",
        Status::Design => "design",
        Status::Unknown => "unknown",
    }
}

/// Lỗi chặn CI của một danh sách kết quả.
pub fn blocking_errors(outcomes: &[Outcome]) -> Vec<String> {
    outcomes
        .iter()
        .filter_map(|o| o.verdict.blocking_error(&o.oracle.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Parse một file oracle trong test.
    fn parse_oracle(text: &str) -> Oracle {
        let parsed: OracleFile = toml::from_str(text).expect("TOML oracle hợp lệ");
        parsed.oracle
    }

    /// Chạy validator trên một file oracle trong test.
    fn validate_text(text: &str) -> Result<(), Error> {
        let claims = claim_ids(&root()).unwrap();
        let gaps = scope_gaps(&root()).unwrap();
        validate(
            &parse_oracle(text),
            Path::new("<test>"),
            &root(),
            &claims,
            &gaps,
        )
    }

    #[test]
    fn every_oracle_file_is_valid() {
        let dir = root().join(ORACLE_DIR);
        let oracles = load_dir(&dir, &root()).expect("mọi file oracle phải hợp lệ");
        assert!(!oracles.is_empty(), "phải có ít nhất một oracle");
        let mut ids: Vec<&str> = oracles.iter().map(|o| o.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), oracles.len(), "id oracle phải là duy nhất");
    }

    #[test]
    fn scope_gaps_are_read_from_scope_v1() {
        let gaps = scope_gaps(&root()).expect("đọc được sổ sai khác");
        assert!(gaps.contains_key("D-01"));
        assert!(gaps.contains_key("G-05"));
    }

    #[test]
    fn engine_value_reads_the_profile_the_engine_uses() {
        let text = std::fs::read_to_string(root().join("data/rules/v1-dev.toml")).unwrap();
        let profile = RulesProfile::from_toml(&text).unwrap();
        let rules = Rules::from_profile(&profile).unwrap();
        assert_eq!(engine_value(&rules, "round.duration_ms"), Some(120000));
        assert_eq!(engine_value(&rules, "grid.tile_px"), Some(40));
        assert_eq!(engine_value(&rules, "item.bomb_capacity_delta"), Some(1));
        assert_eq!(engine_value(&rules, "item.fire_range_delta"), Some(1));
        assert_eq!(engine_value(&rules, "drop.weight_bomb"), Some(3650));
        assert_eq!(engine_value(&rules, "drop.weight_fire"), Some(2800));
        assert_eq!(engine_value(&rules, "drop.weight_roller"), Some(2000));
        assert_eq!(engine_value(&rules, "không.có"), None);
    }

    #[test]
    fn missing_metadata_is_rejected() {
        // Thiếu trường bắt buộc ⇒ parse lỗi, không đoán giá trị.
        let text = "[oracle]\nid = \"O-999\"\nname = \"x\"\n";
        assert!(
            toml::from_str::<OracleFile>(text).is_err(),
            "thiếu metadata phải bị từ chối"
        );
    }

    #[test]
    fn diff_must_exist_in_scope_v1() {
        let claims = claim_ids(&root()).unwrap();
        let gaps = scope_gaps(&root()).unwrap();
        let text = r#"[oracle]
id = "O-999"
name = "x"
feature = "V1-037"
source = "vn_static"
status = "confirmed"
claims = ["R-001"]
evidence = ["docs/specification/gameplay/RULES_V1.md"]
quantity = "tile_size"
unit = "px"
expected = 40
tolerance = 0
diff = "G-99"
profile = "data/rules/v1-dev.toml"

[oracle.check]
kind = "rule"
rule = "grid.tile_px"
"#;
        let parsed: OracleFile = toml::from_str(text).unwrap();
        let err = validate(&parsed.oracle, Path::new("<test>"), &root(), &claims, &gaps)
            .expect_err("mục sai khác không có ở SCOPE_V1 §10 phải là lỗi");
        assert!(err.to_string().contains("G-99"), "lỗi phải nêu mục: {err}");
    }

    #[test]
    fn every_diff_is_recorded_in_scope_v1() {
        // Sai khác chưa ghi nhận ở SCOPE_V1 §10 là lỗi CI: oracle nào cũng không được bỏ trống.
        let dir = root().join(ORACLE_DIR);
        let oracles = load_dir(&dir, &root()).unwrap();
        let outcomes = run(&oracles, &root()).unwrap();
        let errors = blocking_errors(&outcomes);
        assert!(errors.is_empty(), "sai khác chưa ghi nhận: {errors:?}");
    }

    #[test]
    fn measured_values_match_the_claims() {
        // Chốt kết quả đo của các oracle đã biết: đổi luật làm lệch thì test này phải đỏ.
        let dir = root().join(ORACLE_DIR);
        let oracles = load_dir(&dir, &root()).unwrap();
        let outcomes = run(&oracles, &root()).unwrap();
        let measured: BTreeMap<&str, &str> = outcomes
            .iter()
            .map(|o| (o.oracle.id.as_str(), o.measured.as_str()))
            .collect();
        assert_eq!(measured.get("O-001"), Some(&"120000"));
        assert_eq!(measured.get("O-002"), Some(&"40"));
        assert_eq!(measured.get("O-003"), Some(&"1"));
        assert_eq!(measured.get("O-004"), Some(&"1"));
        assert_eq!(measured.get("O-005"), Some(&"3650"));
        assert_eq!(measured.get("O-006"), Some(&"2800"));
        assert_eq!(measured.get("O-007"), Some(&"2000"));
        // 17px/s DESIGN → 283mpx/tick ở60Hz; 60tick=16.98px, báo nguyênpx16.
        assert_eq!(measured.get("O-011"), Some(&"16"));
        assert_eq!(measured.get("O-012"), Some(&"120"));
        assert_eq!(measured.get("O-013"), Some(&"6300"));
    }

    /// Oracle phân phối mẫu: cùng metadata với O-008 nhưng `blocks` đổi theo tham số.
    fn distribution_oracle(blocks: u32) -> Oracle {
        parse_oracle(&format!(
            r#"[oracle]
id = "O-996"
name = "Phân phối rơi item — Bomb"
feature = "V1-037"
source = "vn_static"
status = "confirmed"
claims = ["R-051"]
evidence = ["docs/specification/gameplay/RULES_V1.md"]
quantity = "drop_item_weight_share_bomb"
unit = "permille"
expected = 432
tolerance = 40
profile = "data/rules/v1-dev.toml"

[oracle.check]
kind = "distribution"
item = "Bomb"
blocks = {blocks}
seed = 20261022
"#
        ))
    }

    #[test]
    fn distribution_sample_size_follows_the_declared_blocks() {
        // Số mẫu phải theo đúng `blocks` khai trong metadata: bản đồ dựng đúng số khối mềm đó và
        // engine vỡ đúng số khối đó. Đổi `blocks` thì mẫu phải đổi theo — không đo chung một tập
        // mẫu cố định rồi chỉ đổi con số trong báo cáo.
        let profile = Path::new("data/rules/v1-dev.toml");
        let small = distribution_sample(&root(), profile, "Bomb", 100, 20261022).unwrap();
        let large = distribution_sample(&root(), profile, "Bomb", 10_000, 20261022).unwrap();

        assert_eq!(small.broken_blocks, 100, "mẫu nhỏ phải vỡ đúng 100 khối");
        assert_eq!(
            large.broken_blocks, 10_000,
            "mẫu lớn phải vỡ đúng 10000 khối"
        );
        assert_eq!(small.declared_blocks, 100);
        assert_eq!(large.declared_blocks, 10_000);
        assert!(
            large.items > small.items,
            "mẫu lớn phải rơi nhiều item hơn: {} vs {}",
            large.items,
            small.items
        );
        assert!(small.items > 0 && large.items > 0, "phải có item rơi để đo");
        assert!(
            large.picked > small.picked,
            "mẫu lớn phải rơi nhiều item Bomb hơn: {} vs {}",
            large.picked,
            small.picked
        );
        // Tỷ trọng tính từ mẫu thật: phần nghìn trong khoảng [0, 1000].
        assert!(small.share_permille() <= 1000 && large.share_permille() <= 1000);
    }

    #[test]
    fn distribution_at_the_upper_bound_still_runs() {
        // Trần `blocks` phải chạy được trong CI (mẫu lớn nhất mà oracle cho phép khai).
        let profile = Path::new("data/rules/v1-dev.toml");
        let sample =
            distribution_sample(&root(), profile, "Bomb", MAX_DISTRIBUTION_BLOCKS, 20261022)
                .expect("đo được ở trần");
        assert_eq!(sample.broken_blocks, MAX_DISTRIBUTION_BLOCKS);
        assert!(sample.items > 0);
    }

    #[test]
    fn distribution_detail_reports_the_blocks_actually_broken() {
        // Ghi chú trong báo cáo phải nêu số khối engine thực sự vỡ, không chỉ số khai trong metadata.
        let oracle = distribution_oracle(1234);
        let (_, detail) =
            measure_distribution(&oracle, &root(), "Bomb", 1234, 20261022).expect("đo được");
        assert!(
            detail.contains("khối khai 1234") && detail.contains("engine vỡ 1234 khối"),
            "ghi chú phải nêu cả số khai và số khối thật: {detail}"
        );
    }

    #[test]
    fn distribution_blocks_out_of_range_is_rejected() {
        // Số khối ngoài khoảng hợp lệ là metadata sai: quá nhỏ thì mẫu vô nghĩa, quá lớn thì lần
        // đo kéo dài vô hạn.
        let too_small = distribution_oracle(MIN_DISTRIBUTION_BLOCKS - 1);
        let claims = claim_ids(&root()).unwrap();
        let gaps = scope_gaps(&root()).unwrap();
        let err = validate(&too_small, Path::new("<test>"), &root(), &claims, &gaps)
            .expect_err("số khối quá nhỏ phải bị từ chối");
        assert!(
            err.to_string().contains("ít nhất"),
            "lỗi phải nêu ngưỡng: {err}"
        );

        let too_large = distribution_oracle(MAX_DISTRIBUTION_BLOCKS + 1);
        let err = validate(&too_large, Path::new("<test>"), &root(), &claims, &gaps)
            .expect_err("số khối quá lớn phải bị từ chối");
        assert!(
            err.to_string().contains("tối đa"),
            "lỗi phải nêu trần: {err}"
        );

        // Chặn ngay ở phép đo (không chỉ ở validator) để không chạm trần event của engine.
        let profile = Path::new("data/rules/v1-dev.toml");
        let err = distribution_sample(&root(), profile, "Bomb", MAX_DISTRIBUTION_BLOCKS + 1, 1)
            .expect_err("số khối quá trần phải bị từ chối");
        assert!(
            err.to_string().contains("ngoài khoảng hợp lệ"),
            "lỗi phải nêu khoảng hợp lệ: {err}"
        );
    }

    #[test]
    fn distribution_unit_mismatch_is_rejected() {
        // Đơn vị sai là metadata sai (ADR 0013): `percent` không phải `permille` — oracle không
        // được kết luận "khớp" dưới nhãn percent.
        let text = r#"[oracle]
id = "O-995"
name = "Phân phối rơi item — Bomb"
feature = "V1-037"
source = "vn_static"
status = "confirmed"
claims = ["R-051"]
evidence = ["docs/specification/gameplay/RULES_V1.md"]
quantity = "drop_item_weight_share_bomb"
unit = "percent"
expected = 432
tolerance = 40
profile = "data/rules/v1-dev.toml"

[oracle.check]
kind = "distribution"
item = "Bomb"
blocks = 4096
seed = 20261022
"#;
        let err = validate_text(text).expect_err("đơn vị sai phải là lỗi");
        let message = err.to_string();
        assert!(
            message.contains("permille") && message.contains("percent"),
            "lỗi phải nêu cả đơn vị đúng và đơn vị khai: {message}"
        );
    }

    #[test]
    fn scenario_unit_mismatch_is_rejected() {
        // Đơn vị sai là metadata sai (ADR 0013): `mpx/s` không phải `px/s` — tốc độ đo được không
        // được kết luận "khớp" dưới nhãn mpx/s.
        let text = r#"[oracle]
id = "O-994"
name = "Tốc độ người chơi"
feature = "V1-037"
source = "vn_runtime"
status = "candidate"
claims = ["R-043"]
evidence = ["docs/specification/gameplay/RULES_V1.md"]
quantity = "player_speed"
unit = "mpx/s"
expected = 167
tolerance = 5

[oracle.check]
kind = "scenario"
scenario = "fixtures/scenarios/movement/trapped_moves_slowly.toml"
measure = "player_speed"
player = 0
"#;
        let err = validate_text(text).expect_err("đơn vị sai phải là lỗi");
        let message = err.to_string();
        assert!(
            message.contains("px/s") && message.contains("mpx/s"),
            "lỗi phải nêu cả đơn vị đúng và đơn vị khai: {message}"
        );
    }

    #[test]
    fn rule_unit_mismatch_is_rejected() {
        // Đơn vị của `rule` là đơn vị profile khai — oracle khai khác là lỗi metadata.
        let text = r#"[oracle]
id = "O-993"
name = "Thời hạn vòng 120 s"
feature = "V1-037"
source = "vn_runtime"
status = "confirmed"
claims = ["R-010"]
evidence = ["docs/specification/gameplay/RULES_V1.md"]
quantity = "round_duration"
unit = "s"
expected = 120
tolerance = 0
profile = "data/rules/v1-dev.toml"

[oracle.check]
kind = "rule"
rule = "round.duration_ms"
"#;
        let err = validate_text(text).expect_err("đơn vị sai phải là lỗi");
        let message = err.to_string();
        assert!(
            message.contains("ms") && message.contains("`s`"),
            "lỗi phải nêu cả đơn vị đúng và đơn vị khai: {message}"
        );
    }

    #[test]
    fn every_oracle_unit_matches_its_measurement() {
        // Mọi oracle trong repo phải khai đúng đơn vị của phép đo — validator chặn sai nên
        // `load_dir` thành công là bằng chứng đơn vị khớp.
        let dir = root().join(ORACLE_DIR);
        let oracles = load_dir(&dir, &root()).expect("mọi file oracle phải hợp lệ");
        for oracle in &oracles {
            let required = match &oracle.check {
                Check::Rule { rule } => {
                    let profile = oracle.profile.as_ref().expect("rule cần profile");
                    rule_unit(&root(), profile, rule).expect("profile có khoá rule")
                }
                Check::Scenario { measure, .. } => measure.unit().to_owned(),
                Check::Distribution { .. } => DISTRIBUTION_UNIT.to_owned(),
            };
            assert_eq!(
                oracle.unit, required,
                "{}: đơn vị phải là `{required}`",
                oracle.id
            );
        }
    }
}
