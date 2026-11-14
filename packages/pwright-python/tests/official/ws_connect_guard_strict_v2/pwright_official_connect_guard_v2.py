"""Observe the unchanged official BrowserType.connect early-close test.

Only the test's literal localhost destination is translated to the verified
Tailscale fixture host. The public method, upstream handler, inbound message,
and fixture sendClose are all delegated to their real implementations.
"""

from __future__ import annotations

import hashlib
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
_mode: str | None = None
_state = "unconfigured"
_registrations = 0


class _FixtureServer(Protocol):
    PORT: int


class _TestServer(Protocol):
    server: _FixtureServer


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise pytest.UsageError(f"missing strict BrowserType.connect v2 setting: {name}")
    return value


def _receipt(event: dict[str, object]) -> None:
    path = Path(_setting("PWRIGHT_PYTHON_CONNECT_V2_RECEIPT"))
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise pytest.UsageError("strict connect v2 receipt is not a regular file")
    record = {
        "schema": 2,
        "mode": _mode,
        "testId": _selected_test,
        "runNonce": _setting("PWRIGHT_PYTHON_CONNECT_V2_NONCE"),
        **event,
    }
    descriptor = os.open(path, os.O_WRONLY | os.O_APPEND | getattr(os, "O_NOFOLLOW", 0))
    try:
        os.write(descriptor, (json.dumps(record, sort_keys=True) + "\n").encode())
    finally:
        os.close(descriptor)


def _advance(expected: str, next_state: str, event: dict[str, object]) -> None:
    global _state
    if _state != expected:
        raise pytest.UsageError(f"strict connect v2 event order differs: {_state}")
    _receipt(event)
    _state = next_state


def _observe_fixture_ws(ws: object, handler: Callable[[object], None]) -> None:
    _advance("called", "open", {"event": "open"})
    handler(ws)
    observed_message = cast(Callable[[bytes | str, bool], None], getattr(ws, "onMessage"))
    original_close = cast(Callable[..., object], getattr(ws, "sendClose"))
    if "onMessage" not in vars(ws):
        raise pytest.UsageError("upstream test did not install its WS message handler")

    def observed_close(*args: object, **kwargs: object) -> object:
        if args or kwargs != {"code": 4123, "reason": "Oh my!"}:
            raise pytest.UsageError("upstream fixture close code/reason changed")
        _advance(
            "message",
            "closed",
            {"event": "sendClose", "code": 4123, "reason": "Oh my!"},
        )
        return original_close(*args, **kwargs)

    def observed_inbound(payload: bytes | str, is_binary: bool) -> None:
        raw = payload.encode() if isinstance(payload, str) else payload
        if not isinstance(raw, bytes) or not raw or is_binary is not False:
            raise pytest.UsageError("strict connect v2 inbound WS message is empty or invalid")
        try:
            message = json.loads(raw)
        except (json.JSONDecodeError, UnicodeDecodeError) as error:
            raise pytest.UsageError("strict connect v2 inbound frame is not JSON") from error
        if not isinstance(message, dict):
            raise pytest.UsageError("strict connect v2 inbound frame is not an object")
        params = message.get("params")
        metadata = message.get("metadata")
        message_id = message.get("id")
        if (
            message.get("guid") != ""
            or message.get("method") != "initialize"
            or not isinstance(params, dict)
            or params.get("sdkLanguage") != "python"
            or type(message_id) is not int
            or message_id <= 0
            or not isinstance(metadata, dict)
            or metadata.get("apiName") != "Connection.init"
            or type(metadata.get("wallTime")) is not int
            or metadata["wallTime"] <= 0
        ):
            raise pytest.UsageError("strict connect v2 inbound frame is not protocol initialize")
        _advance(
            "open",
            "message",
            {
                "event": "message",
                "payloadLength": len(raw),
                "payloadSha256": hashlib.sha256(raw).hexdigest(),
                "isBinary": is_binary,
                "guid": "",
                "method": "initialize",
                "sdkLanguage": "python",
                "messageId": message_id,
                "apiName": "Connection.init",
                "wallTime": metadata["wallTime"],
            },
        )
        observed_message(payload, is_binary)

    setattr(ws, "sendClose", observed_close)
    setattr(ws, "onMessage", observed_inbound)


