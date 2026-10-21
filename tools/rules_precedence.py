"""Check that rules profiles follow the rule-source precedence of ADR 0013.

ADR 0013 (`docs/decisions/0013-rule-source-precedence.md`) orders the evidence of one
quantity (same quantity, same unit, same build/context): KR 00967 `confirmed` ->
VN VBNB_116 `confirmed` -> BnB `5c108e8` chosen as `design` -> project's own
`design`. `candidate` (KR/VN, web, OSS, emu_choice) never overrides a confirmed
tier. A profile key that uses a weaker source than the current choice is an error,
and every value must point at the current selection claim of RULES_V1 §11.

The expectation table in this module is the machine-readable form of ADR 0013 +
RULES_V1 §11. Each entry names one quantity (never a similar-looking key), its
unit, its build/context, the required tier, the current selection claim, and the
claims that may be cited together with it. The validator cross-checks the table
against RULES_V1 (the claim exists, is not superseded, its row carries the expected
source/status and mentions the quantity), so RULES_V1 stays the single source of
truth and drift is reported instead of hidden.

Scope: engine profiles under `data/rules/` (AGENTS §3). Test fixtures
(`fixtures/rules/`, `crates/boom-qa/rules/`) are design-only profiles whose schema
is checked by the Rust loader; they are not release values and are not subject to
the release precedence.

Limits: the tool checks source tier, unit, claim linkage, supersession and explicit
exceptions. It does not compare the numeric value with the claim (the reviewer does
that), and it does not judge whether a claim is true. Supersession is read from
RULES_V1 in the `**superseded by R-xxx**` form (table cell or prose), the notation
used by RULES_V1 §10-§11.
"""
import argparse
import re
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RULES_DIR = ROOT / "data" / "rules"
RULES_V1 = ROOT / "docs" / "specification" / "gameplay" / "RULES_V1.md"

SCHEMA = 1
ROOT_KEYS = ("schema", "profile_id", "release")

SOURCES = ("kr_static", "kr_runtime", "vn_static", "vn_runtime", "web", "oss",
           "emu_choice", "design")
STATUSES = ("confirmed", "candidate", "design", "unknown")

# Tầng bằng chứng theo ADR 0013; số nhỏ = mạnh hơn. `candidate` yếu nhất vì
# ADR 0013 nói rõ candidate không đè tầng confirmed.
TIER_RANK = {
    "kr_confirmed": 0,
    "vn_confirmed": 1,
    "bnb_design": 2,
    "project_design": 3,
    "candidate": 4,
}
TIER_LABEL = {
    "kr_confirmed": "KR 00967 confirmed",
    "vn_confirmed": "VN VBNB_116 confirmed",
    "bnb_design": "BnB 5c108e8 design",
    "project_design": "project design",
    "candidate": "candidate",
}

# (source, status) hợp lệ trong profile -> tầng. `unknown` không mang giá trị
# nên không có ở đây (xem load_profile).
PAIR_TIERS = {
    ("kr_static", "confirmed"): "kr_confirmed",
    ("kr_runtime", "confirmed"): "kr_confirmed",
    ("vn_static", "confirmed"): "vn_confirmed",
    ("vn_runtime", "confirmed"): "vn_confirmed",
    ("oss", "design"): "bnb_design",
    ("design", "design"): "project_design",
    ("kr_static", "candidate"): "candidate",
    ("kr_runtime", "candidate"): "candidate",
    ("vn_static", "candidate"): "candidate",
    ("vn_runtime", "candidate"): "candidate",
    ("web", "candidate"): "candidate",
    ("oss", "candidate"): "candidate",
    ("emu_choice", "candidate"): "candidate",
}


class PrecedenceError(Exception):
    """Profile, RULES_V1 hoặc bảng kỳ vọng không đọc/không nhất quán được."""


@dataclass(frozen=True)
class Deviation:
    """Ngoại lệ tường minh so với chuỗi tầng của ADR 0013."""

    kind: str
    reason: str
    claims: tuple[str, ...]
    adr: str


