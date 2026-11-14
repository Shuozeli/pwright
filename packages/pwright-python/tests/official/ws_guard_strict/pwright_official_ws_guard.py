"""Exact transport adapter for two unchanged official CDP WebSocket tests.

The upstream test passes a loopback literal while the existing fixture adapter
binds only the Tailscale interface. This guard rewrites only the transport
destination. It does not catch exceptions, alter headers, or inspect results.
"""

from __future__ import annotations

import json
import os
import re
import socket
from collections.abc import Awaitable, Callable
from importlib import import_module
from pathlib import Path
from typing import Protocol, cast

import pytest

TEST_NAMES = frozenset(
    {
        "test_connect_over_cdp_passing_header_works",
        "test_should_print_custom_ws_close_error",
    }
)
SOURCE = "tests/async/test_browsertype_connect_cdp.py"
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_HOST = re.compile(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net\Z")
_selected_test: str | None = None
_invocations = 0


class _FixtureServer(Protocol):
    PORT: int


class _TestServer(Protocol):
    server: _FixtureServer


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise pytest.UsageError(f"missing strict WebSocket setting: {name}")
    return value


def _receipt(record: dict[str, object]) -> None:
    path = Path(_setting("PWRIGHT_PYTHON_WS_RECEIPT"))
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise pytest.UsageError("strict WebSocket receipt path is not a regular file")
    flags = os.O_WRONLY | os.O_APPEND | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        os.write(descriptor, (json.dumps(record, sort_keys=True) + "\n").encode())
    finally:
        os.close(descriptor)


def pytest_configure() -> None:
    global _selected_test, _invocations
    test_id = _setting("PWRIGHT_PYTHON_WS_EXPECTED_TEST")
    if test_id not in {f"{SOURCE}::{name}[chromium]" for name in TEST_NAMES}:
        raise pytest.UsageError("strict WebSocket test ID is outside the two-case policy")
    upstream = Path(_setting("PWRIGHT_PYTHON_WS_UPSTREAM"))
    if not upstream.is_absolute() or not (upstream / SOURCE).is_file():
        raise pytest.UsageError("strict WebSocket upstream checkout is invalid")
    port_text = _setting("PWRIGHT_PYTHON_WS_PORT")
    if not port_text.isdecimal() or not 1024 <= int(port_text) <= 65535:
        raise pytest.UsageError("strict WebSocket fixture port is invalid")
    port = int(port_text)
    host = _setting("PWRIGHT_OFFICIAL_TAILSCALE_HOST")
    bind = _setting("PWRIGHT_OFFICIAL_TAILSCALE_IP")
    nonce = _setting("PWRIGHT_PYTHON_WS_NONCE")
    if not _HOST.fullmatch(host) or not _NONCE.fullmatch(nonce):
        raise pytest.UsageError("strict WebSocket host or nonce is invalid")
    resolved = {
        entry[4][0]
        for entry in socket.getaddrinfo(host, port, family=socket.AF_INET, type=socket.SOCK_STREAM)
    }
    if resolved != {bind}:
        raise pytest.UsageError("strict WebSocket MagicDNS does not resolve to fixture bind")

    if _selected_test is not None:
        raise pytest.UsageError("strict WebSocket adapter installed twice")
    _selected_test = test_id
    _invocations = 0
    fixture = cast(_TestServer, getattr(import_module("tests.server"), "test_server"))
    fixture.server.PORT = port
    public_class = getattr(import_module("playwright.async_api"), "BrowserType")
    original = cast(Callable[..., Awaitable[object]], getattr(public_class, "connect_over_cdp"))

    async def adapted_connect_over_cdp(self: object, *args: object, **kwargs: object) -> object:
        global _invocations
        literal = f"ws://127.0.0.1:{port}/ws"
        forwarded = f"ws://{host}:{port}/ws"
        if (
            _invocations != 0
            or getattr(self, "name", None) != "chromium"
            or args != (literal,)
            or kwargs != {"headers": {"foo": "bar"}}
        ):
            raise pytest.UsageError("strict WebSocket call differs from pinned upstream call")
        _invocations += 1
        _receipt(
            {
                "schema": 1,
                "testId": test_id,
                "mode": _setting("PWRIGHT_OFFICIAL_MODE"),
                "runNonce": nonce,
                "fixturePort": port,
                "originalEndpoint": literal,
                "forwardedEndpoint": forwarded,
                "headers": {"foo": "bar"},
            }
        )
        return await original(self, forwarded, **kwargs)

    setattr(public_class, "connect_over_cdp", adapted_connect_over_cdp)


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    if _selected_test is None:
        raise pytest.UsageError("strict WebSocket adapter was not configured")
    source, _, selected = _selected_test.partition("::")
    upstream = Path(_setting("PWRIGHT_PYTHON_WS_UPSTREAM"))
    if (
        len(items) != 1
        or Path(str(items[0].path)).resolve() != (upstream / source).resolve()
        or items[0].name != selected
    ):
        raise pytest.UsageError("strict WebSocket collection is not the one pinned case")


def pytest_sessionfinish(session: pytest.Session) -> None:
    if _selected_test is not None and _invocations != 1:
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
