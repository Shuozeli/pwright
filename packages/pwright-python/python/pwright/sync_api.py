# pyright: reportPrivateUsage=false
"""Sync facade backed by one bounded asyncio loop thread per Playwright owner."""

from __future__ import annotations

import asyncio
import concurrent.futures
import inspect
import threading
import traceback
import weakref
from collections import deque
from collections.abc import Awaitable, Callable, Coroutine, Mapping, Sequence
from os import PathLike
from pathlib import Path
from re import Pattern
from typing import Any, Generic, Literal, TypeVar, Union, cast

from greenlet import getcurrent

from . import async_api
from ._errors import Error, InvalidArgumentError, ResourceExhaustedError, UnsupportedError
from ._sync_api_request import (
    APIRequest,
    APIRequestContext,
    APIResponse,
    ClientCertificate,
    StorageState,
    StorageStateCookie,
    _run_callback_safe,
)

T = TypeVar("T")
RequestSizes = async_api.RequestSizes
SecurityDetails = async_api.SecurityDetails
ResourceTiming = async_api.ResourceTiming
SourceLocation = async_api.SourceLocation
DeviceDescriptor = async_api.DeviceDescriptor
ViewportSize = async_api.ViewportSize
FileInput = async_api.FileInput
FilePayload = async_api.FilePayload
Cookie = async_api.Cookie
SetCookieParam = async_api.SetCookieParam

_SYNC_PLAYWRIGHT_ACTIVE_THREADS: set[int] = set()
_SYNC_PLAYWRIGHT_ACTIVE_THREADS_LOCK = threading.Lock()
_SYNC_PLAYWRIGHT_LOOP_ERROR = (
    "It looks like you are using Playwright Sync API inside the asyncio loop. "
    "Please use the Async API instead."
)


def _claim_sync_playwright_thread() -> int:
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        pass
    else:
        raise Error(_SYNC_PLAYWRIGHT_LOOP_ERROR)
    thread_id = threading.get_ident()
    with _SYNC_PLAYWRIGHT_ACTIVE_THREADS_LOCK:
        if thread_id in _SYNC_PLAYWRIGHT_ACTIVE_THREADS:
            raise Error(_SYNC_PLAYWRIGHT_LOOP_ERROR)
        _SYNC_PLAYWRIGHT_ACTIVE_THREADS.add(thread_id)
    return thread_id


def _release_sync_playwright_thread(thread_id: int) -> None:
    with _SYNC_PLAYWRIGHT_ACTIVE_THREADS_LOCK:
        _SYNC_PLAYWRIGHT_ACTIVE_THREADS.discard(thread_id)


class _CallerDispatcher:
    """Pump sync helper tasks on the owning thread without moving browser work there."""

    def __init__(self) -> None:
        self._owner_thread_id = threading.get_ident()
        self._loop = asyncio.new_event_loop()
        self._closed = False
        self._pending_waiters: set[Any] = set()
        # Playwright's sync fixtures schedule helper tasks on the same loop that
        # its dispatcher fiber pumps. Keep that observable contract while the
        # actual Pwright/browser coroutines stay on the bounded background loop.
        asyncio._set_running_loop(self._loop)

    def can_suspend_current(self) -> bool:
        return (
            not self._closed
            and threading.get_ident() == self._owner_thread_id
            and getcurrent().parent is not None
        )

    def is_owner_thread(self) -> bool:
        return threading.get_ident() == self._owner_thread_id

    def schedule(self, callback: Callable[[], None]) -> None:
        if self._closed:
            return
        try:
            self._loop.call_soon_threadsafe(callback)
        except RuntimeError:
            # A concurrently stopped owner cannot receive further callbacks.
            pass

    def suspend_until(self, future: concurrent.futures.Future[Any]) -> None:
        if not self.can_suspend_current():
            raise RuntimeError("Pwright caller dispatcher cannot suspend this greenlet")
        waiter = getcurrent()
        parent = waiter.parent
        assert parent is not None
        self._pending_waiters.add(waiter)

        def schedule_resume(_future: concurrent.futures.Future[Any]) -> None:
            try:
                self._loop.call_soon_threadsafe(self._resume, waiter)
            except RuntimeError:
                # Closing the dispatcher retires all of its caller-side work.
                pass

        future.add_done_callback(schedule_resume)
        parent.switch()

    def _resume(self, waiter: Any) -> None:
        try:
            waiter.switch()
        finally:
            self._pending_waiters.discard(waiter)
            self._loop.stop()

    def switch(self) -> None:
        if threading.get_ident() != self._owner_thread_id:
            raise RuntimeError("Pwright caller dispatcher belongs to another thread")
        if self._closed:
            raise Error("Event loop is closed! Is Playwright already stopped?")
        if not self._pending_waiters:
            # Give newly scheduled caller-side helper tasks one turn to create
            # their greenlet waits. Subsequent switches wait for a real result.
            self._loop.call_soon(self._loop.stop)
        asyncio._set_running_loop(None)
        try:
            self._loop.run_forever()
        finally:
            if not self._closed:
                asyncio._set_running_loop(self._loop)

    def close(self) -> None:
        if self._closed:
            return
        if threading.get_ident() != self._owner_thread_id:
            raise RuntimeError("Pwright caller dispatcher belongs to another thread")
        self._closed = True
        try:
            running_loop = asyncio.get_running_loop()
        except RuntimeError:
            running_loop = None
        if running_loop is self._loop:
            asyncio._set_running_loop(None)
        pending = asyncio.all_tasks(self._loop)
        for task in pending:
            task.cancel()
        if pending:
            self._loop.run_until_complete(asyncio.gather(*pending, return_exceptions=True))
        self._loop.run_until_complete(self._loop.shutdown_asyncgens())
        self._loop.close()


class _LoopThread:
    def __init__(self) -> None:
        self._caller_thread_id = threading.get_ident()
        self._ready = threading.Event()
        self._loop: asyncio.AbstractEventLoop | None = None
        self._thread_id: int | None = None
        self._admission = threading.BoundedSemaphore(32)
        self._event_error_lock = threading.Lock()
        self._event_error: BaseException | None = None
        self._remote_event_lock = threading.Lock()
        self._remote_events: deque[tuple[Callable[[object], object], object]] = deque()
        self._remote_event_scheduled = False
        self._caller_dispatcher: _CallerDispatcher | None = None
        self._thread = threading.Thread(target=self._run, name="pwright-sync-loop", daemon=True)
        self._thread.start()
        if not self._ready.wait(timeout=10):
            raise RuntimeError("Pwright sync event loop did not start within 10 seconds")

    def _run(self) -> None:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        self._loop = loop
        self._thread_id = threading.get_ident()
        self._ready.set()
        loop.run_forever()
        pending = asyncio.all_tasks(loop)
        for task in pending:
            task.cancel()
        if pending:
            loop.run_until_complete(asyncio.gather(*pending, return_exceptions=True))
        loop.close()

    def run(self, coroutine: Coroutine[Any, Any, T]) -> T:
        if threading.get_ident() == self._thread_id:
            coroutine.close()
            raise RuntimeError("Pwright sync API cannot block its own event-loop thread")
        if self._loop is None:
            coroutine.close()
            raise Error("Event loop is closed! Is Playwright already stopped?")
        if not self._admission.acquire(blocking=False):
            coroutine.close()
            raise RuntimeError("Pwright sync pending-call capacity exceeded (32)")
        try:
            future: concurrent.futures.Future[T] = asyncio.run_coroutine_threadsafe(
                coroutine, self._loop
            )
            dispatcher = self._caller_dispatcher
            if dispatcher is not None and dispatcher.can_suspend_current():
                dispatcher.suspend_until(future)
            try:
                result = future.result()
            except BaseException as call_error:
                self._drain_remote_events()
                event_error = self._take_event_error()
                if event_error is not None:
                    self._attach_caller_stack(event_error)
                    raise event_error from call_error
                self._attach_caller_stack(call_error)
                raise
            self._drain_remote_events()
            event_error = self._take_event_error()
            if event_error is not None:
                self._attach_caller_stack(event_error)
                raise event_error
            return result
        finally:
            self._admission.release()

    @staticmethod
    def _attach_caller_stack(error: BaseException) -> None:
        if not isinstance(error, Error):
            return
        caller_stack = "".join(traceback.format_stack())
        existing_stack = getattr(error, "stack", "")
        setattr(error, "stack", caller_stack + existing_stack)

    def _record_event_error(self, error: BaseException) -> None:
        with self._event_error_lock:
            if self._event_error is None:
                self._event_error = error

    def _take_event_error(self) -> BaseException | None:
        with self._event_error_lock:
            error, self._event_error = self._event_error, None
            return error

    def queue_remote_event(self, handler: Callable[[object], object], value: object) -> None:
        dispatcher = self._caller_dispatcher
        with self._remote_event_lock:
            if len(self._remote_events) >= 256:
                self._record_event_error(
                    ResourceExhaustedError("Pwright sync remote event capacity exceeded (256)")
                )
                return
            self._remote_events.append((handler, value))
            if self._remote_event_scheduled:
                return
            self._remote_event_scheduled = True
        if dispatcher is not None:
            dispatcher.schedule(self._drain_remote_events)

    def _drain_remote_events(self) -> None:
        dispatcher = self._caller_dispatcher
        if dispatcher is not None and not dispatcher.is_owner_thread():
            return
        if dispatcher is None and threading.get_ident() != self._caller_thread_id:
            return
        while True:
            with self._remote_event_lock:
                if not self._remote_events:
                    self._remote_event_scheduled = False
                    return
                handler, value = self._remote_events.popleft()
            try:
                handler(value)
            except BaseException as error:
                self._record_event_error(error)

    def run_event_command(self, coroutine: Coroutine[Any, Any, None]) -> None:
        if threading.get_ident() == self._thread_id:
            asyncio.create_task(coroutine)
            return
        self.run(coroutine)

    def install_caller_dispatcher(self, dispatcher: _CallerDispatcher) -> None:
        if self._caller_dispatcher is not None:
            raise RuntimeError("Pwright caller dispatcher is already installed")
        self._caller_dispatcher = dispatcher

    def close(self) -> None:
        loop = self._loop
        if loop is None:
            return
        self._loop = None
        loop.call_soon_threadsafe(loop.stop)
        self._thread.join(timeout=10)
        if self._thread.is_alive():
            raise RuntimeError("Pwright sync event loop did not stop within 10 seconds")
        with self._remote_event_lock:
            self._remote_events.clear()
            self._remote_event_scheduled = False


class Request:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.Request,
        context: BrowserContext | None = None,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._context = context

    @property
    def url(self) -> str:
        return self._value.url

    @property
    def method(self) -> str:
        return self._value.method

    @property
    def resource_type(self) -> str:
        return self._value.resource_type

    @property
    def headers(self) -> dict[str, str]:
        return self._value.headers

    def all_headers(self) -> dict[str, str]:
        return self._bridge.run(self._value.all_headers())

    def header_value(self, name: str) -> str | None:
        return self._bridge.run(self._value.header_value(name))

    @property
    def post_data(self) -> str | None:
        return self._value.post_data

    @property
    def post_data_buffer(self) -> bytes | None:
        return self._value.post_data_buffer

    @property
    def post_data_json(self) -> object | None:
        return self._value.post_data_json

    @property
    def frame(self) -> Frame:
        value = self._value.frame
        if self._context is None:
            raise UnsupportedError("Request.frame is unavailable without its sync BrowserContext")
        page = self._context._wrap_page(value._page)
        return page._wrap_frame(value)

    def is_navigation_request(self) -> bool:
        return self._value.is_navigation_request()

    @property
    def failure(self) -> str | None:
        return self._value.failure

    @property
    def timing(self) -> ResourceTiming:
        return self._value.timing

    @property
    def redirected_from(self) -> Request | None:
        value = self._value.redirected_from
        if value is None:
            return None
        if self._context is not None:
            return self._context._wrap_request(value)
        return Request(self._bridge, value)

    @property
    def redirected_to(self) -> Request | None:
        value = self._value.redirected_to
        if value is None:
            return None
        if self._context is not None:
            return self._context._wrap_request(value)
        return Request(self._bridge, value)

    def sizes(self) -> async_api.RequestSizes:
        return self._bridge.run(self._value.sizes())

    def response(self) -> Response | None:
        value = self._bridge.run(self._value.response())
        if value is None:
            return None
        if self._context is not None:
            return self._context._wrap_response(value)
        return Response(self._bridge, value, self._context)


class Route:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.Route,
        context: BrowserContext | None = None,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._request = (
            context._wrap_request(value.request)
            if context is not None
            else Request(bridge, value.request)
        )
        self._callback_commands: list[Coroutine[Any, Any, None]] = []

    @property
    def request(self) -> Request:
        return self._request

    def _run_command(self, command: Coroutine[Any, Any, None]) -> None:
        if threading.get_ident() == self._bridge._thread_id:
            self._callback_commands.append(command)
        else:
            self._bridge.run(command)

    async def _finish_callback_commands(self) -> None:
        commands, self._callback_commands = self._callback_commands, []
        for command in commands:
            await command

    def _discard_callback_commands(self) -> None:
        commands, self._callback_commands = self._callback_commands, []
        for command in commands:
            command.close()

    def continue_(
        self,
        *,
        url: str | None = None,
        method: str | None = None,
        headers: Mapping[str, str | None] | None = None,
        post_data: Any = None,
    ) -> None:
        self._run_command(
            self._value.continue_(
                url=url,
                method=method,
                headers=headers,
                post_data=post_data,
            )
        )

    def fallback(
        self,
        *,
        url: str | None = None,
        method: str | None = None,
        headers: Mapping[str, str | None] | None = None,
        post_data: Any = None,
    ) -> None:
        self._run_command(
            self._value.fallback(
                url=url,
                method=method,
                headers=headers,
                post_data=post_data,
            )
        )

    def abort(self, error_code: str | None = None) -> None:
        self._run_command(self._value.abort(error_code))

    def fetch(
        self,
        *,
        url: str | None = None,
        method: str | None = None,
        headers: Mapping[str, str] | None = None,
        post_data: object = None,
        timeout: float | None = None,
        max_redirects: int | None = None,
        max_retries: int | None = None,
    ) -> APIResponse:
        operation = self._value.fetch(
            url=url,
            method=method,
            headers=headers,
            post_data=post_data,
            timeout=timeout,
            max_redirects=max_redirects,
            max_retries=max_retries,
        )
        return APIResponse(
            self._bridge,
            _run_callback_safe(
                self._bridge,
                operation,
                worker_name="pwright-sync-route-fetch",
            ),
        )

    def fulfill(
        self,
        *,
        status: int | None = None,
        headers: Mapping[str, str] | None = None,
        body: str | bytes | None = None,
        json: object = None,
        path: str | Path | None = None,
        content_type: str | None = None,
        response: APIResponse | None = None,
    ) -> None:
        self._run_command(
            self._value.fulfill(
                status=status,
                headers=headers,
                body=body,
                json=json,
                path=path,
                content_type=content_type,
                response=None if response is None else response._value,
            )
        )


