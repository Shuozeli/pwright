# pyright: reportPrivateUsage=false
"""Remote terminal events retain Page -> Context -> Browser order."""

from __future__ import annotations

import asyncio
from types import SimpleNamespace
from typing import Any, cast

import pytest
from pwright import TargetClosedError, UnsupportedError, async_api, sync_api


@pytest.fixture(autouse=True)
def legacy_remote_close_profile(monkeypatch: pytest.MonkeyPatch) -> None:
    """These stubs exercise the pre-close-notice disconnect-only profile."""
    features = [
        feature
        for feature in async_api._native.selected_features()
        if feature != "playwright-server-close-notices"
    ]
    monkeypatch.setattr(async_api._native, "selected_features", lambda: features)


class _RemoteBrowserNative:
    version = "155.0"

    def __init__(self) -> None:
        self.disconnected = asyncio.Event()

    def is_connected(self) -> bool:
        return not self.disconnected.is_set()

    async def wait_disconnected(self) -> None:
        await self.disconnected.wait()

    async def close(self) -> None:
        self.disconnected.set()


class _RemoteChildNative:
    def __init__(self, owner: _RemoteBrowserNative, guid: str, sequence: int) -> None:
        self.owner = owner
        self.guid = guid
        self.created_sequence = sequence
        self.open = True
        self.disconnect_on_title = False
        self.title_error: Exception | None = None

    def is_open(self) -> bool:
        return self.open and self.owner.is_connected()

    async def title(self) -> str:
        if self.disconnect_on_title:
            self.owner.disconnected.set()
            raise RuntimeError("remote transport closed")
        if self.title_error is not None:
            raise self.title_error
        return "Remote title"


async def _family() -> tuple[
    async_api.Browser,
    async_api.RemoteBrowserContext,
    async_api.RemotePage,
    _RemoteBrowserNative,
]:
    native = _RemoteBrowserNative()
    playwright = SimpleNamespace(_browsers=[])
    browser_type = SimpleNamespace(_playwright=playwright)
    browser = async_api.Browser(cast(Any, browser_type), cast(Any, native), remote=True)
    playwright._browsers.append(browser)
    context = async_api.RemoteBrowserContext(
        browser, cast(Any, _RemoteChildNative(native, "context@1", 10))
    )
    page = async_api.RemotePage(context, cast(Any, _RemoteChildNative(native, "page@1", 11)))
    browser._remote_contexts[context._object_id] = context
    context._remote_pages[page._object_id] = page
    return browser, context, page, native


@pytest.mark.asyncio
async def test_explicit_remote_close_emits_child_events_once_in_order() -> None:
    # Arrange
    browser, context, page, _native = await _family()
    events: list[tuple[str, object]] = []
    page.on("close", lambda value: events.append(("page", value)))
    context.on("close", lambda value: events.append(("context", value)))
    browser.on("disconnected", lambda value: events.append(("browser", value)))

    # Act
    await browser.close()
    await browser.close()

    # Assert
    assert events == [("page", page), ("context", context), ("browser", browser)]
    assert page.is_closed() and context.is_closed() and not browser.is_connected()
    with pytest.raises(UnsupportedError):
        page.on("load", lambda _: None)
    with pytest.raises(UnsupportedError):
        context.on("page", lambda _: None)


@pytest.mark.asyncio
async def test_remote_transport_loss_emits_child_events_once_in_order() -> None:
    # Arrange
    browser, context, page, native = await _family()
    events: list[str] = []
    page.once("close", lambda _: events.append("page"))
    context.once("close", lambda _: events.append("context"))
    browser.once("disconnected", lambda _: events.append("browser"))

    # Act
    native.disconnected.set()
    assert browser._remote_disconnect_task is not None
    await asyncio.wait_for(browser._remote_disconnect_task, timeout=1)
    await browser.close()

    # Assert
    assert events == ["page", "context", "browser"]


