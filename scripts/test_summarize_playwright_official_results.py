"""Focused tests for strict Core source-only outcome evidence."""

from __future__ import annotations

import filecmp
import hashlib
import io
import json
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from summarize_playwright_official_results import (
    _apply_report,
    _check_generic_source_only_inventory_rows,
    _check_image_tools_destination,
    _check_image_tools_inventory_rows,
    _check_mcp_config_destination,
    _check_mcp_config_inventory_rows,
    _generic_result_mode,
    _image_tools_source_only_selection,
    _load_base,
    _load_inventory,
    _mcp_config_source_only_selection,
    _source_only_selection,
    _verified_core_batch_event_rows,
    _verified_generic_result,
    main,
)

_ROOT = Path(__file__).parents[1]
_CANDIDATE_IMAGE_TOOLS = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-core-candidate-core461-private-image-tools-source-only-v2.json"
)
_ORACLE_IMAGE_TOOLS = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-core-oracle-core461-private-image-tools-source-only-v2.json"
)
_CANDIDATE_BASE = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-route55-cookies-frozen-20261011-v1.jsonl"
)
_ORACLE_BASE = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-oracle-route55-blank-anchor-20261011-v2.jsonl"
)
_CANDIDATE_MCP_CONFIG = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-core-candidate-core462-private-mcp-config-source-only-v1.json"
)
_ORACLE_MCP_CONFIG = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-core-oracle-core462-private-mcp-config-source-only-v1.json"
)
_CANDIDATE_MCP_BASE = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-core461-image-tools-source-only-v2.jsonl"
)
_ORACLE_MCP_BASE = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-oracle-core461-image-tools-source-only-v2.jsonl"
)
_CORE_ROUTE_RESULT = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-core-candidate-route55-large-post-data-frozen-20261011-v2.json"
)
_CORE_ROUTE_BASE = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-core462-mcp-config-source-only-v1.jsonl"
)
_CORE_ROUTE_SUCCESSOR = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-route55-large-post-data-frozen-20261011-v2.jsonl"
)
_CORE_PRIVATE_BATCH_RESULT = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-core-candidate-core460-private-batch-library-v1.json"
)
_CORE_PRIVATE_BATCH_BIDI_RESULT = (
    _ROOT / ".work/official-conformance/results/"
    "playwright-core-candidate-core460-private-batch-bidi-v1.json"
)
_CORE_PRIVATE_BATCH_BASE = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-core459-page-goto64-route55-fill45-selectors-react60-v3.jsonl"
)
_CORE_PRIVATE_BATCH_SUCCESSOR = (
    _ROOT / ".work/official-conformance/outcomes/"
    "playwright-core-candidate-core460-private-batch-v1.jsonl"
)
_CORE_INVENTORY = _ROOT / ".work/official-conformance/playwright-core-v1.58.0.jsonl"


