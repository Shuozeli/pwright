"""Browser-free freshness regressions for both official execution runners."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import run_playwright_official_conformance as core_runner
import run_playwright_python_official_conformance as python_runner
from official_result_destination import prepare_result_destination


class OfficialResultDestinationTests(unittest.TestCase):
    def test_absent_destination_is_not_created_by_guard(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            selected = Path(directory) / "new" / "result.xml"

            # Act
            result = prepare_result_destination(selected)

            # Assert
            self.assertEqual(result, selected.parent.resolve() / selected.name)
            self.assertFalse(result.exists())
            self.assertFalse(result.parent.exists())

    def test_both_runners_reject_existing_destinations_before_any_child(self) -> None:
        for runner in (core_runner, python_runner):
            for sidecar in (False, True):
                for kind in ("file", "directory", "symlink", "dangling-symlink"):
                    with (
                        self.subTest(runner=runner.__name__, sidecar=sidecar, kind=kind),
                        TemporaryDirectory() as directory,
                    ):
                        # Arrange
                        root = Path(directory)
                        result = root / "result.xml"
                        output = result.with_suffix(".provenance.json") if sidecar else result
                        target = root / "target"
                        if kind == "directory":
                            output.mkdir()
                        elif kind in ("symlink", "dangling-symlink"):
                            if kind == "symlink":
                                target.write_text("historical evidence")
                            output.symlink_to(target)
                        else:
                            output.write_text("historical evidence")
                        argv = ["runner", "oracle", str(root), "--result", str(result)]
                        with (
                            patch.object(sys, "argv", argv),
                            patch.object(runner.subprocess, "run") as child,
                        ):
                            # Act and Assert
                            with self.assertRaisesRegex(SystemExit, "result destination"):
                                runner.main()
                            child.assert_not_called()
                        if kind == "file":
                            self.assertEqual(output.read_text(), "historical evidence")
                        elif kind in ("symlink", "dangling-symlink"):
                            self.assertTrue(output.is_symlink())
                            self.assertEqual(target.exists(), kind == "symlink")

    def test_resume_allows_regular_aggregate_and_sidecar(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            result = Path(directory) / "result.xml"
            result.write_text("old aggregate")
            sidecar = result.with_suffix(".provenance.json")
            sidecar.write_text("old provenance")

            # Act
            selected = prepare_result_destination(result, resume_isolated=True)

            # Assert
            self.assertEqual(selected, result.resolve())
            self.assertEqual(result.read_text(), "old aggregate")
            self.assertEqual(sidecar.read_text(), "old provenance")

    def test_resume_rejects_symlinked_aggregate_or_sidecar(self) -> None:
        for sidecar in (False, True):
            with self.subTest(sidecar=sidecar), TemporaryDirectory() as directory:
                # Arrange
                result = Path(directory) / "result.xml"
                output = result.with_suffix(".provenance.json") if sidecar else result
                output.symlink_to(Path(directory) / "missing")

                # Act and Assert
                with self.assertRaisesRegex(SystemExit, "not a regular file path"):
                    prepare_result_destination(result, resume_isolated=True)
                self.assertTrue(output.is_symlink())

    def test_core_fresh_failure_report_still_gets_failure_provenance(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            result = root / "result.json"
            manifest = root / "manifest.json"
            manifest.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "repository": "fixture",
                        "tag": "fixture",
                        "commit": "pinned",
                        "suites": [
                            {
                                "id": "fixture",
                                "role": "fixture",
                                "runner": "runner.js",
                                "requiresBrowser": False,
                            }
                        ],
                    }
                )
            )
            (root / "runner.js").write_text("// Unexecuted child boundary fixture.\n")

            def report_failure(
                command: list[str], **_kwargs: object
            ) -> subprocess.CompletedProcess[str]:
                result.write_text('{"errors":[{"message":"fresh test failure"}]}\n')
                return subprocess.CompletedProcess(command, 1)

            argv = [
                "runner",
                "oracle",
                str(root),
                "--suite",
                "fixture",
                "--manifest",
                str(manifest),
                "--result",
                str(result),
            ]
            with (
                patch.object(sys, "argv", argv),
                patch.object(core_runner, "_git", side_effect=["pinned", ""]),
                patch.object(
                    core_runner,
                    "_tailscale_runtime",
                    return_value=("100.64.0.1", "test.tail.ts.net"),
                ),
                patch.object(core_runner.subprocess, "run", side_effect=report_failure),
            ):
                # Act
                with self.assertRaises(SystemExit) as stopped:
                    core_runner.main()

            # Assert
            provenance = json.loads(result.with_suffix(".provenance.json").read_text())
            self.assertEqual(stopped.exception.code, 1)
            self.assertEqual(provenance["exitCode"], 1)
            self.assertEqual(
                provenance["resultSha256"],
                hashlib.sha256(result.read_bytes()).hexdigest(),
            )

    def test_python_fresh_failure_report_still_gets_failure_provenance(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            result = root / "result.xml"
            manifest = root / "manifest.json"
            manifest.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "repository": "fixture",
                        "tag": "fixture",
                        "commit": "pinned",
                        "corpus": {"expandedTests": 1},
                        "cases": [],
                        "execution": {
                            "requiresBrowser": False,
                            "requiresFixtureServers": True,
                        },
                    }
                )
            )
            package = root / "playwright"
            package.mkdir()
            (package / "__init__.py").write_text("# Fixture, never imported.\n")

            def report_failure(
                command: list[str], **_kwargs: object
            ) -> subprocess.CompletedProcess[str]:
                if command[1].endswith("check_playwright_python_official_source.py"):
                    return subprocess.CompletedProcess(command, 0)
                result.write_text(
                    '<testsuite tests="1" failures="1">'
                    '<testcase name="fresh"><failure/></testcase></testsuite>'
                )
                return subprocess.CompletedProcess(command, 1)

            argv = [
                "runner",
                "oracle",
                str(root),
                "--manifest",
                str(manifest),
                "--manifest-cases",
                "--tailscale-ip",
                "100.64.0.1",
                "--tailscale-host",
                "test.tail.ts.net",
                "--result",
                str(result),
            ]
            identity = {
                "pythonExecutable": sys.executable,
                "pythonVersion": "3.12.0",
                "playwrightPackage": str(package),
                "pwrightPackage": None,
                "pwrightArtifact": None,
            }
            with (
                patch.object(sys, "argv", argv),
                patch.object(python_runner, "probe_runtime_identity", return_value=identity),
                patch.object(python_runner.subprocess, "run", side_effect=report_failure),
            ):
                # Act
                with self.assertRaises(SystemExit) as stopped:
                    python_runner.main()

            # Assert
            provenance = json.loads(result.with_suffix(".provenance.json").read_text())
            self.assertEqual(stopped.exception.code, 1)
            self.assertEqual(provenance["exitCode"], 1)
            self.assertEqual(
                provenance["resultSha256"],
                hashlib.sha256(result.read_bytes()).hexdigest(),
            )


if __name__ == "__main__":
    unittest.main()
