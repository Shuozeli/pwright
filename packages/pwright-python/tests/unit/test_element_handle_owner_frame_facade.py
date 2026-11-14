# pyright: reportPrivateUsage=false

from __future__ import annotations

from types import SimpleNamespace
from typing import Any, cast

import pytest
from pwright import _native
from pwright._errors import Error, UnsupportedError
from pwright.async_api import BrowserContext, ElementHandle, Page
from pwright.sync_api import BrowserContext as SyncBrowserContext
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
    def __init__(self, owner_frame_id: str | None) -> None:
        self.owner_frame_id = owner_frame_id
        self.owner_calls: list[_NativeHandle] = []
        self.page_reads: list[str] = []
        self.frame_reads: list[str] = []
        self.failure: Error | None = None

    async def element_handle_owner_frame(self, handle: _NativeHandle) -> str | None:
        self.owner_calls.append(handle)
        if self.failure is not None:
            raise self.failure
        return self.owner_frame_id

    def pages(self, context_id: str) -> list[tuple[str, str, str, str]]:
        self.page_reads.append(context_id)
        return [
            ("page-a", context_id, "frame-a", "https://example.test/a"),
            ("page-b", context_id, "frame-b", "https://example.test/b"),
        ]

    def frames(self, page_id: str) -> list[tuple[str, str | None, str, str]]:
        self.frame_reads.append(page_id)
        if page_id == "page-a":
            return [
                ("frame-a", None, "", "https://example.test/a"),
                ("child-a", "frame-a", "child", "https://example.test/child"),
            ]
        return [("frame-b", None, "", "https://example.test/b")]


def _pages(native: _NativeSession) -> tuple[BrowserContext, Page, Page]:
    browser = SimpleNamespace(_native=native)
    context = BrowserContext(cast(Any, browser), "context", None)
    source = Page(context, "page-a", "frame-a", "https://example.test/a")
    adopted = Page(context, "page-b", "frame-b", "https://example.test/b")
    context._pages.update({"page-a": source, "page-b": adopted})
    return context, source, adopted


@pytest.mark.asyncio
async def test_owner_frame_preserves_current_child_frame_identity(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom"])
    native = _NativeSession("child-a")
    _context, source, _adopted = _pages(native)
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, native), cast(Any, owner), source)
    child = source._refresh_frames()[1]
    native.frame_reads.clear()

    # Act
    result = await element.owner_frame()

    # Assert
    assert result is child
    assert native.owner_calls == [owner]
    assert native.page_reads == ["context"]
    assert native.frame_reads == ["page-a"]


@pytest.mark.asyncio
async def test_owner_frame_reconciles_adopted_node_with_current_page(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom"])
    native = _NativeSession("frame-b")
    context, source, adopted = _pages(native)
    owner = _NativeHandle()
    element = ElementHandle(cast(Any, native), cast(Any, owner), source)

    # Act
    result = await element.owner_frame()

    # Assert
    assert result is adopted.main_frame
    assert result is not None
    assert result._page is adopted
    assert context._pages["page-b"] is adopted
    assert native.owner_calls == [owner]
    assert native.frame_reads[:2] == ["page-a", "page-b"]


@pytest.mark.asyncio
async def test_owner_frame_preserves_orphan_none_without_page_scan(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom"])
    native = _NativeSession(None)
    _context, source, _adopted = _pages(native)
    element = ElementHandle(cast(Any, native), cast(Any, _NativeHandle()), source)

    # Act
    result = await element.owner_frame()

    # Assert
    assert result is None
    assert native.page_reads == []
    assert native.frame_reads == []


@pytest.mark.asyncio
async def test_owner_frame_fails_closed_when_frame_is_no_longer_current(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom"])
    native = _NativeSession("retired-frame")
    _context, source, _adopted = _pages(native)
    element = ElementHandle(cast(Any, native), cast(Any, _NativeHandle()), source)

    # Act
    with pytest.raises(Error, match="owner Frame is no longer available"):
        await element.owner_frame()

    # Assert
    assert native.frame_reads == ["page-a", "page-b"]


@pytest.mark.asyncio
async def test_owner_frame_requires_opt_in_profile_before_native_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: cast(list[str], []))
    native = _NativeSession("frame-a")
    _context, source, _adopted = _pages(native)
    element = ElementHandle(cast(Any, native), cast(Any, _NativeHandle()), source)

    # Act
    with pytest.raises(UnsupportedError, match="element-handle-dom profile"):
        await element.owner_frame()

    # Assert
    assert native.owner_calls == []
    assert native.page_reads == []


@pytest.mark.asyncio
async def test_owner_frame_rewrites_native_error(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom"])
    native = _NativeSession(None)
    native.failure = Error("Execution context was destroyed")
    _context, source, _adopted = _pages(native)
    element = ElementHandle(cast(Any, native), cast(Any, _NativeHandle()), source)

    # Act
    with pytest.raises(Error, match="ElementHandle.owner_frame: Execution context"):
        await element.owner_frame()

    # Assert
    assert len(native.owner_calls) == 1
    assert native.page_reads == []


def test_sync_owner_frame_uses_canonical_adopted_page_wrapper(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["element-handle-dom"])
    native = _NativeSession("frame-b")
    context, source, adopted = _pages(native)
    bridge = _LoopThread()
    sync_context = SyncBrowserContext(bridge, context, cast(Any, SimpleNamespace()))
    sync_source = sync_context._wrap_page(source)
    owner = _NativeHandle()
    element = SyncElementHandle(
        bridge,
        ElementHandle(cast(Any, native), cast(Any, owner), source),
        sync_source,
    )

    # Act
    try:
        result = element.owner_frame()
        canonical_page = sync_context._wrap_page(adopted)
        canonical_frame = canonical_page._wrap_frame(adopted.main_frame)
    finally:
        bridge.close()

    # Assert
    assert result is canonical_frame
    assert result is not None
    assert result._page is canonical_page
    assert native.owner_calls == [owner]