@pytest.mark.asyncio
async def test_remote_async_operation_flushes_observed_transport_loss() -> None:
    # Arrange
    browser, context, page, native = await _family()
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))
    await browser._cancel_remote_disconnect_watcher()
    native.disconnected.set()

    # Act
    with pytest.raises(TargetClosedError, match="has been closed"):
        await page.title()

    # Assert
    assert events == ["page", "context", "browser"]
    assert not browser.is_connected()


@pytest.mark.asyncio
async def test_remote_page_only_disposal_does_not_disconnect_browser() -> None:
    # Arrange
    browser, context, page, _native = await _family()
    events: list[str] = []
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))
    cast(_RemoteChildNative, page._remote_native).open = False

    # Act
    with pytest.raises(TargetClosedError, match="has been closed"):
        await page.title()

    # Assert
    assert events == []
    assert browser.is_connected()


@pytest.mark.asyncio
async def test_remote_title_returns_typed_result_when_feature_is_selected(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["playwright-server-frame-title"]
    )
    browser, _context, page, _native = await _family()

    # Act
    title = await page.title()

    # Assert
    assert title == "Remote title"
    assert browser.is_connected()


@pytest.mark.asyncio
async def test_remote_title_error_flushes_terminal_events_before_raising(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["playwright-server-frame-title"]
    )
    browser, context, page, _native = await _family()
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))
    await browser._cancel_remote_disconnect_watcher()
    cast(_RemoteChildNative, page._remote_native).disconnect_on_title = True

    # Act
    with pytest.raises(TargetClosedError, match="has been closed"):
        await page.title()

    # Assert
    assert events == ["page", "context", "browser"]
    assert not browser.is_connected()


@pytest.mark.asyncio
async def test_remote_title_preserves_nonterminal_operation_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["playwright-server-frame-title"]
    )
    browser, _context, page, _native = await _family()
    cast(_RemoteChildNative, page._remote_native).title_error = RuntimeError("title rejected")

    # Act / Assert
    with pytest.raises(RuntimeError, match="title rejected"):
        await page.title()
    assert browser.is_connected()


def test_sync_remote_close_callbacks_run_on_caller_and_allow_reentry() -> None:
    # Arrange
    bridge = sync_api._LoopThread()
    dispatcher: sync_api._CallerDispatcher | None = None
    try:
        async_browser, async_context, async_page, _native = bridge.run(_family())
        dispatcher = sync_api._CallerDispatcher()
        bridge.install_caller_dispatcher(dispatcher)
        browser = sync_api.Browser(bridge, async_browser, cast(Any, object()))
        context = browser._wrap_context(async_context)
        page = context._wrap_page(async_page)
        events: list[str] = []
        reentered: list[bool] = []

        def on_page(_value: object) -> None:
            events.append("page")
            reentered.append(not browser.is_connected())
            browser.close()

        page.on("close", on_page)
        context.on("close", lambda _: events.append("context"))
        browser.on("disconnected", lambda _: events.append("browser"))

        # Act
        browser.close()
        browser.close()

        # Assert
        assert events == ["page", "context", "browser"]
        assert reentered == [True]
    finally:
        bridge.close()
        if dispatcher is not None:
            dispatcher.close()


def test_sync_remote_operation_drains_terminal_events_before_raising() -> None:
    # Arrange
    bridge = sync_api._LoopThread()
    try:
        async_browser, async_context, async_page, native = bridge.run(_family())
        bridge.run(async_browser._cancel_remote_disconnect_watcher())
        browser = sync_api.Browser(bridge, async_browser, cast(Any, object()))
        context = browser._wrap_context(async_context)
        page = context._wrap_page(async_page)
        events: list[str] = []
        page.on("close", lambda _: events.append("page"))
        context.on("close", lambda _: events.append("context"))
        browser.on("disconnected", lambda _: events.append("browser"))

        async def mark_transport_lost() -> None:
            native.disconnected.set()

        bridge.run(mark_transport_lost())

        # Act
        with pytest.raises(TargetClosedError, match="has been closed"):
            page.title()

        # Assert
        assert events == ["page", "context", "browser"]
        assert not browser.is_connected()
    finally:
        bridge.close()
