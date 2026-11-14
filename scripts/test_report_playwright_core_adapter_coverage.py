"""Fail-closed tests for the pinned Core adapter-coverage report."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from typing import cast
from unittest.mock import patch

from scripts import report_playwright_core_adapter_coverage as coverage


def _write_json(path: Path, value: dict[str, object]) -> None:
    path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")


class CurrentSnapshotTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = coverage.audit(
            coverage.DEFAULT_MANIFEST,
            coverage.DEFAULT_INVENTORY,
            coverage.DEFAULT_PAIR / "candidate.jsonl",
            coverage.DEFAULT_PAIR / "oracle.jsonl",
            coverage.DEFAULT_PAIR / "promotion-receipt.json",
        )

    def test_complete_count_conservation_and_current_blockers(self) -> None:
        # Arrange
        report = self.report
        summary = cast(dict[str, object], report["summary"])

        # Act
        projects = cast(list[dict[str, object]], report["projects"])

        # Assert
        self.assertEqual(summary["inventoryRows"], 48_477)
        hashes = cast(dict[str, str], report["hashesSha256"])
        self.assertEqual(hashes["manifest"], coverage.EXPECTED_MANIFEST_SHA256)
        self.assertEqual(hashes["receipt"], coverage.EXPECTED_RECEIPT_SHA256)
        self.assertEqual(hashes["priorReceipt"], coverage.EXPECTED_PRIOR_RECEIPT_SHA256)
        self.assertEqual(hashes["baseReceipt"], coverage.EXPECTED_BASE_RECEIPT_SHA256)
        successor_receipt = json.loads(
            (coverage.DEFAULT_PAIR / "promotion-receipt.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(
            hashes["successorManifest"], successor_receipt["manifestSha256"]
        )
        self.assertEqual(
            hashes["strictManifest"],
            cast(
                dict[str, object],
                json.loads(
                    coverage.DEFAULT_PAIR.parent.joinpath(
                        coverage.BASE_PAIR_NAME, "promotion-receipt.json"
                    ).read_text(encoding="utf-8")
                ),
            )["manifestSha256"],
        )
        for name, path in coverage.COHORT_MANIFESTS.items():
            self.assertEqual(hashes[f"cohortManifest:{name}"], coverage.sha256(path))
        self.assertEqual(summary["suiteCount"], 20)
        self.assertEqual(summary["projectCount"], 58)
        self.assertEqual(sum(cast(int, p["total"]) for p in projects), 48_477)
        self.assertEqual(sum(cast(int, p["candidateNotRun"]) for p in projects), 43_857)
        self.assertEqual(sum(cast(int, p["oracleNotRun"]) for p in projects), 44_156)
        self.assertEqual(
            summary["candidateOutcomes"],
            {"expected": 4_544, "not-run": 43_857, "skipped": 21, "unexpected": 55},
        )
        self.assertEqual(
            summary["oracleOutcomes"],
            {"expected": 4_293, "not-run": 44_156, "skipped": 20, "unexpected": 8},
        )
        self.assertEqual(summary["candidateNotRunWithoutSuiteAdapter"], 33_188)
        self.assertEqual(
            summary["candidatePublicApiCandidateNotRunWithoutSuiteAdapter"], 21_810
        )
        self.assertEqual(
            summary["candidateNotRunWithConfiguredAdapterIncompatibleEngine"], 7_675
        )
        self.assertEqual(
            summary[
                "candidatePublicApiCandidateNotRunWithConfiguredAdapterIncompatibleEngine"
            ],
            6_027,
        )
        self.assertEqual(summary["candidateNotRunWithChromiumRouteConfigured"], 2_994)

    def test_no_adapter_never_means_non_applicable(self) -> None:
        # Arrange
        projects = cast(list[dict[str, object]], self.report["projects"])

        # Act
        blocked = [
            p
            for p in projects
            if p["candidateBlockedReason"] == "no-candidate-adapter-configured"
        ]

        # Assert
        self.assertEqual(sum(cast(int, p["candidateNotRun"]) for p in blocked), 33_188)
        self.assertTrue(
            all(
                p["applicabilityDisposition"] == "unresolved-not-automatically-excluded"
                for p in projects
            )
        )
        self.assertTrue(all(p["configuredCandidateAdapter"] is None for p in blocked))
        firefox = next(
            p
            for p in projects
            if p["suite"] == "library" and p["projectId"] == "firefox-page"
        )
        self.assertEqual(
            firefox["candidateBlockedReason"],
            "configured-preload-only-injects-chromium-cdp",
        )
        bidi = next(
            p
            for p in projects
            if p["suite"] == "bidi" and p["projectId"] == "moz-firefox-page"
        )
        self.assertEqual(bidi["protocolRequirement"], "webdriver-bidi")
        self.assertEqual(bidi["channelRequirement"], "moz-firefox")

    def test_firefox_and_webkit_line64_are_strict_red_without_parity(self) -> None:
        # Arrange
        receipt = json.loads(
            (coverage.DEFAULT_PAIR / "promotion-receipt.json").read_text(
                encoding="utf-8"
            )
        )
        target_indexes = set(receipt["targetZeroBasedIndexes"])

        # Act
        rows = [
            (candidate, oracle)
            for index, (candidate, oracle) in enumerate(
                zip(
                    coverage.json_lines(coverage.DEFAULT_PAIR / "candidate.jsonl"),
                    coverage.json_lines(coverage.DEFAULT_PAIR / "oracle.jsonl"),
                )
            )
            if index in target_indexes
        ]

        # Assert
        self.assertEqual(len(rows), 2)
        self.assertEqual(
            {candidate["projectId"] for candidate, _ in rows},
            {"firefox-library", "webkit-library"},
        )
        self.assertFalse(receipt["publicApiParityEvidence"])
        for candidate, oracle in rows:
            self.assertEqual(candidate["line"], 64)
            self.assertEqual(candidate["sourceClassification"], "public-api-candidate")
            self.assertEqual(candidate["outcome"], "unexpected")
            self.assertEqual(oracle["outcome"], "expected")
            self.assertFalse(candidate["publicApiParityEvidence"])
            self.assertFalse(oracle["publicApiParityEvidence"])
            self.assertFalse(candidate["candidateNativeRuntimeInvoked"])
            self.assertFalse(oracle["candidateNativeRuntimeInvoked"])


class SyntheticMutationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="pwright-adapter-audit-test-")
        root = Path(self.temp.name)
        self.manifest = root / "manifest.json"
        self.strict_manifest = root / "strict-manifest.json"
        self.successor_manifest = root / "successor-manifest.json"
        self.cohort_manifests = {
            name: root / f"{name}-manifest.json" for name in coverage.COHORT_MANIFESTS
        }
        self.inventory = root / "inventory.jsonl"
        self.base_pair = root / "base"
        self.base_pair.mkdir()
        self.base_candidate = self.base_pair / "candidate.jsonl"
        self.base_oracle = self.base_pair / "oracle.jsonl"
        self.base_receipt = self.base_pair / "promotion-receipt.json"
        self.prior_pair = root / "prior"
        self.prior_pair.mkdir()
        self.prior_candidate = self.prior_pair / "candidate.jsonl"
        self.prior_oracle = self.prior_pair / "oracle.jsonl"
        self.prior_receipt = self.prior_pair / "promotion-receipt.json"
        self.pair = root / "successor"
        self.pair.mkdir()
        self.candidate = self.pair / "candidate.jsonl"
        self.oracle = self.pair / "oracle.jsonl"
        self.receipt = self.pair / "promotion-receipt.json"
        self.row: dict[str, object] = {
            "classificationReasons": [],
            "column": 1,
            "expectedStatus": "passed",
            "file": "library/browsertype-launch.spec.ts",
            "line": 64,
            "projectId": "firefox-library",
            "projectName": "firefox-library",
            "role": "official-entrypoint",
            "sourceClassification": "public-api-candidate",
            "sourceFile": "tests/library/browsertype-launch.spec.ts",
            "specId": "one",
            "suite": "library",
            "titlePath": [
                "library/browsertype-launch.spec.ts",
                "should reject if launched browser fails immediately",
            ],
        }
        self.manifest_data: dict[str, object] = {
            "schema": 1,
            "repository": "https://github.com/microsoft/playwright.git",
            "tag": "v1.58.0",
            "commit": coverage.EXPECTED_COMMIT,
            "suites": [
                {
                    "id": "library",
                    "role": "official-entrypoint",
                    "expectedTests": 1,
                    "candidateAdapter": "test-mode-playwright-core",
                    "oracleAdapter": "test-mode-playwright-core-cdp",
                }
            ],
        }
        self.project_matrix = {
            "library": {
                "firefox-library": ("upstream-browser-transport", "firefox", "firefox")
            }
        }
        self._seal()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _seal(
        self,
        *,
        candidate_row: dict[str, object] | None = None,
        oracle_row: dict[str, object] | None = None,
    ) -> None:
        _write_json(self.manifest, self.manifest_data)
        _write_json(self.strict_manifest, {"schema": 1, "scope": "synthetic-launch"})
        _write_json(
            self.successor_manifest,
            {
                "schema": 2,
                "repository": "https://github.com/microsoft/playwright.git",
                "tag": "v1.58.0",
                "commit": coverage.EXPECTED_COMMIT,
                "upstreamSources": {
                    "tests/library/browsertype-launch.spec.ts": "synthetic-source-sha"
                },
                "cases": [
                    {"engine": "firefox", "project": "firefox-library", "specId": "one"}
                ],
            },
        )
        for name, path in self.cohort_manifests.items():
            _write_json(path, {"schema": 1, "cohort": name})
        _write_json(self.inventory, self.row)
        prior_row = {**self.row, "attempts": 0, "outcome": "not-run"}
        _write_json(self.base_candidate, prior_row)
        _write_json(self.base_oracle, prior_row)
        _write_json(
            self.base_receipt,
            {
                "schema": 1,
                "recordsPerLane": 1,
                "upstreamCommit": coverage.EXPECTED_COMMIT,
                "inventorySha256": coverage.sha256(self.inventory),
                "manifestSha256": coverage.sha256(self.strict_manifest),
                "lanes": {
                    lane: {
                        "ledger": str(path.resolve()),
                        "ledgerSha256": coverage.sha256(path),
                    }
                    for lane, path in (
                        ("candidate", self.base_candidate),
                        ("oracle", self.base_oracle),
                    )
                },
            },
        )
        _write_json(self.prior_candidate, prior_row)
        _write_json(self.prior_oracle, prior_row)
        _write_json(
            self.prior_receipt,
            {
                "schema": 1,
                "recordsPerLane": 1,
                "upstreamCommit": coverage.EXPECTED_COMMIT,
                "inventorySha256": coverage.sha256(self.inventory),
                "scope": "official-upstream-skip-accounting-only-strict-v1",
                "promotionKind": "skip-accounting-only",
                "skipAccountingOnly": True,
                "publicApiParityEvidence": False,
                "publicApiBehaviorAttested": False,
                "candidateRuntimeAttested": False,
                "priorBundle": "/original-host/outcomes/base",
                "priorBundleReceiptSha256": coverage.sha256(self.base_receipt),
                "manifestSha256ByCohort": {
                    name: coverage.sha256(path)
                    for name, path in self.cohort_manifests.items()
                },
                "evidence": {
                    name: {
                        "manifest": f"/original-host/manifests/{path.name}",
                        "manifestSha256": coverage.sha256(path),
                    }
                    for name, path in self.cohort_manifests.items()
                },
                "lanes": {
                    lane: {
                        "ledger": str(path.resolve()),
                        "ledgerSha256": coverage.sha256(path),
                        "base": f"/original-host/outcomes/base/{lane}.jsonl",
                        "baseSha256": coverage.sha256(base),
                    }
                    for lane, path, base in (
                        ("candidate", self.prior_candidate, self.base_candidate),
                        ("oracle", self.prior_oracle, self.base_oracle),
                    )
                },
            },
        )
        red_fields = {
            "attempts": 1,
            "candidateNativeRuntimeInvoked": False,
            "candidateRuntimeRequired": False,
            "evidenceScope": "official-browser-free-browsertype-dummy-node-other-engines-strict-red-v1",
            "publicApiParityEvidence": False,
        }
        _write_json(
            self.candidate,
            candidate_row or {**self.row, **red_fields, "outcome": "unexpected"},
        )
        _write_json(
            self.oracle, oracle_row or {**self.row, **red_fields, "outcome": "expected"}
        )
        _write_json(
            self.receipt,
            {
                "schema": 1,
                "scope": "official-browser-free-browsertype-dummy-node-other-engines-strict-red-v1",
                "promotionKind": "public-api-official-red",
                "assertionsExecuted": True,
                "fixtureAdapted": True,
                "candidateNativeRuntimeInvoked": False,
                "publicApiParityEvidence": False,
                "publicApiBehaviorAttested": True,
                "candidateRuntimeAttested": False,
                "promotedRowsPerLane": 1,
                "upstreamAssertionsModified": False,
                "recordsPerLane": 1,
                "upstreamCommit": coverage.EXPECTED_COMMIT,
                "inventorySha256": coverage.sha256(self.inventory),
                "manifestSha256": coverage.sha256(self.successor_manifest),
                "selectedUpstreamSpecSha256": "synthetic-source-sha",
                "targetSpecIds": ["one"],
                "targetZeroBasedIndexes": [0],
                "priorBundle": "/original-host/outcomes/prior",
                "priorBundleReceiptSha256": coverage.sha256(self.prior_receipt),
                "lanes": {
                    lane: {
                        "ledger": str(path.resolve()),
                        "ledgerSha256": coverage.sha256(path),
                        "base": f"/original-host/outcomes/prior/{lane}.jsonl",
                        "baseSha256": coverage.sha256(prior),
                        "promotedRows": 1,
                        "oldOutcome": "not-run",
                        "newOutcome": "unexpected"
                        if lane == "candidate"
                        else "expected",
                        "publicApiParityEvidence": False,
                        "candidateRuntimeAttested": False,
                        "outcomes": {
                            ("unexpected" if lane == "candidate" else "expected"): 1
                        },
                    }
                    for lane, path, prior in (
                        ("candidate", self.candidate, self.prior_candidate),
                        ("oracle", self.oracle, self.prior_oracle),
                    )
                },
            },
        )

    def _audit(
        self,
        *,
        expected_receipt_sha256: str | None = None,
        expected_prior_receipt_sha256: str | None = None,
        expected_base_receipt_sha256: str | None = None,
        expected_manifest_sha256: str | None = None,
    ) -> dict[str, object]:
        with (
            patch.object(coverage, "EXPECTED_ROWS", 1),
            patch.object(coverage, "PROJECTS", self.project_matrix),
            patch.object(coverage, "STRICT_MANIFEST", self.strict_manifest),
            patch.object(coverage, "SUCCESSOR_MANIFEST", self.successor_manifest),
            patch.object(coverage, "COHORT_MANIFESTS", self.cohort_manifests),
            patch.object(coverage, "PRIOR_PAIR_NAME", "prior"),
            patch.object(coverage, "BASE_PAIR_NAME", "base"),
            patch.object(
                coverage,
                "EXPECTED_MANIFEST_SHA256",
                expected_manifest_sha256 or coverage.sha256(self.manifest),
            ),
            patch.object(
                coverage,
                "EXPECTED_RECEIPT_SHA256",
                expected_receipt_sha256 or coverage.sha256(self.receipt),
            ),
            patch.object(
                coverage,
                "EXPECTED_PRIOR_RECEIPT_SHA256",
                expected_prior_receipt_sha256 or coverage.sha256(self.prior_receipt),
            ),
            patch.object(
                coverage,
                "EXPECTED_BASE_RECEIPT_SHA256",
                expected_base_receipt_sha256 or coverage.sha256(self.base_receipt),
            ),
        ):
            return coverage.audit(
                self.manifest, self.inventory, self.candidate, self.oracle, self.receipt
            )

    def test_small_valid_pair_preserves_unresolved_disposition(self) -> None:
        # Arrange
        expected = "unresolved-not-automatically-excluded"

        # Act
        report = self._audit()

        # Assert
        summary = cast(dict[str, object], report["summary"])
        projects = cast(list[dict[str, object]], report["projects"])
        self.assertEqual(summary["inventoryRows"], 1)
        self.assertEqual(projects[0]["applicabilityDisposition"], expected)

    def test_pinned_receipt_can_be_relocated_with_co_located_hash_matched_ledgers(
        self,
    ) -> None:
        # Arrange
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        prior_receipt = json.loads(self.prior_receipt.read_text(encoding="utf-8"))
        receipt["lanes"]["candidate"]["ledger"] = (
            "/original-host/bundle/candidate.jsonl"
        )
        receipt["lanes"]["oracle"]["ledger"] = "/original-host/bundle/oracle.jsonl"
        prior_receipt["lanes"]["candidate"]["ledger"] = (
            "/original-host/prior/candidate.jsonl"
        )
        prior_receipt["lanes"]["oracle"]["ledger"] = "/original-host/prior/oracle.jsonl"
        _write_json(self.prior_receipt, prior_receipt)
        receipt["priorBundleReceiptSha256"] = coverage.sha256(self.prior_receipt)
        _write_json(self.receipt, receipt)

        # Act
        report = self._audit()

        # Assert
        summary = cast(dict[str, object], report["summary"])
        self.assertEqual(summary["inventoryRows"], 1)

    def test_rejects_ledger_outside_receipt_bundle(self) -> None:
        # Arrange
        separate = Path(self.temp.name) / "elsewhere"
        separate.mkdir()
        moved = separate / "candidate.jsonl"
        moved.write_bytes(self.candidate.read_bytes())
        self.candidate = moved

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "not co-located"):
            self._audit()

    def test_rejects_prior_receipt_mutation_even_if_successor_link_is_updated(
        self,
    ) -> None:
        # Arrange
        original_prior_sha256 = coverage.sha256(self.prior_receipt)
        prior_receipt = json.loads(self.prior_receipt.read_text(encoding="utf-8"))
        prior_receipt["auditNote"] = "changed"
        _write_json(self.prior_receipt, prior_receipt)
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["priorBundleReceiptSha256"] = coverage.sha256(self.prior_receipt)
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "pinned prior paired receipt"):
            self._audit(expected_prior_receipt_sha256=original_prior_sha256)

    def test_rejects_broken_prior_receipt_link(self) -> None:
        # Arrange
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["priorBundleReceiptSha256"] = "0" * 64
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "prior paired bundle"):
            self._audit()

    def test_rejects_base_receipt_mutation_even_if_skip_link_is_updated(self) -> None:
        # Arrange
        original_base_sha256 = coverage.sha256(self.base_receipt)
        base_receipt = json.loads(self.base_receipt.read_text(encoding="utf-8"))
        base_receipt["auditNote"] = "changed"
        _write_json(self.base_receipt, base_receipt)
        prior_receipt = json.loads(self.prior_receipt.read_text(encoding="utf-8"))
        prior_receipt["priorBundleReceiptSha256"] = coverage.sha256(self.base_receipt)
        _write_json(self.prior_receipt, prior_receipt)
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["priorBundleReceiptSha256"] = coverage.sha256(self.prior_receipt)
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "pinned base paired receipt"):
            self._audit(expected_base_receipt_sha256=original_base_sha256)

    def test_rejects_broken_skip_to_base_receipt_link(self) -> None:
        # Arrange
        prior_receipt = json.loads(self.prior_receipt.read_text(encoding="utf-8"))
        prior_receipt["priorBundleReceiptSha256"] = "0" * 64
        _write_json(self.prior_receipt, prior_receipt)
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["priorBundleReceiptSha256"] = coverage.sha256(self.prior_receipt)
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "base paired bundle"):
            self._audit()

    def test_rejects_changed_base_ledger_without_receipt_update(self) -> None:
        # Arrange
        with self.base_candidate.open("a", encoding="utf-8") as destination:
            destination.write("\n")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "candidate base ledger chain"):
            self._audit()

    def test_rejects_changed_prior_ledger_without_receipt_update(self) -> None:
        # Arrange
        with self.prior_candidate.open("a", encoding="utf-8") as destination:
            destination.write("\n")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "candidate prior ledger chain"):
            self._audit()

    def test_rejects_successor_base_hash_mismatch(self) -> None:
        # Arrange
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["lanes"]["candidate"]["baseSha256"] = "0" * 64
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "candidate prior ledger chain"):
            self._audit()

    def test_rejects_prior_receipt_ledger_hash_mismatch(self) -> None:
        # Arrange
        prior_receipt = json.loads(self.prior_receipt.read_text(encoding="utf-8"))
        prior_receipt["lanes"]["oracle"]["ledgerSha256"] = "0" * 64
        _write_json(self.prior_receipt, prior_receipt)
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["priorBundleReceiptSha256"] = coverage.sha256(self.prior_receipt)
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "oracle prior ledger chain"):
            self._audit()

    def test_rejects_changed_skip_cohort_manifest(self) -> None:
        # Arrange
        _write_json(self.cohort_manifests["firefox"], {"schema": 1, "cohort": "other"})

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "skip cohort manifests"):
            self._audit()

    def test_rejects_mismatched_skip_cohort_evidence(self) -> None:
        # Arrange
        prior_receipt = json.loads(self.prior_receipt.read_text(encoding="utf-8"))
        prior_receipt["evidence"]["firefox"]["manifestSha256"] = "0" * 64
        _write_json(self.prior_receipt, prior_receipt)
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["priorBundleReceiptSha256"] = coverage.sha256(self.prior_receipt)
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "skip cohort manifest evidence"):
            self._audit()

    def test_rejects_claim_of_public_api_parity_for_skips(self) -> None:
        # Arrange
        prior_receipt = json.loads(self.prior_receipt.read_text(encoding="utf-8"))
        prior_receipt["publicApiParityEvidence"] = True
        _write_json(self.prior_receipt, prior_receipt)
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["priorBundleReceiptSha256"] = coverage.sha256(self.prior_receipt)
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "receipt provenance drifted"):
            self._audit()

    def test_rejects_claim_of_public_api_parity_for_strict_red(self) -> None:
        # Arrange
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["publicApiParityEvidence"] = True
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(
            ValueError, "paired ledger receipt provenance drifted"
        ):
            self._audit()

    def test_rejects_unknown_suite(self) -> None:
        # Arrange
        self.row["suite"] = "unreviewed-suite"
        self._seal()

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "unknown suite/project"):
            self._audit()

    def test_rejects_missing_or_wrong_project_mapping(self) -> None:
        # Arrange
        self.project_matrix = {
            "library": {"chromium-page": ("chromium-cdp", "chromium", "chromium")}
        }

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "strict successor manifest case"):
            self._audit()

    def test_rejects_inventory_source_field_mismatch(self) -> None:
        # Arrange
        broken = {
            **self.row,
            "attempts": 0,
            "outcome": "not-run",
            "sourceFile": "tests/library/other.spec.ts",
        }
        self._seal(candidate_row=broken)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "candidate source field drifted"):
            self._audit()

    def test_rejects_changed_ledger_without_receipt_update(self) -> None:
        # Arrange
        with self.candidate.open("a", encoding="utf-8") as destination:
            destination.write("\n")

        # Act / Assert
        with self.assertRaisesRegex(
            ValueError, "candidate ledger does not match paired receipt"
        ):
            self._audit()

    def test_rejects_resealed_candidate_expected_outcome_for_strict_red(self) -> None:
        # Arrange
        candidate = json.loads(self.candidate.read_text(encoding="utf-8"))
        candidate["outcome"] = "expected"
        _write_json(self.candidate, candidate)
        receipt = json.loads(self.receipt.read_text(encoding="utf-8"))
        receipt["lanes"]["candidate"]["ledgerSha256"] = coverage.sha256(self.candidate)
        receipt["lanes"]["candidate"]["outcomes"] = {"expected": 1}
        _write_json(self.receipt, receipt)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "candidate strict red outcome drifted"):
            self._audit()

    def test_rejects_changed_strict_successor_manifest(self) -> None:
        # Arrange
        successor_manifest = json.loads(
            self.successor_manifest.read_text(encoding="utf-8")
        )
        successor_manifest["auditNote"] = "changed"
        _write_json(self.successor_manifest, successor_manifest)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "strict successor manifest"):
            self._audit()

    def test_rejects_consistent_receipt_and_ledger_double_mutation(self) -> None:
        # Arrange
        original_receipt_sha256 = coverage.sha256(self.receipt)
        changed = {**self.row, "attempts": 1, "outcome": "expected"}
        self._seal(candidate_row=changed)
        self.assertNotEqual(coverage.sha256(self.receipt), original_receipt_sha256)
        self.assertEqual(
            json.loads(self.receipt.read_text(encoding="utf-8"))["lanes"]["candidate"][
                "ledgerSha256"
            ],
            coverage.sha256(self.candidate),
        )

        # Act / Assert
        with self.assertRaisesRegex(
            ValueError, "pinned paired receipt SHA-256 drifted"
        ):
            self._audit(expected_receipt_sha256=original_receipt_sha256)

    def test_rejects_full_manifest_content_mutation_with_same_counts(self) -> None:
        # Arrange
        original_manifest_sha256 = coverage.sha256(self.manifest)
        self.manifest_data["auditNote"] = "changed without changing suite counts"
        _write_json(self.manifest, self.manifest_data)

        # Act / Assert
        with self.assertRaisesRegex(
            ValueError, "pinned full Core manifest SHA-256 drifted"
        ):
            self._audit(expected_manifest_sha256=original_manifest_sha256)

    def test_rejects_strict_manifest_drift_from_receipt(self) -> None:
        # Arrange
        _write_json(self.strict_manifest, {"schema": 1, "scope": "mutated-launch"})

        # Act / Assert
        with self.assertRaisesRegex(
            ValueError, "strict launch manifest does not match base paired receipt"
        ):
            self._audit()

    def test_rejects_manifest_count_drift(self) -> None:
        # Arrange
        cast(list[dict[str, object]], self.manifest_data["suites"])[0][
            "expectedTests"
        ] = 2
        self._seal()

        # Act / Assert
        with self.assertRaisesRegex(
            ValueError, "manifest suite coverage/count drifted"
        ):
            self._audit()

    def test_rejects_adapter_matrix_change_until_reviewed(self) -> None:
        # Arrange
        cast(list[dict[str, object]], self.manifest_data["suites"])[0][
            "candidateAdapter"
        ] = "unreviewed"
        self._seal()

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "adapter matrix changed"):
            self._audit()


if __name__ == "__main__":
    unittest.main()
