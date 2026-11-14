"""Oracle-only launch adapter for unchanged upstream Playwright Python tests."""

from __future__ import annotations

import os
import sys
from collections.abc import Awaitable, Callable
from importlib import import_module
from typing import cast

from playwright.async_api import Browser as AsyncBrowser
from playwright.async_api import BrowserContext as AsyncBrowserContext
from playwright.async_api import BrowserType as AsyncBrowserType
from playwright.sync_api import Browser as SyncBrowser
from playwright.sync_api import BrowserContext as SyncBrowserContext
from playwright.sync_api import BrowserType as SyncBrowserType

_installed = False


def _context_identity(context: AsyncBrowserContext | SyncBrowserContext) -> int:
    return id(getattr(context, "_impl_obj"))


def _validate_launch_options(options: dict[str, object]) -> None:
    unsupported = sorted(set(options) - {"headless", "handle_sigint", "channel"})
    if unsupported:
        raise RuntimeError(
            "official oracle launch adapter cannot apply options: " + ", ".join(unsupported)
        )
    if "handle_sigint" in options and options["handle_sigint"] is not False:
        raise RuntimeError("official oracle CDP attach adapter requires handle_sigint=False")
    expected_channel = os.environ.get("PWRIGHT_OFFICIAL_BROWSER_CHANNEL")
    if "channel" in options and options["channel"] != expected_channel:
        raise RuntimeError(
            "official oracle CDP attach adapter requires the configured browser channel"
        )


def install_oracle_launch_adapter() -> None:
    global _installed
    launch_mode = os.environ.get("PWRIGHT_OFFICIAL_LAUNCH_MODE", "cdp-adapter")
    if launch_mode == "owned":
        if _installed:
            raise RuntimeError("oracle CDP launch adapter is already installed")
        if not callable(getattr(AsyncBrowserType, "launch", None)) or not callable(
            getattr(SyncBrowserType, "launch", None)
        ):
            raise RuntimeError("owned launch requires public oracle BrowserType.launch")
        return
    if launch_mode != "cdp-adapter":
        raise RuntimeError(f"unsupported official launch mode: {launch_mode!r}")
    if _installed:
        return
    endpoint = os.environ["PWRIGHT_CDP_ENDPOINT"]
    async_api = import_module("playwright.async_api")
    sync_api = import_module("playwright.sync_api")

    async_connect = cast(
        Callable[..., Awaitable[AsyncBrowser]], async_api.BrowserType.connect_over_cdp
    )
    sync_connect = cast(Callable[..., SyncBrowser], sync_api.BrowserType.connect_over_cdp)

    class AsyncLaunchedBrowser(AsyncBrowser):
        official_fixture_context_ids: set[int]

        @property
        def contexts(self) -> list[AsyncBrowserContext]:
            return [
                context
                for context in super().contexts
                if _context_identity(context) not in self.official_fixture_context_ids
            ]

    class SyncLaunchedBrowser(SyncBrowser):
        official_fixture_context_ids: set[int]

        @property
        def contexts(self) -> list[SyncBrowserContext]:
            return [
                context
                for context in super().contexts
                if _context_identity(context) not in self.official_fixture_context_ids
            ]

    # Playwright derives the public API name in errors from self.__class__.__name__.
    # The adapter-only context filter must not rename Browser.new_context.
    AsyncLaunchedBrowser.__name__ = AsyncBrowser.__name__
    SyncLaunchedBrowser.__name__ = SyncBrowser.__name__

    async def adapted_async_launch(
        self: AsyncBrowserType, *args: object, **options: object
    ) -> AsyncBrowser:
        if os.environ.get("PWRIGHT_OFFICIAL_LAUNCH_MODE", "cdp-adapter") != "cdp-adapter":
            raise RuntimeError("official oracle CDP launch adapter is disabled in owned mode")
        if args:
            raise RuntimeError("official oracle launch adapter accepts keyword options only")
        _validate_launch_options(options)
        if self.name != "chromium":
            raise RuntimeError(
                f"official oracle launch adapter cannot attach the {self.name} engine"
            )
        print(
            "[playwright-oracle] BrowserType.launch fixture adapter -> "
            f"connect_over_cdp({endpoint})",
            file=sys.stderr,
        )
        browser = await async_connect(self, endpoint)
        hidden = {_context_identity(context) for context in browser.contexts}
        browser.__class__ = AsyncLaunchedBrowser
        launched = cast(AsyncLaunchedBrowser, browser)
        launched.official_fixture_context_ids = hidden
        return launched

    def adapted_sync_launch(self: SyncBrowserType, *args: object, **options: object) -> SyncBrowser:
        if os.environ.get("PWRIGHT_OFFICIAL_LAUNCH_MODE", "cdp-adapter") != "cdp-adapter":
            raise RuntimeError("official oracle CDP launch adapter is disabled in owned mode")
        if args:
            raise RuntimeError("official oracle launch adapter accepts keyword options only")
        _validate_launch_options(options)
        if self.name != "chromium":
            raise RuntimeError(
                f"official oracle launch adapter cannot attach the {self.name} engine"
            )
        print(
            "[playwright-oracle] BrowserType.launch fixture adapter -> "
            f"connect_over_cdp({endpoint})",
            file=sys.stderr,
        )
        browser = sync_connect(self, endpoint)
        hidden = {_context_identity(context) for context in browser.contexts}
        browser.__class__ = SyncLaunchedBrowser
        launched = cast(SyncLaunchedBrowser, browser)
        launched.official_fixture_context_ids = hidden
        return launched

    async_api.BrowserType.launch = adapted_async_launch
    sync_api.BrowserType.launch = adapted_sync_launch
    _installed = True


def pytest_configure() -> None:
    install_oracle_launch_adapter()
