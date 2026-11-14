# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import _native
from pwright._errors import Error, InvalidArgumentError, UnsupportedError
from pwright.async_api import ElementHandle, Page
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import _LoopThread


class _NativeLocator:
    def __init__(self) -> None:
        self.nth_index: int | None = None
        self.calls: list[tuple[list[object], int, bool | None, object]] = []
        self.error: Exception | None = None

    def nth(self, index: int) -> _NativeLocator:
        self.nth_index = index
        return self

    async def select_options(
        self,
        values: list[object],
        timeout_ms: int,
        force: bool | None,
        element: object,
    ) -> list[str]:
        self.calls.append((values, timeout_ms, force, element))
        if self.error is not None:
            raise self.error
        return ["blue", "green"]


class _NativeSession:
    def __init__(self) -> None:
        self.locator_value = _NativeLocator()
        self.locator_calls: list[tuple[str, str]] = []

    def locator(self, frame_id: str, selector: str) -> _NativeLocator:
        self.locator_calls.append((frame_id, selector))
        return self.locator_value

    @staticmethod
    def page_default_timeout(_page_id: str, _navigation: bool) -> int:
        return 900


class _Browser:
    def __init__(self, native: _NativeSession) -> None:
        self._native = native


class _Context:
    def __init__(self, native: _NativeSession) -> None:
        self.browser = _Browser(native)


class _NativeHandle:
    @staticmethod
    def is_element() -> bool:
        return True

    @staticmethod
    def preview() -> str:
        return "option"


def _selected_features() -> list[str]:
    return ["page-select-option"]


def _no_features() -> list[str]:
    return []


@pytest.mark.asyncio
async def test_page_select_option_combines_overloads_and_uses_first_selector_match(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected_features)
    native = _NativeSession()
    page = Page(cast(Any, _Context(native)), "page-1", "frame-1", "about:blank")

    # Act
    selected = await page.select_option(
        "select", "blue", label="Green", index=4, force=True, timeout=45, no_wait_after=False
    )

    # Assert
    assert selected == ["blue", "green"]
    assert native.locator_calls == [("frame-1", "select")]
    assert native.locator_value.nth_index == 0
    assert native.locator_value.calls == [
        (["blue", {"label": "Green"}, {"index": 4}], 45, True, None)
    ]


@pytest.mark.asyncio
async def test_page_select_option_forwards_empty_values_and_exact_element_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected_features)
    native = _NativeSession()
    page = Page(cast(Any, _Context(native)), "page-1", "frame-1", "about:blank")
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, native), cast(Any, owner), page)

    # Act
    await page.select_option("select", [], element=element)

    # Assert
    assert native.locator_value.calls == [([], 900, None, owner)]


@pytest.mark.asyncio
async def test_page_select_option_maps_invalid_list_item_to_playwright_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected_features)
    native = _NativeSession()
    native.locator_value.error = InvalidArgumentError("expected string, got object")
    page = Page(cast(Any, _Context(native)), "page-1", "frame-1", "about:blank")

    # Act
    with pytest.raises(Error, match="expected string, got object"):
        await page.select_option("select", cast(Any, ["blue", None]))

    # Assert
    assert native.locator_value.calls == [(["blue", None], 900, None, None)]


@pytest.mark.asyncio
async def test_page_select_option_requires_the_opt_in_profile(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _no_features)
    native = _NativeSession()
    page = Page(cast(Any, _Context(native)), "page-1", "frame-1", "about:blank")

    # Act
    with pytest.raises(UnsupportedError, match="page-select-option profile"):
        await page.select_option("select", "blue")

    # Assert
    assert native.locator_calls == []


def test_sync_page_select_option_reuses_async_rust_boundary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected_features)
    native = _NativeSession()
    async_page = Page(cast(Any, _Context(native)), "page-1", "frame-1", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, object()))

    # Act
    try:
        selected = page.select_option("select", label="Green", timeout=51)
    finally:
        bridge.close()

    # Assert
    assert selected == ["blue", "green"]
    assert native.locator_value.calls == [([{"label": "Green"}], 51, None, None)]