@dataclass(frozen=True)
class Quantity:
    """Một đại lượng luật và lựa chọn hiện hành cho nó."""

    key: str
    quantity: str
    unit: str
    context: str
    tier: str
    sources: tuple[str, ...]
    status: str
    selection: str
    support: tuple[str, ...] = ()
    limits: tuple[tuple[str, str], ...] = ()
    forbidden: tuple[tuple[str, str], ...] = ()
    deviation: Deviation | None = None

    @property
    def group(self) -> str:
        """Nhóm khóa (`grid.tile_px` -> `grid.`); dùng để kiểm claim có nói về nhóm này."""
        return self.key.split(".", 1)[0] + "."

    @property
    def allowed_claims(self) -> tuple[str, ...]:
        return (self.selection,) + self.support

    def expected(self) -> str:
        return f"{TIER_LABEL[self.tier]} ({'/'.join(self.sources)} / {self.status})"


def tier_of(source: str, status: str) -> str:
    """Tầng của một cặp (nguồn, trạng thái); cặp không hợp lệ thì lỗi."""
    if source not in SOURCES:
        raise PrecedenceError(f"unknown source: {source}")
    if status not in STATUSES:
        raise PrecedenceError(f"unknown status: {status}")
    if status == "unknown":
        raise PrecedenceError("status unknown carries no value")
    tier = PAIR_TIERS.get((source, status))
    if tier is None:
        raise PrecedenceError(
            f"invalid source/status pair: {source} / {status} "
            "(confirmed chỉ từ KR/VN, design chỉ từ BnB hoặc design của dự án)")
    return tier


def _characters() -> list[Quantity]:
    """12 khóa chỉ số C00/C01: ngoại lệ candidate tường minh của ADR 0013."""
    out: list[Quantity] = []
    for character, attr_claim in (("c00", "R-040"), ("c01", "R-041")):
        for stat in ("speed", "bombs", "range"):
            for suffix in ("", "_max"):
                out.append(Quantity(
                    key=f"characters.{character}_{stat}{suffix}",
                    quantity="character_stat_level",
                    unit="level",
                    context="KR 00967 BomberAttr của build target; nghĩa trường f2–f7 còn candidate",
                    tier="candidate",
                    sources=("kr_static",),
                    status="candidate",
                    selection="R-117",
                    support=(attr_claim, "R-042"),
                    limits=(("R-042", "nghĩa trường f2–f7 (speed/bombs/range) còn candidate"),),
                    forbidden=(("R-105", "template nhân vật cân bằng BnB đã bị ADR 0013 thay"),),
                    deviation=Deviation(
                        kind="candidate",
                        reason=("ADR 0013: giữ 12 khóa C00/C01 theo BomberAttr của build target; "
                                "nghĩa trường f2–f7 còn candidate nên giá trị cũng là candidate; "
                                "không thay bằng template cân bằng BnB"),
                        claims=(attr_claim, "R-042", "R-117"),
                        adr="ADR 0013 §Ngoại lệ rõ ràng cho nhân vật",
                    ),
                ))
    return out


