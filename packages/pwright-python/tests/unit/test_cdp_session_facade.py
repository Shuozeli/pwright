# pyright: reportPrivateUsage=false
"""CDPSession facades retain the Rust owner and forward raw CDP events."""

from __future__ import annotations

import asyncio
import gc
import json
import threading
import weakref
from types import SimpleNamespace
from typing import Any, Callable, cast

import pytest
from pwright import _native
from pwright._errors import (
    Error,
    InvalidArgumentError,
    ResourceExhaustedError,
    TargetClosedError,
    UnsupportedError,
)
from pwright.async_api import Browser, BrowserContext, Page, Playwright
from pwright.async_api import CDPSession as AsyncCDPSession
from pwright.sync_api import BrowserContext as SyncBrowserContext
from pwright.sync_api import CDPSession as SyncCDPSession
from pwright.sync_api import _LoopThread


class _NativeLease:
    def __init__(self, native: _NativeSession, token: int) -> None:
        self._native = native
        self._token = token
        self._claimed = False

    def token(self) -> int:
        return self._token

    def claim(self) -> int:
        if self._claimed:
            raise AssertionError("CDP delivery lease claimed twice")
        self._claimed = True
        if self._native.on_claim is not None:
            self._native.on_claim(self._token)
        self._native.claimed_tokens.append(self._token)
        return self._token

    def __del__(self) -> None:
        if not self._claimed:
            self._native.unclaimed_tokens.append(self._token)


class _NativeBatch:
    def __init__(self, native: _NativeSession, events: list[tuple[str, dict[str, object]]]) -> None:
        self._native = native
        self._events = events
        self._cursor = 0

    def peek(self) -> tuple[str, dict[str, object]] | None:
        if self._cursor == len(self._events):
            return None
        return self._events[self._cursor]

    def ack_next(self) -> None:
        if self._cursor == len(self._events):
            raise AssertionError("CDP event batch exhausted")
        self._native.acked_events.append(self._events[self._cursor])
        self._cursor += 1

    def rollback(self) -> None:
        self._native.rollback_count += 1
        self._native.events[:0] = self._events[self._cursor :]


class _NativeSession:
    def __init__(self) -> None:
        self.calls: list[tuple[object, ...]] = []
        self.events: list[tuple[str, dict[str, object]]] = []
        self.acked_events: list[tuple[str, dict[str, object]]] = []
        self.rollback_count = 0
        self.claimed_tokens: list[int] = []
        self.unclaimed_tokens: list[int] = []
        self.retired_tokens: set[int] = set()
        self.on_claim: Callable[[int], None] | None = None
        self._next_token = 7
        self._generation = 0
        # Python 3.9 binds Condition at construction, so create it on the
        # bridge loop rather than in a synchronous test's main thread.
        self._generation_condition: asyncio.Condition | None = None
        self._watch_started: asyncio.Event | None = None
        self._watch_cancelled: asyncio.Event | None = None
        self.watch_waiters = 0
        self.max_watch_waiters = 0
        self.closed = False

    @property
    def watch_started(self) -> asyncio.Event:
        event = self._watch_started
        if event is None:
            event = asyncio.Event()
            self._watch_started = event
        return event

    @property
    def watch_cancelled(self) -> asyncio.Event:
        event = self._watch_cancelled
        if event is None:
            event = asyncio.Event()
            self._watch_cancelled = event
        return event

    def _condition(self) -> asyncio.Condition:
        condition = self._generation_condition
        if condition is None:
            condition = asyncio.Condition()
            self._generation_condition = condition
        return condition

    def frames(self, _page_id: str) -> list[tuple[str, str | None, str, str]]:
        return [("frame-1", None, "", "about:blank")]

    async def cdp_session_create(
        self, context_id: str, page_id: str, frame_id: str | None
    ) -> _NativeLease:
        self.calls.append(("create", context_id, page_id, frame_id))
        token = self._next_token
        self._next_token += 1
        return _NativeLease(self, token)

    async def cdp_session_send(
        self, token: int, method: str, params_json: str
    ) -> dict[str, object]:
        self.calls.append(("send", token, method, json.loads(params_json)))
        if method == "Runtime.evaluate":
            self.events.append(("Runtime.consoleAPICalled", {"value": "log"}))
        return {"result": {"value": 3}}

    async def cdp_session_claim(self, token: int, max_events: int) -> _NativeBatch:
        self.calls.append(("claim", token, max_events))
        if token in self.retired_tokens:
            raise TargetClosedError("Target page, context or browser has been closed")
        events = self.events[:max_events]
        del self.events[:max_events]
        return _NativeBatch(self, events)

    async def cdp_session_detach(self, token: int) -> None:
        self.calls.append(("detach", token))

    async def next_cdp_session_event_generation(self, after: int) -> int:
        self.calls.append(("watch", after))
        condition = self._condition()
        async with condition:
            if self._generation != after:
                return self._generation
            self.watch_waiters += 1
            self.max_watch_waiters = max(self.max_watch_waiters, self.watch_waiters)
            self.watch_started.set()
            try:
                await condition.wait_for(lambda: self._generation != after or self.closed)
            except asyncio.CancelledError:
                self.watch_cancelled.set()
                raise
            finally:
                self.watch_waiters -= 1
            if self.closed:
                raise TargetClosedError("Target page, context or browser has been closed")
            return self._generation

    async def publish_event(self, method: str, params: dict[str, object]) -> None:
        condition = self._condition()
        async with condition:
            self.events.append((method, params))
            self._generation += 1
            condition.notify_all()

    async def retire_session(self, token: int) -> None:
        condition = self._condition()
        async with condition:
            self.retired_tokens.add(token)
            self._generation += 1
            condition.notify_all()

    async def close(self) -> None:
        self.closed = True
        condition = self._condition()
        async with condition:
            condition.notify_all()

    async def close_page(self, page_id: str) -> bool:
        self.calls.append(("close_page", page_id))
        return False

    async def goto(
        self, page_id: str, url: str, wait_until: str, referer: str | None, timeout_ms: int
    ) -> dict[str, object]:
        self.calls.append(("goto", page_id, url, wait_until, referer, timeout_ms))
        self.events.append(("Network.requestWillBeSent", {"request": {"url": url}}))
        return {"url": url, "response": None}


