# pyright: reportPrivateUsage=false

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import _native
from pwright._errors import InvalidArgumentError, UnsupportedError
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
        self.calls: list[tuple[object, dict[str, object], int]] = []

    async def element_handle_click(
        self, handle: _NativeHandle, options: dict[str, object], timeout_ms: int
    ) -> None:
        self.calls.append((handle, options, timeout_ms))


class _Page:
    def __init__(self) -> None:
        self.refreshed = 0

    def _effective_timeout(self, value: object) -> int:
        return 800 if value is None else int(cast(float, value))

    def _refresh_snapshot(self) -> None:
        self.refreshed += 1


def _click_features() -> list[str]:
    return ["element-handle-click"]


def _no_features() -> list[str]:
    return []


@pytest.mark.asyncio
async def test_async_element_click_uses_exact_retained_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _click_features)
    session = _NativeSession()
    owner = _NativeHandle()
    page = _Page()
    element = ElementHandle(cast(Any, session), cast(Any, owner), cast(Any, page))

    # Act
    await element.click(button="right", delay=1.25, no_wait_after=True, trial=False, timeout=42)

    # Assert
    assert session.calls == [
        (owner, {"button": "right", "delay": 1.25, "no_wait_after": True, "trial": False}, 42)
    ]
    assert session.calls[0][0] is owner
    assert page.refreshed == 1


@pytest.mark.asyncio
async def test_async_element_click_requires_opt_in_profile(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _no_features)
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()), cast(Any, _Page()))

    # Act
    with pytest.raises(UnsupportedError, match="element-handle-click profile"):
        await element.click()

    # Assert
    assert session.calls == []


@pytest.mark.asyncio
async def test_async_element_click_rejects_invalid_steps_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _click_features)
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()), cast(Any, _Page()))

    # Act
    with pytest.raises(InvalidArgumentError, match="steps must be an integer"):
        await element.click(steps=cast(Any, True))

    # Assert
    assert session.calls == []


def test_sync_element_click_delegates_to_async_retained_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _click_features)
    session = _NativeSession()
    owner = _NativeHandle()
    page = _Page()
    bridge = _LoopThread()
    element = SyncElementHandle(
        bridge, ElementHandle(cast(Any, session), cast(Any, owner), cast(Any, page))
    )

    # Act
    try:
        element.click(force=True, steps=2)
    finally:
        bridge.close()

    # Assert
    assert session.calls == [(owner, {"force": True, "steps": 2}, 800)]
    assert page.refreshed == 1