def expectations() -> list[Quantity]:
    """Bảng kỳ vọng: lựa chọn hiện hành của ADR 0013 / RULES_V1 §11 cho từng đại lượng."""
    quantities: list[Quantity] = [
        Quantity(
            key="sim.ticks_per_second",
            quantity="server_tick_rate",
            unit="Hz",
            context="BnB 5c108e8; KHÔNG phải bước client 30 ms",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-113",
            support=("R-100",),
            limits=(("R-060", "chỉ xác nhận pha GO, không xác nhận 3 s"),),
            forbidden=(("R-013", "đo bước client, không xác nhận tick server"),),
        ),
        Quantity(
            key="grid.tile_px",
            quantity="tile_size",
            unit="px",
            context="VN VBNB_116; chưa có claim confirmed KR cùng khóa",
            tier="vn_confirmed",
            sources=("vn_static",),
            status="confirmed",
            selection="R-110",
            support=("R-001",),
        ),
        Quantity(
            key="round.duration_ms",
            quantity="round_duration",
            unit="ms",
            context="VN VBNB_116 client; thời lượng round không phải timeout vận hành",
            tier="vn_confirmed",
            sources=("vn_runtime",),
            status="confirmed",
            selection="R-110",
            support=("R-010",),
            limits=(("R-011", "map có cờ thì server ghi đè thời lượng"),),
        ),
        Quantity(
            key="player.speed_px_per_s_per_level",
            quantity="speed_px_per_s_per_level",
            unit="px/s",
            context="BnB 5c108e8; cấp tốc độ không phải vận tốc",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-114",
            support=("R-102",),
            forbidden=(("R-043", "raw px/tick, checksum chưa xác thực"),),
        ),
        Quantity(
            key="player.trap_to_death_ms",
            quantity="trap_to_death_timer",
            unit="ms",
            context="BnB 5c108e8; timer nhốt nội bộ, KHÔNG phải khoảng packet",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-115",
            support=("R-103",),
            forbidden=(("R-026", "khoảng packet 6,359 ms có xử lý/gửi, không phải timer nội bộ"),),
        ),
        Quantity(
            key="bomb.fuse_ms",
            quantity="bomb_fuse",
            unit="ms",
            context="BnB 5c108e8; fuse gốc chưa khôi phục",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-115",
            support=("R-103",),
            forbidden=(("R-021", "fuse gốc unknown"), ("R-022", "giá trị emu tự đặt"),
                       ("R-024", "nguồn web: bom bị ném/đá nổ sau 3 s")),
        ),
        Quantity(
            key="bomb.flame_ms",
            quantity="bomb_flame",
            unit="ms",
            context="BnB 5c108e8; thời lượng thời gian, không phải nhịp trình diễn",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-115",
            support=("R-103",),
            forbidden=(("R-079", "timing trình diễn, không phải TTL damage"),),
        ),
        Quantity(
            key="item.fire_range_delta",
            quantity="item_fire_range_delta",
            unit="cell",
            context="VN VBNB_116 runtime",
            tier="vn_confirmed",
            sources=("vn_runtime",),
            status="confirmed",
            selection="R-111",
            support=("R-025",),
        ),
        Quantity(
            key="item.bomb_capacity_delta",
            quantity="item_bomb_capacity_delta",
            unit="bomb",
            context="VN VBNB_116 runtime",
            tier="vn_confirmed",
            sources=("vn_runtime",),
            status="confirmed",
            selection="R-111",
            support=("R-025",),
        ),
        Quantity(
            key="item.roller_speed_delta",
            quantity="item_roller_speed_delta",
            unit="level",
            context="BnB 5c108e8; R-045 candidate nên không nâng lên confirmed",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-111",
            support=("R-104",),
            limits=(("R-045", "claim candidate, chưa có bằng chứng consumer"),),
        ),
        Quantity(
            key="drop.brick_item_chance",
            quantity="drop_brick_item_chance",
            unit="permille",
            context="BnB 5c108e8; chuẩn hóa tương đối, KHÔNG phải xác suất gốc",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-112",
            support=("R-106",),
            limits=(("R-053", "xác suất gốc unknown"),),
        ),
    ]
    for item in ("bomb", "fire", "roller"):
        quantities.append(Quantity(
            key=f"drop.weight_{item}",
            quantity="drop_item_weight",
            unit="weight",
            context="VN VBNB_116 bảng ItemAttr; vai trò của bảng (ai dùng để rơi) còn unknown",
            tier="vn_confirmed",
            sources=("vn_static",),
            status="confirmed",
            selection="R-112",
            support=("R-051",),
            limits=(("R-052", "vai trò bảng drop còn unknown"),),
            deviation=Deviation(
                kind="limited_role",
                reason=("ADR 0013/R-112: số bảng VN confirmed nhưng vai trò bảng chưa xác nhận; "
                        "V1 chọn dùng số đó làm trọng số ba item Bomb/Fire/Roller"),
                claims=("R-051", "R-052", "R-112"),
                adr="ADR 0013 §11 (R-112)",
            ),
        ))
    quantities += _characters()
    quantities += [
        Quantity(
            key="presentation.frame_duration_ms",
            quantity="presentation_frame_duration",
            unit="ms",
            context="Thiết kế V1 (trình diễn, không phải luật chơi)",
            tier="project_design",
            sources=("design",),
            status="design",
            selection="R-118",
            support=("R-085",),
            limits=(("R-079", "pack client gốc không mang trường thời gian"),),
        ),
        Quantity(
            key="player.collision_half_px",
            quantity="collision_box_half",
            unit="px",
            context="BnB 5c108e8",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-116",
            support=("R-107",),
            limits=(("R-028", "không xác nhận khoảng cách chạm"),),
        ),
        Quantity(
            key="player.corner_assist_px",
            quantity="corner_assist",
            unit="px",
            context="BnB 5c108e8",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-116",
            support=("R-107",),
        ),
        Quantity(
            key="player.trap_touch_distance_px",
            quantity="trap_touch_distance",
            unit="px",
            context="BnB 5c108e8; khoảng cách chạm người bị nhốt, không phải bán kính nổ",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-116",
            support=("R-108",),
        ),
        Quantity(
            key="round.warmup_ms",
            quantity="round_warmup",
            unit="ms",
            context="BnB 5c108e8; thời gian GO, không phải thời lượng round",
            tier="bnb_design",
            sources=("oss",),
            status="design",
            selection="R-113",
            support=("R-100",),
        ),
    ]
    return quantities


