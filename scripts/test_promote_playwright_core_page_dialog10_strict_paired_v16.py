"""AAA controls for the immutable Core dialog v16 candidate-only publisher."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import promote_playwright_core_page_dialog10_strict_paired_v16 as publisher


class DialogCandidateOnlyPublisherTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.checked = publisher.independent.verify_inputs()

    def test_prepare_pair_changes_only_historical_candidate_red(self) -> None:
        # Arrange
        old = self.checked["oldLines"]["candidate"]

        # Act
        files = publisher.prepare_pair(self.checked)

        # Assert
        new = files["candidate.jsonl"].splitlines(keepends=True)
        self.assertEqual(len(new), publisher.independent.COUNT)
        self.assertEqual(
            [index for index, (left, right) in enumerate(zip(old, new, strict=True)) if left != right],
            [publisher.independent.RED_INDEX],
        )
        row = json.loads(new[publisher.independent.RED_INDEX])
        self.assertEqual((row["outcome"], row["attempts"]), ("expected", 2))
        self.assertNotIn("error", row)
        self.assertEqual(files["oracle.jsonl"], self.checked["oldFiles"]["oracle.jsonl"])
        self.assertEqual(
            files["oracle.summary.json"], self.checked["oldFiles"]["oracle.summary.json"]
        )

    def test_prepare_pair_refuses_wrong_historical_red(self) -> None:
        # Arrange
        checked = dict(self.checked)
        lines = list(self.checked["oldLines"]["candidate"])
        row = json.loads(lines[publisher.independent.RED_INDEX])
        row["outcome"] = "expected"
        lines[publisher.independent.RED_INDEX] = publisher.independent.canonical_line(row)
        checked["oldLines"] = {**self.checked["oldLines"], "candidate": lines}

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "historical v15 candidate RED"):
            publisher.prepare_pair(checked)

    def test_publish_refuses_existing_immutable_destination(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            destination = Path(directory) / "already-published"
            destination.mkdir()
            with (
                patch.object(publisher.independent, "V16", destination),
                self.assertRaisesRegex(ValueError, "already exists"),
            ):
                # Act / Assert
                publisher.publish({})


if __name__ == "__main__":
    unittest.main()
