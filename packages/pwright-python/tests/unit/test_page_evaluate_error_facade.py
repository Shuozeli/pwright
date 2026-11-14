# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import Error, JavascriptError
from pwright.async_api import ElementHandle, Page


class _Native:
    def __init__(self, failure: Error) -> None:
        self.failure = failure
        self.calls: list[tuple[object, ...]] = []

    async def evaluate(self, frame_id: str, expression: str, argument: object) -> object:
        self.calls.append(("evaluate", frame_id, expression, argument))
        raise self.failure

    async def evaluate_handle_argument(
        self, frame_id: str, expression: str, handle: object
    ) -> object:
        self.calls.append(("evaluate_handle_argument", frame_id, expression, handle))
        raise self.failure


class _NativeHandle:
    @staticmethod
    def preview() -> str:
        return "section"

    @staticmethod
    def is_element() -> bool:
        return True


def _page(native: _Native) -> Page:
    class Browser:
        def __init__(self, session: _Native) -> None:
            self._native = session

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    return Page(cast(Any, Context(Browser(native))), "page-1", "frame-2", "about:blank")


@pytest.mark.asyncio
async def test_page_evaluate_disposed_element_uses_official_guid_error() -> None:
    # Arrange
    original = Error("JSHandle is disposed")
    original.name = "NativeBackendError"
    original.stack = "native evaluation stack"
    native = _Native(original)
    page = _page(native)
    handle = _NativeHandle()
    element = ElementHandle(cast(Any, native), cast(Any, handle), page)

    # Act
    with pytest.raises(Error) as raised:
        await page.evaluate("e => e.textContent", element)

    # Assert
    assert type(raised.value) is Error
    assert raised.value.message == "Page.evaluate: no object with guid: JSHandle is disposed"
    assert raised.value.code == "PWRIGHT_BACKEND"
    assert raised.value.name == "NativeBackendError"
    assert raised.value.stack == "native evaluation stack"
    assert native.calls == [("evaluate_handle_argument", "frame-2", "e => e.textContent", handle)]


@pytest.mark.asyncio
async def test_page_evaluate_reload_uses_official_navigation_error() -> None:
    # Arrange
    native = _Native(Error("Backend: CDP error -32000: Inspected target navigated or closed"))
    page = _page(native)
    expression = "() => { location.reload(); return new Promise(() => {}); }"

    # Act
    with pytest.raises(Error) as raised:
        await page.evaluate(expression)

    # Assert
    assert type(raised.value) is Error
    assert raised.value.message == (
        "Page.evaluate: Execution context was destroyed, most likely because of a navigation."
    )
    assert raised.value.code == "PWRIGHT_BACKEND"
    assert native.calls == [("evaluate", "frame-2", expression, None)]


@pytest.mark.asyncio
async def test_page_evaluate_keeps_unrelated_cdp_errors_unchanged() -> None:
    # Arrange
    message = "Backend: CDP error -32000: Inspected target navigated or closed (other target)"
    native = _Native(Error(message))
    page = _page(native)

    # Act
    with pytest.raises(Error) as raised:
        await page.evaluate("1 + 2")

    # Assert
    assert type(raised.value) is Error
    assert raised.value.message == f"Page.evaluate: {message}"
    assert native.calls == [("evaluate", "frame-2", "1 + 2", None)]


@pytest.mark.asyncio
async def test_page_evaluate_does_not_rewrite_disposal_without_a_handle_argument() -> None:
    # Arrange
    native = _Native(Error("JSHandle is disposed"))
    page = _page(native)

    # Act
    with pytest.raises(Error) as raised:
        await page.evaluate("1 + 2")

    # Assert
    assert raised.value.message == "Page.evaluate: JSHandle is disposed"
    assert native.calls == [("evaluate", "frame-2", "1 + 2", None)]


@pytest.mark.asyncio
async def test_page_evaluate_preserves_javascript_error_subtype_and_message() -> None:
    # Arrange
    native = _Native(JavascriptError("JSHandle is disposed"))
    page = _page(native)
    handle = _NativeHandle()
    element = ElementHandle(cast(Any, native), cast(Any, handle), page)

    # Act
    with pytest.raises(JavascriptError) as raised:
        await page.evaluate("e => e.textContent", element)

    # Assert
    assert type(raised.value) is JavascriptError
    assert raised.value.message == "Page.evaluate: JSHandle is disposed"
    assert raised.value.code == "PWRIGHT_JAVASCRIPT"
    assert native.calls == [("evaluate_handle_argument", "frame-2", "e => e.textContent", handle)]
