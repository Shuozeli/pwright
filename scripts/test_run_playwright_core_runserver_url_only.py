"""Focused fail-closed checks for the versioned RunServer URL attestation."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from run_playwright_core_runserver_url_only import (
    ADAPTER,
    ROOT,
    SUITE,
    pinned_suite,
    run,
    selected_result,
    verified_receipt,
)


class RunServerUrlOnlyTests(unittest.TestCase):
    def test_pinned_source_and_preload_have_current_digests(self) -> None:
        # Arrange
        upstream = Path("/Users/cyuan/projects/thirdparty/playwright-v1.58.0")
        if not upstream.is_dir():
            self.skipTest("pinned upstream checkout is not installed on this host")

        # Act
        suite, source_sha = pinned_suite(upstream)

        # Assert
        self.assertEqual(suite["id"], SUITE)
        self.assertEqual(source_sha, suite["sourceFileSha256"])
        self.assertEqual(
            suite["urlOnlyPreload"]["path"], str(ADAPTER.relative_to(ROOT))
        )

    def test_receipt_rejects_a_loopback_or_unmatched_bind(self) -> None:
        # Arrange
        receipt = {
            "schema": 1,
            "suite": SUITE,
            "pid": 42,
            "boundAddress": "127.0.0.1",
            "boundPort": 54321,
            "sourceEndpoint": "ws://localhost:54321/",
            "adaptedEndpoint": "ws://example.tailnet.ts.net:54321/",
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            path.write_text(json.dumps(receipt))

            # Act / Assert
            with self.assertRaisesRegex(ValueError, "not proven bound"):
                verified_receipt(path, "100.64.0.1", "example.tailnet.ts.net")

    def test_result_requires_the_exact_unmodified_official_row(self) -> None:
        # Arrange
        report = {
            "suites": [
                {
                    "specs": [
                        {
                            "id": "pinned-id",
                            "file": "library/browsertype-connect.spec.ts",
                            "line": 1046,
                            "title": "should refuse connecting when versions do not match",
                            "tests": [
                                {
                                    "projectId": "chromium-library",
                                    "status": "expected",
                                    "results": [{"status": "passed"}],
                                }
                            ],
                        }
                    ]
                }
            ]
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            path.write_text(json.dumps(report))

            # Act
            identity = selected_result(path)

            # Assert
            self.assertEqual(identity, ("pinned-id", "expected"))
            report["suites"][0]["specs"][0]["line"] = 1056
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, "selection differs"):
                selected_result(path)

    def test_ambient_node_options_fail_before_running_any_process(self) -> None:
        # Arrange
        upstream = ROOT
        with tempfile.TemporaryDirectory() as directory:
            result = Path(directory) / "result.json"

            # Act / Assert
            with (
                patch.dict(
                    "os.environ", {"NODE_OPTIONS": "--require=/tmp/unpinned.cjs"}
                ),
                self.assertRaisesRegex(ValueError, "ambient NODE_OPTIONS"),
            ):
                run("oracle", upstream, None, result)


if __name__ == "__main__":
    unittest.main()