class Response:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.Response,
        context: BrowserContext | None = None,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._context = context

    @property
    def url(self) -> str:
        return self._value.url

    @property
    def status(self) -> int:
        return self._value.status

    @property
    def status_text(self) -> str:
        return self._value.status_text

    @property
    def ok(self) -> bool:
        return self._value.ok

    @property
    def headers(self) -> dict[str, str]:
        return self._value.headers

    @property
    def from_service_worker(self) -> bool:
        return self._value.from_service_worker

    @property
    def request(self) -> Request:
        if self._context is not None:
            return self._context._wrap_request(self._value.request)
        return Request(self._bridge, self._value.request, self._context)

    @property
    def frame(self) -> Frame:
        if self._context is not None:
            frame = self._value.frame
            return self._context._wrap_page(frame.page)._wrap_frame(frame)
        return self.request.frame

    def body(self) -> bytes:
        return self._bridge.run(self._value.body())

    def text(self) -> str:
        return self._bridge.run(self._value.text())

    def json(self) -> Any:
        return self._bridge.run(self._value.json())

    def finished(self) -> None:
        self._bridge.run(self._value.finished())

    def security_details(self) -> SecurityDetails | None:
        return self._bridge.run(self._value.security_details())


SyncNetworkValue = Union[Request, Response]
TNetwork = TypeVar("TNetwork", bound=SyncNetworkValue)


class EventContextManager(Generic[TNetwork]):
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.EventContextManager[async_api.NetworkValue],
        context: BrowserContext | None = None,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._context = context

    def __enter__(self) -> EventContextManager[TNetwork]:
        self._bridge.run(self._value.__aenter__())
        return self

    async def _wait(self) -> async_api.NetworkValue:
        return await self._value.value

    @property
    def value(self) -> TNetwork:
        value = self._bridge.run(self._wait())
        if self._context is not None:
            if isinstance(value, async_api.Response):
                return cast(TNetwork, self._context._wrap_response(value))
            return cast(TNetwork, self._context._wrap_request(value))
        if isinstance(value, async_api.Response):
            return cast(TNetwork, Response(self._bridge, value, self._context))
        return cast(TNetwork, Request(self._bridge, value, self._context))

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: object,
    ) -> bool:
        return self._bridge.run(self._value.__aexit__(exc_type, exc, traceback))


class PageEventContextManager:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.PageEventContextManager,
        page: Page,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    def __enter__(self) -> PageEventContextManager:
        self._bridge.run(self._value.__aenter__())
        return self

    async def _wait(self) -> object:
        return await self._value.value

    @property
    def value(self) -> object:
        value = self._bridge.run(self._wait())
        if isinstance(value, async_api.Frame):
            return self._page._wrap_frame(value)
        if isinstance(value, async_api.Page):
            return self._page.context._wrap_page(value)
        if isinstance(value, async_api.Dialog):
            return self._page._wrap_dialog(value)
        if isinstance(value, async_api.FileChooser):
            return self._page._wrap_file_chooser(value)
        if isinstance(value, async_api.Download):
            return Download(self._bridge, value, self._page)
        if isinstance(value, async_api.Worker):
            return self._page._wrap_worker(value)
        if isinstance(value, async_api.ConsoleMessage):
            return ConsoleMessage(value, self._page)
        return value

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: object,
    ) -> bool:
        return self._bridge.run(self._value.__aexit__(exc_type, exc, traceback))


class NavigationContextManager:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.NavigationContextManager,
        page: Page,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    def __enter__(self) -> NavigationContextManager:
        self._bridge.run(self._value.__aenter__())
        return self

    async def _wait(self) -> async_api.Response | None:
        return await self._value.value

    @property
    def value(self) -> Response | None:
        value = self._bridge.run(self._wait())
        return None if value is None else self._page._context._wrap_response(value)

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: object,
    ) -> bool:
        return self._bridge.run(self._value.__aexit__(exc_type, exc, traceback))


class WorkerEventContextManager:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.WorkerEventContextManager,
        worker: Worker,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._worker = worker

    def __enter__(self) -> WorkerEventContextManager:
        self._bridge.run(self._value.__aenter__())
        return self

    async def _wait(self) -> object:
        return await self._value.value

    @property
    def value(self) -> object:
        value = self._bridge.run(self._wait())
        if isinstance(value, async_api.ConsoleMessage):
            return ConsoleMessage(value, self._worker._page)
        if isinstance(value, async_api.Worker):
            return self._worker
        return value

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: object,
    ) -> bool:
        return self._bridge.run(self._value.__aexit__(exc_type, exc, traceback))


class BrowserContextEventContextManager:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.BrowserContextEventContextManager,
        context: BrowserContext,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._context = context

    def __enter__(self) -> BrowserContextEventContextManager:
        self._bridge.run(self._value.__aenter__())
        return self

    async def _wait(self) -> object:
        return await self._value.value

    @property
    def value(self) -> object:
        value = self._bridge.run(self._wait())
        return self._context._wrap_context_event(value)

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: object,
    ) -> bool:
        return self._bridge.run(self._value.__aexit__(exc_type, exc, traceback))


class JSHandle:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.JSHandle,
        page: Page | None = None,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    @property
    def _impl_obj(self) -> async_api.JSHandle:
        """Return the read-only compatibility view used by upstream assertions."""

        return self._value

    def __repr__(self) -> str:
        return repr(self._value)

    def __str__(self) -> str:
        return str(self._value)

    def as_element(self) -> ElementHandle | None:
        return None

    def evaluate(self, expression: str, arg: object = None) -> object:
        value = _async_evaluation_argument(arg)
        return self._bridge.run(self._value.evaluate(expression, value))

    def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        value = _async_evaluation_argument(arg)
        return _wrap_handle(
            self._bridge,
            self._bridge.run(self._value.evaluate_handle(expression, value)),
            self._page,
        )

    def get_property(self, property_name: str) -> JSHandle:
        return _wrap_handle(
            self._bridge,
            self._bridge.run(self._value.get_property(property_name)),
            self._page,
        )

    def get_properties(self) -> dict[str, JSHandle]:
        values = self._bridge.run(self._value.get_properties())
        return {
            name: _wrap_handle(self._bridge, value, self._page) for name, value in values.items()
        }

    def json_value(self) -> object:
        return self._bridge.run(self._value.json_value())

    def dispose(self) -> None:
        self._bridge.run(self._value.dispose())


def _async_evaluation_argument(value: object) -> object:
    """Replace sync Handle wrappers while preserving bounded container identity."""

    class Converter:
        def __init__(self) -> None:
            self.memo: dict[int, object] = {}
            self.nodes = 0
            self.entries = 0

        def convert(self, item: object, depth: int = 0) -> object:
            if depth > async_api._MAX_EVALUATION_ARGUMENT_DEPTH:
                raise InvalidArgumentError("evaluation argument depth exceeds limit")
            self.nodes += 1
            if self.nodes > async_api._MAX_EVALUATION_ARGUMENT_NODES:
                raise InvalidArgumentError("evaluation argument node count exceeds limit")
            if isinstance(item, JSHandle):
                return item._value
            if isinstance(item, list):
                source = cast(list[object], item)
                identity = id(source)
                if identity in self.memo:
                    return self.memo[identity]
                return self._convert_sequence(source, identity, depth)
            if isinstance(item, tuple):
                source = cast(tuple[object, ...], item)
                identity = id(source)
                if identity in self.memo:
                    return self.memo[identity]
                return self._convert_sequence(source, identity, depth)
            if isinstance(item, dict):
                source = cast(dict[object, object], item)
                identity = id(source)
                if identity in self.memo:
                    return self.memo[identity]
                self._charge_entries(len(source))
                output: dict[object, object] = {}
                self.memo[identity] = output
                for key, child in source.items():
                    output[key] = self.convert(child, depth + 1)
                return output
            return item

        def _convert_sequence(
            self, source: Sequence[object], identity: int, depth: int
        ) -> list[object]:
            self._charge_entries(len(source))
            output: list[object] = []
            self.memo[identity] = output
            output.extend(self.convert(child, depth + 1) for child in source)
            return output

        def _charge_entries(self, count: int) -> None:
            self.entries += count
            if self.entries > async_api._MAX_EVALUATION_ARGUMENT_ENTRIES:
                raise InvalidArgumentError("evaluation argument entry count exceeds limit")

    return Converter().convert(value)


def _async_event_init_argument(
    value: Mapping[str, object] | None,
) -> Mapping[str, object] | None:
    if value is None:
        return None
    if not isinstance(cast(object, value), Mapping):
        return value
    return cast(Mapping[str, object], _async_evaluation_argument(dict(value)))


class Dialog:
    def __init__(self, bridge: _LoopThread, value: async_api.Dialog, page: Page) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    @property
    def page(self) -> Page:
        return self._page

    @property
    def type(self) -> str:
        return self._value.type

    @property
    def message(self) -> str:
        return self._value.message

    @property
    def default_value(self) -> str:
        return self._value.default_value

    def accept(self, prompt_text: str | None = None) -> None:
        self._bridge.run_event_command(self._value.accept(prompt_text))

    def dismiss(self) -> None:
        self._bridge.run_event_command(self._value.dismiss())

    def __repr__(self) -> str:
        return (
            f"<Dialog type={self.type} message={self.message} default_value={self.default_value}>"
        )


class WebError:
    def __init__(self, value: async_api.WebError, context: BrowserContext) -> None:
        self._value = value
        self._context = context

    @property
    def page(self) -> Page | None:
        value = self._value.page
        return None if value is None else self._context._wrap_page(value)

    @property
    def error(self) -> Error:
        return self._value.error


