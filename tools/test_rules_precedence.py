"""Integrity tests for the rule-source precedence validator (ADR 0013).

Synthetic profiles never touch the real ones: the fixture text is built from the
expectation table, and a single field is changed per case. The real repository
profiles are checked once as a regression baseline.

The cases for a **merged** claim row (R-111, R-112, R-116) are the point of the
per-key metadata of RULES_V1 §11: one claim may cover several keys with different
tiers, so a tier must never be read from the whole claim row.
"""
import subprocess
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from rules_precedence import (
    SELECTION_HEADING,
    PrecedenceError,
    check_expectations,
    check_profile,
    check_repository,
    expectations,
    load_profile,
    parse_profile,
    parse_rules_v1,
    tier_of,
)


ROOT = Path(__file__).resolve().parents[1]
RULES_V1 = ROOT / "docs" / "specification" / "gameplay" / "RULES_V1.md"
PROFILE = ROOT / "data" / "rules" / "v1-dev.toml"
TOOL = ROOT / "tools" / "rules_precedence.py"

# Dòng claim gộp R-111 và dòng "Bảng lựa chọn theo khóa" của cùng claim: một khóa
# được nâng lên `kr_static` / confirmed còn khóa anh em giữ tầng cũ.
CLAIM_R111_ROW = (
    "| R-111 | `item.bomb_capacity_delta`, `item.fire_range_delta` dùng VN; "
    "`item.roller_speed_delta` giữ BnB | +1 bomb; +1 cell; +1 level | "
    "Bomb/Fire `vn_runtime` / confirmed; Roller `oss` / design |")
CLAIM_R111_ROW_UPGRADED = (
    "| R-111 | `item.bomb_capacity_delta`, `item.fire_range_delta` dùng VN; "
    "`item.roller_speed_delta` giữ BnB | +1 bomb; +1 cell; +1 level | "
    "Bomb/Fire `kr_static` + `vn_runtime` / confirmed; Roller `oss` / design |")
PER_KEY_BOMB_CAPACITY = (
    "| `item.bomb_capacity_delta` | R-111 | item_bomb_capacity_delta | bomb | "
    "`vn_runtime` | confirmed | VN VBNB_116 |")
PER_KEY_BOMB_CAPACITY_UPGRADED = (
    "| `item.bomb_capacity_delta` | R-111 | item_bomb_capacity_delta | bomb | "
    "`kr_static` | confirmed | KR 00967 |")

# Ô nguồn/trạng thái của dòng claim gộp R-116: sửa riêng ô này (không sửa bảng
# lựa chọn theo khóa) là cách tái lập finding "đối chiếu theo cả ô claim gộp".
CLAIM_R116_ROW = (
    "| R-116 | `player.collision_half_px`, `player.corner_assist_px`, "
    "`player.trap_touch_distance_px` giữ BnB | 13/12/28 px | `oss` / design |")
CLAIM_R116_ROW_TAMPERED = (
    "| R-116 | `player.collision_half_px`, `player.corner_assist_px`, "
    "`player.trap_touch_distance_px` giữ BnB | 13/12/28 px | "
    "player.collision_half_px: kr_static / confirmed; assist/touch: oss / design |")
PER_KEY_COLLISION = (
    "| `player.collision_half_px` | R-116 | collision_box_half | px | `oss` | "
    "design | BnB 5c108e8 |")


def rules_v1_text() -> str:
    return RULES_V1.read_text(encoding="utf-8")


def replace_once(text: str, old: str, new: str) -> str:
    """Thay đúng một lần; thất bại thì báo rõ đoạn cần sửa."""
    if old not in text:
        raise AssertionError(f"không tìm thấy đoạn cần sửa trong RULES_V1: {old!r}")
    return text.replace(old, new, 1)


def profile_text(overrides=None, extra=""):
    """Profile hợp lệ theo bảng kỳ vọng, rồi sửa vài trường theo `overrides`."""
    overrides = overrides or {}
    lines = ["schema = 1", 'profile_id = "synthetic"', "release = false", ""]
    for quantity in expectations():
        rule = {
            "value": 1,
            "unit": quantity.unit,
            "source": quantity.sources[0],
            "status": quantity.status,
            "claims": [quantity.selection, *quantity.support],
        }
        rule.update(overrides.get(quantity.key, {}))
        lines.append(f"[{quantity.key}]")
        if rule.get("value") is not None:
            lines.append(f"value = {rule['value']}")
        lines.append(f"unit = \"{rule['unit']}\"")
        lines.append(f"source = \"{rule['source']}\"")
        lines.append(f"status = \"{rule['status']}\"")
        if rule.get("claims"):
            claims = ", ".join(f'"{claim}"' for claim in rule["claims"])
            lines.append(f"claims = [{claims}]")
        lines.append("")
    return "\n".join(lines) + extra


