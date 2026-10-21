"""Completion-integrity tests; synthetic records never change the real registry."""
import copy
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from kr_tracking import check_commit, check_integration, metrics, validate_execution, validate_features


ROOT = Path(__file__).resolve().parents[1]

# Biến GIT_* (vd. GIT_DIR khi chạy trong hook pre-push) làm `git -C <tmp>` thao tác lên repo thật:
# lần trước đã đặt core.bare=true và tạo commit "fixture root" trên nhánh đang push. Repo fixture
# phải chạy với môi trường đã gỡ mọi biến GIT_*.
CLEAN_ENV = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}


class TrackingIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.registry = json.loads((ROOT / "docs/specification/features.json").read_text())
        self.execution = json.loads((ROOT / "docs/planning/execution.json").read_text())
        self.features = self.registry["features"]
        self.baseline = self.registry["baseline"]
        self.commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, check=True, env=CLEAN_ENV,
                                     capture_output=True, text=True).stdout.strip()

    def complete_record(self, feature):
        feature["implementation"] = "done"
        feature["tracking"].update(
            assignee="synthetic-worker", reviewer="synthetic-reviewer", accepted_baseline=self.baseline,
            accepted_spec="synthetic-test-contract-r1", integration_commit=self.commit,
            evidence=["AGENTS.md", "git:" + self.commit],
            verification=dict(status="passed", command="synthetic acceptance check", result="fixture passed"),
            review=dict(status="approved", result="synthetic independent review"))

    def first_open(self):
        """Mục đầu tiên chưa done — sổ thật có thể đã có nhiều mục done."""
        return next(f for f in self.features if f["implementation"] != "done")

    def test_fake_done_without_acceptance_is_rejected(self):
        f = self.first_open()
        f["implementation"] = "done"
        with self.assertRaises(ValueError):
            validate_features(self.features, self.baseline, ROOT)

    def test_completed_record_requires_independent_review(self):
        self.complete_record(self.features[0])
        self.features[0]["tracking"]["reviewer"] = "synthetic-worker"
        with self.assertRaisesRegex(ValueError, "independent review"):
            validate_features(self.features, self.baseline, ROOT)

    def test_done_requires_real_commit_and_existing_evidence(self):
        self.complete_record(self.features[0])
        original = copy.deepcopy(self.features[0])
        self.features[0]["tracking"]["evidence"] = ["docs/does-not-exist.acceptance"]
        with self.assertRaisesRegex(ValueError, "existing repository file"):
            validate_features(self.features, self.baseline, ROOT)
        self.features[0] = original
        self.features[0]["tracking"]["integration_commit"] = "0" * 40
        with self.assertRaisesRegex(ValueError, "commit does not exist"):
            validate_features(self.features, self.baseline, ROOT)

    def test_dependency_reopened_invalidates_dependent_done(self):
        self.complete_record(self.features[0])
        self.complete_record(self.features[1])
        validate_features(self.features, self.baseline, ROOT)
        self.features[0]["implementation"] = "in_review"
        self.features[0]["tracking"]["next_action"] = "review changed rule"
        with self.assertRaisesRegex(ValueError, "dependency is not done"):
            validate_features(self.features, self.baseline, ROOT)

    def test_only_done_earns_credit_and_drops_are_excluded(self):
        # Tương đối với dữ liệu thật: sổ tiến độ có thể đã có mục done.
        target = self.first_open()
        base = metrics(self.features)
        self.complete_record(target)
        m = metrics(self.features)
        self.assertEqual((m["V1"]["done"], m["V1"]["total"]), (base["V1"]["done"] + 1, 37))
        self.assertEqual(
            (m["catalog"]["done"], m["catalog"]["total"], m["excluded"]),
            (base["catalog"]["done"] + 1, 112, 2),
        )
        target["implementation"] = "in_review"
        self.assertEqual(metrics(self.features)["V1"]["percent"], base["V1"]["percent"])

    def test_scope_change_cannot_silently_change_denominator(self):
        self.features[0]["release"] = "later"
        with self.assertRaisesRegex(ValueError, "membership/scope changed"):
            validate_execution(self.execution, self.registry, ROOT)

    def test_existing_but_unintegrated_commit_is_not_completion(self):
        with tempfile.TemporaryDirectory(prefix="kr-tracking-git-") as temp:
            def git(*args, text=None):
                return subprocess.run(["git", "-C", temp, "-c", "user.name=TrackingFixture",
                                       "-c", "user.email=fixture@example.invalid", *args],
                                      input=text, text=True, capture_output=True, check=True,
                                      env=CLEAN_ENV).stdout.strip()
            git("init", "-q")
            # Chặn hồi quy: repo fixture phải là repo riêng trong thư mục tạm, không phải repo dự án.
            self.assertEqual(Path(git("rev-parse", "--show-toplevel")).resolve(), Path(temp).resolve())
            git("commit", "--allow-empty", "-qm", "fixture root")
            parent, tree = git("rev-parse", "HEAD"), git("rev-parse", "HEAD^{tree}")
            unintegrated = git("commit-tree", tree, "-p", parent, text="unmerged fixture\n")
            check_commit(temp, unintegrated)
            with self.assertRaisesRegex(ValueError, "not integrated"):
                check_integration(temp, unintegrated)

    def complete_phases_through_p8(self):
        """Fixture tự dựng mọi pha P0–P8 done hợp lệ, không phụ thuộc trạng thái thật của dự án."""
        for p in self.execution["phases"]:
            if p["id"] != "P9":
                p.update(status="done", reviewer="synthetic-reviewer", integration_commit=self.commit,
                         evidence=["AGENTS.md"], verification="synthetic passed fixture", blocker="")

    def test_p8_cannot_complete_with_unfinished_features_or_gates(self):
        self.complete_phases_through_p8()
        for f in self.features:
            f["implementation"] = "not_started"
        with self.assertRaisesRegex(ValueError, "P8 completion requires"):
            validate_execution(self.execution, self.registry, ROOT)

    def test_p8_guard_reached_regardless_of_real_phase_status(self):
        # Review #1: fixture cũ giữ P0 từ execution.json thật nên dừng ở lỗi predecessor.
        for status in ("not_started", "in_review", "done"):
            self.setUp()
            self.execution["phases"][0]["status"] = status
            self.complete_phases_through_p8()
            with self.assertRaisesRegex(ValueError, "P8 completion requires"):
                validate_execution(self.execution, self.registry, ROOT)


if __name__ == "__main__":
    unittest.main()
