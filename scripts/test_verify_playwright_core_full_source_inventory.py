"""Mutation controls for browser-free pinned Core source inventory preflight."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path
from typing import cast
from unittest.mock import patch

from scripts import verify_playwright_core_full_source_inventory as source_inventory


class FullSourceInventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.upstream = Path(self.temporary.name)
        raw = json.loads(source_inventory.MANIFEST.read_text(encoding="utf-8"))
        self.manifest = cast(dict[str, object], raw)
        fixed = [
            *source_inventory.NON_COLLECTED,
            "tests/components/test-all.spec.js",
        ]
        suffixes = [".spec.ts"] * 517 + [".spec.tsx"] * 25 + [".spec.js"] * 11
        generated = [
            f"{('tests' if index < 505 else 'examples')}/fake/source-{index}{suffix}"
            for index, suffix in enumerate(suffixes)
        ]
        self.specs = sorted([*fixed, *generated])
        self.tracked = set(self.specs)
        for entry in cast(list[dict[str, object]], self.manifest["suites"]):
            runner = cast(str, entry["runner"])
            if runner == source_inventory.GENERATED_RUNNER:
                self.tracked.add(source_inventory.GENERATED_RUNNER_PACKAGE)
            else:
                self.tracked.add(runner)
            if "config" in entry:
                self.tracked.add(cast(str, entry["config"]))
            for arg in cast(list[str], entry.get("args", [])):
                self.tracked.add(arg)
        for relative in self.tracked - set(self.specs):
            target = self.upstream / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("source\n", encoding="utf-8")
        aggregate = self.upstream / "tests/components/test-all.spec.js"
        aggregate.parent.mkdir(parents=True, exist_ok=True)
        aggregate.write_text("source\n", encoding="utf-8")

    def _validate(self) -> dict[str, object]:
        return source_inventory.validate_source_inventory(
            self.manifest, self.specs, self.tracked, self.upstream
        )

    def test_complete_source_and_declared_suite_inventory_is_source_only(self) -> None:
        # Arrange
        expected_scope = "pinned-upstream-source-and-declared-suite-inventory-only"

        # Act
        report = self._validate()

        # Assert
        self.assertEqual(report["scope"], expected_scope)
        self.assertEqual(report["trackedSpecFiles"], 558)
        self.assertEqual(report["nonCollectedFixtureSpecs"], 4)
        self.assertEqual(report["suites"], 20)
        self.assertEqual(report["declaredTestInvocations"], 48_477)
        self.assertEqual(
            report["deferredGeneratedRunner"],
            {
                "suite": "playwright-test",
                "runner": source_inventory.GENERATED_RUNNER,
                "trackedPackageSource": source_inventory.GENERATED_RUNNER_PACKAGE,
                "runnerBinaryVerified": False,
            },
        )
        self.assertIs(report["testsExecuted"], False)
        self.assertIs(report["candidateOracleOutcomesVerified"], False)

    def test_manifest_file_hash_drift_is_rejected_before_git(self) -> None:
        # Arrange
        changed = self.upstream / "manifest.json"
        changed.write_text(
            source_inventory.MANIFEST.read_text(encoding="utf-8") + " ",
            encoding="utf-8",
        )

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "manifest file or SHA-256 differs"):
            source_inventory.verify(self.upstream, changed)

    def test_wrong_upstream_commit_is_rejected(self) -> None:
        # Arrange
        self.manifest["commit"] = "0" * 40

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "manifest provenance"):
            self._validate()

    def test_missing_tracked_spec_is_rejected(self) -> None:
        # Arrange
        self.specs.pop()

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "source counts differ"):
            self._validate()

    def test_extra_tracked_spec_is_rejected(self) -> None:
        # Arrange
        self.specs.append("tests/fake/extra.spec.ts")
        self.specs.sort()
        self.tracked.add("tests/fake/extra.spec.ts")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "source counts differ"):
            self._validate()

    def test_duplicate_spec_identity_is_rejected(self) -> None:
        # Arrange
        self.specs.append(self.specs[0])
        self.specs.sort()

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "not sorted and unique"):
            self._validate()

    def test_untracked_spec_identity_is_rejected(self) -> None:
        # Arrange
        self.tracked.remove(self.specs[0])

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "untracked source"):
            self._validate()

    def test_missing_runner_source_is_rejected(self) -> None:
        # Arrange
        self.tracked.remove("packages/playwright/cli.js")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "tracked regular source is missing"):
            self._validate()

    def test_missing_config_source_is_rejected(self) -> None:
        # Arrange
        self.tracked.remove("tests/library/playwright.config.ts")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "tracked regular source is missing"):
            self._validate()

    def test_generated_runner_requires_tracked_package_source(self) -> None:
        # Arrange
        self.tracked.remove(source_inventory.GENERATED_RUNNER_PACKAGE)

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "tracked regular source is missing"):
            self._validate()

    def test_duplicate_non_collected_fixture_is_rejected(self) -> None:
        # Arrange
        entries = cast(list[dict[str, object]], self.manifest["nonCollectedSpecs"])
        entries.append(copy.deepcopy(entries[0]))

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "duplicate non-collected spec"):
            self._validate()

    def test_untracked_non_collected_fixture_is_rejected(self) -> None:
        # Arrange
        self.specs.remove("tests/installation/fixture-scripts/docker.spec.js")
        self.specs.append("tests/fake/replacement.spec.js")
        self.specs.sort()
        self.tracked.add("tests/fake/replacement.spec.js")

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "fixture spec set differs"):
            self._validate()

    def test_missing_suite_is_rejected(self) -> None:
        # Arrange
        suites = cast(list[dict[str, object]], self.manifest["suites"])
        suites.pop()

        # Act / Assert
        with self.assertRaisesRegex(ValueError, "suite set or declared invocation"):
            self._validate()

    def test_dirty_upstream_source_is_rejected(self) -> None:
        # Arrange
        command_results = [source_inventory.COMMIT, " M tests/library/example.spec.ts"]

        # Act / Assert
        with patch.object(source_inventory, "_git", side_effect=command_results):
            with self.assertRaisesRegex(ValueError, "tracked source is dirty"):
                source_inventory._pinned_specs(self.upstream)

    def test_different_upstream_checkout_commit_is_rejected(self) -> None:
        # Arrange
        observed = "0" * 40

        # Act / Assert
        with patch.object(source_inventory, "_git", return_value=observed):
            with self.assertRaisesRegex(ValueError, "upstream commit differs"):
                source_inventory._pinned_specs(self.upstream)


if __name__ == "__main__":
    unittest.main()
