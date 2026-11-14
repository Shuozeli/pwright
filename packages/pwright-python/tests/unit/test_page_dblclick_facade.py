# pyright: reportPrivateUsage=false
"""Page.dblclick selects a Rust-owned Locator and refreshes its Page snapshot."""

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import async_api
from pwright._errors import Error
from pwright.async_api import Page
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import _LoopThread


class NativeLocator:
    def __init__(
        self,
        native: Native,
        frame_id: str,
        selector: str,
        first: bool = False,
    ) -> None:
        self.native = native
        self.frame_id = frame_id
        self.selector = selector
        self.first = first

    def nth(self, index: int) -> NativeLocator:
        self.native.calls.append(("nth", self.frame_id, self.selector, index))
        return NativeLocator(self.native, self.frame_id, self.selector, first=True)

    async def dblclick(self, timeout_ms: int) -> None:
        self.native.calls.append(("dblclick", self.frame_id, self.selector, self.first, timeout_ms))
        if self.native.fail_action:
            raise Error("native double click failed")


class Native:
    def __init__(self, *, strict_default: bool = False, fail_action: bool = False) -> None:
        self.calls: list[tuple[object, ...]] = []
        self.strict_default = strict_default
        self.fail_action = fail_action

    def locator(self, frame_id: str, selector: str) -> NativeLocator:
        self.calls.append(("locator", frame_id, selector))
        return NativeLocator(self, frame_id, selector)

    def context_selector_is_strict(self, context_id: str, strict: bool | None) -> bool:
        self.calls.append(("context_selector_is_strict", context_id, strict))
        return self.strict_default if strict is None else strict

    def page_default_timeout(self, page_id: str, navigation: bool) -> int:
        self.calls.append(("page_default_timeout", page_id, navigation))
        return 4321

    def pages(self, context_id: str) -> list[tuple[str, str, str, str]]:
        self.calls.append(("pages", context_id))
        return [("page-1", context_id, "new-main-frame", "https://example.test/after")]


def page_with_native(native: Native) -> Page:
    class Browser:
        def __init__(self) -> None:
            self._native = native

    class Context:
        def __init__(self) -> None:
            self.browser = Browser()
            self._object_id = "context-1"

    return Page(cast(Any, Context()), "page-1", "main-frame", "about:blank")


def no_features() -> list[str]:
    return []


@pytest.mark.asyncio
async def test_page_dblclick_uses_first_match_default_timeout_and_refreshes_snapshot(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", no_features)
    native = Native()
    page = page_with_native(native)

    # Act
    await page.dblclick("button")

    # Assert
    assert native.calls == [
        ("locator", "main-frame", "button"),
        ("nth", "main-frame", "button", 0),
        ("page_default_timeout", "page-1", False),
        ("dblclick", "main-frame", "button", True, 4321),
        ("pages", "context-1"),
    ]
    assert page.url == "https://example.test/after"
    assert page._main_frame_id == "new-main-frame"


@pytest.mark.asyncio
async def test_page_dblclick_explicit_strict_retains_unique_locator_and_zero_timeout(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", no_features)
    native = Native()
    page = page_with_native(native)

    # Act
    await page.dblclick("button", strict=True, timeout=0)

    # Assert
    assert native.calls == [
        ("locator", "main-frame", "button"),
        ("dblclick", "main-frame", "button", False, 0),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("explicit", "expected_first"), [(None, False), (False, True), (True, False)]
)
async def test_page_dblclick_resolves_context_strict_default_and_explicit_override(
    monkeypatch: pytest.MonkeyPatch, explicit: bool | None, expected_first: bool
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["context-strict-selectors"]
    )
    native = Native(strict_default=True)
    page = page_with_native(native)

    # Act
    await page.dblclick("button", strict=explicit, timeout=50)

    # Assert
    expected: list[tuple[object, ...]] = [
        ("locator", "main-frame", "button"),
        ("context_selector_is_strict", "context-1", explicit),
    ]
    if expected_first:
        expected.append(("nth", "main-frame", "button", 0))
    expected.extend(
        [
            ("dblclick", "main-frame", "button", expected_first, 50),
            ("pages", "context-1"),
        ]
    )
    assert native.calls == expected


@pytest.mark.asyncio
async def test_page_dblclick_validates_selector_before_native_action(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", no_features)
    native = Native()
    page = page_with_native(native)

    # Act
    with pytest.raises(Error, match="Page.dblclick: expected string"):
        await page.dblclick(cast(str, None))

    # Assert
    assert native.calls == []


@pytest.mark.asyncio
async def test_page_dblclick_rejects_non_boolean_strict_before_action(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", no_features)
    native = Native()
    page = page_with_native(native)

    # Act
    with pytest.raises(TypeError, match="Page.dblclick strict must be a boolean"):
        await page.dblclick("button", strict=cast(bool, "true"))

    # Assert
    assert native.calls == [("locator", "main-frame", "button")]


@pytest.mark.asyncio
async def test_page_dblclick_does_not_refresh_snapshot_when_native_action_fails(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", no_features)
    native = Native(fail_action=True)
    page = page_with_native(native)

    # Act
    with pytest.raises(Error, match="native double click failed"):
        await page.dblclick("button", timeout=25)

    # Assert
    assert native.calls == [
        ("locator", "main-frame", "button"),
        ("nth", "main-frame", "button", 0),
        ("dblclick", "main-frame", "button", True, 25),
    ]
    assert page.url == "about:blank"


def test_sync_page_dblclick_forwards_selector_timeout_and_strict(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(async_api._native, "selected_features", no_features)
    native = Native()
    bridge = _LoopThread()
    page = SyncPage(bridge, page_with_native(native), cast(Any, None))

    # Act
    try:
        page.dblclick("button", timeout=75, strict=True)
    finally:
        bridge.close()

    # Assert
    assert native.calls == [
        ("locator", "main-frame", "button"),
        ("dblclick", "main-frame", "button", False, 75),
        ("pages", "context-1"),
    ]