class PrecedenceTests(unittest.TestCase):
    def setUp(self):
        self.index = parse_rules_v1(RULES_V1.read_text(encoding="utf-8"))
        self.quantities = expectations()

    def errors(self, overrides=None, extra=""):
        profile = parse_profile(profile_text(overrides, extra), "synthetic.toml")
        return check_profile(profile, self.index, self.quantities)

    def test_repository_profiles_follow_adr_0013(self):
        self.assertEqual(check_repository(), [])

    def test_matching_tier_passes(self):
        self.assertEqual(self.errors(), [])

    def test_lower_tier_source_is_rejected(self):
        errors = self.errors({"grid.tile_px": {"source": "oss", "status": "design"}})
        self.assertTrue(any("tầng thấp hơn" in error and "grid.tile_px" in error
                            for error in errors), errors)

    def test_candidate_cannot_override_confirmed(self):
        errors = self.errors({"grid.tile_px": {"source": "kr_static", "status": "candidate"}})
        self.assertTrue(any("grid.tile_px" in error for error in errors), errors)

    def test_claim_from_another_quantity_is_rejected(self):
        errors = self.errors({"sim.ticks_per_second": {"claims": ["R-113", "R-051"]}})
        self.assertTrue(any("không thuộc đại lượng" in error for error in errors), errors)

    def test_claim_of_another_context_is_rejected(self):
        errors = self.errors({"player.trap_to_death_ms": {"claims": ["R-115", "R-026"]}})
        self.assertTrue(any("ngữ cảnh khác" in error and "R-026" in error
                            for error in errors), errors)

    def test_client_step_is_not_a_server_tick(self):
        errors = self.errors({"sim.ticks_per_second": {"claims": ["R-113", "R-013"]}})
        self.assertTrue(any("R-013" in error for error in errors), errors)

    def test_profile_without_current_claim_is_rejected(self):
        errors = self.errors({"grid.tile_px": {"claims": ["R-001"]}})
        self.assertTrue(any("thiếu claim lựa chọn hiện hành R-110" in error
                            for error in errors), errors)

    def test_unit_mismatch_is_rejected(self):
        errors = self.errors({"grid.tile_px": {"unit": "cell"}})
        self.assertTrue(any("đơn vị" in error for error in errors), errors)

    def test_undeclared_key_is_rejected(self):
        errors = self.errors(extra='\n[extra.rule]\nvalue = 1\nunit = "px"\n'
                                    'source = "oss"\nstatus = "design"\n'
                                    'claims = ["R-110"]\n')
        self.assertTrue(any("chưa khai báo đại lượng" in error for error in errors), errors)

    def test_superseded_selection_claim_is_rejected(self):
        text = RULES_V1.read_text(encoding="utf-8")
        marked = text.replace("## 11. Ưu tiên nguồn hiện hành — ADR 0013",
                               "## 11. Ưu tiên nguồn hiện hành — ADR 0013\n\n"
                               "R-110 **superseded by R-900**;", 1)
        index = parse_rules_v1(marked)
        errors = check_expectations(index, self.quantities)
        self.assertTrue(any("R-110" in error and "superseded" in error
                            for error in errors), errors)

    def test_candidate_tier_requires_explicit_exception(self):
        quantity = next(q for q in self.quantities if q.tier == "candidate")
        without = [q if q.key != quantity.key else type(quantity)(
            key=q.key, quantity=q.quantity, unit=q.unit, context=q.context, tier=q.tier,
            sources=q.sources, status=q.status, selection=q.selection, support=q.support,
            limits=q.limits, forbidden=q.forbidden, deviation=None)
            for q in self.quantities]
        errors = check_expectations(self.index, without)
        self.assertTrue(any("ngoại lệ tường minh" in error for error in errors), errors)

    def test_exception_without_reason_is_rejected(self):
        quantity = next(q for q in self.quantities if q.deviation is not None)
        broken = type(quantity)(
            key=quantity.key, quantity=quantity.quantity, unit=quantity.unit,
            context=quantity.context, tier=quantity.tier, sources=quantity.sources,
            status=quantity.status, selection=quantity.selection, support=quantity.support,
            limits=quantity.limits, forbidden=quantity.forbidden,
            deviation=type(quantity.deviation)(kind="candidate", reason="  ",
                                              claims=quantity.deviation.claims,
                                              adr=quantity.deviation.adr))
        errors = check_expectations(self.index, [broken])
        self.assertTrue(any("ngoại lệ thiếu lý do" in error for error in errors), errors)

    def test_unknown_claim_in_table_is_rejected(self):
        quantity = next(q for q in self.quantities if q.tier == "candidate")
        broken = type(quantity)(
            key=quantity.key, quantity=quantity.quantity, unit=quantity.unit,
            context=quantity.context, tier=quantity.tier, sources=quantity.sources,
            status=quantity.status, selection="R-999", support=quantity.support,
            limits=quantity.limits, forbidden=quantity.forbidden, deviation=quantity.deviation)
        errors = check_expectations(self.index, [broken])
        self.assertTrue(any("R-999" in error for error in errors), errors)

    def test_table_source_must_match_rules_v1_row(self):
        quantity = next(q for q in self.quantities if q.key == "grid.tile_px")
        broken = type(quantity)(
            key=quantity.key, quantity=quantity.quantity, unit=quantity.unit,
            context=quantity.context, tier=quantity.tier, sources=("web",),
            status=quantity.status, selection=quantity.selection, support=quantity.support,
            limits=quantity.limits, forbidden=quantity.forbidden, deviation=quantity.deviation)
        errors = check_expectations(self.index, [broken])
        self.assertTrue(any("không nêu nguồn `web`" in error for error in errors), errors)

    def test_parser_errors_fail(self):
        with self.assertRaises(PrecedenceError):
            parse_profile("schema = 1\nprofile_id = 'x'\nrelease = false\n[bad\n", "bad.toml")
        with self.assertRaises(PrecedenceError):
            parse_profile("schema = 1\nprofile_id = 'x'\n", "missing.toml")
        with self.assertRaises(PrecedenceError):
            parse_rules_v1("## 1. Không có bảng claim\n")

    def test_unknown_status_carries_no_value(self):
        with self.assertRaises(PrecedenceError):
            parse_profile("schema = 1\nprofile_id = 'x'\nrelease = false\n\n"
                          "[grid.tile_px]\nvalue = 40\nunit = 'px'\n"
                          "source = 'vn_static'\nstatus = 'unknown'\n", "u.toml")

    def test_invalid_source_status_pair_fails(self):
        with self.assertRaises(PrecedenceError):
            tier_of("oss", "confirmed")
        with self.assertRaises(PrecedenceError):
            tier_of("web", "design")

    def test_cli_fails_on_tampered_profile(self):
        with tempfile.TemporaryDirectory(prefix="rules-precedence-") as temp:
            rules_dir = Path(temp)
            (rules_dir / "synthetic.toml").write_text(
                profile_text({"grid.tile_px": {"source": "oss", "status": "design"}}),
                encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(TOOL), "--rules-dir", str(rules_dir),
                 "--rules-v1", str(RULES_V1)],
                capture_output=True, text=True, cwd=ROOT, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("grid.tile_px", result.stderr)

    def test_cli_passes_on_repository_profiles(self):
        result = subprocess.run(
            [sys.executable, str(TOOL)], capture_output=True, text=True, cwd=ROOT, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Rules precedence OK", result.stdout)


class MergedClaimPerKeyTests(unittest.TestCase):
    """Tầng phải được so **theo từng khóa**, không theo ô của dòng claim gộp.

    Đây là test tái lập finding "Reviewer: Codex" (P1) trên PR #154: RULES_V1 có
    thể đã chọn tầng `confirmed` cao hơn cho đúng khóa, nhưng validator đọc cả ô
    của dòng claim gộp nên vẫn báo xanh khi bảng Python/profile còn ở tầng thấp.
    """

    def setUp(self):
        self.index = parse_rules_v1(rules_v1_text())
        self.quantities = expectations()

    def upgraded_rules_v1(self):
        """RULES_V1 nâng `item.bomb_capacity_delta` lên KR confirmed, khóa anh em giữ tầng cũ."""
        text = rules_v1_text()
        text = replace_once(text, PER_KEY_BOMB_CAPACITY, PER_KEY_BOMB_CAPACITY_UPGRADED)
        text = replace_once(text, CLAIM_R111_ROW, CLAIM_R111_ROW_UPGRADED)
        return parse_rules_v1(text)

    def test_merged_claim_upgrade_breaks_old_table_and_profile(self):
        index = self.upgraded_rules_v1()
        errors = check_expectations(index, self.quantities)
        self.assertTrue(any("item.bomb_capacity_delta" in error for error in errors), errors)
        profile = load_profile(PROFILE)
        profile_errors = check_profile(profile, index, self.quantities)
        self.assertTrue(any("item.bomb_capacity_delta" in error
                            and "tầng thấp hơn" in error
                            for error in profile_errors), profile_errors)

    def test_merged_claim_upgrade_passes_with_updated_table_and_profile(self):
        index = self.upgraded_rules_v1()
        updated = [replace(q, tier="kr_confirmed", sources=("kr_static",),
                           status="confirmed", context="KR 00967 runtime")
                   if q.key == "item.bomb_capacity_delta" else q
                   for q in self.quantities]
        self.assertEqual(check_expectations(index, updated), [])
        profile = parse_profile(
            profile_text({"item.bomb_capacity_delta": {"source": "kr_static",
                                                       "status": "confirmed"}}),
            "updated.toml")
        self.assertEqual(check_profile(profile, index, updated), [])

    def test_sibling_key_does_not_inherit_claim_row_tier(self):
        """Khóa anh em cùng claim R-111 không được hưởng tầng của khóa khác."""
        index = self.upgraded_rules_v1()
        profile = parse_profile(
            profile_text({"item.fire_range_delta": {"source": "kr_static",
                                                    "status": "confirmed"}}),
            "sibling.toml")
        errors = check_profile(profile, index, self.quantities)
        self.assertTrue(any("item.fire_range_delta" in error for error in errors), errors)

    def test_claim_row_source_cell_must_match_per_key_table(self):
        """Sửa riêng ô nguồn/trạng thái của dòng claim gộp thì phải báo lỗi."""
        text = replace_once(rules_v1_text(), CLAIM_R116_ROW, CLAIM_R116_ROW_TAMPERED)
        index = parse_rules_v1(text)
        errors = check_expectations(index, self.quantities)
        self.assertTrue(any("R-116" in error for error in errors), errors)

    def test_cli_fails_on_tampered_claim_row(self):
        text = replace_once(rules_v1_text(), CLAIM_R116_ROW, CLAIM_R116_ROW_TAMPERED)
        with tempfile.TemporaryDirectory(prefix="rules-precedence-") as temp:
            rules_v1 = Path(temp) / "RULES_V1.md"
            rules_v1.write_text(text, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(TOOL), "--rules-v1", str(rules_v1)],
                capture_output=True, text=True, cwd=ROOT, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("R-116", result.stderr)

    def test_key_without_per_key_metadata_fails_closed(self):
        """Khóa chưa được định danh trong bảng lựa chọn theo khóa thì fail closed."""
        text = rules_v1_text().replace(PER_KEY_COLLISION + "\n", "", 1)
        index = parse_rules_v1(text)
        errors = check_expectations(index, self.quantities)
        self.assertTrue(any("player.collision_half_px" in error
                            and "chưa định danh" in error for error in errors), errors)
        profile = load_profile(PROFILE)
        profile_errors = check_profile(profile, index, self.quantities)
        self.assertTrue(any("player.collision_half_px" in error
                            and "chưa định danh" in error for error in profile_errors),
                        profile_errors)

    def test_missing_per_key_table_fails_closed(self):
        text = rules_v1_text().replace(SELECTION_HEADING, "### Bảng lựa chọn theo khóa (bỏ)")
        with self.assertRaises(PrecedenceError):
            parse_rules_v1(text)

    def test_invalid_pair_in_per_key_table_fails_closed(self):
        text = replace_once(
            rules_v1_text(), PER_KEY_COLLISION,
            "| `player.collision_half_px` | R-116 | collision_box_half | px | "
            "`web` | confirmed | BnB 5c108e8 |")
        with self.assertRaises(PrecedenceError):
            parse_rules_v1(text)

    def test_per_key_unit_quantity_and_context_are_checked(self):
        text = replace_once(
            rules_v1_text(), PER_KEY_COLLISION,
            "| `player.collision_half_px` | R-116 | collision_box_half | cell | "
            "`oss` | design | BnB 5c108e8 |")
        errors = check_expectations(parse_rules_v1(text), self.quantities)
        self.assertTrue(any("player.collision_half_px" in error and "đơn vị" in error
                            for error in errors), errors)
        text = replace_once(
            rules_v1_text(), PER_KEY_COLLISION,
            "| `player.collision_half_px` | R-116 | collision_box_half | px | "
            "`oss` | design | VN VBNB_116 |")
        errors = check_expectations(parse_rules_v1(text), self.quantities)
        self.assertTrue(any("player.collision_half_px" in error
                            and "ngữ cảnh/build" in error for error in errors), errors)

    def test_per_key_table_is_the_standard_source_of_tier(self):
        """Tầng kỳ vọng lấy từ metadata theo khóa, không từ tầng của ô claim gộp."""
        chosen = self.index.selection("player.collision_half_px")
        self.assertIsNotNone(chosen)
        self.assertEqual(chosen.claim, "R-116")
        self.assertEqual(chosen.source, "oss")
        self.assertEqual(chosen.status, "design")
        self.assertEqual(chosen.unit, "px")
        self.assertEqual(self.index.selection("item.fire_range_delta").source, "vn_runtime")
        self.assertEqual(self.index.selection("item.roller_speed_delta").source, "oss")


if __name__ == "__main__":
    unittest.main()
