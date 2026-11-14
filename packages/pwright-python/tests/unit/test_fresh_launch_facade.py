# pyright: reportPrivateUsage=false
"""Browserless checks for the optional public Chromium launch boundary."""

from __future__ import annotations

import asyncio
from pathlib import Path
from typing import Any, cast

import pytest
from pwright import Error, ResourceExhaustedError, UnsupportedError
from pwright.async_api import BrowserType as AsyncBrowserType
from pwright.sync_api import BrowserType as SyncBrowserType
from pwright.sync_api import _LoopThread


class _NativeSession:
    def __init__(self) -> None:
        self.closed = False
        self.close_calls = 0
        self.closed_contexts: list[str] = []
        self.created_contexts: list[str] = []

    def contexts(self) -> list[tuple[str, str | None]]:
        return [(context_id, None) for context_id in self.created_contexts]

    async def new_context(self, *_args: object) -> tuple[str, str | None]:
        context_id = f"context-{len(self.created_contexts) + 1}"
        self.created_contexts.append(context_id)
        return context_id, None

    async def close_context(self, context_id: str) -> None:
        self.closed_contexts.append(context_id)

    async def close(self) -> None:
        self.close_calls += 1
        self.closed = True


class _Environment:
    def __init__(self) -> None:
        self.calls: list[tuple[bool, str | None, str | None, int]] = []
        self.session = _NativeSession()

    async def launch_chromium(
        self, headless: bool, executable_path: str | None, channel: str | None, timeout_ms: int
    ) -> _NativeSession:
        self.calls.append((headless, executable_path, channel, timeout_ms))
        return self.session

    async def connect_over_cdp(self, _endpoint_url: str, _timeout_ms: int) -> _NativeSession:
        return self.session


class _HeaderEnvironment:
    def __init__(self) -> None:
        self.session = _NativeSession()
        self.header_calls: list[tuple[str, int, list[tuple[str, str]]]] = []

    async def connect_over_cdp(
        self, endpoint_url: str, timeout_ms: int, headers: list[tuple[str, str]]
    ) -> _NativeSession:
        self.header_calls.append((endpoint_url, timeout_ms, headers))
        return self.session


class _PlaywrightOwner:
    def __init__(self, environment: _Environment | _HeaderEnvironment) -> None:
        self._environment = environment
        self._browsers: list[object] = []


@pytest.mark.asyncio
async def test_chromium_launch_returns_browser_with_no_default_context(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: ["fresh-launch"])
    executable = tmp_path / "installed-chromium"
    executable.touch()
    environment = _Environment()
    owner = _PlaywrightOwner(environment)
    browser_type = AsyncBrowserType(cast(Any, owner), "chromium")

    # Act
    browser = await browser_type.launch(
        executable_path=executable, channel="chromium", headless=False, timeout=1250
    )

    # Assert
    assert browser.contexts == []
    assert environment.calls == [(False, str(executable), "chromium", 1250)]
    assert owner._browsers == [browser]
    await browser.close()
    assert environment.session.closed
    assert owner._browsers == []


@pytest.mark.asyncio
async def test_launch_rejects_page_argument_before_engine_selection() -> None:
    # Arrange
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "webkit")

    # Act
    with pytest.raises(Error, match="can not specify page"):
        await browser_type.launch(args=["http://example.com"])

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
async def test_launch_rejects_nonexistent_executable_before_engine_selection() -> None:
    # Arrange
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "firefox")

    # Act
    with pytest.raises(Error, match="executable doesn't exist"):
        await browser_type.launch(executable_path="random-invalid-path")

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
@pytest.mark.parametrize("engine", ["chromium", "firefox", "webkit"])
async def test_launch_rejects_non_string_proxy_server_before_native(engine: str) -> None:
    # Arrange
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), engine)

    # Act
    with pytest.raises(Error, match="proxy.server: expected string, got number"):
        await browser_type.launch(proxy={"server": 123})

    # Assert
    assert environment.calls == []


