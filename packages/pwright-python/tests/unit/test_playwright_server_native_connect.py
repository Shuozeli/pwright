# pyright: reportPrivateUsage=false
"""Selected-profile native WebSocket Browser, Context, and Page lifecycle smoke."""

from __future__ import annotations

import asyncio
import base64
import contextlib
import hashlib
import ipaddress
import json
import os
import socket
from typing import Literal, cast

import pytest
from pwright import UnsupportedError, _native
from pwright.async_api import Playwright, RemoteBrowserContext


def _tailscale_target() -> tuple[str, str]:
    if "playwright-server-connect-session" not in _native.selected_features():
        pytest.skip("selected native Playwright Server connect profile is not installed")
    ip = os.environ.get("TAILSCALE_IP")
    host = os.environ.get("TAILSCALE_HOST")
    if not ip or not host:
        pytest.skip("TAILSCALE_IP and TAILSCALE_HOST are required for the network fixture")
    address = ipaddress.ip_address(ip)
    assert isinstance(address, ipaddress.IPv4Address)
    assert address in ipaddress.ip_network("100.64.0.0/10")
    assert host.endswith(".ts.net") and "/" not in host and ":" not in host
    resolved = {entry[4][0] for entry in socket.getaddrinfo(host, 80, type=socket.SOCK_STREAM)}
    assert ip in resolved
    return ip, host


def _frame(payload: dict[str, object]) -> bytes:
    encoded = json.dumps(payload, separators=(",", ":")).encode()
    assert len(encoded) < 65_536
    if len(encoded) < 126:
        return bytes((0x81, len(encoded))) + encoded
    return bytes((0x81, 126)) + len(encoded).to_bytes(2, "big") + encoded


async def _read_frame(reader: asyncio.StreamReader) -> tuple[int, bytes]:
    header = await reader.readexactly(2)
    opcode = header[0] & 0x0F
    length = header[1] & 0x7F
    if length == 126:
        length = int.from_bytes(await reader.readexactly(2), "big")
    elif length == 127:
        length = int.from_bytes(await reader.readexactly(8), "big")
    assert length <= 1024 * 1024
    mask = await reader.readexactly(4) if header[1] & 0x80 else None
    payload = await reader.readexactly(length)
    if mask is not None:
        payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    return opcode, payload


def _bootstrap() -> list[dict[str, object]]:
    return [
        {
            "guid": "",
            "method": "__create__",
            "params": {
                "type": "BrowserType",
                "guid": "chromium@1",
                "initializer": {"name": "chromium", "executablePath": "/browser"},
            },
        },
        {
            "guid": "",
            "method": "__create__",
            "params": {
                "type": "BrowserType",
                "guid": "firefox@1",
                "initializer": {"name": "firefox", "executablePath": "/browser"},
            },
        },
        {
            "guid": "",
            "method": "__create__",
            "params": {
                "type": "BrowserType",
                "guid": "webkit@1",
                "initializer": {"name": "webkit", "executablePath": "/browser"},
            },
        },
        {
            "guid": "",
            "method": "__create__",
            "params": {"type": "Android", "guid": "android@1", "initializer": {}},
        },
        {
            "guid": "",
            "method": "__create__",
            "params": {"type": "Electron", "guid": "electron@1", "initializer": {}},
        },
        {
            "guid": "chromium@1",
            "method": "__create__",
            "params": {
                "type": "Browser",
                "guid": "browser@1",
                "initializer": {"name": "chromium", "version": "155.0"},
            },
        },
        {
            "guid": "",
            "method": "__create__",
            "params": {
                "type": "Playwright",
                "guid": "playwright@1",
                "initializer": {
                    "chromium": {"guid": "chromium@1"},
                    "firefox": {"guid": "firefox@1"},
                    "webkit": {"guid": "webkit@1"},
                    "android": {"guid": "android@1"},
                    "electron": {"guid": "electron@1"},
                    "preLaunchedBrowser": {"guid": "browser@1"},
                },
            },
        },
        {"id": 1, "result": {"playwright": {"guid": "playwright@1"}}},
    ]


