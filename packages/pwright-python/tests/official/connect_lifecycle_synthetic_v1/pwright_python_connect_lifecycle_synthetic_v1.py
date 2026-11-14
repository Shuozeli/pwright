"""Fixture-only, browser-free adapter for four pinned Python connect test bodies."""

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
from typing import Any

import pytest

UPSTREAM_COMMIT = "47a5d35ef4f815a2021349f86ae391f7c20c02d6"
SOURCE_HASHES = {
    "tests/async/test_browsertype_connect.py": (
        "d47290f5acb45246ce2bae150d3c583d340806821b660be08d798ac3572e7b11"
    ),
    "tests/sync/test_browsertype_connect.py": (
        "c71497683c6ed7b932dc12a02ac304d51370e32fdfe049aca22d745754d0a58b"
    ),
    "tests/conftest.py": "c4236e98ff1731fff1f7ef1c00b37fe3a0cb1e4a3fd07e89a491dc7129ea93cc",
    "tests/async/conftest.py": "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb",
    "tests/sync/conftest.py": "9ec5a65a98c312de67590792e80d3d9ea45312d56188f45101e71df6f7773d0b",
}
FUNCTIONS = (
    "test_browser_type_disconnected_event_should_have_browser_as_argument",
    "test_browser_type_connect_set_browser_connected_state",
)
ENGINES = ("chromium", "firefox", "webkit")
EXPECTED_IDS = {
    f"tests/{api}/test_browsertype_connect.py::{function}[{engine}]"
    for api in ("async", "sync")
    for function in FUNCTIONS
    for engine in ENGINES
}
_remotes: list[SyntheticRemoteServer] = []


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"missing synthetic fixture setting: {name}")
    return value


def _record(kind: str, **fields: object) -> None:
    receipt = Path(_setting("PWRIGHT_PYTHON_LIFECYCLE_RECEIPT"))
    with receipt.open("a") as stream:
        stream.write(
            json.dumps(
                {
                    "kind": kind,
                    "mode": _setting("PWRIGHT_PYTHON_LIFECYCLE_MODE"),
                    "fixtureHost": _setting("PWRIGHT_OFFICIAL_TAILSCALE_HOST"),
                    "fixtureBind": _setting("PWRIGHT_OFFICIAL_TAILSCALE_IP"),
                    "harnessPid": os.getpid(),
                    **fields,
                },
                sort_keys=True,
            )
            + "\n"
        )


class SyntheticRemoteServer:
    """Exact RemoteServer interface used by the four unchanged test functions."""

    def __init__(self, browser_name: str, launch_server_options: dict[str, Any], tmpfile: Path):
        if browser_name not in ENGINES or set(launch_server_options) != {"headless"}:
            raise RuntimeError("synthetic fixture received an unexpected browser launch request")
        if (
            not isinstance(launch_server_options["headless"], bool)
            or tmpfile.name != "settings-0.json"
        ):
            raise RuntimeError("synthetic fixture options or slot changed")
        if any(
            os.environ.get(key)
            for key in ("PWRIGHT_CDP_ENDPOINT", "PWRIGHT_CDP_WS_URL", "NODE_OPTIONS")
        ):
            raise RuntimeError("synthetic lifecycle diagnostic rejects CDP or inherited Node hooks")
        server_script = Path(_setting("PWRIGHT_PYTHON_LIFECYCLE_SERVER"))
        if not server_script.is_file() or server_script.is_symlink():
            raise RuntimeError("synthetic server source is missing or symlinked")
        env = os.environ.copy()
        env["PWRIGHT_PYTHON_LIFECYCLE_ENGINE"] = browser_name
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
                raise RuntimeError("synthetic server did not announce an endpoint")
            line = self.process.stdout.readline().strip()
            expected_prefix = f"Listening on ws://{_setting('PWRIGHT_OFFICIAL_TAILSCALE_HOST')}:"
            if not line.startswith(expected_prefix) or not line.endswith(
                "/browser-synthetic-python"
            ):
                raise RuntimeError(f"synthetic server returned an invalid endpoint: {line!r}")
            self.ws_endpoint = line.removeprefix("Listening on ")
            self.process.stdout.close()
            _remotes.append(self)
            _record(
                "fixture-created",
                engine=browser_name,
                serverPid=self.process.pid,
                endpointPath="/browser-synthetic-python",
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
            _record(
                "fixture-premature-exit",
                serverPid=self.process.pid,
                returnCode=self.process.returncode,
            )
            return
        self.process.send_signal(signal.SIGINT)
        try:
            return_code = self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=5)
            raise RuntimeError("synthetic server did not stop after SIGINT") from None
        _record("fixture-stopped", serverPid=self.process.pid, returnCode=return_code)


