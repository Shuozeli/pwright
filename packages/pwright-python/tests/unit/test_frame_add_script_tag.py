# pyright: reportPrivateUsage=false
"""Browserless Frame script-tag composition over existing frame evaluation."""

from __future__ import annotations

import asyncio
from collections.abc import Coroutine
from pathlib import Path
from typing import Any, cast

import pytest
from pwright import async_api
from pwright._errors import Error
from pwright.async_api import Browser, BrowserContext, ElementHandle, Frame, Page
from pwright.sync_api import ElementHandle as SyncElementHandle
from pwright.sync_api import Frame as SyncFrame


class NativeHandle:
    def __init__(self, *, is_element: bool = True) -> None:
        self._is_element = is_element

    def preview(self) -> str:
        return "script"

    def is_element(self) -> bool:
        return self._is_element


class NativeFrameEvaluation:
    def __init__(self, handle: NativeHandle | None = None, error: Error | None = None) -> None:
        self.handle = handle or NativeHandle()
        self.error = error
        self.calls: list[tuple[str, str, object]] = []
        self.disposed: list[NativeHandle] = []

    async def frame_evaluate_handle(
        self, frame_id: str, expression: str, argument: object
    ) -> NativeHandle:
        self.calls.append((frame_id, expression, argument))
        if self.error is not None:
            raise self.error
        return self.handle

    async def handle_dispose(self, handle: NativeHandle) -> None:
        self.disposed.append(handle)


def frame_with_native(native: NativeFrameEvaluation) -> tuple[Frame, Page]:
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-1", None)
    page = Page(context, "page-1", "main-frame", "about:blank")
    return Frame(page, "child-frame", "main-frame", "child", "about:blank"), page


@pytest.mark.asyncio
async def test_frame_script_tag_uses_the_child_frame_and_returns_its_element() -> None:
    # Arrange
    native = NativeFrameEvaluation()
    frame, page = frame_with_native(native)

    # Act
    element = await frame.add_script_tag(content="window.injected = 42;", type="module")

    # Assert
    assert isinstance(element, ElementHandle)
    assert element._page is page
    assert native.calls == [
        (
            "child-frame",
            async_api._ADD_SCRIPT_TAG_SOURCE,
            {"url": None, "content": "window.injected = 42;", "type": "module"},
        )
    ]


@pytest.mark.asyncio
async def test_frame_script_tag_preserves_page_source_url_for_a_path(tmp_path: Path) -> None:
    # Arrange
    source = tmp_path / "injected.js"
    source.write_text("window.injected = 42;")
    native = NativeFrameEvaluation()
    frame, _page = frame_with_native(native)

    # Act
    await frame.add_script_tag(path=source)

    # Assert
    assert native.calls[0][2] == {
        "url": None,
        "content": f"window.injected = 42;\n//# sourceURL={source}",
        "type": None,
    }


@pytest.mark.asyncio
async def test_frame_script_tag_rejects_a_missing_source_before_evaluation() -> None:
    # Arrange
    native = NativeFrameEvaluation()
    frame, _page = frame_with_native(native)

    # Act
    with pytest.raises(Error, match="Either path, url, or content parameter must be specified"):
        await frame.add_script_tag()

    # Assert
    assert native.calls == []


@pytest.mark.asyncio
async def test_frame_script_tag_preserves_a_csp_evaluation_error() -> None:
    # Arrange
    violation = Error(
        "Refused to execute inline script because it violates Content Security Policy"
    )
    native = NativeFrameEvaluation(error=violation)
    frame, _page = frame_with_native(native)

    # Act
    with pytest.raises(Error) as raised:
        await frame.add_script_tag(content="window.injected = 42;")

    # Assert
    assert raised.value is violation
    assert len(native.calls) == 1


@pytest.mark.asyncio
async def test_frame_script_tag_disposes_a_non_element_result() -> None:
    # Arrange
    handle = NativeHandle(is_element=False)
    native = NativeFrameEvaluation(handle=handle)
    frame, _page = frame_with_native(native)

    # Act
    with pytest.raises(Error, match="Frame.add_script_tag did not create an ElementHandle"):
        await frame.add_script_tag(content="window.injected = 42;")

    # Assert
    assert native.disposed == [handle]


class Bridge:
    def run(self, awaitable: Coroutine[Any, Any, object]) -> object:
        return asyncio.run(awaitable)


def test_sync_frame_script_tag_wraps_the_async_element_for_its_page() -> None:
    # Arrange
    native = NativeFrameEvaluation()
    async_frame, _async_page = frame_with_native(native)
    sync_page = cast(Any, object())
    sync_frame = SyncFrame(cast(Any, Bridge()), async_frame, sync_page)

    # Act
    element = sync_frame.add_script_tag(url="https://example.test/script.js")

    # Assert
    assert isinstance(element, SyncElementHandle)
    assert element._page is sync_page
    assert element._value._native is native.handle
    assert native.calls[0][2] == {
        "url": "https://example.test/script.js",
        "content": None,
        "type": None,
    }
