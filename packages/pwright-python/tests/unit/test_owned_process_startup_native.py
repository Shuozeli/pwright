"""Real-process proof for the optional, failure-only native startup boundary."""

from __future__ import annotations

import os
from pathlib import Path

import pytest
from pwright import Error, TargetClosedError, native_capabilities
from pwright.async_api import async_playwright
from pwright.sync_api import sync_playwright


def _has_profile() -> bool:
    features = native_capabilities()["features"]
    return isinstance(features, list) and "owned-process-startup" in features


_requires_native_probe = pytest.mark.skipif(
    os.name != "posix" or not Path("/bin/sh").is_file() or not _has_profile(),
    reason="requires the opt-in owned-process-startup native profile on POSIX",
)


@pytest.fixture
def exiting_executable(tmp_path: Path) -> Path:
    executable = tmp_path / "exiting-browser-fixture"
    executable.write_text("#!/bin/sh\nexit 7\n")
    executable.chmod(0o700)
    return executable


@_requires_native_probe
@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("engine", "error_type"), [("firefox", Error), ("webkit", TargetClosedError)]
)
async def test_async_launch_observes_real_exit_without_browser(
    exiting_executable: Path, engine: str, error_type: type[Error]
) -> None:
    # Arrange
    async with async_playwright() as playwright:
        browser_type = playwright.firefox if engine == "firefox" else playwright.webkit

        # Act
        with pytest.raises(Error, match="exit code 7") as raised:
            await browser_type.launch(executable_path=exiting_executable, headless=True)

        # Assert
        assert type(raised.value) is error_type


@_requires_native_probe
@pytest.mark.parametrize(
    ("engine", "error_type"), [("firefox", Error), ("webkit", TargetClosedError)]
)
def test_sync_launch_observes_real_exit_without_browser(
    exiting_executable: Path, engine: str, error_type: type[Error]
) -> None:
    # Arrange
    with sync_playwright() as playwright:
        browser_type = playwright.firefox if engine == "firefox" else playwright.webkit

        # Act
        with pytest.raises(Error, match="exit code 7") as raised:
            browser_type.launch(executable_path=exiting_executable, headless=True)

        # Assert
        assert type(raised.value) is error_type