class _Browser:
    def __init__(self, native: _NativeSession) -> None:
        self._native = native
        self._cdp_detach_tasks: set[asyncio.Task[None]] = set()

    def _raise_deferred_route_error(self) -> None:
        pass

    async def _flush_network_notices(self) -> None:
        pass


def _context_and_page(native: _NativeSession) -> tuple[BrowserContext, Page]:
    context = BrowserContext(cast(Any, _Browser(native)), "context-1", None)
    page = Page(context, "page-1", "frame-1", "about:blank")
    return context, page


def _live_context_and_page(native: _NativeSession) -> tuple[Browser, BrowserContext, Page]:
    playwright = SimpleNamespace(_browsers=[])
    browser_type = SimpleNamespace(_playwright=playwright)
    browser = Browser(cast(Any, browser_type), cast(Any, native))
    playwright._browsers.append(browser)
    context = BrowserContext(browser, "context-1", None)
    page = Page(context, "page-1", "frame-1", "about:blank")
    context._pages[page._object_id] = page
    browser._contexts[context._object_id] = context
    return browser, context, page


def _bridged_cdp_session(
    context: BrowserContext, page: Page
) -> tuple[_LoopThread, AsyncCDPSession, SyncCDPSession]:
    bridge = _LoopThread()

    async def create_owner() -> AsyncCDPSession:
        return AsyncCDPSession(context, page._object_id, 7)

    try:
        owner = bridge.run(create_owner())
    except BaseException:
        bridge.close()
        raise
    return bridge, owner, SyncCDPSession(bridge, owner)


