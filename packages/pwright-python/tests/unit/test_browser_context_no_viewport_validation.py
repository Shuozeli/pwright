"""Pinned Playwright null-viewport preflight parity without browser startup."""

from __future__ import annotations

import asyncio
from collections.abc import Coroutine
from typing import Any, cast

import pytest
from pwright._errors import Error, UnsupportedError
from pwright.async_api import Browser as AsyncBrowser
from pwright.sync_api import Browser as SyncBrowser


class _Bridge:
    def run(self, awaitable: Coroutine[Any, Any, object]) -> object:
        return asyncio.run(awaitable)


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("options", "message"),
    [
        (
            {"device_scale_factor": 1},
            'Browser.new_context: "deviceScaleFactor" option is not supported with null "viewport"',
        ),
        (
            {"is_mobile": True},
            'Browser.new_context: "isMobile" option is not supported with null "viewport"',
        ),
    ],
)
async def test_async_null_viewport_conflicts_match_upstream_error(
    options: dict[str, object], message: str
) -> None:
    # Arrange
    browser = cast(Any, object.__new__(AsyncBrowser))

    # Act
    with pytest.raises(Error) as error:
        await browser.new_context(no_viewport=True, **options)

    # Assert
    assert error.value.message == message


@pytest.mark.parametrize(
    ("options", "message"),
    [
        (
            {"device_scale_factor": 1},
            'Browser.new_context: "deviceScaleFactor" option is not supported with null "viewport"',
        ),
        (
            {"is_mobile": True},
            'Browser.new_context: "isMobile" option is not supported with null "viewport"',
        ),
    ],
)
def test_sync_null_viewport_conflicts_match_upstream_error(
    options: dict[str, object], message: str
) -> None:
    # Arrange
    browser = cast(Any, object.__new__(SyncBrowser))
    browser._bridge = _Bridge()
    browser._value = cast(Any, object.__new__(AsyncBrowser))

    # Act
    with pytest.raises(Error) as error:
        browser.new_context(no_viewport=True, **options)

    # Assert
    assert error.value.message == message


@pytest.mark.asyncio
async def test_null_viewport_without_conflict_is_explicitly_unsupported() -> None:
    # Arrange
    browser = cast(Any, object.__new__(AsyncBrowser))

    # Act and Assert
    with pytest.raises(UnsupportedError, match="requires a null-viewport profile"):
        await browser.new_context(no_viewport=True)
