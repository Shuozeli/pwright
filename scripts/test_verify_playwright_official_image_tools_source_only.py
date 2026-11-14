"""Focused, read-only checks for the dedicated image-tools evidence verifier."""

from __future__ import annotations

import json
import shutil
import unittest
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path
from tempfile import TemporaryDirectory

from verify_playwright_official_image_tools_source_only import (
    _SOURCE_FILES,
    _sha256,
    _source_digest,
    verify,
)

_ROOT = Path(__file__).parents[1]
_SAVED = (
    _ROOT
    / ".work/official-conformance/results/"
    "playwright-core-candidate-core461-private-image-tools-source-only-v2.provenance.json"
)


@contextmanager
def copied_evidence() -> Iterator[tuple[Path, Path, Path, dict[str, object]]]:
    if not _SAVED.is_file():
        raise unittest.SkipTest("the 25-case source-only receipt is not available")
    with TemporaryDirectory() as directory:
        destination = Path(directory)
        original = json.loads(_SAVED.read_text())
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


class ImageToolsSourceOnlyVerifierTests(unittest.TestCase):
    def test_saved_receipt_verifies_without_runtime_attestation(self) -> None:
        # Arrange
        if not _SAVED.is_file():
            self.skipTest("the 25-case source-only receipt is not available")

        # Act
        summary = verify(_SAVED)

        # Assert
        self.assertEqual(summary["passed"], 25)
        self.assertEqual((summary["fixtureTests"], summary["unitTests"]), (21, 4))
        self.assertIs(summary["candidateRuntimeAttested"], False)
        self.assertIs(summary["publicApiParityEvidence"], False)

    def test_rejects_rehashed_result_with_wrong_invocation_id(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, result, _, record):
            report = json.loads(result.read_text())
            first_spec(report["suites"][0])["id"] = "f" * 20 + "-" + "f" * 20
            result.write_text(json.dumps(report))
            record["resultSha256"] = _sha256(result)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "exact invocation IDs"):
                verify(provenance)

    def test_rejects_rehashed_result_with_forged_pass_outcome(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, result, _, record):
            report = json.loads(result.read_text())
            first_spec(report["suites"][0])["tests"][0]["results"][0]["status"] = "failed"
            result.write_text(json.dumps(report))
            record["resultSha256"] = _sha256(result)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "non-pass, skipped, or retried"):
                verify(provenance)

    def test_rejects_rehashed_guard_receipt_without_worker(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, receipt, record):
            records = [json.loads(line) for line in receipt.read_text().splitlines()]
            receipt.write_text("\n".join(
                json.dumps(item) for item in records if item["kind"] != "worker-no-io"
            ) + "\n")
            record["sourceOnlyExecution"]["guardReceiptSha256"] = _sha256(receipt)
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "one runner and separate workers"):
                verify(provenance)

    def test_rejects_tooling_hash_drift(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["toolingSources"][1]["sha256"] = "0" * 64
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "tooling SHA-256"):
                verify(provenance)

    def test_rejects_runtime_attestation_claim(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["sourceOnlyExecution"]["candidateRuntimeAttested"] = True
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "attestation flags"):
                verify(provenance)

    def test_rejects_public_parity_claim(self) -> None:
        # Arrange
        with copied_evidence() as (provenance, _, _, record):
            record["sourceOnlyExecution"]["publicApiParityEvidence"] = True
            provenance.write_text(json.dumps(record))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "attestation flags"):
                verify(provenance)

    def test_source_digest_detects_png_drift(self) -> None:
        # Arrange
        if not _SAVED.is_file():
            self.skipTest("the 25-case source-only receipt is not available")
        saved = json.loads(_SAVED.read_text())
        upstream = Path(saved["upstream"])
        with TemporaryDirectory() as directory:
            copied = Path(directory)
            for relative in _SOURCE_FILES:
                target = copied / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(upstream / relative, target)
            shutil.copytree(
                upstream / "tests/image_tools/fixtures",
                copied / "tests/image_tools/fixtures",
            )
            baseline = _source_digest(copied)
            png = next((copied / "tests/image_tools/fixtures").rglob("*-actual.png"))
            png.write_bytes(png.read_bytes() + b"drift")

            # Act
            changed = _source_digest(copied)

            # Assert
            self.assertNotEqual(changed, baseline)


if __name__ == "__main__":
    unittest.main()
