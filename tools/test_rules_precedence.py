"""Integrity tests for the rule-source precedence validator (ADR 0013).

Synthetic profiles never touch the real ones: the fixture text is built from the
expectation table, and a single field is changed per case. The real repository
profiles are checked once as a regression baseline.
"""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from rules_precedence import (
    PrecedenceError,
    check_expectations,
    check_profile,
    check_repository,
    expectations,
    parse_profile,
    parse_rules_v1,
    tier_of,
)


ROOT = Path(__file__).resolve().parents[1]
RULES_V1 = ROOT / "docs" / "specification" / "gameplay" / "RULES_V1.md"
TOOL = ROOT / "tools" / "rules_precedence.py"


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


if __name__ == "__main__":
    unittest.main()
