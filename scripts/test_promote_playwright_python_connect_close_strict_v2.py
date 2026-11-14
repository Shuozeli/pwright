"""Synthetic-only controls for the unready connect-close v2 paired promoter."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import promote_playwright_python_connect_close_strict_v2 as promoter


def synthetic_reports() -> dict[tuple[str, str], dict[str, object]]:
    return {
        (mode, engine): {
            "result": f"/synthetic/{mode}-{engine}.xml",
            "resultSha256": "1" * 64,
            "callReceiptSha256": "2" * 64,
            "_duration": 0.125,
            "_sidecarSha256": "3" * 64,
        }
        for mode, engine in promoter.KEYS
    }


class BaseAndSuccessorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.originals = promoter.base()

    def test_sealed_base_has_exact_red_and_oracle_targets(self) -> None:
        # Arrange / Act / Assert
        for mode in promoter.MODES:
            lines, rows = self.originals[mode]
            self.assertEqual(len(lines), promoter.RECORDS)
            self.assertEqual(len(rows), promoter.RECORDS)
            self.assertEqual(
                [rows[index]["id"] for index in promoter.TARGET_INDEXES],
                list(promoter.strict.TEST_IDS),
            )
            self.assertEqual(
                {rows[index]["outcome"] for index in promoter.TARGET_INDEXES},
                {"failed" if mode == "candidate" else "passed"},
            )

    def test_candidate_changes_only_three_target_lines(self) -> None:
        # Arrange
        originals = self.originals["candidate"]

        # Act
        ledger, _, summary = promoter.successor(
            "candidate", originals, synthetic_reports(), Path("/synthetic/candidate.jsonl")
        )

        # Assert
        new_lines = ledger.splitlines(keepends=True)
        changed = [
            index
            for index, (before, after) in enumerate(zip(originals[0], new_lines))
            if before != after
        ]
        self.assertEqual(changed, list(promoter.TARGET_INDEXES))
        self.assertEqual(summary["outcomes"]["failed"], 102)
        self.assertEqual(summary["outcomes"]["passed"], 1536)
        for index in changed:
            row = json.loads(new_lines[index])
            self.assertEqual(row["outcome"], "passed")
            self.assertNotIn("error", row)
            self.assertEqual(row["sourceClassification"], "public-api-source")

    def test_oracle_preserves_every_prior_proof_byte(self) -> None:
        # Arrange
        originals = self.originals["oracle"]

        # Act
        ledger, _, summary = promoter.successor(
            "oracle", originals, synthetic_reports(), Path("/synthetic/oracle.jsonl")
        )

        # Assert
        self.assertEqual(ledger, b"".join(originals[0]))
        self.assertEqual(summary["outcomes"]["passed"], 1303)

    def test_candidate_rejects_moved_target(self) -> None:
        # Arrange
        lines, rows = self.originals["candidate"]
        changed = list(rows)
        changed[promoter.TARGET_INDEXES[0]] = {
            **changed[promoter.TARGET_INDEXES[0]],
            "id": "different",
        }

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            promoter.successor(
                "candidate", (lines, changed), synthetic_reports(), Path("/synthetic/out.jsonl")
            )

    def test_oracle_rejects_lost_prior_pass(self) -> None:
        # Arrange
        lines, rows = self.originals["oracle"]
        changed = list(rows)
        changed[promoter.TARGET_INDEXES[1]] = {
            **changed[promoter.TARGET_INDEXES[1]],
            "outcome": "failed",
        }

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            promoter.successor(
                "oracle", (lines, changed), synthetic_reports(), Path("/synthetic/out.jsonl")
            )

    def test_prepared_receipt_disclaims_full_support(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "not-published"

            # Act
            with patch.object(promoter, "reports", return_value=synthetic_reports()):
                content, receipt = promoter.prepare(output)

            # Assert
            self.assertFalse(output.exists())
            self.assertFalse(receipt["fullPlaywrightServerSupport"])
            self.assertFalse(receipt["publicApiParityEvidence"])
            self.assertEqual(len(receipt["reports"]), 6)
            self.assertEqual(sum(report["ledgerRowUpdated"] for report in receipt["reports"]), 3)
            oracle_name = "playwright-python-oracle-browser-connect-close-strict-paired-v2.jsonl"
            self.assertEqual(content[oracle_name], b"".join(self.originals["oracle"][0]))

    def test_unready_tooling_fails_before_sidecars(self) -> None:
        # Arrange / Act / Assert
        with (
            patch.object(promoter, "POLICY_SHA256", ""),
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.tools_ready()

    def test_changed_capture_source_pin_is_rejected(self) -> None:
        # Arrange
        manifest_sha = promoter.sha256(promoter.strict.POLICY.read_bytes())
        paths = set(promoter.strict._CAPTURE_PATHS) | {
            "scripts/verify_playwright_python_connect_close_strict_v2.py"
        }
        tool_hashes = {path: promoter.sha256((promoter.ROOT / path).read_bytes()) for path in paths}
        tool_hashes[promoter.strict._CAPTURE_PATHS[0]] = "0" * 64
        sidecar_hashes = {key: "1" * 64 for key in promoter.KEYS}

        # Act / Assert
        with (
            patch.object(promoter, "POLICY_SHA256", manifest_sha),
            patch.object(promoter.strict, "POLICY_SHA256", manifest_sha),
            patch.object(
                promoter.strict, "_policy", return_value={"candidateWheelSha256": "2" * 64}
            ),
            patch.object(promoter, "CANDIDATE_WHEEL_SHA256", "2" * 64),
            patch.object(promoter, "TOOL_SHA256", tool_hashes),
            patch.object(promoter, "SIDECAR_SHA256", sidecar_hashes),
            self.assertRaises(promoter.PromotionError),
        ):
            promoter.tools_ready()

    def test_existing_output_directory_is_never_clobbered(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)

            # Act / Assert
            with self.assertRaises(promoter.PromotionError):
                promoter._check_output(output)

    def test_atomic_publication_refuses_existing_destination(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            staging = root / "staging"
            destination = root / "existing"
            staging.mkdir()
            destination.mkdir()

            # Act / Assert
            with self.assertRaises(promoter.PromotionError):
                promoter._publish_exclusive(staging, destination)
            self.assertTrue(staging.is_dir())
            self.assertTrue(destination.is_dir())


class SyntheticSidecarTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.sidecars: dict[tuple[str, str], Path] = {}
        self.hashes: dict[tuple[str, str], str] = {}
        self.payloads: dict[Path, dict[str, object]] = {}
        for index, (mode, engine) in enumerate(promoter.KEYS):
            result = self.root / f"{mode}-{engine}-v1.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
                '<testcase classname="tests.async.test_browsertype_connect" '
                f'name="{promoter.strict.NAME}[{engine}]" time="0.125"/>'
                "</testsuite></testsuites>"
            )
            sidecar = result.with_suffix(".provenance.json")
            sidecar.write_text(json.dumps({"synthetic": index}) + "\n")
            key = (mode, engine)
            self.sidecars[key] = sidecar
            self.hashes[key] = promoter.sha256(sidecar.read_bytes())
            self.payloads[sidecar] = {
                "schema": 2,
                "kind": promoter.strict.KIND,
                "evidenceScope": promoter.strict.SCOPE,
                "mode": mode,
                "testId": promoter.target(engine),
                "manifest": str(promoter.strict.POLICY),
                "manifestSha256": "a" * 64,
                "commit": promoter.strict.COMMIT,
                "wheelSha256": "b" * 64,
                "publicConnectPresent": True,
                "fixtureTransportAdaptation": promoter.strict.ADAPTATION,
                "outcome": "passed",
                "exitCode": 0,
                "verifierSha256": "c" * 64,
                "result": str(result),
                "resultSha256": promoter.sha256(result.read_bytes()),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "runNonce": f"{index + 1:032x}",
                "upstream": "/synthetic/upstream",
                "toolingRoot": str(promoter.ROOT),
                "toolingSources": {"synthetic": "e" * 64},
                "pythonVersion": "3.12.14",
                "pythonRuntimeSha256": "f" * 64,
                "wheelPath": "/synthetic/wheel.whl",
                "playwrightPackageSha256": "1" * 64,
                "driverNodeSha256": "2" * 64,
                "fixtureBind": "100.78.97.63",
                "fixtureHost": "machine.tail8f3b66.ts.net",
                "fixturePort": 48000 + index,
                "pythonExecutable": f"/synthetic/{mode}/python",
                "playwrightPackage": f"/synthetic/{mode}/playwright",
                "driverNode": f"/synthetic/{mode}/node",
                "pwrightPackageSha256": "3" * 64 if mode == "candidate" else None,
                "nativeSha256": "4" * 64 if mode == "candidate" else None,
                "runtimeLibraries": {"synthetic.dylib": "5" * 64} if mode == "candidate" else {},
            }
            payload = self.payloads[sidecar]
            common = {
                "schema": 2,
                "mode": mode,
                "testId": promoter.target(engine),
                "runNonce": payload["runNonce"],
            }
            port = payload["fixturePort"]
            receipt = Path(payload["callReceipt"])
            events = [
                {
                    **common,
                    "event": "publicCall",
                    "browserName": engine,
                    "method": "connect",
                    "fixturePort": port,
                    "originalEndpoint": f"ws://localhost:{port}/ws",
                    "forwardedEndpoint": f"ws://{payload['fixtureHost']}:{port}/ws",
                },
                {**common, "event": "open"},
                {
                    **common,
                    "event": "message",
                    "payloadLength": 4,
                    "payloadSha256": "d" * 64,
                    "isBinary": False,
                    "guid": "",
                    "method": "initialize",
                    "sdkLanguage": "python",
                    "messageId": 1,
                    "apiName": "Connection.init",
                    "wallTime": 1720000000000,
                },
                {**common, "event": "sendClose", "code": 4123, "reason": "Oh my!"},
            ]
            receipt.write_text("".join(json.dumps(event) + "\n" for event in events))
            payload["callReceiptSha256"] = promoter.sha256(receipt.read_bytes())

    def _reports(self) -> dict[tuple[str, str], dict[str, object]]:
        called: list[Path] = []

        def fake_verify(path: Path) -> dict[str, object]:
            called.append(path)
            promoter.strict._receipt_events(self.payloads[path])
            return self.payloads[path]

        with (
            patch.object(promoter, "tools_ready"),
            patch.object(promoter, "SIDECARS", self.sidecars),
            patch.object(promoter, "SIDECAR_SHA256", self.hashes),
            patch.object(promoter, "POLICY_SHA256", "a" * 64),
            patch.object(promoter, "CANDIDATE_WHEEL_SHA256", "b" * 64),
            patch.object(
                promoter,
                "TOOL_SHA256",
                {"scripts/verify_playwright_python_connect_close_strict_v2.py": "c" * 64},
            ),
            patch.object(promoter.strict, "verify_provenance", side_effect=fake_verify),
        ):
            reports = promoter.reports()
        self.assertEqual(called, list(self.sidecars.values()))
        return reports

    def test_requires_six_independent_verified_sidecars(self) -> None:
        # Arrange / Act
        reports = self._reports()

        # Assert
        self.assertEqual(set(reports), set(promoter.KEYS))
        self.assertTrue(all(report["_duration"] == 0.125 for report in reports.values()))

    def test_rejects_duplicate_nonce_across_lanes(self) -> None:
        # Arrange
        paths = list(self.payloads)
        second = self.payloads[paths[1]]
        second["runNonce"] = self.payloads[paths[0]]["runNonce"]
        receipt = Path(second["callReceipt"])
        events = [json.loads(line) for line in receipt.read_text().splitlines()]
        for event in events:
            event["runNonce"] = second["runNonce"]
        receipt.write_text("".join(json.dumps(event) + "\n" for event in events))
        second["callReceiptSha256"] = promoter.sha256(receipt.read_bytes())

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self._reports()

    def test_rejects_candidate_sidecar_failure(self) -> None:
        # Arrange
        first = next(iter(self.payloads))
        self.payloads[first]["outcome"] = "failed"

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self._reports()

    def test_rejects_tampered_sidecar_before_verification(self) -> None:
        # Arrange
        first = next(iter(self.payloads))
        first.write_text('{"tampered":true}\n')

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self._reports()

    def test_rejects_fixture_transcript_with_changed_close(self) -> None:
        # Arrange
        first = self.payloads[next(iter(self.payloads))]
        receipt = Path(first["callReceipt"])
        events = [json.loads(line) for line in receipt.read_text().splitlines()]
        events[-1]["reason"] = "not the upstream reason"
        receipt.write_text("".join(json.dumps(event) + "\n" for event in events))
        first["callReceiptSha256"] = promoter.sha256(receipt.read_bytes())

        # Act / Assert
        with self.assertRaises(promoter.strict.StrictWebSocketEvidenceError):
            self._reports()

    def test_rejects_missing_sidecar(self) -> None:
        # Arrange
        first = next(iter(self.payloads))
        first.unlink()

        # Act / Assert
        with self.assertRaises(promoter.PromotionError):
            self._reports()


if __name__ == "__main__":
    unittest.main()
