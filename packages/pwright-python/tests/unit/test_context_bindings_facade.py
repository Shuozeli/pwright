# pyright: reportPrivateUsage=false

from __future__ import annotations

import asyncio
from collections.abc import Awaitable
from types import SimpleNamespace
from typing import Any, cast

import pytest
from pwright import async_api
from pwright.async_api import Browser, BrowserContext, Page
from pwright.sync_api import Browser as SyncBrowser
from pwright.sync_api import BrowserContext as SyncBrowserContext


class NativeBindings:
    def __init__(self) -> None:
        self.registrations: list[tuple[str, str, bool]] = []
        self.acknowledged: list[str] = []
        self.results: list[tuple[str, object]] = []
        self.errors: list[tuple[str, str]] = []
        self.failure: BaseException | None = None

    def frames(self, _page_id: str) -> list[tuple[str, None, str, str]]:
        return [("11", None, "", "about:blank")]

    async def context_expose_binding(self, context_id: str, name: str, needs_handle: bool) -> None:
        if self.failure is not None:
            raise self.failure
        self.registrations.append((context_id, name, needs_handle))

    async def acknowledge_page_binding_call(self, call_id: str) -> None:
        self.acknowledged.append(call_id)

    async def deliver_page_binding_result(self, call_id: str, value: object) -> None:
        self.results.append((call_id, value))

    async def deliver_page_binding_error(
        self, call_id: str, _name: str, message: str, _stack: str | None
    ) -> None:
        self.errors.append((call_id, message))


class QueuedBindings(NativeBindings):
    def __init__(self) -> None:
        super().__init__()
        self.calls: asyncio.Queue[tuple[str, str, str, str, str, str, bool] | None] = (
            asyncio.Queue()
        )

    async def next_page_binding_call(
        self,
    ) -> tuple[str, str, str, str, str, str, bool] | None:
        return await self.calls.get()

    async def close(self) -> None:
        self.calls.put_nowait(None)


def context(native: NativeBindings) -> tuple[Browser, BrowserContext, Page]:
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    browser._closed = False
    browser._fresh_owned = False
    browser._persistent_context = False
    browser._binding_task = None
    browser._binding_watch_error = None
    browser._binding_dispatch_tasks = set()
    browser._binding_registration_tasks = set()
    browser._binding_closing_dispatch = None
    browser._persistent_context_viewport = None
    browser._worker_console_task = None
    browser._lifecycle_task = None
    browser._route_task = None
    browser._request_task = None
    browser._response_task = None
    browser._download_task = None
    browser._worker_task = None
    browser._cdp_event_task = None
    browser._cdp_registration_tasks = set()
    browser._cdp_detach_tasks = set()
    browser._browser_type = cast(
        Any, SimpleNamespace(_playwright=SimpleNamespace(_browsers=[browser]))
    )
    value = BrowserContext(browser, "7", None)
    page = Page(value, "9", "11", "about:blank")
    value._pages["9"] = page
    browser._contexts = {"7": value}
    return browser, value, page


def test_context_binding_lock_is_created_on_registration_loop_after_a_previous_loop_closed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])

    class ConcurrentBindings(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.active = 0
            self.peak = 0

        async def _record(self, owner_id: str, name: str, needs_handle: bool) -> None:
            self.active += 1
            self.peak = max(self.peak, self.active)
            await asyncio.sleep(0)
            self.registrations.append((owner_id, name, needs_handle))
            self.active -= 1

        async def context_expose_binding(
            self, context_id: str, name: str, needs_handle: bool
        ) -> None:
            await self._record(context_id, name, needs_handle)

        async def page_expose_binding(self, page_id: str, name: str, needs_handle: bool) -> None:
            await self._record(page_id, name, needs_handle)

    asyncio.run(asyncio.sleep(0))
    native = ConcurrentBindings()
    _browser, value, page = context(native)
    assert value._binding_registration_lock is None

    async def register_on_owner_loop() -> None:
        await asyncio.gather(
            value.expose_function("from-context", lambda: None),
            page.expose_function("from-page", lambda: None),
        )

    # Act
    asyncio.run(register_on_owner_loop())

    # Assert: neither side can use a constructor-bound or independent lock.
    assert value._binding_registration_lock is not None
    assert native.registrations == [("7", "from-context", False), ("9", "from-page", False)]
    assert native.peak == 1


def test_sync_browser_contexts_materializes_on_caller_without_current_loop() -> None:
    # Arrange: sync Browser.contexts reads the async property on its caller.
    class NativeContexts(NativeBindings):
        def contexts(self) -> list[tuple[str, None]]:
            return [("8", None)]

    asyncio.run(asyncio.sleep(0))
    browser, _value, _page = context(NativeContexts())
    sync_browser = SyncBrowser(cast(Any, object()), browser, cast(Any, object()))

    # Act
    contexts = sync_browser.contexts

    # Assert: Context creation does not create feature-only loop resources.
    assert len(contexts) == 1
    assert contexts[0]._value._object_id == "8"
    assert contexts[0]._value._binding_registration_lock is None


@pytest.mark.asyncio
async def test_context_binding_registration_forwards_handle_mode(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])
    native = NativeBindings()
    _browser, value, _page = context(native)

    # Act
    def take(_source: object, _handle: object) -> None:
        return None

    await value.expose_binding("take", take, handle=True)

    # Assert
    assert native.registrations == [("7", "take", True)]
    assert value._bindings["take"][1] is True


