# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import _native
from pwright._errors import Error, UnsupportedError
from pwright.async_api import ElementHandle
from pwright.sync_api import ElementHandle as SyncElementHandle
from pwright.sync_api import _LoopThread


class _NativeHandle:
    @staticmethod
    def is_element() -> bool:
        return True

    @staticmethod
    def preview() -> str:
        return "input"


class _NativeSession:
    def __init__(self, result: str | bool | None) -> None:
        self.result = result
        self.calls: list[tuple[_NativeHandle, str, str | None]] = []
        self.failure: Exception | None = None

    async def element_handle_dom_read(
        self, handle: _NativeHandle, operation: str, attribute: str | None = None
    ) -> str | bool | None:
        self.calls.append((handle, operation, attribute))
        if self.failure is not None:
            raise self.failure
        return self.result


_READ_CASES: list[tuple[str, tuple[object, ...], dict[str, object], str, str | None, object]] = [
    ("get_attribute", ("name",), {}, "getAttribute", "name", "value"),
    ("inner_html", (), {}, "innerHTML", None, "<b>text</b>"),
    ("input_value", (), {"timeout": 75}, "inputValue", None, "text"),
    ("is_checked", (), {}, "isChecked", None, True),
]


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("method", "args", "kwargs", "operation", "attribute", "expected"), _READ_CASES
)
async def test_async_element_dom_read_preserves_retained_owner(
    monkeypatch: pytest.MonkeyPatch,
    method: str,
    args: tuple[object, ...],
    kwargs: dict[str, object],
    operation: str,
    attribute: str | None,
    expected: object,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom-reads"])
    session = _NativeSession(cast("str | bool | None", expected))
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, owner))

    # Act
    actual = await getattr(element, method)(*args, **kwargs)

    # Assert
    assert actual == expected
    assert session.calls == [(owner, operation, attribute)]
    assert session.calls[0][0] is owner


@pytest.mark.parametrize(
    ("method", "args", "kwargs", "operation", "attribute", "expected"), _READ_CASES
)
def test_sync_element_dom_read_uses_same_native_path(
    monkeypatch: pytest.MonkeyPatch,
    method: str,
    args: tuple[object, ...],
    kwargs: dict[str, object],
    operation: str,
    attribute: str | None,
    expected: object,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom-reads"])
    session = _NativeSession(cast("str | bool | None", expected))
    owner = _NativeHandle()
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, ElementHandle(cast(Any, session), cast(Any, owner)))

    # Act
    try:
        actual = getattr(element, method)(*args, **kwargs)
    finally:
        bridge.close()

    # Assert
    assert actual == expected
    assert session.calls == [(owner, operation, attribute)]


@pytest.mark.asyncio
async def test_get_attribute_preserves_missing_attribute_as_none(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom-reads"])
    session = _NativeSession(None)
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, owner))

    # Act
    actual = await element.get_attribute("missing")

    # Assert
    assert actual is None
    assert session.calls == [(owner, "getAttribute", "missing")]


@pytest.mark.asyncio
@pytest.mark.parametrize("method", ["get_attribute", "inner_html", "input_value", "is_checked"])
async def test_element_dom_read_requires_opt_in_profile(
    monkeypatch: pytest.MonkeyPatch, method: str
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: cast(list[str], []))
    session = _NativeSession("unused")
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))
    args = ("name",) if method == "get_attribute" else ()

    # Act
    with pytest.raises(UnsupportedError, match="element-handle-dom-reads profile"):
        await getattr(element, method)(*args)

    # Assert
    assert session.calls == []


@pytest.mark.asyncio
async def test_element_dom_read_rewrites_native_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom-reads"])
    session = _NativeSession(None)
    session.failure = Error("Element is not attached to the DOM")
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(Error, match="ElementHandle.inner_html: Element is not attached"):
        await element.inner_html()

    # Assert
    assert len(session.calls) == 1


@pytest.mark.asyncio
async def test_input_value_rejects_invalid_timeout_before_native_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom-reads"])
    session = _NativeSession("unused")
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(ValueError, match="timeout must be finite"):
        await element.input_value(timeout=-1)

    # Assert
    assert session.calls == []