@dataclass
class Profile:
    """Profile rules đọc từ TOML, đã kiểm schema."""

    path: Path
    profile_id: str
    release: bool
    rules: dict[str, dict] = field(default_factory=dict)


@dataclass
class RulesIndex:
    """Chỉ mục claim đọc từ RULES_V1: dòng bảng và quan hệ superseded."""

    rows: dict[str, dict[str, str]] = field(default_factory=dict)
    superseded: dict[str, set[str]] = field(default_factory=dict)

    def source_status(self, claim: str) -> str:
        return self.rows.get(claim, {}).get("source_status", "")

    def text(self, claim: str) -> str:
        return self.rows.get(claim, {}).get("text", "")


def parse_profile(text: str, path: Path | str = "<memory>") -> Profile:
    """Kiểm schema một profile rules cho sẵn; lỗi parse/schema thì ném PrecedenceError."""
    try:
        raw = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        raise PrecedenceError(f"{path}: cannot parse TOML: {exc}") from exc
    for key in ROOT_KEYS:
        if key not in raw:
            raise PrecedenceError(f"{path}: missing root key `{key}`")
    if raw["schema"] != SCHEMA:
        raise PrecedenceError(f"{path}: unsupported schema {raw['schema']} (need {SCHEMA})")
    if not isinstance(raw["profile_id"], str) or not raw["profile_id"]:
        raise PrecedenceError(f"{path}: profile_id must be a non-empty string")
    if not isinstance(raw["release"], bool):
        raise PrecedenceError(f"{path}: release must be a boolean")
    profile = Profile(path=Path(path), profile_id=raw["profile_id"], release=raw["release"])
    for group, entries in raw.items():
        if group in ROOT_KEYS:
            continue
        if not isinstance(entries, dict):
            raise PrecedenceError(f"{path}: `{group}` must be a table of rules")
        for name, rule in entries.items():
            key = f"{group}.{name}"
            if not isinstance(rule, dict):
                raise PrecedenceError(f"{path}: rule `{key}` must be a table")
            for field_name in ("unit", "source", "status"):
                if field_name not in rule:
                    raise PrecedenceError(f"{path}: rule `{key}` missing `{field_name}`")
            value = rule.get("value")
            if value is not None and (isinstance(value, bool) or not isinstance(value, int)):
                raise PrecedenceError(f"{path}: rule `{key}` value must be an integer")
            claims = rule.get("claims", [])
            if not isinstance(claims, list) or not all(isinstance(c, str) for c in claims):
                raise PrecedenceError(f"{path}: rule `{key}` claims must be a list of strings")
            if rule["status"] == "unknown":
                if value is not None:
                    raise PrecedenceError(
                        f"{path}: rule `{key}` status unknown must not carry a value")
            else:
                if value is None:
                    raise PrecedenceError(f"{path}: rule `{key}` missing value")
                tier_of(rule["source"], rule["status"])
            if rule["source"] != "design" and not claims:
                raise PrecedenceError(f"{path}: rule `{key}` must point at a claim")
            if profile.release and rule["status"] in ("candidate", "unknown"):
                raise PrecedenceError(
                    f"{path}: release profile must not ship status {rule['status']} (`{key}`)")
            profile.rules[key] = dict(rule)
    return profile