@pytest.mark.asyncio
async def test_native_remote_page_title_uses_adopted_main_frame() -> None:
    # Arrange
    if "playwright-server-frame-title" not in _native.selected_features():
        pytest.skip("selected native Frame.title profile is not installed")
    ip, host = _tailscale_target()
    observed: list[tuple[str, object]] = []
    failure: list[BaseException] = []
    finished = asyncio.Event()

    async def handle(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            request = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), timeout=5)
            lines = request.decode("latin-1").split("\r\n")
            headers = dict(line.split(": ", 1) for line in lines[1:] if ": " in line)
            assert lines[0].startswith("GET /ws ")
            assert headers.get("x-playwright-browser") == "chromium"
            key = headers["Sec-WebSocket-Key"]
            accept = base64.b64encode(
                hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()
            )
            writer.write(
                b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
                b"Connection: Upgrade\r\nSec-WebSocket-Accept: " + accept + b"\r\n\r\n"
            )
            await writer.drain()
            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1 and json.loads(payload)["method"] == "initialize"
            for item in _bootstrap():
                writer.write(_frame(item))
            await writer.drain()

            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            new_context = json.loads(payload)
            assert opcode == 1 and new_context["method"] == "newContext"
            observed.append(("newContext", new_context["guid"]))
            writer.write(
                _frame(
                    {
                        "guid": "browser@1",
                        "method": "__create__",
                        "params": {
                            "type": "BrowserContext",
                            "guid": "context@1",
                            "initializer": {},
                        },
                    }
                )
            )
            writer.write(
                _frame({"id": new_context["id"], "result": {"context": {"guid": "context@1"}}})
            )
            await writer.drain()

            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            new_page = json.loads(payload)
            assert opcode == 1 and new_page["method"] == "newPage"
            observed.append(("newPage", new_page["guid"]))
            writer.write(
                _frame(
                    {
                        "guid": "context@1",
                        "method": "__create__",
                        "params": {
                            "type": "Frame",
                            "guid": "frame@1",
                            "initializer": {"name": "", "url": "about:blank", "loadStates": []},
                        },
                    }
                )
            )
            writer.write(
                _frame(
                    {
                        "guid": "context@1",
                        "method": "__create__",
                        "params": {
                            "type": "Page",
                            "guid": "page@1",
                            "initializer": {"mainFrame": {"guid": "frame@1"}, "isClosed": False},
                        },
                    }
                )
            )
            writer.write(
                _frame({"guid": "page@1", "method": "__adopt__", "params": {"guid": "frame@1"}})
            )
            writer.write(_frame({"id": new_page["id"], "result": {"page": {"guid": "page@1"}}}))
            await writer.drain()

            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            title_command = json.loads(payload)
            assert opcode == 1
            assert title_command["guid"] == "frame@1"
            assert title_command["method"] == "title"
            assert title_command["params"] == {}
            assert title_command["metadata"]["apiName"] == "Page.title"
            observed.append(("title", title_command["guid"]))
            writer.write(_frame({"id": title_command["id"], "result": {"value": "Remote title"}}))
            await writer.drain()
            opcode, _payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 8
            writer.write(b"\x88\x00")
            await writer.drain()
        except BaseException as error:
            failure.append(error)
        finally:
            finished.set()
            writer.close()
            with contextlib.suppress(ConnectionError):
                await writer.wait_closed()

    server = await asyncio.start_server(handle, ip, 0)
    port = server.sockets[0].getsockname()[1]
    playwright = Playwright()
    try:
        # Act
        browser = await asyncio.wait_for(playwright.chromium.connect(f"ws://{host}:{port}/ws"), 5)
        context = await asyncio.wait_for(browser.new_context(), 5)
        page = await asyncio.wait_for(context.new_page(), 5)
        title = await asyncio.wait_for(page.title(), 5)
        await asyncio.wait_for(browser.close(), 5)
        await asyncio.wait_for(finished.wait(), 5)

        # Assert
        assert title == "Remote title"
        assert observed == [
            ("newContext", "browser@1"),
            ("newPage", "context@1"),
            ("title", "frame@1"),
        ]
        assert not failure
        assert not browser.is_connected()
    finally:
        await playwright.stop()
        server.close()
        await server.wait_closed()


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("close_mode", "event_burst"),
    (("browser", False), ("browser", True), ("owner", False), ("server", False)),
)
async def test_native_http_discovery_websocket_browser_lifecycle(
    close_mode: Literal["browser", "owner", "server"],
    event_burst: bool,
) -> None:
    # Arrange
    ip, host = _tailscale_target()
    observed: list[tuple[str, object]] = []
    finished = asyncio.Event()
    trigger_remote_close = asyncio.Event()
    trigger_external_dispose = asyncio.Event()
    failure: list[BaseException] = []

    async def handle(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            request = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), timeout=5)
            lines = request.decode("latin-1").split("\r\n")
            headers = dict(line.split(": ", 1) for line in lines[1:] if ": " in line)
            if lines[0].startswith("GET /prefix/json?token=synthetic "):
                observed.append(("discovery", lines[0]))
                body = b'{"wsEndpointPath":"/browser-123"}'
                writer.write(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: "
                    + str(len(body)).encode()
                    + b"\r\nConnection: close\r\n\r\n"
                    + body
                )
                await writer.drain()
                return
            assert lines[0].startswith("GET /prefix/browser-123?token=synthetic ")
            assert headers.get("x-playwright-browser") == "chromium"
            key = headers["Sec-WebSocket-Key"]
            accept = base64.b64encode(
                hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()
            )
            writer.write(
                b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
                b"Connection: Upgrade\r\nSec-WebSocket-Accept: " + accept + b"\r\n\r\n"
            )
            await writer.drain()
            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1
            initialize = json.loads(payload)
            assert initialize["method"] == "initialize"
            assert initialize["params"] == {"sdkLanguage": "python"}
            observed.append(("initialize", initialize))
            bootstrap = _bootstrap()
            if close_mode == "browser":
                bootstrap.insert(
                    6,
                    {
                        "guid": "browser@1",
                        "method": "__create__",
                        "params": {
                            "type": "BrowserContext",
                            "guid": "external-context@1",
                            "initializer": {},
                        },
                    },
                )
            for item in bootstrap:
                writer.write(_frame(item))
            await writer.drain()
            if close_mode == "browser":
                await asyncio.wait_for(trigger_external_dispose.wait(), timeout=5)
                writer.write(
                    _frame({"guid": "external-context@1", "method": "__dispose__", "params": {}})
                )
                if event_burst:
                    for index in range(160):
                        writer.write(
                            _frame(
                                {
                                    "guid": "browser@1",
                                    "method": "syntheticNoise",
                                    "params": {"index": index},
                                }
                            )
                        )
                await writer.drain()
            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1
            new_context = json.loads(payload)
            assert new_context["id"] == 2
            assert new_context["guid"] == "browser@1"
            assert new_context["method"] == "newContext"
            assert new_context["params"] == {}
            observed.append(("newContext", new_context))
            writer.write(
                _frame(
                    {
                        "guid": "browser@1",
                        "method": "__create__",
                        "params": {
                            "type": "BrowserContext",
                            "guid": "context@1",
                            "initializer": {},
                        },
                    }
                )
            )
            writer.write(_frame({"id": 2, "result": {"context": {"guid": "context@1"}}}))
            await writer.drain()
            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1
            new_page = json.loads(payload)
            assert new_page["id"] == 3
            assert new_page["guid"] == "context@1"
            assert new_page["method"] == "newPage"
            assert new_page["params"] == {}
            observed.append(("newPage", new_page))
            if close_mode == "browser":
                writer.write(
                    _frame(
                        {
                            "guid": "context@1",
                            "method": "__create__",
                            "params": {
                                "type": "Page",
                                "guid": "external-page@1",
                                "initializer": {},
                            },
                        }
                    )
                )
            writer.write(
                _frame(
                    {
                        "guid": "context@1",
                        "method": "__create__",
                        "params": {
                            "type": "Page",
                            "guid": "page@1",
                            "initializer": {},
                        },
                    }
                )
            )
            writer.write(_frame({"id": 3, "result": {"page": {"guid": "page@1"}}}))
            await writer.drain()
            if close_mode == "server":
                await asyncio.wait_for(trigger_remote_close.wait(), timeout=5)
                opcode = 8
                writer.write(b"\x88\x02\x03\xe8")
            else:
                opcode, _ = await asyncio.wait_for(_read_frame(reader), timeout=5)
                assert opcode == 8
                writer.write(b"\x88\x00")
            await writer.drain()
            observed.append(("close", opcode))
            finished.set()
        except BaseException as error:
            failure.append(error)
            finished.set()
        finally:
            writer.close()
            with contextlib.suppress(ConnectionError):
                await writer.wait_closed()

    server = await asyncio.start_server(handle, ip, 0)
    port = server.sockets[0].getsockname()[1]
    playwright = Playwright()
    try:
        # Act
        browser = await asyncio.wait_for(
            playwright.chromium.connect(f"http://{host}:{port}/prefix?token=synthetic"),
            timeout=5,
        )
        assert browser.version == "155.0"
        assert browser.is_connected()
        if close_mode == "browser":
            with pytest.raises(
                UnsupportedError, match="remote Context without an installed wrapper"
            ):
                _ = browser.contexts
            trigger_external_dispose.set()
        context = await asyncio.wait_for(browser.new_context(), timeout=5)
        page = await asyncio.wait_for(context.new_page(), timeout=5)
        assert browser.contexts == [context]
        if close_mode == "browser":
            with pytest.raises(UnsupportedError, match="remote Page without an installed wrapper"):
                _ = context.pages
        else:
            assert context.pages == [page]
        assert page.context is context
        assert not page.is_closed()
        with pytest.raises(UnsupportedError, match="remote operation"):
            await page.goto("https://example.test")
        notices: list[object] = []
        disconnected = asyncio.Event()

        def on_disconnect(value: object) -> None:
            notices.append(value)
            disconnected.set()

        browser.on("disconnected", on_disconnect)
        if close_mode == "owner":
            await asyncio.wait_for(playwright.stop(), timeout=5)
            report = playwright._environment.cleanup_report()
            assert report["remote_sessions_closed"] == 1
            assert report["sessions_zero"] is True
            assert report["ffi_zero"] is True
        elif close_mode == "browser":
            await asyncio.wait_for(browser.close(), timeout=5)
        else:
            trigger_remote_close.set()
            await asyncio.wait_for(disconnected.wait(), timeout=5)
            await asyncio.wait_for(playwright.stop(), timeout=5)
            report = playwright._environment.cleanup_report()
            assert report["remote_sessions_closed"] == 0
            assert report["sessions_zero"] is True
        await asyncio.wait_for(finished.wait(), timeout=5)

        # Assert
        assert not failure
        assert [kind for kind, _ in observed] == [
            "discovery",
            "initialize",
            "newContext",
            "newPage",
            "close",
        ]
        assert not browser.is_connected()
        assert page.is_closed()
        assert notices == [browser]
    finally:
        await playwright.stop()
        server.close()
        await server.wait_closed()


