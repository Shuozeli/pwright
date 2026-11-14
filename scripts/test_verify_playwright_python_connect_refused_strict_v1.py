"""Synthetic positive and negative controls for the closed-endpoint slice."""

from __future__ import annotations

import asyncio
import errno
import json
import os
import sys
from pathlib import Path
from types import ModuleType, SimpleNamespace
from typing import Self
from unittest.mock import patch

import pytest
import run_playwright_python_connect_refused_strict_v1 as runner
import verify_playwright_python_connect_refused_strict_v1 as strict

sys.path.insert(0, str(strict.GUARD))
import pwright_official_connect_refused_guard_v1 as guard


def _unready_policy() -> dict[str, object]:
    policy: dict[str, object] = json.loads(strict.POLICY.read_text())
    policy["captureReady"] = False
    policy["candidateWheelSha256"] = ""
    policy["candidatePackageTreeSha256"] = ""
    policy["candidateNativeSha256"] = ""
    policy["candidateRuntimeLibraries"] = {}
    policy["captureSourceSha256"] = {}
    return policy


def test_unready_policy_is_frozen_but_cannot_capture() -> None:
    # Arrange
    policy = _unready_policy()

    # Act
    digest = strict.sha256_file(strict.POLICY)

    # Assert
    assert digest == strict.POLICY_SHA256
    assert strict._policy(policy, require_ready=False)["captureReady"] is False
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        strict._policy(policy)


def test_policy_rejects_seventh_case_and_partial_wheel_pin() -> None:
    # Arrange
    policy = _unready_policy()
    tests = policy["tests"]
    assert isinstance(tests, list)

    # Act / Assert
    tests.append(f"{strict.SOURCES[0]}::{strict.NAME}[other]")
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        strict._policy(policy, require_ready=False)
    tests.pop()
    policy["candidateWheelSha256"] = "0" * 64
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        strict._policy(policy, require_ready=False)


def test_runner_rejects_unready_policy_before_official_launch(tmp_path: Path) -> None:
    # Arrange
    unready = tmp_path / "unready-policy.json"
    unready.write_text(json.dumps(_unready_policy()))
    args = SimpleNamespace()

    # Act / Assert
    with (
        patch.dict(os.environ, {}, clear=True),
        patch.object(runner, "POLICY", unready),
        patch.object(runner, "POLICY_SHA256", strict.sha256_file(unready)),
        patch.object(runner.subprocess, "run", side_effect=AssertionError("launched")),
        pytest.raises(strict.StrictConnectRefusedV1EvidenceError),
    ):
        runner.run(args)


def test_candidate_feature_probe_checks_actual_package_abi_and_leaf() -> None:
    # Arrange
    policy = _unready_policy()
    payload = {"mode": "candidate", "pythonExecutable": "/selected/python"}

    # Act
    with patch.object(
        strict.subprocess, "run", return_value=SimpleNamespace(stdout="1\n")
    ) as process:
        selected = strict._candidate_feature_probe(payload, policy)

    # Assert
    assert selected is True
    script = process.call_args.args[0][-1]
    assert "from pwright import native_capabilities" in script
    assert "'playwright-server-initialize' in selected_features()" in script
    with patch.object(
        strict.subprocess, "run", return_value=SimpleNamespace(stdout="0\n")
    ):
        assert strict._candidate_feature_probe(payload, policy) is False
    assert strict._candidate_feature_probe({"mode": "oracle"}, policy) is True


def test_fixture_policy_rejects_loopback_binding() -> None:
    # Arrange
    evidence = {
        "fixturePort": 48000,
        "fixtureBind": "127.0.0.1",
        "fixtureHost": "machine.tail8f3b66.ts.net",
        "fixtureAdaptation": strict.ADAPTATION,
    }

    # Act / Assert
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        strict._fixture(evidence)


def test_result_destination_never_clobbers(tmp_path: Path) -> None:
    # Arrange
    destination = tmp_path / "one.xml"

    # Act
    runner._fresh_result(destination)

    # Assert
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        runner._fresh_result(destination)


@pytest.mark.parametrize("index", [1, 4])
def test_junit_identity_checks_async_and_sync(tmp_path: Path, index: int) -> None:
    # Arrange
    source = strict.TEST_IDS[index].split("::", 1)[0]
    class_name = (
        "tests.async.test_browsertype_connect"
        if source.startswith("tests/async/")
        else "tests.sync.test_browsertype_connect"
    )
    result = tmp_path / "one.xml"
    result.write_text(
        '<testsuites><testsuite tests="1" failures="0" errors="0" skipped="0">'
        f'<testcase classname="{class_name}" '
        f'name="{strict.TEST_IDS[index].split("::", 1)[1]}"/>'
        "</testsuite></testsuites>"
    )
    evidence = {
        "mode": "oracle",
        "testId": strict.TEST_IDS[index],
        "result": str(result),
        "resultSha256": strict.sha256_file(result),
        "callReceipt": str(result.with_suffix(".call.jsonl")),
        "exitCode": 0,
    }

    # Act / Assert
    assert strict.report_outcome(evidence) == "passed"
    evidence["testId"] = strict.TEST_IDS[0]
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        strict.report_outcome(evidence)