def load_profile(path: Path) -> Profile:
    """Đọc profile rules từ đĩa rồi kiểm schema."""
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise PrecedenceError(f"profile not found: {path}") from exc
    return parse_profile(text, path)


_CLAIM_RE = re.compile(r"R-\d{3}")
_ROW_SUPERSEDED_RE = re.compile(r"\*\*superseded by ([^*]+)\*\*")
_PROSE_SUPERSEDED_RE = re.compile(
    r"(R-\d{3}(?:/R-\d{3})*)[^|\n]{0,80}?\*\*superseded by ([^*]+)\*\*")


def _mentions(text: str, token: str) -> bool:
    """Có nhắc đúng từ khóa (nguồn/trạng thái) trong ô bảng không."""
    return re.search(rf"\b{re.escape(token)}\b", text) is not None


def parse_rules_v1(text: str) -> RulesIndex:
    """Đọc RULES_V1: dòng bảng theo claim ID và quan hệ superseded."""
    index = RulesIndex()
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("|"):
            cells = [c.strip() for c in stripped.strip("|").split("|")]
            if len(cells) < 5 or not _CLAIM_RE.fullmatch(cells[0]):
                continue
            index.rows[cells[0]] = {
                "claim": cells[1],
                "value": cells[2],
                "source_status": cells[3],
                "locator": cells[4],
                "text": stripped,
            }
            for match in _ROW_SUPERSEDED_RE.finditer(stripped):
                index.superseded.setdefault(cells[0], set()).update(
                    _CLAIM_RE.findall(match.group(1)))
        else:
            for match in _PROSE_SUPERSEDED_RE.finditer(stripped):
                superseders = set(_CLAIM_RE.findall(match.group(2)))
                for claim in match.group(1).split("/"):
                    index.superseded.setdefault(claim, set()).update(superseders)
    if not index.rows:
        raise PrecedenceError("RULES_V1: no claim table found")
    if "## 11." not in text:
        raise PrecedenceError("RULES_V1: section 11 (current selection, ADR 0013) not found")
    return index


