"""Fixtures that run selected upstream tests through a CDP-attached browser."""

from __future__ import annotations

import os
import sys
import types
from collections.abc import AsyncGenerator

import pytest
import pytest_asyncio


class Server:
    """Minimal public surface used by the selected upstream tests."""

    def __init__(self, prefix: str) -> None:
        self.PREFIX = prefix.rstrip("/")
        self.EMPTY_PAGE = f"{self.PREFIX}/empty.html"


class TestServerRequest:
    """Import-only stand-in; selected tests never construct this type."""


server_module = types.ModuleType("tests.server")
setattr(server_module, "Server", Server)
setattr(server_module, "TestServerRequest", TestServerRequest)
sys.modules["tests.server"] = server_module

utils_module = types.ModuleType("tests.utils")
setattr(utils_module, "TARGET_CLOSED_ERROR_MESSAGE", "Target closed")


def must(value: object) -> object:
    """Import-only stand-in for unselected functions in the upstream module."""

    return value


setattr(utils_module, "must", must)
sys.modules["tests.utils"] = utils_module

from playwright.async_api import BrowserContext, Page, async_playwright  # noqa: E402


@pytest.fixture
def server() -> Server:
    prefix = os.environ.get("PWRIGHT_OFFICIAL_SERVER_PREFIX")
    if not prefix:
        pytest.fail("PWRIGHT_OFFICIAL_SERVER_PREFIX is required")
    return Server(prefix)


@pytest_asyncio.fixture
async def page() -> AsyncGenerator[Page, None]:
    endpoint = os.environ.get("PWRIGHT_CDP_ENDPOINT")
    if not endpoint:
        pytest.fail("PWRIGHT_CDP_ENDPOINT is required")

    async with async_playwright() as playwright:
        browser = await playwright.chromium.connect_over_cdp(endpoint)
        context: BrowserContext | None = None
        owned_page: Page | None = None
        try:
            context = await browser.new_context()
            owned_page = await context.new_page()
            yield owned_page
        finally:
            if owned_page is not None and not owned_page.is_closed():
                await owned_page.close()
            if context is not None:
                await context.close()
            await browser.close()
