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
        self.calls: list[tuple[_NativeHandle, str]] = []
        self.failure: Exception | None = None

    async def element_handle_state_predicate(self, handle: _NativeHandle, operation: str) -> bool:
        self.calls.append((handle, operation))
        if self.failure is not None:
            raise self.failure
        return operation in ("isEnabled", "isVisible", "isEditable")


def _predicate_features() -> list[str]:
    return ["element-handle-state-predicates"]


def _no_features() -> list[str]:
    return []


_CASES = [
    ("is_disabled", "isDisabled", False),
    ("is_editable", "isEditable", True),
    ("is_enabled", "isEnabled", True),
    ("is_hidden", "isHidden", False),
    ("is_visible", "isVisible", True),
]


@pytest.mark.asyncio
@pytest.mark.parametrize(("method", "operation", "expected"), _CASES)
async def test_async_element_state_predicate_uses_retained_owner(
    monkeypatch: pytest.MonkeyPatch, method: str, operation: str, expected: bool
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _predicate_features)
    session = _NativeSession()
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, owner))

    # Act
    actual = await getattr(element, method)()

    # Assert
    assert actual is expected
    assert session.calls == [(owner, operation)]
    assert session.calls[0][0] is owner


@pytest.mark.parametrize(("method", "operation", "expected"), _CASES)
def test_sync_element_state_predicate_uses_async_native_path(
    monkeypatch: pytest.MonkeyPatch, method: str, operation: str, expected: bool
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _predicate_features)
    session = _NativeSession()
    owner = _NativeHandle()
    async_element = ElementHandle(cast(Any, session), cast(Any, owner))
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, async_element)

    # Act
    try:
        actual = getattr(element, method)()
    finally:
        bridge.close()

    # Assert
    assert actual is expected
    assert session.calls == [(owner, operation)]


@pytest.mark.asyncio
async def test_element_state_predicate_requires_opt_in_profile(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _no_features)
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(UnsupportedError, match="element-handle-state-predicates profile"):
        await element.is_visible()

    # Assert
    assert session.calls == []


@pytest.mark.asyncio
async def test_element_state_predicate_rewrites_native_errors(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", _predicate_features)
    session = _NativeSession()
    session.failure = Error("Element is not attached to the DOM")
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(Error, match="ElementHandle.is_enabled: Element is not attached"):
        await element.is_enabled()

    # Assert
    assert len(session.calls) == 1
