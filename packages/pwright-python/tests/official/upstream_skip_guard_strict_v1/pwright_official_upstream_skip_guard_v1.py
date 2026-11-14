"""Fail closed if an official marker-skip case reaches its test body."""

from __future__ import annotations

import json
import os
from collections.abc import Iterator
from pathlib import Path
from typing import Any, cast

import pytest


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"missing strict upstream skip setting: {name}")
    return value


def _event(name: str, **fields: object) -> None:
    receipt = Path(_setting("PWRIGHT_PYTHON_SKIP_V1_RECEIPT"))
    payload = {
        "schema": 1,
        "event": name,
        "mode": _setting("PWRIGHT_OFFICIAL_MODE"),
        "testId": _setting("PWRIGHT_PYTHON_SKIP_V1_EXPECTED_TEST"),
        "runNonce": _setting("PWRIGHT_PYTHON_SKIP_V1_NONCE"),
        **fields,
    }
    with receipt.open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(payload, sort_keys=True) + "\n")
        stream.flush()
        os.fsync(stream.fileno())


def _source_id(item: pytest.Item) -> str:
    upstream = Path(_setting("PWRIGHT_PYTHON_SKIP_V1_UPSTREAM"))
    source = Path(str(item.path)).resolve().relative_to(upstream)
    return f"{source.as_posix()}::{item.name}"


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    expected = _setting("PWRIGHT_PYTHON_SKIP_V1_EXPECTED_TEST")
    selected = [_source_id(item) for item in items]
    _event("collection", selected=selected)
    if selected != [expected]:
        raise RuntimeError("strict upstream skip selection differs from one pinned case")


@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item: pytest.Item, call: Any) -> Iterator[None]:
    outcome = cast(Any, (yield))
    report = cast(pytest.TestReport, outcome.get_result())
    if report.when == "setup":
        _event("setup", selected=_source_id(item), outcome=report.outcome)


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_call(item: pytest.Item) -> None:
    _event("bodyAttempted", selected=_source_id(item))
    pytest.fail("strict upstream marker-skip case reached its test body")