def check_expectations(index: RulesIndex, quantities: list[Quantity]) -> list[str]:
    """Đối chiếu bảng kỳ vọng với RULES_V1: claim tồn tại, chưa superseded, đúng nguồn."""
    errors: list[str] = []
    for quantity in quantities:
        selection = quantity.selection
        if selection not in index.rows:
            errors.append(
                f"RULES_V1: claim lựa chọn hiện hành {selection} của `{quantity.key}` "
                "không có trong RULES_V1")
            continue
        if selection in index.superseded:
            superseders = ", ".join(sorted(index.superseded[selection]))
            errors.append(
                f"RULES_V1: claim lựa chọn hiện hành {selection} của `{quantity.key}` "
                f"đã superseded bởi {superseders}")
        row = index.rows[selection]
        cell = row["source_status"]
        for source in quantity.sources:
            if not _mentions(cell, source):
                errors.append(
                    f"RULES_V1: dòng {selection} không nêu nguồn `{source}` "
                    f"(cần {quantity.expected()}) cho `{quantity.key}`")
        if not _mentions(cell, quantity.status):
            errors.append(
                f"RULES_V1: dòng {selection} không nêu trạng thái `{quantity.status}` "
                f"(cần {quantity.expected()}) cho `{quantity.key}`")
        if quantity.group not in row["text"]:
            errors.append(
                f"RULES_V1: dòng {selection} không nói về nhóm `{quantity.group}` "
                f"của `{quantity.key}`")
        for claim in quantity.allowed_claims + tuple(c for c, _ in quantity.limits):
            if claim not in index.rows:
                errors.append(
                    f"RULES_V1: claim {claim} của `{quantity.key}` không có trong RULES_V1")
        if quantity.tier == "candidate" and quantity.deviation is None:
            errors.append(
                f"`{quantity.key}`: tầng candidate phải có ngoại lệ tường minh (ADR 0013)")
        if quantity.deviation is not None:
            if not quantity.deviation.reason.strip():
                errors.append(f"`{quantity.key}`: ngoại lệ thiếu lý do")
            if not quantity.deviation.claims:
                errors.append(f"`{quantity.key}`: ngoại lệ thiếu claim")
            for claim in quantity.deviation.claims:
                if claim not in index.rows:
                    errors.append(
                        f"RULES_V1: claim {claim} của ngoại lệ `{quantity.key}` "
                        "không có trong RULES_V1")
    keys = [q.key for q in quantities]
    for key in sorted({k for k in keys if keys.count(k) > 1}):
        errors.append(f"bảng kỳ vọng: khóa `{key}` khai báo trùng")
    return errors


def check_profile(profile: Profile, index: RulesIndex,
                  quantities: list[Quantity]) -> list[str]:
    """Kiểm một profile theo bảng kỳ vọng của ADR 0013."""
    errors: list[str] = []
    by_key = {q.key: q for q in quantities}
    for key, rule in sorted(profile.rules.items()):
        quantity = by_key.get(key)
        if quantity is None:
            errors.append(
                f"{profile.path.name}: khóa `{key}` chưa khai báo đại lượng "
                "trong bảng kỳ vọng ADR 0013")
            continue
        if rule["unit"] != quantity.unit:
            errors.append(
                f"{profile.path.name}: `{key}`: đơn vị `{rule['unit']}` khác đơn vị "
                f"`{quantity.unit}` của đại lượng `{quantity.quantity}` "
                "(ADR 0013: chỉ so cùng đại lượng và cùng đơn vị)")
        try:
            tier = tier_of(rule["source"], rule["status"])
        except PrecedenceError as exc:
            errors.append(f"{profile.path.name}: `{key}`: {exc}")
            continue
        if (tier != quantity.tier or rule["source"] not in quantity.sources
                or rule["status"] != quantity.status):
            weaker = TIER_RANK[tier] > TIER_RANK[quantity.tier]
            reason = ("dùng tầng thấp hơn tầng ADR 0013 yêu cầu" if weaker else
                      "khác lựa chọn hiện hành (cần cập nhật bảng kỳ vọng + RULES_V1 §11)")
            errors.append(
                f"{profile.path.name}: `{key}`: nguồn/trạng thái `{rule['source']}` / "
                f"`{rule['status']}` (tầng {TIER_LABEL[tier]}) {reason}; "
                f"ADR 0013 chọn {quantity.expected()}")
        claims = list(rule.get("claims", []))
        if quantity.selection not in claims:
            errors.append(
                f"{profile.path.name}: `{key}`: thiếu claim lựa chọn hiện hành "
                f"{quantity.selection} (RULES_V1 §11)")
        forbidden = dict(quantity.forbidden)
        for claim in claims:
            if claim in forbidden:
                errors.append(
                    f"{profile.path.name}: `{key}`: claim {claim} thuộc ngữ cảnh khác — "
                    f"{forbidden[claim]}")
            elif claim not in quantity.allowed_claims:
                errors.append(
                    f"{profile.path.name}: `{key}`: claim {claim} không thuộc đại lượng "
                    f"`{quantity.quantity}` (được trỏ: {', '.join(quantity.allowed_claims)})")
    for quantity in quantities:
        if quantity.key not in profile.rules:
            errors.append(
                f"{profile.path.name}: thiếu khóa `{quantity.key}` "
                f"(đại lượng `{quantity.quantity}`)")
    return errors