class FileChooser:
    def __init__(self, bridge: _LoopThread, value: async_api.FileChooser, page: Page) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page
        self._element = ElementHandle(bridge, value.element, page)

    @property
    def page(self) -> Page:
        return self._page

    def is_multiple(self) -> bool:
        return self._value.is_multiple()

    @property
    def element(self) -> ElementHandle:
        return self._element

    def set_files(
        self,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run_event_command(
            self._value.set_files(files, timeout=timeout, no_wait_after=no_wait_after)
        )

    def __repr__(self) -> str:
        return f"<FileChooser page={self.page} element={self.element}>"


class Keyboard:
    def __init__(self, bridge: _LoopThread, value: async_api.Keyboard) -> None:
        self._bridge = bridge
        self._value = value

    def down(self, key: str) -> None:
        self._bridge.run(self._value.down(key))

    def up(self, key: str) -> None:
        self._bridge.run(self._value.up(key))

    def insert_text(self, text: str) -> None:
        self._bridge.run(self._value.insert_text(text))

    def type(self, text: str, *, delay: float | None = None) -> None:
        self._bridge.run(self._value.type(text, delay=delay))

    def press(self, key: str, *, delay: float | None = None) -> None:
        self._bridge.run(self._value.press(key, delay=delay))


class Touchscreen:
    def __init__(self, bridge: _LoopThread, value: async_api.Touchscreen) -> None:
        self._bridge = bridge
        self._value = value

    def tap(self, x: float, y: float) -> None:
        self._bridge.run(self._value.tap(x, y))


class Mouse:
    def __init__(self, bridge: _LoopThread, value: async_api.Mouse) -> None:
        self._bridge = bridge
        self._value = value

    def move(self, x: float, y: float, *, steps: int | None = None) -> None:
        self._bridge.run(self._value.move(x, y, steps=steps))

    def down(
        self,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
    ) -> None:
        self._bridge.run(self._value.down(button=button, click_count=click_count))

    def up(
        self,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
    ) -> None:
        self._bridge.run(self._value.up(button=button, click_count=click_count))

    def click(
        self,
        x: float,
        y: float,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
        delay: float | None = None,
    ) -> None:
        self._bridge.run(
            self._value.click(
                x,
                y,
                button=button,
                click_count=click_count,
                delay=delay,
            )
        )

    def dblclick(
        self,
        x: float,
        y: float,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        delay: float | None = None,
    ) -> None:
        self._bridge.run(self._value.dblclick(x, y, button=button, delay=delay))

    def wheel(self, delta_x: float, delta_y: float) -> None:
        self._bridge.run(self._value.wheel(delta_x, delta_y))


class Download:
    def __init__(self, bridge: _LoopThread, value: async_api.Download, page: Page) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    @property
    def page(self) -> Page:
        return self._page

    @property
    def url(self) -> str:
        return self._value.url

    @property
    def suggested_filename(self) -> str:
        return self._value.suggested_filename

    def path(self) -> Path:
        return self._bridge.run(self._value.path())

    def __repr__(self) -> str:
        return repr(self._value)


class Worker:
    def __init__(self, bridge: _LoopThread, value: async_api.Worker, page: Page) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page
        self._event_handlers: list[
            tuple[str, Callable[[object], object], Callable[[object], object]]
        ] = []

    @property
    def url(self) -> str:
        return self._value.url

    def evaluate(self, expression: str, arg: object = None) -> object:
        return self._bridge.run(self._value.evaluate(expression, arg))

    def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        return _wrap_handle(
            self._bridge,
            self._bridge.run(self._value.evaluate_handle(expression, arg)),
            self._page,
        )

    def _wrap_event(self, value: object) -> object:
        if isinstance(value, async_api.ConsoleMessage):
            return ConsoleMessage(value, self._page)
        if isinstance(value, async_api.Worker):
            return self
        return value

    def _invoke_event_handler(self, handler: Callable[[object], object], value: object) -> object:
        try:
            return handler(self._wrap_event(value))
        except BaseException as error:
            self._bridge._record_event_error(error)
            raise

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            return self._invoke_event_handler(handler, value)

        self._event_handlers.append((event, handler, invoke))
        self._value.on(event, invoke)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self._event_handlers = [
                registration
                for registration in self._event_handlers
                if registration[2] is not invoke
            ]
            return self._invoke_event_handler(handler, value)

        self._event_handlers.append((event, handler, invoke))
        self._value.once(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        removed = [
            invoke
            for registered_event, registered_handler, invoke in self._event_handlers
            if registered_event == event and registered_handler == handler
        ]
        self._event_handlers = [
            registration
            for registration in self._event_handlers
            if not (registration[0] == event and registration[1] == handler)
        ]
        for invoke in removed:
            self._value.remove_listener(event, invoke)

    def expect_event(
        self,
        event: str,
        predicate: async_api.WorkerEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> WorkerEventContextManager:
        wrapped_predicate: async_api.WorkerEventPredicate | None = None
        if predicate is not None:

            def wrap(value: object) -> object:
                return predicate(self._wrap_event(value))

            wrapped_predicate = wrap
        return WorkerEventContextManager(
            self._bridge,
            self._value.expect_event(event, wrapped_predicate, timeout=timeout),
            self,
        )

    def wait_for_event(
        self,
        event: str,
        predicate: async_api.WorkerEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> object:
        wrapped_predicate: async_api.WorkerEventPredicate | None = None
        if predicate is not None:

            def wrap(value: object) -> object:
                return predicate(self._wrap_event(value))

            wrapped_predicate = wrap
        return self._wrap_event(
            self._bridge.run(self._value.wait_for_event(event, wrapped_predicate, timeout=timeout))
        )

    def __repr__(self) -> str:
        return repr(self._value)


class ConsoleMessage:
    def __init__(self, value: async_api.ConsoleMessage, page: Page) -> None:
        self._value = value
        self._page = page

    @property
    def page(self) -> Page:
        return self._page

    @property
    def worker(self) -> Worker | None:
        value = self._value.worker
        return None if value is None else self._page._wrap_worker(value)

    @property
    def type(self) -> str:
        return self._value.type

    @property
    def text(self) -> str:
        return self._value.text

    @property
    def args(self) -> list[JSHandle]:
        return [JSHandle(self._page._bridge, value, self._page) for value in self._value.args]

    @property
    def location(self) -> async_api.SourceLocation:
        return self._value.location

    def __str__(self) -> str:
        return self._value.text

    def __repr__(self) -> str:
        return repr(self._value)


class ElementHandle(JSHandle):
    def as_element(self) -> ElementHandle:
        return self

    def is_disabled(self) -> bool:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).is_disabled())

    def is_editable(self) -> bool:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).is_editable())

    def is_enabled(self) -> bool:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).is_enabled())

    def is_hidden(self) -> bool:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).is_hidden())

    def is_visible(self) -> bool:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).is_visible())

    def wait_for_element_state(self, state: str, *, timeout: float | None = None) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).wait_for_element_state(
                state, timeout=timeout
            )
        )

    def scroll_into_view_if_needed(self, *, timeout: float | None = None) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).scroll_into_view_if_needed(timeout=timeout)
        )

    def select_text(self, *, force: bool | None = None, timeout: float | None = None) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).select_text(force=force, timeout=timeout)
        )

    def click(
        self,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
        delay: float | None = None,
        force: bool | None = None,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        no_wait_after: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        steps: int | None = None,
    ) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).click(
                button=button,
                click_count=click_count,
                delay=delay,
                force=force,
                modifiers=modifiers,
                no_wait_after=no_wait_after,
                position=position,
                timeout=timeout,
                trial=trial,
                steps=steps,
            )
        )

    def tap(
        self,
        *,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).tap(
                modifiers=modifiers,
                position=position,
                timeout=timeout,
                force=force,
                no_wait_after=no_wait_after,
                trial=trial,
            )
        )

    def dispatch_event(self, type: str, event_init: Mapping[str, object] | None = None) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).dispatch_event(
                type, _async_event_init_argument(event_init)
            )
        )

    def inner_text(self) -> str:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).inner_text())

    def get_attribute(self, name: str) -> str | None:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).get_attribute(name))

    def inner_html(self) -> str:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).inner_html())

    def input_value(self, *, timeout: float | None = None) -> str:
        return self._bridge.run(
            cast(async_api.ElementHandle, self._value).input_value(timeout=timeout)
        )

    def is_checked(self) -> bool:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).is_checked())

    def bounding_box(self) -> dict[str, float] | None:
        return self._bridge.run(cast(async_api.ElementHandle, self._value).bounding_box())

    def query_selector(self, selector: str) -> ElementHandle | None:
        value = self._bridge.run(
            cast(async_api.ElementHandle, self._value).query_selector(selector)
        )
        if value is None:
            return None
        return ElementHandle(self._bridge, value, self._page)

    def press(
        self,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).press(
                key,
                delay=delay,
                timeout=timeout,
                no_wait_after=no_wait_after,
            )
        )

    def type(
        self,
        text: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).type(
                text,
                delay=delay,
                timeout=timeout,
                no_wait_after=no_wait_after,
            )
        )

    def owner_frame(self) -> Frame | None:
        value = self._bridge.run(cast(async_api.ElementHandle, self._value).owner_frame())
        if value is None:
            return None
        if self._page is None:
            raise UnsupportedError("ElementHandle.owner_frame requires its sync Page owner")
        return self._page._context._wrap_page(value._page)._wrap_frame(value)

    def content_frame(self) -> Frame | None:
        value = self._bridge.run(cast(async_api.ElementHandle, self._value).content_frame())
        if value is None:
            return None
        if self._page is None:
            raise UnsupportedError("ElementHandle.content_frame requires its sync Page owner")
        return self._page._wrap_frame(value)

    def screenshot(
        self,
        *,
        type: str | None = None,
        path: str | Path | None = None,
        quality: int | None = None,
        scale: str = "device",
        timeout: float | None = None,
        mask: Sequence[Locator] | None = None,
    ) -> bytes:
        return self._bridge.run(
            cast(async_api.ElementHandle, self._value).screenshot(
                type=type,
                path=path,
                quality=quality,
                scale=scale,
                timeout=timeout,
                mask=None if mask is None else [value._value for value in mask],
            )
        )

    def set_input_files(
        self,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            cast(async_api.ElementHandle, self._value).set_input_files(
                files,
                timeout=timeout,
                no_wait_after=no_wait_after,
            )
        )


def _wrap_handle(
    bridge: _LoopThread,
    value: async_api.JSHandle,
    page: Page | None = None,
) -> JSHandle:
    if value.as_element() is not None:
        return ElementHandle(bridge, value, page)
    return JSHandle(bridge, value, page)


def _drag_target(value: object) -> Locator:
    if not isinstance(value, Locator):
        raise TypeError("Locator.drag_to target must be a Locator")
    return value


def _locator_argument(value: object, api: str) -> Locator:
    if not isinstance(value, Locator):
        raise TypeError(f"{api} requires a Locator")
    return value


