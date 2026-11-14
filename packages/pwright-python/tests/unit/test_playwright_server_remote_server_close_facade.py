# pyright: reportPrivateUsage=false
"""Fake-native coverage for ordered Playwright Server close notices."""

from __future__ import annotations

import asyncio
import threading
from collections.abc import Callable
from types import SimpleNamespace
from typing import Any, cast

import pytest
from pwright import TargetClosedError, async_api, sync_api


class _BrowserNative:
    guid = "browser@1"
    version = "155.0"

    def __init__(self) -> None:
        self.connected = True
        self.notices: asyncio.Queue[tuple[str, str | None, int | None]] = asyncio.Queue()
        self.notice_read = asyncio.Event()
        self.contexts: list[_ContextNative] = []
        self.next_sequence = 10
        self.early_context_close = False
        self.early_page_close = False
        self.early_terminal = False
        self.delay_close_notices = False
        self.close_called = asyncio.Event()
        self.release_close_notices = asyncio.Event()
        self.close_publish_task: asyncio.Task[None] | None = None

    def is_connected(self) -> bool:
        return self.connected

    def live_contexts(self) -> list[tuple[str, int]]:
        return [
            (context.guid, context.created_sequence)
            for context in self.contexts
            if context.is_open()
        ]

    async def wait_disconnected(self) -> None:
        raise AssertionError("the close-notice profile must not start the legacy watcher")

    async def next_close_notice(self) -> tuple[str, str | None, int | None]:
        notice = await self.notices.get()
        self.notice_read.set()
        if notice[0] == "failure":
            raise RuntimeError("synthetic notice stream failed")
        return notice

    def emit(self, kind: str, guid: str | None, generation: int | None) -> None:
        if kind == "page":
            for context in self.contexts:
                for page in context.pages:
                    if (page.guid, page.created_sequence) == (guid, generation):
                        page.open = False
        elif kind == "context":
            for context in self.contexts:
                if (context.guid, context.created_sequence) == (guid, generation):
                    context.open = False
        elif kind == "disconnected":
            self.connected = False
        self.notices.put_nowait((kind, guid, generation))

    async def new_context(self) -> _ContextNative:
        context = _ContextNative(self, "context@1", self.next_sequence)
        self.next_sequence += 1
        self.contexts.append(context)
        if self.early_context_close:
            self.emit("context", context.guid, context.created_sequence)
            if self.early_terminal:
                self.emit("browser", self.guid, 1)
                self.emit("disconnected", None, None)
            await self.notice_read.wait()
        return context

    async def close(self) -> None:
        self.close_called.set()
        if self.delay_close_notices:
            self.close_publish_task = asyncio.create_task(
                self._publish_close_notices_after_release()
            )
            return
        self.emit("disconnected", None, None)

    async def _publish_close_notices_after_release(self) -> None:
        await self.release_close_notices.wait()
        for context in self.contexts:
            for page in context.pages:
                self.emit("page", page.guid, page.created_sequence)
            self.emit("context", context.guid, context.created_sequence)
        self.emit("browser", self.guid, 1)
        self.emit("disconnected", None, None)


class _ContextNative:
    def __init__(self, browser: _BrowserNative, guid: str, sequence: int) -> None:
        self.browser = browser
        self.guid = guid
        self.created_sequence = sequence
        self.open = True
        self.pages: list[_PageNative] = []

    def is_open(self) -> bool:
        return self.open and self.browser.is_connected()

    def live_pages(self) -> list[tuple[str, int]]:
        return [(page.guid, page.created_sequence) for page in self.pages if page.is_open()]

    async def new_page(self) -> _PageNative:
        page = _PageNative(self, "page@1", self.browser.next_sequence)
        self.browser.next_sequence += 1
        self.pages.append(page)
        if self.browser.early_page_close:
            self.browser.emit("page", page.guid, page.created_sequence)
            if self.browser.early_terminal:
                self.browser.emit("context", self.guid, self.created_sequence)
                self.browser.emit("browser", self.browser.guid, 1)
                self.browser.emit("disconnected", None, None)
            await self.browser.notice_read.wait()
        return page

    async def close(self) -> None:
        self.browser.emit("context", self.guid, self.created_sequence)


class _PageNative:
    def __init__(self, context: _ContextNative, guid: str, sequence: int) -> None:
        self.context = context
        self.guid = guid
        self.created_sequence = sequence
        self.open = True
        self.disconnect_on_title = False

    def is_open(self) -> bool:
        return self.open and self.context.is_open()

    async def title(self) -> str:
        if self.disconnect_on_title:
            browser = self.context.browser
            browser.emit("page", self.guid, self.created_sequence)
            browser.emit("context", self.context.guid, self.context.created_sequence)
            browser.emit("browser", browser.guid, 1)
            browser.emit("disconnected", None, None)
            raise RuntimeError("synthetic transport lost")
        return "Remote title"