@pytest.mark.asyncio
async def test_async_cdp_session_sends_and_delivers_prior_events(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    events: list[dict[str, object]] = []
    session.on("Runtime.consoleAPICalled", events.append)

    # Act
    result = await session.send("Runtime.evaluate", {"expression": "1 + 2"})

    # Assert
    assert result == {"result": {"value": 3}}
    assert events == [{"value": "log"}]
    assert native.calls[:2] == [
        ("create", "context-1", "page-1", None),
        ("send", 7, "Runtime.evaluate", {"expression": "1 + 2"}),
    ]
    assert native.acked_events == [("Runtime.consoleAPICalled", {"value": "log"})]


@pytest.mark.asyncio
async def test_cdp_handler_failure_rolls_back_unacknowledged_events(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.extend([("Runtime.event", {"index": 1}), ("Runtime.event", {"index": 2})])

    def fail(_params: dict[str, object]) -> None:
        raise ValueError("callback failed")

    session.on("Runtime.event", fail)

    # Act
    with pytest.raises(ValueError, match="callback failed"):
        await session._drain_events()

    # Assert
    assert native.acked_events == []
    assert native.events == [
        ("Runtime.event", {"index": 1}),
        ("Runtime.event", {"index": 2}),
    ]
    assert native.rollback_count == 1


@pytest.mark.asyncio
async def test_cdp_callback_cancellation_preserves_only_unacknowledged_tail(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.extend([("Runtime.event", {"index": 1}), ("Runtime.event", {"index": 2})])
    delivered: list[int] = []

    def cancel_second(params: dict[str, object]) -> None:
        if params["index"] == 2:
            raise asyncio.CancelledError
        delivered.append(cast(int, params["index"]))

    session.on("Runtime.event", cancel_second)

    # Act
    with pytest.raises(Error, match="event handler cancelled") as captured:
        await session._drain_events()

    # Assert
    assert delivered == [1]
    assert native.acked_events == [("Runtime.event", {"index": 1})]
    assert native.events == [("Runtime.event", {"index": 2})]
    assert native.rollback_count == 1
    assert isinstance(captured.value.__cause__, asyncio.CancelledError)


@pytest.mark.asyncio
async def test_cdp_awaitable_once_handler_runs_after_event_ack(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.append(("Runtime.event", {"index": 1}))
    release = asyncio.Event()
    observed: list[dict[str, object]] = []

    async def handler(params: dict[str, object]) -> None:
        observed.append(params)
        await release.wait()

    session.once("Runtime.event", handler)

    # Act
    await session._drain_events()
    await asyncio.sleep(0)

    # Assert
    assert native.acked_events == [("Runtime.event", {"index": 1})]
    assert native.events == []
    assert observed == [{"index": 1}]
    assert len(session._pending_event_handlers) == 1
    assert session._once_handlers == {}
    release.set()
    await asyncio.gather(*session._pending_event_handlers)
    await asyncio.sleep(0)
    assert session._pending_event_handlers == set()


@pytest.mark.asyncio
async def test_cdp_async_handler_failure_is_reported_without_replaying_event(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.append(("Runtime.event", {"index": 1}))
    errors: list[BaseException] = []
    reported = asyncio.Event()
    loop = asyncio.get_running_loop()
    previous_handler = loop.get_exception_handler()

    def report(_loop: asyncio.AbstractEventLoop, details: dict[str, object]) -> None:
        error = details.get("exception")
        if isinstance(error, BaseException):
            errors.append(error)
        reported.set()

    async def fail(_params: dict[str, object]) -> None:
        raise ValueError("async callback failed")

    loop.set_exception_handler(report)
    session.on("Runtime.event", fail)

    # Act
    try:
        await session._drain_events()
        await asyncio.wait_for(reported.wait(), timeout=1)

        # Assert
        assert len(errors) == 1
        assert isinstance(errors[0], ValueError)
        assert str(errors[0]) == "async callback failed"
        assert native.acked_events == [("Runtime.event", {"index": 1})]
        assert native.events == []
    finally:
        loop.set_exception_handler(previous_handler)


@pytest.mark.asyncio
async def test_cdp_retirement_cancels_owned_async_handlers(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.append(("Runtime.event", {"index": 1}))
    started = asyncio.Event()
    cancelled = asyncio.Event()

    async def handler(_params: dict[str, object]) -> None:
        started.set()
        try:
            await asyncio.Event().wait()
        except asyncio.CancelledError:
            cancelled.set()
            raise

    session.on("Runtime.event", handler)
    await session._drain_events()
    await asyncio.wait_for(started.wait(), timeout=1)

    # Act
    context._retire_cdp_session(session._token)
    await asyncio.wait_for(cancelled.wait(), timeout=1)

    # Assert
    assert session._closed
    assert session._pending_event_handlers == set()
    assert context._cdp_sessions == {}
    assert native.acked_events == [("Runtime.event", {"index": 1})]


@pytest.mark.asyncio
async def test_cdp_async_handler_capacity_preserves_unacknowledged_event(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.extend(("Runtime.event", {"index": index}) for index in range(65))

    async def handler(_params: dict[str, object]) -> None:
        await asyncio.Event().wait()

    session.on("Runtime.event", handler)

    # Act
    with pytest.raises(ResourceExhaustedError, match="pending async event handler limit"):
        await session._drain_events()

    # Assert
    assert len(native.acked_events) == 64
    assert native.events == [("Runtime.event", {"index": 64})]
    assert len(session._pending_event_handlers) == 64
    context._retire_cdp_session(session._token)
    await asyncio.sleep(0)
    assert session._pending_event_handlers == set()


@pytest.mark.asyncio
async def test_cdp_retirement_does_not_cancel_external_handler_future(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.append(("Runtime.event", {"index": 1}))
    external: asyncio.Future[object] = asyncio.get_running_loop().create_future()
    session.on("Runtime.event", lambda _params: external)
    await session._drain_events()

    # Act
    context._retire_cdp_session(session._token)

    # Assert
    assert not external.cancelled()
    assert session._pending_event_handlers == set()
    assert native.acked_events == [("Runtime.event", {"index": 1})]
    external.set_result(None)


@pytest.mark.asyncio
async def test_failed_once_callback_is_not_invoked_again_on_redrain(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.append(("Runtime.event", {"value": 1}))
    calls = 0

    def fail_once(_params: dict[str, object]) -> None:
        nonlocal calls
        calls += 1
        raise ValueError("callback failed")

    session.once("Runtime.event", fail_once)

    # Act
    with pytest.raises(ValueError, match="callback failed"):
        await session._drain_events()
    await session._drain_events()

    # Assert
    assert calls == 1
    assert native.events == []
    assert native.acked_events == [("Runtime.event", {"value": 1})]
    assert session._once_handlers == {}


@pytest.mark.asyncio
async def test_async_once_listener_can_be_removed_by_original_handler(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    delivered: list[dict[str, object]] = []
    handler = delivered.append
    session.once("Runtime.event", handler)

    # Act
    session.remove_listener("Runtime.event", handler)
    native.events.append(("Runtime.event", {"value": 1}))
    await session._drain_events()

    # Assert
    assert delivered == []
    assert session._event_handlers == {}
    assert session._once_handlers == {}


@pytest.mark.asyncio
async def test_cdp_batch_delivers_events_in_order_before_ack(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    native.events.extend(("Runtime.event", {"index": index}) for index in range(64))
    delivered: list[int] = []

    def record(params: dict[str, object]) -> None:
        delivered.append(cast(int, params["index"]))

    session.on("Runtime.event", record)

    # Act
    await session._drain_events()

    # Assert
    assert delivered == list(range(64))
    assert [params["index"] for _method, params in native.acked_events] == list(range(64))
    assert native.events == []


@pytest.mark.asyncio
async def test_main_frame_cdp_session_uses_exact_frame_identity(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)

    # Act
    await context.new_cdp_session(page.main_frame)

    # Assert
    assert native.calls == [("create", "context-1", "page-1", "frame-1")]


@pytest.mark.asyncio
async def test_cdp_delivery_lease_is_claimed_after_wrapper_is_registered(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)

    def check_registered(token: int) -> None:
        assert token in context._cdp_sessions

    native.on_claim = check_registered

    # Act
    session = await context.new_cdp_session(page)

    # Assert
    assert context._cdp_sessions[7] is session
    assert native.claimed_tokens == [7]


@pytest.mark.asyncio
async def test_cdp_create_finishing_after_browser_close_leaves_lease_unclaimed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])

    class DelayedNative(_NativeSession):
        def __init__(self) -> None:
            super().__init__()
            self.started = asyncio.Event()
            self.release = asyncio.Event()

        async def cdp_session_create(
            self, context_id: str, page_id: str, frame_id: str | None
        ) -> _NativeLease:
            lease = await super().cdp_session_create(context_id, page_id, frame_id)
            self.started.set()
            await self.release.wait()
            return lease

    native = DelayedNative()
    browser, context, page = _live_context_and_page(native)
    create = asyncio.create_task(context.new_cdp_session(page))
    await asyncio.wait_for(native.started.wait(), timeout=1)

    # Act
    await browser.close()
    native.release.set()
    with pytest.raises(TargetClosedError):
        await create
    gc.collect()

    # Assert
    assert native.claimed_tokens == []
    assert native.unclaimed_tokens == [7]
    assert context._cdp_sessions == {}
    assert browser._cdp_event_task is None


@pytest.mark.asyncio
async def test_idle_cdp_event_wakes_one_lazy_browser_watcher(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    browser, context, page = _live_context_and_page(native)
    assert browser._cdp_event_task is None
    session = await context.new_cdp_session(page)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)
    delivered = asyncio.Event()
    received: list[dict[str, object]] = []

    def record(params: dict[str, object]) -> None:
        received.append(params)
        delivered.set()

    session.on("Runtime.event", record)

    # Act
    try:
        await native.publish_event("Runtime.event", {"value": 1})
        await asyncio.wait_for(delivered.wait(), timeout=1)

        # Assert
        assert received == [{"value": 1}]
        assert native.acked_events == [("Runtime.event", {"value": 1})]
        assert native.max_watch_waiters == 1
        assert browser._cdp_watch_error is None
    finally:
        await browser.close()


@pytest.mark.asyncio
async def test_last_cdp_detach_cancels_parked_browser_watcher(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    browser, context, page = _live_context_and_page(native)
    first = await context.new_cdp_session(page)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)
    second = await context.new_cdp_session(page)

    # Act
    try:
        await first.detach()
        assert native.watch_waiters == 1
        await second.detach()
        await asyncio.wait_for(native.watch_cancelled.wait(), timeout=1)

        # Assert
        assert native.max_watch_waiters == 1
        assert native.watch_waiters == 0
        assert context._cdp_sessions == {}
    finally:
        await browser.close()


@pytest.mark.asyncio
async def test_cancelled_detach_finishes_native_cleanup_and_retires_wrapper(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])

    class DelayedNative(_NativeSession):
        def __init__(self) -> None:
            super().__init__()
            self.detach_started = asyncio.Event()
            self.release_detach = asyncio.Event()

        async def cdp_session_detach(self, token: int) -> None:
            self.calls.append(("detach", token))
            self.detach_started.set()
            await self.release_detach.wait()

    native = DelayedNative()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)
    watcher = browser._cdp_event_task
    detach = asyncio.create_task(session.detach())

    # Act
    try:
        await asyncio.wait_for(native.detach_started.wait(), timeout=1)
        native_detach = session._detach_task
        assert native_detach is not None
        detach.cancel()
        with pytest.raises(asyncio.CancelledError):
            await detach
        with pytest.raises(TargetClosedError):
            await session.send("Runtime.enable")
        native.release_detach.set()
        await asyncio.wait_for(native_detach, timeout=1)
        if watcher is not None:
            await asyncio.wait_for(asyncio.gather(watcher, return_exceptions=True), timeout=1)
        await asyncio.sleep(0)

        # Assert
        assert session._closed
        assert session._detach_task is None
        assert context._cdp_sessions == {}
        assert browser._cdp_event_task is None
        assert browser._cdp_detach_tasks == set()
        assert [call for call in native.calls if call[0] == "detach"] == [("detach", 7)]
    finally:
        native.release_detach.set()
        await browser.close()


@pytest.mark.asyncio
async def test_failed_detach_after_caller_cancellation_remains_retryable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])

    class DelayedFailureNative(_NativeSession):
        def __init__(self) -> None:
            super().__init__()
            self.detach_started = asyncio.Event()
            self.release_detach = asyncio.Event()
            self.fail_next_detach = True

        async def cdp_session_detach(self, token: int) -> None:
            self.calls.append(("detach", token))
            self.detach_started.set()
            await self.release_detach.wait()
            if self.fail_next_detach:
                self.fail_next_detach = False
                raise ValueError("native detach failed")

    native = DelayedFailureNative()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    detach = asyncio.create_task(session.detach())

    # Act
    try:
        await asyncio.wait_for(native.detach_started.wait(), timeout=1)
        native_detach = session._detach_task
        assert native_detach is not None
        detach.cancel()
        with pytest.raises(asyncio.CancelledError):
            await detach
        native.release_detach.set()
        with pytest.raises(ValueError, match="native detach failed"):
            await asyncio.wait_for(native_detach, timeout=1)
        await asyncio.sleep(0)
        assert not session._closed
        assert session._detach_task is None
        assert context._cdp_sessions[7] is session
        await session.detach()

        # Assert
        assert session._closed
        assert context._cdp_sessions == {}
        assert [call for call in native.calls if call[0] == "detach"] == [
            ("detach", 7),
            ("detach", 7),
        ]
    finally:
        native.release_detach.set()
        await browser.close()


@pytest.mark.asyncio
async def test_browser_close_waits_for_cancelled_native_detach_task(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])

    class DelayedNative(_NativeSession):
        def __init__(self) -> None:
            super().__init__()
            self.detach_started = asyncio.Event()
            self.release_detach = asyncio.Event()

        async def cdp_session_detach(self, token: int) -> None:
            self.calls.append(("detach", token))
            self.detach_started.set()
            await self.release_detach.wait()

    native = DelayedNative()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    detach = asyncio.create_task(session.detach())
    await asyncio.wait_for(native.detach_started.wait(), timeout=1)
    native_detach = session._detach_task
    assert native_detach is not None
    detach.cancel()
    with pytest.raises(asyncio.CancelledError):
        await detach

    # Act
    close = asyncio.create_task(browser.close())
    try:
        for _ in range(5):
            await asyncio.sleep(0)
        assert native.closed
        assert not close.done()
        native.release_detach.set()
        await asyncio.wait_for(close, timeout=1)

        # Assert
        assert native_detach.done()
        assert browser._cdp_detach_tasks == set()
        assert context._cdp_sessions == {}
        assert session._closed
    finally:
        native.release_detach.set()
        if not close.done():
            await asyncio.wait_for(close, timeout=1)


@pytest.mark.asyncio
async def test_native_retirement_removes_idle_watcher_session(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    session.on("Runtime.event", lambda _params: None)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)
    watcher = browser._cdp_event_task

    # Act
    try:
        await native.retire_session(7)
        if watcher is not None:
            await asyncio.wait_for(asyncio.gather(watcher, return_exceptions=True), timeout=1)
        await asyncio.sleep(0)

        # Assert
        assert session._closed
        assert session._event_handlers == {}
        assert context._cdp_sessions == {}
        assert browser._cdp_event_task is None
        with pytest.raises(TargetClosedError):
            await session.send("Runtime.enable")
    finally:
        await browser.close()


@pytest.mark.asyncio
async def test_registration_flush_retires_natively_detached_session(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)
    native.retired_tokens.add(7)

    # Act
    try:
        session.on("Runtime.event", lambda _params: None)
        flush = session._registration_flush_task
        assert flush is not None
        await asyncio.wait_for(flush, timeout=1)
        await asyncio.sleep(0)

        # Assert
        assert session._closed
        assert context._cdp_sessions == {}
        assert browser._cdp_event_task is None or browser._cdp_event_task.cancelled()
    finally:
        await browser.close()


@pytest.mark.asyncio
async def test_listener_registration_flushes_event_without_new_generation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)
    native.events.append(("Runtime.event", {"value": 2}))
    delivered = asyncio.Event()

    # Act
    try:
        session.on("Runtime.event", lambda _params: delivered.set())
        await asyncio.wait_for(delivered.wait(), timeout=1)

        # Assert
        assert native.acked_events == [("Runtime.event", {"value": 2})]
        assert native._generation == 0
    finally:
        await browser.close()


@pytest.mark.asyncio
@pytest.mark.parametrize("failure_type", [ValueError, asyncio.CancelledError])
async def test_idle_handler_failure_stops_watcher_and_retains_event(
    monkeypatch: pytest.MonkeyPatch, failure_type: type[BaseException]
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)
    errors: list[BaseException] = []
    reported = asyncio.Event()
    loop = asyncio.get_running_loop()
    previous_handler = loop.get_exception_handler()

    def report(_loop: asyncio.AbstractEventLoop, details: dict[str, object]) -> None:
        error = details.get("exception")
        if isinstance(error, BaseException):
            errors.append(error)
        reported.set()

    def fail(_params: dict[str, object]) -> None:
        raise failure_type("callback failed")

    loop.set_exception_handler(report)
    session.on("Runtime.event", fail)

    # Act
    try:
        await native.publish_event("Runtime.event", {"value": 3})
        await asyncio.wait_for(reported.wait(), timeout=1)
        task = browser._cdp_event_task
        if task is not None:
            await asyncio.gather(task, return_exceptions=True)
            await asyncio.sleep(0)

        # Assert
        assert len(errors) == 1
        if failure_type is asyncio.CancelledError:
            assert isinstance(errors[0], Error)
            assert isinstance(errors[0].__cause__, asyncio.CancelledError)
        else:
            assert isinstance(errors[0], failure_type)
        assert native.acked_events == []
        assert native.events == [("Runtime.event", {"value": 3})]
        assert browser._cdp_event_task is None
        assert browser._cdp_watch_error is errors[0]
        assert [call for call in native.calls if call[0] == "watch"] == [("watch", 0)]
    finally:
        loop.set_exception_handler(previous_handler)
        await browser.close()


@pytest.mark.asyncio
@pytest.mark.parametrize("shutdown", ["browser", "playwright"])
async def test_cdp_watcher_is_cancelled_on_owner_shutdown(
    monkeypatch: pytest.MonkeyPatch, shutdown: str
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    browser, context, page = _live_context_and_page(native)
    session = await context.new_cdp_session(page)
    await asyncio.wait_for(native.watch_started.wait(), timeout=1)

    # Act
    if shutdown == "browser":
        await browser.close()
    else:

        async def dispose_all() -> None:
            pass

        owner = cast(Any, Playwright.__new__(Playwright))
        owner._stopped = False
        owner._browsers = [browser]
        owner.request = SimpleNamespace(_dispose_all=dispose_all)
        owner._environment = SimpleNamespace(close=native.close)
        await owner.stop()

    # Assert
    assert native.watch_cancelled.is_set()
    assert native.watch_waiters == 0
    assert browser._cdp_event_task is None
    assert context._cdp_sessions == {}
    with pytest.raises(TargetClosedError):
        await session.send("Runtime.enable")


@pytest.mark.asyncio
async def test_cdp_session_detach_prevents_reuse(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)

    # Act
    await session.detach()

    # Assert
    with pytest.raises(TargetClosedError, match="Target page, context or browser"):
        await session.send("Runtime.enable")
    assert native.calls == [("create", "context-1", "page-1", None), ("detach", 7)]


@pytest.mark.asyncio
async def test_page_goto_drains_cdp_events_before_return(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)
    events: list[dict[str, object]] = []
    session.on("Network.requestWillBeSent", events.append)

    # Act
    response = await page.goto("https://example.test/", timeout=20)

    # Assert
    assert response is None
    assert events == [{"request": {"url": "https://example.test/"}}]


@pytest.mark.asyncio
async def test_page_close_retires_python_cdp_session_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, page = _context_and_page(native)
    session = await context.new_cdp_session(page)

    # Act
    await page.close()

    # Assert
    with pytest.raises(TargetClosedError, match="Target page, context or browser"):
        await session.detach()
    assert context._cdp_sessions == {}
    assert native.calls[-1] == ("close_page", "page-1")


@pytest.mark.asyncio
async def test_cdp_session_rejects_wrong_context_without_native_dispatch(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    context, _page = _context_and_page(native)
    other_context = BrowserContext(cast(Any, _Browser(native)), "context-2", None)
    other_page = Page(other_context, "page-2", "frame-2", "about:blank")

    # Act
    with pytest.raises(InvalidArgumentError, match="different BrowserContext"):
        await context.new_cdp_session(other_page)

    # Assert
    assert native.calls == []


@pytest.mark.asyncio
async def test_cdp_session_requires_opt_in_feature(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: cast(list[str], []))
    native = _NativeSession()
    context, page = _context_and_page(native)

    # Act
    with pytest.raises(UnsupportedError, match="cdp-session"):
        await context.new_cdp_session(page)

    # Assert
    assert native.calls == []


def test_sync_cdp_session_reuses_async_owner_and_event_boundary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["cdp-session"])
    native = _NativeSession()
    async_context, async_page = _context_and_page(native)
    bridge = _LoopThread()
    context = SyncBrowserContext(bridge, async_context, cast(Any, object()))
    page = context._wrap_page(async_page)

    # Act
    try:
        session = context.new_cdp_session(page)
        events: list[dict[str, object]] = []
        session.on("Runtime.consoleAPICalled", events.append)
        result = session.send("Runtime.evaluate", {"expression": "1 + 2"})
        session.detach()
    finally:
        bridge.close()

    # Assert
    assert result == {"result": {"value": 3}}
    assert events == [{"value": "log"}]
    assert native.calls[0] == ("create", "context-1", "page-1", None)
    assert native.calls[-1] == ("detach", 7)


@pytest.mark.parametrize("registration", ["on", "once"])
def test_sync_cdp_session_rejected_listener_is_not_retained(registration: str) -> None:
    # Arrange
    context, page = _context_and_page(_NativeSession())
    bridge, _owner, session = _bridged_cdp_session(context, page)
    try:
        for _ in range(64):
            session.on("Runtime.event", lambda _params: None)

        def rejected_callback_reference() -> weakref.ReferenceType[
            Callable[[dict[str, object]], None]
        ]:
            def rejected(_params: dict[str, object]) -> None:
                pass

            callback_reference = weakref.ref(rejected)

            # Act
            with pytest.raises(ResourceExhaustedError, match="handler limit"):
                if registration == "on":
                    session.on("Runtime.event", rejected)
                else:
                    session.once("Runtime.event", rejected)
            return callback_reference

        callback_reference = rejected_callback_reference()
    finally:
        bridge.close()

    # Assert
    gc.collect()
    assert callback_reference() is None


def test_sync_cdp_session_once_releases_callback_emitted_during_registration(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    context, page = _context_and_page(_NativeSession())
    bridge, owner, session = _bridged_cdp_session(context, page)
    delivered: list[dict[str, object]] = []

    def emit_immediately(_event: str, handler: Callable[[dict[str, object]], object]) -> None:
        handler({"value": 1})

    monkeypatch.setattr(owner, "once", emit_immediately)

    def registered_callback_reference() -> weakref.ReferenceType[
        Callable[[dict[str, object]], None]
    ]:
        def callback(params: dict[str, object]) -> None:
            delivered.append(params)

        callback_reference = weakref.ref(callback)

        # Act
        session.once("Runtime.event", callback)
        return callback_reference

    try:
        callback_reference = registered_callback_reference()
    finally:
        bridge.close()

    # Assert
    assert delivered == [{"value": 1}]
    gc.collect()
    assert callback_reference() is None


def test_sync_cdp_listener_mutations_run_on_bridge_loop_from_other_thread(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    context, page = _context_and_page(_NativeSession())
    bridge, owner, session = _bridged_cdp_session(context, page)
    mutation_threads: list[int] = []
    caller_threads: list[int] = []
    failures: list[BaseException] = []
    original_on = owner.on
    original_once = owner.once
    original_remove = owner.remove_listener

    def track_on(event: str, handler: Callable[[dict[str, object]], object]) -> None:
        mutation_threads.append(threading.get_ident())
        original_on(event, handler)

    def track_once(event: str, handler: Callable[[dict[str, object]], object]) -> None:
        mutation_threads.append(threading.get_ident())
        original_once(event, handler)

    def track_remove(event: str, handler: Callable[[dict[str, object]], object]) -> None:
        mutation_threads.append(threading.get_ident())
        original_remove(event, handler)

    monkeypatch.setattr(owner, "on", track_on)
    monkeypatch.setattr(owner, "once", track_once)
    monkeypatch.setattr(owner, "remove_listener", track_remove)

    def listener(_params: dict[str, object]) -> None:
        pass

    def mutate_from_other_thread() -> None:
        caller_threads.append(threading.get_ident())
        try:
            session.on("Runtime.event", listener)
            session.once("Runtime.event", listener)
            session.remove_listener("Runtime.event", listener)
        except BaseException as error:
            failures.append(error)

    # Act
    caller = threading.Thread(target=mutate_from_other_thread)
    try:
        caller.start()
        caller.join(timeout=5)
    finally:
        bridge.close()

    # Assert
    assert not caller.is_alive()
    assert failures == []
    assert len(mutation_threads) >= 3
    assert mutation_threads == [bridge._thread_id] * len(mutation_threads)
    assert caller_threads != [bridge._thread_id]
    assert owner._event_handlers == {}
    assert session._event_handlers == []
