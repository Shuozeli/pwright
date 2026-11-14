"""CI-portable synthetic controls for the paired connect-refused promoter."""

from __future__ import annotations

import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from unittest.mock import patch

import promote_playwright_python_connect_refused_strict_v1 as promoter


def encoded(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def synthetic_reports() -> dict[tuple[str, str, str], dict[str, object]]:
    return {
        key: {
            "result": f"/synthetic/{'-'.join(key)}.xml",
            "resultSha256": "1" * 64,
            "callReceiptSha256": "2" * 64,
            "_duration": 0.125,
            "_sidecarSha256": "3" * 64,
        }
        for key in promoter.KEYS
    }


class SyntheticBaseline:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.inventory = root / "inventory.jsonl"
        self.base_dir = root / "base"
        self.base_dir.mkdir()
        source = root / "scripts/promote_playwright_python_connect_close_strict_v2.py"
        source.parent.mkdir()
        source.write_bytes(b"synthetic pinned predecessor\n")
        self.source_sha = promoter.sha256(source.read_bytes())
        index_to_target = dict(zip(promoter.TARGET_INDEXES, promoter.strict.TEST_IDS))
        inventory_rows = [
            {
                "api": "async" if index < 4000 else "sync",
                "file": "synthetic.py",
                "id": index_to_target.get(index, f"synthetic::{index}"),
                "privateImports": [],
                "sourceClassification": "public-api-source",
            }
            for index in range(promoter.RECORDS)
        ]
        self.inventory.write_bytes(b"".join(map(encoded, inventory_rows)))
        self.inventory_sha = promoter.sha256(self.inventory.read_bytes())
        self.rows = [
            {**row, "attempts": 0, "outcome": "not-run"} for row in inventory_rows
        ]
        self.bases: dict[str, tuple[str, str]] = {}
        summaries: dict[str, dict[str, object]] = {}
        for mode in promoter.MODES:
            path = (
                self.base_dir
                / f"playwright-python-{mode}-browser-connect-close-strict-paired-v2.jsonl"
            )
            path.write_bytes(b"".join(map(encoded, self.rows)))
            ledger_sha = promoter.sha256(path.read_bytes())
            summary = {
                "schema": 1,
                "inventory": str(self.inventory),
                "records": promoter.RECORDS,
                "outcomes": dict(
                    sorted(Counter(row["outcome"] for row in self.rows).items())
                ),
                "ledgerSha256": ledger_sha,
                "ledger": str(path),
            }
            summary_path = path.with_suffix(".summary.json")
            summary_path.write_bytes((json.dumps(summary, indent=2) + "\n").encode())
            self.bases[mode] = (ledger_sha, promoter.sha256(summary_path.read_bytes()))
            summaries[mode] = summary
        receipt = {
            "kind": "playwright-python-browser-connect-close-post-fix-strict-paired-promotion-v2",
            "inventory": str(self.inventory),
            "inventorySha256": self.inventory_sha,
            "promoterSha256": self.source_sha,
            "commit": promoter.strict.COMMIT,
            "recordsPerLane": promoter.RECORDS,
            "focusedPublicBehaviorEvidence": True,
            "fullPlaywrightServerSupport": False,
            "publicApiParityEvidence": False,
            "successors": summaries,
        }
        receipt_path = self.base_dir / "promotion-receipt.json"
        receipt_path.write_bytes((json.dumps(receipt, indent=2) + "\n").encode())
        self.receipt_sha = promoter.sha256(receipt_path.read_bytes())

    def patches(self) -> object:
        return patch.multiple(
            promoter,
            ROOT=self.root,
            INVENTORY=self.inventory,
            INVENTORY_SHA256=self.inventory_sha,
            BASE_DIR=self.base_dir,
            BASES=self.bases,
            BASE_PROMOTER_SHA256=self.source_sha,
            BASE_RECEIPT_SHA256=self.receipt_sha,
        )


class BaselineAndSuccessorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.fixture = SyntheticBaseline(Path(self.directory.name))

    def test_base_validates_six_pinned_not_run_rows_in_both_lanes(self) -> None:
        # Arrange / Act
        with self.fixture.patches():
            originals = promoter.base()

        # Assert
        for mode in promoter.MODES:
            lines, rows = originals[mode]
            self.assertEqual(len(lines), promoter.RECORDS)
            self.assertEqual(
                [rows[index]["id"] for index in promoter.TARGET_INDEXES],
                list(promoter.strict.TEST_IDS),
            )
            self.assertTrue(
                all(
                    rows[index]["outcome"] == "not-run" and rows[index]["attempts"] == 0
                    for index in promoter.TARGET_INDEXES
                )
            )

    def test_changed_immutable_base_ledger_is_rejected(self) -> None:
        # Arrange
        path = (
            self.fixture.base_dir
            / "playwright-python-candidate-browser-connect-close-strict-paired-v2.jsonl"
        )
        path.write_bytes(path.read_bytes() + b"tampered\n")

        # Act / Assert
        with self.fixture.patches(), self.assertRaises(promoter.PromotionError):
            promoter.base()

    def test_each_successor_changes_only_exact_six_lines(self) -> None:
        # Arrange
        with self.fixture.patches():
            originals = promoter.base()

        # Act / Assert
        for mode in promoter.MODES:
            with self.fixture.patches():
                ledger, _, summary = promoter.successor(
                    mode,
                    originals[mode],
                    synthetic_reports(),
                    Path(f"/synthetic/{mode}.jsonl"),
                )
            after = ledger.splitlines(keepends=True)
            changed = [
                index
                for index, (before, new) in enumerate(zip(originals[mode][0], after))
                if before != new
            ]
            self.assertEqual(changed, list(promoter.TARGET_INDEXES))
            self.assertEqual(summary["outcomes"], {"not-run": 6520, "passed": 6})
            for index in changed:
                row = json.loads(after[index])
                self.assertEqual(row["sourceClassification"], "public-api-source")
                self.assertEqual(row["outcome"], "passed")
                self.assertEqual(row["attempts"], 1)
                self.assertEqual(row["durationSeconds"], 0.125)
                self.assertEqual(
                    {
                        key: value
                        for key, value in row.items()
                        if key not in promoter.GENERATED
                    },
                    {
                        key: value
                        for key, value in originals[mode][1][index].items()
                        if key not in promoter.GENERATED
                    },
                )

    def test_moved_target_is_rejected(self) -> None:
        # Arrange
        with self.fixture.patches():
            originals = promoter.base()
        lines, rows = originals["candidate"]
        changed = list(rows)
        index = promoter.TARGET_INDEXES[0]
        changed[index] = {**changed[index], "id": "different"}

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            promoter.successor(
                "candidate",
                (lines, changed),
                synthetic_reports(),
                Path("/synthetic/out.jsonl"),
            )

    def test_receipt_disclaims_remote_browser_and_full_parity(self) -> None:
        # Arrange
        output = Path(self.directory.name) / "not-published"

        # Act
        with (
            self.fixture.patches(),
            patch.object(promoter, "reports", return_value=synthetic_reports()),
        ):
            content, receipt = promoter.prepare(output)

        # Assert
        self.assertFalse(output.exists())
        self.assertFalse(receipt["fullPlaywrightServerSupport"])
        self.assertFalse(receipt["fullBrowserServerSupport"])
        self.assertFalse(receipt["browserServerLaunched"])
        self.assertFalse(receipt["remoteBrowserConstructed"])
        self.assertFalse(receipt["publicApiParityEvidence"])
        self.assertEqual(receipt["fixtureAdaptation"], promoter.strict.ADAPTATION)
        self.assertEqual(len(receipt["reports"]), 12)
        self.assertTrue(
            all(report["ledgerRowUpdated"] for report in receipt["reports"])
        )
        self.assertEqual(len(content), 5)

    def test_empty_sidecar_pins_prevent_publication(self) -> None:
        # Arrange
        output = Path(self.directory.name) / "not-published"

        # Act / Assert
        with (
            self.fixture.patches(),
            patch.object(
                promoter, "SIDECAR_SHA256", {key: "" for key in promoter.KEYS}
            ),
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.publish(output)
        self.assertFalse(output.exists())

    def test_existing_destination_is_never_clobbered(self) -> None:
        # Arrange
        output = Path(self.directory.name)

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            promoter._check_output(output)

    def test_atomic_rename_refuses_existing_destination(self) -> None:
        # Arrange
        staging = Path(self.directory.name) / "staging"
        destination = Path(self.directory.name) / "existing"
        staging.mkdir()
        destination.mkdir()

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            promoter._publish_exclusive(staging, destination)
        self.assertTrue(staging.is_dir())
        self.assertTrue(destination.is_dir())


class SyntheticSidecars(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.sidecars: dict[tuple[str, str, str], Path] = {}
        self.hashes: dict[tuple[str, str, str], str] = {}
        self.payloads: dict[Path, dict[str, object]] = {}
        for index, key in enumerate(promoter.KEYS):
            mode, api, engine = key
            result = self.root / f"{mode}-{api}-{engine}.xml"
            classname = f"tests.{api}.test_browsertype_connect"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
                f'<testcase classname="{classname}" '
                f'name="{promoter.strict.NAME}[{engine}]" time="0.125"/>'
                "</testsuite></testsuites>"
            )
            sidecar = result.with_suffix(".provenance.json")
            sidecar.write_text(json.dumps({"synthetic": index}) + "\n")
            self.sidecars[key] = sidecar
            self.hashes[key] = promoter.sha256(sidecar.read_bytes())
            self.payloads[sidecar] = {
                "schema": 1,
                "kind": promoter.strict.KIND,
                "evidenceScope": promoter.strict.SCOPE,
                "mode": mode,
                "testId": promoter.target(api, engine),
                "manifest": str(promoter.strict.POLICY),
                "manifestSha256": promoter.POLICY_SHA256,
                "commit": promoter.strict.COMMIT,
                "wheelSha256": promoter.CANDIDATE_WHEEL_SHA256,
                "publicConnectPresent": True,
                "fixtureAdaptation": promoter.strict.ADAPTATION,
                "outcome": "passed",
                "exitCode": 0,
                "verifierSha256": promoter.TOOL_SHA256[
                    "scripts/verify_playwright_python_connect_refused_strict_v1.py"
                ],
                "result": str(result),
                "resultSha256": promoter.sha256(result.read_bytes()),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "callReceiptSha256": "4" * 64,
                "runNonce": f"{index + 1:032x}",
                "upstream": "/synthetic/upstream",
                "toolingRoot": str(promoter.ROOT),
                "toolingSources": {"synthetic": "5" * 64},
                "pythonVersion": "3.12.14",
                "pythonRuntimeSha256": "6" * 64,
                "wheelPath": "/synthetic/wheel.whl",
                "playwrightPackageSha256": "7" * 64,
                "driverNodeSha256": "8" * 64,
                "fixtureBind": "100.78.97.63",
                "fixtureHost": "machine.tail8f3b66.ts.net",
                "pythonExecutable": f"/synthetic/{mode}/python",
                "playwrightPackage": f"/synthetic/{mode}/playwright",
                "driverNode": f"/synthetic/{mode}/node",
                "pwrightPackageSha256": "9" * 64 if mode == "candidate" else None,
                "nativeSha256": "a" * 64 if mode == "candidate" else None,
                "runtimeLibraries": {"synthetic.dylib": "b" * 64}
                if mode == "candidate"
                else {},
            }

    def reports(self) -> dict[tuple[str, str, str], dict[str, object]]:
        called: list[Path] = []

        def verify(path: Path) -> dict[str, object]:
            called.append(path)
            return self.payloads[path]

        with (
            patch.object(promoter, "tools_ready"),
            patch.object(promoter, "SIDECARS", self.sidecars),
            patch.object(promoter, "SIDECAR_SHA256", self.hashes),
            patch.object(promoter.strict, "verify_provenance", side_effect=verify),
        ):
            result = promoter.reports()
        self.assertEqual(called, list(self.sidecars.values()))
        return result

    def test_all_twelve_unique_sidecars_are_required(self) -> None:
        # Arrange / Act
        reports = self.reports()

        # Assert
        self.assertEqual(set(reports), set(promoter.KEYS))
        self.assertTrue(
            all(report["_duration"] == 0.125 for report in reports.values())
        )

    def test_wrong_test_id_is_rejected(self) -> None:
        # Arrange
        first = next(iter(self.payloads.values()))
        first["testId"] = "different"

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self.reports()

    def test_duplicate_nonce_is_rejected(self) -> None:
        # Arrange
        payloads = list(self.payloads.values())
        payloads[1]["runNonce"] = payloads[0]["runNonce"]

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self.reports()

    def test_failed_case_is_rejected(self) -> None:
        # Arrange
        first = next(iter(self.payloads.values()))
        first["outcome"] = "failed"

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self.reports()

    def test_tampered_sidecar_is_rejected_before_verification(self) -> None:
        # Arrange
        first = next(iter(self.payloads))
        first.write_text('{"tampered":true}\n')

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self.reports()

    def test_wrong_junit_class_is_rejected(self) -> None:
        # Arrange
        first = next(iter(self.payloads.values()))
        result = Path(first["result"])
        result.write_text(result.read_text().replace("tests.async", "tests.sync"))
        first["resultSha256"] = promoter.sha256(result.read_bytes())

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self.reports()

    def test_missing_sidecar_is_rejected(self) -> None:
        # Arrange
        first = next(iter(self.payloads))
        first.unlink()

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self.reports()


if __name__ == "__main__":
    unittest.main()
