# pyright: reportPrivateUsage=false

from __future__ import annotations

import asyncio
from typing import Any, Optional, cast

import pytest
from pwright.async_api import Browser, BrowserContext, Page, Route
from pwright.sync_api import BrowserContext as SyncBrowserContext
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import Route as SyncRoute
from pwright.sync_api import _LoopThread

_URL = "https://example.test/empty.html"
_RouteCall = tuple[
    str,
    str,
    str,
    str,
    Optional[str],
    str,
    str,
    str,
    list[tuple[str, str]],
    Optional[bytes],
    bool,
    bool,
    Optional[str],
]


def _route_call(url: str) -> _RouteCall:
    return (
        "call-11",
        "11",
        "context-11",
        "page-11",
        "frame-11",
        url,
        "GET",
        "document",
        [],
        None,
        True,
        False,
        None,
    )


class _Native:
    def __init__(self) -> None:
        self.browser: Browser | None = None
        self.calls: list[tuple[str, str]] = []
        self.dispatch_on_next_goto = True

    async def page_route(self, _page_id: str, _pattern: str, _times: int | None) -> str:
        self.calls.append(("route", "11"))
        return "11"

    async def page_route_continue(self, call_id: str, *_options: object) -> None:
        self.calls.append(("continue", call_id))

    async def page_route_abort(self, call_id: str, _error_code: str | None) -> None:
        self.calls.append(("abort", call_id))

    async def goto(
        self,
        _page_id: str,
        url: str,
        _wait_until: str,
        _referer: str | None,
        _timeout_ms: int,
    ) -> dict[str, object]:
        self.calls.append(("goto", url))
        if self.dispatch_on_next_goto:
            self.dispatch_on_next_goto = False
            assert self.browser is not None
            await self.browser._dispatch_page_route(_route_call(url))
        return {"url": url, "response": None}


def _page_with_routed_native() -> tuple[_Native, Browser, BrowserContext, Page]:
    native = _Native()
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    browser._contexts = {}
    browser._deferred_route_error = None

    async def no_network_notices() -> None:
        return None

    browser._flush_network_notices = cast(Any, no_network_notices)
    context = BrowserContext(browser, "context-11", None)
    page = Page(context, "page-11", "frame-11", "about:blank")
    browser._contexts["context-11"] = context
    context._pages["page-11"] = page
    native.browser = browser
    return native, browser, context, page


@pytest.mark.asyncio
async def test_async_route_error_after_continue_reaches_next_goto_once() -> None:
    # Arrange
    native, browser, _context, page = _page_with_routed_native()
    reported: list[dict[str, Any]] = []
    loop = asyncio.get_running_loop()
    prior_handler = loop.get_exception_handler()
    loop.set_exception_handler(lambda _loop, details: reported.append(details))

    async def handler(route: Route) -> None:
        await route.continue_()
        raise Exception("Exception text!?")

    try:
        await page.route("*/**", handler)

        # Act
        assert await page.goto(_URL, timeout=100) is None
        with pytest.raises(Exception, match=r"Exception text!\?") as raised:
            await page.goto(_URL, timeout=100)
        assert await page.goto(_URL, timeout=100) is None
    finally:
        loop.set_exception_handler(prior_handler)

    # Assert
    assert native.calls == [
        ("route", "11"),
        ("goto", _URL),
        ("continue", "call-11"),
        ("goto", _URL),
    ]
    assert len(reported) == 1
    assert reported[0]["exception"] is raised.value
    assert browser._deferred_route_error is None


def test_sync_route_error_after_continue_reaches_next_goto_once() -> None:
    # Arrange
    native, browser, context, async_page = _page_with_routed_native()
    bridge = _LoopThread()
    sync_context = SyncBrowserContext(bridge, context, cast(Any, None))
    page = SyncPage(bridge, async_page, sync_context)

    async def silence_expected_callback_error() -> None:
        asyncio.get_running_loop().set_exception_handler(lambda _loop, _details: None)

    def handler(route: SyncRoute) -> None:
        route.continue_()
        raise Exception("Exception text!?")

    try:
        bridge.run(silence_expected_callback_error())
        page.route("*/**", handler)

        # Act
        assert page.goto(_URL, timeout=100) is None
        with pytest.raises(Exception, match=r"Exception text!\?"):
            page.goto(_URL, timeout=100)
        assert page.goto(_URL, timeout=100) is None
    finally:
        bridge.close()

    # Assert
    assert native.calls == [
        ("route", "11"),
        ("goto", _URL),
        ("continue", "call-11"),
        ("goto", _URL),
    ]
    assert browser._deferred_route_error is None


@pytest.mark.asyncio
async def test_route_dispatch_cancellation_is_not_deferred() -> None:
    # Arrange
    native, browser, _context, page = _page_with_routed_native()
    native.dispatch_on_next_goto = False
    started = asyncio.Event()
    parked = asyncio.Event()

    async def handler(_route: Route) -> None:
        started.set()
        await parked.wait()

    await page.route("*/**", handler)
    dispatch = asyncio.create_task(browser._dispatch_page_route(_route_call(_URL)))
    await started.wait()

    # Act
    dispatch.cancel()
    with pytest.raises(asyncio.CancelledError):
        await dispatch

    # Assert
    assert native.calls == [("route", "11"), ("abort", "call-11")]
    assert browser._deferred_route_error is None


@pytest.mark.asyncio
async def test_route_dispatch_cancellation_after_continue_does_not_abort_or_defer() -> None:
    # Arrange
    native, browser, _context, page = _page_with_routed_native()
    started = asyncio.Event()
    parked = asyncio.Event()

    async def handler(route: Route) -> None:
        await route.continue_()
        started.set()
        await parked.wait()

    await page.route("*/**", handler)
    dispatch = asyncio.create_task(browser._dispatch_page_route(_route_call(_URL)))
    await started.wait()

    # Act
    dispatch.cancel()
    with pytest.raises(asyncio.CancelledError):
        await dispatch

    # Assert
    assert native.calls == [("route", "11"), ("continue", "call-11")]
    assert browser._deferred_route_error is None
