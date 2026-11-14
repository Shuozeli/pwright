# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import _native
from pwright._errors import InvalidArgumentError, UnsupportedError
from pwright.async_api import ElementHandle, Page
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import _LoopThread


class _NativeLocator:
    def __init__(self) -> None:
        self.calls: list[tuple[int, dict[str, object]]] = []

    async def tap(self, timeout_ms: int, options: dict[str, object]) -> None:
        self.calls.append((timeout_ms, options))


class _NativeSession:
    def __init__(self) -> None:
        self.locator_value = _NativeLocator()
        self.page_calls: list[tuple[str, str, dict[str, object], bool, int]] = []
        self.touch_calls: list[tuple[str, float, float]] = []
        self.handle_calls: list[tuple[object, dict[str, object], int]] = []

    def locator(self, _frame_id: str, _selector: str) -> _NativeLocator:
        return self.locator_value

    def pages(self, _context_id: str) -> list[tuple[str, str, str, str]]:
        return [("page-1", "context-1", "frame-1", "about:blank")]

    async def page_tap(
        self,
        frame_id: str,
        selector: str,
        options: dict[str, object],
        strict: bool,
        timeout_ms: int,
    ) -> None:
        self.page_calls.append((frame_id, selector, options, strict, timeout_ms))

    async def touchscreen_tap(self, page_id: str, x: float, y: float) -> None:
        self.touch_calls.append((page_id, x, y))

    async def element_handle_tap(
        self, handle: object, options: dict[str, object], timeout_ms: int
    ) -> None:
        self.handle_calls.append((handle, options, timeout_ms))


class _Context:
    _object_id = "context-1"

    def __init__(self, native: _NativeSession) -> None:
        self.browser = cast(Any, type("Browser", (), {"_native": native})())


class _NativeHandle:
    @staticmethod
    def preview() -> str:
        return "button"

    @staticmethod
    def is_element() -> bool:
        return True


def _page(native: _NativeSession) -> Page:
    return Page(cast(Any, _Context(native)), "page-1", "frame-1", "about:blank")


def test_installed_touchscreen_profile_exports_all_four_native_methods() -> None:
    # Arrange
    features = _native.selected_features()

    # Act
    methods = (
        hasattr(_native.NativeSession, "page_tap"),
        hasattr(_native.NativeLocator, "tap"),
        hasattr(_native.NativeSession, "element_handle_tap"),
        hasattr(_native.NativeSession, "touchscreen_tap"),
    )

    # Assert
    assert "touchscreen-input" in features
    assert "element-handle-tap" in features
    assert methods == (True, True, True, True)


@pytest.mark.asyncio
async def test_page_tap_forwards_exact_options_and_strict_mode(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["touchscreen-input"])
    native = _NativeSession()
    page = _page(native)

    # Act
    await page.tap(
        "button", modifiers=["Alt"], position={"x": 3.5, "y": 4}, strict=True, timeout=42
    )

    # Assert
    assert native.page_calls == [
        (
            "frame-1",
            "button",
            {"modifiers": ["Alt"], "position": {"x": 3.5, "y": 4.0}},
            True,
            42,
        )
    ]


@pytest.mark.asyncio
async def test_locator_and_touchscreen_tap_use_distinct_native_commands(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["touchscreen-input"])
    native = _NativeSession()
    page = _page(native)

    # Act
    await page.locator("button").tap(force=True, timeout=21)
    await page.touchscreen.tap(40, 60.5)

    # Assert
    assert native.locator_value.calls == [(21, {"force": True})]
    assert native.touch_calls == [("page-1", 40.0, 60.5)]


@pytest.mark.asyncio
async def test_element_handle_tap_preserves_retained_identity(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        _native, "selected_features", lambda: ["touchscreen-input", "element-handle-tap"]
    )
    native = _NativeSession()
    page = _page(native)
    retained = _NativeHandle()
    handle = ElementHandle(cast(Any, native), cast(Any, retained), page)

    # Act
    await handle.tap(trial=True, timeout=15)

    # Assert
    assert native.handle_calls == [(retained, {"trial": True}, 15)]


@pytest.mark.asyncio
async def test_tap_rejects_invalid_arguments_before_native_dispatch(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["touchscreen-input"])
    native = _NativeSession()
    page = _page(native)

    # Act
    with pytest.raises(InvalidArgumentError, match="unknown name"):
        await page.tap("button", modifiers=cast(Any, ["Unknown"]))
    with pytest.raises(ValueError, match="finite"):
        await page.touchscreen.tap(float("nan"), 1)

    # Assert
    assert native.page_calls == []
    assert native.touch_calls == []


@pytest.mark.asyncio
async def test_touchscreen_requires_explicit_capability(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: cast(list[str], []))
    native = _NativeSession()
    page = _page(native)

    # Act
    with pytest.raises(UnsupportedError, match="touchscreen-input profile"):
        await page.touchscreen.tap(1, 2)

    # Assert
    assert native.touch_calls == []


def test_sync_page_and_touchscreen_tap_reuse_async_boundary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["touchscreen-input"])
    native = _NativeSession()
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(native), cast(Any, object()))

    # Act
    try:
        page.tap("button", timeout=12)
        page.touchscreen.tap(2, 3)
    finally:
        bridge.close()

    # Assert
    assert native.page_calls == [("frame-1", "button", {}, False, 12)]
    assert native.touch_calls == [("page-1", 2.0, 3.0)]
