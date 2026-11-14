"""Installed-wheel remote Page close notices over a real synthetic WebSocket."""

from __future__ import annotations

import asyncio
import base64
import contextlib
import hashlib
import ipaddress
import json
import os
import socket
from typing import cast

import pytest
from pwright import _native
from pwright.async_api import Playwright, RemoteBrowserContext


def _tailscale_target() -> tuple[str, str]:
    if "playwright-server-close-notices" not in _native.selected_features():
        pytest.skip("the selected native remote close-notices profile is not installed")
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


async def _read_frame(reader: asyncio.StreamReader) -> dict[str, object]:
    header = await reader.readexactly(2)
    assert header[0] & 0x0F == 1
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
    return json.loads(payload)


def _bootstrap() -> list[dict[str, object]]:
    objects: list[tuple[str, str, dict[str, object]]] = [
        ("BrowserType", "chromium@1", {"name": "chromium", "executablePath": "/browser"}),
        ("BrowserType", "firefox@1", {"name": "firefox", "executablePath": "/browser"}),
        ("BrowserType", "webkit@1", {"name": "webkit", "executablePath": "/browser"}),
        ("Android", "android@1", {}),
        ("Electron", "electron@1", {}),
    ]
    messages: list[dict[str, object]] = [
        {
            "guid": "",
            "method": "__create__",
            "params": {"type": kind, "guid": guid, "initializer": initializer},
        }
        for kind, guid, initializer in objects
    ]
    messages.extend(
        [
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
    )
    return messages


@pytest.mark.asyncio
async def test_server_page_close_notifies_only_page_until_context_and_transport_close() -> None:
    # Arrange
    ip, host = _tailscale_target()
    send_page_close = asyncio.Event()
    send_page_dispose = asyncio.Event()
    send_context_close = asyncio.Event()
    send_context_dispose = asyncio.Event()
    drop_transport = asyncio.Event()
    server_errors: list[BaseException] = []

    async def handle(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            request = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            lines = request.decode("latin-1").split("\r\n")
            headers = dict(line.split(": ", 1) for line in lines[1:] if ": " in line)
            assert lines[0].startswith("GET /ws ")
            assert headers.get("x-playwright-browser") == "chromium"
            accept = base64.b64encode(
                hashlib.sha1(
                    (headers["Sec-WebSocket-Key"] + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()
                ).digest()
            )
            writer.write(
                b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
                b"Connection: Upgrade\r\nSec-WebSocket-Accept: " + accept + b"\r\n\r\n"
            )
            await writer.drain()

            initialize = await asyncio.wait_for(_read_frame(reader), 5)
            assert initialize["method"] == "initialize"
            for message in _bootstrap():
                writer.write(_frame(message))
            await writer.drain()

            new_context = await asyncio.wait_for(_read_frame(reader), 5)
            assert new_context["guid"] == "browser@1" and new_context["method"] == "newContext"
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

            new_page = await asyncio.wait_for(_read_frame(reader), 5)
            assert new_page["guid"] == "context@1" and new_page["method"] == "newPage"
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
                            "initializer": {
                                "mainFrame": {"guid": "frame@1"},
                                "isClosed": False,
                            },
                        },
                    }
                )
            )
            writer.write(
                _frame({"guid": "page@1", "method": "__adopt__", "params": {"guid": "frame@1"}})
            )
            writer.write(_frame({"id": new_page["id"], "result": {"page": {"guid": "page@1"}}}))
            await writer.drain()

            await asyncio.wait_for(send_page_close.wait(), 5)
            writer.write(_frame({"guid": "page@1", "method": "close", "params": {}}))
            await writer.drain()
            await asyncio.wait_for(send_page_dispose.wait(), 5)
            writer.write(_frame({"guid": "page@1", "method": "__dispose__", "params": {}}))
            await writer.drain()

            await asyncio.wait_for(send_context_close.wait(), 5)
            writer.write(_frame({"guid": "context@1", "method": "close", "params": {}}))
            await writer.drain()
            await asyncio.wait_for(send_context_dispose.wait(), 5)
            writer.write(_frame({"guid": "context@1", "method": "__dispose__", "params": {}}))
            await writer.drain()

            await asyncio.wait_for(drop_transport.wait(), 5)
            server.close()
            writer.transport.abort()
        except BaseException as error:
            server_errors.append(error)
        finally:
            writer.transport.abort()

    server = await asyncio.start_server(handle, ip, 0)
    port = server.sockets[0].getsockname()[1]
    playwright = Playwright()
    try:
        browser = await asyncio.wait_for(playwright.chromium.connect(f"ws://{host}:{port}/ws"), 5)
        context = cast(RemoteBrowserContext, await asyncio.wait_for(browser.new_context(), 5))
        page = await asyncio.wait_for(context.new_page(), 5)
        notices: list[tuple[str, object]] = []
        page_notified = asyncio.Event()
        context_notified = asyncio.Event()
        browser_notified = asyncio.Event()

        def on_page(value: object) -> None:
            notices.append(("page", value))
            page_notified.set()

        def on_context(value: object) -> None:
            notices.append(("context", value))
            context_notified.set()

        def on_browser(value: object) -> None:
            notices.append(("browser", value))
            browser_notified.set()

        page.on("close", on_page)
        context.on("close", on_context)
        browser.on("disconnected", on_browser)

        # Act: close each layer at a different wire position.
        send_page_close.set()
        await asyncio.wait_for(page_notified.wait(), 5)

        # Assert: Page-only close does not imply a Context or Browser close.
        assert notices == [("page", page)]
        assert page.is_closed()
        assert not context.is_closed()
        assert browser.is_connected()
        assert context.pages == []
        send_page_dispose.set()

        send_context_close.set()
        await asyncio.wait_for(context_notified.wait(), 5)
        assert notices == [("page", page), ("context", context)]
        assert context.is_closed()
        assert browser.is_connected()
        assert browser.contexts == []
        send_context_dispose.set()

        drop_transport.set()
        await asyncio.wait_for(browser_notified.wait(), 5)
        assert notices == [("page", page), ("context", context), ("browser", browser)]
        assert not browser.is_connected()
        assert not server_errors
    finally:
        send_page_close.set()
        send_page_dispose.set()
        send_context_close.set()
        send_context_dispose.set()
        drop_transport.set()
        with contextlib.suppress(TimeoutError):
            await asyncio.wait_for(playwright.stop(), 5)
        server.close()
        await asyncio.wait_for(server.wait_closed(), 5)
