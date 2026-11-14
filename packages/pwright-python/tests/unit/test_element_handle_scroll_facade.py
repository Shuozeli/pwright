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
        return "div"


class _NativeSession:
    def __init__(self) -> None:
        self.calls: list[tuple[_NativeHandle, int]] = []
        self.failure: Error | None = None

    async def element_handle_scroll_into_view_if_needed(
        self, handle: _NativeHandle, timeout_ms: int
    ) -> None:
        self.calls.append((handle, timeout_ms))
        if self.failure is not None:
            raise self.failure


class _Page:
    def __init__(self) -> None:
        self.timeout_values: list[float | None] = []

    def _effective_timeout(self, value: float | None) -> int:
        self.timeout_values.append(value)
        return 750 if value is None else int(value)


@pytest.mark.asyncio
async def test_async_scroll_preserves_retained_handle_and_page_default(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    owner = _NativeHandle()
    page = _Page()
    element = ElementHandle(cast(Any, session), cast(Any, owner), cast(Any, page))

    # Act
    await element.scroll_into_view_if_needed()

    # Assert
    assert session.calls == [(owner, 750)]
    assert session.calls[0][0] is owner
    assert page.timeout_values == [None]


@pytest.mark.asyncio
async def test_async_scroll_passes_explicit_zero_as_unbounded_host_timeout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, owner))

    # Act
    await element.scroll_into_view_if_needed(timeout=0)

    # Assert
    assert session.calls == [(owner, 0)]


@pytest.mark.asyncio
async def test_async_scroll_without_page_uses_default_timeout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, session), cast(Any, owner))

    # Act
    await element.scroll_into_view_if_needed()

    # Assert
    assert session.calls == [(owner, 30_000)]


@pytest.mark.asyncio
async def test_async_scroll_uses_explicit_page_timeout(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    page = _Page()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()), cast(Any, page))

    # Act
    await element.scroll_into_view_if_needed(timeout=3000)

    # Assert
    assert session.calls[0][1] == 3000
    assert page.timeout_values == [3000]


@pytest.mark.asyncio
async def test_async_scroll_rewrites_native_detachment_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    session.failure = Error("Element is not attached to the DOM")
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(
        Error,
        match="ElementHandle.scroll_into_view_if_needed: Element is not attached",
    ):
        await element.scroll_into_view_if_needed()

    # Assert
    assert len(session.calls) == 1


@pytest.mark.asyncio
async def test_async_scroll_preserves_native_timeout_progress(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    session.failure = TimeoutError(
        "Timeout 3000ms exceeded.\nretrying scroll into view action\n  element is not visible"
    )
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(TimeoutError) as error:
        await element.scroll_into_view_if_needed(timeout=3000)

    # Assert
    assert "ElementHandle.scroll_into_view_if_needed" in error.value.message
    assert "retrying scroll into view action" in error.value.message
    assert "element is not visible" in error.value.message
    assert len(session.calls) == 1


@pytest.mark.asyncio
async def test_async_scroll_requires_opt_in_before_native_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: cast(list[str], []))
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(UnsupportedError, match="element-handle-scroll-into-view profile"):
        await element.scroll_into_view_if_needed()

    # Assert
    assert session.calls == []


@pytest.mark.asyncio
async def test_async_scroll_rejects_invalid_timeout_before_native_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    element = ElementHandle(cast(Any, session), cast(Any, _NativeHandle()))

    # Act
    with pytest.raises(ValueError, match="timeout must be finite"):
        await element.scroll_into_view_if_needed(timeout=-1)

    # Assert
    assert session.calls == []


def test_sync_scroll_uses_the_same_retained_native_owner(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-scroll-into-view"])
    session = _NativeSession()
    owner = _NativeHandle()
    page = _Page()
    bridge = _LoopThread()
    element = SyncElementHandle(
        bridge,
        ElementHandle(cast(Any, session), cast(Any, owner), cast(Any, page)),
    )

    # Act
    try:
        element.scroll_into_view_if_needed(timeout=250)
    finally:
        bridge.close()

    # Assert
    assert session.calls == [(owner, 250)]
    assert page.timeout_values == [250]
