# pyright: reportPrivateUsage=false
"""Locator.click keeps legacy calls while forwarding bounded mouse steps and force."""

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright._errors import UnsupportedError
from pwright.async_api import Locator
from pwright.sync_api import Locator as SyncLocator
from pwright.sync_api import _LoopThread


class NativeLocator:
    def __init__(self) -> None:
        self.calls: list[tuple[int | bool | None, ...]] = []

    async def click(self, *args: int | bool | None) -> None:
        self.calls.append(args)


class PageOwner:
    @staticmethod
    def _effective_timeout(timeout: float | None) -> int:
        return 30_000 if timeout is None else int(timeout)


class SelectorPageOwner:
    def __init__(self) -> None:
        self.calls: list[tuple[str, float | None, bool | None]] = []

    async def click(
        self, selector: str, *, timeout: float | None = None, force: bool | None = None
    ) -> None:
        self.calls.append((selector, timeout, force))


@pytest.mark.asyncio
async def test_locator_click_omits_steps_for_legacy_native_call() -> None:
    # Arrange
    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))

    # Act
    await locator.click(timeout=47)

    # Assert
    assert native.calls == [(47,)]


@pytest.mark.asyncio
async def test_locator_click_forwards_explicit_steps_to_native() -> None:
    # Arrange
    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))

    # Act
    await locator.click(timeout=47, steps=5)

    # Assert
    assert native.calls == [(47, 5)]


@pytest.mark.asyncio
@pytest.mark.parametrize("force", [True, False])
async def test_locator_click_forwards_explicit_force_to_native(force: bool) -> None:
    # Arrange
    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))

    # Act
    await locator.click(timeout=47, force=force)

    # Assert
    assert native.calls == [(47, None, force)]


@pytest.mark.asyncio
async def test_locator_click_forwards_force_with_steps_to_native() -> None:
    # Arrange
    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))

    # Act
    await locator.click(timeout=47, steps=5, force=True)

    # Assert
    assert native.calls == [(47, 5, True)]


@pytest.mark.asyncio
@pytest.mark.parametrize("force", [True, False])
async def test_locator_click_forwards_force_to_selector_backed_page(force: bool) -> None:
    # Arrange
    page = SelectorPageOwner()
    locator = Locator(cast(Any, page), "button")

    # Act
    await locator.click(timeout=47, force=force)

    # Assert
    assert page.calls == [("button", 47, force)]


@pytest.mark.asyncio
async def test_locator_click_rejects_invalid_force_before_native_call() -> None:
    # Arrange
    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))

    # Act
    with pytest.raises(TypeError, match="Locator.click force must be a boolean"):
        await locator.click(force=cast(Any, "yes"))

    # Assert
    assert native.calls == []


@pytest.mark.asyncio
async def test_locator_click_rejects_invalid_force_before_selector_fallback() -> None:
    # Arrange
    page = SelectorPageOwner()
    locator = Locator(cast(Any, page), "button")

    # Act
    with pytest.raises(TypeError, match="Locator.click force must be a boolean"):
        await locator.click(force=cast(Any, "yes"))

    # Assert
    assert page.calls == []


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("steps", "error"),
    [(True, TypeError), (1.5, TypeError), (0, ValueError), (10_001, ValueError)],
)
async def test_locator_click_rejects_invalid_steps_before_native_call(
    steps: object, error: type[Exception]
) -> None:
    # Arrange
    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))

    # Act
    with pytest.raises(error):
        await locator.click(steps=cast(Any, steps))

    # Assert
    assert native.calls == []


@pytest.mark.asyncio
async def test_locator_click_rejects_steps_without_rust_owned_query() -> None:
    # Arrange
    locator = Locator(cast(Any, PageOwner()), "button")

    # Act
    with pytest.raises(UnsupportedError, match="Rust-owned query"):
        await locator.click(steps=5)

    # Assert
    assert locator._native_locator is None


def test_sync_locator_click_forwards_explicit_steps() -> None:
    # Arrange
    native = NativeLocator()
    async_locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))
    bridge = _LoopThread()
    locator = SyncLocator(bridge, async_locator)

    # Act
    try:
        locator.click(timeout=47, steps=5)
    finally:
        bridge.close()

    # Assert
    assert native.calls == [(47, 5)]


def test_sync_locator_click_forwards_force_and_steps() -> None:
    # Arrange
    native = NativeLocator()
    async_locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))
    bridge = _LoopThread()
    locator = SyncLocator(bridge, async_locator)

    # Act
    try:
        locator.click(timeout=47, steps=5, force=False)
    finally:
        bridge.close()

    # Assert
    assert native.calls == [(47, 5, False)]


def test_sync_locator_click_forwards_force_to_selector_backed_page() -> None:
    # Arrange
    page = SelectorPageOwner()
    async_locator = Locator(cast(Any, page), "button")
    bridge = _LoopThread()
    locator = SyncLocator(bridge, async_locator)

    # Act
    try:
        locator.click(timeout=47, force=True)
    finally:
        bridge.close()

    # Assert
    assert page.calls == [("button", 47, True)]


def test_sync_locator_click_omits_steps_for_legacy_native_call() -> None:
    # Arrange
    native = NativeLocator()
    async_locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))
    bridge = _LoopThread()
    locator = SyncLocator(bridge, async_locator)

    # Act
    try:
        locator.click(timeout=47)
    finally:
        bridge.close()

    # Assert
    assert native.calls == [(47,)]
