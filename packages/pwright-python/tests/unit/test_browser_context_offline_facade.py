# pyright: reportPrivateUsage=false

from __future__ import annotations

import asyncio
from typing import Any, cast

import pytest
from pwright.async_api import Browser, BrowserContext
from pwright.sync_api import BrowserContext as SyncBrowserContext


class NativeOffline:
    def __init__(self) -> None:
        self.calls: list[tuple[str, bool]] = []

    async def set_context_offline(self, context_id: str, offline: bool) -> None:
        self.calls.append((context_id, offline))


def context(native: NativeOffline) -> BrowserContext:
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    return BrowserContext(browser, "7", None)


@pytest.mark.asyncio
async def test_context_offline_toggle_forwards_both_states_to_native() -> None:
    # Arrange
    native = NativeOffline()
    value = context(native)

    # Act
    await value.set_offline(True)
    await value.set_offline(False)

    # Assert
    assert native.calls == [("7", True), ("7", False)]


@pytest.mark.asyncio
async def test_context_offline_rejects_non_boolean_before_native() -> None:
    # Arrange
    native = NativeOffline()
    value = context(native)

    # Act
    with pytest.raises(TypeError, match="BrowserContext.set_offline offline must be a boolean"):
        await value.set_offline(cast(Any, "yes"))

    # Assert
    assert native.calls == []


def test_sync_context_offline_forwards_to_the_async_owner() -> None:
    # Arrange
    native = NativeOffline()
    async_value = context(native)

    class Bridge:
        def run(self, awaitable: Any) -> None:
            asyncio.run(awaitable)

    sync_value = object.__new__(SyncBrowserContext)
    sync_value._bridge = cast(Any, Bridge())
    sync_value._value = async_value

    # Act
    sync_value.set_offline(True)

    # Assert
    assert native.calls == [("7", True)]
