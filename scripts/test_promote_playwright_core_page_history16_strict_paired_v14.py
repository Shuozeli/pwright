"""AAA controls for the immutable Core page-history v14 publisher."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any
from unittest.mock import patch

import promote_playwright_core_page_history16_strict_paired_v14 as publisher


def _small_pair() -> dict[str, Any]:
    lanes: dict[str, list[bytes]] = {}
    replacements: dict[str, dict[int, dict[str, object]]] = {}
    for lane in ("candidate", "oracle"):
        lanes[lane] = [b'{"id":"off-target","keep":"raw"}\n'] + [
            f'{{"id":"old-{index}"}}\n'.encode() for index in range(16)
        ]
        replacements[lane] = {
            index: {"id": f"{lane}-{index}"} for index in range(1, 17)
        }
    return {
        "oldLines": lanes,
        "replacementRows": replacements,
        "oldSummaries": {"candidate": {}, "oracle": {}},
    }


class HistoryPairPublisherTests(unittest.TestCase):
    def test_prepare_pair_changes_only_sixteen_rows_in_each_lane(self) -> None:
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
                return_value={"scope": "history-v14"},
            ),
            patch.object(publisher.independent, "verify_payloads") as verify,
        ):
            # Act
            files = publisher.prepare_pair(checked)

        # Assert
        self.assertEqual(
            set(files),
            {
                "candidate.jsonl",
                "candidate.summary.json",
                "oracle.jsonl",
                "oracle.summary.json",
                "promotion-receipt.json",
            },
        )
        self.assertEqual(files["candidate.jsonl"].splitlines(keepends=True)[0], old_candidate[0])
        self.assertEqual(files["oracle.jsonl"].splitlines(keepends=True)[0], old_oracle[0])
        self.assertEqual(tuple(checked["oldLines"]["candidate"]), old_candidate)
        self.assertEqual(tuple(checked["oldLines"]["oracle"]), old_oracle)
        for lane in ("candidate", "oracle"):
            lines = files[f"{lane}.jsonl"].splitlines()
            self.assertEqual(len(lines), 17)
            self.assertEqual(
                [json.loads(line)["id"] for line in lines[1:]],
                [f"{lane}-{index}" for index in range(1, 17)],
            )
        verify.assert_called_once_with(files, checked)

    def test_prepare_pair_rejects_missing_target_row(self) -> None:
        # Arrange
        checked = _small_pair()
        checked["replacementRows"]["candidate"].pop(16)

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "exactly 16"):
            publisher.prepare_pair(checked)

    def test_publish_refuses_existing_immutable_destination(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            destination = Path(directory) / "already-published"
            destination.mkdir()
            with (
                patch.object(publisher.independent, "V14", destination),
                self.assertRaisesRegex(ValueError, "already exists"),
            ):
                # Act and Assert
                publisher.publish({})


if __name__ == "__main__":
    unittest.main()