@pytest.mark.asyncio
async def test_context_function_registration_rejects_duplicate_exactly(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])
    native = NativeBindings()
    _browser, value, _page = context(native)
    await value.expose_function("foo", lambda: None)

    # Act
    with pytest.raises(async_api.Error) as error:
        await value.expose_function("foo", lambda: None)

    # Assert
    assert error.value.message == 'Function "foo" has been already registered'
    assert native.registrations == [("7", "foo", False)]


@pytest.mark.asyncio
async def test_context_binding_failed_native_registration_rolls_back_callback(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])
    native = NativeBindings()
    native.failure = async_api.Error("registration failed")
    _browser, value, _page = context(native)

    # Act
    with pytest.raises(async_api.Error, match="registration failed"):
        await value.expose_function("foo", lambda: None)

    # Assert
    assert value._bindings == {}


@pytest.mark.asyncio
async def test_context_binding_caller_cancellation_keeps_callback_until_native_commit(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])

    class DelayedNative(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.started = asyncio.Event()
            self.release = asyncio.Event()

        async def context_expose_binding(
            self, context_id: str, name: str, needs_handle: bool
        ) -> None:
            self.started.set()
            await self.release.wait()
            await super().context_expose_binding(context_id, name, needs_handle)

    native = DelayedNative()
    _browser, value, _page = context(native)
    registering = asyncio.create_task(value.expose_function("slow", lambda: 7))
    await asyncio.wait_for(native.started.wait(), timeout=1)

    # Act
    registering.cancel()
    with pytest.raises(asyncio.CancelledError):
        await registering
    assert "slow" in value._bindings
    native.release.set()
    registration_lock = value._binding_registration_lock
    assert registration_lock is not None
    await asyncio.wait_for(registration_lock.acquire(), timeout=1)
    registration_lock.release()

    # Assert
    assert native.registrations == [("7", "slow", False)]
    assert "slow" in value._bindings


@pytest.mark.asyncio
async def test_context_binding_caller_cancellation_rolls_back_native_failure(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])

    class DelayedNative(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.started = asyncio.Event()
            self.release = asyncio.Event()

        async def context_expose_binding(
            self, context_id: str, name: str, needs_handle: bool
        ) -> None:
            self.started.set()
            await self.release.wait()
            await super().context_expose_binding(context_id, name, needs_handle)

    native = DelayedNative()
    native.failure = async_api.Error("native registration failed")
    _browser, value, _page = context(native)
    registering = asyncio.create_task(value.expose_function("slow", lambda: 7))
    await asyncio.wait_for(native.started.wait(), timeout=1)

    # Act
    registering.cancel()
    with pytest.raises(asyncio.CancelledError):
        await registering
    native.release.set()
    registration_lock = value._binding_registration_lock
    assert registration_lock is not None
    await asyncio.wait_for(registration_lock.acquire(), timeout=1)
    registration_lock.release()

    # Assert
    assert value._bindings == {}
    assert native.registrations == []


@pytest.mark.asyncio
async def test_page_binding_caller_cancellation_keeps_callback_until_native_commit() -> None:
    # Arrange
    class DelayedNative(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.started = asyncio.Event()
            self.release = asyncio.Event()

        async def page_expose_binding(self, page_id: str, name: str, needs_handle: bool) -> None:
            self.started.set()
            await self.release.wait()
            self.registrations.append((page_id, name, needs_handle))

    native = DelayedNative()
    _browser, value, page = context(native)
    registering = asyncio.create_task(page.expose_function("slow", lambda: 7))
    await asyncio.wait_for(native.started.wait(), timeout=1)

    # Act
    registering.cancel()
    with pytest.raises(asyncio.CancelledError):
        await registering
    assert "slow" in page._bindings
    native.release.set()
    registration_lock = value._binding_registration_lock
    assert registration_lock is not None
    await asyncio.wait_for(registration_lock.acquire(), timeout=1)
    registration_lock.release()

    # Assert
    assert native.registrations == [("9", "slow", False)]
    assert "slow" in page._bindings


@pytest.mark.asyncio
async def test_binding_registration_admission_is_shared_and_survives_caller_cancellation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])

    class GatedBindings(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.started = asyncio.Event()
            self.release = asyncio.Event()
            self.entered: list[str] = []

        async def context_expose_binding(
            self, context_id: str, name: str, needs_handle: bool
        ) -> None:
            self.entered.append(name)
            self.started.set()
            await self.release.wait()
            await super().context_expose_binding(context_id, name, needs_handle)

        async def page_expose_binding(self, page_id: str, name: str, needs_handle: bool) -> None:
            await self.context_expose_binding(page_id, name, needs_handle)

    native = GatedBindings()
    browser, value, page = context(native)
    other = BrowserContext(browser, "8", None)
    browser._contexts["8"] = other
    first = asyncio.create_task(value.expose_function("first", lambda: None))
    await asyncio.wait_for(native.started.wait(), timeout=1)
    first.cancel()
    with pytest.raises(asyncio.CancelledError):
        await first

    # Act: detached transactions keep their slots even while waiting for the
    # same Context lock; alternating Page/Context callers cannot bypass it.
    try:
        for index in range(1, 64):
            owner = page if index % 2 else value
            caller = asyncio.create_task(owner.expose_function(f"queued-{index}", lambda: None))
            await asyncio.sleep(0)
            caller.cancel()
            with pytest.raises(asyncio.CancelledError):
                await caller

        # Assert: admission precedes coroutine construction and applies across
        # all Contexts owned by this Browser, before any native FFI admission.
        assert len(browser._binding_registration_tasks) == 64
        assert native.entered == ["first"]
        factory_called = False

        async def rejected_operation() -> None:
            raise AssertionError("rejected transaction must not run")

        def rejected_factory() -> Awaitable[None]:
            nonlocal factory_called
            factory_called = True
            return rejected_operation()

        with pytest.raises(async_api.ResourceExhaustedError, match="registration capacity"):
            await async_api._shield_binding_registration(browser, rejected_factory, "test")
        assert not factory_called
        for index in range(12):
            denied_owner = (page, value, other)[index % 3]
            with pytest.raises(async_api.ResourceExhaustedError, match="registration capacity"):
                await denied_owner.expose_function(f"denied-{index}", lambda: None)
        assert len(browser._binding_registration_tasks) == 64
        assert native.entered == ["first"]
        assert other._bindings == {}
        assert not any(name.startswith("denied-") for name in value._bindings | page._bindings)
    finally:
        admitted = tuple(browser._binding_registration_tasks)
        native.release.set()
        await asyncio.wait_for(asyncio.gather(*admitted), timeout=1)

    assert browser._binding_registration_tasks == set()
    await other.expose_function("after-release", lambda: None)
    assert browser._binding_registration_tasks == set()
    assert native.registrations[-1] == ("8", "after-release", False)


@pytest.mark.asyncio
async def test_canceled_binding_registration_slots_release_after_native_failure(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])

    class FailingBindings(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.started = asyncio.Event()
            self.release = asyncio.Event()

        async def context_expose_binding(
            self, context_id: str, name: str, needs_handle: bool
        ) -> None:
            self.started.set()
            await self.release.wait()
            await super().context_expose_binding(context_id, name, needs_handle)

        async def page_expose_binding(self, page_id: str, name: str, needs_handle: bool) -> None:
            await self.context_expose_binding(page_id, name, needs_handle)

    native = FailingBindings()
    native.failure = async_api.Error("native registration failed")
    browser, value, page = context(native)
    loop = asyncio.get_running_loop()
    old_handler = loop.get_exception_handler()
    reported: list[dict[str, object]] = []
    loop.set_exception_handler(lambda _loop, error: reported.append(error))
    first = asyncio.create_task(value.expose_function("context-failed", lambda: None))
    await asyncio.wait_for(native.started.wait(), timeout=1)
    first.cancel()
    with pytest.raises(asyncio.CancelledError):
        await first
    second = asyncio.create_task(page.expose_function("page-failed", lambda: None))
    await asyncio.sleep(0)
    second.cancel()
    with pytest.raises(asyncio.CancelledError):
        await second
    assert len(browser._binding_registration_tasks) == 2

    # Act
    try:
        admitted = tuple(browser._binding_registration_tasks)
        native.release.set()
        outcomes = await asyncio.wait_for(
            asyncio.gather(*admitted, return_exceptions=True), timeout=1
        )

        # Assert: caller cancellation is not release; completed native rollback is.
        assert all(isinstance(outcome, async_api.Error) for outcome in outcomes)
        assert len(reported) == 2
        assert browser._binding_registration_tasks == set()
        assert value._bindings == {}
        assert page._bindings == {}
        native.failure = None
        await page.expose_function("page-failed", lambda: None)
        assert browser._binding_registration_tasks == set()
        assert native.registrations == [("9", "page-failed", False)]
    finally:
        native.release.set()
        await asyncio.gather(*browser._binding_registration_tasks, return_exceptions=True)
        loop.set_exception_handler(old_handler)


@pytest.mark.asyncio
async def test_context_binding_requires_opt_in_native_feature(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: cast(list[str], []))
    native = NativeBindings()
    _browser, value, _page = context(native)

    # Act
    with pytest.raises(async_api.UnsupportedError, match="capability is not installed"):
        await value.expose_function("foo", lambda: None)

    # Assert
    assert native.registrations == []
    assert value._bindings == {}


@pytest.mark.asyncio
async def test_context_function_dispatch_resolves_page_and_frame_source() -> None:
    # Arrange
    native = NativeBindings()
    browser, value, page = context(native)
    sources: list[object] = []

    def add(source: object, left: int, right: int) -> int:
        sources.append(source)
        return left + right

    value._bindings["add"] = (add, False)

    # Act
    await browser._dispatch_page_binding(("5", "7", "9", "11", "add", "[5,6]", False))

    # Assert
    assert native.acknowledged == ["5"]
    assert native.results == [("5", 11)]
    assert sources == [{"context": value, "page": page, "frame": page.main_frame}]


@pytest.mark.asyncio
async def test_retired_page_call_does_not_stop_other_binding_dispatch() -> None:
    # Arrange
    class RetiredNative(NativeBindings):
        async def acknowledge_page_binding_call(self, call_id: str) -> None:
            raise async_api.InvalidArgumentError(f"unknown retired call {call_id}")

    native = RetiredNative()
    browser, value, _page = context(native)
    value._pages.clear()
    value._closed = True

    # Act: Rust may retire a call after the watcher has dequeued it.
    await browser._dispatch_page_binding(("5", "7", "9", "11", "old", "[]", False))

    # Assert: the watcher may continue with calls from other live Pages.
    assert native.acknowledged == []


@pytest.mark.asyncio
async def test_binding_watcher_dequeues_nested_async_call_before_outer_result() -> None:
    # Arrange
    class QueuedNative(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.calls: asyncio.Queue[tuple[str, str, str, str, str, str, bool] | None] = (
                asyncio.Queue()
            )
            self.finished = asyncio.Event()

        async def next_page_binding_call(
            self,
        ) -> tuple[str, str, str, str, str, str, bool] | None:
            return await self.calls.get()

        async def deliver_page_binding_result(self, call_id: str, value: object) -> None:
            await super().deliver_page_binding_result(call_id, value)
            if len(self.results) == 2:
                self.finished.set()

    native = QueuedNative()
    browser, value, _page = context(native)
    browser._closed = False
    browser._binding_dispatch_tasks = set()
    inner_finished = asyncio.Event()

    async def outer(_source: object) -> str:
        await inner_finished.wait()
        return "outer-result"

    def inner(_source: object) -> str:
        inner_finished.set()
        return "inner-result"

    value._bindings["outer"] = (outer, False)
    value._bindings["inner"] = (inner, False)
    native.calls.put_nowait(("1", "7", "9", "11", "outer", "[]", False))
    native.calls.put_nowait(("2", "7", "9", "11", "inner", "[]", False))
    browser._binding_task = asyncio.create_task(browser._watch_page_bindings())

    # Act
    try:
        await asyncio.wait_for(native.finished.wait(), timeout=1)
    finally:
        browser._closed = True
        native.calls.put_nowait(None)
        await browser._finish_binding_watcher()

    # Assert
    assert set(native.acknowledged) == {"1", "2"}
    assert set(native.results) == {("1", "outer-result"), ("2", "inner-result")}
    assert browser._binding_dispatch_tasks == set()


@pytest.mark.asyncio
async def test_mixed_scope_functions_register_and_settle_sequential_awaits(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange: model the official Context add -> Page mul -> Context sub order.
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])
    call_ids = ("1", "2", "3")
    stages = (
        *(("registration", name) for name in ("add", "mul", "sub")),
        *((stage, call_id) for call_id in call_ids for stage in ("dequeue", "ack", "delivery")),
    )

    class StagedNative(QueuedBindings):
        def __init__(self) -> None:
            super().__init__()
            loop = asyncio.get_running_loop()
            self.entered = {stage: loop.create_future() for stage in stages}
            self.release = {stage: loop.create_future() for stage in stages}
            self.settled: dict[str, asyncio.Future[object]] = {
                call_id: loop.create_future() for call_id in call_ids
            }

        async def _gate(self, stage: str, key: str) -> None:
            self.entered[(stage, key)].set_result(None)
            await self.release[(stage, key)]

        async def context_expose_binding(
            self, context_id: str, name: str, needs_handle: bool
        ) -> None:
            await self._gate("registration", name)
            await super().context_expose_binding(context_id, name, needs_handle)

        async def page_expose_binding(self, page_id: str, name: str, needs_handle: bool) -> None:
            await self._gate("registration", name)
            self.registrations.append((page_id, name, needs_handle))

        async def next_page_binding_call(
            self,
        ) -> tuple[str, str, str, str, str, str, bool] | None:
            call = await super().next_page_binding_call()
            if call is not None:
                await self._gate("dequeue", call[0])
            return call

        async def acknowledge_page_binding_call(self, call_id: str) -> None:
            await self._gate("ack", call_id)
            await super().acknowledge_page_binding_call(call_id)

        async def deliver_page_binding_result(self, call_id: str, value: object) -> None:
            await self._gate("delivery", call_id)
            await super().deliver_page_binding_result(call_id, value)
            self.settled[call_id].set_result(value)

        def pages(self, _context_id: str) -> list[tuple[str, str, str, str]]:
            return [("9", "7", "11", "about:blank")]

        async def evaluate(self, frame_id: str, expression: str, _arg: object) -> object:
            assert frame_id == "11"
            assert expression == "async () => [await mul(3, 4), await add(3, 4), await sub(3, 4)]"
            values: list[object] = []
            for call_id, name in zip(call_ids, ("mul", "add", "sub")):
                self.calls.put_nowait((call_id, "7", "9", "11", name, "[3,4]", False))
                values.append(await self.settled[call_id])
            return values

    native = StagedNative()
    browser, value, page = context(native)
    browser._binding_task = asyncio.create_task(browser._watch_page_bindings())
    registrations: list[asyncio.Task[None]] = []
    evaluation: asyncio.Task[object] | None = None

    def add(left: int, right: int) -> int:
        return left + right

    def mul(left: int, right: int) -> int:
        return left * right

    def sub(left: int, right: int) -> int:
        return left - right

    async def reached(stage: str, key: str) -> None:
        try:
            await asyncio.wait_for(asyncio.shield(native.entered[(stage, key)]), timeout=1)
        except asyncio.TimeoutError:
            pytest.fail(f"binding stalled before {stage} for {key}")

    def release(stage: str, key: str) -> None:
        native.release[(stage, key)].set_result(None)

    # Act: each gate identifies whether registration, dequeue, ack, or delivery stalls.
    try:
        for name, registration in (
            ("add", value.expose_function("add", add)),
            ("mul", page.expose_function("mul", mul)),
            ("sub", value.expose_function("sub", sub)),
        ):
            task = asyncio.create_task(registration)
            registrations.append(task)
            await reached("registration", name)
            assert not task.done()
            release("registration", name)
            await asyncio.wait_for(task, timeout=1)

        assert native.registrations == [
            ("7", "add", False),
            ("9", "mul", False),
            ("7", "sub", False),
        ]
        evaluation = asyncio.create_task(
            page.evaluate("async () => [await mul(3, 4), await add(3, 4), await sub(3, 4)]")
        )
        for call_id in call_ids:
            await reached("dequeue", call_id)
            assert not native.settled[call_id].done()
            release("dequeue", call_id)
            await reached("ack", call_id)
            assert not native.settled[call_id].done()
            release("ack", call_id)
            await reached("delivery", call_id)
            assert not evaluation.done()
            release("delivery", call_id)
            await asyncio.wait_for(asyncio.shield(native.settled[call_id]), timeout=1)

        # Assert: each JavaScript-style await settles before the next call is queued.
        assert await asyncio.wait_for(evaluation, timeout=1) == [12, 7, -1]
        assert native.acknowledged == ["1", "2", "3"]
        assert native.results == [("1", 12), ("2", 7), ("3", -1)]
        assert native.errors == []
        assert browser._binding_watch_error is None
    finally:
        for gate in native.release.values():
            if not gate.done():
                gate.set_result(None)
        if evaluation is not None:
            if not evaluation.done():
                evaluation.cancel()
            await asyncio.gather(evaluation, return_exceptions=True)
        await asyncio.gather(*registrations, return_exceptions=True)
        await asyncio.gather(*browser._binding_registration_tasks, return_exceptions=True)
        browser._closed = True
        native.calls.put_nowait(None)
        await browser._finish_binding_watcher()


@pytest.mark.asyncio
async def test_binding_watcher_cancels_pending_callbacks_at_close() -> None:
    # Arrange
    class QueuedNative(NativeBindings):
        def __init__(self) -> None:
            super().__init__()
            self.calls: asyncio.Queue[tuple[str, str, str, str, str, str, bool] | None] = (
                asyncio.Queue()
            )
            self.acknowledged_call = asyncio.Event()

        async def next_page_binding_call(
            self,
        ) -> tuple[str, str, str, str, str, str, bool] | None:
            return await self.calls.get()

        async def acknowledge_page_binding_call(self, call_id: str) -> None:
            await super().acknowledge_page_binding_call(call_id)
            self.acknowledged_call.set()

    native = QueuedNative()
    browser, value, _page = context(native)
    browser._closed = False
    browser._binding_dispatch_tasks = set()
    blocker = asyncio.Event()

    async def pending(_source: object) -> None:
        await blocker.wait()

    value._bindings["pending"] = (pending, False)
    native.calls.put_nowait(("1", "7", "9", "11", "pending", "[]", False))
    browser._binding_task = asyncio.create_task(browser._watch_page_bindings())
    await asyncio.wait_for(native.acknowledged_call.wait(), timeout=1)

    # Act
    browser._closed = True
    native.calls.put_nowait(None)
    await browser._finish_binding_watcher()

    # Assert
    assert browser._binding_dispatch_tasks == set()
    assert native.results == []


@pytest.mark.asyncio
async def test_binding_close_cancels_callback_while_acknowledgement_is_pending() -> None:
    # Arrange
    class PendingAcknowledgement(QueuedBindings):
        def __init__(self) -> None:
            super().__init__()
            self.ack_started = asyncio.Event()
            self.ack_release = asyncio.Event()
            self.attempts = 0

        async def acknowledge_page_binding_call(self, call_id: str) -> None:
            self.attempts += 1
            if self.attempts == 1:
                self.ack_started.set()
                await self.ack_release.wait()
            else:
                raise async_api.TargetClosedError("closed during acknowledgement")

    native = PendingAcknowledgement()
    browser, value, _page = context(native)
    stopped = asyncio.Event()
    blocker = asyncio.Event()

    async def pending(_source: object) -> None:
        try:
            await blocker.wait()
        finally:
            stopped.set()

    value._bindings["pending"] = (pending, False)
    native.calls.put_nowait(("1", "7", "9", "11", "pending", "[]", False))
    browser._binding_task = asyncio.create_task(browser._watch_page_bindings())
    await asyncio.wait_for(native.ack_started.wait(), timeout=1)

    # Act
    browser._closed = True
    native.calls.put_nowait(None)
    await asyncio.wait_for(browser._finish_binding_watcher(), timeout=1)

    # Assert
    assert stopped.is_set()
    assert browser._binding_dispatch_tasks == set()
    assert native.results == []


@pytest.mark.asyncio
async def test_binding_acknowledgement_failure_cancels_its_callback_child() -> None:
    # Arrange
    class FailedAcknowledgement(NativeBindings):
        async def acknowledge_page_binding_call(self, call_id: str) -> None:
            raise async_api.InvalidArgumentError(f"unknown retired call {call_id}")

    native = FailedAcknowledgement()
    browser, value, _page = context(native)
    stopped = asyncio.Event()
    blocker = asyncio.Event()

    async def pending(_source: object) -> None:
        try:
            await blocker.wait()
        finally:
            stopped.set()

    value._bindings["pending"] = (pending, False)

    # Act
    await asyncio.wait_for(
        browser._dispatch_page_binding(("1", "7", "9", "11", "pending", "[]", False)),
        timeout=1,
    )

    # Assert
    assert stopped.is_set()
    assert native.results == []


@pytest.mark.asyncio
async def test_binding_dispatch_cap_survives_native_page_retirement() -> None:
    # Arrange
    class OverflowBindings(QueuedBindings):
        def __init__(self) -> None:
            super().__init__()
            self.rejected = asyncio.Event()

        async def deliver_page_binding_error(
            self, call_id: str, name: str, message: str, stack: str | None
        ) -> None:
            await super().deliver_page_binding_error(call_id, name, message, stack)
            if call_id == "65":
                self.rejected.set()

    native = OverflowBindings()
    browser, value, _page = context(native)
    started = 0
    all_started = asyncio.Event()
    blocker = asyncio.Event()

    async def pending(_source: object) -> None:
        nonlocal started
        started += 1
        if started == 64:
            all_started.set()
        await blocker.wait()

    value._bindings["pending"] = (pending, False)
    for call_id in range(1, 65):
        native.calls.put_nowait((str(call_id), "7", "9", "11", "pending", "[]", False))
    browser._binding_task = asyncio.create_task(browser._watch_page_bindings())
    await asyncio.wait_for(all_started.wait(), timeout=1)

    # Act: native retirement released all 64 pending entries, but their Python
    # callbacks are still alive when a replacement Page invokes the binding.
    value._pages.clear()
    value._pages["10"] = Page(value, "10", "11", "about:blank")
    native.calls.put_nowait(("65", "7", "10", "11", "pending", "[]", False))
    try:
        await asyncio.wait_for(native.rejected.wait(), timeout=1)

        # Assert
        assert started == 64
        assert len(browser._binding_dispatch_tasks) == 64
        assert native.errors == [("65", "pending binding dispatch capacity exceeded (64)")]
    finally:
        browser._closed = True
        native.calls.put_nowait(None)
        await asyncio.wait_for(browser._finish_binding_watcher(), timeout=1)
    assert browser._binding_dispatch_tasks == set()


@pytest.mark.asyncio
@pytest.mark.parametrize("fail_ack", [False, True])
async def test_binding_callback_can_close_browser_during_acknowledgement(fail_ack: bool) -> None:
    # Arrange
    class SelfClosingBindings(QueuedBindings):
        def __init__(self) -> None:
            super().__init__()
            self.close_started = asyncio.Event()
            self.close_release = asyncio.Event()
            self.ack_observed = asyncio.Event()

        async def close(self) -> None:
            self.close_started.set()
            await self.close_release.wait()
            await super().close()

        async def acknowledge_page_binding_call(self, call_id: str) -> None:
            self.ack_observed.set()
            if fail_ack:
                raise async_api.TargetClosedError("callback initiated closure")
            await super().acknowledge_page_binding_call(call_id)

        async def deliver_page_binding_result(self, call_id: str, value: object) -> None:
            raise async_api.TargetClosedError("callback closed its Page")

    native = SelfClosingBindings()
    browser, value, _page = context(native)
    close_returned = asyncio.Event()

    async def close_from_callback(_source: object) -> None:
        await browser.close()
        close_returned.set()

    value._bindings["close"] = (close_from_callback, False)
    native.calls.put_nowait(("1", "7", "9", "11", "close", "[]", False))
    browser._binding_task = asyncio.create_task(browser._watch_page_bindings())
    await asyncio.wait_for(native.close_started.wait(), timeout=1)
    await asyncio.wait_for(native.ack_observed.wait(), timeout=1)
    dispatches = tuple(browser._binding_dispatch_tasks)

    # Act
    native.close_release.set()
    await asyncio.wait_for(asyncio.gather(*dispatches), timeout=1)

    # Assert
    assert close_returned.is_set()
    assert value._closed
    assert browser._contexts == {}
    assert browser._binding_dispatch_tasks == set()
    assert browser._binding_closing_dispatch is None


@pytest.mark.asyncio
async def test_binding_callback_closing_another_browser_keeps_marker_scoped() -> None:
    # Arrange
    class ResultBindings(QueuedBindings):
        def __init__(self) -> None:
            super().__init__()
            self.delivered = asyncio.Event()

        async def deliver_page_binding_result(self, call_id: str, value: object) -> None:
            await super().deliver_page_binding_result(call_id, value)
            self.delivered.set()

    native_a = ResultBindings()
    native_b = QueuedBindings()
    browser_a, context_a, _page_a = context(native_a)
    browser_b, context_b, _page_b = context(native_b)
    started_b = asyncio.Event()
    stopped_b = asyncio.Event()
    blocker = asyncio.Event()

    async def pending_b(_source: object) -> None:
        started_b.set()
        try:
            await blocker.wait()
        finally:
            stopped_b.set()

    async def close_other(_source: object) -> str:
        await browser_b.close()
        return "closed-other"

    context_a._bindings["close-other"] = (close_other, False)
    context_b._bindings["pending"] = (pending_b, False)
    native_b.calls.put_nowait(("2", "7", "9", "11", "pending", "[]", False))
    browser_b._binding_task = asyncio.create_task(browser_b._watch_page_bindings())
    await asyncio.wait_for(started_b.wait(), timeout=1)
    browser_a._binding_task = asyncio.create_task(browser_a._watch_page_bindings())

    # Act
    try:
        native_a.calls.put_nowait(("1", "7", "9", "11", "close-other", "[]", False))
        await asyncio.wait_for(native_a.delivered.wait(), timeout=1)
        await asyncio.gather(*browser_a._binding_dispatch_tasks)

        # Assert: B retires its own dispatches, never retaining or excluding A's
        # dispatcher, while A stays live and can publish its callback result.
        assert stopped_b.is_set()
        assert context_b._closed
        assert browser_b._contexts == {}
        assert browser_b._binding_dispatch_tasks == set()
        assert browser_b._binding_closing_dispatch is None
        assert browser_a._binding_closing_dispatch is None
        assert not browser_a._closed
        assert browser_a._contexts == {"7": context_a}
        assert native_a.results == [("1", "closed-other")]
    finally:
        blocker.set()
        await browser_a.close()
        await browser_b.close()


@pytest.mark.asyncio
async def test_binding_awaitable_cancellation_keeps_ownership_before_helper_returns() -> None:
    # Arrange
    stopped = asyncio.Event()
    blocker = asyncio.Event()

    async def pending() -> None:
        try:
            await blocker.wait()
        finally:
            stopped.set()

    starter = asyncio.create_task(async_api._start_binding_awaitable(pending()))
    # The starter creates its child and suspends before this task resumes.
    await asyncio.sleep(0)

    # Act
    starter.cancel()
    with pytest.raises(asyncio.CancelledError):
        await starter

    # Assert
    assert stopped.is_set()


@pytest.mark.asyncio
async def test_binding_close_is_bounded_when_callback_resists_cancellation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api, "_BINDING_CLOSE_TIMEOUT_SECONDS", 0.01)
    native = QueuedBindings()
    browser, value, _page = context(native)
    started = asyncio.Event()
    resisted = asyncio.Event()
    stopped = asyncio.Event()
    release = asyncio.Event()

    async def resistant(_source: object) -> None:
        started.set()
        try:
            await release.wait()
        except asyncio.CancelledError:
            resisted.set()
            await release.wait()
        finally:
            stopped.set()

    value._bindings["resistant"] = (resistant, False)
    native.calls.put_nowait(("1", "7", "9", "11", "resistant", "[]", False))
    browser._binding_task = asyncio.create_task(browser._watch_page_bindings())
    await asyncio.wait_for(started.wait(), timeout=1)
    dispatches = tuple(browser._binding_dispatch_tasks)

    # Act
    try:
        await asyncio.wait_for(browser.close(), timeout=1)

        # Assert: close finishes its Context cleanup, while the callback retains
        # its admission slot and reports that user code has not stopped yet.
        assert resisted.is_set()
        assert not stopped.is_set()
        assert value._closed
        assert browser._contexts == {}
        assert len(browser._binding_dispatch_tasks) == 1
        assert str(browser._binding_watch_error) == (
            "Binding callbacks did not stop after Browser closure"
        )
    finally:
        release.set()
        await asyncio.wait_for(asyncio.gather(*dispatches), timeout=1)
    assert stopped.is_set()
    assert browser._binding_dispatch_tasks == set()


def test_sync_context_binding_wraps_source_objects(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-bindings"])
    native = NativeBindings()
    _browser, value, page = context(native)

    class Bridge:
        def run(self, awaitable: Any) -> Any:
            return asyncio.run(awaitable)

    sync_value = object.__new__(SyncBrowserContext)
    sync_value._bridge = cast(Any, Bridge())
    sync_value._value = value
    sync_value._pages = {}
    sources: list[object] = []

    def binding(source: object, arg: int) -> int:
        sources.append(source)
        return arg + 1

    # Act
    sync_value.expose_binding("increment", binding)
    callback, _needs_handle = value._bindings["increment"]
    result = callback({"context": value, "page": page, "frame": page.main_frame}, 3)

    # Assert
    assert result == 4
    assert native.registrations == [("7", "increment", False)]
    assert isinstance(sources[0], dict)
    source = cast(dict[str, object], sources[0])
    sync_page = sync_value._wrap_page(page)
    assert source["context"] is sync_value
    assert source["page"] is sync_page
    assert source["frame"] is sync_page.main_frame
