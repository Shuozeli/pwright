# pyright: reportPrivateUsage=false

from __future__ import annotations

import asyncio
import json
from pathlib import Path
from typing import Any, cast

import pytest
from pwright import Error, InvalidArgumentError, UnsupportedError
from pwright.async_api import Browser, BrowserContext, StorageState
from pwright.sync_api import Browser as SyncBrowser
from pwright.sync_api import BrowserContext as SyncBrowserContext
from pwright.sync_api import _LoopThread

STATE: StorageState = {"cookies": [], "origins": []}


class NativeStorageState:
    def __init__(self) -> None:
        self.active = False
        self.created = False
        self.set_json: str | None = None
        self.snapshot_indexed_db: bool | None = None
        self.closed: list[str] = []
        self.unrelated_contexts: list[tuple[str, None]] = []
        self.creation_started = asyncio.Event()
        self.allow_creation = asyncio.Event()
        self.allow_creation.set()
        self.restore_started = asyncio.Event()
        self.allow_restore = asyncio.Event()
        self.close_started = asyncio.Event()
        self.allow_close = asyncio.Event()
        self.allow_close.set()
        self.fail_restore = False
        self.fail_close = False

    async def new_context(self, *_args: object) -> tuple[str, None]:
        self.created = True
        self.active = True
        self.creation_started.set()
        await self.allow_creation.wait()
        return "context-1", None

    def contexts(self) -> list[tuple[str, None]]:
        return ([("context-1", None)] if self.active else []) + self.unrelated_contexts

    async def context_set_storage_state(self, context_id: str, state_json: str) -> None:
        assert context_id == "context-1"
        self.set_json = state_json
        self.restore_started.set()
        await self.allow_restore.wait()
        if self.fail_restore:
            raise InvalidArgumentError("invalid storage state")

    async def context_storage_state(self, context_id: str, indexed_db: bool) -> StorageState:
        assert context_id == "context-1"
        self.snapshot_indexed_db = indexed_db
        return STATE

    async def close_context(self, context_id: str) -> None:
        self.closed.append(context_id)
        self.close_started.set()
        await self.allow_close.wait()
        if self.fail_close:
            raise RuntimeError("native close failed")
        self.active = False


def _browser(native: NativeStorageState) -> Browser:
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    browser._contexts = {}
    browser._pending_context_ids = set()
    browser._pending_context_cleanup_tasks = {}
    browser._unidentified_storage_state_creations = 0
    browser._persistent_context_viewport = None
    browser._closed = False
    return browser