def _browser(monkeypatch: pytest.MonkeyPatch) -> tuple[async_api.Browser, _BrowserNative]:
    monkeypatch.setattr(
        async_api._native,
        "selected_features",
        lambda: ["playwright-server-close-notices", "playwright-server-frame-title"],
    )
    native = _BrowserNative()
    playwright = SimpleNamespace(_browsers=[])
    browser_type = SimpleNamespace(_playwright=playwright)
    browser = async_api.Browser(cast(Any, browser_type), cast(Any, native), remote=True)
    playwright._browsers.append(browser)
    return browser, native


async def _family(
    monkeypatch: pytest.MonkeyPatch,
) -> tuple[async_api.Browser, async_api.RemoteBrowserContext, async_api.RemotePage, _BrowserNative]:
    browser, native = _browser(monkeypatch)
    context = cast(async_api.RemoteBrowserContext, await browser.new_context())
    page = cast(async_api.RemotePage, await context.new_page())
    return browser, context, page, native


async def _wait_watcher(browser: async_api.Browser) -> None:
    task = browser._remote_disconnect_task
    assert task is not None
    await asyncio.wait_for(task, timeout=1)


async def _wait_until(predicate: Callable[[], bool]) -> None:
    async def poll() -> None:
        while not predicate():
            await asyncio.sleep(0)

    await asyncio.wait_for(poll(), timeout=1)


@pytest.mark.asyncio
async def test_page_notice_closes_only_its_page(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    closed = asyncio.Event()
    events: list[str] = []
    page.on("close", lambda _: (events.append("page"), closed.set()))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    try:
        # Act
        native.emit("page", page._object_id, page._remote_native.created_sequence)
        await asyncio.wait_for(closed.wait(), timeout=1)

        # Assert
        assert events == ["page"]
        assert page.is_closed()
        assert not context.is_closed()
        assert browser.is_connected()
    finally:
        await browser._cancel_remote_disconnect_watcher()


@pytest.mark.asyncio
async def test_ordered_notices_close_each_level_once(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    # Act
    native.emit("page", page._object_id, page._remote_native.created_sequence)
    native.emit("page", page._object_id, page._remote_native.created_sequence)
    native.emit("context", context._object_id, context._remote_native.created_sequence)
    native.emit("context", context._object_id, context._remote_native.created_sequence)
    native.emit("browser", native.guid, 1)
    native.emit("browser", native.guid, 1)
    await _wait_until(lambda: browser._closed)

    # Assert: BrowserClosed retires the Browser without publishing disconnected.
    assert events == ["page", "context"]
    assert not browser.is_connected()
    native.emit("disconnected", None, None)
    await _wait_watcher(browser)
    assert events == ["page", "context", "browser"]
    await browser.close()
    assert events == ["page", "context", "browser"]


@pytest.mark.asyncio
async def test_terminal_flush_waits_for_disconnected_after_browser_closed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))
    native.emit("browser", native.guid, 1)
    await _wait_until(lambda: browser._closed)

    # Act
    flush = asyncio.create_task(browser._flush_remote_disconnect())
    await asyncio.sleep(0)

    # Assert
    assert events == ["page", "context"]
    assert not flush.done()
    native.emit("disconnected", None, None)
    await asyncio.wait_for(flush, timeout=1)
    assert events == ["page", "context", "browser"]


