# pyright: reportPrivateUsage=false
"""BrowserContext forced-colors option stays a thin, opt-in Rust policy input."""

from __future__ import annotations

import asyncio
from collections.abc import Coroutine
from typing import Any, cast

import pytest
from pwright import async_api
from pwright._errors import Error, UnsupportedError
from pwright.sync_api import Browser as SyncBrowser


class Native:
    def __init__(self) -> None:
        self.calls: list[tuple[tuple[object, ...], dict[str, object]]] = []

    async def new_context(self, *args: object, **kwargs: object) -> tuple[str, None]:
        self.calls.append((args, kwargs))
        return "7", None


def browser(native: Native) -> async_api.Browser:
    value = object.__new__(async_api.Browser)
    value._native = cast(Any, native)
    value._contexts = {}
    return value


@pytest.mark.asyncio
async def test_forced_colors_active_uses_only_optional_native_policy_argument(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = Native()
    value = browser(native)
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-forced-colors"])

    # Act
    context = await value.new_context(forced_colors="active")

    # Assert
    assert context._object_id == "7"
    assert len(native.calls) == 1
    assert len(native.calls[0][0]) == 13
    assert native.calls[0][1] == {"forced_colors": "active"}


@pytest.mark.asyncio
async def test_default_context_keeps_legacy_native_call_shape() -> None:
    # Arrange
    native = Native()
    value = browser(native)

    # Act
    await value.new_context()

    # Assert
    assert len(native.calls[0][0]) == 13
    assert native.calls[0][1] == {}


@pytest.mark.asyncio
async def test_forced_colors_requires_profile_and_valid_enum(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = Native()
    value = browser(native)

    def no_features() -> list[str]:
        return []

    monkeypatch.setattr(async_api._native, "selected_features", no_features)

    # Act and Assert
    with pytest.raises(UnsupportedError, match="context-forced-colors profile"):
        await value.new_context(forced_colors="active")
    with pytest.raises(Error, match="must be 'active' or 'none'"):
        await value.new_context(forced_colors=cast(Any, "purple"))
    with pytest.raises(Error, match="forced_colors: expected string, got number"):
        await value.new_context(forced_colors=cast(Any, 1))
    assert native.calls == []


def test_sync_context_forwards_forced_colors_to_async_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = Native()
    async_value = browser(native)
    monkeypatch.setattr(async_api._native, "selected_features", lambda: ["context-forced-colors"])

    class Bridge:
        def run(self, awaitable: Coroutine[Any, Any, object]) -> object:
            return asyncio.run(awaitable)

    sync_value = object.__new__(SyncBrowser)
    sync_value._bridge = cast(Any, Bridge())
    sync_value._value = async_value
    sync_value._contexts = {}

    # Act
    context = sync_value.new_context(forced_colors="none")

    # Assert
    assert context._value._object_id == "7"
    assert native.calls[0][1] == {"forced_colors": "none"}
