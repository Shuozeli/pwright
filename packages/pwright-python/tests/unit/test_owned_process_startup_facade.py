"""Browserless facade checks for the opt-in, failure-only process-startup leaf."""

from __future__ import annotations

from pathlib import Path
from typing import TypedDict, cast

import pytest
from pwright import Error, TargetClosedError, UnsupportedError
from pwright.async_api import BrowserType as AsyncBrowserType
from pwright.async_api import Playwright as AsyncPlaywright
from pwright.sync_api import BrowserType as SyncBrowserType
from pwright.sync_api import _LoopThread  # pyright: ignore[reportPrivateUsage]


class _UnsupportedLaunchOptions(TypedDict, total=False):
    channel: str
    slow_mo: int


def _no_features() -> list[str]:
    return []


class _StartupEnvironment:
    def __init__(self, error: Exception | None = None) -> None:
        self.error = error
        self.calls: list[tuple[str, str, int]] = []

    async def observe_nonchromium_startup(
        self, engine: str, executable_path: str, timeout_ms: int
    ) -> None:
        self.calls.append((engine, executable_path, timeout_ms))
        if self.error is not None:
            raise self.error


class _Owner:
    def __init__(self, environment: _StartupEnvironment) -> None:
        self._environment = environment
        self._browsers: list[object] = []

    @property
    def browsers(self) -> list[object]:
        return self._browsers


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("engine", "error_type"), [("firefox", Error), ("webkit", TargetClosedError)]
)
async def test_explicit_early_exit_is_forwarded_without_browser(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    engine: str,
    error_type: type[Error],
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["owned-process-startup"]
    )
    executable = tmp_path / "dummy-browser"
    executable.touch()
    environment = _StartupEnvironment(error_type("exited before browser protocol was ready"))
    owner = _Owner(environment)
    browser_type = AsyncBrowserType(cast(AsyncPlaywright, owner), engine)

    # Act
    with pytest.raises(Error, match="exited before browser protocol was ready") as raised:
        await browser_type.launch(executable_path=executable, headless=True, timeout=1250)

    # Assert
    assert type(raised.value) is error_type
    assert environment.calls == [(engine, str(executable), 1250)]
    assert owner.browsers == []


def test_sync_explicit_early_exit_is_forwarded_without_browser(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["owned-process-startup"]
    )
    executable = tmp_path / "dummy-browser"
    executable.touch()
    environment = _StartupEnvironment(TargetClosedError("exited before browser protocol was ready"))
    owner = _Owner(environment)
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, AsyncBrowserType(cast(AsyncPlaywright, owner), "webkit"))

    # Act
    try:
        with pytest.raises(Error, match="exited before browser protocol was ready") as raised:
            browser_type.launch(executable_path=executable)
    finally:
        bridge.close()

    # Assert
    assert type(raised.value) is TargetClosedError
    assert environment.calls == [("webkit", str(executable), 30_000)]
    assert owner.browsers == []


@pytest.mark.asyncio
async def test_default_profile_does_not_probe_explicit_executable(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr("pwright.async_api._native.selected_features", _no_features)
    executable = tmp_path / "dummy-browser"
    executable.touch()
    environment = _StartupEnvironment()
    browser_type = AsyncBrowserType(cast(AsyncPlaywright, _Owner(environment)), "firefox")

    # Act
    with pytest.raises(UnsupportedError, match="only available for Chromium"):
        await browser_type.launch(executable_path=executable)

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
async def test_profile_requires_explicit_executable(monkeypatch: pytest.MonkeyPatch) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["owned-process-startup"]
    )
    environment = _StartupEnvironment()
    browser_type = AsyncBrowserType(cast(AsyncPlaywright, _Owner(environment)), "webkit")

    # Act
    with pytest.raises(UnsupportedError, match="only available for Chromium"):
        await browser_type.launch()

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
@pytest.mark.parametrize("options", [{"channel": "webkit"}, {"slow_mo": 5}])
async def test_profile_rejects_unimplemented_options_before_process(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, options: _UnsupportedLaunchOptions
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["owned-process-startup"]
    )
    executable = tmp_path / "dummy-browser"
    executable.touch()
    environment = _StartupEnvironment()
    browser_type = AsyncBrowserType(cast(AsyncPlaywright, _Owner(environment)), "firefox")

    # Act
    with pytest.raises(UnsupportedError, match="not installed"):
        await browser_type.launch(executable_path=executable, **options)

    # Assert
    assert environment.calls == []


@pytest.mark.asyncio
async def test_probe_completion_cannot_become_browser(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features", lambda: ["owned-process-startup"]
    )
    executable = tmp_path / "dummy-browser"
    executable.touch()
    environment = _StartupEnvironment()
    owner = _Owner(environment)
    browser_type = AsyncBrowserType(cast(AsyncPlaywright, owner), "webkit")

    # Act
    with pytest.raises(UnsupportedError, match="protocol is not installed"):
        await browser_type.launch(executable_path=executable)

    # Assert
    assert environment.calls == [("webkit", str(executable), 30_000)]
    assert owner.browsers == []