def _receipt_events(
    test_id: str, host: str, port: int, bind: str
) -> list[dict[str, object]]:
    common = {"schema": 1, "mode": "candidate", "testId": test_id, "runNonce": "a" * 32}
    engine = test_id.rsplit("[", 1)[1].removesuffix("]")
    endpoint = f"ws://{host}:{port}/ws"
    return [
        {
            **common,
            "event": "fixtureListening",
            "browserName": engine,
            "bindIp": bind,
            "fixturePort": port,
            "endpoint": endpoint,
            "reachableConnectErrno": 0,
            "originalRemoteServerLaunched": False,
        },
        {**common, "event": "fixtureClosed", "refusedConnectErrno": errno.ECONNREFUSED},
        {
            **common,
            "event": "publicCall",
            "browserName": engine,
            "method": "connect",
            "endpoint": endpoint,
            "preCallRefusedConnectErrno": errno.ECONNREFUSED,
        },
        {
            **common,
            "event": "publicError",
            "errorClass": "Error",
            "messageSha256": "b" * 64,
            "hasRequiredPrefix": True,
        },
    ]


def test_receipt_requires_actual_listen_close_refusal_and_public_call(
    tmp_path: Path,
) -> None:
    # Arrange
    receipt = tmp_path / "one.call.jsonl"
    test_id = strict.TEST_IDS[1]
    host = "machine.tail8f3b66.ts.net"
    port = 48000
    bind = "100.78.97.63"
    events = _receipt_events(test_id, host, port, bind)

    def verify() -> None:
        receipt.write_text("".join(json.dumps(event) + "\n" for event in events))
        strict._receipt_events(
            {
                "callReceipt": str(receipt),
                "callReceiptSha256": strict.sha256_file(receipt),
                "mode": "candidate",
                "testId": test_id,
                "runNonce": "a" * 32,
                "fixturePort": port,
                "fixtureHost": host,
                "fixtureBind": bind,
            }
        )

    # Act / Assert
    verify()
    events[1]["refusedConnectErrno"] = 0
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        verify()
    events[1]["refusedConnectErrno"] = errno.ECONNREFUSED
    events[2]["endpoint"] = f"ws://localhost:{port}/ws"
    with pytest.raises(strict.StrictConnectRefusedV1EvidenceError):
        verify()


class _FakeSocket:
    listening = False

    def __init__(self, *_args: object, **_kwargs: object) -> None:
        self.is_listener = False

    def __enter__(self) -> Self:
        return self

    def __exit__(self, *_args: object) -> None:
        self.close()

    def settimeout(self, _seconds: int) -> None:
        return None

    def bind(self, destination: tuple[str, int]) -> None:
        assert destination == ("100.78.97.63", 48000)
        self.is_listener = True

    def listen(self, backlog: int) -> None:
        assert backlog == 1
        type(self).listening = True

    def connect_ex(self, destination: tuple[str, int]) -> int:
        assert destination == ("machine.tail8f3b66.ts.net", 48000)
        return 0 if type(self).listening else errno.ECONNREFUSED

    def close(self) -> None:
        if self.is_listener:
            type(self).listening = False


def test_guard_preserves_async_public_call_after_real_fixture_lifecycle(
    tmp_path: Path,
) -> None:
    # Arrange
    upstream = tmp_path / "upstream"
    source = upstream / strict.SOURCES[0]
    source.parent.mkdir(parents=True)
    source.write_text("# synthetic source\n")
    receipt = tmp_path / "one.call.jsonl"
    receipt.touch()
    test_id = strict.TEST_IDS[1]
    environment = {
        "PWRIGHT_PYTHON_REFUSED_V1_EXPECTED_TEST": test_id,
        "PWRIGHT_PYTHON_REFUSED_V1_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_REFUSED_V1_PORT": "48000",
        "PWRIGHT_PYTHON_REFUSED_V1_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_REFUSED_V1_NONCE": "a" * 32,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": "machine.tail8f3b66.ts.net",
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": "100.78.97.63",
        "PWRIGHT_OFFICIAL_MODE": "candidate",
    }
    conftest = ModuleType("tests.conftest")
    original_server = type("RemoteServer", (), {})
    conftest.RemoteServer = original_server
    api = ModuleType("playwright.async_api")

    class Error(Exception):
        def __init__(self, message: str) -> None:
            super().__init__(message)
            self.message = message

    class BrowserType:
        name = "firefox"

        async def connect(self, endpoint: str) -> None:
            assert endpoint == "ws://machine.tail8f3b66.ts.net:48000/ws"
            raise Error("WebSocket error: connection refused")

    api.BrowserType = BrowserType
    api.Error = Error
    _FakeSocket.listening = False
    guard._selected = None
    guard._mode = None
    guard._state = "unconfigured"
    guard._created = guard._killed = guard._called = guard._errored = 0
    guard._original_remote_server = None
    with (
        patch.dict(os.environ, environment, clear=True),
        patch.dict(
            sys.modules, {"tests.conftest": conftest, "playwright.async_api": api}
        ),
        patch.object(
            guard.socket,
            "getaddrinfo",
            return_value=[(2, 1, 6, "", ("100.78.97.63", 48000))],
        ),
        patch.object(
            guard,
            "_connect_probe",
            side_effect=lambda: 0 if _FakeSocket.listening else errno.ECONNREFUSED,
        ),
    ):
        guard.pytest_configure()
        with patch.object(guard.socket, "socket", _FakeSocket):
            fake_server = conftest.RemoteServer(
                "firefox", {"headless": True}, tmp_path / "settings.json"
            )
            fake_server.kill()

        # Act
        with pytest.raises(pytest.UsageError):
            asyncio.run(BrowserType().connect("ws://incorrect.example/ws"))
        with pytest.raises(Error):
            asyncio.run(BrowserType().connect(fake_server.ws_endpoint))

        # Assert
        session = SimpleNamespace(exitstatus=pytest.ExitCode.OK)
        guard.pytest_sessionfinish(session)
        assert session.exitstatus == pytest.ExitCode.OK
        assert [
            json.loads(line)["event"] for line in receipt.read_text().splitlines()
        ] == [
            "fixtureListening",
            "fixtureClosed",
            "publicCall",
            "publicError",
        ]
        guard.pytest_unconfigure()
        assert conftest.RemoteServer is original_server


