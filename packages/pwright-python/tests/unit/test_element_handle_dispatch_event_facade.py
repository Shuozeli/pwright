# pyright: reportPrivateUsage=false

from __future__ import annotations

from types import MappingProxyType
from typing import Any, cast

import pytest
from pwright.async_api import ElementHandle, JSHandle
from pwright.sync_api import ElementHandle as SyncElementHandle
from pwright.sync_api import JSHandle as SyncJSHandle
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
        self.observed: list[tuple[object, ...]] = []

    async def element_handle_dispatch_event(
        self, handle: _NativeHandle, event_type: str, event_init: dict[str, object] | None
    ) -> None:
        self.observed.append(("plain", handle, event_type, event_init))

    async def element_handle_dispatch_event_owned(
        self, handle: _NativeHandle, event_type: str, event_init: dict[str, object]
    ) -> None:
        self.observed.append(("owned", handle, event_type, event_init))


def _element(session: _NativeSession, owner: _NativeHandle) -> ElementHandle:
    return ElementHandle(cast(Any, session), cast(Any, owner))


@pytest.mark.asyncio
async def test_async_element_handle_dispatches_on_exact_retained_owner() -> None:
    # Arrange
    session = _NativeSession()
    owner = _NativeHandle()
    element = _element(session, owner)
    event_init = MappingProxyType({"detail": 2, "bubbles": False})

    # Act
    await element.dispatch_event("click", event_init)

    # Assert
    assert session.observed == [("plain", owner, "click", {"detail": 2, "bubbles": False})]
    assert session.observed[0][1] is owner


@pytest.mark.asyncio
async def test_async_element_handle_dispatches_without_initializer() -> None:
    # Arrange
    session = _NativeSession()
    owner = _NativeHandle()
    element = _element(session, owner)

    # Act
    await element.dispatch_event("click")

    # Assert
    assert session.observed == [("plain", owner, "click", None)]


@pytest.mark.asyncio
async def test_async_element_handle_dispatches_nested_live_handle_on_exact_owner() -> None:
    # Arrange
    session = _NativeSession()
    owner = _NativeHandle()
    live_owner = _NativeHandle()
    element = _element(session, owner)
    live_value = JSHandle(cast(Any, session), cast(Any, live_owner))

    # Act
    await element.dispatch_event("dragstart", {"nested": [{"dataTransfer": live_value}]})

    # Assert
    assert session.observed == [
        ("owned", owner, "dragstart", {"nested": [{"dataTransfer": live_owner}]})
    ]
    assert session.observed[0][1] is owner


@pytest.mark.asyncio
async def test_async_element_handle_rejects_nonserializable_nonhandle_leaf() -> None:
    # Arrange
    session = _NativeSession()
    element = _element(session, _NativeHandle())

    # Act
    with pytest.raises(TypeError, match="JSON-serializable"):
        await element.dispatch_event("click", {"detail": object()})

    # Assert
    assert session.observed == []


@pytest.mark.asyncio
async def test_async_element_handle_rejects_nonserializable_leaf_beside_live_handle() -> None:
    # Arrange
    session = _NativeSession()
    owner = _NativeHandle()
    element = _element(session, owner)
    live_value = JSHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(TypeError, match="JSON-serializable"):
        await element.dispatch_event("drop", {"dataTransfer": live_value, "unsupported": object()})

    # Assert
    assert session.observed == []


@pytest.mark.asyncio
async def test_async_element_handle_rejects_nonmapping_initializer() -> None:
    # Arrange
    session = _NativeSession()
    element = _element(session, _NativeHandle())

    # Act
    with pytest.raises(TypeError, match="must be a mapping or None"):
        await element.dispatch_event("click", cast(Any, [("detail", 2)]))

    # Assert
    assert session.observed == []


def test_sync_element_handle_dispatches_on_async_owner() -> None:
    # Arrange
    session = _NativeSession()
    owner = _NativeHandle()
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, _element(session, owner))

    # Act
    try:
        element.dispatch_event("click", {"detail": 3})
    finally:
        bridge.close()

    # Assert
    assert session.observed == [("plain", owner, "click", {"detail": 3})]
    assert session.observed[0][1] is owner


def test_sync_element_handle_unwraps_live_handle_event_initializer() -> None:
    # Arrange
    session = _NativeSession()
    owner = _NativeHandle()
    live_owner = _NativeHandle()
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, _element(session, owner))
    live_value = SyncJSHandle(bridge, JSHandle(cast(Any, session), cast(Any, live_owner)))

    # Act
    try:
        element.dispatch_event("drop", {"dataTransfer": live_value})
    finally:
        bridge.close()

    # Assert
    assert session.observed == [("owned", owner, "drop", {"dataTransfer": live_owner})]
