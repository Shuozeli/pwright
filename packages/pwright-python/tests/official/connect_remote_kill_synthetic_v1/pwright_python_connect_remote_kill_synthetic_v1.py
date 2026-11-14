"""Select four unchanged Python remote-kill test bodies over a synthetic server.

Only the upstream launch_server fixture's RemoteServer binding is replaced.
The assertions and the public API calls in each upstream test remain unchanged.
"""

from __future__ import annotations

import hashlib
import importlib
from pathlib import Path
from typing import cast

import pytest
import pwright_python_connect_close_events_synthetic_v1 as legacy

UPSTREAM_COMMIT = legacy.UPSTREAM_COMMIT
ASYNC_SOURCE_SHA256 = "d47290f5acb45246ce2bae150d3c583d340806821b660be08d798ac3572e7b11"
ASYNC_CONFTEST_SHA256 = "3001008bb97ba0cc1ddb397a53df530b6881e3e3ba69a1ed2cac0de94cab4cdb"
CASE_IDS = (
    "tests/async/test_browsertype_connect.py::test_browser_type_connect_disconnected_event_should_be_emitted_when_remote_killed_connection[chromium]",
    "tests/async/test_browsertype_connect.py::test_browser_type_connect_should_throw_when_used_after_is_connected_returns_false[chromium]",
    "tests/sync/test_browsertype_connect.py::test_browser_type_connect_should_forward_close_events_on_remote_kill[chromium]",
    "tests/sync/test_browsertype_connect.py::test_browser_type_connect_should_throw_when_used_after_is_connected_returns_false[chromium]",
)


def pytest_configure() -> None:
    upstream = Path(legacy._setting("PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM")).resolve()
    for relative, expected in (
        ("tests/async/test_browsertype_connect.py", ASYNC_SOURCE_SHA256),
        ("tests/async/conftest.py", ASYNC_CONFTEST_SHA256),
    ):
        source = upstream / relative
        if source.is_symlink() or hashlib.sha256(source.read_bytes()).hexdigest() != expected:
            raise RuntimeError(f"pinned async Python source changed: {relative}")
    legacy.pytest_configure()
    api = importlib.import_module("playwright.async_api")
    if api.__file__ is None:
        raise RuntimeError("async public API has no source path")
    legacy._record("runtime-import-async", publicApiPath=str(Path(api.__file__).resolve()))


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    upstream = Path(legacy._setting("PWRIGHT_PYTHON_CLOSE_EVENTS_UPSTREAM")).resolve()
    selected = [f"{item.path.relative_to(upstream).as_posix()}::{item.name}" for item in items]
    if selected != list(CASE_IDS):
        raise pytest.UsageError("remote-kill diagnostic did not collect the exact four cases")
    for item in items:
        fixture_defs = item._fixtureinfo.name2fixturedefs.get("launch_server", ())
        if (
            not fixture_defs
            or fixture_defs[-1].func.__globals__.get("RemoteServer")
            is not legacy.SyntheticRemoteServer
        ):
            raise pytest.UsageError("selected test does not use synthetic launch_server")
        names = cast(list[str], getattr(item, "fixturenames", []))
        if "launch_server" not in names:
            raise pytest.UsageError("selected test lost launch_server fixture")
    legacy._record("selected", caseIds=selected, fixtureAdapted=True, testSourceAdapted=False)


def pytest_runtest_logreport(report: pytest.TestReport) -> None:
    legacy.pytest_runtest_logreport(report)