def pytest_configure() -> None:
    global _selected_test, _mode, _state, _registrations
    test_id = _setting("PWRIGHT_PYTHON_CONNECT_V2_EXPECTED_TEST")
    if test_id not in {f"{SOURCE}::{NAME}[{engine}]" for engine in ENGINES}:
        raise pytest.UsageError("strict connect v2 test ID is outside policy")
    mode = _setting("PWRIGHT_OFFICIAL_MODE")
    if mode not in ("oracle", "candidate"):
        raise pytest.UsageError("strict connect v2 mode is outside policy")
    upstream = Path(_setting("PWRIGHT_PYTHON_CONNECT_V2_UPSTREAM"))
    if not upstream.is_absolute() or not (upstream / SOURCE).is_file():
        raise pytest.UsageError("strict connect v2 upstream is invalid")
    port_text = _setting("PWRIGHT_PYTHON_CONNECT_V2_PORT")
    if not port_text.isdecimal() or not 1024 <= int(port_text) < 65535:
        raise pytest.UsageError("strict connect v2 fixture port is invalid")
    port = int(port_text)
    host = _setting("PWRIGHT_OFFICIAL_TAILSCALE_HOST")
    bind = _setting("PWRIGHT_OFFICIAL_TAILSCALE_IP")
    nonce = _setting("PWRIGHT_PYTHON_CONNECT_V2_NONCE")
    if not _HOST.fullmatch(host) or not _NONCE.fullmatch(nonce):
        raise pytest.UsageError("strict connect v2 host or nonce is invalid")
    resolved = {
        entry[4][0]
        for entry in socket.getaddrinfo(host, port, family=socket.AF_INET, type=socket.SOCK_STREAM)
    }
    if resolved != {bind} or _selected_test is not None:
        raise pytest.UsageError("strict connect v2 MagicDNS or guard installation differs")

    fixture = cast(_TestServer, getattr(import_module("tests.server"), "test_server"))
    fixture.server.PORT = port
    server_class = getattr(import_module("tests.server"), "Server")
    register = cast(Callable[..., None], getattr(server_class, "once_web_socket_connection"))

    def observed_registration(self: object, handler: Callable[[object], None]) -> None:
        global _registrations
        _registrations += 1
        if _registrations != 1 or _state != "configured":
            raise pytest.UsageError("strict connect v2 fixture registration differs")
        register(self, lambda ws: _observe_fixture_ws(ws, handler))

    setattr(server_class, "once_web_socket_connection", observed_registration)
    public_class = getattr(import_module("playwright.async_api"), "BrowserType")
    connect = getattr(public_class, "connect", None)
    if not callable(connect):
        raise pytest.UsageError("strict connect v2 requires real public BrowserType.connect")
    original = cast(Callable[..., Awaitable[object]], connect)
    engine = test_id.rsplit("[", 1)[1].removesuffix("]")

    async def observed_connect(self: object, *args: object, **kwargs: object) -> object:
        literal = f"ws://localhost:{port}/ws"
        forwarded = f"ws://{host}:{port}/ws"
        if (
            _registrations != 1
            or getattr(self, "name", None) != engine
            or args != (literal,)
            or kwargs
        ):
            raise pytest.UsageError("official BrowserType.connect invocation changed")
        _advance(
            "configured",
            "called",
            {
                "event": "publicCall",
                "browserName": engine,
                "method": "connect",
                "fixturePort": port,
                "originalEndpoint": literal,
                "forwardedEndpoint": forwarded,
            },
        )
        return await original(self, forwarded)

    setattr(public_class, "connect", observed_connect)
    _selected_test = test_id
    _mode = mode
    _state = "configured"
    _registrations = 0


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    if _selected_test is None:
        raise pytest.UsageError("strict connect v2 guard was not configured")
    source, _, selected = _selected_test.partition("::")
    upstream = Path(_setting("PWRIGHT_PYTHON_CONNECT_V2_UPSTREAM"))
    if (
        len(items) != 1
        or Path(str(items[0].path)).resolve() != (upstream / source).resolve()
        or items[0].name != selected
    ):
        raise pytest.UsageError("strict connect v2 collection differs")


def pytest_sessionfinish(session: pytest.Session) -> None:
    if _selected_test is not None and (_state != "closed" or _registrations != 1):
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
