# pyright: reportPrivateUsage=false

from __future__ import annotations

from types import MappingProxyType
from typing import Any, cast

import pytest
from pwright.async_api import JSHandle, Page
from pwright.sync_api import JSHandle as SyncJSHandle
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import _LoopThread


class _NativeLocator:
    def __init__(self, observed: list[tuple[object, ...]]) -> None:
        self.observed = observed

    def nth(self, index: int) -> _NativeLocator:
        self.observed.append(("nth", index))
        return self

    async def dispatch_event(
        self, event_type: str, event_init: dict[str, object] | None, timeout_ms: int
    ) -> None:
        self.observed.append(("dispatch_event", event_type, event_init, timeout_ms))

    async def dispatch_event_owned(
        self, event_type: str, event_init: dict[str, object], timeout_ms: int
    ) -> None:
        self.observed.append(("dispatch_event_owned", event_type, event_init, timeout_ms))


class _NativeHandle:
    @staticmethod
    def is_element() -> bool:
        return False

    @staticmethod
    def preview() -> str:
        return "object"


class _Native:
    def __init__(self, observed: list[tuple[object, ...]]) -> None:
        self.observed = observed

    @staticmethod
    def context_selector_is_strict(context_id: str, strict: bool | None) -> bool:
        assert context_id == "context-1"
        return bool(strict)

    def locator(self, frame_id: str, selector: str) -> _NativeLocator:
        self.observed.append(("locator", frame_id, selector))
        return _NativeLocator(self.observed)

    def page_default_timeout(self, page_id: str, navigation: bool) -> int:
        self.observed.append(("page_default_timeout", page_id, navigation))
        return 1234


def _page(observed: list[tuple[object, ...]]) -> Page:
    class Browser:
        def __init__(self, native: _Native) -> None:
            self._native = native

    class Context:
        _object_id = "context-1"

        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    return Page(cast(Any, Context(Browser(_Native(observed)))), "page-1", "frame-2", "about:blank")


@pytest.mark.asyncio
async def test_async_page_dispatch_event_selects_first_match_by_default() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)

    # Act
    await page.dispatch_event("button", "click", {"bubbles": False})

    # Assert
    assert observed == [
        ("locator", "frame-2", "button"),
        ("nth", 0),
        ("page_default_timeout", "page-1", False),
        ("dispatch_event", "click", {"bubbles": False}, 1234),
    ]


@pytest.mark.asyncio
async def test_async_page_dispatch_event_strict_preserves_locator_selection() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)
    event_init = MappingProxyType({"detail": 2})

    # Act
    await page.dispatch_event("button", "click", event_init, strict=True, timeout=75)

    # Assert
    assert observed == [
        ("locator", "frame-2", "button"),
        ("dispatch_event", "click", {"detail": 2}, 75),
    ]


@pytest.mark.asyncio
async def test_async_page_dispatch_event_routes_nested_handle_through_owned_selection() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)
    owner = _NativeHandle()
    live_value = JSHandle(cast(Any, page._context.browser._native), cast(Any, owner))

    # Act
    await page.dispatch_event(
        "button", "drop", {"nested": [{"dataTransfer": live_value}]}, timeout=40
    )

    # Assert
    assert observed == [
        ("locator", "frame-2", "button"),
        ("nth", 0),
        ("dispatch_event_owned", "drop", {"nested": [{"dataTransfer": owner}]}, 40),
    ]


@pytest.mark.asyncio
async def test_async_page_dispatch_event_owned_preserves_strict_selection() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)
    owner = _NativeHandle()
    live_value = JSHandle(cast(Any, page._context.browser._native), cast(Any, owner))

    # Act
    await page.dispatch_event("button", "drop", {"dataTransfer": live_value}, strict=True)

    # Assert
    assert observed == [
        ("locator", "frame-2", "button"),
        ("page_default_timeout", "page-1", False),
        ("dispatch_event_owned", "drop", {"dataTransfer": owner}, 1234),
    ]


@pytest.mark.asyncio
async def test_async_page_dispatch_event_rejects_invalid_strict_before_native_action() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)

    # Act
    with pytest.raises(TypeError, match="Page.dispatch_event strict"):
        await page.dispatch_event("button", "click", strict=cast(bool, "yes"))

    # Assert
    assert observed == [("locator", "frame-2", "button")]


def test_sync_page_dispatch_event_uses_async_owner_and_first_match() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(observed), cast(Any, None))

    # Act
    try:
        page.dispatch_event("button", "click", {"detail": 3}, timeout=40)
    finally:
        bridge.close()

    # Assert
    assert observed == [
        ("locator", "frame-2", "button"),
        ("nth", 0),
        ("dispatch_event", "click", {"detail": 3}, 40),
    ]


def test_sync_page_dispatch_event_forwards_strict_selection() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(observed), cast(Any, None))

    # Act
    try:
        page.dispatch_event("button", "click", strict=True, timeout=40)
    finally:
        bridge.close()

    # Assert
    assert observed == [
        ("locator", "frame-2", "button"),
        ("dispatch_event", "click", None, 40),
    ]


def test_sync_page_dispatch_event_unwraps_nested_live_handle() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(observed), cast(Any, None))
    owner = _NativeHandle()
    live_value = SyncJSHandle(
        bridge,
        JSHandle(cast(Any, page._value._context.browser._native), cast(Any, owner)),
    )

    # Act
    try:
        page.dispatch_event("button", "drop", {"dataTransfer": live_value}, timeout=40)
    finally:
        bridge.close()

    # Assert
    assert observed == [
        ("locator", "frame-2", "button"),
        ("nth", 0),
        ("dispatch_event_owned", "drop", {"dataTransfer": owner}, 40),
    ]
