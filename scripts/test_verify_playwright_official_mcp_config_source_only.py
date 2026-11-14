"""Tamper checks for the narrow MCP configuration source-only receipt."""

from __future__ import annotations

import json
import shutil
import unittest
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path
from tempfile import TemporaryDirectory

from verify_playwright_official_mcp_config_source_only import (
    _ROOT,
    _SOURCE_FILES,
    _sha256,
    _source_digest,
    verify,
)

_RESULTS = _ROOT / ".work/official-conformance/results"
_CANDIDATE = _RESULTS / "playwright-core-candidate-core462-private-mcp-config-source-only-v1.provenance.json"
_ORACLE = _RESULTS / "playwright-core-oracle-core462-private-mcp-config-source-only-v1.provenance.json"


@contextmanager
def copied_evidence() -> Iterator[tuple[Path, Path, Path, dict[str, object]]]:
    if not _CANDIDATE.is_file():
        raise unittest.SkipTest("the eight-case source-only receipt is not available")
    with TemporaryDirectory() as directory:
        destination = Path(directory)
        original = json.loads(_CANDIDATE.read_text())
        result = destination / "copy.json"
        receipt = destination / "copy.guard-receipt.jsonl"
        provenance = destination / "copy.provenance.json"
        shutil.copyfile(original["result"], result)
        shutil.copyfile(original["sourceOnlyExecution"]["guardReceipt"], receipt)
        original["result"] = str(result)
        original["resultSha256"] = _sha256(result)
        original["sourceOnlyExecution"]["guardReceipt"] = str(receipt)
        original["sourceOnlyExecution"]["guardReceiptSha256"] = _sha256(receipt)
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


class McpConfigSourceOnlyVerifierTests(unittest.TestCase):
    def test_saved_receipts_are_source_only_not_public_parity(self) -> None:
        # Arrange
        if not _CANDIDATE.is_file() or not _ORACLE.is_file():
            self.skipTest("the paired eight-case receipts are not available")

        # Act
        candidate = verify(_CANDIDATE)
        oracle = verify(_ORACLE)

        # Assert
        self.assertEqual((candidate["passed"], oracle["passed"]), (8, 8))
        self.assertEqual((candidate["projects"], candidate["lines"]), (4, 2))
        self.assertIs(candidate["candidateRuntimeAttested"], False)
        self.assertIs(candidate["publicApiParityEvidence"], False)

    def test_rejects_rehashed_result_with_wrong_invocation_id(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, result, _, record):
            report = json.loads(result.read_text())
            first_spec(report["suites"][0])["id"] = "f" * 20 + "-" + "f" * 20
            result.write_text(json.dumps(report))
            record["resultSha256"] = _sha256(result)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "exact eight invocation IDs"):
                verify(provenance)

    def test_rejects_rehashed_result_with_wrong_project(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, result, _, record):
            report = json.loads(result.read_text())
            first_spec(report["suites"][0])["tests"][0]["projectId"] = "msedge"
            result.write_text(json.dumps(report))
            record["resultSha256"] = _sha256(result)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "failure, skip, or retried invocation"):
                verify(provenance)

    def test_rejects_rehashed_failed_attempt(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, result, _, record):
            report = json.loads(result.read_text())
            first_spec(report["suites"][0])["tests"][0]["results"][0]["status"] = "failed"
            result.write_text(json.dumps(report))
            record["resultSha256"] = _sha256(result)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "failure, skip, or retried invocation"):
                verify(provenance)

    def test_rejects_forged_candidate_runtime_attestation(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["sourceOnlyExecution"]["candidateRuntimeAttested"] = True
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "attestation flags"):
                verify(provenance)

    def test_rejects_forged_public_parity_attestation(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["sourceOnlyExecution"]["publicApiParityEvidence"] = True
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "attestation flags"):
                verify(provenance)

    def test_rejects_rehashed_receipt_without_workers(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, receipt, record):
            records = [json.loads(line) for line in receipt.read_text().splitlines()]
            receipt.write_text("\n".join(json.dumps(item) for item in records if item["kind"] != "worker-no-io") + "\n")
            record["sourceOnlyExecution"]["guardReceiptSha256"] = _sha256(receipt)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "separate runner and worker"):
                verify(provenance)

    def test_rejects_tooling_hash_drift(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["toolingSources"][1]["sha256"] = "0" * 64
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "tooling SHA-256"):
                verify(provenance)

    def test_source_digest_detects_compiled_implementation_drift(self) -> None:
        # Arrange
        if not _CANDIDATE.is_file():
            self.skipTest("the eight-case source-only receipt is not available")
        upstream = Path(json.loads(_CANDIDATE.read_text())["upstream"])
        with TemporaryDirectory() as directory:
            copied = Path(directory)
            for relative in _SOURCE_FILES:
                target = copied / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(upstream / relative, target)
            baseline = _source_digest(copied)
            implementation = copied / "packages/playwright/lib/mcp/browser/config.js"
            implementation.write_bytes(implementation.read_bytes() + b"\n// drift\n")

            # Act
            changed = _source_digest(copied)

            # Assert
            self.assertNotEqual(changed, baseline)


if __name__ == "__main__":
    unittest.main()
