# pyright: reportPrivateUsage=false
"""Page.click adapts bounded options into one Rust-owned pointer action."""

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright._errors import InvalidArgumentError, UnsupportedError
from pwright.async_api import Page
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import _LoopThread


class _Native:
    def __init__(self, *, strict_default: bool = False) -> None:
        self.calls: list[tuple[object, ...]] = []
        self.strict_default = strict_default

    def page_default_timeout(self, page_id: str, navigation: bool) -> int:
        self.calls.append(("page_default_timeout", page_id, navigation))
        return 1234

    async def click_with_options(
        self,
        frame_id: str,
        selector: str,
        options: dict[str, object],
        strict: bool,
        timeout_ms: int,
    ) -> None:
        self.calls.append(("click_with_options", frame_id, selector, options, strict, timeout_ms))

    def context_selector_is_strict(self, context_id: str, explicit: bool | None) -> bool:
        self.calls.append(("context_selector_is_strict", context_id, explicit))
        return self.strict_default if explicit is None else explicit

    def pages(self, context_id: str) -> list[tuple[str, str, str, str]]:
        self.calls.append(("pages", context_id))
        return [("page-1", context_id, "frame-2", "about:blank")]


def _page(native: _Native) -> Page:
    class Browser:
        def __init__(self) -> None:
            self._native = native

    class Context:
        def __init__(self) -> None:
            self.browser = Browser()
            self._object_id = "context-1"

    return Page(cast(Any, Context()), "page-1", "frame-2", "about:blank")


def _no_features() -> list[str]:
    return []


@pytest.mark.asyncio
async def test_page_click_without_options_selects_first_match(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr("pwright.async_api._native.selected_features", _no_features)

    # Act
    await page.click("a")

    # Assert
    assert native.calls == [
        ("page_default_timeout", "page-1", False),
        ("click_with_options", "frame-2", "a", {}, False, 1234),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
@pytest.mark.parametrize("name", ["Shift", "ControlOrMeta"])
async def test_page_click_forwards_modifier_to_opt_in_native_call(
    monkeypatch: pytest.MonkeyPatch, name: str
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["locator-click-modifiers"],
    )

    # Act
    await page.click("a", modifiers=[cast(Any, name)], timeout=75)

    # Assert
    assert native.calls == [
        ("click_with_options", "frame-2", "a", {"modifiers": [name]}, False, 75),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
async def test_page_click_empty_modifiers_is_explicit_native_override(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["locator-click-modifiers"],
    )

    # Act
    await page.click("a", modifiers=[])

    # Assert
    assert native.calls == [
        ("page_default_timeout", "page-1", False),
        ("click_with_options", "frame-2", "a", {"modifiers": []}, False, 1234),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
async def test_page_click_modifiers_require_opt_in_profile(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: cast(list[str], []))

    # Act
    with pytest.raises(UnsupportedError, match="locator-click-modifiers"):
        await page.click("a", modifiers=["Shift"])

    # Assert
    assert native.calls == []


@pytest.mark.asyncio
async def test_page_click_rejects_oversized_modifier_sequence_before_native_action(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["locator-click-modifiers"],
    )

    # Act
    with pytest.raises(InvalidArgumentError, match="at most five"):
        await page.click("a", modifiers=cast(Any, ["Shift"] * 6), timeout=75)

    # Assert
    assert native.calls == []


def test_sync_page_click_forwards_modifiers_through_async_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native()
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(native), cast(Any, None))
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["locator-click-modifiers"],
    )

    # Act
    try:
        page.click("a", modifiers=["ControlOrMeta"], timeout=40)
    finally:
        bridge.close()

    # Assert
    assert native.calls == [
        ("click_with_options", "frame-2", "a", {"modifiers": ["ControlOrMeta"]}, False, 40),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("option", "value"),
    [
        ("position", {"x": 2.5, "y": 3.25}),
        ("button", "right"),
        ("click_count", 2),
        ("force", True),
    ],
)
async def test_page_click_forwards_each_pointer_option(
    monkeypatch: pytest.MonkeyPatch, option: str, value: object
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr("pwright.async_api._native.selected_features", _no_features)

    # Act
    await page.click("button", **cast(Any, {option: value}), timeout=75)

    # Assert
    assert native.calls == [
        ("click_with_options", "frame-2", "button", {option: value}, False, 75),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
async def test_page_click_combines_options_modifiers_and_context_strict_default(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native(strict_default=True)
    page = _page(native)
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["locator-click-modifiers", "context-strict-selectors"],
    )

    # Act
    await page.click(
        "button",
        button="middle",
        click_count=2,
        force=False,
        position={"x": 1.25, "y": 4.5},
        modifiers=["Shift"],
        timeout=0,
    )

    # Assert
    assert native.calls == [
        ("context_selector_is_strict", "context-1", None),
        (
            "click_with_options",
            "frame-2",
            "button",
            {
                "button": "middle",
                "click_count": 2,
                "force": False,
                "position": {"x": 1.25, "y": 4.5},
                "modifiers": ["Shift"],
            },
            True,
            0,
        ),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
async def test_page_click_explicit_non_strict_overrides_context_default(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native(strict_default=True)
    page = _page(native)
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["context-strict-selectors"],
    )

    # Act
    await page.click("button", strict=False, timeout=25)

    # Assert
    assert native.calls == [
        ("context_selector_is_strict", "context-1", False),
        ("click_with_options", "frame-2", "button", {}, False, 25),
        ("pages", "context-1"),
    ]


@pytest.mark.asyncio
@pytest.mark.parametrize("count", [0, 256, True])
async def test_page_click_rejects_invalid_count_before_action(
    monkeypatch: pytest.MonkeyPatch, count: object
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr("pwright.async_api._native.selected_features", _no_features)

    # Act
    with pytest.raises(InvalidArgumentError, match="click_count"):
        await page.click("button", click_count=cast(Any, count), timeout=25)

    # Assert
    assert native.calls == []


@pytest.mark.asyncio
async def test_page_click_rejects_nonfinite_position_before_action(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native()
    page = _page(native)
    monkeypatch.setattr("pwright.async_api._native.selected_features", _no_features)

    # Act
    with pytest.raises(ValueError, match="finite"):
        await page.click("button", position={"x": float("nan"), "y": 1.0}, timeout=25)

    # Assert
    assert native.calls == []


def test_sync_page_click_forwards_combined_pointer_options(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native = _Native()
    bridge = _LoopThread()
    page = SyncPage(bridge, _page(native), cast(Any, None))
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["locator-click-modifiers"],
    )

    # Act
    try:
        page.click(
            "button",
            button="right",
            click_count=2,
            force=True,
            modifiers=["ControlOrMeta"],
            position={"x": 3.0, "y": 4.0},
            strict=True,
            timeout=40,
        )
    finally:
        bridge.close()

    # Assert
    assert native.calls == [
        (
            "click_with_options",
            "frame-2",
            "button",
            {
                "button": "right",
                "click_count": 2,
                "force": True,
                "position": {"x": 3.0, "y": 4.0},
                "modifiers": ["ControlOrMeta"],
            },
            True,
            40,
        ),
        ("pages", "context-1"),
    ]
