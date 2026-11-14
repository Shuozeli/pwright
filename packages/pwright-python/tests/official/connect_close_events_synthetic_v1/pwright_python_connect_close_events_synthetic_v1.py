"""Fixture-only adapter for one unchanged pinned Python close-event test body."""

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
SOURCE_HASHES = {
    "tests/sync/test_browsertype_connect.py": (
        "c71497683c6ed7b932dc12a02ac304d51370e32fdfe049aca22d745754d0a58b"
    ),
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/sync/conftest.py": "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b",
}
CASE = (
    "tests/sync/test_browsertype_connect.py::"
    "test_browser_type_connect_should_forward_close_events_to_pages[chromium]"
)
_remotes: list[SyntheticRemoteServer] = []


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"missing close-events fixture setting: {name}")
    return value


def _record(kind: str, **fields: object) -> None:
    with Path(_setting("PWRIGHT_PYTHON_CLOSE_EVENTS_RECEIPT")).open("a") as stream:
        stream.write(
            json.dumps(
                {
                    "kind": kind,
                    "mode": _setting("PWRIGHT_PYTHON_CLOSE_EVENTS_MODE"),
                    "fixtureHost": _setting("PWRIGHT_OFFICIAL_TAILSCALE_HOST"),
                    "fixtureBind": _setting("PWRIGHT_OFFICIAL_TAILSCALE_IP"),
                    **fields,
                },
                sort_keys=True,
            )
            + "\n"
        )


class SyntheticRemoteServer:
    """Exact RemoteServer interface exercised by the unchanged upstream test."""

    def __init__(self, browser_name: str, launch_server_options: dict[str, Any], tmpfile: Path):
        if browser_name != "chromium" or set(launch_server_options) != {"headless"}:
            raise RuntimeError("unexpected synthetic close-events browser launch request")
        if (
            not isinstance(launch_server_options["headless"], bool)
            or tmpfile.name != "settings-0.json"
        ):
            raise RuntimeError("synthetic close-events fixture options changed")
        server_script = Path(_setting("PWRIGHT_PYTHON_CLOSE_EVENTS_SERVER"))
        if not server_script.is_file() or server_script.is_symlink():
            raise RuntimeError("synthetic close-events server source is missing or symlinked")
        env = os.environ.copy()
        env["PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE"] = browser_name
        self.process = subprocess.Popen(
            ["node", str(server_script)],
            stdout=subprocess.PIPE,
            stderr=sys.stderr,
            text=True,
            env=env,
        )
        try:
            assert self.process.stdout is not None
            ready, _, _ = select.select([self.process.stdout], [], [], 10)
            if not ready:
                raise RuntimeError("synthetic close-events server did not announce endpoint")
            line = self.process.stdout.readline().strip()
            expected = f"Listening on ws://{_setting('PWRIGHT_OFFICIAL_TAILSCALE_HOST')}:"
            if not line.startswith(expected) or not line.endswith(
                "/browser-synthetic-python-close-events"
            ):
                raise RuntimeError(f"invalid synthetic close-events endpoint: {line!r}")
            self.ws_endpoint = line.removeprefix("Listening on ")
            self.process.stdout.close()
            _remotes.append(self)
            _record(
                "fixture-created",
                engine=browser_name,
                serverPid=self.process.pid,
                endpointPath="/browser-synthetic-python-close-events",
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
            return
        self.process.send_signal(signal.SIGINT)
        try:
            return_code = self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=5)
            raise RuntimeError("synthetic close-events server did not stop after SIGINT") from None
        _record("fixture-stopped", serverPid=self.process.pid, returnCode=return_code)


def _cleanup_remotes() -> None:
    for remote in _remotes:
        if remote.process.poll() is None:
            remote.kill()


atexit.register(_cleanup_remotes)


def pytest_configure() -> None:
    upstream = Path(_setting("PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM")).resolve()
    mode = _setting("PWRIGHT_PYTHON_CLOSE_EVENTS_MODE")
    if mode not in ("oracle", "candidate"):
        raise RuntimeError("synthetic close-events mode changed")
    if _setting("PWRIGHT_PYTHON_CLOSE_EVENTS_ENGINE_FILTER") != "chromium":
        raise RuntimeError("synthetic close-events diagnostic requires Chromium")
    for relative, digest in SOURCE_HASHES.items():
        source = upstream / relative
        if source.is_symlink() or hashlib.sha256(source.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"pinned Python source changed: {relative}")
    if any(os.environ.get(key) for key in ("PWRIGHT_CDP_ENDPOINT", "PWRIGHT_CDP_WS_URL")):
        raise RuntimeError("browser-free Python diagnostic cannot use CDP")
    api = importlib.import_module("playwright.sync_api")
    candidate = importlib.import_module("pwright") if mode == "candidate" else None
    api_path = api.__file__
    candidate_path = candidate.__file__ if candidate is not None else None
    if api_path is None or (candidate is not None and candidate_path is None):
        raise RuntimeError("close-events runtime package has no source path")
    _record(
        "runtime-import",
        mode=mode,
        publicApiPath=str(Path(api_path).resolve()),
        candidatePackagePath=(
            str(Path(candidate_path).resolve()) if candidate_path is not None else None
        ),
    )
    upstream_conftest = importlib.import_module("tests.conftest")
    setattr(upstream_conftest, "RemoteServer", SyntheticRemoteServer)
    fixture = getattr(upstream_conftest, "launch_server")
    fixture_function = fixture._get_wrapped_function()
    if fixture_function.__globals__.get("RemoteServer") is not SyntheticRemoteServer:
        raise RuntimeError("launch_server fixture did not resolve synthetic RemoteServer")
    _record(
        "fixture-adapted",
        fixture="launch_server",
        testSourceAdapted=False,
        browserProcessLaunched=False,
        upstreamCommit=UPSTREAM_COMMIT,
    )


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    upstream = Path(_setting("PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM")).resolve()
    selected = [f"{item.path.relative_to(upstream).as_posix()}::{item.name}" for item in items]
    if selected != [CASE]:
        raise pytest.UsageError("synthetic close-events selection is not the pinned case")
    item = items[0]
    fixture_names = cast(list[str], getattr(item, "fixturenames", []))
    if "launch_server" not in fixture_names:
        raise pytest.UsageError("selected test does not use synthetic launch_server fixture")
    _record("selected", caseId=CASE, testSourceAdapted=False)


def pytest_runtest_logreport(report: pytest.TestReport) -> None:
    if report.when == "call":
        _record("case-call", caseId=report.nodeid, outcome=report.outcome)
