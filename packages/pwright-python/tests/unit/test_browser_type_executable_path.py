# pyright: reportPrivateUsage=false
"""BrowserType metadata must not imply that a browser was installed."""

import platform
import sys
from inspect import getfile
from pathlib import Path
from typing import Any, cast

import pytest
from pwright import _native
from pwright.async_api import BrowserType as AsyncBrowserType
from pwright.sync_api import BrowserType as SyncBrowserType

_MAC_ARM64_PATHS = {
    "chromium": Path(
        "chromium-1208/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/"
        "Google Chrome for Testing"
    ),
    "firefox": Path("firefox-1509/firefox/Nightly.app/Contents/MacOS/firefox"),
    "webkit": Path("webkit-2248/pw_run.sh"),
}


@pytest.mark.skipif(
    sys.platform != "darwin" or platform.machine() != "arm64",
    reason="pinned Playwright v1.58.0 macOS arm64 executable paths",
)
@pytest.mark.parametrize("engine", ["chromium", "firefox", "webkit"])
def test_native_registry_reports_expected_uninstalled_path(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, engine: str
) -> None:
    # Arrange
    monkeypatch.setenv("PLAYWRIGHT_BROWSERS_PATH", str(tmp_path))
    expected = tmp_path / _MAC_ARM64_PATHS[engine]

    # Act
    actual = _native.browser_executable_path(engine, str(tmp_path))

    # Assert
    assert actual == str(expected)
    assert not expected.exists()


@pytest.mark.skipif(
    sys.platform != "darwin" or platform.machine() != "arm64",
    reason="pinned Playwright v1.58.0 macOS arm64 executable paths",
)
def test_zero_browser_path_is_explicitly_pwright_package_local(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    # Arrange
    monkeypatch.setenv("PLAYWRIGHT_BROWSERS_PATH", "0")
    package_root = tmp_path / "pwright-package"
    expected = package_root / ".local-browsers" / _MAC_ARM64_PATHS["chromium"]

    # Act
    actual = _native.browser_executable_path("chromium", str(package_root))

    # Assert
    assert actual == str(expected)
    assert not expected.exists()


@pytest.mark.skipif(
    sys.platform != "darwin" or platform.machine() != "arm64",
    reason="pinned Playwright v1.58.0 macOS arm64 executable paths",
)
@pytest.mark.parametrize("engine", ["chromium", "firefox", "webkit"])
@pytest.mark.parametrize("api", ["async", "sync"])
def test_zero_browser_path_public_api_stays_in_pwright_package(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, engine: str, api: str
) -> None:
    # Arrange
    monkeypatch.setenv("PLAYWRIGHT_BROWSERS_PATH", "0")
    monkeypatch.setenv("INIT_CWD", str(tmp_path))
    async_browser_type = AsyncBrowserType(cast(Any, object()), engine)
    browser_type = (
        async_browser_type
        if api == "async"
        else SyncBrowserType(cast(Any, object()), async_browser_type)
    )
    package_root = Path(getfile(AsyncBrowserType)).resolve().parent
    expected = package_root / ".local-browsers" / _MAC_ARM64_PATHS[engine]

    # Act
    actual = browser_type.executable_path

    # Assert
    assert actual == str(expected)
    assert not expected.is_relative_to(tmp_path)


@pytest.mark.skipif(
    sys.platform != "darwin" or platform.machine() != "arm64",
    reason="pinned Playwright v1.58.0 macOS arm64 executable paths",
)
@pytest.mark.parametrize("engine", ["chromium", "firefox", "webkit"])
def test_async_browser_type_uses_rust_registry(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, engine: str
) -> None:
    # Arrange
    monkeypatch.setenv("PLAYWRIGHT_BROWSERS_PATH", str(tmp_path))
    browser_type = AsyncBrowserType(cast(Any, object()), engine)

    # Act
    actual = browser_type.executable_path

    # Assert
    assert actual == str(tmp_path / _MAC_ARM64_PATHS[engine])


@pytest.mark.skipif(
    sys.platform != "darwin" or platform.machine() != "arm64",
    reason="pinned Playwright v1.58.0 macOS arm64 executable paths",
)
@pytest.mark.parametrize("engine", ["chromium", "firefox", "webkit"])
def test_sync_browser_type_uses_rust_registry(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, engine: str
) -> None:
    # Arrange
    monkeypatch.setenv("PLAYWRIGHT_BROWSERS_PATH", str(tmp_path))
    async_browser_type = AsyncBrowserType(cast(Any, object()), engine)
    browser_type = SyncBrowserType(cast(Any, object()), async_browser_type)

    # Act
    actual = browser_type.executable_path

    # Assert
    assert actual == str(tmp_path / _MAC_ARM64_PATHS[engine])
