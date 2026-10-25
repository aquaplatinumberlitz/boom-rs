"""Check that rules profiles follow the rule-source precedence of ADR 0013.

ADR 0013 (`docs/decisions/0013-rule-source-precedence.md`) orders the evidence of one
quantity (same quantity, same unit, same build/context): KR 00967 `confirmed` ->
VN VBNB_116 `confirmed` -> BnB `5c108e8` chosen as `design` -> project's own
`design`. `candidate` (KR/VN, web, OSS, emu_choice) never overrides a confirmed
tier. A profile key that uses a weaker source than the current choice is an error,
and every value must point at the current selection claim of RULES_V1 §11.

The standard source is RULES_V1 §11: the per-key table "Bảng lựa chọn theo khóa"
binds source, status, quantity, unit and build/context **for every profile key**.
The tier is checked from that per-key metadata, never from a merged claim row:
a claim row may cover several keys with different tiers (R-111, R-112, R-116),
and reading the whole row would let one key's confirmed tier vouch for another's
weaker one. The expectation table in this module is the machine-readable form of
ADR 0013 + RULES_V1 §11; it is cross-checked against the per-key metadata, so
drift between the table, the claim row and the profile is reported, not hidden.

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

# Ngữ cảnh/build mà một đại lượng áp dụng (ADR 0013: chỉ so cùng đại lượng, cùng
# đơn vị, cùng ngữ cảnh và build/profile). Dùng để ràng buộc build theo từng khóa.
BUILD_CONTEXTS = ("KR 00967", "VN VBNB_116", "BnB 5c108e8", "V1 design")

# Bảng lựa chọn theo khóa trong RULES_V1 §11: metadata theo từng khóa profile.
SELECTION_HEADING = "### Bảng lựa chọn theo khóa (máy-đọc)"


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
    """12 logical statpoint keys: VN named-row/consumer evidence, R-182."""
    out: list[Quantity] = []
    for character in ("c00", "c01"):
        for stat in ("speed", "bombs", "range"):
            for suffix in ("", "_max"):
                out.append(Quantity(
                    key=f"characters.{character}_{stat}{suffix}",
                    quantity="character_stat_level",
                    unit="level",
                    context="VN VBNB_116 logical statpoint; physical conversion remains DESIGN",
                    tier="vn_confirmed",
                    sources=("vn_runtime",),
                    status="confirmed",
                    selection="R-182",
                    support=("R-180", "R-175"),
                    limits=(("R-182", "logical statpoint, not native FPS/px/s or all-mode effective cap"),),
                    forbidden=(("R-105", "BnB template superseded"),
                               ("R-117", "historical KR candidate selection superseded by VN consumers")),
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
            context="V1 design; hiệu chuẩn curve VN R-183, không native literal/FPS",
            tier="project_design",
            sources=("design",),
            status="design",
            selection="R-184",
            support=("R-183",),
            limits=(("R-183", "same-actor points4..8 rates; other points/actors/modes DESIGN extrapolation"),),
            forbidden=(("R-043", "raw px/tick, checksum chưa xác thực"),),
        ),
        Quantity(
            key="player.trapped_speed_px_per_s",
            quantity="trapped_speed",
            unit="px/s",
            context="V1 design; hiệu chuẩn slow movement VN R-161/R-162",
            tier="project_design",
            sources=("design",),
            status="design",
            selection="R-162",
            support=("R-161",),
        ),
        Quantity(
            key="player.trap_to_death_ms",
            quantity="trap_to_death_timer",
            unit="ms",
            context="V1 design; hiệu chuẩn endpoint native VN theo R-159/R-160, quyết định24/10",
            tier="project_design",
            sources=("design",),
            status="design",
            selection="R-160",
            support=("R-159",),
            forbidden=(("R-026", "khoảng packet không được dùng làm timer nội bộ"),),
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
            context="V1 design; hiệu chuẩn cửa sổ VN runtime theo R-156/R-158, quyết định24/10",
            tier="project_design",
            sources=("design",),
            status="design",
            selection="R-158",
            support=("R-156",),
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
            context="VN VBNB_116 logical movementpoint; loaded max clamp",
            tier="vn_confirmed",
            sources=("vn_runtime",),
            status="confirmed",
            selection="R-185",
            support=("R-173", "R-182"),
            limits=(("R-185", "+1 logical point, not px/s or all-mode cap"),),
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
            context="V1 design (trình diễn, không phải luật chơi)",
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


@dataclass(frozen=True)
class Selection:
    """Metadata lựa chọn hiện hành của **một khóa profile**, đọc từ RULES_V1 §11.

    Đây là hạt nhỏ so sánh của ADR 0013: cùng đại lượng, cùng đơn vị, cùng
    ngữ cảnh/build. Ô "Nguồn / trạng thái" của một dòng claim gộp không đủ để
    kết luận tầng của từng khóa trong dòng đó.
    """

    key: str
    claim: str
    quantity: str
    unit: str
    source: str
    status: str
    context: str

    @property
    def pair(self) -> tuple[str, str]:
        return (self.source, self.status)

    def tier(self) -> str:
        return tier_of(self.source, self.status)

    def expected(self) -> str:
        return f"{TIER_LABEL[self.tier()]} ({self.source} / {self.status})"


@dataclass
class Profile:
    """Profile rules đọc từ TOML, đã kiểm schema."""

    path: Path
    profile_id: str
    release: bool
    rules: dict[str, dict] = field(default_factory=dict)


@dataclass
class RulesIndex:
    """Chỉ mục claim đọc từ RULES_V1: dòng bảng, metadata theo khóa, superseded."""

    rows: dict[str, dict[str, str]] = field(default_factory=dict)
    superseded: dict[str, set[str]] = field(default_factory=dict)
    selections: dict[str, Selection] = field(default_factory=dict)

    def source_status(self, claim: str) -> str:
        return self.rows.get(claim, {}).get("source_status", "")

    def text(self, claim: str) -> str:
        return self.rows.get(claim, {}).get("text", "")

    def selection(self, key: str) -> Selection | None:
        """Metadata theo khóa của RULES_V1 §11; `None` nếu khóa chưa được định danh."""
        return self.selections.get(key)


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
# Ô "Nguồn / trạng thái" của dòng claim: `src` / status, hoặc `src` + `src2` / status.
_PAIR_RE = re.compile(
    r"`(?P<source>[a-z_]+)`(?:\s*\+\s*`(?P<source2>[a-z_]+)`)?\s*/\s*"
    r"(?P<status>confirmed|candidate|design|unknown)")
_KEY_RE = re.compile(r"^[a-z0-9_]+(?:\.[a-z0-9_]+)+$")
_QUANTITY_RE = re.compile(r"^[a-z0-9_]+$")
_UNIT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9/%_]*$")


def _mentions(text: str, token: str) -> bool:
    """Có nhắc đúng từ khóa (nguồn/trạng thái) trong ô bảng không."""
    return re.search(rf"\b{re.escape(token)}\b", text) is not None


def build_contexts(text: str) -> frozenset[str]:
    """Tập build/ngữ cảnh mà một mô tả đại lượng nhắc tới."""
    return frozenset(token for token in BUILD_CONTEXTS if token in text)


def claim_pairs(cell: str) -> set[tuple[str, str]]:
    """Các cặp (nguồn, trạng thái) mà ô "Nguồn / trạng thái" của dòng claim nêu."""
    pairs: set[tuple[str, str]] = set()
    for match in _PAIR_RE.finditer(cell):
        pairs.add((match.group("source"), match.group("status")))
        if match.group("source2"):
            pairs.add((match.group("source2"), match.group("status")))
    return pairs


def parse_selection_table(text: str) -> dict[str, Selection]:
    """Đọc "Bảng lựa chọn theo khóa" của RULES_V1 §11: metadata theo từng khóa.

    Bảng này là nguồn chuẩn để so tầng theo khóa. Dòng thiếu cột, sai định danh
    hoặc cặp nguồn/trạng thái không hợp lệ đều là lỗi (fail closed), không suy đoán.
    """
    lines = text.splitlines()
    if SELECTION_HEADING not in lines:
        raise PrecedenceError(
            f"RULES_V1: thiếu bảng lựa chọn theo khóa ({SELECTION_HEADING}) ở §11")
    start = lines.index(SELECTION_HEADING)
    table: list[str] = []
    for line in lines[start + 1:]:
        stripped = line.strip()
        if not stripped.startswith("|"):
            if table:
                break
            continue
        table.append(stripped)
    if len(table) < 3:
        raise PrecedenceError("RULES_V1: bảng lựa chọn theo khóa rỗng hoặc thiếu dòng đầu")
    selections: dict[str, Selection] = {}
    for line in table[2:]:
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 7:
            raise PrecedenceError(
                "RULES_V1: dòng bảng lựa chọn theo khóa phải có 7 cột "
                f"(khóa, claim, đại lượng, đơn vị, nguồn, trạng thái, ngữ cảnh): {line}")
        key = cells[0].strip("`").strip()
        claim = cells[1].strip()
        quantity = cells[2].strip()
        unit = cells[3].strip()
        source = cells[4].strip("`").strip()
        status = cells[5].strip()
        context = cells[6].strip()
        if not _KEY_RE.fullmatch(key):
            raise PrecedenceError(f"RULES_V1: khóa không hợp lệ trong bảng lựa chọn theo khóa: {key!r}")
        if not _CLAIM_RE.fullmatch(claim):
            raise PrecedenceError(f"RULES_V1: khóa `{key}`: claim lựa chọn không hợp lệ: {claim!r}")
        if not _QUANTITY_RE.fullmatch(quantity):
            raise PrecedenceError(f"RULES_V1: khóa `{key}`: đại lượng không hợp lệ: {quantity!r}")
        if not _UNIT_RE.fullmatch(unit):
            raise PrecedenceError(f"RULES_V1: khóa `{key}`: đơn vị không hợp lệ: {unit!r}")
        if not context:
            raise PrecedenceError(f"RULES_V1: khóa `{key}`: thiếu ngữ cảnh/build")
        try:
            tier_of(source, status)
        except PrecedenceError as exc:
            raise PrecedenceError(f"RULES_V1: khóa `{key}`: {exc}") from exc
        if key in selections:
            raise PrecedenceError(f"RULES_V1: khóa `{key}` lặp trong bảng lựa chọn theo khóa")
        selections[key] = Selection(
            key=key, claim=claim, quantity=quantity, unit=unit,
            source=source, status=status, context=context)
    if not selections:
        raise PrecedenceError("RULES_V1: bảng lựa chọn theo khóa không có dòng nào")
    return selections


def parse_rules_v1(text: str) -> RulesIndex:
    """Đọc RULES_V1: dòng bảng theo claim ID, metadata theo khóa và superseded."""
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
    index.selections = parse_selection_table(text)
    if not index.rows:
        raise PrecedenceError("RULES_V1: no claim table found")
    if "## 11." not in text:
        raise PrecedenceError("RULES_V1: section 11 (current selection, ADR 0013) not found")
    return index


def check_expectations(index: RulesIndex, quantities: list[Quantity]) -> list[str]:
    """Đối chiếu bảng kỳ vọng với RULES_V1 **theo từng khóa**.

    Mỗi đại lượng được so với metadata theo khóa của RULES_V1 §11 ("Bảng lựa chọn
    theo khóa"): claim lựa chọn, đại lượng, đơn vị, nguồn, trạng thái và
    ngữ cảnh/build. Không đọc tầng từ ô của dòng claim gộp — dòng gộp che tầng
    của các khóa khác nhau, và ô đó không định danh được từng khóa.
    """
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
        chosen = index.selection(quantity.key)
        if chosen is None:
            errors.append(
                f"RULES_V1: bảng lựa chọn theo khóa chưa định danh `{quantity.key}` "
                f"(claim {selection}) — cần ràng buộc nguồn/trạng thái/đại lượng/đơn vị "
                "theo từng khóa; validator fail closed, không đoán theo ô claim gộp")
            continue
        cell = f"{chosen.source} / {chosen.status}"
        for source in quantity.sources:
            if not _mentions(cell, source):
                errors.append(
                    f"RULES_V1: dòng {selection} không nêu nguồn `{source}` "
                    f"(cần {quantity.expected()}) cho `{quantity.key}`")
        if not _mentions(cell, quantity.status):
            errors.append(
                f"RULES_V1: dòng {selection} không nêu trạng thái `{quantity.status}` "
                f"(cần {quantity.expected()}) cho `{quantity.key}`")
        tier = chosen.tier()
        if tier != quantity.tier:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}` dùng tầng {TIER_LABEL[tier]} "
                f"({chosen.source} / {chosen.status}) khác bảng kỳ vọng "
                f"{quantity.expected()} — bảng kỳ vọng lệch nguồn chuẩn theo khóa")
        if chosen.source not in quantity.sources:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}` dùng nguồn `{chosen.source}` khác "
                f"nguồn {'/'.join(quantity.sources)} của bảng kỳ vọng")
        if chosen.status != quantity.status:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}` dùng trạng thái `{chosen.status}` khác "
                f"trạng thái `{quantity.status}` của bảng kỳ vọng")
        if chosen.claim != selection:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}` trỏ claim {chosen.claim} khác claim "
                f"lựa chọn hiện hành {selection}")
        if chosen.quantity != quantity.quantity:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}`: đại lượng `{chosen.quantity}` khác "
                f"đại lượng `{quantity.quantity}` của bảng kỳ vọng")
        if chosen.unit != quantity.unit:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}`: đơn vị `{chosen.unit}` khác đơn vị "
                f"`{quantity.unit}` của bảng kỳ vọng")
        standard_contexts = build_contexts(chosen.context)
        table_contexts = build_contexts(quantity.context)
        if not standard_contexts:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}`: bảng lựa chọn theo khóa thiếu "
                "ngữ cảnh/build (ADR 0013 chỉ so cùng ngữ cảnh và build)")
        elif standard_contexts != table_contexts:
            errors.append(
                f"RULES_V1: khóa `{quantity.key}`: ngữ cảnh/build "
                f"{sorted(table_contexts)} của bảng kỳ vọng khác "
                f"{sorted(standard_contexts)} ghi ở bảng lựa chọn theo khóa")
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
    errors += _check_claim_rows(index, quantities)
    keys = [q.key for q in quantities]
    for key in sorted({k for k in keys if keys.count(k) > 1}):
        errors.append(f"bảng kỳ vọng: khóa `{key}` khai báo trùng")
    return errors


def _check_claim_rows(index: RulesIndex, quantities: list[Quantity]) -> list[str]:
    """Ô "Nguồn / trạng thái" của dòng claim phải khớp metadata theo từng khóa.

    Đây là chiều kiểm ngược: sửa ô của dòng claim gộp mà không sửa bảng lựa chọn
    theo khóa (hoặc ngược lại) phải bị báo, không được âm thầm dùng ô gộp để
    "xác nhận" tầng cho một khóa khác.
    """
    errors: list[str] = []
    known = {quantity.key for quantity in quantities}
    per_claim: dict[str, set[tuple[str, str]]] = {}
    for key, chosen in index.selections.items():
        if key not in known:
            errors.append(
                f"RULES_V1: bảng lựa chọn theo khóa có `{key}` chưa có trong bảng kỳ vọng")
            continue
        per_claim.setdefault(chosen.claim, set()).add(chosen.pair)
    for claim, pairs in sorted(per_claim.items()):
        row = index.rows.get(claim)
        if row is None:
            errors.append(
                f"RULES_V1: claim {claim} của bảng lựa chọn theo khóa không có trong RULES_V1")
            continue
        cell_pairs = claim_pairs(row["source_status"])
        if cell_pairs != pairs:
            errors.append(
                f"RULES_V1: dòng {claim} khai báo nguồn/trạng thái "
                f"{sorted(cell_pairs)} khác bảng lựa chọn theo khóa {sorted(pairs)} — "
                "ô claim gộp phải khớp từng khóa")
    return errors


def check_profile(profile: Profile, index: RulesIndex,
                  quantities: list[Quantity]) -> list[str]:
    """Kiểm một profile theo bảng kỳ vọng của ADR 0013.

    Tầng kỳ vọng lấy từ metadata **theo khóa** của RULES_V1 §11, không lấy từ ô của
    dòng claim gộp: hai khóa cùng một claim có thể khác tầng, và ô gộp không định
    danh được khóa nào đang ở tầng nào.
    """
    errors: list[str] = []
    by_key = {q.key: q for q in quantities}
    for key, rule in sorted(profile.rules.items()):
        quantity = by_key.get(key)
        if quantity is None:
            errors.append(
                f"{profile.path.name}: khóa `{key}` chưa khai báo đại lượng "
                "trong bảng kỳ vọng ADR 0013")
            continue
        chosen = index.selection(key)
        if chosen is None:
            errors.append(
                f"{profile.path.name}: `{key}`: RULES_V1 §11 chưa định danh metadata theo "
                "khóa (bảng lựa chọn theo khóa) — validator fail closed, không đoán")
            continue
        if rule["unit"] != chosen.unit:
            errors.append(
                f"{profile.path.name}: `{key}`: đơn vị `{rule['unit']}` khác đơn vị "
                f"`{chosen.unit}` của đại lượng `{chosen.quantity}` "
                "(ADR 0013: chỉ so cùng đại lượng và cùng đơn vị)")
        try:
            tier = tier_of(rule["source"], rule["status"])
        except PrecedenceError as exc:
            errors.append(f"{profile.path.name}: `{key}`: {exc}")
            continue
        expected_tier = chosen.tier()
        if (tier != expected_tier or rule["source"] != chosen.source
                or rule["status"] != chosen.status):
            weaker = TIER_RANK[tier] > TIER_RANK[expected_tier]
            reason = ("dùng tầng thấp hơn tầng ADR 0013 yêu cầu" if weaker else
                      "khác lựa chọn hiện hành (cần cập nhật bảng kỳ vọng + RULES_V1 §11)")
            errors.append(
                f"{profile.path.name}: `{key}`: nguồn/trạng thái `{rule['source']}` / "
                f"`{rule['status']}` (tầng {TIER_LABEL[tier]}) {reason}; "
                f"ADR 0013 chọn {chosen.expected()} theo RULES_V1 §11 cho khóa này")
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