class GenericResultProvenanceTests(unittest.TestCase):
    def test_source_bound_candidate_result_passes_current_verifier(self) -> None:
        # Arrange
        if not _CORE_ROUTE_RESULT.is_file():
            self.skipTest("source-bound Core route receipt is unavailable")

        # Act
        digest = _verified_generic_result(_CORE_ROUTE_RESULT, "candidate")

        # Assert
        self.assertEqual(digest, hashlib.sha256(_CORE_ROUTE_RESULT.read_bytes()).hexdigest())

    def test_generic_result_rejects_missing_or_tampered_sidecar(self) -> None:
        # Arrange
        if not _CORE_ROUTE_RESULT.is_file():
            self.skipTest("source-bound Core route receipt is unavailable")
        with TemporaryDirectory() as directory:
            report = Path(directory) / "report.json"
            report.write_bytes(_CORE_ROUTE_RESULT.read_bytes())
            sidecar = report.with_suffix(".provenance.json")

            # Act / Assert
            with self.assertRaisesRegex(SystemExit, "sibling provenance"):
                _verified_generic_result(report, "candidate")
            provenance = json.loads(_CORE_ROUTE_RESULT.with_suffix(".provenance.json").read_text())
            provenance["result"] = str(report)
            provenance["toolingSources"][1]["sha256"] = "0" * 64
            sidecar.write_text(json.dumps(provenance))
            with self.assertRaisesRegex(SystemExit, "provenance verification failed"):
                _verified_generic_result(report, "candidate")

    def test_generic_result_rejects_wrong_lane_or_unattested_candidate(self) -> None:
        # Arrange
        if not _CORE_ROUTE_RESULT.is_file():
            self.skipTest("source-bound Core route receipt is unavailable")

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "exact schema-2 lane"):
            _verified_generic_result(_CORE_ROUTE_RESULT, "oracle")
        with (
            patch("verify_playwright_official_provenance.verify_provenance", return_value={
                "schema": 3, "suite": "library-page-route-public-chromium",
                "mode": "candidate", "result": str(_CORE_ROUTE_RESULT),
                "candidateArtifactSha256": "a" * 64,
            }),
            self.assertRaisesRegex(SystemExit, "exact schema-2 lane"),
        ):
            _verified_generic_result(_CORE_ROUTE_RESULT, "candidate")
        with (
            patch("verify_playwright_official_provenance.verify_provenance", return_value={
                "schema": 2, "suite": "library-page-route-public-chromium",
                "mode": "candidate", "result": str(_CORE_ROUTE_RESULT),
                "candidateArtifactSha256": None,
            }),
            self.assertRaisesRegex(SystemExit, "candidate artifact evidence"),
        ):
            _verified_generic_result(_CORE_ROUTE_RESULT, "candidate")

    def test_real_manifest_suite_promotes_to_different_inventory_suite(self) -> None:
        # Arrange
        if not all(path.is_file() for path in (_CORE_ROUTE_RESULT, _CORE_ROUTE_BASE, _CORE_ROUTE_SUCCESSOR)):
            self.skipTest("real 55-case Core route fixtures are unavailable")
        provenance = json.loads(_CORE_ROUTE_RESULT.with_suffix(".provenance.json").read_text())
        self.assertEqual(provenance["suite"], "library-page-route-public-chromium")
        with TemporaryDirectory() as directory:
            output = Path(directory) / "playwright-core-candidate-route-gate.jsonl"
            argv = [
                "summarize_playwright_official_results.py",
                "--base", str(_CORE_ROUTE_BASE),
                "--result", f"library={_CORE_ROUTE_RESULT}",
                "--output", str(output),
            ]

            # Act
            with patch("sys.argv", argv), redirect_stdout(io.StringIO()):
                main()

            # Assert
            self.assertEqual(
                hashlib.sha256(output.read_bytes()).hexdigest(),
                hashlib.sha256(_CORE_ROUTE_SUCCESSOR.read_bytes()).hexdigest(),
            )

    def test_generic_result_mode_rejects_cross_lane_base(self) -> None:
        # Arrange
        output = Path("playwright-core-candidate-new.jsonl")
        oracle_base = Path("playwright-core-oracle-old.jsonl")

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "same lane"):
            _generic_result_mode(output, oracle_base)
        self.assertEqual(_generic_result_mode(Path("playwright-core-candidate.jsonl"), None), "candidate")

    def test_report_mutation_after_verification_cannot_change_row(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            report = Path(directory) / "report.json"
            report.write_text('{"suites":[]}')
            original_digest = hashlib.sha256(report.read_bytes()).hexdigest()
            report.write_text('{"suites":[{"specs":[]}]}')
            records = [{"suite": "library", "specId": "one", "projectId": "chromium-page", "outcome": "not-run"}]
            keys = {("library", "one", "chromium-page"): 0}

            # Act / Assert
            with self.assertRaisesRegex(SystemExit, "changed after provenance verification"):
                _apply_report(
                    records, keys, "library", report, set(), replace_base=False,
                    verified_digest=original_digest,
                )
            self.assertEqual(records[0]["outcome"], "not-run")

    def test_cli_missing_sidecar_does_not_create_successor(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.jsonl"
            inventory.write_text(json.dumps({
                "suite": "library", "specId": "one", "projectId": "chromium-page",
            }) + "\n")
            report = root / "report.json"
            report.write_text('{"suites":[]}')
            output = root / "playwright-core-candidate-gate.jsonl"
            argv = [
                "summarize_playwright_official_results.py", "--inventory", str(inventory),
                "--result", f"library={report}", "--output", str(output),
            ]

            # Act / Assert
            with patch("sys.argv", argv), self.assertRaisesRegex(SystemExit, "sibling provenance"):
                main()
            self.assertFalse(output.exists())


class SourceOnlySummaryTests(unittest.TestCase):
    def test_schema3_selection_binds_the_verified_report_bytes(self) -> None:
        # Arrange
        if not _CORE_PRIVATE_BATCH_RESULT.is_file():
            self.skipTest("source-only batch receipt is unavailable")

        # Act
        pairs, batch, verified_digest = _source_only_selection(
            _CORE_PRIVATE_BATCH_RESULT, "candidate"
        )

        # Assert
        self.assertTrue(batch)
        self.assertTrue(pairs)
        self.assertEqual(
            verified_digest,
            hashlib.sha256(_CORE_PRIVATE_BATCH_RESULT.read_bytes()).hexdigest(),
        )

    def test_schema3_selection_rejects_cross_lane_promotion(self) -> None:
        # Arrange
        if not _CORE_PRIVATE_BATCH_RESULT.is_file():
            self.skipTest("source-only batch receipt is unavailable")

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "lane differs"):
            _source_only_selection(_CORE_PRIVATE_BATCH_RESULT, "oracle")

    def test_generic_source_only_selection_rejects_public_inventory_row(self) -> None:
        # Arrange
        pair = ("spec-a", "chromium-page")
        records = [{
            "suite": "library", "specId": pair[0], "projectId": pair[1],
            "role": "official-entrypoint", "sourceClassification": "public-api-candidate",
        }]
        keys = {("library", *pair): 0}

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "not a private entrypoint"):
            _check_generic_source_only_inventory_rows(records, keys, "library", {pair})

    def test_core460_audit_admits_only_its_441_indirect_private_event_rows(self) -> None:
        # Arrange
        if not all(path.is_file() for path in (_CORE_PRIVATE_BATCH_RESULT, _CORE_INVENTORY)):
            self.skipTest("Core460 batch evidence is unavailable")
        pairs, batch, _ = _source_only_selection(_CORE_PRIVATE_BATCH_RESULT, "candidate")
        records, keys = _load_inventory(_CORE_INVENTORY)

        # Act
        _check_generic_source_only_inventory_rows(
            records, keys, "library", pairs,
            source_only_batch=batch, inventory_path=_CORE_INVENTORY,
        )

        # Assert
        event_rows = _verified_core_batch_event_rows(_CORE_INVENTORY)
        self.assertEqual(len(event_rows), 441)
        self.assertEqual(sum(key[0] == "library" for key in event_rows), 147)
        self.assertEqual(sum(key[0] == "bidi" for key in event_rows), 294)
        self.assertEqual(sum(
            records[keys[("library", spec_id, project_id)]]["sourceClassification"]
            == "public-api-candidate"
            for spec_id, project_id in pairs
        ), 147)

    def test_core460_batch_rejects_a_new_public_inventory_row(self) -> None:
        # Arrange
        if not all(path.is_file() for path in (_CORE_PRIVATE_BATCH_RESULT, _CORE_INVENTORY)):
            self.skipTest("Core460 batch evidence is unavailable")
        pairs, batch, _ = _source_only_selection(_CORE_PRIVATE_BATCH_RESULT, "candidate")
        records, keys = _load_inventory(_CORE_INVENTORY)
        internal = next(
            keys[("library", spec_id, project_id)]
            for spec_id, project_id in pairs
            if records[keys[("library", spec_id, project_id)]]["sourceClassification"]
            == "implementation-internal"
        )
        records[internal]["sourceClassification"] = "public-api-candidate"

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "not a private entrypoint"):
            _check_generic_source_only_inventory_rows(
                records, keys, "library", pairs,
                source_only_batch=batch, inventory_path=_CORE_INVENTORY,
            )

    def test_core460_batch_rejects_unpinned_audit_digest(self) -> None:
        # Arrange
        if not all(path.is_file() for path in (_CORE_PRIVATE_BATCH_RESULT, _CORE_INVENTORY)):
            self.skipTest("Core460 batch evidence is unavailable")
        pairs, batch, _ = _source_only_selection(_CORE_PRIVATE_BATCH_RESULT, "candidate")
        records, keys = _load_inventory(_CORE_INVENTORY)

        # Act / Assert
        with (
            patch("summarize_playwright_official_results.CORE_BATCH_AUDIT_SHA256", "0" * 64),
            self.assertRaisesRegex(SystemExit, "lost its pinned source or digest"),
        ):
            _check_generic_source_only_inventory_rows(
                records, keys, "library", pairs,
                source_only_batch=batch, inventory_path=_CORE_INVENTORY,
            )

    def test_core460_two_report_replay_matches_frozen_candidate_bytes(self) -> None:
        # Arrange
        required = (
            _CORE_PRIVATE_BATCH_RESULT, _CORE_PRIVATE_BATCH_BIDI_RESULT,
            _CORE_PRIVATE_BATCH_BASE, _CORE_PRIVATE_BATCH_SUCCESSOR,
        )
        if not all(path.is_file() for path in required):
            self.skipTest("Core460 two-report replay evidence is unavailable")
        with TemporaryDirectory() as directory:
            output = Path(directory) / "playwright-core-candidate-core460-replay.jsonl"
            argv = [
                "summarize_playwright_official_results.py",
                "--base", str(_CORE_PRIVATE_BATCH_BASE),
                "--source-only-result", f"library={_CORE_PRIVATE_BATCH_RESULT}",
                "--source-only-result", f"bidi={_CORE_PRIVATE_BATCH_BIDI_RESULT}",
                "--output", str(output),
            ]

            # Act
            with patch("sys.argv", argv), redirect_stdout(io.StringIO()):
                main()

            # Assert
            self.assertTrue(filecmp.cmp(output, _CORE_PRIVATE_BATCH_SUCCESSOR, shallow=False))

    def test_mcp_config_v1_selection_is_exactly_eight_for_each_mode(self) -> None:
        # Arrange
        if not _CANDIDATE_MCP_CONFIG.is_file() or not _ORACLE_MCP_CONFIG.is_file():
            self.skipTest("MCP config v1 receipts are unavailable")

        # Act
        candidate, candidate_digest = _mcp_config_source_only_selection(_CANDIDATE_MCP_CONFIG, "candidate")
        oracle, oracle_digest = _mcp_config_source_only_selection(_ORACLE_MCP_CONFIG, "oracle")

        # Assert
        self.assertEqual(candidate, oracle)
        self.assertEqual(len(candidate), 8)
        self.assertEqual({project for _, project in candidate}, {"chrome", "chromium", "firefox", "webkit"})
        self.assertEqual(candidate_digest, hashlib.sha256(_CANDIDATE_MCP_CONFIG.read_bytes()).hexdigest())
        self.assertEqual(oracle_digest, hashlib.sha256(_ORACLE_MCP_CONFIG.read_bytes()).hexdigest())

    def test_mcp_config_v1_rejects_mode_mismatch(self) -> None:
        # Arrange
        if not _CANDIDATE_MCP_CONFIG.is_file():
            self.skipTest("MCP config v1 candidate receipt is unavailable")

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "mode, pass, or attestation mismatch"):
            _mcp_config_source_only_selection(_CANDIDATE_MCP_CONFIG, "oracle")

    def test_mcp_config_v1_rejects_runtime_attestation_claim(self) -> None:
        # Arrange
        if not _CANDIDATE_MCP_CONFIG.is_file():
            self.skipTest("MCP config v1 candidate receipt is unavailable")
        forged = {
            "mode": "candidate", "passed": 8, "classification": "implementation-internal",
            "scope": "upstream-private-source-only", "candidateRuntimeAttested": True,
            "publicApiParityEvidence": False,
            "resultSha256": json.loads(
                _CANDIDATE_MCP_CONFIG.with_suffix(".provenance.json").read_text()
            )["resultSha256"],
        }

        # Act / Assert
        with (
            patch("verify_playwright_official_mcp_config_source_only.verify", return_value=forged),
            self.assertRaisesRegex(SystemExit, "mode, pass, or attestation mismatch"),
        ):
            _mcp_config_source_only_selection(_CANDIDATE_MCP_CONFIG, "candidate")

    def test_mcp_config_v1_rejects_same_stem_alternate_result(self) -> None:
        # Arrange
        if not _CANDIDATE_MCP_CONFIG.is_file():
            self.skipTest("MCP config v1 candidate receipt is unavailable")
        with TemporaryDirectory() as directory:
            alternate = Path(directory) / "alternate.txt"
            alternate.write_text('{"suites":[]}')
            alternate.with_suffix(".provenance.json").write_text(
                _CANDIDATE_MCP_CONFIG.with_suffix(".provenance.json").read_text()
            )
            original_provenance = json.loads(
                _CANDIDATE_MCP_CONFIG.with_suffix(".provenance.json").read_text()
            )
            forged_verified = {
                "mode": "candidate", "passed": 8, "classification": "implementation-internal",
                "scope": "upstream-private-source-only", "candidateRuntimeAttested": False,
                "publicApiParityEvidence": False,
                "resultSha256": original_provenance["resultSha256"],
            }

            # Act / Assert
            with (
                patch("verify_playwright_official_mcp_config_source_only.verify", return_value=forged_verified),
                self.assertRaisesRegex(SystemExit, "result path or digest differs"),
            ):
                _mcp_config_source_only_selection(alternate, "candidate")

    def test_mcp_config_v1_preserves_private_classification_and_runtime_false(self) -> None:
        # Arrange
        if not _CANDIDATE_MCP_CONFIG.is_file():
            self.skipTest("MCP config v1 candidate receipt is unavailable")
        pairs, verified_digest = _mcp_config_source_only_selection(_CANDIDATE_MCP_CONFIG, "candidate")
        records = [
            {
                "suite": "mcp", "specId": spec_id, "projectId": project_id,
                "sourceClassification": "implementation-internal",
                "role": "official-entrypoint", "outcome": "not-run", "attempts": 0,
            }
            for spec_id, project_id in sorted(pairs)
        ]
        keys = {
            (row["suite"], row["specId"], row["projectId"]): index
            for index, row in enumerate(records)
        }
        _check_mcp_config_inventory_rows(records, keys, pairs)

        # Act
        _apply_report(
            records, keys, "mcp", _CANDIDATE_MCP_CONFIG, set(),
            replace_base=True, source_only_pairs=pairs, source_only_batch=True,
            verified_digest=verified_digest,
        )

        # Assert
        self.assertEqual({row["outcome"] for row in records}, {"expected"})
        self.assertEqual({row["attempts"] for row in records}, {1})
        self.assertEqual({row["sourceClassification"] for row in records}, {"implementation-internal"})
        self.assertEqual({row["evidenceScope"] for row in records}, {"upstream-private-source-only"})
        self.assertEqual({row["candidateRuntimeRequired"] for row in records}, {False})

    def test_mcp_config_v1_rejects_public_or_already_run_row(self) -> None:
        # Arrange
        pairs = {(f"{index:020x}-{index:020x}", "chromium") for index in range(8)}
        records = [
            {
                "sourceClassification": "implementation-internal", "role": "official-entrypoint",
                "outcome": "not-run", "attempts": 0,
            }
            for _ in range(8)
        ]
        keys = {("mcp", spec_id, project_id): index for index, (spec_id, project_id) in enumerate(sorted(pairs))}

        # Act / Assert
        records[0]["sourceClassification"] = "public-api-candidate"
        with self.assertRaisesRegex(SystemExit, "not an unrun private entrypoint"):
            _check_mcp_config_inventory_rows(records, keys, pairs)
        records[0]["sourceClassification"] = "implementation-internal"
        records[0]["outcome"] = "expected"
        with self.assertRaisesRegex(SystemExit, "not an unrun private entrypoint"):
            _check_mcp_config_inventory_rows(records, keys, pairs)

    def test_mcp_config_v1_rejects_wrong_or_existing_successor(self) -> None:
        # Arrange
        if not _CANDIDATE_MCP_BASE.is_file() or not _ORACLE_MCP_BASE.is_file():
            self.skipTest("current strict Core bases are unavailable")
        output = _CANDIDATE_MCP_BASE.parent / "playwright-core-candidate-core462-mcp-config-test-only.jsonl"
        existing = _CANDIDATE_MCP_BASE.parent / "playwright-core-candidate-core460-private-batch-v1.jsonl"

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "pinned current strict ledger"):
            _check_mcp_config_destination(_ROOT, _ORACLE_MCP_BASE, output, "candidate")
        with self.assertRaisesRegex(SystemExit, "successor already exists"):
            _check_mcp_config_destination(_ROOT, _CANDIDATE_MCP_BASE, existing, "candidate")

    def test_image_tools_v2_candidate_selection_is_exactly_25(self) -> None:
        # Arrange
        if not _CANDIDATE_IMAGE_TOOLS.is_file():
            self.skipTest("image-tools v2 candidate receipt is unavailable")

        # Act
        pairs, verified_digest = _image_tools_source_only_selection(_CANDIDATE_IMAGE_TOOLS, "candidate")

        # Assert
        self.assertEqual(len(pairs), 25)
        self.assertEqual({project for _, project in pairs}, {"image_tools"})
        self.assertEqual(verified_digest, hashlib.sha256(_CANDIDATE_IMAGE_TOOLS.read_bytes()).hexdigest())

    def test_image_tools_v2_oracle_selection_is_exactly_25(self) -> None:
        # Arrange
        if not _ORACLE_IMAGE_TOOLS.is_file():
            self.skipTest("image-tools v2 oracle receipt is unavailable")

        # Act
        pairs, verified_digest = _image_tools_source_only_selection(_ORACLE_IMAGE_TOOLS, "oracle")

        # Assert
        self.assertEqual(len(pairs), 25)
        self.assertEqual(verified_digest, hashlib.sha256(_ORACLE_IMAGE_TOOLS.read_bytes()).hexdigest())

    def test_image_tools_v2_rejects_mode_mismatch(self) -> None:
        # Arrange
        if not _CANDIDATE_IMAGE_TOOLS.is_file():
            self.skipTest("image-tools v2 candidate receipt is unavailable")

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "mode, pass, or attestation mismatch"):
            _image_tools_source_only_selection(_CANDIDATE_IMAGE_TOOLS, "oracle")

    def test_image_tools_v2_rejects_runtime_attestation_claim(self) -> None:
        # Arrange
        if not _CANDIDATE_IMAGE_TOOLS.is_file():
            self.skipTest("image-tools v2 candidate receipt is unavailable")
        forged = {
            "mode": "candidate", "passed": 25,
            "candidateRuntimeAttested": True, "publicApiParityEvidence": False,
        }

        # Act / Assert
        with (
            patch("verify_playwright_official_image_tools_source_only.verify", return_value=forged),
            self.assertRaisesRegex(SystemExit, "mode, pass, or attestation mismatch"),
        ):
            _image_tools_source_only_selection(_CANDIDATE_IMAGE_TOOLS, "candidate")

    def test_image_tools_v2_rejects_nonexpected_result_after_verifier(self) -> None:
        # Arrange
        if not _CANDIDATE_IMAGE_TOOLS.is_file():
            self.skipTest("image-tools v2 candidate receipt is unavailable")
        with TemporaryDirectory() as directory:
            report = json.loads(_CANDIDATE_IMAGE_TOOLS.read_text())
            current = report["suites"][0]
            while not current.get("specs"):
                current = current["suites"][0]
            current["specs"][0]["tests"][0]["status"] = "unexpected"
            result = Path(directory) / "forged.json"
            result.write_text(json.dumps(report))
            result.with_suffix(".provenance.json").write_text(json.dumps({"result": str(result)}))
            forged = {
                "mode": "candidate", "passed": 25,
                "candidateRuntimeAttested": False, "publicApiParityEvidence": False,
            }

            # Act / Assert
            with (
                patch("verify_playwright_official_image_tools_source_only.verify", return_value=forged),
                self.assertRaisesRegex(SystemExit, "non-expected outcome"),
            ):
                _image_tools_source_only_selection(result, "candidate")

    def test_image_tools_v2_preserves_private_classification_and_runtime_false(self) -> None:
        # Arrange
        if not _CANDIDATE_IMAGE_TOOLS.is_file():
            self.skipTest("image-tools v2 candidate receipt is unavailable")
        pairs, verified_digest = _image_tools_source_only_selection(_CANDIDATE_IMAGE_TOOLS, "candidate")
        records = [
            {
                "suite": "playwright-test", "specId": spec_id, "projectId": project_id,
                "sourceClassification": "implementation-internal",
                "role": "official-entrypoint", "outcome": "not-run", "attempts": 0,
            }
            for spec_id, project_id in sorted(pairs)
        ]
        keys = {
            (row["suite"], row["specId"], row["projectId"]): index
            for index, row in enumerate(records)
        }
        _check_image_tools_inventory_rows(records, keys, pairs)

        # Act
        _apply_report(
            records, keys, "playwright-test", _CANDIDATE_IMAGE_TOOLS, set(),
            replace_base=True, source_only_pairs=pairs, source_only_batch=True,
            verified_digest=verified_digest,
        )

        # Assert
        self.assertEqual({row["outcome"] for row in records}, {"expected"})
        self.assertEqual({row["attempts"] for row in records}, {1})
        self.assertEqual({row["sourceClassification"] for row in records}, {"implementation-internal"})
        self.assertEqual({row["evidenceScope"] for row in records}, {"upstream-private-source-only"})
        self.assertEqual({row["candidateRuntimeRequired"] for row in records}, {False})

    def test_image_tools_v2_rejects_public_inventory_row(self) -> None:
        # Arrange
        pairs = {(f"{index:020x}-{index:020x}", "image_tools") for index in range(25)}
        records = [
            {"sourceClassification": "implementation-internal", "role": "official-entrypoint", "outcome": "not-run"}
            for _ in range(25)
        ]
        keys = {("playwright-test", spec_id, project_id): index for index, (spec_id, project_id) in enumerate(sorted(pairs))}
        records[0]["sourceClassification"] = "public-api-candidate"

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "not an unrun private entrypoint"):
            _check_image_tools_inventory_rows(records, keys, pairs)

    def test_image_tools_v2_rejects_already_run_inventory_row(self) -> None:
        # Arrange
        pairs = {(f"{index:020x}-{index:020x}", "image_tools") for index in range(25)}
        records = [
            {"sourceClassification": "implementation-internal", "role": "official-entrypoint", "outcome": "not-run"}
            for _ in range(25)
        ]
        keys = {("playwright-test", spec_id, project_id): index for index, (spec_id, project_id) in enumerate(sorted(pairs))}
        records[0]["outcome"] = "expected"

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "not an unrun private entrypoint"):
            _check_image_tools_inventory_rows(records, keys, pairs)

    def test_image_tools_v2_rejects_wrong_or_existing_successor(self) -> None:
        # Arrange
        if not _CANDIDATE_BASE.is_file() or not _ORACLE_BASE.is_file():
            self.skipTest("current strict Core bases are unavailable")
        output = _CANDIDATE_BASE.parent / "playwright-core-candidate-core461-image-tools-test-only.jsonl"
        existing = _CANDIDATE_BASE.parent / "playwright-core-candidate-core460-private-batch-v1.jsonl"

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "pinned current strict ledger"):
            _check_image_tools_destination(_ROOT, _ORACLE_BASE, output, "candidate")
        with self.assertRaisesRegex(SystemExit, "successor already exists"):
            _check_image_tools_destination(_ROOT, _CANDIDATE_BASE, existing, "candidate")

    def test_batch_preserves_red_outcome_and_attempt_count(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.jsonl"
            rows = [
                {"suite": "library", "specId": "spec-a", "projectId": "chromium-library"},
                {"suite": "library", "specId": "spec-b", "projectId": "chromium-library"},
            ]
            inventory.write_text("".join(json.dumps(row) + "\n" for row in rows))
            report = root / "report.json"
            report.write_text(json.dumps({"suites": [{"specs": [
                {"id": "spec-a", "tests": [{
                    "projectId": "chromium-library", "status": "expected",
                    "results": [{"status": "passed"}],
                }]},
                {"id": "spec-b", "tests": [{
                    "projectId": "chromium-library", "status": "unexpected",
                    "results": [{"status": "failed", "error": {"message": "original failure\ncall log"}}],
                }]},
            ]}]}))
            records, keys = _load_inventory(inventory)

            # Act
            _apply_report(
                records, keys, "library", report, set(), replace_base=True,
                source_only_pairs={("spec-a", "chromium-library"), ("spec-b", "chromium-library")},
                source_only_batch=True,
            )

            # Assert
            self.assertEqual([row["outcome"] for row in records], ["expected", "unexpected"])
            self.assertEqual([row["attempts"] for row in records], [1, 1])
            self.assertEqual(records[1]["error"], "original failure")
            self.assertTrue(all(row["candidateRuntimeRequired"] is False for row in records))

    def test_batch_never_overwrites_a_base_outcome(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.jsonl"
            inventory.write_text(json.dumps({
                "suite": "library", "specId": "spec-a", "projectId": "chromium-library",
            }) + "\n")
            report = root / "report.json"
            report.write_text(json.dumps({"suites": [{"specs": [{
                "id": "spec-a", "tests": [{
                    "projectId": "chromium-library", "status": "expected",
                    "results": [{"status": "passed"}],
                }],
            }]}]}))
            records, keys = _load_inventory(inventory)
            records[0]["outcome"] = "unexpected"

            # Act and Assert
            with self.assertRaisesRegex(SystemExit, "cannot overwrite an existing outcome"):
                _apply_report(
                    records, keys, "library", report, set(), replace_base=True,
                    source_only_pairs={("spec-a", "chromium-library")},
                    source_only_batch=True,
                )
            self.assertEqual(records[0]["outcome"], "unexpected")

    def test_verified_private_report_marks_only_its_exact_inventory_row(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.jsonl"
            rows = [
                {"suite": "library", "specId": "spec-a", "projectId": "chromium-library"},
                {"suite": "library", "specId": "spec-b", "projectId": "webkit-library"},
            ]
            inventory.write_text("".join(json.dumps(row) + "\n" for row in rows))
            report = root / "report.json"
            report.write_text(json.dumps({"suites": [{"specs": [{
                "id": "spec-a", "tests": [{
                    "projectId": "chromium-library", "status": "expected",
                    "results": [{"status": "passed"}],
                }],
            }]}]}))
            records, keys = _load_inventory(inventory)

            # Act
            _apply_report(
                records, keys, "library", report, set(), replace_base=False,
                source_only_pairs={("spec-a", "chromium-library")},
            )

            # Assert
            self.assertEqual(records[0]["outcome"], "expected")
            self.assertEqual(records[0]["evidenceScope"], "upstream-private-source-only")
            self.assertIs(records[0]["candidateRuntimeRequired"], False)
            self.assertEqual(records[1]["outcome"], "not-run")
            self.assertNotIn("evidenceScope", records[1])

    def test_private_report_rejects_unverified_invocation_pair(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.jsonl"
            inventory.write_text(json.dumps({
                "suite": "library", "specId": "spec-a", "projectId": "chromium-library",
            }) + "\n")
            report = root / "report.json"
            report.write_text(json.dumps({"suites": [{"specs": [{
                "id": "spec-a", "tests": [{
                    "projectId": "chromium-library", "status": "expected",
                    "results": [{"status": "passed"}],
                }],
            }]}]}))
            records, keys = _load_inventory(inventory)

            # Act and Assert
            with self.assertRaisesRegex(SystemExit, "differ from verified provenance"):
                _apply_report(
                    records, keys, "library", report, set(), replace_base=False,
                    source_only_pairs={("spec-a", "firefox-library")},
                )

    def test_successor_preserves_prior_explicit_private_scope(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / "inventory.jsonl"
            row = {"suite": "library", "specId": "spec-a", "projectId": "chromium-library"}
            inventory.write_text(json.dumps(row) + "\n")
            base = root / "base.jsonl"
            base.write_text(json.dumps({
                **row, "outcome": "expected", "attempts": 1,
                "evidenceScope": "upstream-private-source-only",
                "candidateRuntimeRequired": False,
            }) + "\n")
            records, _ = _load_inventory(inventory)

            # Act
            _load_base(base, records)

            # Assert
            self.assertEqual(records[0]["evidenceScope"], "upstream-private-source-only")
            self.assertIs(records[0]["candidateRuntimeRequired"], False)


if __name__ == "__main__":
    unittest.main()
