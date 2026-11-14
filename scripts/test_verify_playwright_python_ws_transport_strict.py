"""Fail-closed checks for the exact official CDP WebSocket transport policy."""

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
import verify_playwright_python_ws_transport_strict as strict

sys.path.insert(0, str(strict.GUARD))
import pwright_official_ws_guard as guard


class PolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = json.loads(strict.POLICY.read_text())

    def test_policy_accepts_only_two_exact_chromium_ids(self) -> None:
        self.assertEqual(strict._policy(self.policy)["tests"], list(strict.TEST_IDS))
        self.policy["tests"].append(
            f"{strict.SOURCE}::test_should_print_custom_ws_close_error[firefox]"
        )
        with self.assertRaises(strict.StrictWebSocketEvidenceError):
            strict._policy(self.policy)

    def test_policy_rejects_unfrozen_capture_sources(self) -> None:
        self.policy["captureReady"] = False
        with self.assertRaises(strict.StrictWebSocketEvidenceError):
            strict._policy(self.policy)

    def test_sandbox_allows_only_selected_local_port_and_oracle_node(self) -> None:
        profile, allowed = strict.sandbox_profile(
            Path("/env/bin/python"),
            Path("/env/lib/playwright/driver/node"),
            "oracle",
            48000,
        )
        self.assertIn('(remote ip "localhost:48000")', profile)
        self.assertNotIn("localhost:48001", profile)
        self.assertIn("/env/lib/playwright/driver/node", allowed)
        candidate, candidate_allowed = strict.sandbox_profile(
            Path("/env/bin/python"),
            Path("/env/lib/playwright/driver/node"),
            "candidate",
            48000,
        )
        self.assertNotIn("/env/lib/playwright/driver/node", candidate_allowed)
        self.assertIn("(deny process-exec)", candidate)

    def test_fixture_rejects_loopback_binding(self) -> None:
        with self.assertRaises(strict.StrictWebSocketEvidenceError):
            strict._fixture(
                {
                    "fixturePort": 48000,
                    "fixtureBind": "127.0.0.1",
                    "fixtureHost": "test.tail8f3b66.ts.net",
                    "fixtureTransportAdaptation": "exact-127.0.0.1-ws-to-tailscale-magicdns-only",
                }
            )

    def test_report_distinguishes_failure_from_pass(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = Path(directory) / "case.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="1" errors="0" skipped="0">'
                '<testcase classname="tests.async.test_browsertype_connect_cdp" '
                'name="test_connect_over_cdp_passing_header_works[chromium]">'
                '<failure message="real assertion failed" />'
                "</testcase></testsuite></testsuites>"
            )
            payload = {
                "result": str(result),
                "resultSha256": strict.sha256_file(result),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "testId": strict.TEST_IDS[0],
                "exitCode": 1,
            }
            self.assertEqual(strict.report_outcome(payload), "failed")
            payload["exitCode"] = 0
            with self.assertRaises(strict.StrictWebSocketEvidenceError):
                strict.report_outcome(payload)


class GuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        source = self.root / strict.SOURCE
        source.parent.mkdir(parents=True)
        source.write_text("# fake source for guard unit test\n")
        self.receipt = self.root / "one.call.jsonl"
        self.receipt.touch()
        self.mode = "oracle"
        self.host = "test.tail8f3b66.ts.net"
        self.port = 48000
        self.env = {
            "PWRIGHT_PYTHON_WS_EXPECTED_TEST": strict.TEST_IDS[0],
            "PWRIGHT_PYTHON_WS_UPSTREAM": str(self.root),
            "PWRIGHT_PYTHON_WS_PORT": str(self.port),
            "PWRIGHT_PYTHON_WS_RECEIPT": str(self.receipt),
            "PWRIGHT_PYTHON_WS_NONCE": "a" * 32,
            "PWRIGHT_OFFICIAL_TAILSCALE_HOST": self.host,
            "PWRIGHT_OFFICIAL_TAILSCALE_IP": "100.78.97.63",
            "PWRIGHT_OFFICIAL_MODE": self.mode,
        }
        guard._selected_test = None
        guard._invocations = 0
        self.addCleanup(self._reset_guard)
        self.observed: list[tuple[str, object]] = []

        class BrowserType:
            name = "chromium"

            async def connect_over_cdp(
                inner_self: object, endpoint: str, *, headers: object
            ) -> str:
                self.observed.append((endpoint, headers))
                return "connected"

        self.browser_type = BrowserType
        official = ModuleType("playwright.async_api")
        official.BrowserType = BrowserType
        fixture = ModuleType("tests.server")
        fixture.test_server = SimpleNamespace(server=SimpleNamespace(PORT=1))
        self.modules = {
            "playwright.async_api": official,
            "tests.server": fixture,
        }
        self.fixture = fixture

    @staticmethod
    def _reset_guard() -> None:
        guard._selected_test = None
        guard._invocations = 0

    @staticmethod
    def _dns(*args: object, **kwargs: object) -> list[tuple[object, ...]]:
        return [(2, 1, 6, "", ("100.78.97.63", 48000))]

    def _configure(self) -> None:
        with (
            patch.dict(os.environ, self.env, clear=True),
            patch.dict(sys.modules, self.modules),
            patch.object(guard.socket, "getaddrinfo", self._dns),
        ):
            guard.pytest_configure()

    def test_guard_rewrites_only_transport_and_preserves_headers(self) -> None:
        self._configure()
        self.assertEqual(self.fixture.test_server.server.PORT, self.port)
        with patch.dict(os.environ, self.env, clear=True):
            outcome = asyncio.run(
                self.browser_type().connect_over_cdp(
                    f"ws://127.0.0.1:{self.port}/ws", headers={"foo": "bar"}
                )
            )
        self.assertEqual(outcome, "connected")
        self.assertEqual(
            self.observed, [(f"ws://{self.host}:{self.port}/ws", {"foo": "bar"})]
        )
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt["originalEndpoint"], f"ws://127.0.0.1:{self.port}/ws")
        self.assertEqual(
            receipt["forwardedEndpoint"], f"ws://{self.host}:{self.port}/ws"
        )
        self.assertEqual(receipt["headers"], {"foo": "bar"})

    def test_guard_rejects_other_url_without_calling_product(self) -> None:
        self._configure()
        with (
            patch.dict(os.environ, self.env, clear=True),
            self.assertRaises(pytest.UsageError),
        ):
            asyncio.run(
                self.browser_type().connect_over_cdp(
                    f"ws://127.0.0.1:{self.port}/other", headers={"foo": "bar"}
                )
            )
        self.assertEqual(self.observed, [])
        self.assertEqual(self.receipt.read_text(), "")

    def test_guard_rejects_changed_headers_without_calling_product(self) -> None:
        self._configure()
        with (
            patch.dict(os.environ, self.env, clear=True),
            self.assertRaises(pytest.UsageError),
        ):
            asyncio.run(
                self.browser_type().connect_over_cdp(
                    f"ws://127.0.0.1:{self.port}/ws", headers={"foo": "different"}
                )
            )
        self.assertEqual(self.observed, [])


if __name__ == "__main__":
    unittest.main()
