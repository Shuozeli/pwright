# pyright: reportPrivateUsage=false
"""Browserless public facade gates for Playwright Server connect observation."""

from __future__ import annotations

import asyncio
import math
import threading
from collections.abc import Awaitable, Callable
from typing import cast

import pytest
from pwright import Error, ResourceExhaustedError, TargetClosedError, UnsupportedError
from pwright.async_api import BrowserType as AsyncBrowserType
from pwright.async_api import Playwright as AsyncPlaywright
from pwright.async_api import RemoteBrowserContext
from pwright.sync_api import BrowserType as SyncBrowserType
from pwright.sync_api import _LoopThread


class _Environment:
    def __init__(self, error: Exception | None = None) -> None:
        self.calls: list[tuple[str, int]] = []
        self.error = error

    async def observe_playwright_server_initialize(
        self, endpoint_url: str, timeout_ms: int
    ) -> None:
        self.calls.append((endpoint_url, timeout_ms))
        if self.error is not None:
            raise self.error

    async def connect_over_cdp(self, _endpoint_url: str, _timeout_ms: int) -> None:
        raise AssertionError("Playwright Server connect must not use CDP")


class _RemoteNative:
    def __init__(self) -> None:
        self.version = "test-browser-version"
        self.connected = True
        self.close_count = 0
        self.disconnected = asyncio.Event()
        self.contexts: list[_RemoteContextNative] = []
        self.next_sequence = 1

    def is_connected(self) -> bool:
        return self.connected

    def live_contexts(self) -> list[tuple[str, int]]:
        if not self.is_connected():
            return []
        return [
            (context.guid, context.created_sequence) for context in self.contexts if context.open
        ]

    async def wait_disconnected(self) -> None:
        await self.disconnected.wait()

    async def close(self) -> None:
        self.close_count += 1
        self.connected = False
        self.disconnected.set()

    async def new_context(self) -> _RemoteContextNative:
        context = _RemoteContextNative(self, f"context@{len(self.contexts) + 1}")
        self.contexts.append(context)
        return context


class _RemoteContextNative:
    def __init__(self, browser: _RemoteNative, guid: str) -> None:
        self.browser = browser
        self.guid = guid
        self.created_sequence = browser.next_sequence
        browser.next_sequence += 1
        self.open = True
        self.pages: list[_RemotePageNative] = []
        self.close_count = 0
        self.close_error: Exception | None = None
        self.new_page_error: Exception | None = None

    def is_open(self) -> bool:
        return self.open and self.browser.is_connected() and self in self.browser.contexts

    def live_pages(self) -> list[tuple[str, int]]:
        if not self.is_open():
            return []
        return [(page.guid, page.created_sequence) for page in self.pages if page.open]

    async def new_page(self) -> _RemotePageNative:
        assert self.is_open()
        if self.new_page_error is not None:
            raise self.new_page_error
        page = _RemotePageNative(self, f"page@{len(self.pages) + 1}")
        self.pages.append(page)
        return page

    async def close(self) -> None:
        self.close_count += 1
        if self.close_error is not None:
            raise self.close_error
        self.open = False
        for page in self.pages:
            page.open = False


class _RemotePageNative:
    def __init__(self, context: _RemoteContextNative, guid: str) -> None:
        self.context = context
        self.guid = guid
        self.created_sequence = context.browser.next_sequence
        context.browser.next_sequence += 1
        self.open = True

    def is_open(self) -> bool:
        return self.open and self.context.is_open() and self in self.context.pages


class _RemoteEnvironment(_Environment):
    def __init__(self) -> None:
        super().__init__()
        self.remote_calls: list[tuple[str, str, int]] = []
        self.remote_native: _RemoteNative | None = None

    async def connect_playwright_server(
        self, endpoint_url: str, browser_name: str, timeout_ms: int
    ) -> _RemoteNative:
        self.remote_calls.append((endpoint_url, browser_name, timeout_ms))
        native = _RemoteNative()
        self.remote_native = native
        return native

    async def disconnect(self) -> None:
        native = self.remote_native
        assert native is not None
        native.connected = False
        native.disconnected.set()


class _Owner:
    def __init__(self, environment: _Environment) -> None:
        self._environment = environment
        self._browsers: list[object] = []


