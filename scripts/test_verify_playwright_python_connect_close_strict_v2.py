"""Synthetic positive and negative controls for connect-close v2 evidence."""

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
import run_playwright_python_connect_close_strict_v2 as runner
import verify_playwright_python_connect_close_strict_v2 as strict
from verify_playwright_python_ws_transport_strict import StrictWebSocketEvidenceError

sys.path.insert(0, str(strict.GUARD))
import pwright_official_connect_guard_v2 as guard


class PolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = json.loads(strict.POLICY.read_text())

    def _unready(self) -> None:
        self.policy["captureReady"] = False
        self.policy["candidateWheelSha256"] = ""
        self.policy["candidatePackageTreeSha256"] = ""
        self.policy["candidateNativeSha256"] = ""
        self.policy["candidateRuntimeLibraries"] = {}
        self.policy["captureSourceSha256"] = {}

    def test_unready_policy_is_structurally_valid_but_cannot_capture(self) -> None:
        # Arrange
        self._unready()
        self.assertFalse(strict._policy(self.policy, require_ready=False)["captureReady"])

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            strict._policy(self.policy)

    def test_unready_policy_rejects_partial_wheel_pin(self) -> None:
        # Arrange
        self._unready()
        self.policy["candidateWheelSha256"] = "0" * 64

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            strict._policy(self.policy, require_ready=False)

    def test_policy_rejects_fourth_engine(self) -> None:
        # Arrange
        self.policy["tests"].append(f"{strict.SOURCE}::{strict.NAME}[other]")

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            strict._policy(self.policy, require_ready=False)

    def test_fixture_rejects_loopback_binding(self) -> None:
        # Arrange
        evidence = {
            "fixturePort": 48000,
            "fixtureBind": "127.0.0.1",
            "fixtureHost": "machine.tail8f3b66.ts.net",
            "fixtureTransportAdaptation": strict.ADAPTATION,
        }

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            strict._fixture(evidence)

    def test_result_destinations_are_no_clobber(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "one.xml"
            runner._fresh_result_v2(output)

            # Act / Assert
            with self.assertRaises(ValueError):
                runner._fresh_result_v2(output)

    def test_runner_rejects_changed_policy_pin_before_launch(self) -> None:
        # Arrange
        args = SimpleNamespace()

        # Act / Assert
        with (
            patch.dict(os.environ, {}, clear=True),
            patch.object(runner, "POLICY_SHA256", "0" * 64),
            patch.object(runner.subprocess, "run", side_effect=AssertionError("launched")),
            self.assertRaises(StrictWebSocketEvidenceError),
        ):
            runner.run(args)

    def test_junit_requires_unchanged_case_to_pass(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            result = Path(directory) / "one.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
                '<testcase classname="tests.async.test_browsertype_connect" '
                'name="test_should_print_custom_ws_close_error[firefox]"/>'
                "</testsuite></testsuites>"
            )
            evidence = {
                "mode": "candidate",
                "testId": strict.TEST_IDS[1],
                "result": str(result),
                "resultSha256": strict.sha256_file(result),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "exitCode": 0,
            }

            # Act / Assert
            self.assertEqual(strict.report_outcome(evidence), "passed")
            evidence["exitCode"] = 1
            with self.assertRaises(StrictWebSocketEvidenceError):
                strict.report_outcome(evidence)

    def test_junit_rejects_wrong_expanded_case(self) -> None:
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            result = Path(directory) / "one.xml"
            result.write_text(
                '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
                '<testcase classname="tests.async.test_browsertype_connect" '
                'name="test_should_print_custom_ws_close_error[webkit]"/>'
                "</testsuite></testsuites>"
            )
            evidence = {
                "mode": "candidate",
                "testId": strict.TEST_IDS[1],
                "result": str(result),
                "resultSha256": strict.sha256_file(result),
                "callReceipt": str(result.with_suffix(".call.jsonl")),
                "exitCode": 0,
            }

            # Act / Assert
            with self.assertRaises(StrictWebSocketEvidenceError):
                strict.report_outcome(evidence)


class ReceiptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.receipt = Path(self.directory.name) / "one.call.jsonl"
        self.port = 48000
        self.host = "machine.tail8f3b66.ts.net"
        self.nonce = "a" * 32
        self.test_id = strict.TEST_IDS[1]
        self.common = {
            "schema": 2,
            "mode": "candidate",
            "testId": self.test_id,
            "runNonce": self.nonce,
        }
        self.events = [
            {
                **self.common,
                "event": "publicCall",
                "browserName": "firefox",
                "method": "connect",
                "fixturePort": self.port,
                "originalEndpoint": f"ws://localhost:{self.port}/ws",
                "forwardedEndpoint": f"ws://{self.host}:{self.port}/ws",
            },
            {**self.common, "event": "open"},
            {
                **self.common,
                "event": "message",
                "payloadLength": 4,
                "payloadSha256": "b" * 64,
                "isBinary": False,
                "guid": "",
                "method": "initialize",
                "sdkLanguage": "python",
                "messageId": 1,
                "apiName": "Connection.init",
                "wallTime": 1720000000000,
            },
            {**self.common, "event": "sendClose", "code": 4123, "reason": "Oh my!"},
        ]

    def _verify(self) -> None:
        self.receipt.write_text("".join(json.dumps(event) + "\n" for event in self.events))
        strict._receipt_events(
            {
                "callReceipt": str(self.receipt),
                "callReceiptSha256": strict.sha256_file(self.receipt),
                "mode": "candidate",
                "testId": self.test_id,
                "runNonce": self.nonce,
                "fixturePort": self.port,
                "fixtureHost": self.host,
            }
        )

    def test_accepts_ordered_public_call_and_real_fixture_receipt(self) -> None:
        # Arrange / Act / Assert
        self._verify()

    def test_rejects_missing_inbound_message(self) -> None:
        # Arrange
        del self.events[2]

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_zero_length_message(self) -> None:
        # Arrange
        self.events[2]["payloadLength"] = 0

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_non_initialize_message(self) -> None:
        # Arrange
        self.events[2]["method"] = "different"

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_non_python_sdk_language(self) -> None:
        # Arrange
        self.events[2]["sdkLanguage"] = "javascript"

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_wrong_close_reason(self) -> None:
        # Arrange
        self.events[3]["reason"] = "different"

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_wrong_close_code(self) -> None:
        # Arrange
        self.events[3]["code"] = 1000

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_nonce_substitution(self) -> None:
        # Arrange
        self.events[2]["runNonce"] = "c" * 32

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_reordered_transport_events(self) -> None:
        # Arrange
        self.events[1], self.events[2] = self.events[2], self.events[1]

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()

    def test_rejects_wrong_forwarded_endpoint(self) -> None:
        # Arrange
        self.events[0]["forwardedEndpoint"] = f"ws://localhost:{self.port}/ws"

        # Act / Assert
        with self.assertRaises(StrictWebSocketEvidenceError):
            self._verify()


class GuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        source = self.root / strict.SOURCE
        source.parent.mkdir(parents=True)
        source.write_text("# synthetic source\n")
        self.receipt = self.root / "one.call.jsonl"
        self.receipt.touch()
        self.port = 48000
        self.host = "machine.tail8f3b66.ts.net"
        self.env = {
            "PWRIGHT_PYTHON_CONNECT_V2_EXPECTED_TEST": strict.TEST_IDS[1],
            "PWRIGHT_PYTHON_CONNECT_V2_UPSTREAM": str(self.root),
            "PWRIGHT_PYTHON_CONNECT_V2_PORT": str(self.port),
            "PWRIGHT_PYTHON_CONNECT_V2_RECEIPT": str(self.receipt),
            "PWRIGHT_PYTHON_CONNECT_V2_NONCE": "a" * 32,
            "PWRIGHT_OFFICIAL_TAILSCALE_HOST": self.host,
            "PWRIGHT_OFFICIAL_TAILSCALE_IP": "100.78.97.63",
            "PWRIGHT_OFFICIAL_MODE": "candidate",
        }

        class Server:
            PORT = 1

            def once_web_socket_connection(self, handler: object) -> None:
                self.handler = handler

        self.server = Server()
        self.fixture = ModuleType("tests.server")
        self.fixture.Server = Server
        self.fixture.test_server = SimpleNamespace(server=self.server)
        self.official = ModuleType("playwright.async_api")
        self.addCleanup(self._reset_guard)
        self._reset_guard()

    @staticmethod
    def _reset_guard() -> None:
        guard._selected_test = None
        guard._mode = None
        guard._state = "unconfigured"
        guard._registrations = 0

    @staticmethod
    def _dns(*args: object, **kwargs: object) -> list[tuple[object, ...]]:
        return [(2, 1, 6, "", ("100.78.97.63", 48000))]

    def _configure(self, browser_type: type[object]) -> None:
        self.official.BrowserType = browser_type
        with (
            patch.dict(os.environ, self.env, clear=True),
            patch.dict(
                sys.modules, {"tests.server": self.fixture, "playwright.async_api": self.official}
            ),
            patch.object(guard.socket, "getaddrinfo", self._dns),
        ):
            guard.pytest_configure()

    def test_guard_observes_real_handler_message_and_close_without_substitution(self) -> None:
        # Arrange
        observed: list[object] = []
        server = self.server
        port = self.port
        host = self.host

        class WebSocket:
            def sendClose(self, *, code: int, reason: str) -> None:
                observed.append(("close", code, reason))

        class BrowserType:
            name = "firefox"

            async def connect(self, endpoint: str) -> str:
                observed.append(endpoint)
                ws = WebSocket()
                server.handler(ws)
                ws.onMessage(
                    b'{"id":1,"guid":"","method":"initialize","params":{"sdkLanguage":"python"},'
                    b'"metadata":{"apiName":"Connection.init","wallTime":1720000000000}}',
                    False,
                )
                return "delegated"

        self._configure(BrowserType)

        def upstream_handler(ws: object) -> None:
            def on_message(payload: bytes, is_binary: bool) -> None:
                observed.append(("message", payload, is_binary))
                ws.sendClose(code=4123, reason="Oh my!")

            setattr(ws, "onMessage", on_message)

        self.server.once_web_socket_connection(upstream_handler)

        # Act
        with patch.dict(os.environ, self.env, clear=True):
            outcome = asyncio.run(BrowserType().connect(f"ws://localhost:{port}/ws"))

        # Assert
        self.assertEqual(outcome, "delegated")
        self.assertEqual(observed[0], f"ws://{host}:{port}/ws")
        self.assertEqual(observed[-1], ("close", 4123, "Oh my!"))
        self.assertEqual(
            [json.loads(line)["event"] for line in self.receipt.read_text().splitlines()],
            ["publicCall", "open", "message", "sendClose"],
        )
        message = json.loads(self.receipt.read_text().splitlines()[2])
        self.assertEqual(message["method"], "initialize")
        self.assertEqual(message["apiName"], "Connection.init")
        session = SimpleNamespace(exitstatus=pytest.ExitCode.OK)
        guard.pytest_sessionfinish(session)
        self.assertEqual(session.exitstatus, pytest.ExitCode.OK)

    def test_guard_refuses_missing_public_method(self) -> None:
        # Arrange
        class BrowserType:
            name = "firefox"

        # Act / Assert
        with self.assertRaises(pytest.UsageError):
            self._configure(BrowserType)
        self.assertEqual(self.receipt.read_text(), "")

    def test_guard_refuses_wrong_public_url_before_product_call(self) -> None:
        # Arrange
        observed: list[str] = []

        class BrowserType:
            name = "firefox"

            async def connect(self, endpoint: str) -> None:
                observed.append(endpoint)

        self._configure(BrowserType)
        self.server.once_web_socket_connection(lambda ws: None)

        # Act / Assert
        with patch.dict(os.environ, self.env, clear=True), self.assertRaises(pytest.UsageError):
            asyncio.run(BrowserType().connect(f"ws://localhost:{self.port}/wrong"))
        self.assertEqual(observed, [])
        self.assertEqual(self.receipt.read_text(), "")

    def test_guard_refuses_empty_inbound_frame(self) -> None:
        # Arrange
        class WebSocket:
            def sendClose(self, *, code: int, reason: str) -> None:
                pass

        ws = WebSocket()
        guard._state = "called"
        with patch.object(guard, "_receipt"):
            guard._observe_fixture_ws(
                ws, lambda value: setattr(value, "onMessage", lambda *_: None)
            )

            # Act / Assert
            with self.assertRaises(pytest.UsageError):
                ws.onMessage(b"", False)

    def test_guard_refuses_changed_real_fixture_close(self) -> None:
        # Arrange
        class WebSocket:
            def sendClose(self, *, code: int, reason: str) -> None:
                pass

        def upstream_handler(ws: object) -> None:
            def on_message(payload: bytes, is_binary: bool) -> None:
                ws.sendClose(code=1000, reason="different")

            setattr(ws, "onMessage", on_message)

        ws = WebSocket()
        guard._state = "called"
        with patch.object(guard, "_receipt"):
            guard._observe_fixture_ws(ws, upstream_handler)

            # Act / Assert
            with self.assertRaises(pytest.UsageError):
                ws.onMessage(
                    b'{"id":1,"guid":"","method":"initialize","params":{"sdkLanguage":"python"},'
                    b'"metadata":{"apiName":"Connection.init","wallTime":1720000000000}}',
                    False,
                )

    def test_guard_refuses_arbitrary_nonempty_json_message(self) -> None:
        # Arrange
        class WebSocket:
            def sendClose(self, *, code: int, reason: str) -> None:
                pass

        ws = WebSocket()
        guard._state = "called"
        with patch.object(guard, "_receipt"):
            guard._observe_fixture_ws(
                ws, lambda value: setattr(value, "onMessage", lambda *_: None)
            )

            # Act / Assert
            with self.assertRaises(pytest.UsageError):
                ws.onMessage(b'{"method":"unrelated"}', False)


if __name__ == "__main__":
    unittest.main()
