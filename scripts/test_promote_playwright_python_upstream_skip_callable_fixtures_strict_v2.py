"""Synthetic mutation controls for the paired, skip-only Python promoter."""

from __future__ import annotations

import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from unittest.mock import patch

import promote_playwright_python_upstream_skip_callable_fixtures_strict_v2 as promoter


def encoded(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def synthetic_reports() -> dict[tuple[str, int], dict[str, object]]:
    result: dict[tuple[str, int], dict[str, object]] = {}
    for mode, ordinal in promoter.KEYS:
        test_id = promoter.target(ordinal)
        engine = test_id.rsplit("[", 1)[1].removesuffix("]")
        result[(mode, ordinal)] = {
            "mode": mode,
            "testId": test_id,
            "outcome": "skipped",
            "result": f"/synthetic/{mode}-{ordinal:03d}.xml",
            "resultSha256": "1" * 64,
            "callReceipt": f"/synthetic/{mode}-{ordinal:03d}.call.jsonl",
            "callReceiptSha256": "2" * 64,
            "_sidecarSha256": "3" * 64,
            "_duration": 0.125,
            "_reason": f"skipped for this browser: {engine}",
        }
    return result


class SyntheticBaseline:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.outcomes = root / "outcomes"
        self.outcomes.mkdir()
        self.base_dir = self.outcomes / "base"
        self.base_dir.mkdir()
        self.output_dir = self.outcomes / "successor"
        self.inventory = root / "inventory.jsonl"
        source = root / "scripts/promote_playwright_python_upstream_skip_markers_strict_v1.py"
        source.parent.mkdir()
        source.write_bytes(b"synthetic predecessor source\n")
        self.source = source
        targets = dict(zip(promoter.TARGET_INDEXES, promoter.strict.TEST_IDS))
        inventory_rows = [
            {
                "api": "async" if index < 4_000 else "sync",
                "file": "synthetic.py",
                "id": targets.get(index, f"synthetic::{index}"),
                "privateImports": [],
                "sourceClassification": "public-api-source",
            }
            for index in range(promoter.RECORDS)
        ]
        self.inventory.write_bytes(b"".join(map(encoded, inventory_rows)))
        self.inventory_sha = promoter.sha256(self.inventory.read_bytes())
        self.rows = [{**row, "attempts": 0, "outcome": "not-run"} for row in inventory_rows]
        self.bases: dict[str, tuple[str, str]] = {}
        summaries: dict[str, dict[str, object]] = {}
        for mode in promoter.MODES:
            ledger = self.base_dir / promoter.base_name(mode)
            ledger.write_bytes(b"".join(map(encoded, self.rows)))
            ledger_sha = promoter.sha256(ledger.read_bytes())
            summary = {
                "schema": 1,
                "inventory": str(self.inventory),
                "records": promoter.RECORDS,
                "outcomes": dict(sorted(Counter(row["outcome"] for row in self.rows).items())),
                "ledgerSha256": ledger_sha,
                "ledger": str(ledger),
            }
            summary_path = ledger.with_suffix(".summary.json")
            summary_path.write_bytes((json.dumps(summary, indent=2) + "\n").encode())
            self.bases[mode] = (
                ledger_sha,
                promoter.sha256(summary_path.read_bytes()),
            )
            summaries[mode] = summary
        self.receipt = {
            "kind": promoter.prior.RECEIPT_KIND,
            "inventory": str(self.inventory),
            "inventorySha256": self.inventory_sha,
            "recordsPerLane": promoter.RECORDS,
            "commit": promoter.strict.COMMIT,
            "promoterSha256": promoter.sha256(source.read_bytes()),
            "publicApiParityEvidence": False,
            "successors": summaries,
        }
        receipt_path = self.base_dir / "promotion-receipt.json"
        receipt_path.write_bytes((json.dumps(self.receipt, indent=2) + "\n").encode())
        self.receipt_sha = promoter.sha256(receipt_path.read_bytes())

    def patches(self) -> tuple[object, object, object]:
        return (
            patch.multiple(
                promoter,
                ROOT=self.root,
                OUTCOMES=self.outcomes,
                BASE_DIR=self.base_dir,
                OUTPUT_DIR=self.output_dir,
                INVENTORY=self.inventory,
                INVENTORY_SHA256=self.inventory_sha,
                BASE_PROMOTER_SHA256=promoter.sha256(self.source.read_bytes()),
                BASE_RECEIPT_SHA256=self.receipt_sha,
                BASES=self.bases,
            ),
            patch.object(promoter.prior, "__file__", str(self.source)),
            patch.object(promoter.prior, "verify_bundle", return_value=self.receipt),
        )


class BaselineAndSuccessorTests(unittest.TestCase):
    def setUp(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.fixture = SyntheticBaseline(Path(directory.name).resolve())

    def test_base_accepts_all_eleven_exact_not_run_rows(self) -> None:
        # Arrange / Act
        with (
            self.fixture.patches()[0],
            self.fixture.patches()[1],
            self.fixture.patches()[2],
        ):
            originals = promoter.base()

        # Assert
        for mode in promoter.MODES:
            _, rows = originals[mode]
            self.assertEqual(
                [rows[index]["id"] for index in promoter.TARGET_INDEXES],
                list(promoter.strict.TEST_IDS),
            )
            self.assertTrue(
                all(
                    rows[index] == {**self.fixture.rows[index]} for index in promoter.TARGET_INDEXES
                )
            )

    def test_modified_base_ledger_is_rejected(self) -> None:
        # Arrange
        ledger = self.fixture.base_dir / promoter.base_name("candidate")
        ledger.write_bytes(ledger.read_bytes() + b"tampered\n")

        # Act / Assert
        with (
            self.fixture.patches()[0],
            self.fixture.patches()[1],
            self.fixture.patches()[2],
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.base()

    def test_relocated_inventory_target_is_rejected(self) -> None:
        # Arrange
        indexes = list(promoter.TARGET_INDEXES)
        indexes[0], indexes[1] = indexes[1], indexes[0]

        # Act / Assert
        with (
            self.fixture.patches()[0],
            self.fixture.patches()[1],
            self.fixture.patches()[2],
            patch.object(promoter, "TARGET_INDEXES", tuple(indexes)),
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.base()

    def test_successor_changes_only_eleven_raw_lines_per_lane(self) -> None:
        # Arrange
        with (
            self.fixture.patches()[0],
            self.fixture.patches()[1],
            self.fixture.patches()[2],
        ):
            originals = promoter.base()
        reports = synthetic_reports()

        # Act / Assert
        for mode in promoter.MODES:
            with self.fixture.patches()[0]:
                ledger, _, summary = promoter.successor(
                    mode,
                    originals[mode],
                    reports,
                    Path(f"/synthetic/{mode}.jsonl"),
                )
            after = ledger.splitlines(keepends=True)
            changed = [
                index
                for index, (old, new) in enumerate(zip(originals[mode][0], after))
                if old != new
            ]
            self.assertEqual(changed, sorted(promoter.TARGET_INDEXES))
            self.assertEqual(summary["outcomes"], {"not-run": 6515, "skipped": 11})
            for index in changed:
                row = json.loads(after[index])
                self.assertEqual(row["sourceClassification"], "public-api-source")
                self.assertEqual(row["outcome"], "skipped")
                self.assertEqual(row["attempts"], 1)
                self.assertEqual(row["durationSeconds"], 0.125)
                self.assertEqual(
                    {key: value for key, value in row.items() if key not in promoter.GENERATED},
                    {
                        key: value
                        for key, value in originals[mode][1][index].items()
                        if key not in promoter.GENERATED
                    },
                )

    def test_moved_target_and_pass_claim_are_rejected(self) -> None:
        # Arrange
        with (
            self.fixture.patches()[0],
            self.fixture.patches()[1],
            self.fixture.patches()[2],
        ):
            originals = promoter.base()
        lines, rows = originals["candidate"]
        altered = list(rows)
        index = promoter.TARGET_INDEXES[0]
        altered[index] = {**rows[index], "id": "moved"}
        reports = synthetic_reports()

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            promoter.successor("candidate", (lines, altered), reports, Path("/synthetic/out"))
        reports[("candidate", 0)]["outcome"] = "passed"
        with self.assertRaises(promoter.PromotionError):
            promoter.successor("candidate", (lines, rows), reports, Path("/synthetic/out"))

    def test_receipt_grants_zero_public_behavior_credit(self) -> None:
        # Arrange
        reports = synthetic_reports()

        # Act
        with (
            self.fixture.patches()[0],
            self.fixture.patches()[1],
            self.fixture.patches()[2],
            patch.object(promoter, "reports", return_value=reports),
        ):
            content, receipt = promoter.prepare(self.fixture.output_dir)

        # Assert
        self.assertFalse(self.fixture.output_dir.exists())
        self.assertEqual(len(content), 5)
        self.assertEqual(len(receipt["reports"]), 22)
        self.assertEqual(receipt["newPublicBehaviorPasses"], 0)
        self.assertEqual(receipt["newPublicBehaviorParityCredit"], 0)
        self.assertFalse(receipt["focusedPublicBehaviorEvidenceThisPromotion"])
        self.assertFalse(receipt["publicApiParityEvidence"])
        self.assertFalse(receipt["fullPlaywrightServerSupport"])

    def test_existing_bundle_tamper_is_rejected_without_replacement(self) -> None:
        # Arrange
        reports = synthetic_reports()
        with (
            self.fixture.patches()[0],
            self.fixture.patches()[1],
            self.fixture.patches()[2],
            patch.object(promoter, "reports", return_value=reports),
        ):
            promoter.publish(self.fixture.output_dir)
            promoter.verify_bundle(self.fixture.output_dir)
            ledger = self.fixture.output_dir / promoter.output_name("oracle")
            ledger.write_bytes(ledger.read_bytes() + b"tampered\n")

            # Act / Assert
            with self.assertRaises(promoter.PromotionError):
                promoter.verify_bundle(self.fixture.output_dir)
            with self.assertRaises(promoter.PromotionError):
                promoter.publish(self.fixture.output_dir)


class SidecarMutationTests(unittest.TestCase):
    def setUp(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.result_dir = Path(directory.name).resolve()
        self.sidecars: dict[tuple[str, int], Path] = {}
        self.pins: dict[tuple[str, int], str] = {}
        self.payloads: dict[Path, dict[str, object]] = {}
        for mode, ordinal in promoter.KEYS:
            test_id = promoter.target(ordinal)
            engine = test_id.rsplit("[", 1)[1].removesuffix("]")
            stem = f"{mode}-{ordinal:03d}"
            result = self.result_dir / f"{stem}.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" skipped="1" failures="0" errors="0">'
                f'<testcase time="0.125"><skipped type="pytest.skip" '
                f'message="skipped for this browser: {engine}"/></testcase>'
                "</testsuite></testsuites>"
            )
            call = result.with_suffix(".call.jsonl")
            call.write_bytes(b"synthetic receipt\n")
            sidecar = result.with_suffix(".provenance.json")
            sidecar.write_bytes(encoded({"mode": mode, "ordinal": ordinal}))
            self.sidecars[(mode, ordinal)] = sidecar
            self.pins[(mode, ordinal)] = promoter.sha256(sidecar.read_bytes())
            self.payloads[sidecar] = {
                "mode": mode,
                "testId": test_id,
                "kind": promoter.strict.KIND,
                "evidenceScope": promoter.strict.SCOPE,
                "manifestSha256": promoter.POLICY_SHA256,
                "commit": promoter.strict.COMMIT,
                "outcome": "skipped",
                "exitCode": 0,
                "runnerSha256": promoter.sha256(promoter.strict.RUNNER.read_bytes()),
                "verifierSha256": promoter.sha256(Path(promoter.strict.__file__).read_bytes()),
                "guardSha256": promoter.sha256(
                    (
                        promoter.strict.GUARD / "pwright_official_upstream_skip_guard_v1.py"
                    ).read_bytes()
                ),
                "result": str(result),
                "resultSha256": promoter.sha256(result.read_bytes()),
                "callReceipt": str(call),
                "callReceiptSha256": promoter.sha256(call.read_bytes()),
                "runNonce": f"{len(self.payloads) + 1:032x}",
                "upstream": "/synthetic/upstream",
                "manifest": str(promoter.strict.POLICY),
                "fixtureBind": "100.64.0.1",
                "fixtureHost": "synthetic-yuacx.tail8f3b66.ts.net",
                "artifact": {"mode": mode},
            }
        self.tools = {
            relative: promoter.sha256((promoter.ROOT / relative).read_bytes())
            for relative in promoter.TOOL_SHA256
        }

    def pair(self, candidate: Path, oracle: Path) -> tuple[dict[str, object], dict[str, object]]:
        return self.payloads[candidate], self.payloads[oracle]

    def patches(self) -> tuple[object, object, object, object, object]:
        return (
            patch.object(promoter, "RESULT_DIR", self.result_dir),
            patch.object(promoter, "SIDECARS", self.sidecars),
            patch.object(promoter, "SIDECAR_SHA256", self.pins),
            patch.object(promoter, "TOOL_SHA256", self.tools),
            patch.object(promoter.strict, "verify_pair", side_effect=self.pair),
        )

    def test_twenty_two_pinned_pairs_verify_as_skip_only(self) -> None:
        # Arrange / Act
        with (
            self.patches()[0],
            self.patches()[1],
            self.patches()[2],
            self.patches()[3],
            self.patches()[4],
        ):
            reports = promoter.reports()

        # Assert
        self.assertEqual(set(reports), set(promoter.KEYS))
        self.assertTrue(
            all(
                report["_reason"].startswith("skipped for this browser:")
                for report in reports.values()
            )
        )

    def test_missing_and_wrong_sidecar_pins_are_rejected(self) -> None:
        # Arrange
        absent = dict(self.pins)
        absent.pop(("oracle", 10))
        wrong = dict(self.pins)
        wrong[("oracle", 10)] = "0" * 64

        # Act / Assert
        with (
            self.patches()[0],
            self.patches()[1],
            patch.object(promoter, "SIDECAR_SHA256", absent),
            self.patches()[3],
            self.patches()[4],
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.reports()
        with (
            self.patches()[0],
            self.patches()[1],
            patch.object(promoter, "SIDECAR_SHA256", wrong),
            self.patches()[3],
            self.patches()[4],
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.reports()

    def test_mispaired_case_and_invalid_duration_are_rejected(self) -> None:
        # Arrange
        payload = self.payloads[self.sidecars[("oracle", 0)]]
        old_id = payload["testId"]
        payload["testId"] = promoter.target(1)

        # Act / Assert
        with (
            self.patches()[0],
            self.patches()[1],
            self.patches()[2],
            self.patches()[3],
            self.patches()[4],
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.reports()
        payload["testId"] = old_id
        result = Path(payload["result"])
        result.write_bytes(result.read_bytes().replace(b'time="0.125"', b'time="nan"'))
        payload["resultSha256"] = promoter.sha256(result.read_bytes())
        with (
            self.patches()[0],
            self.patches()[1],
            self.patches()[2],
            self.patches()[3],
            self.patches()[4],
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.reports()


if __name__ == "__main__":
    unittest.main()