class _Bridge:
    def run(self, operation: Awaitable[object]) -> object:
        async def await_operation() -> object:
            return await operation

        return asyncio.run(await_operation())


def _browser_type(
    environment: _Environment, engine: str = "chromium"
) -> tuple[AsyncBrowserType, _Owner]:
    owner = _Owner(environment)
    return AsyncBrowserType(cast(AsyncPlaywright, owner), engine), owner


@pytest.mark.asyncio
@pytest.mark.parametrize("engine", ("chromium", "firefox", "webkit"))
async def test_connect_default_off_rejects_all_engines(
    monkeypatch: pytest.MonkeyPatch, engine: str
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: cast(list[str], []))
    environment = _Environment()
    browser_type, owner = _browser_type(environment, engine)

    # Act
    with pytest.raises(UnsupportedError, match="playwright-server-initialize profile"):
        await browser_type.connect("ws://server.test/ws")

    # Assert
    assert environment.calls == []
    assert owner._browsers == []


@pytest.mark.asyncio
@pytest.mark.parametrize("engine", ("chromium", "firefox", "webkit"))
async def test_connect_observes_close_reason_without_creating_browser(
    monkeypatch: pytest.MonkeyPatch, engine: str
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment(Error("Oh my!"))
    browser_type, owner = _browser_type(environment, engine)

    # Act
    with pytest.raises(Error, match="Oh my!"):
        await browser_type.connect("wss://server.test/ws?token=abc", timeout=1250)

    # Assert
    assert environment.calls == [("wss://server.test/ws?token=abc", 1250)]
    assert owner._browsers == []


@pytest.mark.asyncio
async def test_connect_does_not_fabricate_browser_after_native_returns(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment()
    browser_type, owner = _browser_type(environment)

    # Act
    with pytest.raises(UnsupportedError, match="remote object graph is not installed"):
        await browser_type.connect("ws://server.test/ws")

    # Assert
    assert environment.calls == [("ws://server.test/ws", 30_000)]
    assert owner._browsers == []


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "url",
    (
        "http://server.test/ws",
        "ws://",
        "ws://server.test:invalid/ws",
        "ws://[broken/ws",
        "ws://server.test/ws#fragment",
        "ws://server.test/white space",
        "ws://user:password@server.test/ws",
    ),
)
async def test_connect_rejects_invalid_playwright_server_urls_before_native(
    monkeypatch: pytest.MonkeyPatch, url: str
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment()
    browser_type, owner = _browser_type(environment)

    # Act
    with pytest.raises(ValueError, match="valid ws:// or wss:// endpoint"):
        await browser_type.connect(url)

    # Assert
    assert environment.calls == []
    assert owner._browsers == []


@pytest.mark.asyncio
async def test_connect_rejects_oversized_endpoint_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment()
    browser_type, owner = _browser_type(environment)

    # Act
    with pytest.raises(ResourceExhaustedError, match="8192 UTF-8 bytes"):
        await browser_type.connect("ws://server.test/" + "x" * 8_193)

    # Assert
    assert environment.calls == []
    assert owner._browsers == []


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("timeout", "error_type"),
    (
        (True, TypeError),
        ("1000", TypeError),
        (-1, ValueError),
        (math.inf, ValueError),
        (math.nan, ValueError),
        (4_294_967_296, ValueError),
    ),
)
async def test_connect_rejects_invalid_timeouts_before_native(
    monkeypatch: pytest.MonkeyPatch,
    timeout: object,
    error_type: type[Exception],
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment()
    browser_type, owner = _browser_type(environment)

    # Act
    with pytest.raises(error_type, match="timeout"):
        await browser_type.connect("ws://server.test/ws", timeout=cast(float, timeout))

    # Assert
    assert environment.calls == []
    assert owner._browsers == []


@pytest.mark.asyncio
@pytest.mark.parametrize("option", ("slow_mo", "headers", "expose_network"))
async def test_connect_rejects_uninstalled_options_before_native(
    monkeypatch: pytest.MonkeyPatch, option: str
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment()
    browser_type, owner = _browser_type(environment)
    options: dict[str, object] = {
        "slow_mo": 0,
        "headers": {},
        "expose_network": "",
    }

    # Act
    with pytest.raises(UnsupportedError, match=f"option '{option}' is not installed"):
        connect = cast(Callable[..., Awaitable[object]], browser_type.connect)
        await connect("ws://server.test/ws", **{option: options[option]})

    # Assert
    assert environment.calls == []
    assert owner._browsers == []


@pytest.mark.asyncio
async def test_connect_rejects_unknown_keyword_without_native_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment()
    browser_type, owner = _browser_type(environment)

    # Act
    with pytest.raises(TypeError, match="unexpected keyword"):
        connect = cast(Callable[..., Awaitable[object]], browser_type.connect)
        await connect("ws://server.test/ws", unsupported=True)

    # Assert
    assert environment.calls == []
    assert owner._browsers == []


def test_sync_connect_forwards_playwright_server_and_propagates_close_reason(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment(Error("Oh my!"))
    async_type, owner = _browser_type(environment, "webkit")
    browser_type = SyncBrowserType(cast(_LoopThread, _Bridge()), async_type)

    # Act
    with pytest.raises(Error, match="Oh my!"):
        browser_type.connect("ws://server.test/ws", timeout=0)

    # Assert
    assert environment.calls == [("ws://server.test/ws", 0)]
    assert owner._browsers == []


def test_sync_connect_never_returns_fake_browser_after_native_returns(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-initialize"],
    )
    environment = _Environment()
    async_type, owner = _browser_type(environment)
    browser_type = SyncBrowserType(cast(_LoopThread, _Bridge()), async_type)

    # Act
    with pytest.raises(UnsupportedError, match="remote object graph is not installed"):
        browser_type.connect("ws://server.test/ws")

    # Assert
    assert environment.calls == [("ws://server.test/ws", 30_000)]
    assert owner._browsers == []


@pytest.mark.asyncio
@pytest.mark.parametrize("engine", ("chromium", "firefox", "webkit"))
async def test_remote_connect_returns_real_browser_lifecycle(
    monkeypatch: pytest.MonkeyPatch, engine: str
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, owner = _browser_type(environment, engine)
    events: list[object] = []

    # Act
    browser = await browser_type.connect("https://server.test/json", timeout=2500)
    browser.on("disconnected", events.append)
    browser.once("disconnected", events.append)
    assert browser.version == "test-browser-version"
    assert browser.is_connected()
    assert browser.contexts == []
    context = await browser.new_context()
    page = await context.new_page()
    assert context.browser is browser
    assert browser.contexts == [context]
    assert context.pages == [page]
    assert page.context is context
    assert not page.is_closed()
    with pytest.raises(UnsupportedError, match="remote options"):
        await browser.new_page(base_url="https://example.test")
    await browser.close()
    await browser.close()

    # Assert
    assert environment.remote_calls == [("https://server.test/json", engine, 2500)]
    assert environment.calls == []
    assert not browser.is_connected()
    assert browser.contexts == []
    assert page.is_closed()
    assert events == [browser, browser]
    assert environment.remote_native is not None
    assert environment.remote_native.close_count == 1
    assert owner._browsers == []


@pytest.mark.asyncio
async def test_remote_browser_new_page_owns_implicit_context_until_page_close(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    events: list[str] = []

    # Act
    page = await browser.new_page()
    context = cast(RemoteBrowserContext, page.context)
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))
    assert browser.contexts == [context]
    await page.close()
    await page.close()

    # Assert
    assert page.is_closed()
    assert context.is_closed()
    assert browser.is_connected()
    assert browser.contexts == []
    assert events == ["page", "context"]
    assert environment.remote_native is not None
    assert environment.remote_native.contexts[0].close_count == 1
    await browser.close()


@pytest.mark.asyncio
async def test_remote_browser_new_page_failure_cleans_implicit_context(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    native = environment.remote_native
    assert native is not None
    original_new_context = native.new_context

    async def failing_context() -> _RemoteContextNative:
        context = await original_new_context()
        context.new_page_error = Error("remote Page creation failed")
        return context

    monkeypatch.setattr(native, "new_context", failing_context)

    # Act
    with pytest.raises(Error, match="remote Page creation failed"):
        await browser.new_page()

    # Assert
    assert len(native.contexts) == 1
    assert native.contexts[0].close_count == 1
    assert browser.contexts == []
    assert browser.is_connected()
    await browser.close()


@pytest.mark.asyncio
async def test_remote_page_operation_after_browser_close_reports_target_closed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    page = await browser.new_page()

    # Act
    await browser.close()

    # Assert
    assert page.is_closed()
    with pytest.raises(Error, match="has been closed"):
        await page.goto("https://example.test")


@pytest.mark.asyncio
async def test_remote_disconnect_emits_once_and_retires_browser(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    event = asyncio.Event()
    notices: list[object] = []

    def on_disconnect(value: object) -> None:
        notices.append(value)
        event.set()

    browser.on("disconnected", on_disconnect)

    # Act
    await environment.disconnect()
    await asyncio.wait_for(event.wait(), timeout=1)
    await browser.close()

    # Assert
    assert not browser.is_connected()
    assert notices == [browser]
    assert owner._browsers == []
    assert environment.remote_native is not None
    assert environment.remote_native.close_count == 0


@pytest.mark.asyncio
async def test_remote_context_page_disposal_and_unsupported_methods(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    context = await browser.new_context()
    page = await context.new_page()
    native = environment.remote_native
    assert native is not None

    # Act / Assert
    with pytest.raises(UnsupportedError, match="remote options"):
        await browser.new_context(base_url="https://example.test")
    with pytest.raises(UnsupportedError, match="reason"):
        await context.close(reason="not installed")
    with pytest.raises(UnsupportedError, match="Context-owned Page"):
        await page.close()
    with pytest.raises(UnsupportedError, match="remote operation"):
        await page.goto("https://example.test")
    with pytest.raises(UnsupportedError, match="remote operation"):
        await page.evaluate("1 + 1")
    with pytest.raises(UnsupportedError, match="remote operation"):
        _ = page.url
    native.contexts[0].pages[0].open = False
    assert page.is_closed()
    assert context.pages == []
    native.contexts[0].open = False
    assert browser.contexts == []
    with pytest.raises(TargetClosedError):
        await context.new_page()
    await browser.close()


@pytest.mark.asyncio
async def test_remote_context_close_retires_pages_before_context_without_disconnect(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    context = await browser.new_context()
    page = await context.new_page()
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    # Act
    await context.close()
    await context.close()

    # Assert
    assert events == ["page", "context"]
    assert page.is_closed() and cast(RemoteBrowserContext, context).is_closed()
    assert context.pages == []
    assert browser.contexts == []
    assert browser.is_connected()
    assert environment.remote_native is not None
    assert environment.remote_native.contexts[0].close_count == 1
    await browser.close()
    assert events == ["page", "context", "browser"]


@pytest.mark.asyncio
async def test_remote_context_close_refusal_keeps_live_children(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    context = await browser.new_context()
    page = await context.new_page()
    native = environment.remote_native
    assert native is not None
    native.contexts[0].close_error = RuntimeError("synthetic refusal")
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))

    # Act
    with pytest.raises(RuntimeError, match="synthetic refusal"):
        await context.close()

    # Assert
    assert events == []
    assert browser.contexts == [context]
    assert context.pages == [page]
    assert browser.is_connected()
    await browser.close()


@pytest.mark.asyncio
async def test_remote_inventories_reject_external_live_objects(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    native = environment.remote_native
    assert native is not None
    external_context = _RemoteContextNative(native, "external-context")
    native.contexts.append(external_context)

    # Act / Assert
    with pytest.raises(UnsupportedError, match="remote Context without an installed wrapper"):
        _ = browser.contexts
    native.contexts.remove(external_context)
    context = await browser.new_context()
    external_page = _RemotePageNative(native.contexts[0], "external-page")
    native.contexts[0].pages.append(external_page)
    with pytest.raises(UnsupportedError, match="remote Page without an installed wrapper"):
        _ = context.pages
    native.contexts[0].pages.remove(external_page)
    assert browser.contexts == [context]
    assert context.pages == []
    await browser.close()


@pytest.mark.asyncio
async def test_remote_inventories_reject_recycled_guid_generations(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, _owner = _browser_type(environment)
    browser = await browser_type.connect("wss://server.test/ws")
    context = await browser.new_context()
    page = await context.new_page()
    native = environment.remote_native
    assert native is not None
    old_context = native.contexts[0]
    old_page = old_context.pages[0]
    recycled_page = _RemotePageNative(old_context, old_page.guid)
    old_context.pages[0] = recycled_page

    # Act / Assert
    assert page.is_closed()
    with pytest.raises(UnsupportedError, match="remote Page without an installed wrapper"):
        _ = context.pages
    assert browser.contexts == [context]
    recycled_context = _RemoteContextNative(native, old_context.guid)
    native.contexts[0] = recycled_context
    assert not old_context.is_open()
    with pytest.raises(UnsupportedError, match="remote Context without an installed wrapper"):
        _ = browser.contexts
    await browser.close()


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "url",
    (
        "ftp://server.test/ws",
        "http://user:secret@server.test/json",
        "https://server.test/json#fragment",
        "ws://server.test/white space",
    ),
)
async def test_remote_profile_rejects_invalid_endpoint_before_native(
    monkeypatch: pytest.MonkeyPatch, url: str
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, owner = _browser_type(environment)

    # Act
    with pytest.raises(ValueError, match=r"valid ws\(s\) or http\(s\) endpoint"):
        await browser_type.connect(url)

    # Assert
    assert environment.remote_calls == []
    assert owner._browsers == []


@pytest.mark.asyncio
@pytest.mark.parametrize("timeout", (0, 120_001))
async def test_remote_profile_rejects_unbounded_timeout_before_native(
    monkeypatch: pytest.MonkeyPatch, timeout: int
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    browser_type, owner = _browser_type(environment)

    # Act
    with pytest.raises(ValueError, match="between 1 and 120000"):
        await browser_type.connect("wss://server.test/ws", timeout=timeout)

    # Assert
    assert environment.remote_calls == []
    assert owner._browsers == []


def test_sync_remote_connect_close_and_disconnected_event(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    async_type, owner = _browser_type(environment, "firefox")
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, async_type)
    event = threading.Event()
    notices: list[object] = []

    try:
        # Act
        browser = browser_type.connect("http://server.test/json")
        browser.on("disconnected", lambda value: (notices.append(value), event.set()))
        assert browser.version == "test-browser-version"
        assert browser.is_connected()
        context = browser.new_context()
        page = context.new_page()
        assert browser.contexts == [context]
        assert context.pages == [page]
        assert page.context is context
        assert not page.is_closed()
        with pytest.raises(UnsupportedError, match="Context-owned Page"):
            page.close()
        browser.close()

        # Assert
        assert event.wait(timeout=1)
        assert notices == [browser]
        assert environment.remote_calls == [("http://server.test/json", "firefox", 30_000)]
        assert owner._browsers == []
        assert page.is_closed()
        assert environment.remote_native is not None
        assert environment.remote_native.close_count == 1
    finally:
        bridge.close()


def test_sync_remote_browser_new_page_closes_implicit_context(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    async_type, _owner = _browser_type(environment)
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, async_type)

    try:
        browser = browser_type.connect("wss://server.test/ws")

        # Act
        page = browser.new_page()
        context = page.context
        page.close()
        page.close()

        # Assert
        assert page.is_closed()
        assert browser.contexts == []
        assert browser.is_connected()
        assert environment.remote_native is not None
        assert environment.remote_native.contexts[0].close_count == 1
        assert context.pages == []
        browser.close()
    finally:
        bridge.close()


def test_sync_remote_inventories_reject_external_live_objects(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["playwright-server-connect-session"],
    )
    environment = _RemoteEnvironment()
    async_type, _owner = _browser_type(environment)
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, async_type)

    try:
        browser = browser_type.connect("wss://server.test/ws")
        native = environment.remote_native
        assert native is not None
        external_context = _RemoteContextNative(native, "external-context")
        native.contexts.append(external_context)

        # Act / Assert
        with pytest.raises(UnsupportedError, match="remote Context without an installed wrapper"):
            _ = browser.contexts
        native.contexts.remove(external_context)
        context = browser.new_context()
        external_page = _RemotePageNative(native.contexts[0], "external-page")
        native.contexts[0].pages.append(external_page)
        with pytest.raises(UnsupportedError, match="remote Page without an installed wrapper"):
            _ = context.pages
        browser.close()
    finally:
        bridge.close()
