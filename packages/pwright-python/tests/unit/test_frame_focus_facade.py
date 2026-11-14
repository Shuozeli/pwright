# pyright: reportPrivateUsage=false
"""Frame focus delegates to the Rust-owned operation for the selected frame."""

from __future__ import annotations

import asyncio
from collections.abc import Coroutine
from typing import Any, cast

import pytest
from pwright import async_api
from pwright._errors import Error
from pwright.async_api import Browser, BrowserContext, Frame, Page
from pwright.sync_api import Frame as SyncFrame


class NativeFocusLocator:
    def __init__(self, native: NativeFocus, frame_id: str, selector: str) -> None:
        self.native = native
        self.frame_id = frame_id
        self.selector = selector

    def nth(self, index: int) -> NativeFocusLocator:
        self.native.calls.append(("nth", self.frame_id, self.selector, index))
        return self

    async def focus(self, timeout_ms: int) -> None:
        self.native.calls.append(("locator_focus", self.frame_id, self.selector, timeout_ms))


class NativeFocus:
    def __init__(self, *, strict_default: bool = False) -> None:
        self.calls: list[tuple[object, ...]] = []
        self.strict_default = strict_default

    def page_default_timeout(self, page_id: str, navigation: bool) -> int:
        assert page_id == "page-1"
        assert not navigation
        return 4321

    def context_selector_is_strict(self, context_id: str, strict: bool | None) -> bool:
        assert context_id == "context-1"
        return self.strict_default if strict is None else strict

    def locator(self, frame_id: str, selector: str) -> NativeFocusLocator:
        self.calls.append(("locator", frame_id, selector))
        return NativeFocusLocator(self, frame_id, selector)

    async def focus(self, frame_id: str, selector: str, timeout_ms: int) -> None:
        self.calls.append(("focus", frame_id, selector, timeout_ms))


def frame_with_native(native: NativeFocus) -> Frame:
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-1", None)
    page = Page(context, "page-1", "main-frame", "about:blank")
    return Frame(page, "child-frame", "main-frame", "child", "about:blank")


@pytest.mark.asyncio
async def test_frame_focus_defaults_to_first_match_with_frame_id_and_page_timeout() -> None:
    # Arrange
    native = NativeFocus()
    frame = frame_with_native(native)

    # Act
    await frame.focus("input")

    # Assert
    assert native.calls == [
        ("locator", "child-frame", "input"),
        ("nth", "child-frame", "input", 0),
        ("locator_focus", "child-frame", "input", 4321),
    ]


@pytest.mark.asyncio
async def test_frame_focus_strict_uses_native_focus_for_child_frame_with_zero_timeout() -> None:
    # Arrange
    native = NativeFocus()
    frame = frame_with_native(native)

    # Act
    await frame.focus("textarea", strict=True, timeout=0)

    # Assert
    assert native.calls == [("focus", "child-frame", "textarea", 0)]


@pytest.mark.asyncio
async def test_frame_focus_honors_context_strict_default_when_installed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["context-strict-selectors"]
    )
    native = NativeFocus(strict_default=True)
    frame = frame_with_native(native)

    # Act
    await frame.focus("input", timeout=75)

    # Assert
    assert native.calls == [("focus", "child-frame", "input", 75)]


@pytest.mark.asyncio
async def test_frame_focus_validates_arguments_before_invoking_native() -> None:
    # Arrange
    native = NativeFocus()
    frame = frame_with_native(native)

    # Act
    with pytest.raises(Error, match="Frame.focus: expected string"):
        await frame.focus(cast(str, None))
    with pytest.raises(TypeError, match="Frame.focus strict must be a boolean"):
        await frame.focus("input", strict=cast(bool, "true"))

    # Assert
    assert native.calls == []


class Bridge:
    def run(self, awaitable: Coroutine[Any, Any, None]) -> None:
        asyncio.run(awaitable)


def test_sync_frame_focus_forwards_selector_strict_and_timeout_to_child_frame() -> None:
    # Arrange
    native = NativeFocus()
    async_frame = frame_with_native(native)
    sync_frame = SyncFrame(cast(Any, Bridge()), async_frame, cast(Any, object()))

    # Act
    sync_frame.focus("button", strict=False, timeout=250)

    # Assert
    assert native.calls == [
        ("locator", "child-frame", "button"),
        ("nth", "child-frame", "button", 0),
        ("locator_focus", "child-frame", "button", 250),
    ]
