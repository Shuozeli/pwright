# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import _native
from pwright._errors import Error, TimeoutError, UnsupportedError
from pwright.async_api import ElementHandle
from pwright.sync_api import ElementHandle as SyncElementHandle
from pwright.sync_api import _LoopThread


class _NativeHandle:
    @staticmethod
    def is_element() -> bool:
        return True

    @staticmethod
    def preview() -> str:
        return "textarea"


class _NativeSession:
    def __init__(self) -> None:
        self.calls: list[tuple[_NativeHandle, bool, int]] = []
        self.failure: Error | None = None

    async def element_handle_select_text(
        self, handle: _NativeHandle, force: bool, timeout_ms: int
    ) -> None:
        self.calls.append((handle, force, timeout_ms))
        if self.failure is not None:
            raise self.failure


class _Page:
    def __init__(self) -> None:
        self.timeout_values: list[float | None] = []

    def _effective_timeout(self, value: float | None) -> int:
        self.timeout_values.append(value)
        return 750 if value is None else int(value)


def _selected() -> list[str]:
    return ["element-handle-select-text"]


@pytest.mark.asyncio
async def test_async_select_text_uses_original_handle_and_page_default(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    owner = _NativeHandle()
    page = _Page()
    element = ElementHandle(cast(Any, session), cast(Any, owner), cast(Any, page))

    # Act
    await element.select_text()

    # Assert
    assert session.calls == [(owner, False, 750)]
    assert session.calls[0][0] is owner
    assert page.timeout_values == [None]


@pytest.mark.asyncio
async def test_async_select_text_without_page_uses_default_timeout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    await element.select_text()

    # Assert
    assert session.calls[0][1:] == (False, 30_000)


@pytest.mark.asyncio
async def test_async_select_text_preserves_force_and_zero_timeout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, owner))

    # Act
    await element.select_text(force=True, timeout=0)

    # Assert
    assert session.calls == [(owner, True, 0)]


@pytest.mark.asyncio
async def test_async_select_text_uses_explicit_page_timeout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    page = _Page()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()), cast(Any, page))

    # Act
    await element.select_text(timeout=3000)

    # Assert
    assert session.calls[0][1:] == (False, 3000)
    assert page.timeout_values == [3000]


@pytest.mark.asyncio
async def test_async_select_text_requires_opt_in_before_native_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: cast(list[str], []))
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(UnsupportedError, match="element-handle-select-text profile"):
        await element.select_text()

    # Assert
    assert session.calls == []


@pytest.mark.asyncio
async def test_async_select_text_rejects_non_boolean_force_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(TypeError, match="force must be a boolean"):
        await element.select_text(force=cast(Any, "yes"))

    # Assert
    assert session.calls == []


@pytest.mark.asyncio
async def test_async_select_text_rejects_negative_timeout_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(ValueError, match="timeout must be finite"):
        await element.select_text(timeout=-1)

    # Assert
    assert session.calls == []


@pytest.mark.asyncio
async def test_async_select_text_preserves_visibility_timeout_progress(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    session.failure = TimeoutError(
        "Timeout 3000ms exceeded.\nretrying selectText action\n  element is not visible"
    )
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(TimeoutError) as error:
        await element.select_text(timeout=3000)

    # Assert
    assert "ElementHandle.select_text" in error.value.message
    assert "element is not visible" in error.value.message
    assert len(session.calls) == 1


@pytest.mark.asyncio
async def test_async_select_text_preserves_detachment_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    session.failure = Error("Element is not attached to the DOM")
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(Error, match="ElementHandle.select_text: Element is not attached"):
        await element.select_text()

    # Assert
    assert len(session.calls) == 1


def test_sync_select_text_uses_same_retained_native_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _selected)
    session = _NativeSession()
    owner = _NativeHandle()
    page = _Page()
    bridge = _LoopThread()
    element = SyncElementHandle(
        bridge, ElementHandle(cast(Any, session), cast(Any, owner), cast(Any, page))
    )

    # Act
    try:
        element.select_text(force=True, timeout=125)
    finally:
        bridge.close()

    # Assert
    assert session.calls == [(owner, True, 125)]
    assert page.timeout_values == [125]