def test_guard_preserves_sync_public_call_after_real_fixture_lifecycle(
    tmp_path: Path,
) -> None:
    # Arrange
    upstream = tmp_path / "upstream"
    source = upstream / strict.SOURCES[1]
    source.parent.mkdir(parents=True)
    source.write_text("# synthetic source\n")
    receipt = tmp_path / "one.call.jsonl"
    receipt.touch()
    test_id = strict.TEST_IDS[4]
    environment = {
        "PWRIGHT_PYTHON_REFUSED_V1_EXPECTED_TEST": test_id,
        "PWRIGHT_PYTHON_REFUSED_V1_UPSTREAM": str(upstream),
        "PWRIGHT_PYTHON_REFUSED_V1_PORT": "48000",
        "PWRIGHT_PYTHON_REFUSED_V1_RECEIPT": str(receipt),
        "PWRIGHT_PYTHON_REFUSED_V1_NONCE": "a" * 32,
        "PWRIGHT_OFFICIAL_TAILSCALE_HOST": "machine.tail8f3b66.ts.net",
        "PWRIGHT_OFFICIAL_TAILSCALE_IP": "100.78.97.63",
        "PWRIGHT_OFFICIAL_MODE": "oracle",
    }
    conftest = ModuleType("tests.conftest")
    original_server = type("RemoteServer", (), {})
    conftest.RemoteServer = original_server
    api = ModuleType("playwright.sync_api")

    class Error(Exception):
        def __init__(self, message: str) -> None:
            super().__init__(message)
            self.message = message

    class BrowserType:
        name = "firefox"

        def connect(self, endpoint: str) -> None:
            assert endpoint == "ws://machine.tail8f3b66.ts.net:48000/ws"
            raise Error("WebSocket error: connection refused")

    api.BrowserType = BrowserType
    api.Error = Error
    _FakeSocket.listening = False
    guard._selected = None
    guard._mode = None
    guard._state = "unconfigured"
    guard._created = guard._killed = guard._called = guard._errored = 0
    guard._original_remote_server = None
    with (
        patch.dict(os.environ, environment, clear=True),
        patch.dict(
            sys.modules, {"tests.conftest": conftest, "playwright.sync_api": api}
        ),
        patch.object(
            guard.socket,
            "getaddrinfo",
            return_value=[(2, 1, 6, "", ("100.78.97.63", 48000))],
        ),
        patch.object(
            guard,
            "_connect_probe",
            side_effect=lambda: 0 if _FakeSocket.listening else errno.ECONNREFUSED,
        ),
    ):
        guard.pytest_configure()
        with patch.object(guard.socket, "socket", _FakeSocket):
            fake_server = conftest.RemoteServer(
                "firefox", {"headless": True}, tmp_path / "settings.json"
            )
            fake_server.kill()

        # Act
        with pytest.raises(Error):
            BrowserType().connect(fake_server.ws_endpoint)

        # Assert
        session = SimpleNamespace(exitstatus=pytest.ExitCode.OK)
        guard.pytest_sessionfinish(session)
        assert session.exitstatus == pytest.ExitCode.OK
        assert [
            json.loads(line)["event"] for line in receipt.read_text().splitlines()
        ] == [
            "fixtureListening",
            "fixtureClosed",
            "publicCall",
            "publicError",
        ]
        guard.pytest_unconfigure()
        assert conftest.RemoteServer is original_server
