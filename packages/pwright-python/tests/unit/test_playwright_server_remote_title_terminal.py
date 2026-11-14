"""A pending typed remote title call settles when its WebSocket server dies."""

from __future__ import annotations

import asyncio
import base64
import contextlib
import hashlib
import ipaddress
import json
import os
import socket

import pytest
from pwright import TargetClosedError, _native
from pwright.async_api import Playwright, RemoteBrowserContext, RemotePage


def _tailscale_target() -> tuple[str, str]:
    features = _native.selected_features()
    if not {"playwright-server-connect-session", "playwright-server-frame-title"}.issubset(
        features
    ):
        pytest.skip("the selected native remote Frame.title profile is not installed")
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


async def _read_frame(reader: asyncio.StreamReader) -> tuple[int, dict[str, object]]:
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
    return opcode, json.loads(payload)


def _bootstrap() -> list[dict[str, object]]:
    objects: list[tuple[str, str, dict[str, str]]] = [
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
async def test_pending_remote_page_title_settles_and_emits_ordered_terminal_events() -> None:
    # Arrange
    ip, host = _tailscale_target()
    title_received = asyncio.Event()
    drop_server = asyncio.Event()
    finished = asyncio.Event()
    server_errors: list[BaseException] = []
    commands: list[dict[str, object]] = []

    async def handle(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            request = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
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

            opcode, initialize = await asyncio.wait_for(_read_frame(reader), 5)
            assert opcode == 1 and initialize["method"] == "initialize"
            for message in _bootstrap():
                writer.write(_frame(message))
            await writer.drain()

            opcode, new_context = await asyncio.wait_for(_read_frame(reader), 5)
            assert opcode == 1 and new_context["method"] == "newContext"
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

            opcode, new_page = await asyncio.wait_for(_read_frame(reader), 5)
            assert opcode == 1 and new_page["method"] == "newPage"
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

            opcode, title = await asyncio.wait_for(_read_frame(reader), 5)
            assert opcode == 1
            assert title["guid"] == "frame@1"
            assert title["method"] == "title"
            assert title["params"] == {}
            metadata = title["metadata"]
            assert isinstance(metadata, dict) and metadata["apiName"] == "Page.title"
            commands.append(title)
            title_received.set()
            await asyncio.wait_for(drop_server.wait(), 5)
            server.close()
            writer.transport.abort()  # No title response or WebSocket close handshake.
        except BaseException as error:
            server_errors.append(error)
        finally:
            writer.transport.abort()
            finished.set()

    server = await asyncio.start_server(handle, ip, 0)
    port = server.sockets[0].getsockname()[1]
    playwright = Playwright()
    try:
        browser = await asyncio.wait_for(playwright.chromium.connect(f"ws://{host}:{port}/ws"), 5)
        context = await asyncio.wait_for(browser.new_context(), 5)
        page = await asyncio.wait_for(context.new_page(), 5)
        assert isinstance(context, RemoteBrowserContext)
        assert isinstance(page, RemotePage)
        events: list[tuple[str, object]] = []
        page.on("close", lambda value: events.append(("page", value)))
        context.on("close", lambda value: events.append(("context", value)))
        browser.on("disconnected", lambda value: events.append(("browser", value)))

        # Act
        title_task = asyncio.create_task(page.title())
        await asyncio.wait_for(title_received.wait(), 5)
        assert not title_task.done()
        drop_server.set()
        with pytest.raises(TargetClosedError, match="has been closed"):
            await asyncio.wait_for(title_task, 5)
        await asyncio.wait_for(finished.wait(), 5)

        # Assert
        assert len(commands) == 1
        assert not server_errors
        assert events == [("page", page), ("context", context), ("browser", browser)]
        assert page.is_closed()
        assert context.is_closed()
        assert not browser.is_connected()
    finally:
        drop_server.set()
        with contextlib.suppress(TimeoutError):
            await asyncio.wait_for(playwright.stop(), 5)
        server.close()
        await asyncio.wait_for(server.wait_closed(), 5)
