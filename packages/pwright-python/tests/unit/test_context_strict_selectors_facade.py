# pyright: reportPrivateUsage=false
"""Context strict-selector defaults are opt-in and resolved in the native policy."""

from __future__ import annotations

from typing import Any, cast

import pytest
from pwright import async_api
from pwright._errors import Error, UnsupportedError


class NativeHandle:
    @staticmethod
    def is_element() -> bool:
        return True

    @staticmethod
    def preview() -> str:
        return "<button>"


class NativeLocator:
    def __init__(self, owner: Native, selector: str, first: bool = False) -> None:
        self.owner = owner
        self.selector = selector
        self.first = first

    def nth(self, index: int) -> NativeLocator:
        assert index == 0
        return NativeLocator(self.owner, self.selector, True)

    async def read(self, operation: str, _timeout_ms: int, _attribute: str | None) -> object:
        assert operation == "count"
        return 1 if self.first else 2

    async def element_handle(self, _timeout_ms: int) -> NativeHandle:
        if not self.first:
            raise Error("strict mode violation")
        return NativeHandle()


class Native:
    def __init__(self) -> None:
        self.context_defaults: dict[str, bool] = {}
        self.next_context_id = 0
        self.read_strictness: list[tuple[str, bool]] = []
        self.creation_kwargs: list[dict[str, object]] = []

    async def new_context(self, *_args: object, **kwargs: object) -> tuple[str, None]:
        self.next_context_id += 1
        context_id = str(self.next_context_id)
        self.context_defaults[context_id] = kwargs.get("strict_selectors") is True
        self.creation_kwargs.append(dict(kwargs))
        return context_id, None

    def context_selector_is_strict(self, context_id: str, strict: bool | None) -> bool:
        return self.context_defaults[context_id] if strict is None else strict

    def page_default_timeout(self, _page_id: str, _navigation: bool) -> int:
        return 30000

    async def locator_read(
        self,
        _frame_id: str,
        selector: str,
        _operation: str,
        _timeout_ms: int,
        _attribute: str | None,
        strict: bool,
    ) -> str:
        self.read_strictness.append((selector, strict))
        if strict:
            raise Error("strict mode violation")
        return "first"

    def validate_selector(self, _frame_id: str, _selector: str) -> None:
        return None

    async def registered_selector_match_count(self, _frame_id: str, _selector: str) -> None:
        return None

    def locator(self, _frame_id: str, selector: str) -> NativeLocator:
        return NativeLocator(self, selector)


def browser(native: Native) -> async_api.Browser:
    value = object.__new__(async_api.Browser)
    value._native = cast(Any, native)
    value._contexts = {}
    return value


@pytest.mark.asyncio
async def test_context_policy_is_isolated_and_explicit_options_win(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["context-strict-selectors"]
    )
    native = Native()
    owner = browser(native)
    strict_context = await owner.new_context(strict_selectors=True)
    ordinary_context = await owner.new_context()
    strict_page = async_api.Page(strict_context, "strict-page", "strict-frame", "about:blank")
    ordinary_page = async_api.Page(
        ordinary_context, "ordinary-page", "ordinary-frame", "about:blank"
    )

    # Act and Assert
    with pytest.raises(Error, match="strict mode violation"):
        await strict_page.text_content("button")
    assert await strict_page.text_content("button", strict=False) == "first"
    assert await ordinary_page.text_content("button") == "first"
    with pytest.raises(Error, match="strict mode violation"):
        await ordinary_page.text_content("button", strict=True)
    assert native.creation_kwargs == [{"strict_selectors": True}, {}]
    assert native.read_strictness == [
        ("button", True),
        ("button", False),
        ("button", False),
        ("button", True),
    ]


@pytest.mark.asyncio
async def test_query_selector_uses_same_context_policy_and_explicit_override(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["context-strict-selectors"]
    )
    native = Native()
    owner = browser(native)
    context = await owner.new_context(strict_selectors=True)
    page = async_api.Page(context, "page", "frame", "about:blank")

    # Act and Assert
    with pytest.raises(Error, match="strict mode violation"):
        await page.query_selector("button")
    assert isinstance(await page.query_selector("button", strict=False), async_api.ElementHandle)


@pytest.mark.asyncio
async def test_frame_query_selector_inherits_context_and_accepts_explicit_false(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["context-strict-selectors"]
    )
    native = Native()
    owner = browser(native)
    context = await owner.new_context(strict_selectors=True)
    page = async_api.Page(context, "page", "main-frame", "about:blank")
    frame = async_api.Frame(page, "child-frame", "main-frame", "child", "about:blank")

    # Act and Assert
    with pytest.raises(Error, match="strict mode violation"):
        await frame.query_selector("button")
    assert isinstance(await frame.query_selector("button", strict=False), async_api.ElementHandle)


@pytest.mark.asyncio
async def test_strict_context_requires_independent_capability(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    def no_features() -> list[str]:
        return []

    monkeypatch.setattr(async_api._native, "selected_features", no_features)
    native = Native()
    owner = browser(native)

    # Act and Assert
    with pytest.raises(UnsupportedError, match="context-strict-selectors profile"):
        await owner.new_context(strict_selectors=True)
    assert native.creation_kwargs == []


@pytest.mark.asyncio
async def test_context_strict_option_rejects_non_boolean_before_native_creation(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        async_api._native, "selected_features", lambda: ["context-strict-selectors"]
    )
    native = Native()
    owner = browser(native)

    # Act and Assert
    with pytest.raises(TypeError, match="Browser.new_context strict_selectors"):
        await owner.new_context(strict_selectors=cast(bool, "true"))
    assert native.creation_kwargs == []