@pytest.mark.asyncio
@pytest.mark.parametrize("close_kind", ("context", "implicit_page"))
async def test_native_websocket_context_and_implicit_page_close(
    close_kind: Literal["context", "implicit_page"],
) -> None:
    # Arrange
    ip, host = _tailscale_target()
    observed_commands: list[str] = []
    failure: list[BaseException] = []
    finished = asyncio.Event()

    async def handle(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            request = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), timeout=5)
            lines = request.decode("latin-1").split("\r\n")
            headers = dict(line.split(": ", 1) for line in lines[1:] if ": " in line)
            assert lines[0].startswith("GET /ws ")
            assert headers.get("x-playwright-browser") == "chromium"
            key = headers["Sec-WebSocket-Key"]
            accept = base64.b64encode(
                hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()
            )
            writer.write(
                b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
                b"Connection: Upgrade\r\nSec-WebSocket-Accept: " + accept + b"\r\n\r\n"
            )
            await writer.drain()

            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1
            initialize = json.loads(payload)
            assert initialize["id"] == 1
            assert initialize["method"] == "initialize"
            assert initialize["params"] == {"sdkLanguage": "python"}
            observed_commands.append("initialize")
            for item in _bootstrap():
                writer.write(_frame(item))
            await writer.drain()

            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1
            new_context = json.loads(payload)
            assert new_context["id"] == 2
            assert new_context["guid"] == "browser@1"
            assert new_context["method"] == "newContext"
            assert new_context["params"] == {}
            observed_commands.append("newContext")
            writer.write(
                _frame(
                    {
                        "guid": "browser@1",
                        "method": "__create__",
                        "params": {
                            "type": "BrowserContext",
                            "guid": "context@1",
                            "initializer": {},
                        },
                    }
                )
            )
            writer.write(_frame({"id": 2, "result": {"context": {"guid": "context@1"}}}))
            await writer.drain()

            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1
            new_page = json.loads(payload)
            assert new_page["id"] == 3
            assert new_page["guid"] == "context@1"
            assert new_page["method"] == "newPage"
            assert new_page["params"] == {}
            observed_commands.append("newPage")
            writer.write(
                _frame(
                    {
                        "guid": "context@1",
                        "method": "__create__",
                        "params": {"type": "Page", "guid": "page@1", "initializer": {}},
                    }
                )
            )
            writer.write(_frame({"id": 3, "result": {"page": {"guid": "page@1"}}}))
            await writer.drain()

            opcode, payload = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 1
            close_context = json.loads(payload)
            assert close_context["id"] == 4
            assert close_context["guid"] == "context@1"
            assert close_context["method"] == "close"
            assert close_context["params"] == {}
            assert close_context["metadata"]["apiName"] == "BrowserContext.close"
            observed_commands.append("close")
            writer.write(_frame({"guid": "page@1", "method": "__dispose__", "params": {}}))
            writer.write(_frame({"guid": "context@1", "method": "__dispose__", "params": {}}))
            writer.write(_frame({"id": 4, "result": {}}))
            await writer.drain()

            opcode, _ = await asyncio.wait_for(_read_frame(reader), timeout=5)
            assert opcode == 8
            observed_commands.append("socketClose")
            writer.write(b"\x88\x00")
            await writer.drain()
        except BaseException as error:
            failure.append(error)
        finally:
            finished.set()
            writer.close()
            with contextlib.suppress(ConnectionError):
                await writer.wait_closed()

    server = await asyncio.start_server(handle, ip, 0)
    port = server.sockets[0].getsockname()[1]
    playwright = Playwright()
    try:
        browser = await asyncio.wait_for(
            playwright.chromium.connect(f"ws://{host}:{port}/ws"), timeout=5
        )
        if close_kind == "implicit_page":
            page = await asyncio.wait_for(browser.new_page(), timeout=5)
            context = page.context
        else:
            context = await asyncio.wait_for(browser.new_context(), timeout=5)
            page = await asyncio.wait_for(context.new_page(), timeout=5)
        events: list[tuple[str, object]] = []
        page.on("close", lambda value: events.append(("page", value)))
        context.on("close", lambda value: events.append(("context", value)))
        browser.on("disconnected", lambda value: events.append(("browser", value)))
        assert browser.contexts == [context]
        assert context.pages == [page]

        # Act
        if close_kind == "implicit_page":
            await asyncio.wait_for(page.close(), timeout=5)
        else:
            await asyncio.wait_for(context.close(), timeout=5)

        # Assert
        assert events == [("page", page), ("context", context)]
        assert page.is_closed()
        assert cast(RemoteBrowserContext, context).is_closed()
        assert context.pages == []
        assert browser.contexts == []
        assert browser.is_connected()
        assert observed_commands == ["initialize", "newContext", "newPage", "close"]
        await browser.close()
        await asyncio.wait_for(finished.wait(), timeout=5)
        assert not failure
        assert observed_commands == [
            "initialize",
            "newContext",
            "newPage",
            "close",
            "socketClose",
        ]
        assert events == [("page", page), ("context", context), ("browser", browser)]
    finally:
        await playwright.stop()
        server.close()
        await server.wait_closed()
