# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import UnsupportedError
from pwright.async_api import Page
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import _LoopThread


class _NativeLocator:
    def __init__(self, observed: list[tuple[object, ...]]) -> None:
        self.observed = observed

    def nth(self, index: int) -> _NativeLocator:
        self.observed.append(("nth", index))
        return self

    async def set_checked(
        self,
        checked: bool,
        timeout_ms: int,
        force: bool | None,
        trial: bool | None,
    ) -> None:
        self.observed.append(("set_checked", checked, timeout_ms, force, trial))


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
async def test_async_page_check_uses_first_match_and_forwards_options() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)

    # Act
    await page.check("input", force=True, timeout=75, trial=True)

    # Assert
    assert observed == [
        ("locator", "frame-2", "input"),
        ("nth", 0),
        ("set_checked", True, 75, True, True),
    ]


@pytest.mark.asyncio
async def test_async_page_uncheck_with_strict_does_not_select_first_match() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)

    # Act
    await page.uncheck("input", strict=True)

    # Assert
    assert observed == [
        ("locator", "frame-2", "input"),
        ("page_default_timeout", "page-1", False),
        ("set_checked", False, 1234, None, None),
    ]


@pytest.mark.asyncio
async def test_async_page_check_rejects_uninstalled_position_without_native_action() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)

    # Act
    with pytest.raises(UnsupportedError, match="position"):
        await page.check("input", position={"x": 1, "y": 2})

    # Assert
    assert observed == [("locator", "frame-2", "input"), ("nth", 0)]


@pytest.mark.asyncio
async def test_async_page_uncheck_rejects_invalid_strict_before_native_action() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    page = _page(observed)

    # Act
    with pytest.raises(TypeError, match="Page.uncheck strict"):
        await page.uncheck("input", strict=cast(Any, "yes"))

    # Assert
    assert observed == [("locator", "frame-2", "input")]


def test_sync_page_check_uses_async_owner_and_preserves_options() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(observed), cast(Any, None))

    # Act
    try:
        page.check("input", force=False, timeout=40, trial=False, strict=True)
    finally:
        bridge.close()

    # Assert
    assert observed == [
        ("locator", "frame-2", "input"),
        ("set_checked", True, 40, False, False),
    ]


def test_sync_page_uncheck_uses_first_match_by_default() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(observed), cast(Any, None))

    # Act
    try:
        page.uncheck("input")
    finally:
        bridge.close()

    # Assert
    assert observed == [
        ("locator", "frame-2", "input"),
        ("nth", 0),
        ("page_default_timeout", "page-1", False),
        ("set_checked", False, 1234, None, None),
    ]
