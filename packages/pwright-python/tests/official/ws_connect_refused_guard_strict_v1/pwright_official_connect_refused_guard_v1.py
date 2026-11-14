"""Fixture-only guard for the official closed-Playwright-Server connect test.

The upstream test body and assertions are untouched. Its ``launch_server``
fixture normally launches a browser before immediately killing its server.
For this deliberately narrower, browser-free transport slice, replace only
the RemoteServer constructor with a real Tailscale-bound TCP listener that
the unchanged test kills before calling the real public BrowserType.connect.
"""

from __future__ import annotations

import errno
import hashlib
import ipaddress
import json
import os
import re
import socket
from collections.abc import Awaitable, Callable
from importlib import import_module
from pathlib import Path
from typing import Protocol, cast

import pytest

NAME = "test_connect_to_closed_server_without_hangs"
SOURCES = (
    "tests/async/test_browsertype_connect.py",
    "tests/sync/test_browsertype_connect.py",
)
ENGINES = ("chromium", "firefox", "webkit")
TEST_IDS = tuple(f"{source}::{NAME}[{engine}]" for source in SOURCES for engine in ENGINES)
_HOST = re.compile(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*\.ts\.net\Z")
_NONCE = re.compile(r"[0-9a-f]{32}\Z")
_selected: str | None = None
_mode: str | None = None
_host: str | None = None
_bind: str | None = None
_port: int | None = None
_state = "unconfigured"
_created = 0
_killed = 0
_called = 0
_errored = 0
_original_remote_server: type[object] | None = None


class _BrowserType(Protocol):
    name: str


def _setting(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise pytest.UsageError(f"missing strict connect-refused setting: {name}")
    return value


def _receipt(event: dict[str, object]) -> None:
    path = Path(_setting("PWRIGHT_PYTHON_REFUSED_V1_RECEIPT"))
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise pytest.UsageError("strict connect-refused receipt is not a regular file")
    record = {
        "schema": 1,
        "mode": _mode,
        "testId": _selected,
        "runNonce": _setting("PWRIGHT_PYTHON_REFUSED_V1_NONCE"),
        **event,
    }
    encoded = (json.dumps(record, sort_keys=True) + "\n").encode()
    descriptor = os.open(path, os.O_WRONLY | os.O_APPEND | getattr(os, "O_NOFOLLOW", 0))
    try:
        if os.write(descriptor, encoded) != len(encoded):
            raise pytest.UsageError("strict connect-refused receipt write was incomplete")
    finally:
        os.close(descriptor)


def _advance(expected: str, successor: str, event: dict[str, object]) -> None:
    global _state
    if _state != expected:
        raise pytest.UsageError(f"strict connect-refused order differs: {_state}")
    _receipt(event)
    _state = successor


def _connect_probe() -> int:
    if _host is None or _port is None:
        raise pytest.UsageError("strict connect-refused fixture is unconfigured")
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.settimeout(2)
        return probe.connect_ex((_host, _port))


class _ClosedEndpointServer:
    """The narrow fixture adapter used by the unchanged upstream fixture."""

    def __init__(
        self, browser_name: str, launch_server_options: dict[str, object], tmpfile: Path
    ) -> None:
        global _created
        if _selected is None or _bind is None or _host is None or _port is None:
            raise pytest.UsageError("strict connect-refused fixture is unconfigured")
        engine = _selected.rsplit("[", 1)[1].removesuffix("]")
        if (
            _state != "configured"
            or _created != 0
            or browser_name != engine
            or launch_server_options != {"headless": True}
            or not isinstance(tmpfile, Path)
        ):
            raise pytest.UsageError("upstream launch_server fixture invocation changed")
        listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        try:
            listener.bind((_bind, _port))
            listener.listen(1)
            if _connect_probe() != 0:
                raise pytest.UsageError("Tailscale-bound fixture was not reachable before kill")
        except BaseException:
            listener.close()
            raise
        self._listener: socket.socket | None = listener
        self.ws_endpoint = f"ws://{_host}:{_port}/ws"
        _created += 1
        _advance(
            "configured",
            "listening",
            {
                "event": "fixtureListening",
                "browserName": browser_name,
                "bindIp": _bind,
                "fixturePort": _port,
                "endpoint": self.ws_endpoint,
                "reachableConnectErrno": 0,
                "originalRemoteServerLaunched": False,
            },
        )

    def kill(self) -> None:
        global _killed
        listener = self._listener
        if listener is None:
            if _state not in ("closed", "called", "errored"):
                raise pytest.UsageError("strict connect-refused fixture double kill is premature")
            return
        if _state != "listening" or _killed != 0:
            raise pytest.UsageError("strict connect-refused fixture kill order differs")
        listener.close()
        self._listener = None
        refused = _connect_probe()
        if refused != errno.ECONNREFUSED:
            raise pytest.UsageError(f"fixture did not refuse TCP after kill: {refused}")
        _killed += 1
        _advance(
            "listening",
            "closed",
            {"event": "fixtureClosed", "refusedConnectErrno": refused},
        )


def _record_public_error(error: BaseException, public_error: type[BaseException]) -> None:
    global _errored
    if not isinstance(error, public_error):
        raise pytest.UsageError(
            "public BrowserType.connect did not raise Playwright Error"
        ) from error
    raw_message = getattr(error, "message", str(error))
    if not isinstance(raw_message, str):
        raise pytest.UsageError("public BrowserType.connect error message is not text")
    _errored += 1
    _advance(
        "called",
        "errored",
        {
            "event": "publicError",
            "errorClass": type(error).__name__,
            "messageSha256": hashlib.sha256(raw_message.encode()).hexdigest(),
            "hasRequiredPrefix": "WebSocket error: " in raw_message,
        },
    )


def pytest_configure() -> None:
    global _selected, _mode, _host, _bind, _port, _state, _original_remote_server
    selected = _setting("PWRIGHT_PYTHON_REFUSED_V1_EXPECTED_TEST")
    if selected not in TEST_IDS:
        raise pytest.UsageError("strict connect-refused test ID is outside policy")
    mode = _setting("PWRIGHT_OFFICIAL_MODE")
    if mode not in ("oracle", "candidate"):
        raise pytest.UsageError("strict connect-refused mode is outside policy")
    upstream = Path(_setting("PWRIGHT_PYTHON_REFUSED_V1_UPSTREAM"))
    source = selected.split("::", 1)[0]
    if not upstream.is_absolute() or not (upstream / source).is_file():
        raise pytest.UsageError("strict connect-refused upstream is invalid")
    port_text = _setting("PWRIGHT_PYTHON_REFUSED_V1_PORT")
    if not port_text.isdecimal() or not 1024 <= int(port_text) < 65535:
        raise pytest.UsageError("strict connect-refused fixture port is invalid")
    port = int(port_text)
    host = _setting("PWRIGHT_OFFICIAL_TAILSCALE_HOST")
    bind = _setting("PWRIGHT_OFFICIAL_TAILSCALE_IP")
    nonce = _setting("PWRIGHT_PYTHON_REFUSED_V1_NONCE")
    try:
        address = ipaddress.ip_address(bind)
    except ValueError as error:
        raise pytest.UsageError("strict connect-refused bind IP is invalid") from error
    if (
        address not in ipaddress.ip_network("100.64.0.0/10")
        or not _HOST.fullmatch(host)
        or not _NONCE.fullmatch(nonce)
        or _selected is not None
    ):
        raise pytest.UsageError("strict connect-refused host, nonce or installation differs")
    resolved = {
        entry[4][0]
        for entry in socket.getaddrinfo(host, port, family=socket.AF_INET, type=socket.SOCK_STREAM)
    }
    if resolved != {bind}:
        raise pytest.UsageError("strict connect-refused MagicDNS does not resolve to bind IP")

    conftest = import_module("tests.conftest")
    original_remote_server = getattr(conftest, "RemoteServer")
    if not isinstance(original_remote_server, type):
        raise pytest.UsageError("upstream RemoteServer fixture class is missing")
    setattr(conftest, "RemoteServer", _ClosedEndpointServer)
    source_api = (
        "playwright.async_api" if source.startswith("tests/async/") else "playwright.sync_api"
    )
    api = import_module(source_api)
    public_class = getattr(api, "BrowserType")
    public_error = getattr(api, "Error")
    original_connect = getattr(public_class, "connect", None)
    if not callable(original_connect) or not isinstance(public_error, type):
        raise pytest.UsageError("real public BrowserType.connect/Error is missing")
    engine = selected.rsplit("[", 1)[1].removesuffix("]")
    endpoint = f"ws://{host}:{port}/ws"

    def check_public_call(
        self: _BrowserType, args: tuple[object, ...], kwargs: dict[str, object]
    ) -> None:
        global _called
        if (
            _state != "closed"
            or _called != 0
            or self.name != engine
            or args != (endpoint,)
            or kwargs
        ):
            raise pytest.UsageError("upstream public BrowserType.connect call changed")
        refused = _connect_probe()
        if refused != errno.ECONNREFUSED:
            raise pytest.UsageError(f"endpoint was not refused before public connect: {refused}")
        _called += 1
        _advance(
            "closed",
            "called",
            {
                "event": "publicCall",
                "browserName": engine,
                "method": "connect",
                "endpoint": endpoint,
                "preCallRefusedConnectErrno": refused,
            },
        )

    if source.startswith("tests/async/"):
        original_async = cast(Callable[..., Awaitable[object]], original_connect)

        async def observed_async(self: _BrowserType, *args: object, **kwargs: object) -> object:
            check_public_call(self, args, kwargs)
            try:
                return await original_async(self, *args, **kwargs)
            except BaseException as error:
                _record_public_error(error, public_error)
                raise

        setattr(public_class, "connect", observed_async)
    else:
        original_sync = cast(Callable[..., object], original_connect)

        def observed_sync(self: _BrowserType, *args: object, **kwargs: object) -> object:
            check_public_call(self, args, kwargs)
            try:
                return original_sync(self, *args, **kwargs)
            except BaseException as error:
                _record_public_error(error, public_error)
                raise

        setattr(public_class, "connect", observed_sync)
    _selected, _mode, _host, _bind, _port = selected, mode, host, bind, port
    _state = "configured"
    _original_remote_server = original_remote_server


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    if _selected is None:
        raise pytest.UsageError("strict connect-refused guard was not configured")
    source, _, selected = _selected.partition("::")
    upstream = Path(_setting("PWRIGHT_PYTHON_REFUSED_V1_UPSTREAM"))
    if (
        len(items) != 1
        or Path(str(items[0].path)).resolve() != (upstream / source).resolve()
        or items[0].name != selected
    ):
        raise pytest.UsageError("strict connect-refused collection differs")


def pytest_sessionfinish(session: pytest.Session) -> None:
    if _selected is not None and (
        _state != "errored" or (_created, _killed, _called, _errored) != (1, 1, 1, 1)
    ):
        session.exitstatus = pytest.ExitCode.TESTS_FAILED


def pytest_unconfigure() -> None:
    if _original_remote_server is not None:
        setattr(import_module("tests.conftest"), "RemoteServer", _original_remote_server)