@pytest.mark.asyncio
async def test_context_notice_closes_remaining_pages_first(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, first, native = await _family(monkeypatch)
    second_native = _PageNative(cast(_ContextNative, context._remote_native), "page@2", 99)
    cast(_ContextNative, context._remote_native).pages.append(second_native)
    second = async_api.RemotePage(context, cast(Any, second_native))
    context._remote_pages[second._object_id] = second
    events: list[str] = []
    closed = asyncio.Event()
    first.on("close", lambda _: events.append("first"))
    second.on("close", lambda _: events.append("second"))
    context.on("close", lambda _: (events.append("context"), closed.set()))

    try:
        # Act
        native.emit("context", context._object_id, context._remote_native.created_sequence)
        await asyncio.wait_for(closed.wait(), timeout=1)

        # Assert
        assert events == ["first", "second", "context"]
        assert browser.is_connected()
    finally:
        await browser._cancel_remote_disconnect_watcher()


@pytest.mark.asyncio
async def test_disconnected_notice_closes_remaining_children_in_order(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    # Act
    native.emit("disconnected", None, None)
    await _wait_watcher(browser)

    # Assert
    assert events == ["page", "context", "browser"]
    assert page.is_closed() and context.is_closed() and not browser.is_connected()


@pytest.mark.asyncio
async def test_failed_operation_flushes_ordered_terminal_notices_before_raising(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, _native = await _family(monkeypatch)
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))
    cast(_PageNative, page._remote_native).disconnect_on_title = True

    # Act
    with pytest.raises(TargetClosedError, match="has been closed"):
        await page.title()

    # Assert
    assert events == ["page", "context", "browser"]
    assert not browser.is_connected()


@pytest.mark.asyncio
async def test_explicit_closes_stay_idempotent_with_notice_watcher(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, _native = await _family(monkeypatch)
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    # Act
    await context.close()
    await context.close()
    await browser.close()
    await browser.close()

    # Assert
    assert events == ["page", "context", "browser"]
    assert browser._remote_disconnect_task is None


@pytest.mark.asyncio
async def test_browser_close_waits_for_delayed_ordered_notices(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    native.delay_close_notices = True
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    # Act
    closing = asyncio.create_task(browser.close())
    await asyncio.wait_for(native.close_called.wait(), timeout=1)
    assert not closing.done() and events == []
    native.release_close_notices.set()
    await asyncio.wait_for(closing, timeout=1)
    await browser.close()

    # Assert
    assert events == ["page", "context", "browser"]
    assert page.is_closed() and context.is_closed() and not browser.is_connected()


@pytest.mark.asyncio
async def test_browser_close_falls_back_when_notices_never_arrive(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    native.delay_close_notices = True
    monkeypatch.setattr(async_api, "_REMOTE_CREATION_FENCE_TIMEOUT_SECONDS", 0.01)
    events: list[str] = []
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    # Act
    await asyncio.wait_for(browser.close(), timeout=1)
    native.release_close_notices.set()
    assert native.close_publish_task is not None
    await native.close_publish_task

    # Assert
    assert events == ["page", "context", "browser"]
    assert browser._remote_disconnect_task is None


@pytest.mark.asyncio
async def test_close_notices_survive_live_property_pruning(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    events: list[str] = []
    done = asyncio.Event()
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: (events.append("context"), done.set()))
    cast(_PageNative, page._remote_native).open = False
    cast(_ContextNative, context._remote_native).open = False
    assert context.pages == []
    assert browser.contexts == []

    try:
        # Act
        native.emit("page", page._object_id, page._remote_native.created_sequence)
        native.emit("context", context._object_id, context._remote_native.created_sequence)
        await asyncio.wait_for(done.wait(), timeout=1)

        # Assert
        assert events == ["page", "context"]
        assert browser.is_connected()
    finally:
        await browser._cancel_remote_disconnect_watcher()


@pytest.mark.asyncio
async def test_guid_reuse_matches_creation_sequence(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, old_context, old_page, native = await _family(monkeypatch)
    new_context_native = _ContextNative(native, old_context._object_id, 30)
    native.contexts.append(new_context_native)
    new_context = async_api.RemoteBrowserContext(browser, cast(Any, new_context_native))
    browser._remote_contexts[new_context._object_id] = new_context
    new_page_native = _PageNative(new_context_native, old_page._object_id, 31)
    new_context_native.pages.append(new_page_native)
    new_page = async_api.RemotePage(new_context, cast(Any, new_page_native))
    new_context._remote_pages[new_page._object_id] = new_page
    events: list[str] = []
    done = asyncio.Event()
    old_page.on("close", lambda _: events.append("old page"))
    new_page.on("close", lambda _: events.append("new page"))
    old_context.on("close", lambda _: (events.append("old context"), done.set()))
    new_context.on("close", lambda _: events.append("new context"))

    try:
        # Act
        native.emit("page", old_page._object_id, old_page._remote_native.created_sequence)
        native.emit("context", old_context._object_id, old_context._remote_native.created_sequence)
        await asyncio.wait_for(done.wait(), timeout=1)

        # Assert
        assert events == ["old page", "old context"]
        assert not new_page.is_closed() and not new_context.is_closed()
        assert browser.contexts == [new_context]
        assert new_context.pages == [new_page]
    finally:
        await browser._cancel_remote_disconnect_watcher()


@pytest.mark.asyncio
async def test_early_page_and_terminal_notices_wait_for_creation_fence(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, native = _browser(monkeypatch)
    context = cast(async_api.RemoteBrowserContext, await browser.new_context())
    native.early_page_close = True
    native.early_terminal = True
    events: list[str] = []
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    # Act
    page = cast(async_api.RemotePage, await context.new_page())
    page.on("close", lambda _: events.append("page"))
    await _wait_watcher(browser)

    # Assert
    assert events == ["page", "context", "browser"]
    assert page.is_closed() and context.is_closed() and not browser.is_connected()


@pytest.mark.asyncio
async def test_early_context_close_waits_for_wrapper_creation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, native = _browser(monkeypatch)
    native.early_context_close = True
    events: list[str] = []
    closed = asyncio.Event()
    browser.on("disconnected", lambda _: events.append("browser"))

    try:
        # Act
        context = cast(async_api.RemoteBrowserContext, await browser.new_context())
        context.on("close", lambda _: (events.append("context"), closed.set()))
        await asyncio.wait_for(closed.wait(), timeout=1)

        # Assert
        assert events == ["context"]
        assert context.is_closed() and browser.is_connected()
    finally:
        await browser._cancel_remote_disconnect_watcher()


@pytest.mark.asyncio
async def test_notice_watcher_failure_closes_children_before_disconnected(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    browser, context, page, native = await _family(monkeypatch)
    events: list[str] = []
    failures: list[object] = []
    loop = asyncio.get_running_loop()
    previous_handler = loop.get_exception_handler()
    loop.set_exception_handler(lambda _loop, details: failures.append(details.get("exception")))
    page.on("close", lambda _: events.append("page"))
    context.on("close", lambda _: events.append("context"))
    browser.on("disconnected", lambda _: events.append("browser"))

    try:
        # Act
        native.emit("failure", None, None)
        await _wait_watcher(browser)

        # Assert
        assert events == ["page", "context", "browser"]
        assert len(failures) == 1 and isinstance(failures[0], RuntimeError)
    finally:
        loop.set_exception_handler(previous_handler)


def test_sync_notice_callbacks_run_on_caller_in_order_and_allow_reentry(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    bridge = sync_api._LoopThread()
    dispatcher: sync_api._CallerDispatcher | None = None
    try:
        async_browser, async_context, async_page, native = bridge.run(_family(monkeypatch))
        dispatcher = sync_api._CallerDispatcher()
        bridge.install_caller_dispatcher(dispatcher)
        browser = sync_api.Browser(bridge, async_browser, cast(Any, object()))
        context = browser._wrap_context(async_context)
        page = context._wrap_page(async_page)
        caller = threading.get_ident()
        events: list[tuple[str, int, bool]] = []

        def on_page(_value: object) -> None:
            events.append(("page", threading.get_ident(), browser.is_connected()))

        page.on("close", on_page)
        context.on(
            "close",
            lambda _: events.append(("context", threading.get_ident(), browser.is_connected())),
        )
        browser.on(
            "disconnected",
            lambda _: events.append(("browser", threading.get_ident(), browser.is_connected())),
        )

        async def publish_page() -> None:
            native.emit("page", async_page._object_id, async_page._remote_native.created_sequence)
            await _wait_until(lambda: async_page._remote_close_emitted)

        async def publish_rest() -> None:
            native.emit(
                "context", async_context._object_id, async_context._remote_native.created_sequence
            )
            native.emit("browser", native.guid, 1)
            native.emit("disconnected", None, None)
            await _wait_watcher(async_browser)

        # Act
        bridge.run(publish_page())
        bridge.run(publish_rest())
        browser.close()

        # Assert
        assert events == [
            ("page", caller, True),
            ("context", caller, False),
            ("browser", caller, False),
        ]
    finally:
        bridge.close()
        if dispatcher is not None:
            dispatcher.close()


def test_sync_page_notice_callback_can_close_browser_reentrantly(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    bridge = sync_api._LoopThread()
    dispatcher: sync_api._CallerDispatcher | None = None
    try:
        async_browser, async_context, async_page, native = bridge.run(_family(monkeypatch))
        dispatcher = sync_api._CallerDispatcher()
        bridge.install_caller_dispatcher(dispatcher)
        browser = sync_api.Browser(bridge, async_browser, cast(Any, object()))
        context = browser._wrap_context(async_context)
        page = context._wrap_page(async_page)
        caller = threading.get_ident()
        events: list[tuple[str, int]] = []

        def on_page(_value: object) -> None:
            events.append(("page", threading.get_ident()))
            browser.close()

        page.on("close", on_page)
        context.on("close", lambda _: events.append(("context", threading.get_ident())))
        browser.on("disconnected", lambda _: events.append(("browser", threading.get_ident())))

        async def publish_page() -> None:
            native.emit("page", async_page._object_id, async_page._remote_native.created_sequence)
            await _wait_until(lambda: async_page._remote_close_emitted)

        # Act
        bridge.run(publish_page())

        # Assert
        assert events == [("page", caller), ("context", caller), ("browser", caller)]
        assert not browser.is_connected()
    finally:
        bridge.close()
        if dispatcher is not None:
            dispatcher.close()
