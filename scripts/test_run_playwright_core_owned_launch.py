#!/usr/bin/env python3
"""Browserless tests for the one-spec official Core owned-launch lane."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import cast

from run_playwright_core_owned_launch import (
    CANDIDATE_PRELOAD,
    LOCAL_SOURCES,
    MANIFEST,
    PROJECT,
    PROFILE,
    ROOT,
    SPEC_FILE,
    SPEC_ID,
    command,
    digest,
    pinned_checkout,
    pinned_manifest,
    passed_case,
    receipt_events,
    report_case,
    source_digest,
    verify,
)


UPSTREAM = ROOT.parents[2] / "thirdparty/playwright-v1.58.0"


def report(status: str = "expected") -> dict[str, object]:
    return {
        "suites": [
            {
                "specs": [
                    {
                        "id": SPEC_ID,
                        "file": SPEC_FILE,
                        "line": 130,
                        "title": "should be callable twice",
                        "tests": [
                            {
                                "projectId": PROJECT,
                                "expectedStatus": "passed",
                                "status": status,
                                "results": [{"status": "passed", "retry": 0}],
                            }
                        ],
                    }
                ],
                "suites": [],
            }
        ]
    }


class OwnedLaunchEvidenceTests(unittest.TestCase):
    def test_manifest_pins_exact_official_sources_and_checkout(self) -> None:
        # Arrange
        upstream = UPSTREAM
        if not upstream.is_dir():
            self.skipTest("pinned Playwright Core checkout is not present")

        # Act
        manifest = pinned_manifest(upstream)
        pinned_checkout(upstream, manifest)

        # Assert
        self.assertEqual(manifest["commit"], "961381ec73a9fa2661d92504eab1a6bd6eaffc75")
        self.assertEqual(cast(dict[str, object], manifest["suite"])["specId"], SPEC_ID)
        self.assertEqual(len(digest(MANIFEST)), 64)

    def test_report_accepts_only_the_selected_official_case(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            path = Path(directory) / "result.json"
            path.write_text(json.dumps(report()))

            # Act
            test = report_case(path)

            # Assert
            self.assertEqual(test["projectId"], PROJECT)
            self.assertEqual(test["status"], "expected")

    def test_report_rejects_an_extra_official_spec(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            path = Path(directory) / "result.json"
            extra = report()
            specs = cast(
                list[dict[str, object]],
                cast(dict[str, object], cast(list[object], extra["suites"])[0])[
                    "specs"
                ],
            )
            specs.append({**specs[0], "id": "extra"})
            path.write_text(json.dumps(extra))

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "exactly one spec"):
                report_case(path)

    def test_red_selected_result_remains_capturable_but_not_green(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            path = Path(directory) / "red.json"
            red = report("unexpected")
            test = cast(
                dict[str, object],
                cast(
                    list[object],
                    cast(dict[str, object], cast(list[object], red["suites"])[0])[
                        "specs"
                    ],
                )[0],
            )
            selected = cast(dict[str, object], cast(list[object], test["tests"])[0])
            selected["results"] = [{"status": "failed", "retry": 0}]
            path.write_text(json.dumps(red))

            # Act
            captured = report_case(path)

            # Assert
            self.assertFalse(passed_case(captured))

    def test_receipt_requires_one_successful_public_launch(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            receipt = Path(directory) / "receipt.jsonl"
            executable = Path("/test-only/Chrome")
            event = {
                "mode": "candidate",
                "publicMethod": "chromium.launch",
                "executablePath": str(executable),
                "headless": True,
            }
            receipt.write_text(json.dumps({**event, "event": "launch-invoked"}) + "\n")

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "one real public launch"):
                receipt_events(
                    receipt,
                    mode="candidate",
                    executable=executable,
                    require_success=True,
                )

    def test_verifier_binds_candidate_artifact_result_and_launch_receipt(self) -> None:
        # Arrange
        if not UPSTREAM.is_dir():
            self.skipTest("pinned Playwright Core checkout is not present")
        with TemporaryDirectory() as directory:
            root = Path(directory)
            candidate = root / "candidate"
            (candidate / "src").mkdir(parents=True)
            (candidate / "package.json").write_text('{"main":"src/index.cjs"}\n')
            (candidate / "src/index.cjs").write_text("module.exports = {};\n")
            artifact = candidate / "native/owned-launch/pwright.node"
            artifact.parent.mkdir(parents=True)
            artifact.write_bytes(b"fake-native-artifact-for-browserless-verifier-test")
            executable = Path(sys.executable).resolve()
            result = root / "case.json"
            result.write_text(json.dumps(report()))
            receipt = result.with_suffix(".launch-receipt.jsonl")
            events = [
                {
                    "event": name,
                    "mode": "candidate",
                    "publicMethod": "chromium.launch",
                    "executablePath": str(executable),
                    "headless": True,
                }
                for name in ("launch-invoked", "launch-succeeded")
            ]
            receipt.write_text("".join(json.dumps(event) + "\n" for event in events))
            manifest = pinned_manifest(UPSTREAM)
            sidecar = result.with_suffix(".provenance.json")
            provenance = {
                "schema": 1,
                "launchMode": "owned",
                "evidenceScope": "diagnostic-public-launch-no-process-profile-attestation",
                "mode": "candidate",
                "manifest": str(MANIFEST),
                "manifestSha256": digest(MANIFEST),
                "commit": manifest["commit"],
                "suite": cast(dict[str, object], manifest["suite"])["id"],
                "project": PROJECT,
                "specId": SPEC_ID,
                "upstream": str(UPSTREAM),
                "executable": str(executable),
                "executableSha256": digest(executable),
                "candidate": str(candidate),
                "candidateProfile": PROFILE,
                "candidateSourceSha256": source_digest(candidate),
                "candidateArtifactSha256": digest(artifact),
                "localSources": {
                    relative: digest(ROOT / relative)
                    for relative in (*LOCAL_SOURCES, CANDIDATE_PRELOAD)
                },
                "command": command(UPSTREAM),
                "exitCode": 0,
                "result": str(result),
                "resultSha256": digest(result),
                "receipt": str(receipt),
                "receiptSha256": digest(receipt),
            }
            sidecar.write_text(json.dumps(provenance))

            # Act
            verify(sidecar)
            result.write_text(json.dumps({"suites": []}))

            # Assert
            with self.assertRaisesRegex(
                ValueError, "result or launch receipt identity"
            ):
                verify(sidecar)


if __name__ == "__main__":
    unittest.main()
