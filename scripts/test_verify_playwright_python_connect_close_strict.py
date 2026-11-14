"""Negative checks for the frozen official BrowserType.connect evidence path."""

from __future__ import annotations

import asyncio
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from types import ModuleType, SimpleNamespace
from unittest.mock import patch

import pytest
import verify_playwright_python_connect_close_strict as strict

sys.path.insert(0, str(strict.GUARD))
import pwright_official_connect_guard as guard


class PolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = json.loads(strict.POLICY.read_text())

    def test_policy_accepts_exact_three_engines_and_rejects_fourth(self) -> None:
        # Arrange
        self.assertEqual(strict._policy(self.policy)["tests"], list(strict.TEST_IDS))
        self.policy["tests"].append(f"{strict.SOURCE}::{strict.NAME}[unknown]")

        # Act / Assert
        with self.assertRaises(strict.StrictBrowserConnectEvidenceError):
            strict._policy(self.policy)

    def test_policy_rejects_changed_wheel(self) -> None:
        # Arrange
        self.policy["wheelSha256"] = "0" * 64

        # Act / Assert
        with self.assertRaises(strict.StrictBrowserConnectEvidenceError):
            strict._policy(self.policy)

    def test_policy_rejects_unfrozen_capture(self) -> None:
        # Arrange
        self.policy["captureReady"] = False

        # Act / Assert
        with self.assertRaises(strict.StrictBrowserConnectEvidenceError):
            strict._policy(self.policy)

    def test_fixture_rejects_loopback_bind(self) -> None:
        # Arrange
        fixture = {
            "fixturePort": 48000,
            "fixtureBind": "127.0.0.1",
            "fixtureHost": "test.tail8f3b66.ts.net",
            "fixtureTransportAdaptation": strict.ADAPTATION,
        }

        # Act / Assert
        with self.assertRaises(strict.StrictBrowserConnectEvidenceError):
            strict._fixture(fixture)

    def test_candidate_junit_is_not_pass_credit(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            result = Path(directory) / "one.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="1" errors="0" skipped="0">'
                '<testcase classname="tests.async.test_browsertype_connect" '
                'name="test_should_print_custom_ws_close_error[chromium]">'
                '<failure message="AttributeError">'
                "AttributeError: 'BrowserType' object has no attribute 'connect'"
                "</failure></testcase></testsuite></testsuites>"
            )
            evidence = {
                "mode": "candidate",
                "testId": strict.TEST_IDS[0],
                "result": str(result),
                "resultSha256": strict.sha256_file(result),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "exitCode": 1,
            }

            # Act / Assert
            self.assertEqual(strict.report_outcome(evidence), "failed")
            evidence["exitCode"] = 0
            with self.assertRaises(strict.StrictBrowserConnectEvidenceError):
                strict.report_outcome(evidence)

    def test_candidate_junit_rejects_other_failure(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            result = Path(directory) / "one.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="1" errors="0" skipped="0">'
                '<testcase classname="tests.async.test_browsertype_connect" '
                'name="test_should_print_custom_ws_close_error[firefox]">'
                '<failure message="AssertionError">different</failure>'
                "</testcase></testsuite></testsuites>"
            )
            evidence = {
                "mode": "candidate",
                "testId": strict.TEST_IDS[1],
                "result": str(result),
                "resultSha256": strict.sha256_file(result),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "exitCode": 1,
            }

            # Act / Assert
            with self.assertRaises(strict.StrictBrowserConnectEvidenceError):
                strict.report_outcome(evidence)


class GuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        source = self.root / strict.SOURCE
        source.parent.mkdir(parents=True)
        source.write_text("# synthetic guard source\n")
        self.receipt = self.root / "one.call.jsonl"
        self.receipt.touch()
        self.engine = "firefox"
        self.host = "test.tail8f3b66.ts.net"
        self.port = 48000
        self.env = {
            "PWRIGHT_PYTHON_CONNECT_EXPECTED_TEST": strict.TEST_IDS[1],
            "PWRIGHT_PYTHON_CONNECT_UPSTREAM": str(self.root),
            "PWRIGHT_PYTHON_CONNECT_PORT": str(self.port),
            "PWRIGHT_PYTHON_CONNECT_RECEIPT": str(self.receipt),
            "PWRIGHT_PYTHON_CONNECT_NONCE": "a" * 32,
            "PWRIGHT_OFFICIAL_TAILSCALE_HOST": self.host,
            "PWRIGHT_OFFICIAL_TAILSCALE_IP": "100.78.97.63",
            "PWRIGHT_OFFICIAL_MODE": "oracle",
        }
        guard._selected_test = None
        guard._invocations = 0
        guard._method_present = False
        self.addCleanup(self._reset_guard)
        self.fixture = ModuleType("tests.server")
        self.fixture.test_server = SimpleNamespace(server=SimpleNamespace(PORT=1))
        self.official = ModuleType("playwright.async_api")

    @staticmethod
    def _reset_guard() -> None:
        guard._selected_test = None
        guard._invocations = 0
        guard._method_present = False

    @staticmethod
    def _dns(*args: object, **kwargs: object) -> list[tuple[object, ...]]:
        return [(2, 1, 6, "", ("100.78.97.63", 48000))]

    def _configure(self) -> None:
        with (
            patch.dict(os.environ, self.env, clear=True),
            patch.dict(
                sys.modules,
                {"playwright.async_api": self.official, "tests.server": self.fixture},
            ),
            patch.object(guard.socket, "getaddrinfo", self._dns),
        ):
            guard.pytest_configure()

    def test_oracle_rewrites_only_exact_same_port_url(self) -> None:
        # Arrange
        observed: list[str] = []

        class BrowserType:
            name = "firefox"

            async def connect(self, endpoint: str) -> str:
                observed.append(endpoint)
                return "closed"

        self.official.BrowserType = BrowserType
        self._configure()

        # Act
        with patch.dict(os.environ, self.env, clear=True):
            outcome = asyncio.run(
                BrowserType().connect(f"ws://localhost:{self.port}/ws")
            )

        # Assert
        self.assertEqual(outcome, "closed")
        self.assertEqual(observed, [f"ws://{self.host}:{self.port}/ws"])
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt["method"], "connect")
        self.assertEqual(receipt["originalEndpoint"], f"ws://localhost:{self.port}/ws")
        self.assertEqual(receipt["forwardedEndpoint"], observed[0])

    def test_guard_rejects_another_url_without_calling_product(self) -> None:
        # Arrange
        observed: list[str] = []

        class BrowserType:
            name = "firefox"

            async def connect(self, endpoint: str) -> str:
                observed.append(endpoint)
                return "closed"

        self.official.BrowserType = BrowserType
        self._configure()

        # Act / Assert
        with (
            patch.dict(os.environ, self.env, clear=True),
            self.assertRaises(pytest.UsageError),
        ):
            asyncio.run(BrowserType().connect(f"ws://localhost:{self.port}/other"))
        self.assertEqual(observed, [])
        self.assertEqual(self.receipt.read_text(), "")

    def test_candidate_missing_method_remains_missing(self) -> None:
        # Arrange
        self.env["PWRIGHT_OFFICIAL_MODE"] = "candidate"

        class BrowserType:
            name = "firefox"

        self.official.BrowserType = BrowserType

        # Act
        self._configure()

        # Assert
        self.assertFalse(hasattr(BrowserType, "connect"))
        self.assertEqual(self.receipt.read_text(), "")
        session = SimpleNamespace(exitstatus=pytest.ExitCode.OK)
        guard.pytest_sessionfinish(session)
        self.assertEqual(session.exitstatus, pytest.ExitCode.OK)


if __name__ == "__main__":
    unittest.main()