@pytest.mark.parametrize("engine", ["chromium", "firefox", "webkit"])
def test_sync_launch_rejects_non_string_proxy_server_before_native(engine: str) -> None:
    # Arrange
    environment = _Environment()
    bridge = _LoopThread()
    browser_type = SyncBrowserType(
        bridge, AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), engine)
    )

    # Act
    try:
        with pytest.raises(Error, match="proxy.server: expected string, got number"):
            browser_type.launch(proxy={"server": 123})
    finally:
        bridge.close()

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("engine", "message"),
    [
        ("chromium", "options are not installed: proxy"),
        ("firefox", "only available for Chromium"),
        ("webkit", "only available for Chromium"),
    ],
)
async def test_launch_valid_proxy_keeps_existing_unsupported_boundary(
    monkeypatch: pytest.MonkeyPatch, engine: str, message: str
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: ["fresh-launch"])
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), engine)

    # Act
    with pytest.raises(UnsupportedError, match=message):
        await browser_type.launch(proxy={"server": "http://proxy.example:8080"})

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("engine", "message"),
    [
        ("chromium", "fresh-launch profile"),
        ("firefox", "only available for Chromium"),
        ("webkit", "only available for Chromium"),
    ],
)
async def test_launch_non_proxy_option_keeps_provider_gate_ordering(
    monkeypatch: pytest.MonkeyPatch, engine: str, message: str
) -> None:
    # Arrange
    def no_features() -> list[str]:
        return []

    monkeypatch.setattr("pwright.async_api._native.selected_features", no_features)
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), engine)

    # Act
    with pytest.raises(UnsupportedError, match=message):
        await browser_type.launch(slow_mo=5)

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
async def test_launch_rejects_uninstalled_options_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: ["fresh-launch"])
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(UnsupportedError, match="args are not installed"):
        await browser_type.launch(args=["--no-sandbox"])
    with pytest.raises(UnsupportedError, match="slow_mo"):
        await browser_type.launch(slow_mo=5)
    with pytest.raises(TypeError, match="headless must be a boolean"):
        await browser_type.launch(headless=cast(Any, "false"))

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
async def test_launch_requires_explicit_native_feature(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    def no_features() -> list[str]:
        return []

    monkeypatch.setattr("pwright.async_api._native.selected_features", no_features)
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(UnsupportedError, match="fresh-launch profile"):
        await browser_type.launch()

    # Assert
    assert environment.calls == []


def test_sync_launch_forwards_narrow_options() -> None:
    # Arrange
    class AsyncType:
        def __init__(self) -> None:
            self.options: dict[str, object] = {}

        async def launch(self, **kwargs: object) -> object:
            self.options = kwargs
            return object()

    class Bridge:
        def run(self, awaitable: Any) -> object:
            return asyncio.run(awaitable)

    async_type = AsyncType()
    browser_type = SyncBrowserType(cast(Any, Bridge()), cast(Any, async_type))

    # Act
    browser = browser_type.launch(channel="chrome", headless=False, timeout=1200)

    # Assert
    assert browser is not None
    assert async_type.options == {
        "executable_path": None,
        "channel": "chrome",
        "headless": False,
        "timeout": 1200,
        "args": None,
    }


def test_sync_cdp_handshake_headers_forward_to_async_facade() -> None:
    # Arrange
    class AsyncType:
        def __init__(self) -> None:
            self.options: dict[str, object] = {}

        async def connect_over_cdp(self, endpoint_url: str, **kwargs: object) -> object:
            self.options = {"endpoint_url": endpoint_url, **kwargs}
            return object()

    class Bridge:
        def run(self, awaitable: Any) -> object:
            return asyncio.run(awaitable)

    async_type = AsyncType()
    browser_type = SyncBrowserType(cast(Any, Bridge()), cast(Any, async_type))

    # Act
    browser = browser_type.connect_over_cdp("ws://example.test/ws", headers={"X-Test": "abc"})

    # Assert
    assert browser is not None
    assert async_type.options == {
        "endpoint_url": "ws://example.test/ws",
        "timeout": None,
        "slow_mo": None,
        "headers": {"X-Test": "abc"},
        "is_local": None,
    }


@pytest.mark.asyncio
async def test_owned_browser_close_emits_all_context_close_events_once(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: ["fresh-launch"])
    executable = tmp_path / "installed-chromium"
    executable.touch()
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")
    browser = await browser_type.launch(executable_path=executable)
    first = await browser.new_context()
    second = await browser.new_context()
    observed: list[tuple[object, bool]] = []
    first.on("close", lambda value: observed.append((value, environment.session.closed)))
    second.on("close", lambda value: observed.append((value, environment.session.closed)))

    # Act
    await asyncio.gather(browser.close(), browser.close())
    await browser.close()

    # Assert
    assert observed == [(first, True), (second, True)]
    assert environment.session.close_calls == 1
    assert environment.session.closed_contexts == []
    assert browser.contexts == []


def test_sync_owned_browser_close_emits_context_close_before_returning(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: ["fresh-launch"])
    executable = tmp_path / "installed-chromium"
    executable.touch()
    environment = _Environment()
    owner = _PlaywrightOwner(environment)
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, AsyncBrowserType(cast(Any, owner), "chromium"))

    # Act
    try:
        browser = browser_type.launch(executable_path=executable)
        context = browser.new_context()
        observed: list[object] = []
        context.on("close", lambda value: observed.append(value))
        browser.close()
        browser.close()
    finally:
        bridge.close()

    # Assert
    assert observed == [context]
    assert environment.session.close_calls == 1
    assert environment.session.closed_contexts == []


