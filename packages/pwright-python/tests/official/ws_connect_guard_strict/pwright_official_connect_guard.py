"""One-URL transport guard for the unchanged official BrowserType.connect case.

This fixture adapter changes only the selected server's hard-coded localhost
destination to the same port on this Mac's full Tailscale MagicDNS name. It
does not emulate BrowserType.connect or catch its errors. In the candidate
lane the absent method is deliberately left absent for the upstream assertion.
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

SOURCE = "tests/async/test_browsertype_connect.py"
NAME = "test_should_print_custom_ws_close_error"
ENGINES = ("chromium", "firefox", "webkit")
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_HOST = re.compile(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net\Z")
_selected_test: str | None = None
_invocations = 0
_method_present = False


class _FixtureServer(Protocol):
    PORT: int


class _TestServer(Protocol):
    server: _FixtureServer


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise pytest.UsageError(f"missing strict BrowserType.connect setting: {name}")
    return value


def _receipt(record: dict[str, object]) -> None:
    path = Path(_setting("PWRIGHT_PYTHON_CONNECT_RECEIPT"))
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise pytest.UsageError("strict BrowserType.connect receipt is not a regular file")
    flags = os.O_WRONLY | os.O_APPEND | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        os.write(descriptor, (json.dumps(record, sort_keys=True) + "\n").encode())
    finally:
        os.close(descriptor)


def pytest_configure() -> None:
    global _selected_test, _invocations, _method_present
    test_id = _setting("PWRIGHT_PYTHON_CONNECT_EXPECTED_TEST")
    if test_id not in {f"{SOURCE}::{NAME}[{engine}]" for engine in ENGINES}:
        raise pytest.UsageError("strict BrowserType.connect test ID is outside policy")
    upstream = Path(_setting("PWRIGHT_PYTHON_CONNECT_UPSTREAM"))
    if not upstream.is_absolute() or not (upstream / SOURCE).is_file():
        raise pytest.UsageError("strict BrowserType.connect upstream is invalid")
    port_text = _setting("PWRIGHT_PYTHON_CONNECT_PORT")
    if not port_text.isdecimal() or not 1024 <= int(port_text) < 65535:
        raise pytest.UsageError("strict BrowserType.connect fixture port is invalid")
    port = int(port_text)
    host = _setting("PWRIGHT_OFFICIAL_TAILSCALE_HOST")
    bind = _setting("PWRIGHT_OFFICIAL_TAILSCALE_IP")
    nonce = _setting("PWRIGHT_PYTHON_CONNECT_NONCE")
    if not _HOST.fullmatch(host) or not _NONCE.fullmatch(nonce):
        raise pytest.UsageError("strict BrowserType.connect host or nonce is invalid")
    resolved = {
        entry[4][0]
        for entry in socket.getaddrinfo(host, port, family=socket.AF_INET, type=socket.SOCK_STREAM)
    }
    if resolved != {bind}:
        raise pytest.UsageError("strict BrowserType.connect MagicDNS differs from bind")
    if _selected_test is not None:
        raise pytest.UsageError("strict BrowserType.connect guard installed twice")

    _selected_test = test_id
    _invocations = 0
    fixture = cast(_TestServer, getattr(import_module("tests.server"), "test_server"))
    fixture.server.PORT = port
    public_class = getattr(import_module("playwright.async_api"), "BrowserType")
    original = getattr(public_class, "connect", None)
    _method_present = callable(original)
    if not _method_present:
        if _setting("PWRIGHT_OFFICIAL_MODE") != "candidate" or original is not None:
            raise pytest.UsageError("public BrowserType.connect has unexpected shape")
        return

    if _setting("PWRIGHT_OFFICIAL_MODE") != "oracle":
        raise pytest.UsageError("this frozen candidate wheel unexpectedly has connect")
    connect = cast(Callable[..., Awaitable[object]], original)
    engine = test_id.rsplit("[", 1)[1].removesuffix("]")

    async def adapted_connect(self: object, *args: object, **kwargs: object) -> object:
        global _invocations
        literal = f"ws://localhost:{port}/ws"
        forwarded = f"ws://{host}:{port}/ws"
        if (
            _invocations != 0
            or getattr(self, "name", None) != engine
            or args != (literal,)
            or kwargs
        ):
            raise pytest.UsageError("official BrowserType.connect invocation changed")
        _invocations += 1
        _receipt(
            {
                "schema": 1,
                "testId": test_id,
                "mode": "oracle",
                "runNonce": nonce,
                "browserName": engine,
                "method": "connect",
                "fixturePort": port,
                "originalEndpoint": literal,
                "forwardedEndpoint": forwarded,
            }
        )
        return await connect(self, forwarded)

    setattr(public_class, "connect", adapted_connect)


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    if _selected_test is None:
        raise pytest.UsageError("strict BrowserType.connect guard was not configured")
    source, _, selected = _selected_test.partition("::")
    upstream = Path(_setting("PWRIGHT_PYTHON_CONNECT_UPSTREAM"))
    if (
        len(items) != 1
        or Path(str(items[0].path)).resolve() != (upstream / source).resolve()
        or items[0].name != selected
    ):
        raise pytest.UsageError("strict BrowserType.connect collection differs")


def pytest_sessionfinish(session: pytest.Session) -> None:
    expected = 1 if _method_present else 0
    if _selected_test is not None and _invocations != expected:
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