def check_repository(rules_dir: Path = RULES_DIR, rules_v1: Path = RULES_V1) -> list[str]:
    """Kiểm toàn bộ profile trong `rules_dir` theo RULES_V1; trả về danh sách lỗi."""
    try:
        text = rules_v1.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise PrecedenceError(f"RULES_V1 not found: {rules_v1}") from exc
    index = parse_rules_v1(text)
    quantities = expectations()
    errors = check_expectations(index, quantities)
    profiles = sorted(rules_dir.glob("*.toml"))
    if not profiles:
        raise PrecedenceError(f"no rules profile found in {rules_dir}")
    for path in profiles:
        profile = load_profile(path)
        errors += check_profile(profile, index, quantities)
    return errors


def _summary(quantities: list[Quantity], profiles: list[Path], index: RulesIndex) -> list[str]:
    """Dòng tóm tắt: tầng dùng, ngoại lệ tường minh và giới hạn áp dụng, gom theo đại lượng."""
    tiers: dict[str, int] = {}
    for path in profiles:
        for rule in load_profile(path).rules.values():
            tier = tier_of(rule["source"], rule["status"])
            tiers[tier] = tiers.get(tier, 0) + 1
    lines = [
        f"Rules precedence OK: {len(quantities)} quantities, {len(profiles)} profile(s); "
        + ", ".join(f"{TIER_LABEL[tier]}={tiers.get(tier, 0)}" for tier in TIER_RANK)
    ]
    grouped: dict[str, list[Quantity]] = {}
    for quantity in quantities:
        grouped.setdefault(quantity.quantity, []).append(quantity)
    for quantity_id, group in grouped.items():
        keys = ", ".join(q.key for q in group)
        deviation = group[0].deviation
        if deviation is not None:
            lines.append(f"  exception [{deviation.kind}] {quantity_id} ({keys}): "
                         f"{deviation.reason} — {deviation.adr}")
        limits = []
        for q in group:
            for claim, reason in q.limits:
                if (claim, reason) not in limits:
                    limits.append((claim, reason))
        for claim, reason in limits:
            lines.append(f"  limit {quantity_id}: {claim} — {reason}")
    for path in profiles:
        history: dict[str, list[str]] = {}
        for key, rule in sorted(load_profile(path).rules.items()):
            for claim in rule.get("claims", []):
                if claim in index.superseded:
                    history.setdefault(claim, []).append(key)
        for claim, keys in history.items():
            superseders = ", ".join(sorted(index.superseded[claim]))
            lines.append(f"  history {path.name}: {claim} (superseded by {superseders}) được trỏ "
                         f"làm lịch sử ở {', '.join(keys)} — claim lựa chọn hiện hành vẫn được trỏ")
    return lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rules-dir", type=Path, default=RULES_DIR,
                        help="thư mục profile rules của engine")
    parser.add_argument("--rules-v1", type=Path, default=RULES_V1,
                        help="file RULES_V1 làm nguồn chuẩn claim")
    args = parser.parse_args()
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    try:
        errors = check_repository(args.rules_dir, args.rules_v1)
    except PrecedenceError as exc:
        print(f"rules precedence: cannot validate: {exc}", file=sys.stderr)
        return 1
    if errors:
        print("rules precedence: ADR 0013 violations:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    profiles = sorted(args.rules_dir.glob("*.toml"))
    for line in _summary(expectations(), profiles,
                         parse_rules_v1(args.rules_v1.read_text(encoding="utf-8"))):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