@pytest.mark.asyncio
async def test_cdp_attached_browser_close_keeps_existing_context_event_behavior(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    def no_features() -> list[str]:
        return []

    monkeypatch.setattr("pwright.async_api._native.selected_features", no_features)
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")
    browser = await browser_type.connect_over_cdp("http://127.0.0.1:9222")
    context = await browser.new_context()
    observed: list[object] = []
    context.on("close", lambda value: observed.append(value))

    # Act
    await browser.close()

    # Assert
    assert observed == []
    assert environment.session.close_calls == 1
    assert environment.session.closed_contexts == []


@pytest.mark.asyncio
async def test_cdp_handshake_headers_forward_to_native_when_installed(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["cdp-handshake-headers"]
    )
    environment = _HeaderEnvironment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")

    # Act
    browser = await browser_type.connect_over_cdp(
        "ws://example.test/devtools/browser/1", headers={"X-Test": "a", "foo": "bar"}
    )

    # Assert
    assert browser is not None
    assert environment.header_calls == [
        ("ws://example.test/devtools/browser/1", 30_000, [("X-Test", "a"), ("foo", "bar")])
    ]


@pytest.mark.asyncio
async def test_cdp_handshake_headers_require_optional_capability(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: cast(list[str], []))
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(UnsupportedError, match="cdp-handshake-headers profile"):
        await browser_type.connect_over_cdp("ws://example.test/ws", headers={"foo": "bar"})

    # Assert
    assert environment.session.close_calls == 0


@pytest.mark.asyncio
async def test_empty_cdp_handshake_headers_keep_existing_native_signature(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", lambda: cast(list[str], []))
    environment = _Environment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")

    # Act
    browser = await browser_type.connect_over_cdp("ws://example.test/ws", headers={})

    # Assert
    assert browser is not None


@pytest.mark.asyncio
async def test_invalid_cdp_handshake_header_value_fails_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["cdp-handshake-headers"]
    )
    environment = _HeaderEnvironment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(Error, match="header value"):
        await browser_type.connect_over_cdp("ws://example.test/ws", headers=cast(Any, {"foo": 7}))

    # Assert
    assert environment.header_calls == []


@pytest.mark.asyncio
async def test_cdp_handshake_header_count_is_bounded_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["cdp-handshake-headers"]
    )
    environment = _HeaderEnvironment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")
    headers = {f"x-{index}": "v" for index in range(33)}

    # Act
    with pytest.raises(ResourceExhaustedError, match="header count exceeds 32"):
        await browser_type.connect_over_cdp("ws://example.test/ws", headers=headers)

    # Assert
    assert environment.header_calls == []


@pytest.mark.asyncio
async def test_cdp_handshake_header_bytes_are_bounded_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["cdp-handshake-headers"]
    )
    environment = _HeaderEnvironment()
    browser_type = AsyncBrowserType(cast(Any, _PlaywrightOwner(environment)), "chromium")
    headers = {f"x-{index}": "v" * 1024 for index in range(9)}

    # Act
    with pytest.raises(ResourceExhaustedError, match="header bytes exceed 8192"):
        await browser_type.connect_over_cdp("ws://example.test/ws", headers=headers)

    # Assert
    assert environment.header_calls == []
