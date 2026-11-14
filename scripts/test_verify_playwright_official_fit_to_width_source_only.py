"""Fail-closed checks for the two upstream-private fit-to-width invocations."""

from __future__ import annotations

import json
import shutil
import subprocess
import unittest
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path
from tempfile import TemporaryDirectory

from verify_playwright_official_fit_to_width_source_only import (
    _GUARD,
    _MANIFEST,
    _ROOT,
    _SUITE,
    _file_sha256,
    verify,
    verify_manifest,
    verify_upstream,
)

_UPSTREAM = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
_RESULTS = _ROOT / ".work/official-conformance/results"
_CANDIDATE = _RESULTS / "playwright-core-candidate-core463-private-fit-to-width-source-only-v2.provenance.json"
_ORACLE = _RESULTS / "playwright-core-oracle-core463-private-fit-to-width-source-only-v2.provenance.json"


@contextmanager
def copied_evidence() -> Iterator[tuple[Path, Path, Path, dict[str, object]]]:
    if not _CANDIDATE.is_file():
        raise unittest.SkipTest("the paired v2 fit-to-width receipts are not available")
    with TemporaryDirectory() as directory:
        original = json.loads(_CANDIDATE.read_text())
        result = Path(directory) / Path(original["result"]).name
        receipt = result.with_suffix(".guard-receipt.jsonl")
        provenance = result.with_suffix(".provenance.json")
        shutil.copyfile(original["result"], result)
        shutil.copyfile(original["sourceOnlyExecution"]["guardReceipt"], receipt)
        original["result"] = str(result)
        original["resultSha256"] = _file_sha256(result)
        original["sourceOnlyExecution"]["guardReceipt"] = str(receipt)
        original["sourceOnlyExecution"]["guardReceiptSha256"] = _file_sha256(receipt)
        provenance.write_text(json.dumps(original))
        yield provenance, result, receipt, original


def first_spec(raw_suite: dict[str, object]) -> dict[str, object]:
    specs = raw_suite.get("specs", [])
    if specs:
        return specs[0]
    for child in raw_suite.get("suites", []):
        found = first_spec(child)
        if found:
            return found
    return {}


class FitToWidthSourceOnlyVerifierTests(unittest.TestCase):
    def test_pinned_manifest_and_zero_fixture_source(self) -> None:
        # Arrange
        upstream = _UPSTREAM

        # Act
        manifest = verify_manifest()
        source = verify_upstream(upstream)

        # Assert
        self.assertEqual(manifest["suites"][0]["id"], _SUITE)
        self.assertEqual(source["expectedTestsPerProject"], 2)
        self.assertEqual([item["line"] for item in source["expectedInvocations"]], [20, 25])

    def test_ast_guard_rejects_wrong_suite(self) -> None:
        # Arrange
        command = ["node", str(_GUARD), "--check", str(_MANIFEST), "wrong-suite", str(_UPSTREAM)]

        # Act
        result = subprocess.run(command, capture_output=True, text=True, check=False)

        # Assert
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("manifest, suite, or no-I/O guard changed", result.stderr)

    def test_saved_pair_has_no_candidate_or_public_api_attestation(self) -> None:
        # Arrange
        if not _CANDIDATE.is_file() or not _ORACLE.is_file():
            self.skipTest("the paired v2 fit-to-width receipts are not available")

        # Act
        candidate = verify(_CANDIDATE)
        oracle = verify(_ORACLE)

        # Assert
        self.assertEqual((candidate["passed"], oracle["passed"]), (2, 2))
        self.assertEqual((candidate["classification"], oracle["classification"]),
                         ("product-infrastructure", "product-infrastructure"))
        self.assertIs(candidate["candidateRuntimeAttested"], False)
        self.assertIs(candidate["publicApiParityEvidence"], False)

    def test_rejects_rehashed_wrong_invocation_id(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, result, _, record):
            report = json.loads(result.read_text())
            first_spec(report["suites"][0])["id"] = "f" * 20 + "-" + "f" * 20
            result.write_text(json.dumps(report))
            record["resultSha256"] = _file_sha256(result)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "unpinned spec"):
                verify(provenance)

    def test_rejects_rehashed_failed_attempt(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, result, _, record):
            report = json.loads(result.read_text())
            first_spec(report["suites"][0])["tests"][0]["results"][0]["status"] = "failed"
            result.write_text(json.dumps(report))
            record["resultSha256"] = _file_sha256(result)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "non-pass"):
                verify(provenance)

    def test_rejects_cross_lane_provenance_even_with_same_upstream_body(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["mode"] = "oracle"
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "unpinned process"):
                verify(provenance)

    def test_rejects_runtime_or_public_parity_forgery(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["sourceOnlyExecution"]["candidateRuntimeAttested"] = True
            record["sourceOnlyExecution"]["publicApiParityEvidence"] = True
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "attestation flags"):
                verify(provenance)

    def test_rejects_rehashed_guard_receipt_without_worker(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, receipt, record):
            rows = [json.loads(line) for line in receipt.read_text().splitlines()]
            receipt.write_text("\n".join(json.dumps(row) for row in rows if row["kind"] != "worker-no-io") + "\n")
            record["sourceOnlyExecution"]["guardReceiptSha256"] = _file_sha256(receipt)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "runner and separate workers"):
                verify(provenance)

    def test_rejects_tooling_hash_drift(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["toolingSources"][1]["sha256"] = "0" * 64
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "tooling selection or SHA-256"):
                verify(provenance)


if __name__ == "__main__":
    unittest.main()