def _cleanup_remotes() -> None:
    for remote in _remotes:
        if remote.process.poll() is None:
            remote.kill()


atexit.register(_cleanup_remotes)


def pytest_configure() -> None:
    upstream = Path(_setting("PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM")).resolve()
    mode = _setting("PWRIGHT_PYTHON_LIFECYCLE_MODE")
    if mode not in ("oracle", "candidate"):
        raise RuntimeError("synthetic lifecycle mode changed")
    if _setting("PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER") not in ENGINES:
        raise RuntimeError("synthetic lifecycle diagnostic requires one exact engine")
    for relative, digest in SOURCE_HASHES.items():
        source = upstream / relative
        if source.is_symlink() or hashlib.sha256(source.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"pinned Python source changed: {relative}")
    if any(os.environ.get(key) for key in ("PWRIGHT_CDP_ENDPOINT", "PWRIGHT_CDP_WS_URL")):
        raise RuntimeError("browser-free Python diagnostic cannot use CDP")
    public_api = importlib.import_module("playwright.async_api")
    candidate_package = importlib.import_module("pwright") if mode == "candidate" else None
    _record(
        "runtime-import",
        publicApiPath=str(Path(public_api.__file__).resolve()),
        candidatePackagePath=(
            str(Path(candidate_package.__file__).resolve())
            if candidate_package is not None
            else None
        ),
    )
    upstream_conftest = importlib.import_module("tests.conftest")
    if upstream_conftest.RemoteServer.__name__ != "RemoteServer":
        raise RuntimeError("upstream RemoteServer fixture source changed")
    upstream_conftest.RemoteServer = SyntheticRemoteServer
    fixture_function = upstream_conftest.launch_server._get_wrapped_function()
    if fixture_function.__globals__.get("RemoteServer") is not SyntheticRemoteServer:
        raise RuntimeError("launch_server fixture does not resolve to synthetic RemoteServer")
    _record(
        "fixture-adapted",
        fixture="launch_server",
        testSourceAdapted=False,
        fixtureAdapted=True,
        browserProcessLaunched=False,
        fixtureBindingIdentity=True,
        cdpEndpointPresent=False,
        upstreamCommit=UPSTREAM_COMMIT,
        upstreamFixtureSha256=SOURCE_HASHES["tests/conftest.py"],
    )


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    upstream = Path(_setting("PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM")).resolve()
    engine = _setting("PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER")
    expected = {case_id for case_id in EXPECTED_IDS if case_id.endswith(f"[{engine}]")}
    selected = {f"{item.path.relative_to(upstream).as_posix()}::{item.name}" for item in items}
    if selected != expected or len(items) != len(expected):
        raise pytest.UsageError("synthetic lifecycle selection is not the pinned four engine cases")
    for item in items:
        fixture_defs = item._fixtureinfo.name2fixturedefs.get("launch_server", ())
        if (
            not fixture_defs
            or fixture_defs[-1].func.__globals__.get("RemoteServer") is not SyntheticRemoteServer
        ):
            raise pytest.UsageError(
                "selected test does not use the synthetic launch_server fixture"
            )
    _record("selected", caseIds=sorted(selected), testSourceAdapted=False)


def pytest_runtest_logreport(report: pytest.TestReport) -> None:
    if report.when == "call":
        _record("case-call", caseId=report.nodeid, outcome=report.outcome)
