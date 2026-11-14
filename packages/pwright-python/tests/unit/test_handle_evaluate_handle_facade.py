# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright.async_api import ElementHandle, JSHandle
from pwright.sync_api import ElementHandle as SyncElementHandle
from pwright.sync_api import JSHandle as SyncJSHandle
from pwright.sync_api import _LoopThread


class _NativeHandle:
    def __init__(self, *, element: bool, preview: str) -> None:
        self.element = element
        self.value_preview = preview

    def is_element(self) -> bool:
        return self.element

    def preview(self) -> str:
        return self.value_preview


class _NativeSession:
    def __init__(self, result: _NativeHandle) -> None:
        self.result = result
        self.calls: list[tuple[_NativeHandle, str, object]] = []

    async def handle_evaluate_handle(
        self, handle: _NativeHandle, source: str, argument: object
    ) -> _NativeHandle:
        self.calls.append((handle, source, argument))
        return self.result


@pytest.mark.asyncio
async def test_async_element_evaluate_handle_retains_target_and_result_owner() -> None:
    # Arrange
    owner = _NativeHandle(element=True, preview="JSHandle@<div></div>")
    result_owner = _NativeHandle(element=False, preview="JSHandle@array")
    session = _NativeSession(result_owner)
    page = cast(Any, object())
    element = ElementHandle(cast(Any, session), cast(Any, owner), page)
    expression = "target => [target.textContent]"

    # Act
    result = await element.evaluate_handle(expression)

    # Assert
    assert type(result) is JSHandle
    assert result._native is result_owner
    assert result._page is page
    assert session.calls == [(owner, expression, None)]
    assert session.calls[0][0] is owner


@pytest.mark.asyncio
async def test_async_jshandle_evaluate_handle_preserves_nested_live_argument() -> None:
    # Arrange
    owner = _NativeHandle(element=False, preview="JSHandle@object")
    argument_owner = _NativeHandle(element=False, preview="JSHandle@object")
    result_owner = _NativeHandle(element=True, preview="JSHandle@<span></span>")
    session = _NativeSession(result_owner)
    target = JSHandle(cast(Any, session), cast(Any, owner))
    argument = JSHandle(cast(Any, session), cast(Any, argument_owner))
    expression = "(target, arg) => arg.nested[0]"

    # Act
    result = await target.evaluate_handle(expression, {"nested": [argument]})

    # Assert
    assert type(result) is ElementHandle
    assert result._native is result_owner
    assert session.calls == [(owner, expression, {"nested": [argument_owner]})]
    captured = cast(dict[str, list[object]], session.calls[0][2])
    assert captured["nested"][0] is argument_owner


def test_sync_element_evaluate_handle_uses_same_native_owner() -> None:
    # Arrange
    owner = _NativeHandle(element=True, preview="JSHandle@<button></button>")
    result_owner = _NativeHandle(element=False, preview="JSHandle@array")
    session = _NativeSession(result_owner)
    async_element = ElementHandle(cast(Any, session), cast(Any, owner))
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, async_element)

    # Act
    try:
        result = element.evaluate_handle("target => []")
    finally:
        bridge.close()

    # Assert
    assert type(result) is SyncJSHandle
    assert result._value._native is result_owner
    assert session.calls == [(owner, "target => []", None)]


def test_sync_jshandle_evaluate_handle_converts_nested_sync_argument() -> None:
    # Arrange
    owner = _NativeHandle(element=False, preview="JSHandle@object")
    argument_owner = _NativeHandle(element=False, preview="JSHandle@object")
    result_owner = _NativeHandle(element=True, preview="JSHandle@<span></span>")
    session = _NativeSession(result_owner)
    bridge = _LoopThread()
    target = SyncJSHandle(bridge, JSHandle(cast(Any, session), cast(Any, owner)))
    argument = SyncJSHandle(bridge, JSHandle(cast(Any, session), cast(Any, argument_owner)))

    # Act
    try:
        result = target.evaluate_handle("(target, arg) => arg.nested[0]", {"nested": [argument]})
    finally:
        bridge.close()

    # Assert
    assert type(result) is SyncElementHandle
    assert result._value._native is result_owner
    assert session.calls == [
        (owner, "(target, arg) => arg.nested[0]", {"nested": [argument_owner]})
    ]


@pytest.mark.parametrize(
    "preview",
    [
        'JSHandle@<div id="outer" name="value">…</div>',
        'JSHandle@<div id="inner">Text,↵more text</div>',
        'JSHandle@<input checked id="check" foo="bar"" type="checkbox"/>',
    ],
)
def test_element_handle_exposes_exact_rust_owned_node_preview(preview: str) -> None:
    # Arrange
    native = _NativeHandle(element=True, preview=preview)
    async_handle = ElementHandle(cast(Any, object()), cast(Any, native))
    sync_handle = SyncElementHandle(cast(Any, object()), async_handle)

    # Act
    async_text = str(async_handle)
    sync_text = str(sync_handle)

    # Assert
    assert async_text == preview
    assert sync_text == preview
    assert repr(async_handle) == f"<JSHandle preview={preview}>"


@pytest.mark.asyncio
async def test_evaluate_handle_exposes_rust_owned_text_node_preview() -> None:
    # Arrange
    owner = _NativeHandle(element=True, preview='JSHandle@<div id="inner"></div>')
    text = _NativeHandle(element=True, preview="JSHandle@#text=Text,↵more text")
    session = _NativeSession(text)
    element = ElementHandle(cast(Any, session), cast(Any, owner))

    # Act
    result = await element.evaluate_handle("e => e.firstChild")

    # Assert
    assert type(result) is ElementHandle
    assert str(result) == "JSHandle@#text=Text,↵more text"
    assert session.calls == [(owner, "e => e.firstChild", None)]