@pytest.mark.asyncio
async def test_storage_state_snapshot_writes_returned_native_state(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    browser = _browser(native)
    context = BrowserContext(browser, "context-1", None)
    path = tmp_path / "state.json"

    # Act
    state = await context.storage_state(path=path, indexed_db=True)

    # Assert
    assert state == STATE
    assert json.loads(path.read_text("utf-8")) == STATE
    assert native.snapshot_indexed_db is True


@pytest.mark.asyncio
async def test_new_context_restores_state_before_it_becomes_public(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    native.allow_creation.clear()
    browser = _browser(native)
    task = asyncio.create_task(browser.new_context(storage_state=STATE))
    await native.creation_started.wait()

    # A native Context exists before its ID is returned, but no Page is created
    # by new_context. The facade must not discover it through Browser.contexts.
    unpublished_before_id = browser.contexts
    native.allow_creation.set()
    await native.restore_started.wait()

    # Act
    browser._receive_lifecycle_notice(
        2,
        "context-1",
        "temporary-restore-page",
        None,
        None,
        "temporary-frame",
        "about:blank",
        None,
        None,
    )
    unpublished_during_restore = browser.contexts
    native.allow_restore.set()
    context = await task

    # Assert
    assert unpublished_before_id == []
    assert unpublished_during_restore == []
    assert browser.contexts == [context]
    assert json.loads(cast(str, native.set_json)) == STATE
    assert native.closed == []


@pytest.mark.asyncio
async def test_unrelated_native_context_remains_visible_during_storage_restore(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    browser = _browser(native)
    task = asyncio.create_task(browser.new_context(storage_state=STATE))
    await native.restore_started.wait()
    native.unrelated_contexts.append(("unrelated-context", None))

    try:
        # Act
        visible = browser.contexts

        # Assert
        assert [context._object_id for context in visible] == ["unrelated-context"]
        assert not visible[0]._closed
    finally:
        native.allow_restore.set()
        await task


@pytest.mark.asyncio
async def test_context_collection_closes_removed_context_during_storage_restore(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    native.unrelated_contexts.append(("unrelated-context", None))
    browser = _browser(native)
    unrelated = browser.contexts[0]
    task = asyncio.create_task(browser.new_context(storage_state=STATE))
    await native.restore_started.wait()
    native.unrelated_contexts.clear()

    try:
        # Act
        visible = browser.contexts

        # Assert
        assert visible == []
        assert unrelated._closed
        assert browser._contexts == {}
    finally:
        native.allow_restore.set()
        await task


@pytest.mark.asyncio
async def test_new_context_closes_native_context_when_restore_fails(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    native.allow_restore.set()
    native.fail_restore = True
    browser = _browser(native)

    # Act
    with pytest.raises(InvalidArgumentError, match="invalid storage state"):
        await browser.new_context(storage_state=STATE)

    # Assert
    assert native.closed == ["context-1"]
    assert browser.contexts == []


@pytest.mark.asyncio
async def test_new_context_reports_cleanup_failure_and_keeps_context_unpublished(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    native.allow_restore.set()
    native.fail_restore = True
    native.fail_close = True
    browser = _browser(native)

    # Act
    with pytest.raises(Error, match="Context cleanup failed: native close failed") as raised:
        await browser.new_context(storage_state=STATE)

    # Assert
    assert "invalid storage state" in str(raised.value)
    assert native.closed == ["context-1"]
    assert browser.contexts == []


@pytest.mark.asyncio
async def test_new_context_cancellation_after_id_closes_native_context(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    browser = _browser(native)
    task = asyncio.create_task(browser.new_context(storage_state=STATE))
    await native.restore_started.wait()

    # Act
    task.cancel()
    with pytest.raises(asyncio.CancelledError):
        await task

    # Assert
    assert native.closed == ["context-1"]
    assert not native.active
    assert browser.contexts == []
    assert browser._pending_context_ids == set()
    assert browser._pending_context_cleanup_tasks == {}


@pytest.mark.asyncio
async def test_new_context_second_cancellation_preserves_detached_cleanup(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    native.allow_close.clear()
    browser = _browser(native)
    task = asyncio.create_task(browser.new_context(storage_state=STATE))
    await native.restore_started.wait()
    task.cancel()
    await native.close_started.wait()

    # Act
    task.cancel()
    with pytest.raises(asyncio.CancelledError):
        await task
    cleanup_task = browser._pending_context_cleanup_tasks["context-1"]
    hidden_while_closing = browser.contexts
    native.allow_close.set()
    await cleanup_task
    await asyncio.sleep(0)

    # Assert
    assert hidden_while_closing == []
    assert native.closed == ["context-1"]
    assert not native.active
    assert browser._pending_context_ids == set()
    assert browser._pending_context_cleanup_tasks == {}


@pytest.mark.asyncio
async def test_new_context_detached_cleanup_failure_keeps_context_hidden(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    native.allow_close.clear()
    native.fail_close = True
    browser = _browser(native)
    loop = asyncio.get_running_loop()
    errors: list[dict[str, Any]] = []
    previous_handler = loop.get_exception_handler()
    loop.set_exception_handler(lambda _loop, context: errors.append(context))
    try:
        task = asyncio.create_task(browser.new_context(storage_state=STATE))
        await native.restore_started.wait()
        task.cancel()
        await native.close_started.wait()

        # Act
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task
        cleanup_task = browser._pending_context_cleanup_tasks["context-1"]
        native.allow_close.set()
        with pytest.raises(RuntimeError, match="native close failed"):
            await cleanup_task
        await asyncio.sleep(0)

        # Assert
        assert native.active
        assert browser.contexts == []
        assert browser._pending_context_ids == {"context-1"}
        assert browser._pending_context_cleanup_tasks == {}
        assert len(errors) == 1
        assert isinstance(errors[0]["exception"], RuntimeError)
    finally:
        loop.set_exception_handler(previous_handler)


@pytest.mark.asyncio
async def test_new_context_accepts_storage_state_path(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    path = tmp_path / "state.json"
    path.write_text(json.dumps(STATE), "utf-8")
    native = NativeStorageState()
    native.allow_restore.set()
    browser = _browser(native)

    # Act
    context = await browser.new_context(storage_state=path)

    # Assert
    assert context in browser.contexts
    assert json.loads(cast(str, native.set_json)) == STATE


@pytest.mark.asyncio
async def test_storage_state_rejects_invalid_arguments_before_native_creation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )
    native = NativeStorageState()
    browser = _browser(native)
    context = BrowserContext(browser, "context-1", None)

    # Act
    with pytest.raises(TypeError, match="must be a mapping or path"):
        await browser.new_context(storage_state=cast(Any, 7))
    with pytest.raises(TypeError, match="must be a boolean"):
        await context.storage_state(indexed_db=cast(Any, "yes"))

    # Assert
    assert not native.created
    assert native.snapshot_indexed_db is None


@pytest.mark.asyncio
async def test_storage_state_requires_its_explicit_feature(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    def no_features() -> list[str]:
        return []

    monkeypatch.setattr("pwright.async_api._native.selected_features", no_features)
    native = NativeStorageState()
    browser = _browser(native)
    context = BrowserContext(browser, "context-1", None)

    # Act
    with pytest.raises(UnsupportedError, match="context-storage-state"):
        await browser.new_context(storage_state=STATE)
    with pytest.raises(UnsupportedError, match="context-storage-state"):
        await context.storage_state()

    # Assert
    assert not native.created
    assert native.snapshot_indexed_db is None


def test_sync_storage_state_facade_delegates_to_async_native_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-storage-state"],
    )

    bridge = _LoopThread()

    async def native_on_owner_loop() -> NativeStorageState:
        native = NativeStorageState()
        native.allow_restore.set()
        return native

    # Act
    try:
        native = bridge.run(native_on_owner_loop())
        async_browser = _browser(native)
        sync_browser = SyncBrowser(bridge, async_browser, cast(Any, object()))
        context: SyncBrowserContext = sync_browser.new_context(storage_state=STATE)
        state = context.storage_state(indexed_db=True)
    finally:
        bridge.close()

    # Assert
    assert state == STATE
    assert native.snapshot_indexed_db is True
    assert json.loads(cast(str, native.set_json)) == STATE