class Locator:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.Locator,
        page: Page | None = None,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    @property
    def page(self) -> Page:
        if self._page is None:
            raise UnsupportedError("Locator.page is unavailable without its sync Page owner")
        return self._page

    def __repr__(self) -> str:
        return repr(self._value)

    def __str__(self) -> str:
        return str(self._value)

    @property
    def description(self) -> str | None:
        return self._value.description

    def describe(self, description: str) -> Locator:
        return Locator(self._bridge, self._value.describe(description), self._page)

    def locator(
        self,
        selector_or_locator: str | Locator,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        value = (
            selector_or_locator._value
            if isinstance(selector_or_locator, Locator)
            else selector_or_locator
        )
        return Locator(
            self._bridge,
            self._value.locator(
                value,
                has_text=has_text,
                has_not_text=has_not_text,
                has=None if has is None else has._value,
                has_not=None if has_not is None else has_not._value,
            ),
            self._page,
        )

    def frame_locator(self, selector: str) -> FrameLocator:
        if self._page is None:
            raise UnsupportedError("Locator.frame_locator requires its sync Page owner")
        return FrameLocator(
            self._bridge,
            self._value.frame_locator(selector),
            self._page,
        )

    @property
    def content_frame(self) -> FrameLocator:
        if self._page is None:
            raise UnsupportedError("Locator.content_frame requires its sync Page owner")
        return FrameLocator(
            self._bridge,
            self._value.content_frame,
            self._page,
        )

    def filter(
        self,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
        visible: bool | None = None,
    ) -> Locator:
        return Locator(
            self._bridge,
            self._value.filter(
                has_text=has_text,
                has_not_text=has_not_text,
                has=None if has is None else has._value,
                has_not=None if has_not is None else has_not._value,
                visible=visible,
            ),
            self._page,
        )

    def and_(self, locator: Locator) -> Locator:
        locator_value = _locator_argument(locator, "Locator.and_")
        return Locator(self._bridge, self._value.and_(locator_value._value), self._page)

    def or_(self, locator: Locator) -> Locator:
        locator_value = _locator_argument(locator, "Locator.or_")
        return Locator(self._bridge, self._value.or_(locator_value._value), self._page)

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_text(text, exact=exact), self._page)

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_label(text, exact=exact), self._page)

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_placeholder(text, exact=exact),
            self._page,
        )

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_alt_text(text, exact=exact),
            self._page,
        )

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_title(text, exact=exact), self._page)

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        return Locator(self._bridge, self._value.get_by_test_id(test_id), self._page)

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_role(
                role,
                checked=checked,
                disabled=disabled,
                expanded=expanded,
                include_hidden=include_hidden,
                level=level,
                name=name,
                pressed=pressed,
                selected=selected,
                exact=exact,
            ),
            self._page,
        )

    @property
    def first(self) -> Locator:
        return Locator(self._bridge, self._value.first, self._page)

    @property
    def last(self) -> Locator:
        return Locator(self._bridge, self._value.last, self._page)

    def nth(self, index: int) -> Locator:
        return Locator(self._bridge, self._value.nth(index), self._page)

    def click(
        self,
        *,
        timeout: float | None = None,
        steps: int | None = None,
        force: bool | None = None,
    ) -> None:
        self._bridge.run(self._value.click(timeout=timeout, steps=steps, force=force))

    def tap(
        self,
        *,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.tap(
                modifiers=modifiers,
                position=position,
                timeout=timeout,
                force=force,
                no_wait_after=no_wait_after,
                trial=trial,
            )
        )

    def dblclick(self, *, timeout: float | None = None) -> None:
        self._bridge.run(self._value.dblclick(timeout=timeout))

    def bounding_box(self, *, timeout: float | None = None) -> dict[str, float] | None:
        return self._bridge.run(self._value.bounding_box(timeout=timeout))

    def screenshot(
        self,
        *,
        type: str | None = None,
        path: str | Path | None = None,
        quality: int | None = None,
        scale: str = "device",
        timeout: float | None = None,
        mask: Sequence[Locator] | None = None,
    ) -> bytes:
        return self._bridge.run(
            self._value.screenshot(
                type=type,
                path=path,
                quality=quality,
                scale=scale,
                timeout=timeout,
                mask=None if mask is None else [value._value for value in mask],
            )
        )

    def set_input_files(
        self,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.set_input_files(
                files,
                timeout=timeout,
                no_wait_after=no_wait_after,
            )
        )

    def element_handle(self, *, timeout: float | None = None) -> ElementHandle:
        return ElementHandle(
            self._bridge,
            self._bridge.run(self._value.element_handle(timeout=timeout)),
            self._page,
        )

    def element_handles(self) -> list[ElementHandle]:
        return [
            ElementHandle(self._bridge, value, self._page)
            for value in self._bridge.run(self._value.element_handles())
        ]

    def highlight(self) -> None:
        self._bridge.run(self._value.highlight())

    def dispatch_event(
        self,
        type: str,
        event_init: Mapping[str, object] | None = None,
        *,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(
            self._value.dispatch_event(
                type, _async_event_init_argument(event_init), timeout=timeout
            )
        )

    def evaluate_handle(
        self,
        expression: str,
        arg: object = None,
        *,
        timeout: float | None = None,
    ) -> JSHandle:
        return _wrap_handle(
            self._bridge,
            self._bridge.run(self._value.evaluate_handle(expression, arg, timeout=timeout)),
            self._page,
        )

    def select_text(
        self,
        *,
        force: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(self._value.select_text(force=force, timeout=timeout))

    def fill(
        self,
        value: str,
        *,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.fill(
                value,
                timeout=timeout,
                force=force,
                no_wait_after=no_wait_after,
            )
        )

    def press(
        self,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.press(
                key,
                delay=delay,
                timeout=timeout,
                no_wait_after=no_wait_after,
            )
        )

    def set_checked(
        self,
        checked: bool,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.set_checked(
                checked,
                force=force,
                position=position,
                timeout=timeout,
                trial=trial,
                no_wait_after=no_wait_after,
            )
        )

    def check(
        self,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.check(
                force=force,
                position=position,
                timeout=timeout,
                trial=trial,
                no_wait_after=no_wait_after,
            )
        )

    def uncheck(
        self,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.uncheck(
                force=force,
                position=position,
                timeout=timeout,
                trial=trial,
                no_wait_after=no_wait_after,
            )
        )

    def focus(self, *, timeout: float | None = None) -> None:
        self._bridge.run(self._value.focus(timeout=timeout))

    def blur(self, *, timeout: float | None = None) -> None:
        self._bridge.run(self._value.blur(timeout=timeout))

    def hover(
        self,
        *,
        force: bool | None = None,
        modifiers: list[str] | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.hover(
                force=force,
                modifiers=modifiers,
                position=position,
                timeout=timeout,
                trial=trial,
            )
        )

    def clear(
        self,
        *,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(
            self._value.clear(
                force=force,
                no_wait_after=no_wait_after,
                timeout=timeout,
            )
        )

    def select_option(
        self,
        value: str,
        *,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> list[str]:
        return self._bridge.run(
            self._value.select_option(
                value,
                force=force,
                no_wait_after=no_wait_after,
                timeout=timeout,
            )
        )

    def scroll_into_view_if_needed(self, *, timeout: float | None = None) -> None:
        self._bridge.run(self._value.scroll_into_view_if_needed(timeout=timeout))

    def press_sequentially(
        self,
        text: str,
        *,
        delay: float | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(
            self._value.press_sequentially(
                text,
                delay=delay,
                no_wait_after=no_wait_after,
                timeout=timeout,
            )
        )

    def type(
        self,
        text: str,
        *,
        delay: float | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(
            self._value.type(
                text,
                delay=delay,
                no_wait_after=no_wait_after,
                timeout=timeout,
            )
        )

    def input_value(self, *, timeout: float | None = None) -> str:
        return self._bridge.run(self._value.input_value(timeout=timeout))

    def is_hidden(self, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_hidden(timeout=timeout))

    def is_visible(self, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_visible(timeout=timeout))

    def is_checked(self, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_checked(timeout=timeout))

    def is_enabled(self, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_enabled(timeout=timeout))

    def is_disabled(self, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_disabled(timeout=timeout))

    def is_editable(self, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_editable(timeout=timeout))

    def text_content(self, *, timeout: float | None = None) -> str | None:
        return self._bridge.run(self._value.text_content(timeout=timeout))

    def inner_text(self, *, timeout: float | None = None) -> str:
        return self._bridge.run(self._value.inner_text(timeout=timeout))

    def inner_html(self, *, timeout: float | None = None) -> str:
        return self._bridge.run(self._value.inner_html(timeout=timeout))

    def get_attribute(self, name: str, *, timeout: float | None = None) -> str | None:
        return self._bridge.run(self._value.get_attribute(name, timeout=timeout))

    def count(self) -> int:
        return self._bridge.run(self._value.count())

    def all_inner_texts(self) -> list[str]:
        return self._bridge.run(self._value.all_inner_texts())

    def all_text_contents(self) -> list[str]:
        return self._bridge.run(self._value.all_text_contents())

    def all(self) -> list[Locator]:
        values = self._bridge.run(self._value.all())
        return [Locator(self._bridge, value, self._page) for value in values]

    def wait_for(self, *, timeout: float | None = None, state: str | None = None) -> None:
        self._bridge.run(self._value.wait_for(timeout=timeout, state=state))

    def evaluate(
        self, expression: str, arg: object = None, *, timeout: float | None = None
    ) -> object:
        return self._bridge.run(self._value.evaluate(expression, arg, timeout=timeout))

    def evaluate_all(self, expression: str, arg: object = None) -> object:
        return self._bridge.run(self._value.evaluate_all(expression, arg))

    def _expect_count(self, expected: int, timeout_ms: int, is_not: bool) -> dict[str, object]:
        return self._bridge.run(self._value._expect_count(expected, timeout_ms, is_not))

    def _expect_visibility(self, visible: bool, timeout_ms: int, is_not: bool) -> dict[str, object]:
        return self._bridge.run(self._value._expect_visibility(visible, timeout_ms, is_not))

    def _expect_text(
        self,
        expected: str | Pattern[str] | list[str],
        timeout_ms: int,
        is_not: bool,
        *,
        substring: bool = False,
        use_inner_text: bool = False,
        ignore_case: bool | None = None,
    ) -> dict[str, object]:
        return self._bridge.run(
            self._value._expect_text(
                expected,
                timeout_ms,
                is_not,
                substring=substring,
                use_inner_text=use_inner_text,
                ignore_case=ignore_case,
            )
        )

    def _expect_class(
        self,
        expected: tuple[str, str | None] | list[tuple[str, str | None]],
        timeout_ms: int,
        is_not: bool,
    ) -> dict[str, object]:
        return self._bridge.run(self._value._expect_class(expected, timeout_ms, is_not))

    def _expect_state(self, state: str, timeout_ms: int, is_not: bool) -> dict[str, object]:
        return self._bridge.run(self._value._expect_state(state, timeout_ms, is_not))

    def _expect_checked(
        self,
        checked: bool | None,
        indeterminate: bool | None,
        timeout_ms: int,
        is_not: bool,
    ) -> dict[str, object]:
        return self._bridge.run(
            self._value._expect_checked(checked, indeterminate, timeout_ms, is_not)
        )

    def _expect_property(
        self, name: str, expected: object, timeout_ms: int, is_not: bool
    ) -> dict[str, object]:
        return self._bridge.run(self._value._expect_property(name, expected, timeout_ms, is_not))

    def _expect_attribute(
        self,
        name: str,
        expected: str,
        flags: str | None,
        ignore_case: bool | None,
        timeout_ms: int,
        is_not: bool,
    ) -> dict[str, object]:
        return self._bridge.run(
            self._value._expect_attribute(
                name,
                expected,
                flags,
                ignore_case,
                timeout_ms,
                is_not,
            )
        )

    def drag_to(
        self,
        target: Locator,
        *,
        source_position: Mapping[str, float] | None = None,
        target_position: Mapping[str, float] | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        steps: int | None = None,
    ) -> None:
        target_value = _drag_target(target)
        self._bridge.run(
            self._value.drag_to(
                target_value._value,
                source_position=source_position,
                target_position=target_position,
                force=force,
                no_wait_after=no_wait_after,
                timeout=timeout,
                trial=trial,
                steps=steps,
            )
        )


class FrameLocator:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.FrameLocator,
        page: Page,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    def __repr__(self) -> str:
        return repr(self._value)

    def __str__(self) -> str:
        return str(self._value)

    def locator(
        self,
        selector_or_locator: str | Locator,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        value = (
            selector_or_locator._value
            if isinstance(selector_or_locator, Locator)
            else selector_or_locator
        )
        return Locator(
            self._bridge,
            self._value.locator(
                value,
                has_text=has_text,
                has_not_text=has_not_text,
                has=None if has is None else has._value,
                has_not=None if has_not is None else has_not._value,
            ),
            self._page,
        )

    def frame_locator(self, selector: str) -> FrameLocator:
        return FrameLocator(
            self._bridge,
            self._value.frame_locator(selector),
            self._page,
        )

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_text(text, exact=exact), self._page)

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_label(text, exact=exact), self._page)

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_placeholder(text, exact=exact),
            self._page,
        )

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_alt_text(text, exact=exact),
            self._page,
        )

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_title(text, exact=exact), self._page)

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        return Locator(self._bridge, self._value.get_by_test_id(test_id), self._page)

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_role(
                role,
                checked=checked,
                disabled=disabled,
                expanded=expanded,
                include_hidden=include_hidden,
                level=level,
                name=name,
                pressed=pressed,
                selected=selected,
                exact=exact,
            ),
            self._page,
        )

    @property
    def owner(self) -> Locator:
        return Locator(self._bridge, self._value.owner, self._page)

    @property
    def first(self) -> FrameLocator:
        return FrameLocator(self._bridge, self._value.first, self._page)

    @property
    def last(self) -> FrameLocator:
        return FrameLocator(self._bridge, self._value.last, self._page)

    def nth(self, index: int) -> FrameLocator:
        return FrameLocator(self._bridge, self._value.nth(index), self._page)


class LocatorAssertions:
    def __init__(
        self,
        locator: Locator,
        timeout_ms: int,
        is_not: bool = False,
        message: str | None = None,
    ) -> None:
        self._locator = locator
        self._timeout_ms = timeout_ms
        self._is_not = is_not
        self._message = message

    @property
    def not_to(self) -> LocatorAssertions:
        return LocatorAssertions(
            self._locator,
            self._timeout_ms,
            not self._is_not,
            self._message,
        )

    def to_have_count(self, count: int, *, timeout: float | None = None) -> None:
        expected = async_api._expected_count(count)
        timeout_ms = self._timeout_ms if timeout is None else async_api._timeout_ms(timeout)
        result = self._locator._expect_count(expected, timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}have count {expected}, received {received}"
            )

    def to_be_visible(self, *, timeout: float | None = None) -> None:
        timeout_ms = self._timeout_ms if timeout is None else async_api._timeout_ms(timeout)
        result = self._locator._expect_visibility(True, timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}be visible, received {received}"
            )

    def to_be_hidden(self, *, timeout: float | None = None) -> None:
        timeout_ms = self._timeout_ms if timeout is None else async_api._timeout_ms(timeout)
        result = self._locator._expect_visibility(False, timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}be hidden, received {received}"
            )

    def to_be_checked(
        self,
        *,
        timeout: float | None = None,
        checked: bool | None = None,
        indeterminate: bool | None = None,
    ) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_be_checked(
                timeout=timeout,
                checked=checked,
                indeterminate=indeterminate,
            )
        )

    def to_be_empty(self, *, timeout: float | None = None) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_be_empty(timeout=timeout)
        )

    def to_have_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_have_text(
                expected,
                use_inner_text=use_inner_text,
                ignore_case=ignore_case,
                timeout=timeout,
            )
        )

    def to_contain_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        timeout: float | None = None,
        ignore_case: bool | None = None,
    ) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_contain_text(
                expected,
                use_inner_text=use_inner_text,
                timeout=timeout,
                ignore_case=ignore_case,
            )
        )

    def to_have_value(
        self,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_have_value(value, timeout=timeout)
        )

    def to_have_class(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        timeout: float | None = None,
    ) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_have_class(expected, timeout=timeout)
        )

    def to_have_attribute(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_have_attribute(
                name,
                value,
                ignore_case=ignore_case,
                timeout=timeout,
            )
        )

    def to_have_css(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        self._locator._bridge.run(
            async_api.LocatorAssertions(
                self._locator._value,
                self._timeout_ms,
                self._is_not,
                self._message,
            ).to_have_css(name, value, timeout=timeout)
        )

    def to_have_id(
        self,
        id: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        self.to_have_attribute("id", id, timeout=timeout)

    def not_to_have_count(self, count: int, *, timeout: float | None = None) -> None:
        self.not_to.to_have_count(count, timeout=timeout)

    def not_to_be_visible(self, *, timeout: float | None = None) -> None:
        self.not_to.to_be_visible(timeout=timeout)

    def not_to_be_hidden(self, *, timeout: float | None = None) -> None:
        self.not_to.to_be_hidden(timeout=timeout)

    def not_to_be_checked(
        self,
        *,
        timeout: float | None = None,
        checked: bool | None = None,
        indeterminate: bool | None = None,
    ) -> None:
        self.not_to.to_be_checked(
            timeout=timeout,
            checked=checked,
            indeterminate=indeterminate,
        )

    def not_to_be_empty(self, *, timeout: float | None = None) -> None:
        self.not_to.to_be_empty(timeout=timeout)

    def not_to_have_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self.not_to.to_have_text(
            expected,
            use_inner_text=use_inner_text,
            ignore_case=ignore_case,
            timeout=timeout,
        )

    def not_to_contain_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        timeout: float | None = None,
        ignore_case: bool | None = None,
    ) -> None:
        self.not_to.to_contain_text(
            expected,
            use_inner_text=use_inner_text,
            timeout=timeout,
            ignore_case=ignore_case,
        )

    def not_to_have_value(
        self,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        self.not_to.to_have_value(value, timeout=timeout)

    def not_to_have_class(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        timeout: float | None = None,
    ) -> None:
        self.not_to.to_have_class(expected, timeout=timeout)

    def not_to_have_attribute(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self.not_to.to_have_attribute(
            name,
            value,
            ignore_case=ignore_case,
            timeout=timeout,
        )

    def not_to_have_css(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        self.not_to.to_have_css(name, value, timeout=timeout)

    def not_to_have_id(
        self,
        id: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        self.not_to.to_have_id(id, timeout=timeout)


class Expect:
    def __init__(self) -> None:
        self._timeout_ms = 5_000

    def set_options(self, *, timeout: float | None = None) -> None:
        if timeout is not None:
            self._timeout_ms = async_api._timeout_ms(timeout)

    def __call__(self, actual: object, message: str | None = None) -> LocatorAssertions:
        if not isinstance(actual, Locator):
            raise UnsupportedError("expect currently requires a Locator")
        return LocatorAssertions(actual, self._timeout_ms, message=message)


expect = Expect()


class Frame:
    def __init__(self, bridge: _LoopThread, value: async_api.Frame, page: Page) -> None:
        self._bridge = bridge
        self._value = value
        self._page = page

    def __repr__(self) -> str:
        return repr(self._value)

    def __str__(self) -> str:
        return str(self._value)

    @property
    def page(self) -> Page:
        return self._page

    @property
    def name(self) -> str:
        return self._value.name

    @property
    def url(self) -> str:
        return self._value.url

    @property
    def parent_frame(self) -> Frame | None:
        value = self._value.parent_frame
        return None if value is None else self._page._wrap_frame(value)

    @property
    def child_frames(self) -> list[Frame]:
        return [self._page._wrap_frame(value) for value in self._value.child_frames]

    def locator(
        self,
        selector: str,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        return Locator(
            self._bridge,
            self._value.locator(
                selector,
                has_text=has_text,
                has_not_text=has_not_text,
                has=None if has is None else has._value,
                has_not=None if has_not is None else has_not._value,
            ),
            self._page,
        )

    def frame_locator(self, selector: str) -> FrameLocator:
        return FrameLocator(
            self._bridge,
            self._value.frame_locator(selector),
            self._page,
        )

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_text(text, exact=exact), self._page)

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_label(text, exact=exact), self._page)

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_placeholder(text, exact=exact),
            self._page,
        )

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_alt_text(text, exact=exact),
            self._page,
        )

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_title(text, exact=exact), self._page)

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        return Locator(self._bridge, self._value.get_by_test_id(test_id), self._page)

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_role(
                role,
                checked=checked,
                disabled=disabled,
                expanded=expanded,
                include_hidden=include_hidden,
                level=level,
                name=name,
                pressed=pressed,
                selected=selected,
                exact=exact,
            ),
            self._page,
        )

    def evaluate(self, expression: str, arg: object = None) -> object:
        value = _async_evaluation_argument(arg)
        return self._bridge.run(self._value.evaluate(expression, value))

    def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        value = _async_evaluation_argument(arg)
        return _wrap_handle(
            self._bridge,
            self._bridge.run(self._value.evaluate_handle(expression, value)),
            self._page,
        )

    def add_script_tag(
        self,
        *,
        content: str | None = None,
        path: str | Path | None = None,
        type: str | None = None,
        url: str | None = None,
    ) -> ElementHandle:
        value = self._bridge.run(
            self._value.add_script_tag(content=content, path=path, type=type, url=url)
        )
        return ElementHandle(self._bridge, value, self._page)

    def wait_for_function(
        self,
        expression: str,
        arg: object = None,
        *,
        polling: float | str | None = None,
        timeout: float | None = None,
    ) -> JSHandle:
        value = arg._value if isinstance(arg, JSHandle) else arg
        return _wrap_handle(
            self._bridge,
            self._bridge.run(
                self._value.wait_for_function(expression, value, polling=polling, timeout=timeout)
            ),
            self._page,
        )

    def wait_for_load_state(
        self,
        state: str | None = None,
        *,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(self._value.wait_for_load_state(state, timeout=timeout))

    def goto(
        self,
        url: str,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
        referer: str | None = None,
    ) -> Response | None:
        value = self._bridge.run(
            self._value.goto(url, timeout=timeout, wait_until=wait_until, referer=referer)
        )
        return None if value is None else self._page._context._wrap_response(value)

    def expect_navigation(
        self,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        wait_until: str | None = None,
        timeout: float | None = None,
    ) -> NavigationContextManager:
        return NavigationContextManager(
            self._bridge,
            self._value.expect_navigation(url=url, wait_until=wait_until, timeout=timeout),
            self._page,
        )

    def query_selector(
        self,
        selector: str,
        *,
        strict: bool | None = None,
    ) -> ElementHandle | None:
        value = self._bridge.run(self._value.query_selector(selector, strict=strict))
        return None if value is None else ElementHandle(self._bridge, value, self._page)

    def eval_on_selector(
        self,
        selector: str,
        expression: str,
        arg: object = None,
        *,
        strict: bool | None = None,
    ) -> object:
        return self._bridge.run(
            self._value.eval_on_selector(
                selector,
                expression,
                arg,
                strict=strict,
            )
        )

    def eval_on_selector_all(
        self,
        selector: str,
        expression: str,
        arg: object = None,
    ) -> object:
        return self._bridge.run(self._value.eval_on_selector_all(selector, expression, arg))

    def focus(
        self,
        selector: str,
        *,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(self._value.focus(selector, strict=strict, timeout=timeout))

    def press(
        self,
        selector: str,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.press(
                selector,
                key,
                delay=delay,
                timeout=timeout,
                no_wait_after=no_wait_after,
            )
        )

    def type(
        self,
        selector: str,
        text: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.type(
                selector,
                text,
                delay=delay,
                timeout=timeout,
                no_wait_after=no_wait_after,
                strict=strict,
            )
        )


class Page:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.Page,
        context: BrowserContext,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._context = context
        self._frames: dict[int, Frame] = {}
        self._dialogs: dict[int, Dialog] = {}
        self._file_choosers: dict[int, FileChooser] = {}
        self._workers: dict[str, Worker] = {}
        self._remote = isinstance(value, async_api.RemotePage)
        if self._remote:
            self._keyboard = cast(Keyboard, None)
            self._mouse = cast(Mouse, None)
            self._touchscreen = cast(Touchscreen, None)
        else:
            self._keyboard = Keyboard(bridge, value.keyboard)
            self._mouse = Mouse(bridge, value.mouse)
            self._touchscreen = Touchscreen(bridge, value.touchscreen)
        self._event_handlers: list[
            tuple[str, Callable[[object], object], Callable[[object], object]]
        ] = []

    def __enter__(self) -> Page:
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __repr__(self) -> str:
        return repr(self._value)

    @property
    def context(self) -> BrowserContext:
        return self._context

    @property
    def request(self) -> APIRequestContext:
        return self._context.request

    @property
    def keyboard(self) -> Keyboard:
        if self._remote:
            raise UnsupportedError("Page.keyboard remote operation is not installed")
        return self._keyboard

    @property
    def mouse(self) -> Mouse:
        if self._remote:
            raise UnsupportedError("Page.mouse remote operation is not installed")
        return self._mouse

    @property
    def touchscreen(self) -> Touchscreen:
        if self._remote:
            raise UnsupportedError("Page.touchscreen remote operation is not installed")
        return self._touchscreen

    @property
    def url(self) -> str:
        return self._value.url

    @property
    def workers(self) -> list[Worker]:
        values = self._value.workers
        live_ids = {value._worker_id for value in values}
        for worker_id in set(self._workers) - live_ids:
            self._workers.pop(worker_id, None)
        return [self._wrap_worker(value) for value in values]

    def is_closed(self) -> bool:
        return self._value.is_closed()

    def opener(self) -> Page | None:
        value = self._bridge.run(self._value.opener())
        return None if value is None else self._context._wrap_page(value)

    def _wrap_page_event(self, value: object) -> object:
        if isinstance(value, async_api.Frame):
            return self._wrap_frame(value)
        if isinstance(value, async_api.Page):
            return self._context._wrap_page(value)
        if isinstance(value, async_api.Dialog):
            return self._wrap_dialog(value)
        if isinstance(value, async_api.FileChooser):
            return self._wrap_file_chooser(value)
        if isinstance(value, async_api.Download):
            return Download(self._bridge, value, self)
        if isinstance(value, async_api.Worker):
            return self._wrap_worker(value)
        if isinstance(value, async_api.ConsoleMessage):
            return ConsoleMessage(value, self)
        if isinstance(value, async_api.Request):
            context = cast("BrowserContext | None", self._context)
            if context is None:
                return Request(self._bridge, value)
            return context._wrap_request(value)
        if isinstance(value, async_api.Response):
            context = cast("BrowserContext | None", self._context)
            if context is None:
                return Response(self._bridge, value)
            return context._wrap_response(value)
        return value

    def _wrap_dialog(self, value: async_api.Dialog) -> Dialog:
        return self._dialogs.setdefault(id(value), Dialog(self._bridge, value, self))

    def _wrap_file_chooser(self, value: async_api.FileChooser) -> FileChooser:
        return self._file_choosers.setdefault(id(value), FileChooser(self._bridge, value, self))

    def _wrap_worker(self, value: async_api.Worker) -> Worker:
        return self._workers.setdefault(value._worker_id, Worker(self._bridge, value, self))

    def _invoke_event_handler(self, handler: Callable[[object], object], value: object) -> object:
        try:
            return handler(self._wrap_page_event(value))
        except BaseException as error:
            self._bridge._record_event_error(error)
            raise

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            if self._remote and event == "close":
                self._bridge.queue_remote_event(
                    lambda received: self._invoke_event_handler(handler, received), value
                )
                return None
            return self._invoke_event_handler(handler, value)

        self._event_handlers.append((event, handler, invoke))
        self._value.on(event, invoke)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self._event_handlers = [
                registration
                for registration in self._event_handlers
                if registration[2] is not invoke
            ]
            if self._remote and event == "close":
                self._bridge.queue_remote_event(
                    lambda received: self._invoke_event_handler(handler, received), value
                )
                return None
            return self._invoke_event_handler(handler, value)

        self._event_handlers.append((event, handler, invoke))
        self._value.once(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        removed = [
            invoke
            for registered_event, registered_handler, invoke in self._event_handlers
            if registered_event == event and registered_handler == handler
        ]
        self._event_handlers = [
            registration
            for registration in self._event_handlers
            if not (registration[0] == event and registration[1] == handler)
        ]
        for invoke in removed:
            self._value.remove_listener(event, invoke)

    def set_default_timeout(self, timeout: float) -> None:
        self._value.set_default_timeout(timeout)

    def set_default_navigation_timeout(self, timeout: float) -> None:
        self._value.set_default_navigation_timeout(timeout)

    def pause(self) -> None:
        self._bridge.run(self._value.pause())

    def wait_for_timeout(self, timeout: float) -> None:
        self._bridge.run(self._value.wait_for_timeout(timeout))

    def wait_for_selector(
        self,
        selector: str,
        *,
        state: str | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> ElementHandle | None:
        value = self._bridge.run(
            self._value.wait_for_selector(
                selector,
                state=state,
                strict=strict,
                timeout=timeout,
            )
        )
        return None if value is None else ElementHandle(self._bridge, value, self)

    def request_gc(self) -> None:
        self._bridge.run(self._value.request_gc())

    def pdf(
        self,
        *,
        path: str | Path | None = None,
        tagged: bool | None = None,
        outline: bool | None = None,
        **kwargs: object,
    ) -> bytes:
        return self._bridge.run(
            self._value.pdf(path=path, tagged=tagged, outline=outline, **kwargs)
        )

    def screenshot(
        self,
        *,
        timeout: float | None = None,
        type: str | None = None,
        path: str | Path | None = None,
        quality: int | None = None,
        full_page: bool | None = None,
        scale: str = "device",
        mask: Sequence[Locator] | None = None,
        **kwargs: object,
    ) -> bytes:
        return self._bridge.run(
            self._value.screenshot(
                timeout=timeout,
                type=type,
                path=path,
                quality=quality,
                full_page=full_page,
                scale=scale,
                mask=None if mask is None else [value._value for value in mask],
                **kwargs,
            )
        )

    def set_viewport_size(self, viewport_size: ViewportSize) -> None:
        self._bridge.run(self._value.set_viewport_size(viewport_size))

    @property
    def viewport_size(self) -> ViewportSize | None:
        return self._value.viewport_size

    def page_errors(self) -> list[Error]:
        return self._bridge.run(self._value.page_errors())

    def console_messages(self) -> list[ConsoleMessage]:
        return [
            ConsoleMessage(value, self)
            for value in self._bridge.run(self._value.console_messages())
        ]

    def requests(self) -> list[Request]:
        return [
            self._context._wrap_request(value) for value in self._bridge.run(self._value.requests())
        ]

    def _wrap_frame(self, value: async_api.Frame) -> Frame:
        return self._frames.setdefault(id(value), Frame(self._bridge, value, self))

    @property
    def frames(self) -> list[Frame]:
        return [self._wrap_frame(value) for value in self._value.frames]

    @property
    def main_frame(self) -> Frame:
        return self._wrap_frame(self._value.main_frame)

    def frame(
        self,
        name: str | None = None,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
    ) -> Frame | None:
        value = self._value.frame(name, url=url)
        return None if value is None else self._wrap_frame(value)

    def title(self) -> str:
        return self._bridge.run(self._value.title())

    def content(self) -> str:
        return self._bridge.run(self._value.content())

    def add_init_script(
        self,
        script: str | None = None,
        *,
        path: str | Path | None = None,
    ) -> None:
        self._bridge.run(self._value.add_init_script(script, path=path))

    def expose_binding(
        self,
        name: str,
        callback: Callable[..., object],
        *,
        handle: bool | None = None,
    ) -> None:
        def invoke(source: object, *args: object) -> object:
            if not isinstance(source, dict):
                raise RuntimeError("Page binding source is unavailable")
            async_frame = cast(async_api.Frame, source["frame"])
            sync_source = {
                "context": self._context,
                "page": self,
                "frame": self._wrap_frame(async_frame),
            }
            sync_arguments = [
                _wrap_handle(self._bridge, value, self)
                if isinstance(value, async_api.JSHandle)
                else value
                for value in args
            ]
            return callback(sync_source, *sync_arguments)

        self._bridge.run(self._value.expose_binding(name, invoke, handle=handle))

    def expose_function(self, name: str, callback: Callable[..., object]) -> None:
        self._bridge.run(self._value.expose_function(name, callback))

    def set_extra_http_headers(self, headers: Mapping[str, str]) -> None:
        self._bridge.run(self._value.set_extra_http_headers(headers))

    def route(
        self,
        url: str | Pattern[str] | Callable[[str], bool],
        handler: Callable[..., object],
        *,
        times: int | None = None,
    ) -> None:
        async def invoke(route: async_api.Route, _request: async_api.Request) -> object:
            sync_route = Route(self._bridge, route, self._context)
            try:
                try:
                    argument_count = len(inspect.signature(handler).parameters)
                except (TypeError, ValueError):
                    argument_count = 2
                outcome = await asyncio.to_thread(
                    handler,
                    *[sync_route, sync_route.request][:argument_count],
                )
                if inspect.isawaitable(outcome):
                    outcome = await cast(Awaitable[object], outcome)
                await sync_route._finish_callback_commands()
                return outcome
            except BaseException:
                sync_route._discard_callback_commands()
                raise

        self._bridge.run(self._value._route_with_key(url, invoke, handler, times=times))

    def unroute(
        self,
        url: str | Pattern[str] | Callable[[str], bool],
        handler: Callable[..., object] | None = None,
    ) -> None:
        self._bridge.run(self._value.unroute(url, handler))

    def route_from_har(
        self,
        har: str | Path,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        not_found: Literal["abort", "fallback"] | None = None,
        update: bool | None = None,
        update_content: Literal["attach", "embed"] | None = None,
        update_mode: Literal["full", "minimal"] | None = None,
    ) -> None:
        self._bridge.run(
            self._value.route_from_har(
                har,
                url=url,
                not_found=not_found,
                update=update,
                update_content=update_content,
                update_mode=update_mode,
            )
        )

    def unroute_all(
        self,
        *,
        behavior: Literal["default", "ignoreErrors", "wait"] | None = None,
    ) -> None:
        self._bridge.run(self._value.unroute_all(behavior=behavior))

    def add_script_tag(
        self,
        *,
        content: str | None = None,
        path: str | Path | None = None,
        type: str | None = None,
        url: str | None = None,
    ) -> ElementHandle:
        value = self._bridge.run(
            self._value.add_script_tag(content=content, path=path, type=type, url=url)
        )
        return ElementHandle(self._bridge, value, self)

    def add_style_tag(
        self,
        *,
        content: str | None = None,
        path: str | Path | None = None,
        url: str | None = None,
    ) -> ElementHandle:
        value = self._bridge.run(self._value.add_style_tag(content=content, path=path, url=url))
        return ElementHandle(self._bridge, value, self)

    def set_content(
        self,
        html: str,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> None:
        self._bridge.run(self._value.set_content(html, timeout=timeout, wait_until=wait_until))

    def emulate_media(
        self,
        *,
        media: str | None = None,
        color_scheme: str | None = None,
        reduced_motion: str | None = None,
        forced_colors: str | None = None,
        contrast: str | None = None,
    ) -> None:
        self._bridge.run(
            self._value.emulate_media(
                media=media,
                color_scheme=color_scheme,
                reduced_motion=reduced_motion,
                forced_colors=forced_colors,
                contrast=contrast,
            )
        )

    def click(
        self,
        selector: str,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
        force: bool | None = None,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(
            self._value.click(
                selector,
                button=button,
                click_count=click_count,
                force=force,
                modifiers=modifiers,
                position=position,
                strict=strict,
                timeout=timeout,
            )
        )

    def tap(
        self,
        selector: str,
        *,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.tap(
                selector,
                modifiers=modifiers,
                position=position,
                timeout=timeout,
                force=force,
                no_wait_after=no_wait_after,
                strict=strict,
                trial=trial,
            )
        )

    def dblclick(
        self,
        selector: str,
        *,
        timeout: float | None = None,
        strict: bool | None = None,
    ) -> None:
        self._bridge.run(self._value.dblclick(selector, timeout=timeout, strict=strict))

    def dispatch_event(
        self,
        selector: str,
        type: str,
        event_init: Mapping[str, object] | None = None,
        *,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(
            self._value.dispatch_event(
                selector,
                type,
                _async_event_init_argument(event_init),
                strict=strict,
                timeout=timeout,
            )
        )

    def locator(
        self,
        selector: str,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        return Locator(
            self._bridge,
            self._value.locator(
                selector,
                has_text=has_text,
                has_not_text=has_not_text,
                has=None if has is None else has._value,
                has_not=None if has_not is None else has_not._value,
            ),
            self,
        )

    def set_input_files(
        self,
        selector: str,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.set_input_files(
                selector,
                files,
                timeout=timeout,
                no_wait_after=no_wait_after,
                strict=strict,
            )
        )

    def frame_locator(self, selector: str) -> FrameLocator:
        return FrameLocator(
            self._bridge,
            self._value.frame_locator(selector),
            self,
        )

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_text(text, exact=exact), self)

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_label(text, exact=exact), self)

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_placeholder(text, exact=exact),
            self,
        )

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_alt_text(text, exact=exact),
            self,
        )

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return Locator(self._bridge, self._value.get_by_title(text, exact=exact), self)

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        return Locator(self._bridge, self._value.get_by_test_id(test_id), self)

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        return Locator(
            self._bridge,
            self._value.get_by_role(
                role,
                checked=checked,
                disabled=disabled,
                expanded=expanded,
                include_hidden=include_hidden,
                level=level,
                name=name,
                pressed=pressed,
                selected=selected,
                exact=exact,
            ),
            self,
        )

    def drag_and_drop(
        self,
        source: str,
        target: str,
        *,
        source_position: Mapping[str, float] | None = None,
        target_position: Mapping[str, float] | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        steps: int | None = None,
    ) -> None:
        self._bridge.run(
            self._value.drag_and_drop(
                source,
                target,
                source_position=source_position,
                target_position=target_position,
                force=force,
                no_wait_after=no_wait_after,
                strict=strict,
                timeout=timeout,
                trial=trial,
                steps=steps,
            )
        )

    def fill(
        self,
        selector: str,
        value: str,
        *,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.fill(
                selector,
                value,
                timeout=timeout,
                force=force,
                no_wait_after=no_wait_after,
            )
        )

    def select_option(
        self,
        selector: str,
        value: str | Sequence[str] | None = None,
        *,
        index: int | None = None,
        label: str | None = None,
        element: ElementHandle | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> list[str]:
        if element is not None and not isinstance(cast(object, element), ElementHandle):
            raise InvalidArgumentError("Page.select_option element must be an ElementHandle")
        async_element = None if element is None else cast(async_api.ElementHandle, element._value)
        return self._bridge.run(
            self._value.select_option(
                selector,
                value,
                index=index,
                label=label,
                element=async_element,
                force=force,
                no_wait_after=no_wait_after,
                strict=strict,
                timeout=timeout,
            )
        )

    def press(
        self,
        selector: str,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.press(
                selector,
                key,
                delay=delay,
                timeout=timeout,
                no_wait_after=no_wait_after,
            )
        )

    def type(
        self,
        selector: str,
        text: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.type(
                selector,
                text,
                delay=delay,
                timeout=timeout,
                no_wait_after=no_wait_after,
                strict=strict,
            )
        )

    def set_checked(
        self,
        selector: str,
        checked: bool,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.set_checked(
                selector,
                checked,
                force=force,
                position=position,
                timeout=timeout,
                trial=trial,
                no_wait_after=no_wait_after,
            )
        )

    def check(
        self,
        selector: str,
        *,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.check(
                selector,
                position=position,
                timeout=timeout,
                force=force,
                no_wait_after=no_wait_after,
                strict=strict,
                trial=trial,
            )
        )

    def uncheck(
        self,
        selector: str,
        *,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.uncheck(
                selector,
                position=position,
                timeout=timeout,
                force=force,
                no_wait_after=no_wait_after,
                strict=strict,
                trial=trial,
            )
        )

    def focus(self, selector: str, *, timeout: float | None = None) -> None:
        self._bridge.run(self._value.focus(selector, timeout=timeout))

    def input_value(self, selector: str, *, timeout: float | None = None) -> str:
        return self._bridge.run(self._value.input_value(selector, timeout=timeout))

    def is_hidden(self, selector: str, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_hidden(selector, timeout=timeout))

    def is_visible(self, selector: str, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_visible(selector, timeout=timeout))

    def is_checked(self, selector: str, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_checked(selector, timeout=timeout))

    def is_enabled(self, selector: str, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_enabled(selector, timeout=timeout))

    def is_disabled(self, selector: str, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_disabled(selector, timeout=timeout))

    def is_editable(self, selector: str, *, timeout: float | None = None) -> bool:
        return self._bridge.run(self._value.is_editable(selector, timeout=timeout))

    def text_content(self, selector: str, *, timeout: float | None = None) -> str | None:
        return self._bridge.run(self._value.text_content(selector, timeout=timeout))

    def inner_text(self, selector: str, *, timeout: float | None = None) -> str:
        return self._bridge.run(self._value.inner_text(selector, timeout=timeout))

    def inner_html(self, selector: str, *, timeout: float | None = None) -> str:
        return self._bridge.run(self._value.inner_html(selector, timeout=timeout))

    def get_attribute(
        self, selector: str, name: str, *, timeout: float | None = None
    ) -> str | None:
        return self._bridge.run(self._value.get_attribute(selector, name, timeout=timeout))

    def evaluate(self, expression: str, arg: object = None) -> object:
        value = _async_evaluation_argument(arg)
        return self._bridge.run(self._value.evaluate(expression, value))

    def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        value = _async_evaluation_argument(arg)
        return _wrap_handle(
            self._bridge,
            self._bridge.run(self._value.evaluate_handle(expression, value)),
            self,
        )

    def wait_for_function(
        self,
        expression: str,
        arg: object = None,
        *,
        polling: float | str | None = None,
        timeout: float | None = None,
    ) -> JSHandle:
        value = arg._value if isinstance(arg, JSHandle) else arg
        return _wrap_handle(
            self._bridge,
            self._bridge.run(
                self._value.wait_for_function(expression, value, polling=polling, timeout=timeout)
            ),
            self,
        )

    def query_selector(
        self,
        selector: str,
        *,
        strict: bool | None = None,
    ) -> ElementHandle | None:
        value = self._bridge.run(self._value.query_selector(selector, strict=strict))
        return None if value is None else ElementHandle(self._bridge, value, self)

    def query_selector_all(self, selector: str) -> list[ElementHandle]:
        return [
            ElementHandle(self._bridge, value, self)
            for value in self._bridge.run(self._value.query_selector_all(selector))
        ]

    def eval_on_selector(
        self,
        selector: str,
        expression: str,
        arg: object = None,
        *,
        timeout: float | None = None,
    ) -> object:
        return self._bridge.run(
            self._value.eval_on_selector(selector, expression, arg, timeout=timeout)
        )

    def eval_on_selector_all(
        self,
        selector: str,
        expression: str,
        arg: object = None,
    ) -> object:
        return self._bridge.run(self._value.eval_on_selector_all(selector, expression, arg))

    def goto(
        self,
        url: str,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
        referer: str | None = None,
    ) -> Response | None:
        value = self._bridge.run(
            self._value.goto(url, timeout=timeout, wait_until=wait_until, referer=referer)
        )
        return None if value is None else Response(self._bridge, value)

    def reload(
        self,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> Response | None:
        value = self._bridge.run(self._value.reload(timeout=timeout, wait_until=wait_until))
        return None if value is None else Response(self._bridge, value)

    def go_back(
        self,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> Response | None:
        value = self._bridge.run(self._value.go_back(timeout=timeout, wait_until=wait_until))
        return None if value is None else Response(self._bridge, value)

    def go_forward(
        self,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> Response | None:
        value = self._bridge.run(self._value.go_forward(timeout=timeout, wait_until=wait_until))
        return None if value is None else Response(self._bridge, value)

    def wait_for_url(
        self,
        url: str | Pattern[str] | Callable[[str], bool],
        *,
        wait_until: str | None = None,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(self._value.wait_for_url(url, wait_until=wait_until, timeout=timeout))

    def wait_for_load_state(
        self,
        state: str | None = None,
        *,
        timeout: float | None = None,
    ) -> None:
        self._bridge.run(self._value.wait_for_load_state(state, timeout=timeout))

    def expect_navigation(
        self,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        wait_until: str | None = None,
        timeout: float | None = None,
    ) -> NavigationContextManager:
        return NavigationContextManager(
            self._bridge,
            self._value.expect_navigation(url=url, wait_until=wait_until, timeout=timeout),
            self,
        )

    def expect_event(
        self,
        event: str,
        predicate: async_api.NetworkMatcher | async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[SyncNetworkValue] | PageEventContextManager:
        wrapped_predicate = predicate
        if event in {"frameattached", "framenavigated"} and predicate is not None:

            def wrap_frame(value: object) -> object:
                return predicate(self._wrap_page_event(value))

            wrapped_predicate = wrap_frame
        value = self._value.expect_event(event, wrapped_predicate, timeout=timeout)
        if isinstance(value, async_api.PageEventContextManager):
            return PageEventContextManager(self._bridge, value, self)
        return EventContextManager(self._bridge, value, self._context)

    def expect_popup(
        self,
        predicate: async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        return PageEventContextManager(
            self._bridge, self._value.expect_popup(predicate, timeout=timeout), self
        )

    def expect_worker(
        self,
        predicate: async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        wrapped_predicate: async_api.PageEventPredicate | None = None
        if predicate is not None:

            def wrap_worker(value: object) -> object:
                return predicate(self._wrap_page_event(value))

            wrapped_predicate = wrap_worker
        return PageEventContextManager(
            self._bridge,
            self._value.expect_worker(wrapped_predicate, timeout=timeout),
            self,
        )

    def expect_console_message(
        self,
        predicate: async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        wrapped_predicate: async_api.PageEventPredicate | None = None
        if predicate is not None:

            def wrap_console(value: object) -> object:
                return predicate(self._wrap_page_event(value))

            wrapped_predicate = wrap_console

        return PageEventContextManager(
            self._bridge,
            self._value.expect_console_message(wrapped_predicate, timeout=timeout),
            self,
        )

    def expect_download(
        self,
        predicate: async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        wrapped_predicate: async_api.PageEventPredicate | None = None
        if predicate is not None:

            def wrap_download(value: object) -> object:
                return predicate(self._wrap_page_event(value))

            wrapped_predicate = wrap_download

        return PageEventContextManager(
            self._bridge,
            self._value.expect_download(wrapped_predicate, timeout=timeout),
            self,
        )

    def expect_file_chooser(
        self,
        predicate: async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        return PageEventContextManager(
            self._bridge,
            self._value.expect_file_chooser(predicate, timeout=timeout),
            self,
        )

    def expect_request(
        self,
        url_or_predicate: async_api.NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[Request]:
        value = cast(
            async_api.EventContextManager[async_api.NetworkValue],
            self._value.expect_request(url_or_predicate, timeout=timeout),
        )
        return EventContextManager(self._bridge, value, self._context)

    def expect_response(
        self,
        url_or_predicate: async_api.NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[Response]:
        value = cast(
            async_api.EventContextManager[async_api.NetworkValue],
            self._value.expect_response(url_or_predicate, timeout=timeout),
        )
        return EventContextManager(self._bridge, value, self._context)

    def wait_for_event(
        self,
        event: str,
        predicate: async_api.NetworkMatcher | async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> object:
        value = self._bridge.run(self._value.wait_for_event(event, predicate, timeout=timeout))
        if isinstance(value, async_api.Page):
            return self._context._wrap_page(value)
        if isinstance(value, async_api.Response):
            return self._context._wrap_response(value)
        if isinstance(value, async_api.Request):
            return self._context._wrap_request(value)
        if isinstance(value, async_api.Dialog):
            return self._wrap_dialog(value)
        if isinstance(value, async_api.FileChooser):
            return self._wrap_file_chooser(value)
        return value

    def wait_for_request(
        self,
        url_or_predicate: async_api.NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> Request:
        return self._context._wrap_request(
            self._bridge.run(self._value.wait_for_request(url_or_predicate, timeout=timeout))
        )

    def wait_for_response(
        self,
        url_or_predicate: async_api.NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> Response:
        return self._context._wrap_response(
            self._bridge.run(self._value.wait_for_response(url_or_predicate, timeout=timeout))
        )

    def close(self, *, run_before_unload: bool = False, reason: str | None = None) -> None:
        self._bridge.run(self._value.close(run_before_unload=run_before_unload, reason=reason))


class Tracing:
    def __init__(self, bridge: _LoopThread, value: async_api.Tracing) -> None:
        self._bridge = bridge
        self._value = value

    def stop(self, *, path: str | Path | None = None) -> None:
        self._bridge.run(self._value.stop(path=path))


class CDPSession:
    def __init__(self, bridge: _LoopThread, value: async_api.CDPSession) -> None:
        self._bridge = bridge
        self._value = value
        self._event_handlers: list[
            tuple[str, Callable[[dict[str, object]], object], Callable[[dict[str, object]], object]]
        ] = []

    def send(self, method: str, params: Mapping[str, object] | None = None) -> dict[str, object]:
        return self._bridge.run(self._value.send(method, params))

    def detach(self) -> None:
        self._bridge.run(self._value.detach())

    def on(self, event: str, handler: Callable[[dict[str, object]], object]) -> None:
        def invoke(params: dict[str, object]) -> object:
            try:
                return handler(params)
            except BaseException as error:
                self._bridge._record_event_error(error)
                raise

        async def register() -> None:
            self._event_handlers.append((event, handler, invoke))
            try:
                self._value.on(event, invoke)
            except BaseException:
                self._event_handlers = [
                    registration
                    for registration in self._event_handlers
                    if registration[2] is not invoke
                ]
                raise

        self._bridge.run(register())

    def once(self, event: str, handler: Callable[[dict[str, object]], object]) -> None:
        def invoke(params: dict[str, object]) -> object:
            self._event_handlers = [
                registration
                for registration in self._event_handlers
                if registration[2] is not invoke
            ]
            try:
                return handler(params)
            except BaseException as error:
                self._bridge._record_event_error(error)
                raise

        async def register() -> None:
            self._event_handlers.append((event, handler, invoke))
            try:
                self._value.once(event, invoke)
            except BaseException:
                self._event_handlers = [
                    registration
                    for registration in self._event_handlers
                    if registration[2] is not invoke
                ]
                raise

        self._bridge.run(register())

    def remove_listener(self, event: str, handler: Callable[[dict[str, object]], object]) -> None:
        async def remove() -> None:
            removed = [
                invoke
                for registered_event, registered_handler, invoke in self._event_handlers
                if registered_event == event and registered_handler is handler
            ]
            self._event_handlers = [
                registration
                for registration in self._event_handlers
                if not (registration[0] == event and registration[1] is handler)
            ]
            for invoke in removed:
                self._value.remove_listener(event, invoke)

        self._bridge.run(remove())


class BrowserContext:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.BrowserContext,
        browser: Browser,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._browser = browser
        self._pages: dict[int, Page] = {}
        self._requests: weakref.WeakValueDictionary[int, Request] = weakref.WeakValueDictionary()
        self._responses: weakref.WeakValueDictionary[int, Response] = weakref.WeakValueDictionary()
        self._event_handlers: list[
            tuple[str, Callable[[object], object], Callable[[object], object]]
        ] = []
        self._request_context: APIRequestContext | None = None
        self._remote = isinstance(value, async_api.RemoteBrowserContext)
        self._tracing = cast(Tracing, None) if self._remote else Tracing(bridge, value.tracing)

    def __enter__(self) -> BrowserContext:
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __repr__(self) -> str:
        return repr(self._value)

    @property
    def browser(self) -> Browser:
        return self._browser

    @property
    def tracing(self) -> Tracing:
        if self._remote:
            raise UnsupportedError("BrowserContext.tracing remote operation is not installed")
        return self._tracing

    @property
    def request(self) -> APIRequestContext:
        if self._remote:
            raise UnsupportedError("BrowserContext.request remote operation is not installed")
        if self._request_context is None:
            try:
                self._request_context = APIRequestContext(self._bridge, self._value.request)
            except UnsupportedError:
                pass
        if self._request_context is None:
            raise UnsupportedError(
                "BrowserContext.request requires the context-api-request profile"
            )
        return self._request_context

    def new_cdp_session(self, page: Page | Frame) -> CDPSession:
        if not isinstance(cast(object, page), (Page, Frame)):
            raise TypeError("BrowserContext.new_cdp_session requires a Page or Frame")
        return CDPSession(self._bridge, self._bridge.run(self._value.new_cdp_session(page._value)))

    def _wrap_page(self, value: async_api.Page) -> Page:
        return self._pages.setdefault(id(value), Page(self._bridge, value, self))

    def _wrap_request(self, value: async_api.Request) -> Request:
        key = id(value)
        request = self._requests.get(key)
        if request is None:
            request = Request(self._bridge, value, self)
            self._requests[key] = request
        return request

    def _wrap_response(self, value: async_api.Response) -> Response:
        key = id(value)
        response = self._responses.get(key)
        if response is None:
            response = Response(self._bridge, value, self)
            self._responses[key] = response
        return response

    def _wrap_context_event(self, value: object) -> object:
        if value is self._value:
            return self
        if isinstance(value, async_api.Page):
            return self._wrap_page(value)
        if isinstance(value, async_api.ConsoleMessage):
            return ConsoleMessage(value, self._wrap_page(value.page))
        if isinstance(value, async_api.Dialog):
            return self._wrap_page(value.page)._wrap_dialog(value)
        if isinstance(value, async_api.WebError):
            return WebError(value, self)
        return value

    def _invoke_event_handler(self, handler: Callable[[object], object], value: object) -> object:
        try:
            return handler(self._wrap_context_event(value))
        except BaseException as error:
            self._bridge._record_event_error(error)
            raise

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            if self._remote and event == "close":
                self._bridge.queue_remote_event(
                    lambda received: self._invoke_event_handler(handler, received), value
                )
                return None
            return self._invoke_event_handler(handler, value)

        self._event_handlers.append((event, handler, invoke))
        self._value.on(event, invoke)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self._event_handlers = [
                registration
                for registration in self._event_handlers
                if registration[2] is not invoke
            ]
            self._value.remove_listener(event, invoke)
            if self._remote and event == "close":
                self._bridge.queue_remote_event(
                    lambda received: self._invoke_event_handler(handler, received), value
                )
                return None
            return self._invoke_event_handler(handler, value)

        self._event_handlers.append((event, handler, invoke))
        self._value.on(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        removed = [
            invoke
            for registered_event, registered_handler, invoke in self._event_handlers
            if registered_event == event and registered_handler == handler
        ]
        self._event_handlers = [
            registration
            for registration in self._event_handlers
            if not (registration[0] == event and registration[1] == handler)
        ]
        for invoke in removed:
            self._value.remove_listener(event, invoke)

    @property
    def pages(self) -> list[Page]:
        values = self._value.pages
        return [self._wrap_page(value) for value in values]

    def new_page(self) -> Page:
        return self._wrap_page(self._bridge.run(self._value.new_page()))

    def set_default_timeout(self, timeout: float) -> None:
        self._value.set_default_timeout(timeout)

    def set_default_navigation_timeout(self, timeout: float) -> None:
        self._value.set_default_navigation_timeout(timeout)

    def cookies(self, urls: str | Sequence[str] | None = None) -> list[Cookie]:
        return self._bridge.run(self._value.cookies(urls))

    def add_cookies(self, cookies: Sequence[SetCookieParam]) -> None:
        self._bridge.run(self._value.add_cookies(cookies))

    def storage_state(
        self,
        *,
        path: str | PathLike[str] | None = None,
        indexed_db: bool = False,
    ) -> StorageState:
        return self._bridge.run(self._value.storage_state(path=path, indexed_db=indexed_db))

    def clear_cookies(
        self,
        *,
        name: str | Pattern[str] | None = None,
        domain: str | Pattern[str] | None = None,
        path: str | Pattern[str] | None = None,
    ) -> None:
        self._bridge.run(self._value.clear_cookies(name=name, domain=domain, path=path))

    def add_init_script(
        self,
        script: str | None = None,
        *,
        path: str | Path | None = None,
    ) -> None:
        self._bridge.run(self._value.add_init_script(script, path=path))

    def set_extra_http_headers(self, headers: Mapping[str, str]) -> None:
        self._bridge.run(self._value.set_extra_http_headers(headers))

    def set_offline(self, offline: bool) -> None:
        self._bridge.run(self._value.set_offline(offline))

    def expose_binding(
        self,
        name: str,
        callback: Callable[..., object],
        *,
        handle: bool | None = None,
    ) -> None:
        def invoke(source: object, *args: object) -> object:
            if not isinstance(source, dict):
                raise RuntimeError("BrowserContext binding source is unavailable")
            async_page = cast(async_api.Page, source["page"])
            async_frame = cast(async_api.Frame, source["frame"])
            page = self._wrap_page(async_page)
            sync_source = {
                "context": self,
                "page": page,
                "frame": page._wrap_frame(async_frame),
            }
            sync_arguments = [
                _wrap_handle(self._bridge, value, page)
                if isinstance(value, async_api.JSHandle)
                else value
                for value in args
            ]
            return callback(sync_source, *sync_arguments)

        self._bridge.run(self._value.expose_binding(name, invoke, handle=handle))

    def expose_function(self, name: str, callback: Callable[..., object]) -> None:
        self._bridge.run(self._value.expose_function(name, callback))

    def route(
        self,
        url: str | Pattern[str] | Callable[[str], bool],
        handler: Callable[..., object],
        *,
        times: int | None = None,
    ) -> None:
        async def invoke(route: async_api.Route, _request: async_api.Request) -> object:
            sync_route = Route(self._bridge, route, self)
            try:
                try:
                    argument_count = len(inspect.signature(handler).parameters)
                except (TypeError, ValueError):
                    argument_count = 2
                outcome = await asyncio.to_thread(
                    handler,
                    *[sync_route, sync_route.request][:argument_count],
                )
                if inspect.isawaitable(outcome):
                    outcome = await cast(Awaitable[object], outcome)
                await sync_route._finish_callback_commands()
                return outcome
            except BaseException:
                sync_route._discard_callback_commands()
                raise

        self._bridge.run(self._value._route_with_key(url, invoke, handler, times=times))

    def unroute(
        self,
        url: str | Pattern[str] | Callable[[str], bool],
        handler: Callable[..., object] | None = None,
    ) -> None:
        self._bridge.run(self._value.unroute(url, handler))

    def route_from_har(
        self,
        har: str | Path,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        not_found: Literal["abort", "fallback"] | None = None,
        update: bool | None = None,
        update_content: Literal["attach", "embed"] | None = None,
        update_mode: Literal["full", "minimal"] | None = None,
    ) -> None:
        self._bridge.run(
            self._value.route_from_har(
                har,
                url=url,
                not_found=not_found,
                update=update,
                update_content=update_content,
                update_mode=update_mode,
            )
        )

    def unroute_all(
        self,
        *,
        behavior: Literal["default", "ignoreErrors", "wait"] | None = None,
    ) -> None:
        self._bridge.run(self._value.unroute_all(behavior=behavior))

    def expect_event(
        self,
        event: str,
        predicate: async_api.NetworkMatcher | async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[SyncNetworkValue] | BrowserContextEventContextManager:
        value = self._value.expect_event(event, predicate, timeout=timeout)
        if isinstance(value, async_api.BrowserContextEventContextManager):
            return BrowserContextEventContextManager(self._bridge, value, self)
        return EventContextManager(self._bridge, value, self)

    def expect_console_message(
        self,
        predicate: async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> BrowserContextEventContextManager:
        wrapped_predicate: async_api.PageEventPredicate | None = None
        if predicate is not None:

            def wrap_console(value: object) -> object:
                wrapped = (
                    ConsoleMessage(value, self._wrap_page(value.page))
                    if isinstance(value, async_api.ConsoleMessage)
                    else value
                )
                return predicate(wrapped)

            wrapped_predicate = wrap_console

        return BrowserContextEventContextManager(
            self._bridge,
            self._value.expect_console_message(wrapped_predicate, timeout=timeout),
            self,
        )

    def expect_page(
        self,
        predicate: Callable[[Page], bool] | None = None,
        *,
        timeout: float | None = None,
    ) -> BrowserContextEventContextManager:
        wrapped_predicate: async_api.PageEventPredicate | None = None
        if predicate is not None:

            def wrap_page(value: object) -> object:
                wrapped = self._wrap_page(value) if isinstance(value, async_api.Page) else value
                return predicate(cast(Page, wrapped))

            wrapped_predicate = wrap_page

        return BrowserContextEventContextManager(
            self._bridge,
            self._value.expect_page(wrapped_predicate, timeout=timeout),
            self,
        )

    def wait_for_event(
        self,
        event: str,
        predicate: async_api.NetworkMatcher | async_api.PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> object:
        value = self._bridge.run(self._value.wait_for_event(event, predicate, timeout=timeout))
        if value is self._value:
            return self
        if isinstance(value, async_api.Page):
            return self._wrap_page(value)
        if isinstance(value, async_api.Response):
            return self._wrap_response(value)
        if isinstance(value, async_api.Request):
            return self._wrap_request(value)
        return self._wrap_context_event(value)

    def close(self, *, reason: str | None = None) -> None:
        self._bridge.run(self._value.close(reason=reason))


class Browser:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.Browser,
        browser_type: BrowserType,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._browser_type = browser_type
        self._remote = getattr(value, "_remote", False)
        self._contexts: dict[int, BrowserContext] = {}
        self._event_handlers: list[
            tuple[str, Callable[[object], object], Callable[[object], object]]
        ] = []

    def __enter__(self) -> Browser:
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def __repr__(self) -> str:
        return repr(self._value)

    @property
    def browser_type(self) -> BrowserType:
        return self._browser_type

    def _wrap_context(self, value: async_api.BrowserContext) -> BrowserContext:
        return self._contexts.setdefault(id(value), BrowserContext(self._bridge, value, self))

    @property
    def version(self) -> str:
        return self._value.version

    def is_connected(self) -> bool:
        return self._value.is_connected()

    def _invoke_event_handler(self, handler: Callable[[object], object], value: object) -> object:
        try:
            return handler(self if value is self._value else value)
        except BaseException as error:
            self._bridge._record_event_error(error)
            raise

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            if self._remote and event == "disconnected":
                self._bridge.queue_remote_event(
                    lambda received: self._invoke_event_handler(handler, received), value
                )
                return None
            return self._invoke_event_handler(handler, value)

        self._value.on(event, invoke)
        self._event_handlers.append((event, handler, invoke))

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self._event_handlers = [
                registration
                for registration in self._event_handlers
                if registration[2] is not invoke
            ]
            self._value.remove_listener(event, invoke)
            if self._remote and event == "disconnected":
                self._bridge.queue_remote_event(
                    lambda received: self._invoke_event_handler(handler, received), value
                )
                return None
            return self._invoke_event_handler(handler, value)

        self._value.on(event, invoke)
        self._event_handlers.append((event, handler, invoke))

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        removed = [
            invoke
            for registered_event, registered_handler, invoke in self._event_handlers
            if registered_event == event and registered_handler == handler
        ]
        self._event_handlers = [
            registration
            for registration in self._event_handlers
            if not (registration[0] == event and registration[1] == handler)
        ]
        for invoke in removed:
            self._value.remove_listener(event, invoke)

    @property
    def contexts(self) -> list[BrowserContext]:
        values = self._value.contexts
        return [self._wrap_context(value) for value in values]

    def new_context(
        self,
        *,
        base_url: str | None = None,
        viewport: Mapping[str, int] | None = None,
        no_viewport: bool = False,
        screen: Mapping[str, int] | None = None,
        user_agent: str | None = None,
        device_scale_factor: float | None = None,
        is_mobile: bool | None = None,
        has_touch: bool | None = None,
        default_browser_type: str | None = None,
        locale: str | None = None,
        timezone_id: str | None = None,
        geolocation: Mapping[str, float] | None = None,
        permissions: Sequence[str] | None = None,
        extra_http_headers: Mapping[str, str | None] | None = None,
        http_credentials: Mapping[str, object] | None = None,
        ignore_https_errors: bool = False,
        offline: bool = False,
        bypass_csp: bool = False,
        java_script_enabled: bool = True,
        accept_downloads: bool = False,
        service_workers: Literal["allow", "block"] = "allow",
        storage_state: Mapping[str, object] | str | PathLike[str] | None = None,
        forced_colors: Literal["active", "none"] | None = None,
        strict_selectors: bool = False,
        **kwargs: object,
    ) -> BrowserContext:
        return self._wrap_context(
            self._bridge.run(
                self._value.new_context(
                    base_url=base_url,
                    viewport=viewport,
                    no_viewport=no_viewport,
                    screen=screen,
                    user_agent=user_agent,
                    device_scale_factor=device_scale_factor,
                    is_mobile=is_mobile,
                    has_touch=has_touch,
                    default_browser_type=default_browser_type,
                    locale=locale,
                    timezone_id=timezone_id,
                    geolocation=geolocation,
                    permissions=permissions,
                    extra_http_headers=extra_http_headers,
                    http_credentials=http_credentials,
                    ignore_https_errors=ignore_https_errors,
                    offline=offline,
                    bypass_csp=bypass_csp,
                    java_script_enabled=java_script_enabled,
                    accept_downloads=accept_downloads,
                    service_workers=service_workers,
                    storage_state=storage_state,
                    forced_colors=forced_colors,
                    strict_selectors=strict_selectors,
                    **kwargs,
                )
            )
        )

    def new_page(
        self,
        *,
        base_url: str | None = None,
        accept_downloads: bool = False,
        **kwargs: object,
    ) -> Page:
        value = self._bridge.run(
            self._value.new_page(base_url=base_url, accept_downloads=accept_downloads, **kwargs)
        )
        return self._wrap_context(value.context)._wrap_page(value)

    def close(self, *, reason: str | None = None) -> None:
        self._bridge.run(self._value.close(reason=reason))


class BrowserType:
    def __init__(self, bridge: _LoopThread, value: async_api.BrowserType) -> None:
        self._bridge = bridge
        self._value = value

    def __repr__(self) -> str:
        return repr(self._value)

    @property
    def name(self) -> str:
        return self._value.name

    @property
    def executable_path(self) -> str:
        return self._value.executable_path

    def connect(
        self,
        ws_endpoint: str,
        *,
        timeout: float | None = None,
        slow_mo: float | None = None,
        headers: Mapping[str, str] | None = None,
        expose_network: str | None = None,
    ) -> Browser:
        value = self._bridge.run(
            self._value.connect(
                ws_endpoint,
                timeout=timeout,
                slow_mo=slow_mo,
                headers=headers,
                expose_network=expose_network,
            )
        )
        return Browser(self._bridge, value, self)

    def connect_over_cdp(
        self,
        endpoint_url: str,
        *,
        timeout: float | None = None,
        slow_mo: float | None = None,
        headers: Mapping[str, str] | None = None,
        is_local: bool | None = None,
    ) -> Browser:
        value = self._bridge.run(
            self._value.connect_over_cdp(
                endpoint_url,
                timeout=timeout,
                slow_mo=slow_mo,
                headers=headers,
                is_local=is_local,
            )
        )
        return Browser(self._bridge, value, self)

    def launch(
        self,
        *,
        executable_path: str | Path | None = None,
        channel: str | None = None,
        headless: bool | None = None,
        timeout: float | None = None,
        args: Sequence[str] | None = None,
        **kwargs: object,
    ) -> Browser:
        value = self._bridge.run(
            self._value.launch(
                executable_path=executable_path,
                channel=channel,
                headless=headless,
                timeout=timeout,
                args=args,
                **kwargs,
            )
        )
        return Browser(self._bridge, value, self)

    def launch_persistent_context(
        self,
        user_data_dir: str | Path,
        *,
        headless: bool | None = None,
        base_url: str | None = None,
        extra_http_headers: Mapping[str, str] | None = None,
        locale: str | None = None,
        timezone_id: str | None = None,
        geolocation: Mapping[str, float] | None = None,
        permissions: Sequence[str] | None = None,
        http_credentials: Mapping[str, object] | None = None,
        ignore_https_errors: bool = False,
        offline: bool = False,
        bypass_csp: bool = False,
        java_script_enabled: bool = True,
        viewport: Mapping[str, int] | None = None,
        screen: Mapping[str, int] | None = None,
        user_agent: str | None = None,
        device_scale_factor: float | None = None,
        is_mobile: bool | None = None,
        has_touch: bool | None = None,
        color_scheme: Literal["dark", "light", "no-preference"] | None = None,
        reduced_motion: Literal["no-preference", "reduce"] | None = None,
        contrast: Literal["more", "no-preference"] | None = None,
        args: Sequence[str] | None = None,
        executable_path: str | Path | None = None,
        channel: str | None = None,
        timeout: float | None = None,
        **kwargs: object,
    ) -> BrowserContext:
        value = self._bridge.run(
            self._value.launch_persistent_context(
                user_data_dir,
                headless=headless,
                base_url=base_url,
                extra_http_headers=extra_http_headers,
                locale=locale,
                timezone_id=timezone_id,
                geolocation=geolocation,
                permissions=permissions,
                http_credentials=http_credentials,
                ignore_https_errors=ignore_https_errors,
                offline=offline,
                bypass_csp=bypass_csp,
                java_script_enabled=java_script_enabled,
                viewport=viewport,
                screen=screen,
                user_agent=user_agent,
                device_scale_factor=device_scale_factor,
                is_mobile=is_mobile,
                has_touch=has_touch,
                color_scheme=color_scheme,
                reduced_motion=reduced_motion,
                contrast=contrast,
                args=args,
                executable_path=executable_path,
                channel=channel,
                timeout=timeout,
                **kwargs,
            )
        )
        browser = Browser(self._bridge, value.browser, self)
        return browser._wrap_context(value)


class Selectors:
    def __init__(self, bridge: _LoopThread, value: async_api.Selectors) -> None:
        self._bridge = bridge
        self._value = value

    def register(
        self,
        name: str,
        script: str | None = None,
        *,
        path: Path | str | None = None,
        content_script: bool | None = None,
    ) -> None:
        self._bridge.run(
            self._value.register(
                name,
                script,
                path=path,
                content_script=content_script,
            )
        )

    def set_test_id_attribute(self, attribute_name: str) -> None:
        self._value.set_test_id_attribute(attribute_name)


class Playwright:
    def __init__(
        self,
        bridge: _LoopThread,
        value: async_api.Playwright,
        owner_thread_id: int,
        dispatcher: _CallerDispatcher,
    ) -> None:
        self._bridge = bridge
        self._value = value
        self._owner_thread_id = owner_thread_id
        self._dispatcher_fiber = dispatcher
        self._stopped = False
        self.chromium = BrowserType(bridge, value.chromium)
        self.firefox = BrowserType(bridge, value.firefox)
        self.webkit = BrowserType(bridge, value.webkit)
        self.selectors = Selectors(bridge, value.selectors)
        self.devices = value.devices
        self.request = APIRequest(bridge, value.request)

    def __getitem__(self, name: str) -> BrowserType:
        if name == "chromium":
            return self.chromium
        if name == "firefox":
            return self.firefox
        if name == "webkit":
            return self.webkit
        raise KeyError(name)

    def stop(self) -> None:
        if self._stopped:
            return
        self._stopped = True
        try:
            self._bridge.run(self._value.stop())
        finally:
            try:
                self._bridge.close()
            finally:
                try:
                    self._dispatcher_fiber.close()
                finally:
                    _release_sync_playwright_thread(self._owner_thread_id)


class PlaywrightContextManager:
    def __init__(self) -> None:
        self._bridge: _LoopThread | None = None
        self._playwright: Playwright | None = None
        self._dispatcher: _CallerDispatcher | None = None

    def start(self) -> Playwright:
        if self._playwright is not None:
            return self._playwright
        owner_thread_id = _claim_sync_playwright_thread()
        try:
            self._bridge = _LoopThread()
            value = self._bridge.run(async_api.async_playwright().start())
            self._dispatcher = _CallerDispatcher()
            self._bridge.install_caller_dispatcher(self._dispatcher)
            self._playwright = Playwright(
                self._bridge,
                value,
                owner_thread_id,
                self._dispatcher,
            )
        except BaseException:
            try:
                if self._bridge is not None:
                    self._bridge.close()
            finally:
                try:
                    if self._dispatcher is not None:
                        self._dispatcher.close()
                finally:
                    _release_sync_playwright_thread(owner_thread_id)
            raise
        return self._playwright

    def __enter__(self) -> Playwright:
        return self.start()

    def __exit__(self, *_exc: object) -> None:
        if self._playwright is not None:
            self._playwright.stop()
        elif self._bridge is not None:
            try:
                self._bridge.close()
            finally:
                if self._dispatcher is not None:
                    self._dispatcher.close()


def sync_playwright() -> PlaywrightContextManager:
    return PlaywrightContextManager()


__all__ = [
    "APIRequest",
    "APIRequestContext",
    "APIResponse",
    "ClientCertificate",
    "Cookie",
    "ConsoleMessage",
    "Dialog",
    "WebError",
    "FileChooser",
    "DeviceDescriptor",
    "Browser",
    "BrowserContext",
    "BrowserContextEventContextManager",
    "BrowserType",
    "Frame",
    "FrameLocator",
    "JSHandle",
    "ElementHandle",
    "FileInput",
    "FilePayload",
    "Locator",
    "LocatorAssertions",
    "Keyboard",
    "Mouse",
    "Touchscreen",
    "Page",
    "Playwright",
    "PlaywrightContextManager",
    "Request",
    "RequestSizes",
    "SecurityDetails",
    "ResourceTiming",
    "Response",
    "Selectors",
    "SetCookieParam",
    "Route",
    "StorageState",
    "StorageStateCookie",
    "Tracing",
    "ViewportSize",
    "SourceLocation",
    "EventContextManager",
    "PageEventContextManager",
    "Worker",
    "sync_playwright",
    "expect",
]
