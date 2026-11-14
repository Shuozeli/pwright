"""Select exactly six unchanged upstream state assertions over the pinned v1 fixture.

The only adaptation is the existing in-memory launch_server RemoteServer binding.
This diagnostic is deliberately not an unadapted official test or ledger run.
"""

from __future__ import annotations

from pathlib import Path

import pytest
import pwright_python_connect_lifecycle_synthetic_v1 as fixture

ENGINES = ("chromium", "firefox", "webkit")
FUNCTION = "test_browser_type_connect_set_browser_connected_state"
EXPECTED_IDS = {
    f"tests/{api}/test_browsertype_connect.py::{FUNCTION}[{engine}]"
    for api in ("async", "sync")
    for engine in ENGINES
}


def pytest_configure() -> None:
    fixture.pytest_configure()


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    upstream = Path(fixture._setting("PWRIGHT_PYTHON_LIFECYCLE_UPSTREAM")).resolve()
    engine = fixture._setting("PWRIGHT_PYTHON_LIFECYCLE_ENGINE_FILTER")
    if engine not in ENGINES:
        raise pytest.UsageError("unexpected state-lifecycle engine")
    expected = {case_id for case_id in EXPECTED_IDS if case_id.endswith(f"[{engine}]")}
    selected = {f"{item.path.relative_to(upstream).as_posix()}::{item.name}" for item in items}
    if selected != expected or len(items) != 2:
        raise pytest.UsageError("state-lifecycle selection is not the exact two engine cases")
    for item in items:
        fixture_defs = item._fixtureinfo.name2fixturedefs.get("launch_server", ())
        if (
            not fixture_defs
            or fixture_defs[-1].func.__globals__.get("RemoteServer")
            is not fixture.SyntheticRemoteServer
        ):
            raise pytest.UsageError("selected test does not use the adapted launch_server")
    fixture._record("selected", caseIds=sorted(selected), testSourceAdapted=False)


def pytest_runtest_logreport(report: pytest.TestReport) -> None:
    fixture.pytest_runtest_logreport(report)
