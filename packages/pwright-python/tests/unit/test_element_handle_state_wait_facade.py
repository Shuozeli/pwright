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
        return "button"


class _NativeSession:
    def __init__(self) -> None:
        self.calls: list[tuple[_NativeHandle, str, int]] = []
        self.failure: Exception | None = None

    async def element_handle_wait_for_element_state(
        self, handle: _NativeHandle, state: str, timeout_ms: int
    ) -> None:
        self.calls.append((handle, state, timeout_ms))
        if self.failure is not None:
            raise self.failure


class _Page:
    def _effective_timeout(self, value: object) -> int:
        return 750 if value is None else int(cast(float, value))


def _state_wait_features() -> list[str]:
    return ["element-handle-state-wait"]


def _no_features() -> list[str]:
    return []


@pytest.mark.asyncio
async def test_async_state_wait_uses_exact_handle_and_page_timeout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _state_wait_features)
    session = _NativeSession()
    native_handle = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, native_handle), cast(Any, _Page()))

    # Act
    await element.wait_for_element_state("enabled")

    # Assert
    assert session.calls == [(native_handle, "enabled", 750)]
    assert session.calls[0][0] is native_handle


@pytest.mark.asyncio
async def test_async_state_wait_preserves_explicit_timeout_without_page(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _state_wait_features)
    session = _NativeSession()
    native_handle = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, native_handle))

    # Act
    await element.wait_for_element_state("hidden", timeout=0)

    # Assert
    assert session.calls == [(native_handle, "hidden", 0)]


@pytest.mark.asyncio
async def test_async_state_wait_rewrites_native_error_with_api_name(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _state_wait_features)
    session = _NativeSession()
    session.failure = Error("Element is not attached to the DOM")
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(
        Error, match="ElementHandle.wait_for_element_state: Element is not attached"
    ):
        await element.wait_for_element_state("visible", timeout=20)

    # Assert
    assert len(session.calls) == 1


@pytest.mark.asyncio
async def test_async_state_wait_requires_opt_in_profile(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _no_features)
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(UnsupportedError, match="element-handle-state-wait profile"):
        await element.wait_for_element_state("visible")

    # Assert
    assert session.calls == []


def test_sync_state_wait_uses_async_native_path(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _state_wait_features)
    session = _NativeSession()
    native_handle = _NativeHandle()
    page = _Page()
    async_element = ElementHandle(cast(Any, session), cast(Any, native_handle), cast(Any, page))
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, async_element)

    # Act
    try:
        element.wait_for_element_state("editable", timeout=125)
    finally:
        bridge.close()

    # Assert
    assert session.calls == [(native_handle, "editable", 125)]
