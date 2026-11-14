"""Adapt only launch_server for the unchanged pinned navigation-after-close test."""

from __future__ import annotations

import atexit
import hashlib
import importlib
import json
import os
import select
import signal
import subprocess
import sys
from pathlib import Path
from typing import Any, cast

import pytest

UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
TEST_NAME = "test_browser_type_connect_should_reject_navigation_when_browser_closes"
TEST_FILE = "tests/async/test_browsertype_connect.py"
SOURCE_HASHES = {
    TEST_FILE: "d47290f5acb45246ce2bae150d3c583d340806821b660be08d798ac3572e7b11",
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/async/conftest.py": "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb",
}
ENGINES = ("chromium", "firefox", "webkit")
ENDPOINT_PATH = "/browser-synthetic-python-navigation-closed-v1"
_remotes: list[SyntheticRemoteServer] = []


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"missing navigation-closed diagnostic setting: {name}")
    return value


def _record(kind: str, **fields: object) -> None:
    with Path(_setting("PWRIGHT_PYTHON_NAV_CLOSED_RECEIPT")).open("a") as stream:
        stream.write(
            json.dumps(
                {
                    "kind": kind,
                    "mode": _setting("PWRIGHT_PYTHON_NAV_CLOSED_MODE"),
                    "engine": _setting("PWRIGHT_PYTHON_NAV_CLOSED_ENGINE"),
                    "fixtureBind": _setting("PWRIGHT_OFFICIAL_TAILSCALE_IP"),
                    "fixtureHost": _setting("PWRIGHT_OFFICIAL_TAILSCALE_HOST"),
                    **fields,
                },
                sort_keys=True,
            )
            + "\n"
        )


class SyntheticRemoteServer:
    """RemoteServer replacement reached through the original launch_server fixture."""

    def __init__(self, browser_name: str, launch_server_options: dict[str, Any], tmpfile: Path):
        if (
            browser_name != _setting("PWRIGHT_PYTHON_NAV_CLOSED_ENGINE")
            or browser_name not in ENGINES
            or set(launch_server_options) != {"headless"}
            or not isinstance(launch_server_options["headless"], bool)
            or tmpfile.name != "settings-0.json"
        ):
            raise RuntimeError("unexpected navigation-closed launch_server request")
        script = Path(_setting("PWRIGHT_PYTHON_NAV_CLOSED_SERVER"))
        if not script.is_file() or script.is_symlink():
            raise RuntimeError("navigation-closed protocol fixture is missing or symlinked")
        self.process = subprocess.Popen(
            ["node", str(script)],
            stdout=subprocess.PIPE,
            stderr=sys.stderr,
            text=True,
            env=os.environ.copy(),
        )
        try:
            assert self.process.stdout is not None
            ready, _, _ = select.select([self.process.stdout], [], [], 10)
            if not ready:
                raise RuntimeError("navigation-closed fixture did not announce endpoint")
            line = self.process.stdout.readline().strip()
            prefix = f"Listening on ws://{_setting('PWRIGHT_OFFICIAL_TAILSCALE_HOST')}:"
            if not line.startswith(prefix) or not line.endswith(ENDPOINT_PATH):
                raise RuntimeError(f"invalid navigation-closed endpoint: {line!r}")
            self.ws_endpoint = line.removeprefix("Listening on ")
            self.process.stdout.close()
            _remotes.append(self)
            _record(
                "fixture-created",
                serverPid=self.process.pid,
                endpointPath=ENDPOINT_PATH,
                fixtureAdapted=True,
                testSourceAdapted=False,
                browserProcessLaunched=False,
            )
        except BaseException:
            self.process.kill()
            self.process.wait(timeout=5)
            raise

    def kill(self) -> None:
        if self.process.poll() is not None:
            _record("fixture-premature-exit", serverPid=self.process.pid)
            return
        self.process.send_signal(signal.SIGINT)
        try:
            return_code = self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=5)
            raise RuntimeError("navigation-closed fixture did not stop") from None
        _record("fixture-stopped", serverPid=self.process.pid, returnCode=return_code)


def _cleanup_remotes() -> None:
    for remote in _remotes:
        if remote.process.poll() is None:
            remote.kill()


atexit.register(_cleanup_remotes)


def pytest_configure() -> None:
    upstream = Path(_setting("PWRIGHT_PYTHON_NAV_CLOSED_UPSTREAM")).resolve()
    mode = _setting("PWRIGHT_PYTHON_NAV_CLOSED_MODE")
    engine = _setting("PWRIGHT_PYTHON_NAV_CLOSED_ENGINE")
    if mode not in ("oracle", "candidate") or engine not in ENGINES:
        raise RuntimeError("navigation-closed diagnostic lane or engine changed")
    if any(os.environ.get(key) for key in ("PWRIGHT_CDP_ENDPOINT", "PWRIGHT_CDP_WS_URL")):
        raise RuntimeError("browser-free navigation-closed diagnostic cannot use CDP")
    for relative, expected in SOURCE_HASHES.items():
        source = upstream / relative
        if source.is_symlink() or hashlib.sha256(source.read_bytes()).hexdigest() != expected:
            raise RuntimeError(f"pinned Python source changed: {relative}")
    public_api = importlib.import_module("playwright.async_api")
    candidate = importlib.import_module("pwright") if mode == "candidate" else None
    public_api_path = public_api.__file__
    candidate_path = candidate.__file__ if candidate is not None else None
    if public_api_path is None or (candidate is not None and candidate_path is None):
        raise RuntimeError("runtime package has no source path")
    _record(
        "runtime-import",
        publicApiPath=str(Path(public_api_path).resolve()),
        candidatePackagePath=(
            str(Path(candidate_path).resolve()) if candidate_path is not None else None
        ),
    )
    upstream_conftest = importlib.import_module("tests.conftest")
    if upstream_conftest.RemoteServer.__name__ != "RemoteServer":
        raise RuntimeError("upstream RemoteServer fixture source changed")
    setattr(upstream_conftest, "RemoteServer", SyntheticRemoteServer)
    fixture = upstream_conftest.launch_server._get_wrapped_function()
    if fixture.__globals__.get("RemoteServer") is not SyntheticRemoteServer:
        raise RuntimeError("launch_server does not resolve synthetic RemoteServer")
    _record(
        "fixture-adapted",
        fixture="launch_server",
        testSourceAdapted=False,
        browserProcessLaunched=False,
        upstreamCommit=UPSTREAM_COMMIT,
    )


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    upstream = Path(_setting("PWRIGHT_PYTHON_NAV_CLOSED_UPSTREAM")).resolve()
    engine = _setting("PWRIGHT_PYTHON_NAV_CLOSED_ENGINE")
    expected = f"{TEST_FILE}::{TEST_NAME}[{engine}]"
    selected = [f"{item.path.relative_to(upstream).as_posix()}::{item.name}" for item in items]
    if selected != [expected]:
        raise pytest.UsageError("navigation-closed diagnostic selection changed")
    fixture_names = cast(list[str], getattr(items[0], "fixturenames", []))
    if "launch_server" not in fixture_names:
        raise pytest.UsageError("test does not use adapted launch_server fixture")
    _record("selected", caseId=expected, testSourceAdapted=False)


def pytest_runtest_logreport(report: pytest.TestReport) -> None:
    if report.when == "call":
        _record("case-call", caseId=report.nodeid, outcome=report.outcome)
