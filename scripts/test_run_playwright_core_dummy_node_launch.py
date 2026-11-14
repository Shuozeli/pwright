#!/usr/bin/env python3
"""Negative-only, browserless checks for the isolated dummy-Node Core policy."""

from __future__ import annotations

import json
import os
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import run_playwright_core_dummy_node_launch as dummy
import run_playwright_official_conformance as general
import verify_playwright_core_dummy_node_launch_strict_v1 as strict


UPSTREAM = dummy.ROOT.parents[2] / "thirdparty/playwright-v1.58.0"


class DummyNodeLaunchPolicyTests(unittest.TestCase):
    def test_strict_manifest_pins_current_source_and_artifact(self) -> None:
        # Arrange
        manifest = json.loads(dummy.STRICT_MANIFEST.read_text())
        candidate = dummy.ROOT / "packages/pwright"
        artifact = candidate / "native/playwright-trace/pwright.node"

        # Act
        selected = dummy.pinned_manifest(UPSTREAM, dummy.STRICT_MANIFEST)

        # Assert
        self.assertEqual(selected, manifest)
        self.assertEqual(dummy.source_digest(candidate), manifest["suite"]["candidateSourceSha256"])
        self.assertEqual(dummy.digest(artifact), manifest["suite"]["candidateArtifactSha256"])

    def test_strict_report_rejects_silent_skip(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            report = {
                "config": {"workers": 1, "projects": [{"id": dummy.PROJECT, "retries": 0}]},
                "errors": [],
                "stats": {"expected": 0, "unexpected": 0, "skipped": 1, "flaky": 0},
                "suites": [{"specs": [{
                    "id": dummy.SPEC_ID, "file": dummy.SPEC_FILE, "line": 64,
                    "column": 3, "title": dummy.TITLE, "ok": True,
                    "tests": [{
                        "projectId": dummy.PROJECT, "expectedStatus": "passed",
                        "status": "skipped", "annotations": [{"type": "skip"}],
                        "results": [{"status": "skipped", "retry": 0, "workerIndex": 0, "errors": []}],
                    }],
                }]}],
            }
            result.write_text(json.dumps(report))

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "zero skips"):
                dummy.strict_report_case(result)

    def test_strict_process_receipt_rejects_forged_profile_inode(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            receipt = Path(directory) / "process.jsonl"
            profile = Path(directory) / "pwright-node-removed"
            node = Path("/opt/homebrew/bin/node")
            uid = os.getuid()
            event = {
                "event": "dummy-node-exited", "exitCode": 1,
                "asset": str(UPSTREAM / dummy.ASSET), "runId": "a" * 32,
                "mode": "candidate", "guardPid": 21, "guardParentPid": 22,
                "childPid": 24, "childReaped": True,
                "childSnapshot": {"pid": 24, "ppid": 21, "uid": uid, "executable": str(node)},
                "ancestry": [
                    {"pid": 21, "ppid": 22, "uid": uid, "executable": "/usr/bin/python3"},
                    {"pid": 22, "ppid": 23, "uid": uid, "executable": str(node)},
                    {"pid": 23, "ppid": 1, "uid": uid, "executable": "/usr/bin/python3"},
                ],
                "candidateProfile": dummy.TRACE_PROFILE,
                "harnessPid": 23, "browserProfilePath": str(profile),
                "browserProfileStat": {"device": 1, "inode": 0, "uid": uid},
                "browserArguments": [f"--user-data-dir={profile}"],
            }
            receipt.write_text(json.dumps(event) + "\n")

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "owned profile"):
                dummy.strict_process_receipt(
                    receipt, run_id="a" * 32, mode="candidate",
                    executable=UPSTREAM / dummy.ASSET, node=node,
                )

    def test_strict_pair_rejects_reused_run_identity(self) -> None:
        # Arrange
        candidate = {"mode": "candidate", "result": "/tmp/candidate.json", "runId": "a" * 32}
        oracle = {"mode": "oracle", "result": "/tmp/oracle.json", "runId": "a" * 32}

        # Act and Assert
        with (
            patch.object(strict, "verify_strict", side_effect=[candidate, oracle]),
            self.assertRaisesRegex(ValueError, "distinct same-case"),
        ):
            strict.verify_pair(Path("candidate.strict.json"), Path("oracle.strict.json"))

    def test_manifest_path_normalizes_relative_input_but_rejects_link(self) -> None:
        # Arrange
        relative = Path(os.path.relpath(dummy.TRACE_MANIFEST, Path.cwd()))
        with TemporaryDirectory() as directory:
            link = Path(directory) / "manifest.json"
            link.symlink_to(dummy.TRACE_MANIFEST)

            # Act
            selected = dummy.checked_manifest_path(relative)

            # Assert
            self.assertEqual(selected, dummy.TRACE_MANIFEST)
            with self.assertRaisesRegex(ValueError, "dedicated local file"):
                dummy.checked_manifest_path(link)

    def test_trace_manifest_selects_only_the_pinned_trace_profile(self) -> None:
        # Arrange
        selected = dummy.TRACE_MANIFEST

        # Act
        profile, source_sha, artifact_sha = dummy.candidate_policy(selected)
        manifest = json.loads(selected.read_text())

        # Assert
        self.assertEqual(profile, "playwright-trace")
        self.assertEqual(source_sha, manifest["suite"]["candidateSourceSha256"])
        self.assertEqual(artifact_sha, manifest["suite"]["candidateArtifactSha256"])
        self.assertEqual(manifest["suite"]["candidateProfile"], profile)
        self.assertNotEqual(dummy.candidate_policy(dummy.MANIFEST)[0], profile)
        if UPSTREAM.is_dir():
            self.assertEqual(dummy.pinned_manifest(UPSTREAM, selected), manifest)

    def test_trace_manifest_is_not_accepted_through_the_general_runner(self) -> None:
        # Arrange
        argv = [
            "run_playwright_official_conformance.py",
            "oracle",
            str(UPSTREAM),
            "--collect-only",
            "--manifest",
            str(dummy.TRACE_MANIFEST),
        ]

        # Act and Assert
        with (
            patch.object(sys, "argv", argv),
            patch.object(
                general.subprocess,
                "run",
                side_effect=AssertionError("subprocess bypass"),
            ) as process,
        ):
            with self.assertRaisesRegex(SystemExit, "unsupported manifest schema: 2"):
                general.main()
            process.assert_not_called()

    def test_general_runner_rejects_schema_before_any_subprocess(self) -> None:
        # Arrange
        argv = [
            "run_playwright_official_conformance.py",
            "oracle",
            str(UPSTREAM),
            "--collect-only",
            "--manifest",
            str(dummy.MANIFEST),
        ]

        # Act and Assert
        with (
            patch.object(sys, "argv", argv),
            patch.object(
                general.subprocess,
                "run",
                side_effect=AssertionError("subprocess bypass"),
            ) as process,
        ):
            with self.assertRaisesRegex(SystemExit, "unsupported manifest schema: 2"):
                general.main()
            process.assert_not_called()

    def test_wrapper_rejects_unrelated_historical_manifest(self) -> None:
        # Arrange
        historical = (
            dummy.ROOT
            / "tests/official/playwright-core-browserless-launch-invalid-executable-manifest.json"
        )
        before = dummy.digest(historical)

        # Act and Assert
        with self.assertRaisesRegex(ValueError, "dedicated local file"):
            dummy.pinned_manifest(UPSTREAM, historical)
        self.assertEqual(json.loads(historical.read_text())["schema"], 1)
        self.assertEqual(dummy.digest(historical), before)

    def test_unrelated_schema_one_manifest_still_reaches_mocked_general_runner(
        self,
    ) -> None:
        # Arrange
        historical = (
            dummy.ROOT
            / "tests/official/playwright-core-browserless-launch-invalid-executable-manifest.json"
        )
        argv = [
            "run_playwright_official_conformance.py",
            "oracle",
            str(UPSTREAM),
            "--collect-only",
            "--manifest",
            str(historical),
            "--suite",
            "library-browsertype-launch-invalid-executable-chromium",
        ]
        commands: list[list[str]] = []

        def stop_before_process(command: list[str], **_kwargs: object) -> None:
            commands.append(command)
            raise RuntimeError("mocked official runner boundary")

        # Act and Assert
        with (
            patch.object(sys, "argv", argv),
            patch.object(general, "_git", side_effect=[dummy.COMMIT, ""]),
            patch.object(general.subprocess, "run", side_effect=stop_before_process),
        ):
            with self.assertRaisesRegex(
                RuntimeError, "mocked official runner boundary"
            ):
                general.main()
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][0], "node")
        self.assertIn("--list", commands[0])

    def test_wrapper_rejects_selector_or_policy_bypass(self) -> None:
        if not UPSTREAM.is_dir():
            self.skipTest("pinned Core checkout is unavailable")
        # Arrange
        original = json.loads(dummy.MANIFEST.read_text())
        with TemporaryDirectory() as directory:
            altered = Path(directory) / "dummy.json"
            for key, value in (
                ("selector", "tests/library/browsertype-launch.spec.ts:75"),
                ("fixtureMode", None),
                ("executable", "/Applications/Google Chrome.app"),
            ):
                changed = json.loads(json.dumps(original))
                changed["suite"][key] = value
                altered.write_text(json.dumps(changed))

                # Act and Assert
                with patch.object(dummy, "MANIFEST", altered.resolve()):
                    with self.assertRaisesRegex(
                        ValueError, "not the exact one-spec policy"
                    ):
                        dummy.pinned_manifest(UPSTREAM, altered.resolve())
            changed = json.loads(json.dumps(original))
            changed["executionPolicy"] = "none"
            altered.write_text(json.dumps(changed))
            with patch.object(dummy, "MANIFEST", altered.resolve()):
                with self.assertRaisesRegex(ValueError, "execution policy differs"):
                    dummy.pinned_manifest(UPSTREAM, altered.resolve())

    def test_wrapper_rejects_inherited_cdp_or_node_override(self) -> None:
        # Arrange, Act and Assert
        for inherited in (
            {"NODE_OPTIONS": "--require=/tmp/bypass.cjs"},
            {"NODE_PATH": "/tmp/bypass"},
            {"PWRIGHT_CANDIDATE_CDP_ENDPOINT": "http://127.0.0.1:9222"},
            {"PWTEST_MODE": "service"},
            {"CRPATH": "/Applications/Google Chrome.app"},
        ):
            with self.subTest(inherited=inherited):
                with self.assertRaisesRegex(ValueError, "forbids inherited"):
                    dummy.preflight_environment(inherited)

    def test_receipt_rejects_missing_or_successful_guarded_launch(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            receipt = Path(directory) / "receipt.jsonl"
            receipt.write_text("")
            arguments = {
                "mode": "oracle",
                "manifest_sha256": dummy.digest(dummy.MANIFEST),
                "executable": UPSTREAM / dummy.ASSET,
            }

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "one guarded failed public launch"):
                dummy.receipt_events(receipt, **arguments)
            event = {
                "event": "launch-succeeded",
                "mode": "oracle",
                "publicMethod": "chromium.launch",
                "executablePath": str(UPSTREAM / dummy.ASSET),
                "assetSha256": dummy.UPSTREAM_SOURCES[dummy.ASSET],
                "manifestSha256": arguments["manifest_sha256"],
            }
            receipt.write_text(json.dumps(event) + "\n")
            with self.assertRaisesRegex(ValueError, "one guarded failed public launch"):
                dummy.receipt_events(receipt, **arguments)

    def test_process_receipt_requires_exact_pinned_exit(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            receipt = Path(directory) / "process.jsonl"
            asset = UPSTREAM / dummy.ASSET
            receipt.write_text("")

            # Act and Assert
            with self.assertRaisesRegex(ValueError, "one pinned exit"):
                dummy.process_receipt_event(receipt, executable=asset)
            receipt.write_text(
                json.dumps(
                    {"event": "dummy-node-exited", "exitCode": 0, "asset": str(asset)}
                )
                + "\n"
            )
            with self.assertRaisesRegex(ValueError, "one pinned exit"):
                dummy.process_receipt_event(receipt, executable=asset)
            receipt.write_text(
                json.dumps(
                    {"event": "dummy-node-exited", "exitCode": 1, "asset": str(asset)}
                )
                + "\n"
            )
            dummy.process_receipt_event(receipt, executable=asset)

    def test_report_accepts_exact_leaf_suite_without_nested_suites_key(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"
            selected = {
                "projectId": dummy.PROJECT,
                "expectedStatus": "passed",
                "status": "expected",
                "results": [{"status": "passed"}],
            }
            result.write_text(
                json.dumps(
                    {
                        "suites": [
                            {
                                "specs": [
                                    {
                                        "id": dummy.SPEC_ID,
                                        "file": dummy.SPEC_FILE,
                                        "line": 64,
                                        "title": dummy.TITLE,
                                        "tests": [selected],
                                    }
                                ]
                            }
                        ]
                    }
                )
            )

            # Act
            observed = dummy.report_case(result)

            # Assert
            self.assertEqual(observed, selected)

    def test_sandbox_profile_denies_unlisted_exec_and_outbound_network(self) -> None:
        # Arrange
        allowed = [Path("/usr/bin/env"), Path("/opt/homebrew/bin/node")]

        # Act
        profile = dummy.sandbox_profile(allowed)

        # Assert
        self.assertIn("(deny process-exec)", profile)
        self.assertIn("(deny network-outbound)", profile)
        self.assertIn('(allow process-exec (literal "/usr/bin/env"))', profile)
        self.assertNotIn("/Applications/Google Chrome.app", profile)
        with self.assertRaisesRegex(ValueError, "allowlist is invalid"):
            dummy.sandbox_profile([allowed[0], allowed[0]])


if __name__ == "__main__":
    unittest.main()
