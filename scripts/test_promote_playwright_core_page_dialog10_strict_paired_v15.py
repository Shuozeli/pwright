"""AAA controls for the immutable Core page-dialog v15 publisher."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any
from unittest.mock import patch

import promote_playwright_core_page_dialog10_strict_paired_v15 as publisher


def _small_pair() -> dict[str, Any]:
    lanes: dict[str, list[bytes]] = {}
    replacements: dict[str, dict[int, dict[str, object]]] = {}
    for lane in ("candidate", "oracle"):
        lanes[lane] = [b'{"id":"off-target","keep":"raw"}\n'] + [
            f'{{"id":"old-{index}"}}\n'.encode() for index in range(10)
        ]
        replacements[lane] = {
            index: {"id": f"{lane}-{index}"} for index in range(1, 11)
        }
    return {
        "oldLines": lanes,
        "replacementRows": replacements,
        "oldSummaries": {"candidate": {}, "oracle": {}},
    }


class DialogPairPublisherTests(unittest.TestCase):
    def test_prepare_pair_changes_only_ten_rows_in_each_lane(self) -> None:
        # Arrange
        checked = _small_pair()
        old_candidate = tuple(checked["oldLines"]["candidate"])
        old_oracle = tuple(checked["oldLines"]["oracle"])
        with (
            patch.object(
                publisher.independent,
                "expected_summary",
                side_effect=lambda lane, _summary, _ledger: {
                    "outcomes": {"lane": lane}
                },
            ),
            patch.object(
                publisher.independent,
                "expected_receipt",
                return_value={"scope": "dialog-v15"},
            ),
            patch.object(publisher.independent, "verify_payloads") as verify,
        ):
            # Act
            files = publisher.prepare_pair(checked)

        # Assert
        self.assertEqual(set(files), publisher.independent.BUNDLE_NAMES)
        self.assertEqual(
            files["candidate.jsonl"].splitlines(keepends=True)[0], old_candidate[0]
        )
        self.assertEqual(
            files["oracle.jsonl"].splitlines(keepends=True)[0], old_oracle[0]
        )
        self.assertEqual(tuple(checked["oldLines"]["candidate"]), old_candidate)
        self.assertEqual(tuple(checked["oldLines"]["oracle"]), old_oracle)
        for lane in ("candidate", "oracle"):
            lines = files[f"{lane}.jsonl"].splitlines()
            self.assertEqual(len(lines), 11)
            self.assertEqual(
                [json.loads(line)["id"] for line in lines[1:]],
                [f"{lane}-{index}" for index in range(1, 11)],
            )
        verify.assert_called_once_with(files, checked)

    def test_prepare_pair_rejects_missing_target_row(self) -> None:
        # Arrange
        checked = _small_pair()
        checked["replacementRows"]["candidate"].pop(10)

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "exactly 10"):
            publisher.prepare_pair(checked)

    def test_publish_refuses_existing_immutable_destination(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            destination = Path(directory) / "already-published"
            destination.mkdir()
            with (
                patch.object(publisher.independent, "V15", destination),
                self.assertRaisesRegex(ValueError, "already exists"),
            ):
                # Act and Assert
                publisher.publish({})


if __name__ == "__main__":
    unittest.main()
