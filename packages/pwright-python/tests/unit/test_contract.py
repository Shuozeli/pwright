# pyright: reportPrivateUsage=false

from __future__ import annotations

import asyncio
import importlib.util
import os
import re
import threading
from collections.abc import Awaitable, Callable, Coroutine
from pathlib import Path
from typing import Any, Optional, Union, cast

import pwright
import pytest
from greenlet import greenlet
from pwright import _native
from pwright.async_api import (
    APIResponse as AsyncAPIResponse,
)
from pwright.async_api import (
    Browser,
    BrowserContext,
    BrowserType,
    ConsoleMessage,
    Dialog,
    Download,
    ElementHandle,
    FileChooser,
    FilePayload,
    Frame,
    FrameLocator,
    Locator,
    Page,
    Request,
    Response,
    Route,
    Selectors,
    WebError,
    Worker,
    _device_options,
    _drag_position,
    _drag_steps,
    _init_script_source,
    _read_selector_engine_source,
    _start_binding_awaitable,
    _string_argument,
    _wrap_handler,
    async_playwright,
)
from pwright.async_api import (
    JSHandle as AsyncJSHandle,
)
from pwright.async_api import (
    expect as async_expect,
)
from pwright.sync_api import APIResponse as SyncAPIResponse
from pwright.sync_api import Browser as SyncBrowser
from pwright.sync_api import BrowserContext as SyncBrowserContext
from pwright.sync_api import BrowserType as SyncBrowserType
from pwright.sync_api import ConsoleMessage as SyncConsoleMessage
from pwright.sync_api import Dialog as SyncDialog
from pwright.sync_api import Download as SyncDownload
from pwright.sync_api import ElementHandle as SyncElementHandle
from pwright.sync_api import Frame as SyncFrame
from pwright.sync_api import JSHandle as SyncJSHandle
from pwright.sync_api import Locator as SyncLocator
from pwright.sync_api import Page as SyncPage
from pwright.sync_api import Request as SyncRequest
from pwright.sync_api import Response as SyncResponse
from pwright.sync_api import Route as SyncRoute
from pwright.sync_api import WebError as SyncWebError
from pwright.sync_api import Worker as SyncWorker
from pwright.sync_api import _LoopThread, sync_playwright
from pwright.sync_api import expect as sync_expect


def test_source_abi_rejects_older_native_before_first_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    source = Path(__file__).resolve().parents[2] / "python/pwright/_abi.py"
    spec = importlib.util.spec_from_file_location("pwright._abi_old_native_probe", source)
    assert spec is not None and spec.loader is not None
    probe = importlib.util.module_from_spec(spec)
    declared_abi = re.search(r"^NATIVE_ABI_VERSION = (\d+)$", source.read_text(), re.MULTILINE)
    assert declared_abi is not None
    expected_abi = int(declared_abi.group(1))
    monkeypatch.setattr(_native, "native_abi_version", lambda: expected_abi - 1)

    # Act
    with pytest.raises(ImportError) as failure:
        spec.loader.exec_module(probe)

    # Assert
    assert str(failure.value) == (
        f"incompatible Pwright native extension: expected ABI {expected_abi}, "
        f"observed ABI {expected_abi - 1}"
    )


def test_frame_events_abi_is_131() -> None:
    # Arrange / Act
    declared = pwright.NATIVE_ABI_VERSION
    observed = _native.native_abi_version()

    # Assert
    assert declared == observed == 131


def test_remote_close_notice_method_matches_selected_feature() -> None:
    # Arrange
    features = set(_native.selected_features())
    remote_browser_type = getattr(_native, "NativeRemoteBrowser", None)

    # Act
    has_notice_method = remote_browser_type is not None and hasattr(
        remote_browser_type, "next_close_notice"
    )

    # Assert
    assert has_notice_method == ("playwright-server-close-notices" in features)
    if has_notice_method:
        assert "playwright-server-connect-session" in features
        assert hasattr(remote_browser_type, "wait_disconnected")


def test_native_abi_and_selected_profile_are_explicit() -> None:
    # Arrange
    expected_limits = {
        "sessions": 16,
        "connects": 16,
        "commands": 32,
        "closes": 16,
        "watches": 256,
        "request_history_items_per_page": 100,
        "request_history_owners": 4096,
        "request_history_owners_per_page": 256,
        "device_contexts": 16,
        "device_context_bytes": 64 * 1024,
        "locale_contexts": 16,
        "locale_bytes_per_context": 256,
        "timezone_contexts": 16,
        "timezone_bytes_per_context": 256,
        "offline_contexts": 16,
        "geolocation_contexts": 16,
        "geolocation_permission_contexts": 16,
        "http_credential_contexts": 16,
        "http_credential_field_bytes": 64 * 1024,
        "persistent_launch_arguments": 256,
        "persistent_launch_argument_bytes": 1024 * 1024,
        "ignore_https_error_contexts": 16,
        "bypass_csp_contexts": 16,
        "javascript_disabled_contexts": 16,
        "har_replay_archive_bytes": 8 * 1024 * 1024,
        "har_replay_archives": 8,
        "har_replay_retained_bytes": 8 * 1024 * 1024,
        "route_fulfill_body_bytes": 1024 * 1024,
        "route_fulfill_path_bytes": 4 * 1024,
        "route_initializing_sessions": 64,
        "context_cookie_count": 1024,
        "context_cookie_batch_bytes": 4 * 1024 * 1024,
        "context_cookie_urls": 256,
        "context_cookie_url_bytes": 64 * 1024,
        "context_storage_state_bytes": 8 * 1024 * 1024,
        "context_storage_origins": 128,
        "context_storage_items": 4096,
        "pdf_bytes": 2 * 1024 * 1024,
        "pdf_concurrent_captures": 2,
        "page_viewport_pages": 64,
        "page_viewport_bytes": 8 * 1024,
        "screenshot_bytes": 2 * 1024 * 1024,
        "screenshot_concurrent_captures": 2,
        "file_input_items": 64,
        "file_input_path_bytes": 4096,
        "file_input_name_bytes": 4096,
        "file_input_mime_bytes": 1024,
        "selector_engines": 64,
        "selector_engine_name_bytes": 64,
        "selector_engine_source_bytes": 1024 * 1024,
        "selector_registry_bytes": 8 * 1024 * 1024,
        "test_id_attribute_bytes": 1024,
        "layout_selector_parts": 256,
        "layout_selector_program_bytes": 256 * 1024,
        "request_size_owners": 64,
        "request_event_contexts": 16,
        "request_event_notices": 256,
        "request_event_bytes": 16 * 1024 * 1024,
        "response_event_contexts": 16,
        "response_event_notices": 256,
        "response_event_bytes": 16 * 1024 * 1024,
        "response_finished_owners": 256,
        "response_finished_bytes": 64 * 1024,
        "download_contexts": 16,
        "download_owners": 64,
        "download_notices": 64,
        "download_path_bytes": 4096,
        "worker_owners": 64,
        "worker_notices": 128,
        "worker_url_bytes": 1024 * 1024,
        "worker_concurrent_evaluations": 8,
        "worker_handles": 64,
        "worker_handle_concurrent_calls": 8,
        "worker_console_notices": 128,
        "worker_console_bytes": 16 * 1024 * 1024,
        "wait_for_timeout_operations": 16,
        "wait_for_timeout_bytes": 12 * 1024,
        "reserved_bytes": 64 * 1024 * 1024,
    }
    if "cdp-session" in _native.selected_features():
        expected_limits.update(
            {
                "cdp_sessions": 8,
                "cdp_session_events": 64,
                "cdp_session_event_bytes": 1024 * 1024,
            }
        )

    # Act
    capabilities = pwright.native_capabilities()

    # Assert
    assert capabilities["abi"] == pwright.NATIVE_ABI_VERSION
    expected_features = [
        "browser-registry",
        "fresh-launch",
        "persistent-context",
        "persistent-context-locale",
        "persistent-context-timezone",
        "persistent-context-offline",
        "persistent-context-geolocation",
        "persistent-context-geolocation-permission",
        "persistent-context-http-credentials",
        "persistent-context-launch-arguments",
        "persistent-context-ignore-https-errors",
        "persistent-context-device-emulation",
        "persistent-context-emulate-media",
        "persistent-context-extra-headers",
        "persistent-context-bypass-csp",
        "persistent-context-javascript-disabled",
        "context-cookies",
        "context-storage-state",
        "handles",
        "element-handle-dom",
        "element-handle-dom-reads",
        "element-handle-scroll-into-view",
        "element-handle-select-text",
        "element-handle-state-predicates",
        "element-handle-state-wait",
        "element-handle-file-input",
        "page-bindings",
        "context-bindings",
        "page-events",
        "console-events",
        "console-argument-handles",
        "page-dialogs",
        "file-chooser-events",
        "file-chooser-native-paths",
        "page-errors",
        "console-history",
        "request-history",
        "page-routes",
        "route-redirects",
        "route-fulfill-path",
        "route-fulfill-api-response",
        "route-fetch",
        "har-replay",
        "wait-for-function",
        "wait-for-timeout",
        "file-input",
        "file-input-native-paths",
        "mouse",
        "keyboard",
        "locator-bounding-box",
        "locator-highlight",
        "locator-screenshot",
        "locator-dom-actions",
        "locator-click-modifiers",
        "page-select-option",
        "extra-headers",
        "frame-lifecycle",
        "frame-navigation",
        "navigation-url-match",
        "history-navigation",
        "response-body",
        "wait-for-url",
        "set-content",
        "selector-registration",
        "layout-selectors",
        "emulate-media",
        "context-forced-colors",
        "device-emulation",
        "context-locale",
        "context-timezone",
        "context-offline",
        "context-geolocation",
        "context-geolocation-permission",
        "context-http-credentials",
        "ignore-https-errors",
        "bypass-csp",
        "javascript-disabled",
        "page-viewport",
        "timeout-settings",
        "page-pause",
        "request-gc",
        "page-pdf",
        "network-events",
        "context-network-events",
        "request-frame",
        "request-timing",
        "request-post-data-json",
        "request-events",
        "response-events",
        "response-finished",
        "navigation-response-finished",
        "response-security-details",
        "download-events",
        "worker-events",
        "worker-handle-evaluation",
        "worker-console-events",
        "worker-console-argument-handles",
        "worker-error-events",
        "worker-network-events",
        "init-scripts",
        "service-worker-policy",
        "api-request",
        "context-api-request",
        "api-request-client-certificates",
        "drag-and-drop",
    ]
    conformance_features = expected_features.copy()
    conformance_features.insert(
        conformance_features.index("element-handle-state-wait") + 1,
        "element-handle-click",
    )
    conformance_features.insert(
        conformance_features.index("element-handle-click") + 1,
        "element-handle-tap",
    )
    conformance_features.insert(
        conformance_features.index("device-emulation") + 1,
        "touchscreen-input",
    )
    cdp_conformance_features = conformance_features.copy()
    cdp_conformance_features.insert(
        cdp_conformance_features.index("context-storage-state") + 1,
        "cdp-session",
    )
    ws_conformance_features = cdp_conformance_features.copy()
    ws_conformance_features.insert(
        ws_conformance_features.index("cdp-session") + 1,
        "cdp-handshake-headers",
    )
    ws_conformance_features.insert(
        ws_conformance_features.index("cdp-handshake-headers") + 1,
        "cdp-ws-close-reason",
    )
    strict_ws_conformance_features = ws_conformance_features.copy()
    strict_ws_conformance_features.insert(
        strict_ws_conformance_features.index("cdp-ws-close-reason") + 1,
        "cdp-ws-invalid-frame",
    )
    frame_conformance_features = strict_ws_conformance_features.copy()
    frame_conformance_features.insert(
        frame_conformance_features.index("page-events") + 1,
        "frame-events",
    )
    server_initialize_conformance_features = strict_ws_conformance_features.copy()
    server_initialize_conformance_features.insert(
        server_initialize_conformance_features.index("fresh-launch") + 1,
        "playwright-server-initialize",
    )
    features = cast(list[str], capabilities["features"])
    selected_remote_features = [
        "browser-registry",
        "playwright-server-initialize",
        "playwright-server-connect-session",
    ]
    selected_remote_with_title = [*selected_remote_features, "playwright-server-frame-title"]
    selected_remote_with_notices = [
        *selected_remote_features,
        "playwright-server-close-notices",
    ]
    selected_remote_with_notices_and_title = [
        *selected_remote_with_notices,
        "playwright-server-frame-title",
    ]
    if features in (
        selected_remote_features,
        selected_remote_with_title,
        selected_remote_with_notices,
        selected_remote_with_notices_and_title,
    ):
        assert capabilities["profile"] == "+".join(features)
        limits = cast(dict[str, int], capabilities["limits"])
        assert {
            name: limits[name] for name in ("sessions", "connects", "commands", "closes", "watches")
        } == {
            "sessions": 16,
            "connects": 16,
            "commands": 32,
            "closes": 16,
            "watches": 16,
        }
        return
    assert features in (
        expected_features,
        conformance_features,
        cdp_conformance_features,
        ws_conformance_features,
        strict_ws_conformance_features,
        frame_conformance_features,
        server_initialize_conformance_features,
    )
    assert capabilities["profile"] == "+".join(features)
    assert capabilities["limits"] == expected_limits


def test_native_timeout_error_exposes_playwright_name_and_message() -> None:
    # Arrange
    error = pwright.TimeoutError("selector timed out")

    # Act
    name = error.name
    message = error.message

    # Assert
    assert name == "TimeoutError"
    assert message == "selector timed out"


def test_native_invalid_argument_is_a_playwright_error() -> None:
    # Arrange
    error = pwright.InvalidArgumentError("invalid cookie")

    # Act
    is_playwright_error = isinstance(error, pwright.Error)

    # Assert
    assert is_playwright_error


def test_public_object_representations_match_playwright_shape(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    class Native:
        version = "154.0"

    executable_path = "/opt/browser/chrome"

    def fake_executable_path(_engine: str, _package_root: str) -> str:
        return executable_path

    monkeypatch.setattr(
        "pwright.async_api._native.browser_executable_path",
        fake_executable_path,
    )
    browser_type = BrowserType(cast(Any, object()), "chromium")
    browser = object.__new__(Browser)
    browser._browser_type = browser_type
    browser._native = cast(Any, Native())
    context = BrowserContext(browser, "context-7", None)
    page = Page(context, "page-8", "frame-9", "https://example.test/")
    sync_browser_type = SyncBrowserType(cast(Any, object()), browser_type)
    sync_browser = SyncBrowser(cast(Any, object()), browser, sync_browser_type)
    sync_context = SyncBrowserContext(cast(Any, object()), context, sync_browser)
    sync_page = SyncPage(cast(Any, object()), page, sync_context)

    # Act
    representations = {
        "browser_type": repr(sync_browser_type),
        "browser": repr(sync_browser),
        "context": repr(sync_context),
        "page": repr(sync_page),
    }

    # Assert
    assert browser_type.executable_path == str(executable_path)
    assert representations == {
        "browser_type": f"<BrowserType name=chromium executable_path={executable_path}>",
        "browser": (
            f"<Browser type=<BrowserType name=chromium executable_path={executable_path}> "
            "version=154.0>"
        ),
        "context": (
            "<BrowserContext browser=<Browser type=<BrowserType name=chromium "
            f"executable_path={executable_path}> version=154.0>>"
        ),
        "page": "<Page url='https://example.test/'>",
    }


@pytest.mark.asyncio
async def test_page_evaluate_rewrites_native_error_with_playwright_api_name() -> None:
    # Arrange
    class Native:
        async def evaluate(self, frame_id: str, expression: str, argument: object) -> object:
            raise pwright.JavascriptError("ReferenceError: does_not_exist is not defined")

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    page = Page(cast(Any, ContextOwner(BrowserOwner(Native()))), "page-7", "frame-8", "about:blank")

    # Act
    with pytest.raises(pwright.JavascriptError) as raised:
        await page.evaluate("does_not_exist")

    # Assert
    assert str(raised.value) == ("Page.evaluate: ReferenceError: does_not_exist is not defined")
    assert raised.value.message == str(raised.value)
    assert raised.value.name == "JavascriptError"
    assert raised.value.code == "PWRIGHT_JAVASCRIPT"


@pytest.mark.asyncio
async def test_selectors_register_delegates_owned_source_to_native_rust() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[str, str, bool]] = None

        def register_selector_engine(self, name: str, source: str, content_script: bool) -> None:
            self.observed = (name, source, content_script)

    native = Native()
    selectors = Selectors(cast(Any, native))

    # Act
    await selectors.register("tag", "() => ({ query() {} })", content_script=True)

    # Assert
    assert native.observed == ("tag", "() => ({ query() {} })", True)


def test_selectors_test_id_attribute_delegates_to_native_rust() -> None:
    # Arrange
    class Native:
        observed: Optional[str] = None

        def set_test_id_attribute(self, name: str) -> None:
            self.observed = name

    native = Native()
    selectors = Selectors(cast(Any, native))

    # Act
    selectors.set_test_id_attribute("data-custom-id")

    # Assert
    assert native.observed == "data-custom-id"


def test_selector_source_reader_rejects_files_before_an_unbounded_copy(tmp_path: Path) -> None:
    # Arrange
    source_path = tmp_path / "selector.js"
    source_path.write_bytes(b"x" * (1024 * 1024 + 1))

    # Act
    with pytest.raises(
        pwright.ResourceExhaustedError,
        match="selector engine source exceeds 1048576 UTF-8 bytes",
    ):
        _read_selector_engine_source(source_path)

    # Assert
    assert source_path.stat().st_size == 1024 * 1024 + 1


@pytest.mark.asyncio
async def test_selectors_register_rejects_missing_or_empty_source() -> None:
    # Arrange
    class Native:
        def register_selector_engine(self, name: str, source: str, content_script: bool) -> None:
            raise AssertionError("native registration must not run without source")

    selectors = Selectors(cast(Any, Native()))

    # Act
    with pytest.raises(pwright.Error) as raised:
        await selectors.register("tag", "")

    # Assert
    assert str(raised.value) == "Either source or path should be specified"


@pytest.mark.asyncio
async def test_device_catalog_and_context_options_match_the_rust_owned_pixel_2_entry() -> None:
    # Arrange
    manager = async_playwright()
    playwright = await manager.start()

    # Act
    descriptor = playwright.devices["Pixel 2"]
    options = _device_options(
        user_agent=descriptor["user_agent"],
        viewport=descriptor["viewport"],
        screen=descriptor.get("screen"),
        device_scale_factor=descriptor["device_scale_factor"],
        is_mobile=descriptor["is_mobile"],
        has_touch=descriptor["has_touch"],
    )

    # Assert
    assert len(playwright.devices) == 143
    assert descriptor["default_browser_type"] == "chromium"
    assert playwright[descriptor["default_browser_type"]] is playwright.chromium
    assert options == (
        descriptor["user_agent"],
        (411, 731),
        None,
        2.625,
        True,
        True,
    )
    await playwright.stop()


@pytest.mark.asyncio
async def test_page_pause_delegates_only_the_page_identity_to_native_rust() -> None:
    # Arrange
    class Native:
        observed_page_id: Optional[str] = None

        async def page_pause(self, page_id: str) -> None:
            self.observed_page_id = page_id

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    await page.pause()

    # Assert
    assert native.observed_page_id == "page-7"


@pytest.mark.asyncio
async def test_page_wait_for_timeout_delegates_validated_milliseconds_to_native_rust() -> None:
    # Arrange
    class Native:
        observed_timeout_ms: Optional[int] = None

        async def wait_for_timeout(self, timeout_ms: int) -> None:
            self.observed_timeout_ms = timeout_ms

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    await page.wait_for_timeout(83.9)

    # Assert
    assert native.observed_timeout_ms == 83


@pytest.mark.asyncio
async def test_page_go_back_delegates_navigation_options_to_native_rust() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[str, str, int]] = None

        @staticmethod
        def page_default_timeout(_page_id: str, _navigation: bool) -> int:
            return 30_000

        async def go_back(self, page_id: str, wait_until: str, timeout_ms: int) -> None:
            self.observed = (page_id, wait_until, timeout_ms)

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    result = await page.go_back(timeout=83, wait_until="domcontentloaded")

    # Assert
    assert result is None
    assert native.observed == ("page-7", "domcontentloaded", 83)


@pytest.mark.asyncio
async def test_page_go_forward_updates_same_document_url_through_native_rust() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[str, str, int]] = None
        frame_url = "about:blank"

        @staticmethod
        def page_default_timeout(_page_id: str, _navigation: bool) -> int:
            return 30_000

        def frames(self, _page_id: str) -> list[tuple[str, None, str, str]]:
            return [("frame-8", None, "", self.frame_url)]

        async def go_forward(
            self, page_id: str, wait_until: str, timeout_ms: int
        ) -> dict[str, object]:
            self.observed = (page_id, wait_until, timeout_ms)
            self.frame_url = "https://example.test/first.html"
            return {"url": "https://example.test/first.html", "response": None}

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

        async def _flush_network_notices(self) -> None:
            return None

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")
    retained_main_frame = page.main_frame

    # Act
    result = await page.go_forward(timeout=83, wait_until="domcontentloaded")

    # Assert
    assert result is None
    assert native.observed == ("page-7", "domcontentloaded", 83)
    assert page.url == "https://example.test/first.html"
    assert retained_main_frame.url == "https://example.test/first.html"


@pytest.mark.asyncio
async def test_page_go_forward_returns_none_without_an_available_history_entry() -> None:
    # Arrange
    class Native:
        @staticmethod
        def page_default_timeout(_page_id: str, _navigation: bool) -> int:
            return 30_000

        async def go_forward(self, _page_id: str, _wait_until: str, _timeout_ms: int) -> None:
            return None

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")

    # Act
    result = await page.go_forward()

    # Assert
    assert result is None
    assert page.url == "about:blank"


def test_sync_page_go_forward_uses_the_loop_bridge() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[str, str, int]] = None

        @staticmethod
        def page_default_timeout(_page_id: str, _navigation: bool) -> int:
            return 30_000

        async def go_forward(self, page_id: str, wait_until: str, timeout_ms: int) -> None:
            self.observed = (page_id, wait_until, timeout_ms)

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    async_page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))

    # Act
    try:
        result = page.go_forward(timeout=83, wait_until="commit")
    finally:
        bridge.close()

    # Assert
    assert result is None
    assert native.observed == ("page-7", "commit", 83)


@pytest.mark.asyncio
async def test_page_wait_for_selector_uses_first_match_and_one_native_wait_owner() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []

    class NativeLocatorRecipe:
        def nth(self, index: int) -> "NativeLocatorRecipe":
            observed.append(("nth", index))
            return self

        async def wait_for_element(self, state: str, timeout_ms: int) -> Optional[object]:
            observed.append(("wait_for_element", state, timeout_ms))
            return None

    class Native:
        def locator(self, frame_id: str, selector: str) -> NativeLocatorRecipe:
            observed.append(("locator", frame_id, selector))
            return NativeLocatorRecipe()

        @staticmethod
        def page_default_timeout(_page_id: str, _navigation: bool) -> int:
            return 321

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")

    # Act
    value = await page.wait_for_selector("input", state="hidden")

    # Assert
    assert value is None
    assert observed == [
        ("locator", "frame-8", "input"),
        ("nth", 0),
        ("wait_for_element", "hidden", 321),
    ]


def test_sync_page_wait_for_selector_uses_the_same_native_wait_owner() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []

    class NativeLocatorRecipe:
        async def wait_for_element(self, state: str, timeout_ms: int) -> Optional[object]:
            observed.append(("wait_for_element", state, timeout_ms))
            return None

    class Native:
        def locator(self, frame_id: str, selector: str) -> NativeLocatorRecipe:
            observed.append(("locator", frame_id, selector))
            return NativeLocatorRecipe()

        @staticmethod
        def page_default_timeout(_page_id: str, _navigation: bool) -> int:
            return 321

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    async_page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))

    # Act
    try:
        value = page.wait_for_selector("input", state="detached", strict=True, timeout=89)
    finally:
        bridge.close()

    # Assert
    assert value is None
    assert observed == [
        ("locator", "frame-8", "input"),
        ("wait_for_element", "detached", 89),
    ]


def test_sync_page_wait_for_timeout_uses_the_loop_bridge() -> None:
    # Arrange
    class Native:
        observed_timeout_ms: Optional[int] = None

        async def wait_for_timeout(self, timeout_ms: int) -> None:
            self.observed_timeout_ms = timeout_ms

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    async_page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))

    # Act
    try:
        page.wait_for_timeout(125)
    finally:
        bridge.close()

    # Assert
    assert native.observed_timeout_ms == 125


@pytest.mark.asyncio
async def test_page_expect_navigation_arms_native_wait_before_action() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []

    class Native:
        def wait_for_navigation(
            self,
            page_id: str,
            frame_id: str,
            wait_until: str,
            timeout_ms: int,
        ) -> Awaitable[Optional[dict[str, object]]]:
            observed.append((page_id, frame_id, wait_until, timeout_ms))

            async def wait() -> Optional[dict[str, object]]:
                return None

            return wait()

        @staticmethod
        def pages(context_id: str) -> list[tuple[str, str, str, str]]:
            return [("page-7", context_id, "frame-8", "about:blank#clicked")]

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        _object_id = "context-6"

        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")
    navigation = page.expect_navigation(wait_until="commit", timeout=83.9)
    assert not navigation.is_done()

    # Act
    async with navigation:
        assert observed == [("page-7", "frame-8", "commit", 83)]
        assert not navigation.is_done()
    value = await navigation.value

    # Assert
    assert navigation.is_done()
    assert value is None
    assert page.url == "about:blank#clicked"


def _match_target_url(candidate: str) -> bool:
    return "target" in candidate


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "url",
    ["**/target.html", re.compile(r"target\.html"), _match_target_url],
)
async def test_page_expect_navigation_registers_url_match_before_action(
    url: Union[str, re.Pattern[str], Callable[[str], bool]],
) -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []

    class Native:
        def wait_for_navigation(self, *_args: object) -> Awaitable[None]:
            raise AssertionError("unfiltered navigation wait must not run")

        def wait_for_navigation_matching(
            self,
            page_id: str,
            frame_id: str,
            matcher: object,
            wait_until: str,
            timeout_ms: int,
        ) -> Awaitable[None]:
            observed.append((page_id, frame_id, matcher, wait_until, timeout_ms))

            async def wait() -> None:
                return None

            return wait()

        @staticmethod
        def pages(context_id: str) -> list[tuple[str, str, str, str]]:
            return [("page-7", context_id, "frame-8", "about:blank#done")]

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        _object_id = "context-6"

        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    async with page.expect_navigation(url=url, timeout=50):
        assert observed == [("page-7", "frame-8", url, "load", 50)]

    # Assert
    assert page.url == "about:blank#done"


@pytest.mark.asyncio
async def test_page_expect_navigation_propagates_url_predicate_exception() -> None:
    # Arrange
    observed: list[str] = []

    def matcher(candidate: str) -> bool:
        observed.append(candidate)
        raise ValueError("predicate failed")

    class Native:
        def wait_for_navigation_matching(
            self,
            _page_id: str,
            _frame_id: str,
            predicate: Callable[[str], bool],
            _wait_until: str,
            _timeout_ms: int,
        ) -> Awaitable[None]:
            async def wait() -> None:
                predicate("https://test/target.html")

            return wait()

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")

    # Act
    with pytest.raises(ValueError, match="predicate failed"):
        async with page.expect_navigation(url=matcher, timeout=50):
            pass

    # Assert
    assert observed == ["https://test/target.html"]


def test_sync_page_expect_navigation_uses_the_same_native_wait_owner() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []

    class Native:
        def wait_for_navigation(
            self,
            page_id: str,
            frame_id: str,
            wait_until: str,
            timeout_ms: int,
        ) -> Awaitable[Optional[dict[str, object]]]:
            observed.append((page_id, frame_id, wait_until, timeout_ms))

            async def wait() -> Optional[dict[str, object]]:
                return None

            return wait()

        @staticmethod
        def pages(context_id: str) -> list[tuple[str, str, str, str]]:
            return [("page-7", context_id, "frame-8", "about:blank#sync")]

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        _object_id = "context-6"

        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    async_page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, object()))

    # Act
    try:
        with page.expect_navigation(wait_until="domcontentloaded", timeout=97) as navigation:
            pass
        value = navigation.value
    finally:
        bridge.close()

    # Assert
    assert value is None
    assert observed == [("page-7", "frame-8", "domcontentloaded", 97)]
    assert page.url == "about:blank#sync"


class _FrameNavigationNative:
    def __init__(self) -> None:
        self.goto_calls: list[tuple[object, ...]] = []
        self.wait_calls: list[tuple[object, ...]] = []
        self.frame_url = "about:blank"
        self.body_calls: list[tuple[str, str, str]] = []
        self.request_lookup_calls: list[tuple[str, str]] = []

    def pages(self, context_id: str) -> list[tuple[str, str, str, str]]:
        return [("page-7", context_id, "main-8", "https://example.test/")]

    def frames(self, _page_id: str) -> list[tuple[str, Optional[str], str, str]]:
        return [
            ("main-8", None, "", "https://example.test/"),
            ("child-9", "main-8", "child", self.frame_url),
        ]

    def page_requests(self, _page_id: str) -> list[dict[str, object]]:
        raise AssertionError("navigation must not materialize full request history")

    def page_default_timeout(self, _page_id: str, _navigation: bool) -> int:
        return 30_000

    async def request_by_native_id(self, page_id: str, request_id: str) -> dict[str, object]:
        self.request_lookup_calls.append((page_id, request_id))
        return {
            "id": request_id,
            "native_request_id": request_id,
            "url": "https://example.test/next",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
        }

    async def frame_response_body(self, page_id: str, frame_id: str, request_id: str) -> bytes:
        self.body_calls.append((page_id, frame_id, request_id))
        return b"child response"

    async def frame_goto(
        self,
        page_id: str,
        frame_id: str,
        url: str,
        wait_until: str,
        referrer: Optional[str],
        timeout_ms: int,
    ) -> dict[str, object]:
        self.goto_calls.append((page_id, frame_id, url, wait_until, referrer, timeout_ms))
        self.frame_url = url
        return {
            "request_id": "request-10",
            "url": url,
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }

    def wait_for_navigation(
        self, page_id: str, frame_id: str, wait_until: str, timeout_ms: int
    ) -> Awaitable[dict[str, object]]:
        self.wait_calls.append((page_id, frame_id, wait_until, timeout_ms))

        async def wait() -> dict[str, object]:
            self.frame_url = "https://example.test/next"
            return {
                "request_id": "request-11",
                "url": "https://example.test/next",
                "status": 200,
                "status_text": "OK",
                "ok": True,
                "headers": {},
            }

        return wait()


def _frame_navigation_fixture() -> tuple[_FrameNavigationNative, Frame]:
    native = _FrameNavigationNative()

    class Browser:
        def __init__(self, native: _FrameNavigationNative) -> None:
            self._native = native

        def _raise_deferred_route_error(self) -> None:
            return None

        async def _flush_network_notices(self) -> None:
            return None

    class Context:
        _object_id = "context-6"

        def __init__(self, browser: Browser) -> None:
            self.browser = browser

        async def _drain_cdp_sessions(self, _page_id: str) -> None:
            return None

    page = Page(cast(Any, Context(Browser(native))), "page-7", "main-8", "about:blank")
    frame = Frame(page, "child-9", "main-8", "child", "about:blank")
    page._frames[frame._object_id] = frame
    return native, frame


@pytest.mark.asyncio
async def test_frame_goto_delegates_target_options_and_response_owner() -> None:
    # Arrange
    native, frame = _frame_navigation_fixture()

    # Act
    response = await frame.goto(
        "https://example.test/next",
        wait_until="domcontentloaded",
        referer="https://example.test/from",
        timeout=87.9,
    )

    # Assert
    assert native.goto_calls == [
        (
            "page-7",
            "child-9",
            "https://example.test/next",
            "domcontentloaded",
            "https://example.test/from",
            87,
        )
    ]
    assert response is not None
    assert response.ok
    assert response.frame is frame
    assert frame.url == "https://example.test/next"
    assert await response.body() == b"child response"
    assert native.body_calls == [("page-7", "child-9", "request-10")]
    assert response.request.url == "https://example.test/next"
    assert native.request_lookup_calls == [("page-7", "request-10")]


@pytest.mark.asyncio
async def test_frame_expect_navigation_arms_child_wait_before_action() -> None:
    # Arrange
    native, frame = _frame_navigation_fixture()
    navigation = frame.expect_navigation(wait_until="commit", timeout=91.9)

    # Act
    async with navigation:
        assert native.wait_calls == [("page-7", "child-9", "commit", 91)]
    response = await navigation.value

    # Assert
    assert response is not None
    assert response.url == "https://example.test/next"
    assert response.frame is frame
    assert frame.url == "https://example.test/next"
    assert await response.body() == b"child response"
    assert native.body_calls == [("page-7", "child-9", "request-11")]
    assert response.request.url == "https://example.test/next"
    assert native.request_lookup_calls == [("page-7", "request-11")]


def test_frame_expect_navigation_url_reports_missing_matcher_capability() -> None:
    # Arrange
    native, frame = _frame_navigation_fixture()

    # Act
    with pytest.raises(pwright.UnsupportedError, match=r"Frame\.expect_navigation\(url=\.\.\.\)"):
        frame.expect_navigation(url="**/next")

    # Assert
    assert native.wait_calls == []


@pytest.mark.asyncio
async def test_frame_expect_navigation_preserves_body_error_after_wait_failure(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native, frame = _frame_navigation_fixture()

    async def failed_wait() -> dict[str, object]:
        raise ValueError("navigation failed")

    def arm_failed_wait(*_args: object) -> Awaitable[dict[str, object]]:
        return failed_wait()

    monkeypatch.setattr(native, "wait_for_navigation", arm_failed_wait)

    # Act
    with pytest.raises(RuntimeError, match="body failed"):
        async with frame.expect_navigation():
            await asyncio.sleep(0)
            raise RuntimeError("body failed")


@pytest.mark.asyncio
async def test_frame_expect_navigation_normalizes_detached_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native, frame = _frame_navigation_fixture()

    async def detached_wait() -> dict[str, object]:
        raise pwright.TargetClosedError("Frame was detached")

    def arm_detached(*_args: object) -> Awaitable[dict[str, object]]:
        return detached_wait()

    monkeypatch.setattr(native, "wait_for_navigation", arm_detached)

    # Act
    with pytest.raises(pwright.TargetClosedError) as failure:
        async with frame.expect_navigation(timeout=20):
            pass

    # Assert
    assert failure.value.message == "frame was detached"


@pytest.mark.asyncio
async def test_frame_goto_consumes_deferred_route_error_before_native_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    native, frame = _frame_navigation_fixture()

    def raise_deferred() -> None:
        raise ValueError("route callback failed")

    monkeypatch.setattr(frame.page._context.browser, "_raise_deferred_route_error", raise_deferred)

    # Act
    with pytest.raises(ValueError, match="route callback failed"):
        await frame.goto("https://example.test/next")

    # Assert
    assert native.goto_calls == []


def test_sync_frame_goto_preserves_its_frame_response_identity() -> None:
    # Arrange
    native, async_frame = _frame_navigation_fixture()
    bridge = _LoopThread()

    class Context:
        page: SyncPage

        def _wrap_response(self, value: object) -> SyncResponse:
            return SyncResponse(bridge, cast(Any, value), cast(Any, self))

        def _wrap_page(self, _value: Page) -> SyncPage:
            return self.page

    context = Context()
    page = SyncPage(bridge, async_frame.page, cast(Any, context))
    context.page = page
    frame = SyncFrame(bridge, async_frame, page)
    page._frames[id(async_frame)] = frame

    # Act
    try:
        response = frame.goto("https://example.test/next", timeout=42)
    finally:
        bridge.close()

    # Assert
    assert native.goto_calls == [
        ("page-7", "child-9", "https://example.test/next", "load", None, 42)
    ]
    assert response is not None
    assert response.frame is frame


def _page_goto_error_fixture(error: Exception) -> Page:
    class Native:
        async def goto(self, *_args: object) -> dict[str, object]:
            raise error

    class Browser:
        _native = Native()

        def _raise_deferred_route_error(self) -> None:
            return None

    class Context:
        browser = Browser()

    return Page(cast(Any, Context()), "page-7", "main-8", "about:blank")


@pytest.mark.asyncio
async def test_page_goto_timeout_keeps_type_and_adds_navigation_call_log() -> None:
    # Arrange
    page = _page_goto_error_fixture(pwright.TimeoutError("Timeout 5ms exceeded."))

    # Act
    with pytest.raises(pwright.TimeoutError) as failure:
        await page.goto("https://example.test/slow", wait_until="networkidle", timeout=5)

    # Assert
    assert "Timeout 5ms exceeded." in failure.value.message
    assert 'navigating to "https://example.test/slow", waiting until "networkidle"' in (
        failure.value.message
    )
    assert failure.value.stack


@pytest.mark.asyncio
async def test_page_goto_invalid_wait_option_uses_python_argument_name() -> None:
    # Arrange
    page = _page_goto_error_fixture(
        pwright.InvalidArgumentError("waitUntil: expected one of (load|domcontentloaded)")
    )

    # Act
    with pytest.raises(pwright.InvalidArgumentError) as failure:
        await page.goto("https://example.test/", wait_until="networkidle2", timeout=5)

    # Assert
    assert "wait_until: expected one of (load|domcontentloaded)" in str(failure.value)
    assert "Call log:" not in str(failure.value)


@pytest.mark.asyncio
async def test_page_goto_referer_conflict_includes_target_url() -> None:
    # Arrange
    page = _page_goto_error_fixture(
        pwright.InvalidArgumentError('"referer" is already specified as extra HTTP header')
    )

    # Act
    with pytest.raises(pwright.InvalidArgumentError) as failure:
        await page.goto("https://example.test/", referer="https://other.test/", timeout=5)

    # Assert
    assert '"referer" is already specified as extra HTTP header' in str(failure.value)
    assert "https://example.test/" in str(failure.value)


@pytest.mark.asyncio
async def test_page_wait_for_load_state_delegates_to_native_frame_lifecycle() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []

    class Native:
        async def wait_for_load_state(
            self,
            page_id: str,
            frame_id: str,
            wait_until: str,
            timeout_ms: int,
        ) -> None:
            observed.append((page_id, frame_id, wait_until, timeout_ms))

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")

    # Act
    await page.wait_for_load_state("domcontentloaded", timeout=83.9)

    # Assert
    assert observed == [("page-7", "frame-8", "domcontentloaded", 83)]


@pytest.mark.asyncio
async def test_page_wait_for_load_state_invalid_value_uses_python_argument_name() -> None:
    # Arrange
    class Native:
        async def wait_for_load_state(self, *_args: object) -> None:
            raise pwright.InvalidArgumentError(
                "wait_until must be commit, domcontentloaded, load, or networkidle"
            )

    class Browser:
        _native = Native()

    class Context:
        browser = Browser()

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")

    # Act
    with pytest.raises(pwright.InvalidArgumentError) as failure:
        await page.wait_for_load_state("bad", timeout=20)

    # Assert
    assert "state: expected one of (load|domcontentloaded|networkidle|commit)" in str(failure.value)


def test_sync_page_wait_for_load_state_uses_the_loop_bridge() -> None:
    # Arrange
    observed: list[tuple[object, ...]] = []

    class Native:
        async def wait_for_load_state(
            self,
            page_id: str,
            frame_id: str,
            wait_until: str,
            timeout_ms: int,
        ) -> None:
            observed.append((page_id, frame_id, wait_until, timeout_ms))

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    async_page = Page(cast(Any, Context(Browser(Native()))), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))

    # Act
    try:
        page.wait_for_load_state("commit", timeout=97.6)
    finally:
        bridge.close()

    # Assert
    assert observed == [("page-7", "frame-8", "commit", 97)]


@pytest.mark.asyncio
async def test_page_request_gc_delegates_only_the_page_identity_to_native_rust() -> None:
    # Arrange
    class Native:
        observed_page_id: Optional[str] = None

        async def page_request_gc(self, page_id: str) -> None:
            self.observed_page_id = page_id

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    await page.request_gc()

    # Assert
    assert native.observed_page_id == "page-7"


@pytest.mark.asyncio
async def test_page_pdf_delegates_identity_and_normalized_path_to_native_rust(
    tmp_path: Path,
) -> None:
    # Arrange
    class Native:
        observed: object = None

        async def page_pdf(
            self, page_id: str, path: Optional[str], tagged: bool, outline: bool
        ) -> bytes:
            self.observed = (page_id, path, tagged, outline)
            return b"%PDF-1.7\n"

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")
    output = tmp_path / "output.pdf"

    # Act
    pdf = await page.pdf(path=output, tagged=True, outline=True)

    # Assert
    assert pdf == b"%PDF-1.7\n"
    assert native.observed == ("page-7", os.fspath(output), True, True)


@pytest.mark.asyncio
async def test_page_pdf_rejects_uninstalled_options_before_native_work() -> None:
    # Arrange
    class Native:
        called = False

        async def page_pdf(
            self, _page_id: str, _path: Optional[str], _tagged: bool, _outline: bool
        ) -> bytes:
            self.called = True
            return b""

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    with pytest.raises(
        pwright.UnsupportedError, match="Page.pdf options are not installed: landscape"
    ):
        await page.pdf(landscape=True)

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_page_pdf_rejects_non_boolean_outline_before_native_work() -> None:
    # Arrange
    class Native:
        called = False

        async def page_pdf(
            self, _page_id: str, _path: Optional[str], _tagged: bool, _outline: bool
        ) -> bytes:
            self.called = True
            return b""

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    with pytest.raises(TypeError, match="Page.pdf outline must be a boolean"):
        await page.pdf(outline=cast(Any, "yes"))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_page_eval_on_selector_all_delegates_collection_evaluation_to_rust() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[str, str, str, object]] = None

        async def eval_on_selector_all(
            self, frame_id: str, selector: str, expression: str, argument: object
        ) -> list[str]:
            self.observed = (frame_id, selector, expression, argument)
            return ["hello"]

        @staticmethod
        def pages(context_id: str) -> list[tuple[str, str, str, str]]:
            assert context_id == "context-6"
            return [("page-7", context_id, "frame-8", "about:blank")]

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        _object_id = "context-6"

        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    result = await page.eval_on_selector_all(
        'div >> internal:and=".foo"',
        "elements => elements.map(element => element.textContent)",
        {"suffix": "!"},
    )

    # Assert
    assert result == ["hello"]
    assert native.observed == (
        "frame-8",
        'div >> internal:and=".foo"',
        "elements => elements.map(element => element.textContent)",
        {"suffix": "!"},
    )


@pytest.mark.asyncio
async def test_registered_query_selector_uses_rust_world_result_for_no_match() -> None:
    # Arrange
    class Native:
        validated: Optional[tuple[str, str]] = None
        counted: Optional[tuple[str, str]] = None

        def validate_selector(self, frame_id: str, selector: str) -> None:
            self.validated = (frame_id, selector)

        async def registered_selector_match_count(self, frame_id: str, selector: str) -> int:
            self.counted = (frame_id, selector)
            return 0

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    result = await page.query_selector("isolated=ignored")

    # Assert
    assert result is None
    assert native.validated == ("frame-8", "isolated=ignored")
    assert native.counted == ("frame-8", "isolated=ignored")


@pytest.mark.asyncio
async def test_page_query_selector_uses_typed_locator_count_for_no_match() -> None:
    # Arrange
    class Native:
        validated: Optional[tuple[str, str]] = None

        def validate_selector(self, frame_id: str, selector: str) -> None:
            self.validated = (frame_id, selector)

        @staticmethod
        async def registered_selector_match_count(_frame_id: str, _selector: str) -> None:
            return None

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    class EmptyLocator:
        @staticmethod
        async def count() -> int:
            return 0

    def locator(_selector: str) -> EmptyLocator:
        return EmptyLocator()

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")
    cast(Any, page).locator = locator

    # Act
    result = await page.query_selector("text=missing")

    # Assert
    assert result is None
    assert native.validated == ("frame-8", "text=missing")


@pytest.mark.asyncio
async def test_page_query_selector_all_uses_the_bounded_locator_handle_collection() -> None:
    # Arrange
    expected = [cast(ElementHandle, object()), cast(ElementHandle, object())]
    observed: list[str] = []

    class HandleLocator:
        async def element_handles(self) -> list[ElementHandle]:
            return expected

    class Browser:
        def __init__(self, native: object) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    def locator(selector: str) -> HandleLocator:
        observed.append(selector)
        return HandleLocator()

    page = Page(cast(Any, Context(Browser(object()))), "page-7", "frame-8", "about:blank")
    cast(Any, page).locator = locator

    # Act
    handles = await page.query_selector_all("text=ready")

    # Assert
    assert handles is expected
    assert observed == ["text=ready"]


@pytest.mark.asyncio
async def test_locator_facade_forwards_collection_action_and_evaluation_to_page_owner() -> None:
    # Arrange
    class PageOwner:
        observed: list[tuple[object, ...]]

        def __init__(self) -> None:
            self.observed = []

        async def _locator_read(
            self,
            selector: str,
            operation: str,
            *,
            timeout: Optional[float] = None,
            attribute: Optional[str] = None,
            strict: Optional[bool] = None,
        ) -> object:
            self.observed.append(("read", selector, operation, timeout, attribute, strict))
            return ["A", "B"]

        async def set_checked(
            self,
            selector: str,
            checked: bool,
            *,
            force: Optional[bool] = None,
            position: object = None,
            timeout: Optional[float] = None,
            trial: Optional[bool] = None,
            no_wait_after: Optional[bool] = None,
        ) -> None:
            self.observed.append(
                (
                    "set_checked",
                    selector,
                    checked,
                    force,
                    position,
                    timeout,
                    trial,
                    no_wait_after,
                )
            )

        async def eval_on_selector(
            self,
            selector: str,
            expression: str,
            arg: object = None,
            *,
            timeout: Optional[float] = None,
        ) -> object:
            self.observed.append(("evaluate", selector, expression, arg, timeout))
            return True

    page = PageOwner()
    locator = Locator(cast(Any, page), "input")

    # Act
    texts = await locator.all_text_contents()
    await locator.check(force=True, timeout=75)
    active = await locator.evaluate("element => element === document.activeElement", timeout=25)

    # Assert
    assert texts == ["A", "B"]
    assert active is True
    expected_strict = True if "context-strict-selectors" in _native.selected_features() else None
    assert page.observed == [
        ("read", "input", "allTextContents", None, None, expected_strict),
        ("set_checked", "input", True, True, None, 75, None, None),
        ("evaluate", "input", "element => element === document.activeElement", None, 25),
    ]


@pytest.mark.asyncio
async def test_async_locator_count_expectation_delegates_polling_to_rust() -> None:
    # Arrange
    class PageOwner:
        observed: object = None

        async def _locator_expect_count(
            self, selector: str, expected: int, timeout_ms: int, is_not: bool
        ) -> dict[str, object]:
            self.observed = (selector, expected, timeout_ms, is_not)
            return {"matches": True, "received": expected, "timed_out": False}

    page = PageOwner()
    locator = Locator(cast(Any, page), ".item")

    # Act
    await async_expect(locator).to_have_count(2, timeout=73)

    # Assert
    assert page.observed == (".item", 2, 73, False)


def test_sync_locator_count_expectation_delegates_polling_to_rust() -> None:
    # Arrange
    class PageOwner:
        observed: object = None

        async def _locator_expect_count(
            self, selector: str, expected: int, timeout_ms: int, is_not: bool
        ) -> dict[str, object]:
            self.observed = (selector, expected, timeout_ms, is_not)
            return {"matches": False, "received": 1, "timed_out": False}

    page = PageOwner()
    async_locator = Locator(cast(Any, page), ".item")
    bridge = _LoopThread()
    locator = SyncLocator(bridge, async_locator)

    # Act
    try:
        sync_expect(locator).not_to.to_have_count(2, timeout=74)
    finally:
        bridge.close()

    # Assert
    assert page.observed == (".item", 2, 74, True)


@pytest.mark.asyncio
async def test_async_locator_visibility_expectation_delegates_polling_to_rust() -> None:
    # Arrange
    class NativeLocatorRecipe:
        observed: object = None

        async def expect_visibility(
            self, visible: bool, timeout_ms: int, is_not: bool
        ) -> dict[str, object]:
            self.observed = (visible, timeout_ms, is_not)
            return {"matches": True, "received": True, "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_be_visible(timeout=76)

    # Assert
    assert native.observed == (True, 76, False)


def test_sync_locator_visibility_expectation_adapts_the_same_native_recipe() -> None:
    # Arrange
    class NativeLocatorRecipe:
        observed: object = None

        async def expect_visibility(
            self, visible: bool, timeout_ms: int, is_not: bool
        ) -> dict[str, object]:
            self.observed = (visible, timeout_ms, is_not)
            return {"matches": True, "received": True, "timed_out": False}

    native = NativeLocatorRecipe()
    async_locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))
    bridge = _LoopThread()
    locator = SyncLocator(bridge, async_locator)

    # Act
    try:
        sync_expect(locator).to_be_visible(timeout=77)
    finally:
        bridge.close()

    # Assert
    assert native.observed == (True, 77, False)


@pytest.mark.asyncio
async def test_binding_awaitable_starts_before_its_publication_barrier_opens() -> None:
    # Arrange
    callback_started = False
    release = asyncio.Event()

    async def callback() -> object:
        nonlocal callback_started
        callback_started = True
        await release.wait()
        return "done"

    # Act
    task = await _start_binding_awaitable(callback())

    # Assert
    assert callback_started
    assert not task.done()
    release.set()
    assert await task == "done"


def test_public_handler_wrapper_drops_arguments_beyond_callback_signature() -> None:
    # Arrange
    observed = False

    def callback() -> None:
        nonlocal observed
        observed = True

    wrapped = _wrap_handler(callback)

    # Act
    wrapped({"isTrusted": True})

    # Assert
    assert observed


def test_public_handler_wrapper_preserves_declared_argument_order() -> None:
    # Arrange
    observed: Optional[tuple[object, object]] = None

    def callback(first: object, second: object) -> None:
        nonlocal observed
        observed = (first, second)

    wrapped = _wrap_handler(callback)

    # Act
    wrapped("source", "value", "ignored")

    # Assert
    assert observed == ("source", "value")


@pytest.mark.asyncio
async def test_browser_new_context_forwards_typed_context_policies_to_native_rust() -> None:
    # Arrange
    class Native:
        observed: Optional[
            tuple[
                Optional[str],
                object,
                Optional[str],
                Optional[str],
                Optional[tuple[float, float, float]],
                bool,
                Optional[tuple[str, str, Optional[str], bool]],
                bool,
                bool,
                bool,
                bool,
                bool,
                bool,
            ]
        ] = None

        async def new_context(
            self,
            base_url: Optional[str],
            device: object,
            locale: Optional[str],
            timezone_id: Optional[str],
            geolocation: Optional[tuple[float, float, float]],
            geolocation_permission: bool,
            http_credentials: Optional[tuple[str, str, Optional[str], bool]],
            ignore_https_errors: bool,
            offline: bool,
            bypass_csp: bool,
            javascript_enabled: bool,
            accept_downloads: bool,
            block_service_workers: bool,
        ) -> tuple[str, Optional[str]]:
            self.observed = (
                base_url,
                device,
                locale,
                timezone_id,
                geolocation,
                geolocation_permission,
                http_credentials,
                ignore_https_errors,
                offline,
                bypass_csp,
                javascript_enabled,
                accept_downloads,
                block_service_workers,
            )
            return ("context-7", base_url)

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    context = await browser.new_context(
        base_url="https://example.test",
        locale="ru-RU",
        timezone_id="America/Jamaica",
        geolocation={"longitude": -70, "latitude": 30, "accuracy": 7},
        permissions=["geolocation"],
        http_credentials={
            "username": "user",
            "password": "pass",
            "origin": "https://example.test",
        },
        ignore_https_errors=True,
        offline=True,
        bypass_csp=True,
        java_script_enabled=False,
        accept_downloads=True,
        service_workers="block",
    )

    # Assert
    assert native.observed == (
        "https://example.test",
        None,
        "ru-RU",
        "America/Jamaica",
        (-70.0, 30.0, 7.0),
        True,
        ("user", "pass", "https://example.test", False),
        True,
        True,
        True,
        False,
        True,
        True,
    )
    assert context._object_id == "context-7"


@pytest.mark.asyncio
async def test_browser_new_context_forwards_filtered_extra_headers_with_other_options() -> None:
    # Arrange
    class Native:
        options: tuple[object, ...] = ()
        keywords: dict[str, object] = {}

        async def new_context(
            self, *options: object, **keywords: object
        ) -> tuple[str, Optional[str]]:
            self.options = options
            self.keywords = keywords
            return ("context-with-headers", cast(Optional[str], options[0]))

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    context = await browser.new_context(
        base_url="https://example.test",
        viewport={"width": 400, "height": 500},
        user_agent="header-agent",
        locale="en-US",
        offline=True,
        extra_http_headers={"foo": "bar", "omit": None, "empty": ""},
    )

    # Assert
    assert native.options[0] == "https://example.test"
    assert native.options[1] == ("header-agent", (400, 500), None, None, None, None)
    assert native.options[2] == "en-US"
    assert native.options[8] is True
    assert native.keywords == {
        "forced_colors": None,
        "strict_selectors": False,
        "extra_http_headers": [("foo", "bar"), ("empty", "")],
    }
    assert context._object_id == "context-with-headers"


@pytest.mark.asyncio
async def test_browser_new_context_rejects_invalid_extra_headers_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object, **_kwargs: object) -> None:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(
        TypeError, match="Browser.new_context extra_http_headers must be a mapping of strings"
    ):
        await browser.new_context(extra_http_headers=cast(Any, [("foo", "bar")]))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_extra_headers_count_only_includes_serialized_entries() -> None:
    # Arrange
    class Native:
        headers: object = None

        async def new_context(
            self, *_args: object, extra_http_headers: object = None, **_kwargs: object
        ) -> tuple[str, Optional[str]]:
            self.headers = extra_http_headers
            return ("context-at-header-limit", None)

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}
    headers: dict[str, Optional[str]] = {f"x-{index}": "value" for index in range(256)}
    headers["omitted"] = None

    # Act
    context = await browser.new_context(extra_http_headers=headers)

    # Assert
    assert context._object_id == "context-at-header-limit"
    assert native.headers == [(f"x-{index}", "value") for index in range(256)]


@pytest.mark.asyncio
async def test_browser_new_context_extra_headers_rejects_more_than_256_retained_entries() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object, **_kwargs: object) -> None:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}
    headers: dict[str, Optional[str]] = {f"x-{index}": "value" for index in range(257)}
    headers["omitted"] = None

    # Act
    with pytest.raises(
        pwright.ResourceExhaustedError,
        match="Browser.new_context extra header count exceeds 256",
    ):
        await browser.new_context(extra_http_headers=headers)

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_extra_headers_rejects_excess_bytes_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object, **_kwargs: object) -> None:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(
        pwright.ResourceExhaustedError,
        match="Browser.new_context extra header bytes exceed 65536",
    ):
        await browser.new_context(extra_http_headers={"foo": "x" * 65534})

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_extra_headers_require_selected_profile(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object, **_kwargs: object) -> None:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    def no_features() -> list[str]:
        return []

    monkeypatch.setattr(_native, "selected_features", no_features)

    # Act
    with pytest.raises(
        pwright.UnsupportedError,
        match="Browser.new_context extra_http_headers requires the extra-headers profile",
    ):
        await browser.new_context(extra_http_headers={"foo": "bar"})

    # Assert
    assert not native.called


def test_sync_browser_new_context_forwards_extra_headers_to_async_browser() -> None:
    # Arrange
    class Native:
        headers: object = None

        async def new_context(
            self, *_args: object, extra_http_headers: object = None, **_kwargs: object
        ) -> tuple[str, Optional[str]]:
            self.headers = extra_http_headers
            return ("sync-context-with-headers", None)

    native = Native()
    async_browser = cast(Any, object.__new__(Browser))
    async_browser._native = native
    async_browser._contexts = {}
    bridge = _LoopThread()
    browser = SyncBrowser(bridge, async_browser, cast(Any, None))

    # Act
    try:
        context = browser.new_context(extra_http_headers={"foo": "bar", "omit": None})
    finally:
        bridge.close()

    # Assert
    assert native.headers == [("foo", "bar")]
    assert context._value._object_id == "sync-context-with-headers"


@pytest.mark.asyncio
async def test_browser_new_context_rejects_incomplete_geolocation_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object) -> tuple[str, Optional[str]]:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(TypeError, match="Browser.new_context geolocation latitude must be numeric"):
        await browser.new_context(geolocation={"longitude": 10})

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_rejects_incomplete_http_credentials_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object) -> tuple[str, Optional[str]]:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(
        TypeError,
        match="Browser.new_context http_credentials username and password must be strings",
    ):
        await browser.new_context(http_credentials={"username": "user"})

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_persistent_context_forwards_context_defaults_before_native_launch(
    tmp_path: Path,
) -> None:
    # Arrange
    class NativeLaunchObserved(Exception):
        pass

    class Environment:
        observed: Optional[tuple[str, bool, tuple[object, ...]]] = None

        async def launch_persistent_context(
            self,
            user_data_dir: str,
            headless: bool,
            options: tuple[object, ...],
        ) -> object:
            self.observed = (user_data_dir, headless, options)
            raise NativeLaunchObserved

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(NativeLaunchObserved):
        await browser_type.launch_persistent_context(
            tmp_path,
            headless=False,
            base_url="https://example.test/root/",
            extra_http_headers={"foo": "bar", "x-sequence": "two"},
            locale="fr-FR",
            timezone_id="America/Jamaica",
            geolocation={"longitude": -70, "latitude": 30, "accuracy": 7},
            permissions=["geolocation"],
            http_credentials={"username": "user", "password": "pass"},
            ignore_https_errors=True,
            offline=True,
            bypass_csp=True,
            java_script_enabled=False,
            viewport={"width": 456, "height": 789},
            screen={"width": 1080, "height": 1920},
            user_agent="persistent-agent",
            device_scale_factor=3,
            is_mobile=True,
            has_touch=True,
            color_scheme="dark",
            reduced_motion="reduce",
            contrast="more",
            args=["--disable-features=Translate"],
            channel="chrome",
            timeout=2_500,
        )

    # Assert
    assert environment.observed == (
        str(tmp_path.resolve()),
        False,
        (
            "https://example.test/root/",
            (
                "fr-FR",
                "America/Jamaica",
                (-70.0, 30.0, 7.0),
                True,
                ("user", "pass", None, False),
                True,
                True,
                True,
                False,
            ),
            (
                "persistent-agent",
                (456, 789),
                (1080, 1920),
                3.0,
                True,
                True,
            ),
            ("dark", "reduce", "more"),
            [("foo", "bar"), ("x-sequence", "two")],
            ["--disable-features=Translate"],
            None,
            "chrome",
            2_500,
        ),
    )


@pytest.mark.asyncio
async def test_persistent_context_rejects_uninstalled_permission_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(
        pwright.UnsupportedError,
        match="permissions are not installed: camera",
    ):
        await browser_type.launch_persistent_context(
            tmp_path,
            permissions=["geolocation", "camera"],
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_non_boolean_https_policy_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(
        TypeError,
        match="BrowserType.launch_persistent_context ignore_https_errors must be a boolean",
    ):
        await browser_type.launch_persistent_context(
            tmp_path,
            ignore_https_errors=cast(Any, "yes"),
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_non_boolean_offline_policy_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(
        TypeError,
        match="BrowserType.launch_persistent_context offline must be a boolean",
    ):
        await browser_type.launch_persistent_context(
            tmp_path,
            offline=cast(Any, "yes"),
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_non_boolean_bypass_csp_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(
        TypeError,
        match="BrowserType.launch_persistent_context bypass_csp must be a boolean",
    ):
        await browser_type.launch_persistent_context(
            tmp_path,
            bypass_csp=cast(Any, "yes"),
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_non_boolean_javascript_policy_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(
        TypeError,
        match="BrowserType.launch_persistent_context java_script_enabled must be a boolean",
    ):
        await browser_type.launch_persistent_context(
            tmp_path,
            java_script_enabled=cast(Any, "yes"),
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_unknown_media_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(pwright.Error, match="color_scheme: expected one of"):
        await browser_type.launch_persistent_context(
            tmp_path,
            color_scheme=cast(Any, "sepia"),
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_excess_headers_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")
    headers = {f"x-{index}": "value" for index in range(257)}

    # Act
    with pytest.raises(
        pwright.ResourceExhaustedError,
        match="persistent Context extra header count exceeds 256",
    ):
        await browser_type.launch_persistent_context(
            tmp_path,
            extra_http_headers=headers,
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_page_launch_argument_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(pwright.Error, match="Arguments can not specify page"):
        await browser_type.launch_persistent_context(
            tmp_path,
            args=["https://example.test/page"],
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_webkit_persistent_context_rejects_page_argument_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "webkit")

    # Act
    with pytest.raises(pwright.Error, match="Arguments can not specify page"):
        await browser_type.launch_persistent_context(
            tmp_path,
            args=["https://example.test/page"],
        )

    # Assert
    assert not environment.called


@pytest.mark.asyncio
@pytest.mark.parametrize("engine", ["firefox", "webkit"])
async def test_non_chromium_persistent_context_still_rejects_valid_launch_before_native(
    tmp_path: Path, engine: str
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), engine)

    # Act
    with pytest.raises(pwright.UnsupportedError, match="only available for Chromium"):
        await browser_type.launch_persistent_context(tmp_path, args=["--headless"])

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_rejects_empty_profile_before_native() -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")

    # Act
    with pytest.raises(pwright.UnsupportedError, match="requires a temporary profile"):
        await browser_type.launch_persistent_context("")

    # Assert
    assert not environment.called


def test_sync_persistent_context_uses_the_same_page_argument_preflight(tmp_path: Path) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    async_browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, async_browser_type)

    # Act
    try:
        with pytest.raises(pwright.Error, match="Arguments can not specify page"):
            browser_type.launch_persistent_context(
                tmp_path,
                args=["https://example.test/page"],
            )
    finally:
        bridge.close()

    # Assert
    assert not environment.called


def test_sync_webkit_persistent_context_rejects_page_argument_before_native(
    tmp_path: Path,
) -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    async_browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "webkit")
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, async_browser_type)

    # Act
    try:
        with pytest.raises(pwright.Error, match="Arguments can not specify page"):
            browser_type.launch_persistent_context(
                tmp_path,
                args=["https://example.test/page"],
            )
    finally:
        bridge.close()

    # Assert
    assert not environment.called


def test_sync_persistent_context_rejects_empty_profile_before_native() -> None:
    # Arrange
    class Environment:
        called = False

        async def launch_persistent_context(self, *_args: object) -> object:
            self.called = True
            raise AssertionError("native persistent launch must not run")

    class PlaywrightOwner:
        def __init__(self, environment: Environment) -> None:
            self._environment = environment
            self._browsers: list[object] = []

    environment = Environment()
    async_browser_type = BrowserType(cast(Any, PlaywrightOwner(environment)), "chromium")
    bridge = _LoopThread()
    browser_type = SyncBrowserType(bridge, async_browser_type)

    # Act
    try:
        with pytest.raises(pwright.UnsupportedError, match="requires a temporary profile"):
            browser_type.launch_persistent_context("")
    finally:
        bridge.close()

    # Assert
    assert not environment.called


@pytest.mark.asyncio
async def test_persistent_context_close_releases_its_native_browser_session() -> None:
    # Arrange
    observed: list[str] = []

    class Native:
        async def close_context(self, context_id: str) -> None:
            observed.append(f"context:{context_id}")

    class PersistentBrowser:
        def __init__(self) -> None:
            self._native = Native()
            self._persistent_context = True
            self._contexts: dict[str, BrowserContext] = {}

        async def close(self) -> None:
            observed.append("browser")

    browser = PersistentBrowser()
    context = BrowserContext(cast(Any, browser), "context-7", None)
    browser._contexts[context._object_id] = context

    # Act
    await context.close()

    # Assert
    assert observed == ["context:context-7", "browser"]
    assert context._closed


@pytest.mark.asyncio
async def test_owned_context_close_keeps_its_shared_browser_session_open() -> None:
    # Arrange
    observed: list[str] = []

    class Native:
        async def close_context(self, context_id: str) -> None:
            observed.append(f"context:{context_id}")

    class SharedBrowser:
        def __init__(self) -> None:
            self._native = Native()
            self._persistent_context = False
            self._contexts: dict[str, BrowserContext] = {}

        async def close(self) -> None:
            observed.append("browser")

    browser = SharedBrowser()
    context = BrowserContext(cast(Any, browser), "context-8", None)
    browser._contexts[context._object_id] = context

    # Act
    await context.close()

    # Assert
    assert observed == ["context:context-8"]
    assert context._closed


@pytest.mark.asyncio
async def test_browser_close_releases_its_python_environment_owner() -> None:
    # Arrange
    class Native:
        closed = False

        async def close(self) -> None:
            self.closed = True

    class PlaywrightOwner:
        def __init__(self) -> None:
            self._browsers: list[Browser] = []

    class BrowserTypeOwner:
        def __init__(self, playwright: PlaywrightOwner) -> None:
            self._playwright = playwright

    native = Native()
    playwright = PlaywrightOwner()
    browser = cast(Any, object.__new__(Browser))
    browser._browser_type = BrowserTypeOwner(playwright)
    browser._native = native
    browser._fresh_owned = False
    browser._contexts = {}
    browser._closed = False
    browser._binding_task = None
    browser._binding_dispatch_tasks = set()
    browser._route_task = None
    browser._lifecycle_task = None
    browser._request_task = None
    browser._response_task = None
    browser._download_task = None
    browser._worker_task = None
    browser._worker_console_task = None
    browser._cdp_event_task = None
    browser._cdp_registration_tasks = set()
    browser._cdp_detach_tasks = set()
    playwright._browsers.append(browser)

    # Act
    await browser.close()

    # Assert
    assert native.closed
    assert playwright._browsers == []


@pytest.mark.asyncio
async def test_browser_new_context_rejects_non_boolean_https_policy_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object) -> tuple[str, Optional[str]]:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(
        TypeError,
        match="Browser.new_context ignore_https_errors must be a boolean",
    ):
        await browser.new_context(ignore_https_errors=cast(Any, "yes"))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_rejects_non_boolean_offline_policy_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object) -> tuple[str, Optional[str]]:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(TypeError, match="Browser.new_context offline must be a boolean"):
        await browser.new_context(offline=cast(Any, "yes"))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_rejects_non_boolean_bypass_csp_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object) -> tuple[str, Optional[str]]:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(
        TypeError,
        match="Browser.new_context bypass_csp must be a boolean",
    ):
        await browser.new_context(bypass_csp=cast(Any, "yes"))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_rejects_non_boolean_javascript_policy_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object) -> tuple[str, Optional[str]]:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(
        TypeError,
        match="Browser.new_context java_script_enabled must be a boolean",
    ):
        await browser.new_context(java_script_enabled=cast(Any, "yes"))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_browser_new_context_rejects_unknown_service_worker_policy_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def new_context(self, *_args: object) -> tuple[str, Optional[str]]:
            self.called = True
            raise AssertionError("native context creation must not run")

    native = Native()
    browser = cast(Any, object.__new__(Browser))
    browser._native = native
    browser._contexts = {}

    # Act
    with pytest.raises(
        pwright.Error,
        match="service_workers: expected one of allow, block, got 'reject'",
    ):
        await browser.new_context(service_workers=cast(Any, "reject"))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_async_resource_context_managers_close_each_public_owner() -> None:
    # Arrange
    observed: list[str] = []
    resources = [
        ("page", cast(Any, object.__new__(Page))),
        ("context", cast(Any, object.__new__(BrowserContext))),
        ("browser", cast(Any, object.__new__(Browser))),
    ]
    for label, resource in resources:

        async def close(*, current: str = label) -> None:
            observed.append(current)

        resource.close = close

    # Act
    for _label, resource in resources:
        assert await resource.__aenter__() is resource
        await resource.__aexit__(None, None, None)

    # Assert
    assert observed == ["page", "context", "browser"]


def test_sync_resource_context_managers_close_each_public_owner() -> None:
    # Arrange
    observed: list[str] = []
    resources = [
        ("page", cast(Any, object.__new__(SyncPage))),
        ("context", cast(Any, object.__new__(SyncBrowserContext))),
        ("browser", cast(Any, object.__new__(SyncBrowser))),
    ]
    for label, resource in resources:

        def close(*, current: str = label) -> None:
            observed.append(current)

        resource.close = close

    # Act
    for _label, resource in resources:
        assert resource.__enter__() is resource
        resource.__exit__(None, None, None)

    # Assert
    assert observed == ["page", "context", "browser"]


@pytest.mark.asyncio
async def test_async_locator_wait_and_text_expectations_delegate_to_native_recipe() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: list[tuple[object, ...]] = []

        async def wait_for(self, state: str, timeout_ms: int) -> None:
            self.observed.append(("wait_for", state, timeout_ms))

        async def expect_text(
            self,
            expected: str,
            timeout_ms: int,
            is_not: bool,
            substring: bool,
            normalize_whitespace: bool,
            ignore_case: bool,
            use_inner_text: bool,
        ) -> dict[str, object]:
            self.observed.append(
                (
                    "expect_text",
                    expected,
                    timeout_ms,
                    is_not,
                    substring,
                    normalize_whitespace,
                    ignore_case,
                    use_inner_text,
                )
            )
            return {"matches": True, "received": expected, "timed_out": False}

        async def expect_texts(
            self,
            expected: list[str],
            timeout_ms: int,
            is_not: bool,
            substring: bool,
            normalize_whitespace: bool,
            ignore_case: bool,
            use_inner_text: bool,
        ) -> dict[str, object]:
            self.observed.append(
                (
                    "expect_texts",
                    expected,
                    timeout_ms,
                    is_not,
                    substring,
                    normalize_whitespace,
                    ignore_case,
                    use_inner_text,
                )
            )
            return {"matches": True, "received": expected, "timed_out": False}

        async def expect_string(
            self,
            kind: str,
            expected: str,
            flags: Optional[str],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed.append(("expect_string", kind, expected, flags, timeout_ms, is_not))
            return {"matches": True, "received": expected, "timed_out": False}

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float]) -> int:
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, native))

    # Act
    await locator.wait_for(state="attached", timeout=71)
    await async_expect(locator).to_have_text("ready", timeout=72)
    await async_expect(locator).not_to_have_text(["one", "two"], timeout=73)
    await async_expect(locator).to_have_class("selected", timeout=74)

    # Assert
    assert native.observed == [
        ("wait_for", "attached", 71),
        ("expect_text", "ready", 72, False, False, True, False, False),
        ("expect_texts", ["one", "two"], 73, True, False, True, False, False),
        ("expect_string", "class", "selected", None, 74, False),
    ]


def test_sync_locator_negated_text_expectation_adapts_the_same_native_recipe() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_text(
            self,
            expected: str,
            timeout_ms: int,
            is_not: bool,
            substring: bool,
            normalize_whitespace: bool,
            ignore_case: bool,
            use_inner_text: bool,
        ) -> dict[str, object]:
            self.observed = (
                expected,
                timeout_ms,
                is_not,
                substring,
                normalize_whitespace,
                ignore_case,
                use_inner_text,
            )
            return {"matches": True, "received": expected, "timed_out": False}

    native = NativeLocatorRecipe()
    async_locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))
    bridge = _LoopThread()
    locator = SyncLocator(bridge, async_locator)

    # Act
    try:
        sync_expect(locator).not_to_have_text("ready", timeout=75)
    finally:
        bridge.close()

    # Assert
    assert native.observed == ("ready", 75, True, False, True, False, False)


@pytest.mark.asyncio
async def test_async_contain_text_delegates_match_options_to_native_recipe() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_text(
            self,
            expected: str,
            timeout_ms: int,
            is_not: bool,
            substring: bool,
            normalize_whitespace: bool,
            ignore_case: bool,
            use_inner_text: bool,
        ) -> dict[str, object]:
            self.observed = (
                expected,
                timeout_ms,
                is_not,
                substring,
                normalize_whitespace,
                ignore_case,
                use_inner_text,
            )
            return {"matches": True, "received": expected, "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).not_to_contain_text(
        "ready", timeout=76, ignore_case=True, use_inner_text=True
    )

    # Assert
    assert native.observed == ("ready", 76, True, True, True, True, True)


@pytest.mark.asyncio
async def test_async_regex_text_expectation_delegates_canonical_flags() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_text_regex(
            self,
            source: str,
            flags: str,
            ignore_case: Optional[bool],
            use_inner_text: bool,
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed = (
                source,
                flags,
                ignore_case,
                use_inner_text,
                timeout_ms,
                is_not,
            )
            return {"matches": True, "received": "READY", "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_have_text(re.compile("ready", re.IGNORECASE), timeout=77)

    # Assert
    assert native.observed == ("ready", "i", None, False, 77, False)


@pytest.mark.asyncio
async def test_async_checked_expectation_delegates_tristate_options() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_checked(
            self,
            checked: Optional[bool],
            indeterminate: Optional[bool],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed = (checked, indeterminate, timeout_ms, is_not)
            return {"matches": True, "received": "mixed", "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_be_checked(indeterminate=True, timeout=78)

    # Assert
    assert native.observed == (None, True, 78, False)


@pytest.mark.asyncio
async def test_async_empty_expectation_delegates_typed_state() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_state(
            self, state: str, timeout_ms: int, is_not: bool
        ) -> dict[str, object]:
            self.observed = (state, timeout_ms, is_not)
            return {"matches": True, "received": "empty", "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).not_to_be_empty(timeout=79)

    # Assert
    assert native.observed == ("empty", 79, True)


@pytest.mark.asyncio
async def test_async_value_expectation_delegates_typed_string_matcher() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_string(
            self,
            kind: str,
            expected: str,
            flags: Optional[str],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed = (kind, expected, flags, timeout_ms, is_not)
            return {"matches": True, "received": expected, "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_have_value(re.compile("completed", re.I), timeout=80)

    # Assert
    assert native.observed == ("value", "completed", "i", 80, False)


@pytest.mark.asyncio
async def test_async_class_collection_expectation_delegates_exact_order() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_class_matchers(
            self,
            expected: list[tuple[str, Optional[str]]],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed = (expected, timeout_ms, is_not)
            return {"matches": True, "received": expected, "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_have_class(
        ["completed", re.compile("^ready$", re.M)], timeout=81
    )

    # Assert
    assert native.observed == ([("completed", None), ("^ready$", "m")], 81, False)


@pytest.mark.asyncio
async def test_async_attribute_expectation_keeps_regex_matching_in_rust() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_attribute(
            self,
            name: str,
            expected: str,
            match_options: tuple[Optional[str], Optional[bool]],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            flags, ignore_case = match_options
            self.observed = (name, expected, flags, ignore_case, timeout_ms, is_not)
            return {"matches": True, "received": "Target", "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_have_attribute(
        "id", re.compile("target", re.I | re.M), ignore_case=False, timeout=76
    )

    # Assert
    assert native.observed == ("id", "target", "im", False, 76, False)


@pytest.mark.asyncio
async def test_async_css_expectation_keeps_computed_style_matching_in_rust() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_css(
            self,
            name: str,
            expected: str,
            flags: Optional[str],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed = (name, expected, flags, timeout_ms, is_not)
            return {"matches": True, "received": expected, "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_have_css(
        "background-color", re.compile(r"rgb\(255, 0, 0\)", re.I), timeout=79
    )

    # Assert
    assert native.observed == (
        "background-color",
        r"rgb\(255, 0, 0\)",
        "i",
        79,
        False,
    )


@pytest.mark.asyncio
async def test_async_id_expectation_delegates_to_the_attribute_owner() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_attribute(
            self,
            name: str,
            expected: str,
            match_options: tuple[Optional[str], Optional[bool]],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed = (name, expected, match_options, timeout_ms, is_not)
            return {"matches": True, "received": expected, "timed_out": False}

    native = NativeLocatorRecipe()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    await async_expect(locator).to_have_id("target", timeout=78)

    # Assert
    assert native.observed == ("id", "target", (None, None), 78, False)


def test_sync_attribute_expectation_preserves_negative_polarity() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_attribute(
            self,
            name: str,
            expected: str,
            match_options: tuple[Optional[str], Optional[bool]],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            flags, ignore_case = match_options
            self.observed = (name, expected, flags, ignore_case, timeout_ms, is_not)
            return {"matches": False, "received": "other", "timed_out": False}

    native = NativeLocatorRecipe()
    bridge = _LoopThread()
    locator = SyncLocator(
        bridge,
        Locator(cast(Any, object()), None, native_locator=cast(Any, native)),
    )

    # Act
    try:
        sync_expect(locator).not_to.to_have_attribute("id", "target", timeout=77)
    finally:
        bridge.close()

    # Assert
    assert native.observed == ("id", "target", None, None, 77, True)


def test_sync_id_expectation_delegates_to_the_attribute_owner() -> None:
    # Arrange
    class NativeLocatorRecipe:
        def __init__(self) -> None:
            self.observed: object = None

        async def expect_attribute(
            self,
            name: str,
            expected: str,
            match_options: tuple[Optional[str], Optional[bool]],
            timeout_ms: int,
            is_not: bool,
        ) -> dict[str, object]:
            self.observed = (name, expected, match_options, timeout_ms, is_not)
            return {"matches": True, "received": expected, "timed_out": False}

    native = NativeLocatorRecipe()
    bridge = _LoopThread()
    locator = SyncLocator(
        bridge,
        Locator(cast(Any, object()), None, native_locator=cast(Any, native)),
    )

    # Act
    try:
        sync_expect(locator).to_have_id("target", timeout=79)
    finally:
        bridge.close()

    # Assert
    assert native.observed == ("id", "target", (None, None), 79, False)


def test_async_locator_description_returns_a_new_facade_without_native_work() -> None:
    # Arrange
    page = object()
    locator = Locator(cast(Any, page), "button")

    # Act
    described = locator.describe('Submit "button"')

    # Assert
    assert locator.description is None
    assert described.description == 'Submit "button"'
    assert described._page is page
    assert described._selector == "button"


def test_sync_locator_description_adapts_the_same_facade_metadata() -> None:
    # Arrange
    bridge = _LoopThread()
    locator = SyncLocator(bridge, Locator(cast(Any, object()), "button"))

    # Act
    try:
        described = locator.describe("Submit button")
    finally:
        bridge.close()

    # Assert
    assert locator.description is None
    assert described.description == "Submit button"


@pytest.mark.asyncio
async def test_locator_chaining_and_nth_keep_the_query_recipe_in_native_rust() -> None:
    # Arrange
    class NativeLocator:
        def __init__(self, recipe: tuple[object, ...] = ()) -> None:
            self.recipe = recipe

        def locator(self, selector: str) -> "NativeLocator":
            return NativeLocator((*self.recipe, ("within", selector)))

        def nth(self, index: int) -> "NativeLocator":
            return NativeLocator((*self.recipe, ("nth", index)))

        async def read(
            self,
            operation: str,
            timeout_ms: int,
            attribute: Optional[str],
        ) -> object:
            assert operation == "count"
            assert timeout_ms == 30_000
            assert attribute is None
            return len(self.recipe)

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert timeout is None
            assert navigation is False
            return 30_000

    root = Locator(cast(Any, PageOwner()), "section", native_locator=cast(Any, NativeLocator()))

    # Act
    derived = root.locator("p").nth(2)
    count = await derived.count()

    # Assert
    assert derived._selector is None
    assert derived._native_locator is not None
    assert cast(Any, derived._native_locator).recipe == (("within", "p"), ("nth", 2))
    assert count == 2


def test_locator_filters_and_combinations_keep_typed_recipes_in_native_rust() -> None:
    # Arrange
    class NativeLocator:
        def __init__(self, recipe: tuple[object, ...]) -> None:
            self.recipe = recipe

        def filter_text(self, text: str, exclude: bool) -> "NativeLocator":
            return NativeLocator((*self.recipe, ("text", text, exclude)))

        def filter_regex(self, source: str, flags: str, exclude: bool) -> "NativeLocator":
            return NativeLocator((*self.recipe, ("regex", source, flags, exclude)))

        def combine(self, other: "NativeLocator", kind: str) -> "NativeLocator":
            return NativeLocator((*self.recipe, (kind, other.recipe)))

        def visible(self, visible: bool) -> "NativeLocator":
            return NativeLocator((*self.recipe, ("visible", visible)))

        def semantic(
            self,
            kind: str,
            source: str,
            exact: bool,
            flags: Optional[str],
        ) -> "NativeLocator":
            return NativeLocator((*self.recipe, (kind, source, exact, flags)))

    page = cast(Any, object())
    outer = Locator(page, "article", native_locator=cast(Any, NativeLocator(("outer",))))
    inner = Locator(page, "span", native_locator=cast(Any, NativeLocator(("inner",))))

    # Act
    filtered = outer.filter(
        has_text=re.compile("^title$", re.I | re.M | re.S),
        has_not_text="draft",
        has=inner,
        visible=True,
    )
    child = filtered.get_by_text("ready", exact=True)
    combined = child.or_(inner).and_(outer)

    # Assert
    assert cast(Any, combined._native_locator).recipe == (
        "outer",
        ("regex", "^title$", "ims", False),
        ("text", "draft", True),
        ("has", ("inner",)),
        ("visible", True),
        ("text", "ready", True, None),
        ("or", ("inner",)),
        ("and", ("outer",)),
    )


def test_page_text_locator_uses_native_semantic_leaf_without_python_selector_encoding() -> None:
    # Arrange
    class NativeSession:
        observed: Optional[tuple[object, ...]] = None
        owner = object()

        def semantic_locator(
            self,
            frame_id: str,
            kind: str,
            source: str,
            exact: bool,
            flags: Optional[str],
        ) -> object:
            self.observed = (frame_id, kind, source, exact, flags)
            return self.owner

    native = NativeSession()

    class Browser:
        _native = native

    class Context:
        browser = Browser()

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")

    # Act
    locator = page.get_by_text(re.compile("Ready.*", re.I | re.S))

    # Assert
    assert native.observed == ("frame-8", "text", "Ready.*", False, "is")
    assert locator._selector is None
    assert locator._native_locator is native.owner


def test_page_and_frame_user_facing_locators_use_typed_native_semantic_leaves() -> None:
    # Arrange
    class NativeSession:
        def __init__(self) -> None:
            self.observed: list[tuple[object, ...]] = []

        def semantic_locator(
            self,
            frame_id: str,
            kind: str,
            source: str,
            exact: bool,
            flags: Optional[str],
        ) -> object:
            recipe = (frame_id, kind, source, exact, flags)
            self.observed.append(recipe)
            return recipe

    native = NativeSession()

    class Browser:
        _native = native

    class Context:
        browser = Browser()

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")
    frame = Frame(page, "frame-9", "frame-8", "child", "about:blank")

    # Act
    page_locators = [
        page.get_by_label("Name", exact=True),
        page.get_by_placeholder(re.compile("mail", re.I)),
        page.get_by_alt_text("Logo"),
        page.get_by_title("Issues"),
    ]
    frame_locators = [
        frame.get_by_text("Ready"),
        frame.get_by_label(re.compile("name", re.I)),
        frame.get_by_placeholder("Email", exact=True),
        frame.get_by_alt_text("Logo"),
        frame.get_by_title("Issues"),
    ]

    # Assert
    assert native.observed == [
        ("frame-8", "label", "Name", True, None),
        ("frame-8", "placeholder", "mail", False, "i"),
        ("frame-8", "altText", "Logo", False, None),
        ("frame-8", "title", "Issues", False, None),
        ("frame-9", "text", "Ready", False, None),
        ("frame-9", "label", "name", False, "i"),
        ("frame-9", "placeholder", "Email", True, None),
        ("frame-9", "altText", "Logo", False, None),
        ("frame-9", "title", "Issues", False, None),
    ]
    assert all(locator._selector is None for locator in [*page_locators, *frame_locators])


@pytest.mark.asyncio
async def test_locator_role_test_id_and_evaluation_keep_the_recipe_in_native_rust() -> None:
    # Arrange
    class NativeLocator:
        def __init__(self, recipe: tuple[object, ...] = ()) -> None:
            self.recipe = recipe
            self.evaluation: object = None

        def semantic(
            self,
            kind: str,
            source: str,
            exact: bool,
            flags: Optional[str],
        ) -> "NativeLocator":
            return NativeLocator((*self.recipe, (kind, source, exact, flags)))

        def role(self, *arguments: object) -> "NativeLocator":
            return NativeLocator((*self.recipe, ("role", *arguments)))

        async def evaluate(self, expression: str, argument: object, timeout_ms: int) -> object:
            self.evaluation = (expression, argument, timeout_ms)
            return {"recipe": self.recipe}

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float]) -> int:
            return 30_000 if timeout is None else int(timeout)

    root = Locator(cast(Any, PageOwner()), None, native_locator=cast(Any, NativeLocator()))

    # Act
    derived = root.get_by_test_id(re.compile("foo|bar", re.I)).get_by_role(
        "button",
        checked=True,
        disabled=False,
        expanded=True,
        include_hidden=True,
        level=2,
        name=re.compile("Submit", re.I),
        pressed=False,
        selected=True,
        exact=True,
    )
    result = await derived.evaluate("(element, arg) => [element, arg]", {"id": 7}, timeout=91)

    # Assert
    native = cast(Any, derived._native_locator)
    assert native.recipe == (
        ("testId", "foo|bar", False, "i"),
        (
            "role",
            "button",
            "Submit",
            "i",
            True,
            True,
            False,
            True,
            True,
            2,
            False,
            True,
        ),
    )
    assert native.evaluation == ("(element, arg) => [element, arg]", {"id": 7}, 91)
    assert result == {"recipe": native.recipe}


def test_page_role_and_test_id_locators_use_native_typed_leaves() -> None:
    # Arrange
    test_id_owner = object()
    role_owner = object()

    class NativeSession:
        observed: list[tuple[object, ...]] = []

        def semantic_locator(self, *arguments: object) -> object:
            self.observed.append(("semantic", *arguments))
            return test_id_owner

        def role_locator(self, *arguments: object) -> object:
            self.observed.append(("role", *arguments))
            return role_owner

    native = NativeSession()

    class Browser:
        _native = native

    class Context:
        browser = Browser()

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")

    # Act
    test_id = page.get_by_test_id(re.compile("foo|bar", re.I))
    role = page.get_by_role("button", name="Submit", exact=True)

    # Assert
    assert native.observed == [
        ("semantic", "frame-8", "testId", "foo|bar", False, "i"),
        (
            "role",
            "frame-8",
            "button",
            "Submit",
            None,
            True,
            None,
            None,
            None,
            False,
            None,
            None,
            None,
        ),
    ]
    assert test_id._native_locator is test_id_owner
    assert role._native_locator is role_owner


def test_frame_locator_forwards_frame_identity_to_native_owner() -> None:
    # Arrange
    class NativeSession:
        observed: Optional[tuple[str, str]] = None
        locator_owner = object()

        def locator(self, frame_id: str, selector: str) -> object:
            self.observed = (frame_id, selector)
            return self.locator_owner

    native = NativeSession()

    class Browser:
        _native = native

    class Context:
        browser = Browser()

    class PageOwner:
        _context = Context()

    page = cast(Any, PageOwner())
    frame = Frame(page, "child-frame-7", "main-frame", "child", "https://example.test/")

    # Act
    locator = frame.locator("button")

    # Assert
    assert native.observed == ("child-frame-7", "button")
    assert locator._page is page
    assert locator._native_locator is native.locator_owner


def test_page_frame_locator_forwards_identity_to_rust_recipe_owner() -> None:
    # Arrange
    class NativeFrameLocator:
        pass

    class NativeSession:
        observed: Optional[tuple[str, str]] = None
        owner = NativeFrameLocator()

        def frame_locator(self, frame_id: str, selector: str) -> NativeFrameLocator:
            self.observed = (frame_id, selector)
            return self.owner

    native = NativeSession()

    class Browser:
        _native = native

    class Context:
        browser = Browser()

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")

    # Act
    frame_locator = page.frame_locator("iframe")

    # Assert
    assert native.observed == ("frame-8", "iframe")
    assert frame_locator._native_locator is native.owner


def test_locator_frame_entry_remains_a_native_rust_recipe() -> None:
    # Arrange
    class NativeFrameLocator:
        pass

    class NativeLocator:
        def __init__(self) -> None:
            self.observed: list[tuple[str, object]] = []
            self.frame_owner = NativeFrameLocator()
            self.content_owner = NativeFrameLocator()

        def frame_locator(self, selector: str) -> NativeFrameLocator:
            self.observed.append(("frame_locator", selector))
            return self.frame_owner

        def content_frame(self) -> NativeFrameLocator:
            self.observed.append(("content_frame", None))
            return self.content_owner

    native = NativeLocator()
    locator = Locator(cast(Any, object()), "body", native_locator=cast(Any, native))

    # Act
    nested = locator.frame_locator("iframe")
    content = locator.content_frame

    # Assert
    assert native.observed == [("frame_locator", "iframe"), ("content_frame", None)]
    assert nested._native_locator is native.frame_owner
    assert content._native_locator is native.content_owner


def test_frame_locator_re_roots_string_and_locator_recipes_in_rust() -> None:
    # Arrange
    class NativeLocator:
        pass

    class NativeFrameLocator:
        def __init__(self) -> None:
            self.observed: list[tuple[str, object]] = []
            self.string_owner = NativeLocator()
            self.locator_owner = NativeLocator()
            self.nth_owner = cast(Any, self)
            self.owner_locator = NativeLocator()

        def locator(self, selector: str) -> NativeLocator:
            self.observed.append(("locator", selector))
            return self.string_owner

        def locator_native(self, locator: NativeLocator) -> NativeLocator:
            self.observed.append(("locator_native", locator))
            return self.locator_owner

        def nth(self, index: int) -> "NativeFrameLocator":
            self.observed.append(("nth", index))
            return self

        def owner(self) -> NativeLocator:
            self.observed.append(("owner", None))
            return self.owner_locator

    page = cast(Any, object())
    native = NativeFrameLocator()
    frame_locator = FrameLocator(page, "iframe", cast(Any, native))
    inner_native = NativeLocator()
    inner = Locator(page, "input", native_locator=cast(Any, inner_native))

    # Act
    by_string = frame_locator.locator("button")
    by_locator = frame_locator.locator(inner)
    second = frame_locator.nth(1)
    owner = frame_locator.owner

    # Assert
    assert native.observed == [
        ("locator", "button"),
        ("locator_native", inner_native),
        ("nth", 1),
        ("owner", None),
    ]
    assert by_string._native_locator is native.string_owner
    assert by_locator._native_locator is native.locator_owner
    assert second._native_locator is native
    assert owner._native_locator is native.owner_locator


def test_frame_locator_user_facing_queries_stay_in_native_rust() -> None:
    # Arrange
    class NativeLocator:
        def __init__(self, recipe: tuple[object, ...]) -> None:
            self.recipe = recipe

    class NativeFrameLocator:
        def __init__(self) -> None:
            self.observed: list[tuple[object, ...]] = []

        def semantic(
            self,
            kind: str,
            source: str,
            exact: bool,
            flags: Optional[str],
        ) -> NativeLocator:
            recipe = (kind, source, exact, flags)
            self.observed.append(recipe)
            return NativeLocator(recipe)

        def role(self, *arguments: object) -> NativeLocator:
            recipe = ("role", *arguments)
            self.observed.append(recipe)
            return NativeLocator(recipe)

    page = cast(Any, object())
    native = NativeFrameLocator()
    frame_locator = FrameLocator(page, "iframe", cast(Any, native))

    # Act
    locators = [
        frame_locator.get_by_text("Ready"),
        frame_locator.get_by_label(re.compile("name", re.I)),
        frame_locator.get_by_placeholder("Email", exact=True),
        frame_locator.get_by_alt_text("Logo"),
        frame_locator.get_by_title("Issues"),
        frame_locator.get_by_test_id("save"),
        frame_locator.get_by_role("button", name="Save"),
    ]

    # Assert
    assert native.observed[:6] == [
        ("text", "Ready", False, None),
        ("label", "name", False, "i"),
        ("placeholder", "Email", True, None),
        ("altText", "Logo", False, None),
        ("title", "Issues", False, None),
        ("testId", "save", False, None),
    ]
    assert native.observed[6][0] == "role"
    assert all(locator._selector is None for locator in locators)


@pytest.mark.asyncio
async def test_locator_all_derives_bounded_native_recipes() -> None:
    # Arrange
    class NativeLocator:
        def __init__(self, recipe: tuple[object, ...] = ()) -> None:
            self.recipe = recipe

        def nth(self, index: int) -> "NativeLocator":
            return NativeLocator((*self.recipe, ("nth", index)))

        async def read(
            self,
            operation: str,
            timeout_ms: int,
            attribute: Optional[str],
        ) -> object:
            assert (operation, timeout_ms, attribute) == ("count", 30_000, None)
            return 3

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert timeout is None
            assert navigation is False
            return 30_000

    root = Locator(cast(Any, PageOwner()), "article", native_locator=cast(Any, NativeLocator()))

    # Act
    locators = await root.all()

    # Assert
    assert [cast(Any, locator._native_locator).recipe for locator in locators] == [
        (("nth", 0),),
        (("nth", 1),),
        (("nth", 2),),
    ]


@pytest.mark.asyncio
async def test_locator_all_rejects_unbounded_result_before_deriving_recipes() -> None:
    # Arrange
    class NativeLocator:
        nth_calls = 0

        def nth(self, _index: int) -> "NativeLocator":
            self.nth_calls += 1
            return self

        async def read(
            self,
            operation: str,
            timeout_ms: int,
            attribute: Optional[str],
        ) -> object:
            assert (operation, timeout_ms, attribute) == ("count", 30_000, None)
            return 1025

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert timeout is None
            assert navigation is False
            return 30_000

    native = NativeLocator()
    root = Locator(cast(Any, PageOwner()), "article", native_locator=cast(Any, native))

    # Act
    with pytest.raises(pwright.Error, match="fixed 1024-item boundary"):
        await root.all()

    # Assert
    assert native.nth_calls == 0


@pytest.mark.asyncio
async def test_locator_clear_delegates_empty_fill_to_native_recipe() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[object, ...]] = None

        async def fill(self, value: str, timeout_ms: int, force: bool) -> None:
            self.observed = (value, timeout_ms, force)

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "input", native_locator=cast(Any, native))

    # Act
    await locator.clear(timeout=73, force=True)

    # Assert
    assert native.observed == ("", 73, True)


@pytest.mark.asyncio
async def test_locator_double_click_and_file_paths_delegate_to_native_recipes(
    tmp_path: Path,
) -> None:
    # Arrange
    class NativeLocator:
        observed: list[tuple[object, ...]]

        def __init__(self) -> None:
            self.observed = []

        async def dblclick(self, timeout_ms: int) -> None:
            self.observed.append(("dblclick", timeout_ms))

        async def set_input_files_paths(self, base: str, paths: list[str], timeout_ms: int) -> None:
            self.observed.append(("paths", base, paths, timeout_ms))

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    source = tmp_path / "input.txt"
    source.write_text("payload")
    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "input", native_locator=cast(Any, native))

    # Act
    await locator.dblclick(timeout=71)
    await locator.set_input_files(source, timeout=72)

    # Assert
    assert native.observed == [
        ("dblclick", 71),
        ("paths", os.getcwd(), [str(source)], 72),
    ]


@pytest.mark.asyncio
async def test_locator_file_payloads_remain_bytes_at_the_native_boundary() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[list[dict[str, object]], int]] = None

        async def set_input_file_payloads(
            self, payloads: list[dict[str, object]], timeout_ms: int
        ) -> None:
            self.observed = (payloads, timeout_ms)

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "input", native_locator=cast(Any, native))
    payload: FilePayload = {
        "name": "input.txt",
        "mimeType": "text/plain",
        "buffer": b"payload",
    }

    # Act
    await locator.set_input_files(payload, timeout=73)

    # Assert
    assert native.observed == ([payload], 73)


@pytest.mark.asyncio
async def test_page_set_input_files_delegates_to_the_rust_locator(
    tmp_path: Path,
) -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[str, list[str], int]] = None

        async def set_input_files_paths(self, base: str, paths: list[str], timeout_ms: int) -> None:
            self.observed = (base, paths, timeout_ms)

    class NativeSession:
        def __init__(self, locator: NativeLocator) -> None:
            self._locator = locator

        def locator(self, frame_id: str, selector: str) -> NativeLocator:
            assert (frame_id, selector) == ("frame-3", "input")
            return self._locator

    class BrowserOwner:
        def __init__(self, native: NativeSession) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    source = tmp_path / "page-upload.txt"
    source.write_text("payload")
    native_locator = NativeLocator()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(NativeSession(native_locator)))),
        "page-2",
        "frame-3",
        "about:blank",
    )

    # Act
    await page.set_input_files("input", source, timeout=74)

    # Assert
    assert native_locator.observed == (os.getcwd(), [str(source)], 74)


@pytest.mark.asyncio
async def test_mouse_move_delegates_to_the_single_native_mouse_boundary() -> None:
    # Arrange
    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        async def mouse(self, *arguments: object) -> None:
            self.observed = arguments

    class BrowserOwner:
        def __init__(self, native: NativeSession) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    native = NativeSession()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(native))),
        "page-2",
        "frame-3",
        "about:blank",
    )

    # Act
    await page.mouse.move(50, 60, steps=2)

    # Assert
    assert native.observed == ("page-2", "move", 50.0, 60.0, (0.0, 2.0, 1.0, 0.0))


@pytest.mark.asyncio
async def test_mouse_wheel_delegates_to_the_single_native_mouse_boundary() -> None:
    # Arrange
    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        async def mouse(self, *arguments: object) -> None:
            self.observed = arguments

    class BrowserOwner:
        def __init__(self, native: NativeSession) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    native = NativeSession()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(native))),
        "page-2",
        "frame-3",
        "about:blank",
    )

    # Act
    await page.mouse.wheel(0, 100)

    # Assert
    assert native.observed == ("page-2", "wheel", 0.0, 100.0, (0.0, 1.0, 1.0, 0.0))


class _ObservedKeyboardNative:
    observed: Optional[tuple[object, ...]] = None

    async def keyboard(self, *arguments: object) -> None:
        self.observed = arguments


class _KeyboardBrowserOwner:
    def __init__(self, native: _ObservedKeyboardNative) -> None:
        self._native = native


class _KeyboardContextOwner:
    def __init__(self, native: _ObservedKeyboardNative) -> None:
        self.browser = _KeyboardBrowserOwner(native)
        self._object_id = "context-1"


def _keyboard_page(native: _ObservedKeyboardNative) -> Page:
    return Page(
        cast(Any, _KeyboardContextOwner(native)),
        "page-2",
        "frame-3",
        "about:blank",
    )


@pytest.mark.asyncio
async def test_keyboard_down_delegates_to_the_single_native_keyboard_boundary() -> None:
    # Arrange
    native = _ObservedKeyboardNative()
    page = _keyboard_page(native)

    # Act
    await page.keyboard.down("Shift")

    # Assert
    assert native.observed == ("page-2", "down", "Shift", 0.0)


@pytest.mark.asyncio
async def test_keyboard_up_delegates_to_the_single_native_keyboard_boundary() -> None:
    # Arrange
    native = _ObservedKeyboardNative()
    page = _keyboard_page(native)

    # Act
    await page.keyboard.up("Shift")

    # Assert
    assert native.observed == ("page-2", "up", "Shift", 0.0)


@pytest.mark.asyncio
async def test_keyboard_insert_text_delegates_to_native_keyboard_boundary() -> None:
    # Arrange
    native = _ObservedKeyboardNative()
    page = _keyboard_page(native)

    # Act
    await page.keyboard.insert_text("hello")

    # Assert
    assert native.observed == ("page-2", "insert_text", "hello", 0.0)


@pytest.mark.asyncio
async def test_keyboard_type_delegates_delay_to_native_keyboard_boundary() -> None:
    # Arrange
    native = _ObservedKeyboardNative()
    page = _keyboard_page(native)

    # Act
    await page.keyboard.type("hello", delay=12.5)

    # Assert
    assert native.observed == ("page-2", "type", "hello", 12.5)


@pytest.mark.asyncio
async def test_keyboard_press_delegates_delay_to_native_keyboard_boundary() -> None:
    # Arrange
    native = _ObservedKeyboardNative()
    page = _keyboard_page(native)

    # Act
    await page.keyboard.press("Shift+Tab", delay=9)

    # Assert
    assert native.observed == ("page-2", "press", "Shift+Tab", 9.0)


@pytest.mark.asyncio
async def test_keyboard_press_rewrites_native_error_with_official_api_name() -> None:
    # Arrange
    class FailingKeyboardNative(_ObservedKeyboardNative):
        async def keyboard(self, *arguments: object) -> None:
            del arguments
            raise pwright.Error('Unknown key: "NotARealKey"')

    page = _keyboard_page(FailingKeyboardNative())

    # Act
    with pytest.raises(pwright.Error) as raised:
        await page.keyboard.press("NotARealKey")

    # Assert
    assert raised.value.message == 'Keyboard.press: Unknown key: "NotARealKey"'


@pytest.mark.asyncio
async def test_page_type_uses_first_rust_locator_and_preserves_fractional_delay() -> None:
    # Arrange
    class NativeLocator:
        def __init__(self, index: Optional[int] = None) -> None:
            self.index = index
            self.observed: Optional[tuple[object, ...]] = None

        def nth(self, index: int) -> "NativeLocator":
            return NativeLocator(index)

        async def press_sequentially(self, *arguments: object) -> None:
            self.observed = arguments
            selected.append(self)

    class NativeSession:
        def locator(self, frame_id: str, selector: str) -> NativeLocator:
            assert (frame_id, selector) == ("frame-3", "textarea")
            return NativeLocator()

        @staticmethod
        def context_selector_is_strict(context_id: str, strict: Optional[bool]) -> bool:
            assert context_id == "context-1"
            return bool(strict)

        @staticmethod
        def page_default_timeout(page_id: str, navigation: bool) -> int:
            assert (page_id, navigation) == ("page-2", False)
            return 30_000

    selected: list[NativeLocator] = []
    native = cast(Any, NativeSession())
    page = Page(
        cast(Any, _KeyboardContextOwner(native)),
        "page-2",
        "frame-3",
        "about:blank",
    )

    # Act
    await page.type("textarea", "hello", delay=2.5)

    # Assert
    assert len(selected) == 1
    assert selected[0].index == 0
    assert selected[0].observed == ("hello", 30_000, 2)


@pytest.mark.asyncio
async def test_frame_query_selector_returns_rust_owned_element_handle() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "textarea"

    class NativeLocator:
        def __init__(self, index: Optional[int] = None) -> None:
            self.index = index

        def nth(self, index: int) -> "NativeLocator":
            return NativeLocator(index)

        async def read(self, operation: str, timeout_ms: int, attribute: Optional[str]) -> int:
            assert (operation, timeout_ms, attribute) == ("count", 30_000, None)
            return 1

        async def element_handle(self, timeout_ms: int) -> NativeHandle:
            assert timeout_ms == 30_000
            assert self.index == 0
            return NativeHandle()

    class NativeSession:
        def locator(self, frame_id: str, selector: str) -> NativeLocator:
            assert (frame_id, selector) == ("child-frame", "textarea")
            return NativeLocator()

        @staticmethod
        def context_selector_is_strict(context_id: str, strict: Optional[bool]) -> bool:
            assert context_id == "context-1"
            return bool(strict)

        @staticmethod
        def page_default_timeout(page_id: str, navigation: bool) -> int:
            assert (page_id, navigation) == ("page-2", False)
            return 30_000

    native = cast(Any, NativeSession())
    page = Page(
        cast(Any, _KeyboardContextOwner(native)),
        "page-2",
        "frame-3",
        "about:blank",
    )
    frame = Frame(page, "child-frame", "frame-3", "child", "about:blank")

    # Act
    handle = await frame.query_selector("textarea")

    # Assert
    assert isinstance(handle, ElementHandle)


@pytest.mark.asyncio
async def test_frame_eval_on_selector_delegates_exact_frame_to_rust() -> None:
    # Arrange
    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        @staticmethod
        def context_selector_is_strict(context_id: str, strict: Optional[bool]) -> bool:
            assert context_id == "context-1"
            return bool(strict)

        async def eval_on_selector(
            self,
            frame_id: str,
            selector: str,
            expression: str,
            argument: object,
            timeout_ms: int,
        ) -> str:
            self.observed = (frame_id, selector, expression, argument, timeout_ms)
            return "emoji"

        @staticmethod
        def page_default_timeout(page_id: str, navigation: bool) -> int:
            assert (page_id, navigation) == ("page-2", False)
            return 30_000

        @staticmethod
        def pages(context_id: str) -> list[tuple[str, str, str, str]]:
            assert context_id == "context-1"
            return [("page-2", context_id, "frame-3", "about:blank")]

    native = cast(Any, NativeSession())
    context = cast(Any, _KeyboardContextOwner(native))
    context._object_id = "context-1"
    page = Page(
        context,
        "page-2",
        "frame-3",
        "about:blank",
    )
    frame = Frame(page, "child-frame", "frame-3", "child", "about:blank")

    # Act
    value = await frame.eval_on_selector(
        "textarea",
        "(element, suffix) => element.value + suffix",
        {"suffix": "!"},
    )

    # Assert
    assert value == "emoji"
    assert native.observed == (
        "child-frame",
        "textarea",
        "(element, suffix) => element.value + suffix",
        {"suffix": "!"},
        30_000,
    )


@pytest.mark.asyncio
async def test_frame_eval_on_selector_all_delegates_collection_to_rust() -> None:
    # Arrange
    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        async def eval_on_selector_all(
            self,
            frame_id: str,
            selector: str,
            expression: str,
            argument: object,
        ) -> list[str]:
            self.observed = (frame_id, selector, expression, argument)
            return ["one", "two"]

        @staticmethod
        def pages(context_id: str) -> list[tuple[str, str, str, str]]:
            assert context_id == "context-1"
            return [("page-2", context_id, "frame-3", "about:blank")]

    native = cast(Any, NativeSession())
    context = cast(Any, _KeyboardContextOwner(native))
    context._object_id = "context-1"
    page = Page(
        context,
        "page-2",
        "frame-3",
        "about:blank",
    )
    frame = Frame(page, "child-frame", "frame-3", "child", "about:blank")

    # Act
    value = await frame.eval_on_selector_all(
        "li",
        "elements => elements.map(element => element.textContent)",
        None,
    )

    # Assert
    assert value == ["one", "two"]
    assert native.observed == (
        "child-frame",
        "li",
        "elements => elements.map(element => element.textContent)",
        None,
    )


@pytest.mark.asyncio
async def test_element_handle_press_uses_single_rust_keyboard_boundary() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "textarea"

    class NativeSession:
        observed: list[tuple[object, ...]] = []

        async def element_handle_keyboard(self, *arguments: object) -> None:
            self.observed.append(arguments)

    native = cast(Any, NativeSession())
    page = _keyboard_page(native)
    handle_owner = cast(Any, NativeHandle())
    handle = ElementHandle(native, handle_owner, page)

    # Act
    await handle.press("Control+A", delay=1.25)

    # Assert
    assert native.observed == [
        (handle_owner, "page-2", "press", "Control+A", 1.25),
    ]


@pytest.mark.asyncio
async def test_element_handle_type_uses_single_rust_keyboard_boundary() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "textarea"

    class NativeSession:
        observed: list[tuple[object, ...]] = []

        async def element_handle_keyboard(self, *arguments: object) -> None:
            self.observed.append(arguments)

    native = cast(Any, NativeSession())
    page = _keyboard_page(native)
    handle_owner = cast(Any, NativeHandle())
    handle = ElementHandle(native, handle_owner, page)

    # Act
    await handle.type("hello", delay=2.5)

    # Assert
    assert native.observed == [
        (handle_owner, "page-2", "type", "hello", 2.5),
    ]


def test_sync_keyboard_press_delegates_to_async_keyboard_facade() -> None:
    # Arrange
    native = _ObservedKeyboardNative()
    async_page = _keyboard_page(native)
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))

    # Act
    try:
        page.keyboard.press("Enter", delay=4.5)
    finally:
        bridge.close()

    # Assert
    assert native.observed == ("page-2", "press", "Enter", 4.5)


def test_file_chooser_repr_contains_its_page_and_element() -> None:
    # Arrange
    class NativeChooser:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "JSHandle@<input/>"

    class BrowserOwner:
        _native = cast(Any, object())

    class ContextOwner:
        browser = BrowserOwner()

    page = Page(cast(Any, ContextOwner()), "page-2", "frame-3", "about:blank")
    chooser = FileChooser(page, cast(Any, NativeChooser()), False)

    # Act
    value = repr(chooser)

    # Assert
    assert value == f"<FileChooser page={page} element={chooser.element}>"
    assert chooser.element.as_element() is chooser.element


def test_sync_locator_adapts_double_click_and_file_paths(tmp_path: Path) -> None:
    # Arrange
    class NativeLocator:
        observed: list[tuple[object, ...]]

        def __init__(self) -> None:
            self.observed = []

        async def dblclick(self, timeout_ms: int) -> None:
            self.observed.append(("dblclick", timeout_ms))

        async def set_input_files_paths(self, base: str, paths: list[str], timeout_ms: int) -> None:
            self.observed.append(("paths", base, paths, timeout_ms))

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    source = tmp_path / "input.txt"
    source.write_text("payload")
    native = NativeLocator()
    bridge = _LoopThread()
    locator = SyncLocator(
        bridge,
        Locator(cast(Any, PageOwner()), "input", native_locator=cast(Any, native)),
    )

    # Act
    try:
        locator.dblclick(timeout=74)
        locator.set_input_files(source, timeout=75)
    finally:
        bridge.close()

    # Assert
    assert native.observed == [
        ("dblclick", 74),
        ("paths", os.getcwd(), [str(source)], 75),
    ]


@pytest.mark.asyncio
async def test_locator_highlight_delegates_description_to_native_recipe() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[str] = None

        async def highlight(self, description: str) -> None:
            self.observed = description

    native = NativeLocator()
    locator = Locator(
        cast(Any, object()),
        ".box",
        description="important box",
        native_locator=cast(Any, native),
    )

    # Act
    await locator.highlight()

    # Assert
    assert native.observed == "important box"


def test_sync_locator_adapts_highlight() -> None:
    # Arrange
    class NativeLocator:
        called = False

        async def highlight(self, description: str) -> None:
            assert description == ""
            self.called = True

    native = NativeLocator()
    bridge = _LoopThread()
    locator = SyncLocator(
        bridge,
        Locator(cast(Any, object()), ".box", native_locator=cast(Any, native)),
    )

    # Act
    try:
        locator.highlight()
    finally:
        bridge.close()

    # Assert
    assert native.called is True


@pytest.mark.asyncio
async def test_locator_evaluate_all_delegates_native_recipe_without_selector_roundtrip() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[str, object]] = None

        async def evaluate_all(self, source: str, argument: object) -> object:
            self.observed = (source, argument)
            return ["<button>Click me</button>"]

    native = NativeLocator()
    locator = Locator(cast(Any, object()), None, native_locator=cast(Any, native))

    # Act
    result = await locator.evaluate_all("els => els.map(e => e.outerHTML)", {"limit": 1})

    # Assert
    assert result == ["<button>Click me</button>"]
    assert native.observed == (
        "els => els.map(e => e.outerHTML)",
        {"limit": 1},
    )


@pytest.mark.asyncio
async def test_locator_dom_actions_delegate_bounded_values_to_native_recipes() -> None:
    # Arrange
    class NativeLocator:
        observed: list[tuple[object, ...]] = []

        async def dispatch_event(
            self, event_type: str, event_init: Optional[dict[str, object]], timeout_ms: int
        ) -> None:
            self.observed.append(("dispatch", event_type, event_init, timeout_ms))

        async def select_text(self, timeout_ms: int, force: bool) -> None:
            self.observed.append(("select", timeout_ms, force))

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "textarea", native_locator=cast(Any, native))

    # Act
    await locator.dispatch_event("click", {"detail": 2}, timeout=71)
    await locator.select_text(force=True, timeout=72)

    # Assert
    assert native.observed == [
        ("dispatch", "click", {"detail": 2}, 71),
        ("select", 72, True),
    ]


@pytest.mark.asyncio
async def test_locator_evaluate_handle_preserves_native_preview_and_selector_chain() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def preview() -> str:
            return "JSHandle@#text=Text,↵more text"

        @staticmethod
        def is_element() -> bool:
            return False

    class NativeLocator:
        observed: Optional[tuple[str, object, int]] = None

        def locator(self, selector: str) -> "NativeLocator":
            assert selector == "#inner"
            return self

        async def evaluate_handle(
            self, source: str, argument: object, timeout_ms: int
        ) -> NativeHandle:
            self.observed = (source, argument, timeout_ms)
            return NativeHandle()

    class Browser:
        _native = cast(Any, object())

    class Context:
        browser = Browser()

    class Frame:
        def __repr__(self) -> str:
            return "<Frame name= url='http://example.test/dom.html'>"

    class PageOwner:
        context = Context()
        _context = context
        main_frame = Frame()

        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    outer = Locator(cast(Any, PageOwner()), "#outer", native_locator=cast(Any, native))
    inner = outer.locator("#inner")

    # Act
    handle = await inner.evaluate_handle("e => e.firstChild", timeout=73)

    # Assert
    assert native.observed == ("e => e.firstChild", None, 73)
    assert str(inner) == (
        "<Locator frame=<Frame name= url='http://example.test/dom.html'> "
        "selector='#outer >> #inner'>"
    )
    assert str(handle) == "JSHandle@#text=Text,↵more text"


@pytest.mark.asyncio
async def test_locator_hover_delegates_options_to_native_recipe() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[object, ...]] = None

        async def hover(self, timeout_ms: int, force: bool, trial: bool) -> None:
            self.observed = (timeout_ms, force, trial)

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "button", native_locator=cast(Any, native))

    # Act
    await locator.hover(timeout=74, force=True, trial=True)

    # Assert
    assert native.observed == (74, True, True)


@pytest.mark.asyncio
async def test_locator_sequential_typing_delegates_to_native_recipe() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[object, ...]] = None

        async def press_sequentially(
            self, text: str, timeout_ms: int, delay_ms: Optional[int]
        ) -> None:
            self.observed = (text, timeout_ms, delay_ms)

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "input", native_locator=cast(Any, native))

    # Act
    await locator.press_sequentially("hello", delay=7, timeout=75)

    # Assert
    assert native.observed == ("hello", 75, 7)


@pytest.mark.asyncio
async def test_locator_bounding_box_delegates_to_native_rust() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[int] = None

        async def bounding_box(self, timeout_ms: int) -> dict[str, float]:
            self.observed = timeout_ms
            return {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "div", native_locator=cast(Any, native))

    # Act
    box = await locator.bounding_box(timeout=81)

    # Assert
    assert native.observed == 81
    assert box == {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}


@pytest.mark.asyncio
async def test_locator_screenshot_delegates_bounded_options_to_native_rust() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[object, ...]] = None

        async def screenshot(
            self,
            format: int,
            path: Optional[str],
            quality: Optional[int],
            scale: int,
            timeout_ms: int,
            mask: list[object],
        ) -> bytes:
            self.observed = (format, path, quality, scale, timeout_ms, mask)
            return b"image"

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "div", native_locator=cast(Any, native))

    # Act
    image = await locator.screenshot(type="jpeg", quality=80, scale="css", timeout=82)

    # Assert
    assert native.observed == (1, None, 80, 1, 82, [])
    assert image == b"image"


def test_sync_locator_screenshot_waits_for_native_bytes() -> None:
    # Arrange
    class NativeLocator:
        async def screenshot(
            self,
            format: int,
            path: Optional[str],
            quality: Optional[int],
            scale: int,
            timeout_ms: int,
            mask: list[object],
        ) -> bytes:
            assert (format, path, quality, scale, timeout_ms, mask) == (
                -1,
                None,
                None,
                0,
                83,
                [],
            )
            return b"sync-image"

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    bridge = _LoopThread()
    async_locator = Locator(
        cast(Any, PageOwner()), "div", native_locator=cast(Any, NativeLocator())
    )
    locator = SyncLocator(bridge, async_locator, cast(Any, object()))

    # Act
    try:
        image = locator.screenshot(timeout=83)
    finally:
        bridge.close()

    # Assert
    assert image == b"sync-image"


@pytest.mark.asyncio
async def test_page_screenshot_delegates_path_inference_and_masks_to_native_rust(
    tmp_path: Path,
) -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[object, ...]] = None

        @staticmethod
        def page_default_timeout(_page_id: str, _navigation: bool) -> int:
            return 30_000

        async def page_screenshot(self, *arguments: object) -> bytes:
            self.observed = arguments
            return b"page-image"

    native = Native()
    context = cast(Any, type("Context", (), {})())
    context.browser = cast(Any, type("Browser", (), {"_native": native})())
    page = Page(context, "11", "12", "about:blank")
    output = tmp_path / "capture.jpeg"

    # Act
    image = await page.screenshot(path=output)

    # Assert
    assert native.observed == ("11", -1, str(output), None, False, 0, 30_000, [])
    assert image == b"page-image"


@pytest.mark.asyncio
async def test_element_screenshot_delegates_exact_handle_to_native_rust() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "body"

    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        async def element_screenshot(self, *arguments: object) -> bytes:
            self.observed = arguments
            return b"element-image"

    session = NativeSession()
    handle = cast(Any, NativeHandle())
    element = ElementHandle(cast(Any, session), handle)

    # Act
    image = await element.screenshot(type="png", timeout=84)

    # Assert
    assert session.observed == (handle, 0, None, None, 0, 84, [])
    assert image == b"element-image"


@pytest.mark.asyncio
async def test_element_set_input_files_delegates_paths_to_exact_native_handle(
    tmp_path: Path,
) -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "input"

    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        async def element_handle_set_input_files_paths(self, *arguments: object) -> None:
            self.observed = arguments

    session = NativeSession()
    native_handle = cast(Any, NativeHandle())
    element = ElementHandle(cast(Any, session), native_handle)
    upload = tmp_path / "upload.txt"
    upload.write_text("payload")

    # Act
    await element.set_input_files(upload, timeout=85)

    # Assert
    assert session.observed == (native_handle, os.getcwd(), [str(upload)], 85)


@pytest.mark.asyncio
async def test_element_set_input_files_delegates_payloads_to_exact_native_handle() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "input"

    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        async def element_handle_set_input_file_payloads(self, *arguments: object) -> None:
            self.observed = arguments

    session = NativeSession()
    native_handle = cast(Any, NativeHandle())
    element = ElementHandle(cast(Any, session), native_handle)
    payload: FilePayload = {
        "name": "upload.txt",
        "mimeType": "text/plain",
        "buffer": b"payload",
    }

    # Act
    await element.set_input_files(payload, timeout=86)

    # Assert
    assert session.observed == (native_handle, [payload], 86)


def test_sync_element_set_input_files_waits_for_native_completion() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "input"

    class NativeSession:
        observed: Optional[tuple[object, ...]] = None

        async def element_handle_set_input_file_payloads(self, *arguments: object) -> None:
            self.observed = arguments

    session = NativeSession()
    native_handle = cast(Any, NativeHandle())
    async_element = ElementHandle(cast(Any, session), native_handle)
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, async_element)
    payload: FilePayload = {
        "name": "upload.txt",
        "mimeType": "text/plain",
        "buffer": b"payload",
    }

    # Act
    try:
        element.set_input_files(payload, timeout=87)
    finally:
        bridge.close()

    # Assert
    assert session.observed == (native_handle, [payload], 87)


@pytest.mark.asyncio
async def test_element_handle_dom_reads_delegate_to_native_rust() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "div"

    class NativeSession:
        observed: list[tuple[object, ...]] = []

        async def element_handle_inner_text(self, handle: object) -> str:
            self.observed.append(("inner_text", handle))
            return "child text"

        async def element_handle_query_selector(
            self, handle: object, selector: str
        ) -> NativeHandle:
            self.observed.append(("query_selector", handle, selector))
            return NativeHandle()

    session = NativeSession()
    owner = cast(Any, NativeHandle())
    element = ElementHandle(cast(Any, session), owner)

    # Act
    child = await element.query_selector(".child")
    text = await element.inner_text()

    # Assert
    assert isinstance(child, ElementHandle)
    assert text == "child text"
    assert session.observed == [
        ("query_selector", owner, ".child"),
        ("inner_text", owner),
    ]


@pytest.mark.asyncio
async def test_element_handle_bounding_box_uses_exact_native_owner() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "div"

    class NativeSession:
        observed: Optional[object] = None

        async def element_handle_bounding_box(self, handle: object) -> dict[str, float]:
            self.observed = handle
            return {"x": -3.5, "y": 4.0, "width": 12.25, "height": 8.0}

    session = NativeSession()
    owner = cast(Any, NativeHandle())
    element = ElementHandle(cast(Any, session), owner)

    # Act
    box = await element.bounding_box()

    # Assert
    assert session.observed is owner
    assert box == {"x": -3.5, "y": 4.0, "width": 12.25, "height": 8.0}


@pytest.mark.asyncio
async def test_element_handle_bounding_box_preserves_native_none() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "div"

    class NativeSession:
        @staticmethod
        async def element_handle_bounding_box(_handle: object) -> None:
            return None

    element = ElementHandle(cast(Any, NativeSession()), cast(Any, NativeHandle()))

    # Act
    box = await element.bounding_box()

    # Assert
    assert box is None


def test_sync_element_handle_bounding_box_waits_for_native_geometry() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "div"

    class NativeSession:
        observed: Optional[object] = None

        async def element_handle_bounding_box(self, handle: object) -> dict[str, float]:
            self.observed = handle
            return {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}

    session = NativeSession()
    owner = cast(Any, NativeHandle())
    async_element = ElementHandle(cast(Any, session), owner)
    bridge = _LoopThread()
    element = SyncElementHandle(bridge, async_element)

    # Act
    try:
        box = element.bounding_box()
    finally:
        bridge.close()

    # Assert
    assert session.observed is owner
    assert box == {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}


@pytest.mark.asyncio
async def test_element_handle_query_selector_preserves_native_none() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def is_element() -> bool:
            return True

        @staticmethod
        def preview() -> str:
            return "div"

    class NativeSession:
        @staticmethod
        async def element_handle_query_selector(_handle: object, selector: str) -> None:
            assert selector == ".missing"
            return None

    element = ElementHandle(cast(Any, NativeSession()), cast(Any, NativeHandle()))

    # Act
    child = await element.query_selector(".missing")

    # Assert
    assert child is None


@pytest.mark.asyncio
async def test_locator_select_option_returns_native_selected_values() -> None:
    # Arrange
    class NativeLocator:
        observed: Optional[tuple[object, ...]] = None

        async def select_option(
            self, value: str, timeout_ms: int, force: Optional[bool]
        ) -> list[str]:
            self.observed = (value, timeout_ms, force)
            return [value]

    class PageOwner:
        @staticmethod
        def _effective_timeout(timeout: Optional[float], *, navigation: bool = False) -> int:
            assert navigation is False
            return 30_000 if timeout is None else int(timeout)

    native = NativeLocator()
    locator = Locator(cast(Any, PageOwner()), "select", native_locator=cast(Any, native))

    # Act
    selected = await locator.select_option("blue", timeout=76, force=True)

    # Assert
    assert native.observed == ("blue", 76, True)
    assert selected == ["blue"]


def test_locator_repr_matches_playwright_frame_and_selector_shape() -> None:
    # Arrange
    class PageOwner:
        pass

    page = cast(Any, PageOwner())
    page.main_frame = Frame(page, "frame-9", None, "", "https://example.test/page")
    locator = Locator(page, "button", native_locator=cast(Any, object()))

    # Act
    rendered = str(locator)

    # Assert
    assert rendered == (
        "<Locator frame=<Frame name= url='https://example.test/page'> selector='button'>"
    )


@pytest.mark.asyncio
async def test_page_request_listener_can_be_removed_without_leaving_a_python_subscription() -> None:
    # Arrange
    class Browser:
        _native = object()

        @staticmethod
        def is_connected() -> bool:
            return True

    class Context:
        browser = Browser()

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")
    observed: list[str] = []

    def listener(value: object) -> None:
        assert isinstance(value, Request)
        observed.append(f"{value.method}{value.url}")

    page.on("request", listener)
    first = Request(
        {
            "url": "https://example.test/first",
            "method": "GET",
            "resource_type": "fetch",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )
    second = Request(
        {
            "url": "https://example.test/second",
            "method": "POST",
            "resource_type": "fetch",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )

    # Act
    page._emit_page_event("request", first)
    page.remove_listener("request", listener)
    page._emit_page_event("request", second)

    # Assert
    assert observed == ["GEThttps://example.test/first"]


@pytest.mark.asyncio
async def test_request_notice_watcher_emits_requestfailed_with_the_native_failure() -> None:
    # Arrange
    class Native:
        calls = 0

        async def next_request_notice(self) -> tuple[str, int, dict[str, object]]:
            self.calls += 1
            if self.calls == 1:
                return (
                    "page-7",
                    1,
                    {
                        "kind": "requestfailed",
                        "id": "request-9",
                        "page_id": "page-7",
                        "url": "https://example.test/failure",
                        "method": "GET",
                        "resource_type": "document",
                        "headers": {},
                        "post_data": None,
                        "post_data_buffer": None,
                        "is_navigation_request": True,
                        "frame_available": True,
                        "failure_text": "net::ERR_INTERNET_DISCONNECTED",
                    },
                )
            raise pwright.TargetClosedError("browser closed")

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser
            self._pages: dict[str, Page] = {}

    browser = object.__new__(Browser)
    browser._native = cast(Any, Native())
    browser._closed = False
    browser._request_watch_error = None
    browser._request_notice_sequence = 0
    browser._network_notice_condition = asyncio.Condition()
    context = Context(browser)
    page = Page(cast(Any, context), "page-7", "frame-8", "about:blank")
    context._pages["page-7"] = page
    browser._contexts = {"context-6": cast(Any, context)}
    observed: list[Request] = []
    page.on("requestfailed", lambda request: observed.append(cast(Request, request)))

    # Act
    await browser._watch_request_notices()

    # Assert
    assert browser._request_watch_error is None
    assert len(observed) == 1
    assert observed[0].url == "https://example.test/failure"
    assert observed[0].failure == "net::ERR_INTERNET_DISCONNECTED"
    assert browser._request_notice_sequence == 1


@pytest.mark.asyncio
async def test_network_notice_flush_waits_for_both_published_streams() -> None:
    # Arrange
    class Native:
        @staticmethod
        def network_notice_fence() -> tuple[int, int]:
            return (2, 3)

    browser = object.__new__(Browser)
    browser._native = cast(Any, Native())
    browser._closed = False
    browser._request_watch_error = None
    browser._response_watch_error = None
    browser._request_notice_sequence = 1
    browser._response_notice_sequence = 2
    browser._network_notice_condition = asyncio.Condition()
    order: list[str] = []

    async def publish_remaining_notices() -> None:
        async with browser._network_notice_condition:
            browser._request_notice_sequence = 2
            order.append("request")
            browser._network_notice_condition.notify_all()
        await asyncio.sleep(0)
        async with browser._network_notice_condition:
            browser._response_notice_sequence = 3
            order.append("response")
            browser._network_notice_condition.notify_all()

    async def flush() -> None:
        await browser._flush_network_notices()
        order.append("flush")

    # Act
    await asyncio.gather(flush(), publish_remaining_notices())

    # Assert
    assert order == ["request", "response", "flush"]


def test_sync_page_request_listener_removes_the_exact_wrapped_callback() -> None:
    # Arrange
    class Browser:
        _native = object()

        @staticmethod
        def is_connected() -> bool:
            return True

    class Context:
        browser = Browser()

    async_page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))
    observed: list[str] = []

    def listener(value: object) -> None:
        assert isinstance(value, SyncRequest)
        observed.append(f"{value.method}{value.url}")

    first = Request(
        {
            "url": "https://example.test/first",
            "method": "GET",
            "resource_type": "fetch",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )
    second = Request(
        {
            "url": "https://example.test/second",
            "method": "POST",
            "resource_type": "fetch",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )

    async def emit(value: Request) -> None:
        async_page._emit_page_event("request", value)

    # Act
    try:
        page.on("request", listener)
        bridge.run(emit(first))
        page.remove_listener("request", listener)
        bridge.run(emit(second))
    finally:
        bridge.close()

    # Assert
    assert observed == ["GEThttps://example.test/first"]


@pytest.mark.asyncio
async def test_page_response_listener_can_be_removed_without_leaving_a_python_subscription() -> (
    None
):
    # Arrange
    class Browser:
        _native = object()

        @staticmethod
        def is_connected() -> bool:
            return True

    class Context:
        browser = Browser()

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")
    observed: list[str] = []

    def listener(value: object) -> None:
        assert isinstance(value, Response)
        observed.append(value.url)

    page.on("response", listener)
    first = Response(
        {
            "url": "https://example.test/first",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }
    )
    second = Response(
        {
            "url": "https://example.test/second",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }
    )

    # Act
    page._emit_page_event("response", first)
    page.remove_listener("response", listener)
    page._emit_page_event("response", second)

    # Assert
    assert observed == ["https://example.test/first"]


def test_sync_page_response_listener_removes_the_exact_wrapped_callback() -> None:
    # Arrange
    class Browser:
        _native = object()

        @staticmethod
        def is_connected() -> bool:
            return True

    class Context:
        browser = Browser()

    async_page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))
    observed: list[str] = []

    def listener(value: object) -> None:
        assert isinstance(value, SyncResponse)
        observed.append(value.url)

    first = Response(
        {
            "url": "https://example.test/first",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }
    )
    second = Response(
        {
            "url": "https://example.test/second",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }
    )

    async def emit(value: Response) -> None:
        async_page._emit_page_event("response", value)

    # Act
    try:
        page.on("response", listener)
        bridge.run(emit(first))
        page.remove_listener("response", listener)
        bridge.run(emit(second))
    finally:
        bridge.close()

    # Assert
    assert observed == ["https://example.test/first"]


def test_sync_page_listener_error_propagates_at_the_public_call_boundary() -> None:
    # Arrange
    class Browser:
        _native = object()

        @staticmethod
        def is_connected() -> bool:
            return True

    class Context:
        browser = Browser()

    async_page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))
    response = Response(
        {
            "url": "https://example.test/failure",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }
    )

    def fail(_value: object) -> None:
        raise RuntimeError("listener failed")

    async def emit() -> None:
        async_page._emit_page_event("response", response)

    # Act / Assert
    try:
        page.on("response", fail)
        with pytest.raises(RuntimeError, match="listener failed"):
            bridge.run(emit())

        async def succeeds() -> int:
            return 7

        assert bridge.run(succeeds()) == 7
    finally:
        bridge.close()


@pytest.mark.asyncio
async def test_page_viewport_delegates_validated_dimensions_to_native_rust() -> None:
    # Arrange
    class Native:
        observed: object = None

        async def page_set_viewport_size(self, page_id: str, width: int, height: int) -> None:
            self.observed = (page_id, width, height)

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    await page.set_viewport_size({"width": 500, "height": 400})

    # Assert
    assert native.observed == ("page-7", 500, 400)
    assert page.viewport_size == {"width": 500, "height": 400}


def test_page_viewport_size_reports_the_context_policy_as_a_copy() -> None:
    # Arrange
    class Context:
        _viewport_size = (456, 789)

    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")

    # Act
    viewport = page.viewport_size

    # Assert
    assert viewport == {"width": 456, "height": 789}
    assert viewport is not None
    viewport["width"] = 1
    assert page.viewport_size == {"width": 456, "height": 789}


@pytest.mark.asyncio
async def test_page_viewport_rejects_boolean_dimensions_before_native_work() -> None:
    # Arrange
    class Native:
        called = False

        async def page_set_viewport_size(self, _page_id: str, _width: int, _height: int) -> None:
            self.called = True

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    with pytest.raises(TypeError, match="viewport_size must contain integer width and height"):
        await page.set_viewport_size(cast(Any, {"width": True, "height": 400}))

    # Assert
    assert not native.called


def test_sync_page_viewport_waits_for_the_native_command() -> None:
    # Arrange
    class Native:
        observed: object = None

        async def page_set_viewport_size(self, page_id: str, width: int, height: int) -> None:
            self.observed = (page_id, width, height)

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    async_page = Page(cast(Any, Context(Browser(native))), "page-9", "frame-10", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, object()))

    # Act
    try:
        page.set_viewport_size({"width": 640, "height": 480})
    finally:
        bridge.close()

    # Assert
    assert native.observed == ("page-9", 640, 480)
    assert page.viewport_size == {"width": 640, "height": 480}


@pytest.mark.asyncio
async def test_page_errors_delegates_only_identity_and_converts_native_records() -> None:
    # Arrange
    class Native:
        observed_page_id: Optional[str] = None

        def page_errors(self, page_id: str) -> list[tuple[str, str, str]]:
            self.observed_page_id = page_id
            return [("TypeError", "broken", "TypeError: broken\n    at app.js:1:2")]

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    errors = await page.page_errors()

    # Assert
    assert native.observed_page_id == "page-7"
    assert len(errors) == 1
    assert errors[0].name == "TypeError"
    assert errors[0].message == "broken"
    assert errors[0].stack == "TypeError: broken\n    at app.js:1:2"


@pytest.mark.asyncio
async def test_console_history_delegates_only_identity_and_converts_native_records() -> None:
    # Arrange
    class Native:
        observed_page_id: Optional[str] = None

        def console_messages(self, page_id: str) -> list[tuple[str, str, str, str, int, int]]:
            self.observed_page_id = page_id
            return [("log", "hello", '["hello"]', "https://example.test/app.js", 4, 7)]

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    messages = await page.console_messages()

    # Assert
    assert native.observed_page_id == "page-7"
    assert len(messages) == 1
    assert messages[0].page is page
    assert messages[0].type == "log"
    assert messages[0].text == "hello"
    assert messages[0].location == {
        "url": "https://example.test/app.js",
        "lineNumber": 4,
        "columnNumber": 7,
    }


def test_sync_console_event_wraps_the_rust_lifecycle_descriptor() -> None:
    # Arrange
    bridge = _LoopThread()

    class Native:
        subscriptions: list[tuple[str, bool]] = []

        def set_page_console_listener(self, page_id: str, enabled: bool) -> None:
            self.subscriptions.append((page_id, enabled))

    native = Native()

    async def arrange() -> tuple[Browser, Page]:
        browser = object.__new__(Browser)
        browser._closed = False
        browser._contexts = {}
        browser._native = cast(Any, native)
        context = BrowserContext(browser, "context-1", None)
        page = Page(context, "page-2", "frame-3", "about:blank")
        context._pages["page-2"] = page
        browser._contexts["context-1"] = context
        return browser, page

    browser, async_page = bridge.run(arrange())
    page = SyncPage(bridge, async_page, cast(Any, None))
    observed: list[SyncConsoleMessage] = []

    # Act
    try:
        page.on("console", lambda message: observed.append(cast(SyncConsoleMessage, message)))

        async def emit() -> None:
            browser._receive_lifecycle_notice(
                6,
                "context-1",
                "page-2",
                None,
                None,
                "frame-3",
                "about:blank",
                None,
                (
                    "log",
                    "hello 7",
                    '["hello",7]',
                    "https://example.test/app.js",
                    4,
                    9,
                ),
            )

        bridge.run(emit())
    finally:
        bridge.close()

    # Assert
    assert len(observed) == 1
    assert isinstance(observed[0], SyncConsoleMessage)
    assert observed[0].page is page
    assert observed[0].type == "log"
    assert observed[0].text == "hello 7"
    assert observed[0].location == {
        "url": "https://example.test/app.js",
        "lineNumber": 4,
        "columnNumber": 9,
    }
    assert native.subscriptions == [("page-2", True)]
    assert repr(observed[0]) == "<ConsoleMessage type=log text=hello 7>"


def test_sync_page_expect_console_message_wraps_predicate_and_value() -> None:
    # Arrange
    bridge = _LoopThread()

    class Native:
        flushes = 0
        flush_action: Any = None
        subscriptions: list[tuple[str, object]] = []

        async def flush_lifecycle_events(self) -> None:
            self.flushes += 1
            if self.flush_action is not None:
                self.flush_action()

        def subscribe_page_console(self, page_id: str) -> int:
            self.subscriptions.append((page_id, "subscribe"))
            return 41

        def release_console_subscription(self, token: int) -> None:
            self.subscriptions.append(("release", token))

    native = Native()

    async def arrange() -> tuple[Browser, Page]:
        browser = object.__new__(Browser)
        browser._closed = False
        browser._contexts = {}
        browser._native = cast(Any, native)
        context = BrowserContext(browser, "context-1", None)
        page = Page(context, "page-2", "frame-3", "about:blank")
        context._pages["page-2"] = page
        browser._contexts["context-1"] = context
        return browser, page

    browser, async_page = bridge.run(arrange())
    page = SyncPage(bridge, async_page, cast(Any, None))
    observed_predicates: list[SyncConsoleMessage] = []
    native.flush_action = lambda: browser._receive_lifecycle_notice(
        6,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        ("error", "stale", '["stale"]', "", 0, 0),
    )

    # Act
    try:
        with page.expect_console_message(
            lambda message: observed_predicates.append(cast(SyncConsoleMessage, message)) is None,
            timeout=0,
        ) as message_info:

            async def emit() -> None:
                browser._receive_lifecycle_notice(
                    6,
                    "context-1",
                    "page-2",
                    None,
                    None,
                    "frame-3",
                    "about:blank",
                    None,
                    ("error", "late stale", '["late stale"]', "", 0, 0, 1),
                )
                browser._receive_lifecycle_notice(
                    6,
                    "context-1",
                    "page-2",
                    None,
                    None,
                    "frame-3",
                    "about:blank",
                    None,
                    (
                        "log",
                        "hello",
                        '["hello"]',
                        "",
                        0,
                        0,
                        9_000_000_000_000_000,
                    ),
                )

            bridge.run(emit())
        message = cast(SyncConsoleMessage, message_info.value)
    finally:
        bridge.close()

    # Assert
    assert len(observed_predicates) == 1
    assert native.flushes == 1
    assert native.subscriptions == [("page-2", "subscribe"), ("release", 41)]
    assert isinstance(observed_predicates[0], SyncConsoleMessage)
    assert isinstance(message, SyncConsoleMessage)
    assert message.page is page
    assert message.text == "hello"


def test_sync_context_expect_console_message_wraps_predicate_and_value() -> None:
    # Arrange
    bridge = _LoopThread()

    class Native:
        flushes = 0
        subscriptions: list[tuple[str, object]] = []

        async def flush_lifecycle_events(self) -> None:
            self.flushes += 1

        def subscribe_context_console(self, context_id: str) -> int:
            self.subscriptions.append((context_id, "subscribe"))
            return 42

        def release_console_subscription(self, token: int) -> None:
            self.subscriptions.append(("release", token))

    native = Native()

    async def arrange() -> tuple[Browser, BrowserContext]:
        browser = object.__new__(Browser)
        browser._closed = False
        browser._contexts = {}
        browser._native = cast(Any, native)
        context = BrowserContext(browser, "context-1", None)
        page = Page(context, "page-2", "frame-3", "about:blank")
        context._pages["page-2"] = page
        browser._contexts["context-1"] = context
        return browser, context

    browser, async_context = bridge.run(arrange())
    context = SyncBrowserContext(bridge, async_context, cast(Any, None))

    # Act
    try:
        with context.expect_console_message(
            lambda message: isinstance(message, SyncConsoleMessage), timeout=0
        ) as message_info:

            async def emit() -> None:
                browser._receive_lifecycle_notice(
                    6,
                    "context-1",
                    "page-2",
                    None,
                    None,
                    "frame-3",
                    "about:blank",
                    None,
                    (
                        "warning",
                        "careful",
                        '["careful"]',
                        "",
                        0,
                        0,
                        9_000_000_000_000_000,
                    ),
                )

            bridge.run(emit())
        message = cast(SyncConsoleMessage, message_info.value)
    finally:
        bridge.close()

    # Assert
    assert native.flushes == 1
    assert native.subscriptions == [("context-1", "subscribe"), ("release", 42)]
    assert isinstance(message, SyncConsoleMessage)
    assert message.page.context is context
    assert message.type == "warning"
    assert message.text == "careful"


@pytest.mark.asyncio
async def test_request_history_keeps_native_owners_for_response_and_body() -> None:
    # Arrange
    history_owner = object()
    body_owner = object()

    class Native:
        observed_page_id: Optional[str] = None
        observed_history_owner: object = None
        observed_body_owner: object = None

        def page_requests(self, page_id: str) -> list[dict[str, object]]:
            self.observed_page_id = page_id
            return [
                {
                    "id": "request-1",
                    "url": "https://example.test/fetch?1",
                    "method": "GET",
                    "resource_type": "fetch",
                    "headers": {},
                    "post_data": None,
                    "post_data_buffer": None,
                    "is_navigation_request": False,
                    "sizes_owner": None,
                    "history_owner": history_owner,
                }
            ]

        async def request_history_response(self, owner: object) -> dict[str, object]:
            self.observed_history_owner = owner
            return {
                "url": "https://example.test/fetch?1",
                "status": 200,
                "status_text": "OK",
                "ok": True,
                "headers": {},
                "from_service_worker": False,
                "body_owner": body_owner,
            }

        async def response_body(self, owner: object) -> bytes:
            self.observed_body_owner = owner
            return b"url:https://example.test/fetch?1"

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    class Context:
        def __init__(self, browser: Browser) -> None:
            self.browser = browser

    native = Native()
    page = Page(cast(Any, Context(Browser(native))), "page-7", "frame-8", "about:blank")

    # Act
    requests = await page.requests()
    response = await requests[0].response()
    assert response is not None
    text = await response.text()

    # Assert
    assert native.observed_page_id == "page-7"
    assert native.observed_history_owner is history_owner
    assert native.observed_body_owner is body_owner
    assert requests[0].url == "https://example.test/fetch?1"
    assert response.request is requests[0]
    assert text == "url:https://example.test/fetch?1"


def test_response_rehydrates_its_native_request_association() -> None:
    # Arrange
    request_value: dict[str, Any] = {
        "url": "https://example.test/fetch",
        "method": "POST",
        "resource_type": "fetch",
        "headers": {"content-type": "text/plain"},
        "post_data": "hello",
        "post_data_buffer": b"hello",
        "is_navigation_request": False,
        "sizes_owner": None,
        "history_owner": object(),
    }
    response = Response(
        {
            "url": "https://example.test/fetch",
            "status": 201,
            "status_text": "Created",
            "ok": True,
            "headers": {},
            "request": request_value,
        }
    )

    # Act
    request = response.request

    # Assert
    assert request.url == "https://example.test/fetch"
    assert request.method == "POST"
    assert request.post_data == "hello"


def test_response_timing_merges_into_the_existing_request_identity() -> None:
    # Arrange
    request = Request(
        {
            "url": "https://example.test/timed",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
            "timing": {
                "startTime": 0.0,
                "domainLookupStart": -1.0,
                "domainLookupEnd": -1.0,
                "connectStart": -1.0,
                "secureConnectionStart": -1.0,
                "connectEnd": -1.0,
                "requestStart": -1.0,
                "responseStart": -1.0,
                "responseEnd": -1.0,
            },
        }
    )
    response_timing = {
        "startTime": 1_700_000_000_000.5,
        "domainLookupStart": 0.1,
        "domainLookupEnd": 0.2,
        "connectStart": 0.2,
        "secureConnectionStart": 0.3,
        "connectEnd": 0.5,
        "requestStart": 0.6,
        "responseStart": 2.5,
        "responseEnd": -1.0,
    }

    # Act
    response = Response(
        {
            "url": "https://example.test/timed",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
            "timing": response_timing,
        },
        request=request,
    )

    # Assert
    assert response.request is request
    assert request.timing == response_timing
    returned = request.timing
    returned["responseStart"] = 99.0
    assert request.timing["responseStart"] == 2.5


def test_terminal_request_snapshot_replaces_response_end_timing() -> None:
    # Arrange
    request = Request(
        {
            "url": "https://example.test/timed",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
            "timing": {
                "startTime": 1_700_000_000_000.5,
                "domainLookupStart": 0.1,
                "domainLookupEnd": 0.2,
                "connectStart": 0.2,
                "secureConnectionStart": 0.3,
                "connectEnd": 0.5,
                "requestStart": 0.6,
                "responseStart": 2.5,
                "responseEnd": -1.0,
            },
        }
    )

    # Act
    request._merge(
        {
            "timing": {
                **request.timing,
                "responseEnd": 5.0,
            }
        },
        None,
        None,
    )

    # Assert
    assert request.timing["responseEnd"] == 5.0
    assert request.timing["responseEnd"] >= request.timing["responseStart"]


def test_stale_request_notice_cannot_erase_completed_timing() -> None:
    # Arrange
    completed = {
        "startTime": 1_700_000_000_000.5,
        "domainLookupStart": 0.1,
        "domainLookupEnd": 0.2,
        "connectStart": 0.2,
        "secureConnectionStart": 0.3,
        "connectEnd": 0.5,
        "requestStart": 0.6,
        "responseStart": 2.5,
        "responseEnd": 5.0,
    }
    request = Request(
        {
            "url": "https://example.test/timed",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
            "timing": completed,
        }
    )

    # Act
    request._merge(
        {
            "timing": {
                "startTime": 0.0,
                "domainLookupStart": -1.0,
                "domainLookupEnd": -1.0,
                "connectStart": -1.0,
                "secureConnectionStart": -1.0,
                "connectEnd": -1.0,
                "requestStart": -1.0,
                "responseStart": -1.0,
                "responseEnd": -1.0,
            }
        },
        None,
        None,
    )

    # Assert
    assert request.timing == completed


def test_sync_request_timing_delegates_to_the_async_snapshot() -> None:
    # Arrange
    request = Request(
        {
            "url": "https://example.test/timed",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
            "timing": {
                "startTime": 1.0,
                "domainLookupStart": -1.0,
                "domainLookupEnd": -1.0,
                "connectStart": -1.0,
                "secureConnectionStart": -1.0,
                "connectEnd": -1.0,
                "requestStart": 2.0,
                "responseStart": 3.0,
                "responseEnd": 4.0,
            },
        }
    )
    sync_request = SyncRequest(cast(Any, object()), request)

    # Act
    timing = sync_request.timing

    # Assert
    assert timing["startTime"] == 1.0
    assert timing["responseEnd"] == 4.0


def test_navigation_response_reuses_the_rust_request_history_identity() -> None:
    # Arrange
    class Native:
        observed_page_id: Optional[str] = None

        def page_requests(self, page_id: str) -> list[dict[str, object]]:
            self.observed_page_id = page_id
            return [
                {
                    "id": "request-9",
                    "native_request_id": "loader-9",
                    "page_id": page_id,
                    "frame_available": True,
                    "url": "https://example.test/navigation",
                    "method": "GET",
                    "resource_type": "document",
                    "headers": {},
                    "post_data": None,
                    "post_data_buffer": None,
                    "is_navigation_request": True,
                    "sizes_owner": None,
                    "history_owner": object(),
                }
            ]

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        _object_id = "context-6"

        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser
            self._pages: dict[str, Page] = {}

    native = Native()
    context = cast(Any, ContextOwner(BrowserOwner(native)))
    page = Page(context, "page-7", "frame-8", "about:blank")
    context._pages["page-7"] = page

    # Act
    response = page._navigation_response(
        {
            "request_id": "loader-9",
            "url": "https://example.test/navigation",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }
    )

    # Assert
    assert native.observed_page_id == "page-7"
    assert response.request.url == "https://example.test/navigation"
    assert response.request.failure is None
    assert response.request is page._requests["request-9"]


def test_page_navigation_response_has_main_frame_without_request_history(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    class BrowserOwner:
        _native = object()

    class ContextOwner:
        browser = BrowserOwner()

    page = Page(cast(Any, ContextOwner()), "page-7", "frame-8", "about:blank")
    main_frame = page._frames["frame-8"]
    monkeypatch.setattr(_native, "selected_features", lambda: list[str]())

    # Act
    response = page._navigation_response(
        {
            "url": "https://example.test/navigation",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        },
    )

    # Assert
    assert response.frame is main_frame
    with pytest.raises(pwright.UnsupportedError, match="Response.request capability"):
        _ = response.request


def test_navigation_response_selects_the_final_request_and_links_redirects() -> None:
    # Arrange
    class Native:
        def page_requests(self, page_id: str) -> list[dict[str, object]]:
            values: list[dict[str, object]] = []
            for index in range(1, 6):
                values.append(
                    {
                        "id": f"request-{index}",
                        "native_request_id": "loader-9",
                        "page_id": page_id,
                        "frame_available": True,
                        "url": f"https://example.test/redirect-{index}",
                        "method": "GET",
                        "resource_type": "document",
                        "headers": {},
                        "post_data": None,
                        "post_data_buffer": None,
                        "is_navigation_request": True,
                        "redirected_from_id": None if index == 1 else f"request-{index - 1}",
                        "sizes_owner": None,
                        "history_owner": object(),
                    }
                )
            return values

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        _object_id = "context-6"

        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser
            self._pages: dict[str, Page] = {}

    context = cast(Any, ContextOwner(BrowserOwner(Native())))
    page = Page(context, "page-7", "frame-8", "about:blank")
    context._pages["page-7"] = page

    # Act
    response = page._navigation_response(
        {
            "request_id": "loader-9",
            "url": "https://example.test/redirect-5",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        }
    )

    # Assert
    assert response.request is page._requests["request-5"]
    assert response.request.redirected_from is page._requests["request-4"]
    assert page._requests["request-1"].redirected_to is page._requests["request-2"]
    assert page._requests["request-5"].redirected_to is None


def test_page_network_identity_map_is_weak_and_shared_by_response_request() -> None:
    # Arrange
    class Native:
        pass

    class BrowserOwner:
        def __init__(self) -> None:
            self._native = Native()

    class ContextOwner:
        def __init__(self) -> None:
            self.browser = BrowserOwner()
            self._pages: dict[str, Page] = {}

    context = cast(Any, ContextOwner())
    page = Page(context, "page-7", "frame-8", "about:blank")
    context._pages["page-7"] = page
    request_value: dict[str, Any] = {
        "id": "request-10",
        "page_id": "page-7",
        "frame_available": False,
        "url": "https://example.test/worker",
        "method": "GET",
        "resource_type": "fetch",
        "headers": {},
        "post_data": None,
        "post_data_buffer": None,
        "is_navigation_request": False,
        "sizes_owner": None,
        "history_owner": None,
    }

    # Act
    request = page._wrap_request(request_value)
    response = page._wrap_response(
        {
            "id": "response-11",
            "request_id": "request-10",
            "url": "https://example.test/worker",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
            "from_service_worker": False,
            "request": request_value,
        }
    )

    # Assert
    assert response.request is request
    assert len(page._requests) == 1
    assert len(page._responses) == 1


def test_string_argument_reports_playwright_number_type() -> None:
    # Arrange
    value = 123

    # Act
    with pytest.raises(pwright.Error) as raised:
        _string_argument(value, "Page.fill")

    # Assert
    assert "Page.fill: expected string, got number" in str(raised.value)


def test_drag_arguments_preserve_finite_points_and_integer_steps() -> None:
    # Arrange
    position = {"x": 34, "y": 7.5}

    # Act
    converted_position = _drag_position(position, "source_position")
    converted_steps = _drag_steps(4)

    # Assert
    assert converted_position == (34.0, 7.5)
    assert converted_steps == 4.0


def test_drag_arguments_reject_boolean_steps_before_native_work() -> None:
    # Arrange
    steps = True

    # Act
    with pytest.raises(TypeError, match="steps must be an integer"):
        _drag_steps(steps)

    # Assert
    assert steps is True


@pytest.mark.asyncio
async def test_route_fulfill_normalizes_public_values_before_the_native_call() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_route_fulfill(
            self,
            call_id: str,
            status: int,
            headers: list[tuple[str, str]],
            body: bytes,
        ) -> None:
            self.call = (call_id, status, headers, body)

    native = Native()
    request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
        }
    )
    route = Route(native, "7", request)  # type: ignore[arg-type]

    # Act
    await route.fulfill(
        status=201,
        headers={"Content-Type": "ignored", "x-test": "yes"},
        content_type="text/plain",
        body="héllo",
    )

    # Assert
    assert native.call == (
        "7",
        201,
        [("x-test", "yes"), ("content-type", "text/plain")],
        "héllo".encode(),
    )


@pytest.mark.asyncio
async def test_route_fulfill_response_keeps_the_retained_body_in_rust() -> None:
    # Arrange
    class ResponseNative:
        def body(self) -> bytes:
            raise AssertionError("Python must not copy an APIResponse body for Route.fulfill")

    class Native:
        call: object = None

        async def page_route_fulfill_response(
            self,
            call_id: str,
            response: object,
            status: Optional[int],
            headers: Optional[list[tuple[str, str]]],
            content_type: Optional[str],
        ) -> None:
            self.call = (call_id, response, status, headers, content_type)

    response_native = ResponseNative()
    response = AsyncAPIResponse(cast(Any, response_native))
    native = Native()
    request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
        }
    )
    route = Route(native, "9", request)  # type: ignore[arg-type]

    # Act
    await route.fulfill(response=response, content_type="application/custom")

    # Assert
    assert native.call == (
        "9",
        response_native,
        None,
        None,
        "application/custom",
    )
    assert route._handled


@pytest.mark.asyncio
async def test_route_fulfill_path_delegates_file_loading_to_rust() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_route_fulfill_path(
            self,
            call_id: str,
            status: int,
            headers: list[tuple[str, str]],
            path: str,
        ) -> None:
            self.call = (call_id, status, headers, path)

    native = Native()
    request = Request(
        {
            "url": "https://example.test/image.png",
            "method": "GET",
            "resource_type": "image",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )
    route = Route(native, "8", request)  # type: ignore[arg-type]

    # Act
    await route.fulfill(
        status=202,
        headers={"x-test": "yes"},
        content_type="shouldBeIgnored",
        path=Path("fixture.png"),
    )

    # Assert
    assert native.call == (
        "8",
        202,
        [("x-test", "yes"), ("content-type", "shouldBeIgnored")],
        "fixture.png",
    )
    assert route._handled


@pytest.mark.asyncio
async def test_route_continue_normalizes_every_override_before_the_native_call() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_route_continue(
            self,
            call_id: str,
            url: Optional[str],
            method: Optional[str],
            headers: Optional[list[tuple[str, str]]],
            post_data: Optional[bytes],
        ) -> None:
            self.call = (call_id, url, method, headers, post_data)

    native = Native()
    request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
        }
    )
    route = Route(native, "9", request)  # type: ignore[arg-type]

    # Act
    await route.continue_(
        url="https://example.test/next",
        method="POST",
        headers={"x-test": "yes", "remove-me": None},
        post_data={"hello": "world"},
    )

    # Assert
    assert native.call == (
        "9",
        "https://example.test/next",
        "POST",
        [("x-test", "yes")],
        b'{"hello": "world"}',
    )


@pytest.mark.asyncio
async def test_page_route_from_har_delegates_path_and_policy_to_rust() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_route_from_har(
            self,
            page_id: str,
            path: str,
            pattern: Optional[str],
            not_found: Optional[str],
        ) -> str:
            self.call = (page_id, path, pattern, not_found)
            return "12"

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)
            self._pages: dict[str, Page] = {}

    native = Native()
    context = cast(Any, ContextOwner(native))
    page = Page(context, "page-7", "frame-8", "about:blank")

    # Act
    await page.route_from_har(
        Path("fixtures/replay.har"),
        url="**/*.js",
        not_found="fallback",
    )

    # Assert
    assert native.call == ("page-7", "fixtures/replay.har", "**/*.js", "fallback")


@pytest.mark.asyncio
async def test_context_route_from_har_delegates_path_and_policy_to_rust() -> None:
    # Arrange
    class Native:
        call: object = None

        async def context_route_from_har(
            self,
            context_id: str,
            path: str,
            pattern: Optional[str],
            not_found: Optional[str],
        ) -> str:
            self.call = (context_id, path, pattern, not_found)
            return "13"

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-7", None)

    # Act
    await context.route_from_har(
        Path("fixtures/replay.har"),
        url="**/*.js",
        not_found="fallback",
    )

    # Assert
    assert native.call == ("context-7", "fixtures/replay.har", "**/*.js", "fallback")


@pytest.mark.asyncio
async def test_context_route_registers_handler_by_native_registration() -> None:
    # Arrange
    class Native:
        call: object = None

        async def context_route(self, context_id: str, pattern: str, times: Optional[int]) -> str:
            self.call = (context_id, pattern, times)
            return "14"

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-8", None)

    async def handler(_route: object) -> None:
        return None

    # Act
    await context.route("**/*.js", handler, times=2)

    # Assert
    assert native.call == ("context-8", "**/*.js", 2)
    assert context._routes == {"14": handler}


@pytest.mark.asyncio
async def test_page_set_extra_http_headers_delegates_owned_entries_to_rust() -> None:
    # Arrange
    class Native:
        call: object = None

        async def set_page_extra_http_headers(
            self, page_id: str, headers: list[tuple[str, str]]
        ) -> None:
            self.call = (page_id, headers)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)

    native = Native()
    page = Page(cast(Any, ContextOwner(native)), "page-headers", "frame-headers", "about:blank")

    # Act
    await page.set_extra_http_headers({"foo": "bar", "x-sequence": "two"})

    # Assert
    assert native.call == (
        "page-headers",
        [("foo", "bar"), ("x-sequence", "two")],
    )


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("headers", "location"),
    [
        ({"foo": 1}, "headers[0].value"),
        ({"foo": "bar", "x-count": 1}, "headers[1].value"),
    ],
)
async def test_page_set_extra_http_headers_reports_indexed_value_error_before_native(
    headers: dict[str, object], location: str
) -> None:
    # Arrange
    class Native:
        called = False

        async def set_page_extra_http_headers(self, *_args: object) -> None:
            self.called = True

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)

    native = Native()
    page = Page(cast(Any, ContextOwner(native)), "page-headers", "frame-headers", "about:blank")

    # Act
    with pytest.raises(pwright.Error) as failure:
        await page.set_extra_http_headers(cast(Any, headers))

    # Assert
    assert str(failure.value) == (
        f"Page.set_extra_http_headers: {location}: expected string, got number"
    )
    assert not native.called


@pytest.mark.asyncio
async def test_context_set_extra_http_headers_delegates_owned_entries_to_rust() -> None:
    # Arrange
    class Native:
        call: object = None

        async def set_context_extra_http_headers(
            self, context_id: str, headers: list[tuple[str, str]]
        ) -> None:
            self.call = (context_id, headers)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-headers", None)

    # Act
    await context.set_extra_http_headers({"referer": "https://example.test/"})

    # Assert
    assert native.call == (
        "context-headers",
        [("referer", "https://example.test/")],
    )


@pytest.mark.asyncio
async def test_context_set_extra_http_headers_keeps_existing_value_error_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def set_context_extra_http_headers(self, *_args: object) -> None:
            self.called = True

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-headers", None)

    # Act
    with pytest.raises(pwright.Error) as failure:
        await context.set_extra_http_headers(cast(Any, {"foo": 1}))

    # Assert
    assert str(failure.value) == (
        "BrowserContext.set_extra_http_headers header value: expected string, got number"
    )
    assert not native.called


@pytest.mark.asyncio
async def test_page_set_extra_http_headers_rejects_excess_entries_before_native() -> None:
    # Arrange
    class Native:
        called = False

        async def set_page_extra_http_headers(self, *_args: object) -> None:
            self.called = True

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)

    native = Native()
    page = Page(cast(Any, ContextOwner(native)), "page-headers", "frame-headers", "about:blank")
    headers = {f"x-{index}": "value" for index in range(257)}

    # Act
    with pytest.raises(
        pwright.ResourceExhaustedError,
        match="Page.set_extra_http_headers: extra header count exceeds 256",
    ):
        await page.set_extra_http_headers(headers)

    # Assert
    assert not native.called


def test_sync_page_set_extra_http_headers_preserves_mapping_entries() -> None:
    # Arrange
    class Native:
        call: object = None

        async def set_page_extra_http_headers(
            self, page_id: str, headers: list[tuple[str, str]]
        ) -> None:
            self.call = (page_id, headers)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)

    native = Native()
    async_page = Page(
        cast(Any, ContextOwner(native)),
        "page-sync-headers",
        "frame-sync-headers",
        "about:blank",
    )
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))

    # Act
    try:
        page.set_extra_http_headers({"foo": "bar"})
    finally:
        bridge.close()

    # Assert
    assert native.call == ("page-sync-headers", [("foo", "bar")])


@pytest.mark.asyncio
async def test_page_unroute_removes_only_the_matching_handler_registration() -> None:
    # Arrange
    class Native:
        removed: object = None
        next_registration = 10

        async def page_route(self, page_id: str, pattern: str, times: Optional[int]) -> str:
            registration = self.next_registration
            self.next_registration += 1
            return str(registration)

        async def page_unroute(self, page_id: str, registration_ids: list[int]) -> None:
            self.removed = (page_id, registration_ids)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)
            self._pages: dict[str, Page] = {}

    native = Native()
    page = Page(cast(Any, ContextOwner(native)), "page-10", "frame-10", "about:blank")

    async def first(_route: object) -> None:
        return None

    async def second(_route: object) -> None:
        return None

    await page.route("**/*.js", first)
    await page.route("**/*.js", second)
    await page.route("**/*.css", first)

    # Act
    await page.unroute("**/*.js", first)

    # Assert
    assert native.removed == ("page-10", [10])
    assert page._routes == {"11": second, "12": first}
    assert page._route_patterns == {"11": "**/*.js", "12": "**/*.css"}


@pytest.mark.asyncio
async def test_page_regex_route_registers_a_binding_filter_and_unroutes_by_pattern() -> None:
    # Arrange
    class Native:
        registered: object = None
        removed: object = None

        async def page_route(self, page_id: str, pattern: str, times: Optional[int]) -> str:
            self.registered = (page_id, pattern, times)
            return "21"

        async def page_unroute(self, page_id: str, registration_ids: list[int]) -> None:
            self.removed = (page_id, registration_ids)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)
            self._pages: dict[str, Page] = {}

    native = Native()
    page = Page(cast(Any, ContextOwner(native)), "page-21", "frame-21", "about:blank")
    pattern = re.compile("empty\\.html")

    def handler(_route: Route) -> None:
        return None

    await page.route(pattern, handler)

    # Act
    await page.unroute(re.compile("empty\\.html"))

    # Assert
    assert native.registered == ("page-21", "**/*", None)
    assert native.removed == ("page-21", [21])
    assert page._routes == {}


@pytest.mark.asyncio
async def test_page_predicate_route_falls_back_when_the_url_does_not_match() -> None:
    # Arrange
    class Native:
        fallback: object = None

        async def page_route(self, _page_id: str, _pattern: str, _times: Optional[int]) -> str:
            return "22"

        async def page_route_fallback(
            self,
            call_id: str,
            url: Optional[str],
            method: Optional[str],
            headers: Optional[list[tuple[str, str]]],
            post_data: Optional[bytes],
        ) -> None:
            self.fallback = (call_id, url, method, headers, post_data)

    native = Native()
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-22", None)
    page = Page(context, "page-22", "frame-22", "about:blank")
    browser._contexts = {"context-22": context}
    context._pages = {"page-22": page}
    observed: list[str] = []

    def handler(route: Route) -> None:
        observed.append(route.request.url)

    await page.route(
        lambda url: url.endswith("/match"),
        handler,
    )

    # Act
    await browser._dispatch_page_route(
        (
            "call-22",
            "22",
            "context-22",
            "page-22",
            "frame-22",
            "https://example.test/miss",
            "GET",
            "document",
            [],
            None,
            True,
            False,
            None,
        )
    )

    # Assert
    assert observed == []
    assert native.fallback == ("call-22", None, None, None, None)
    assert "22" in page._routes


@pytest.mark.asyncio
async def test_page_binding_route_times_counts_matches_and_retires_after_handling() -> None:
    # Arrange
    class Native:
        calls: list[object]

        def __init__(self) -> None:
            self.calls = []

        async def page_route(self, page_id: str, pattern: str, times: Optional[int]) -> str:
            self.calls.append(("route", page_id, pattern, times))
            return "23"

        async def page_route_continue(
            self,
            call_id: str,
            url: Optional[str],
            method: Optional[str],
            headers: Optional[list[tuple[str, str]]],
            post_data: Optional[bytes],
        ) -> None:
            self.calls.append(("continue", call_id, url, method, headers, post_data))

        async def page_unroute(self, page_id: str, registration_ids: list[int]) -> None:
            self.calls.append(("unroute", page_id, registration_ids))

    native = Native()
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-23", None)
    page = Page(context, "page-23", "frame-23", "about:blank")
    browser._contexts = {"context-23": context}
    context._pages = {"page-23": page}

    def handler(route: Route) -> None:
        asyncio.create_task(route.continue_())

    await page.route(re.compile("empty\\.html"), handler, times=1)

    # Act
    await browser._dispatch_page_route(
        (
            "call-23",
            "23",
            "context-23",
            "page-23",
            "frame-23",
            "https://example.test/empty.html",
            "GET",
            "document",
            [],
            None,
            True,
            False,
            None,
        )
    )

    # Assert
    assert native.calls == [
        ("route", "page-23", "**/*", None),
        ("continue", "call-23", None, None, None, None),
        ("unroute", "page-23", [23]),
    ]
    assert page._routes == {}


@pytest.mark.asyncio
async def test_page_route_handler_can_return_without_handling_the_route() -> None:
    # Arrange
    class Native:
        pass

    native = Native()
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-unhandled", None)
    page = Page(context, "page-unhandled", "frame-unhandled", "about:blank")
    browser._contexts = {"context-unhandled": context}
    context._pages = {"page-unhandled": page}
    observed: list[str] = []

    def handler(route: Route) -> None:
        observed.append(route.request.url)

    page._routes["unhandled"] = handler

    # Act
    await asyncio.wait_for(
        browser._dispatch_page_route(
            (
                "call-unhandled",
                "unhandled",
                "context-unhandled",
                "page-unhandled",
                "frame-unhandled",
                "https://example.test/unhandled",
                "GET",
                "document",
                [],
                None,
                True,
                False,
                None,
            )
        ),
        timeout=0.1,
    )

    # Assert
    assert observed == ["https://example.test/unhandled"]


@pytest.mark.asyncio
async def test_routed_request_response_uses_the_exact_native_network_identity() -> None:
    # Arrange
    history_owner = object()

    class Native:
        calls: list[object]

        def __init__(self) -> None:
            self.calls = []

        async def page_route_continue(
            self,
            call_id: str,
            url: Optional[str],
            method: Optional[str],
            headers: Optional[list[tuple[str, str]]],
            post_data: Optional[bytes],
        ) -> None:
            self.calls.append(("continue", call_id, url, method, headers, post_data))

        async def request_by_native_id(
            self, page_id: str, native_request_id: str
        ) -> dict[str, object]:
            self.calls.append(("request", page_id, native_request_id))
            return {
                "id": "request-24",
                "native_request_id": native_request_id,
                "page_id": page_id,
                "frame_id": "frame-24",
                "frame_available": True,
                "url": "https://example.test/fonts?helvetica|arial",
                "method": "GET",
                "resource_type": "stylesheet",
                "headers": {},
                "post_data": None,
                "post_data_buffer": None,
                "is_navigation_request": False,
                "redirected_from_id": None,
                "sizes_owner": None,
                "history_owner": history_owner,
            }

        async def request_history_response(self, owner: object) -> dict[str, object]:
            self.calls.append(("response", owner))
            return {
                "id": "response-24",
                "request_id": "request-24",
                "url": "https://example.test/fonts?helvetica|arial",
                "status": 404,
                "status_text": "Not Found",
                "ok": False,
                "headers": {},
                "from_service_worker": False,
                "body_owner": None,
                "finished_owner": None,
            }

    native = Native()
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-24", None)
    page = Page(context, "page-24", "frame-24", "about:blank")
    browser._contexts = {"context-24": context}
    context._pages = {"page-24": page}
    observed: dict[str, object] = {}

    async def handler(route: Route) -> None:
        await route.continue_()
        observed["request"] = route.request
        observed["response"] = await route.request.response()

    page._routes["24"] = handler

    # Act
    await browser._dispatch_page_route(
        (
            "call-24",
            "24",
            "context-24",
            "page-24",
            "frame-24",
            "https://example.test/fonts?helvetica|arial",
            "GET",
            "stylesheet",
            [],
            None,
            False,
            False,
            "network-24",
        )
    )

    # Assert
    request = cast(Request, observed["request"])
    response = cast(Response, observed["response"])
    assert native.calls == [
        ("request", "page-24", "network-24"),
        ("continue", "call-24", None, None, None, None),
        ("response", history_owner),
    ]
    assert response.status == 404
    assert response.request is request


def test_sync_page_unroute_preserves_original_handler_identity() -> None:
    # Arrange
    class Native:
        removed: object = None

        async def page_route(self, page_id: str, pattern: str, times: Optional[int]) -> str:
            return "17"

        async def page_unroute(self, page_id: str, registration_ids: list[int]) -> None:
            self.removed = (page_id, registration_ids)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)
            self._pages: dict[str, Page] = {}

    native = Native()
    async_page = Page(cast(Any, ContextOwner(native)), "page-17", "frame-17", "about:blank")
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))

    def handler(_route: object) -> None:
        return None

    # Act
    try:
        page.route("**/*", handler)
        page.unroute("**/*", handler)
    finally:
        bridge.close()

    # Assert
    assert native.removed == ("page-17", [17])
    assert async_page._routes == {}


@pytest.mark.asyncio
async def test_route_fallback_delegates_composable_overrides_to_rust() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_route_fallback(
            self,
            call_id: str,
            url: Optional[str],
            method: Optional[str],
            headers: Optional[list[tuple[str, str]]],
            post_data: Optional[bytes],
        ) -> None:
            self.call = (call_id, url, method, headers, post_data)

    native = Native()
    request = Request(
        {
            "url": "https://example.test/original",
            "method": "GET",
            "resource_type": "fetch",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )
    route = Route(cast(Any, native), "call-18", request)

    # Act
    await route.fallback(
        url="https://example.test/next",
        method="POST",
        headers={"x-test": "yes", "x-drop": None},
        post_data={"ok": True},
    )

    # Assert
    assert native.call == (
        "call-18",
        "https://example.test/next",
        "POST",
        [("x-test", "yes")],
        b'{"ok": true}',
    )
    assert route._handled


@pytest.mark.asyncio
async def test_expiring_native_route_call_releases_the_python_handler() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_route_continue(
            self,
            call_id: str,
            url: Optional[str],
            method: Optional[str],
            headers: Optional[list[tuple[str, str]]],
            post_data: Optional[bytes],
        ) -> None:
            self.call = (call_id, url, method, headers, post_data)

    class PageOwner:
        def __init__(self) -> None:
            self._routes: dict[str, Callable[..., object]] = {}

    class ContextOwner:
        def __init__(self, page: PageOwner) -> None:
            self._pages = {"page-1": page}
            self._routes: dict[str, Callable[..., object]] = {}

    observed: list[str] = []

    async def handler(route: Route) -> None:
        observed.append(route.request.url)
        await route.continue_()

    native = Native()
    page = PageOwner()
    page._routes["7"] = handler
    context = ContextOwner(page)
    browser = object.__new__(Browser)
    browser._native = cast(Any, native)
    browser._contexts = {"context-1": cast(Any, context)}
    call: tuple[
        str,
        str,
        str,
        str,
        Optional[str],
        str,
        str,
        str,
        list[tuple[str, str]],
        Optional[bytes],
        bool,
        bool,
        Optional[str],
    ] = (
        "call-1",
        "7",
        "context-1",
        "page-1",
        "frame-1",
        "https://example.test/empty.html",
        "GET",
        "document",
        [],
        None,
        True,
        True,
        None,
    )

    # Act
    await browser._dispatch_page_route(call)

    # Assert
    assert observed == ["https://example.test/empty.html"]
    assert page._routes == {}
    assert native.call == ("call-1", None, None, None, None)


@pytest.mark.asyncio
async def test_page_unroute_all_wait_delegates_and_releases_handlers() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_unroute_all(self, page_id: str, wait: bool) -> None:
            self.call = (page_id, wait)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)
            self._pages: dict[str, Page] = {}

    native = Native()
    page = Page(cast(Any, ContextOwner(native)), "page-9", "frame-10", "about:blank")
    page._routes["15"] = lambda: None

    # Act
    await page.unroute_all(behavior="wait")

    # Assert
    assert native.call == ("page-9", True)
    assert page._routes == {}


@pytest.mark.asyncio
async def test_context_unroute_all_wait_delegates_and_releases_handlers() -> None:
    # Arrange
    class Native:
        call: object = None

        async def context_unroute_all(self, context_id: str, wait: bool) -> None:
            self.call = (context_id, wait)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-9", None)
    context._routes["16"] = lambda: None

    # Act
    await context.unroute_all(behavior="wait")

    # Assert
    assert native.call == ("context-9", True)
    assert context._routes == {}


def test_sync_route_callback_awaits_the_queued_native_command() -> None:
    # Arrange
    class Native:
        call: object = None

        async def page_route_fulfill(
            self,
            call_id: str,
            status: int,
            headers: list[tuple[str, str]],
            body: bytes,
        ) -> None:
            self.call = (call_id, status, headers, body)

    native = Native()
    request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "fetch",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )
    async_route = Route(native, "8", request)  # type: ignore[arg-type]
    bridge = _LoopThread()
    route = SyncRoute(bridge, async_route)

    async def invoke() -> None:
        route.fulfill(status=202, body="done")
        await route._finish_callback_commands()

    # Act
    try:
        bridge.run(invoke())
    finally:
        bridge.close()

    # Assert
    assert native.call == ("8", 202, [], b"done")


def test_sync_page_route_handler_can_call_sync_page_api_without_loop_reentry() -> None:
    # Arrange
    class Native:
        calls: list[object]

        def __init__(self) -> None:
            self.calls = []

        async def page_route(
            self,
            page_id: str,
            pattern: str,
            times: Optional[int],
        ) -> str:
            self.calls.append(("route", page_id, pattern, times))
            return "sync-nested"

        async def wait_for_timeout(self, timeout_ms: int) -> None:
            self.calls.append(("wait", timeout_ms))

        async def page_route_fallback(
            self,
            call_id: str,
            url: Optional[str],
            method: Optional[str],
            headers: Optional[list[tuple[str, str]]],
            post_data: Optional[bytes],
        ) -> None:
            self.calls.append(("fallback", call_id, url, method, headers, post_data))

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, native: Native) -> None:
            self.browser = BrowserOwner(native)
            self._pages: dict[str, Page] = {}

    native = Native()
    async_page = Page(
        cast(Any, ContextOwner(native)),
        "page-sync-nested",
        "frame-sync-nested",
        "about:blank",
    )
    bridge = _LoopThread()
    page = SyncPage(bridge, async_page, cast(Any, None))
    request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
        }
    )
    async_route = Route(cast(Any, native), "call-sync-nested", request)
    handler_threads: list[int] = []

    def handler(route: SyncRoute) -> None:
        handler_threads.append(threading.get_ident())
        page.wait_for_timeout(500)
        route.fallback()

    # Act
    try:
        page.route("**/*", handler)
        bridge.run(
            cast(
                Coroutine[Any, Any, object],
                async_page._routes["sync-nested"](async_route, request),
            )
        )
    finally:
        bridge.close()

    # Assert
    assert handler_threads
    assert handler_threads[0] != bridge._thread_id
    assert native.calls == [
        ("route", "page-sync-nested", "**/*", None),
        ("wait", 500),
        ("fallback", "call-sync-nested", None, None, None, None),
    ]


def test_sync_route_fulfill_unwraps_the_sync_api_response() -> None:
    # Arrange
    class ResponseNative:
        pass

    class Native:
        response: object = None

        async def page_route_fulfill_response(
            self,
            call_id: str,
            response: object,
            status: Optional[int],
            headers: Optional[list[tuple[str, str]]],
            content_type: Optional[str],
        ) -> None:
            self.response = response

    response_native = ResponseNative()
    async_response = AsyncAPIResponse(cast(Any, response_native))
    native = Native()
    request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "fetch",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )
    async_route = Route(native, "10", request)  # type: ignore[arg-type]
    bridge = _LoopThread()
    response = SyncAPIResponse(bridge, async_response)
    route = SyncRoute(bridge, async_route)

    async def invoke() -> None:
        route.fulfill(response=response)
        await route._finish_callback_commands()

    # Act
    try:
        bridge.run(invoke())
    finally:
        bridge.close()

    # Assert
    assert native.response is response_native


def test_sync_api_response_json_is_safe_inside_the_route_callback_loop() -> None:
    # Arrange
    class ResponseNative:
        def body(self) -> bytes:
            return b'{"tags":["a","b"]}'

    async_response = AsyncAPIResponse(cast(Any, ResponseNative()))
    bridge = _LoopThread()
    response = SyncAPIResponse(bridge, async_response)
    observed: dict[str, object] = {}

    async def invoke() -> None:
        observed["json"] = response.json()

    # Act
    try:
        bridge.run(invoke())
    finally:
        bridge.close()

    # Assert
    assert observed == {"json": {"tags": ["a", "b"]}}


@pytest.mark.asyncio
async def test_response_body_uses_the_rust_owned_body_handle() -> None:
    # Arrange
    owner = object()

    class Native:
        async def response_body(self, observed_owner: object) -> bytes:
            assert observed_owner is owner
            return b"fulfilled"

    response = Response(
        {
            "url": "https://example.test/",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
            "body_owner": owner,
        },
        Native(),  # type: ignore[arg-type]
    )

    # Act
    body = await response.body()

    # Assert
    assert body == b"fulfilled"


@pytest.mark.asyncio
async def test_response_json_decodes_the_rust_owned_body() -> None:
    # Arrange
    owner = object()

    class Native:
        async def response_body(self, observed_owner: object) -> bytes:
            assert observed_owner is owner
            return b'{"answer": 42}'

    response = Response(
        {
            "url": "https://example.test/",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {"content-type": "application/json"},
            "body_owner": owner,
        },
        Native(),  # type: ignore[arg-type]
    )

    # Act
    value = await response.json()

    # Assert
    assert value == {"answer": 42}


def test_sync_response_json_delegates_to_the_async_response() -> None:
    # Arrange
    owner = object()

    class Native:
        async def response_body(self, observed_owner: object) -> bytes:
            assert observed_owner is owner
            return b'["value"]'

    async_response = Response(
        {
            "url": "https://example.test/",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {"content-type": "application/json"},
            "body_owner": owner,
        },
        Native(),  # type: ignore[arg-type]
    )
    bridge = _LoopThread()
    response = SyncResponse(bridge, async_response)

    # Act
    try:
        value = response.json()
    finally:
        bridge.close()

    # Assert
    assert value == ["value"]


@pytest.mark.asyncio
async def test_navigation_response_body_uses_the_rust_response_body_leaf() -> None:
    # Arrange
    observed: list[tuple[str, str, str]] = []

    class Native:
        async def frame_response_body(self, page_id: str, frame_id: str, request_id: str) -> bytes:
            observed.append((page_id, frame_id, request_id))
            return b"navigation"

    response = Response(
        {
            "url": "https://example.test/",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        },
        Native(),  # type: ignore[arg-type]
        navigation_body=("page-7", "frame-8", "request-9"),
    )

    # Act
    body = await response.body()

    # Assert
    assert body == b"navigation"
    assert observed == [("page-7", "frame-8", "request-9")]


@pytest.mark.asyncio
async def test_response_finished_uses_the_rust_owned_terminal_handle() -> None:
    # Arrange
    owner = object()

    class Native:
        observed_owner: Optional[object] = None

        async def response_finished(self, observed_owner: object) -> None:
            self.observed_owner = observed_owner

    native = Native()
    response = Response(
        {
            "url": "https://example.test/",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
            "finished_owner": owner,
        },
        native,  # type: ignore[arg-type]
    )

    # Act
    await response.finished()

    # Assert
    assert native.observed_owner is owner


@pytest.mark.asyncio
async def test_page_goto_returned_response_retains_the_native_finished_owner(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    owner = object()
    observed: list[object] = []

    class Native:
        async def goto(self, *_args: object) -> dict[str, object]:
            return {
                "url": "https://example.test/next",
                "response": {
                    "request_id": "native-next",
                    "url": "https://example.test/next",
                    "status": 200,
                    "status_text": "OK",
                    "ok": True,
                    "headers": {},
                    "finished_owner": owner,
                },
            }

        async def response_finished(self, received: object) -> None:
            observed.append(received)

    class Browser:
        _native = Native()

        def _raise_deferred_route_error(self) -> None:
            return None

        async def _flush_network_notices(self) -> None:
            return None

    class Context:
        browser = Browser()

        async def _drain_cdp_sessions(self, _page_id: str) -> None:
            return None

    monkeypatch.setattr(_native, "selected_features", lambda: ["navigation-response-finished"])
    page = Page(cast(Any, Context()), "page-7", "frame-8", "about:blank")

    # Act
    response = await page.goto("https://example.test/next", timeout=100)
    assert response is not None
    await response.finished()

    # Assert
    assert response.url == "https://example.test/next"
    assert observed == [owner]


@pytest.mark.asyncio
async def test_navigation_response_without_finished_owner_keeps_unsupported_boundary() -> None:
    # Arrange
    class Native:
        async def response_finished(self, _owner: object) -> None:
            raise AssertionError("no response-finished owner was attached")

    response = Response(
        {
            "url": "https://example.test/next",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        },
        Native(),  # type: ignore[arg-type]
    )

    # Act
    with pytest.raises(pwright.UnsupportedError, match="Response.finished capability"):
        await response.finished()

    # Assert
    assert response.url == "https://example.test/next"


def test_sync_response_finished_uses_the_async_bridge_once() -> None:
    # Arrange
    owner = object()

    class Native:
        observed_owner: Optional[object] = None

        async def response_finished(self, observed_owner: object) -> None:
            self.observed_owner = observed_owner

    native = Native()
    async_response = Response(
        {
            "url": "https://example.test/",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
            "finished_owner": owner,
        },
        native,  # type: ignore[arg-type]
    )
    bridge = _LoopThread()
    response = SyncResponse(bridge, async_response)

    # Act
    try:
        response.finished()
    finally:
        bridge.close()

    # Assert
    assert native.observed_owner is owner


@pytest.mark.asyncio
async def test_response_security_details_returns_only_the_typed_snapshot() -> None:
    # Arrange
    details = {
        "issuer": "Test Authority",
        "protocol": "TLS 1.3",
        "subjectName": "example.test",
        "validFrom": 1_700_000_000.25,
        "validTo": 1_800_000_000.5,
    }
    response = Response({"security_details": details})

    # Act
    observed = await response.security_details()

    # Assert
    assert observed == details
    assert observed is not details


@pytest.mark.asyncio
async def test_http_response_security_details_is_none_when_observed() -> None:
    # Arrange
    response = Response({"security_details": None})

    # Act
    observed = await response.security_details()

    # Assert
    assert observed is None


@pytest.mark.asyncio
async def test_response_security_details_is_unsupported_without_native_observation() -> None:
    # Arrange
    response = Response({"url": "https://example.test/"})

    # Act
    with pytest.raises(pwright.UnsupportedError, match="Response.security_details capability"):
        await response.security_details()

    # Assert
    assert response.url == "https://example.test/"


@pytest.mark.asyncio
async def test_response_terminal_merge_preserves_observed_security_details() -> None:
    # Arrange
    details = {
        "issuer": "Test Authority",
        "protocol": "TLS 1.3",
        "subjectName": "example.test",
        "validFrom": 1.0,
        "validTo": 2.0,
    }
    response = Response({"security_details": details})

    # Act
    response._merge({"security_details": None}, None)

    # Assert
    assert await response.security_details() == details


def test_sync_response_security_details_calls_the_bridge_once() -> None:
    # Arrange
    details = {
        "issuer": "Test Authority",
        "protocol": "TLS 1.3",
        "subjectName": "example.test",
        "validFrom": 1.0,
        "validTo": 2.0,
    }
    async_response = Response({"security_details": details})

    class Bridge:
        calls = 0

        def run(self, awaitable: Awaitable[Any]) -> Any:
            self.calls += 1
            return asyncio.run(awaitable)

    bridge = Bridge()
    response = SyncResponse(cast(Any, bridge), async_response)

    # Act
    observed = response.security_details()

    # Assert
    assert observed == details
    assert bridge.calls == 1


def test_request_post_data_json_returns_the_rust_converted_value_and_error() -> None:
    # Arrange
    parsed = Request(
        {
            "url": "https://example.test/post",
            "method": "POST",
            "resource_type": "fetch",
            "headers": {},
            "post_data": "foo=bar",
            "post_data_buffer": b"foo=bar",
            "post_data_json": {"foo": "bar"},
            "is_navigation_request": False,
        }
    )
    invalid = Request(
        {
            "url": "https://example.test/post",
            "method": "POST",
            "resource_type": "fetch",
            "headers": {},
            "post_data": "not-json",
            "post_data_buffer": b"not-json",
            "post_data_json": None,
            "post_data_json_error": "POST data is not a valid JSON object: not-json",
            "is_navigation_request": False,
        }
    )

    # Act
    parsed_value = parsed.post_data_json

    # Assert
    assert parsed_value == {"foo": "bar"}
    with pytest.raises(pwright.Error, match="POST data is not a valid JSON object"):
        _ = invalid.post_data_json


def test_sync_request_post_data_json_forwards_the_async_value() -> None:
    # Arrange
    async_request = Request(
        {
            "url": "https://example.test/post",
            "method": "POST",
            "resource_type": "fetch",
            "headers": {},
            "post_data": "foo=bar",
            "post_data_buffer": b"foo=bar",
            "post_data_json": {"foo": "bar"},
            "is_navigation_request": False,
        }
    )
    bridge = _LoopThread()
    request = SyncRequest(bridge, async_request)

    # Act
    try:
        value = request.post_data_json
    finally:
        bridge.close()

    # Assert
    assert value == {"foo": "bar"}


@pytest.mark.asyncio
async def test_request_full_header_helpers_use_the_rust_snapshot_case_insensitively() -> None:
    # Arrange
    request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "fetch",
            "headers": {"x-first": "one", "X-Second": "two"},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )

    # Act
    headers = await request.all_headers()
    first = await request.header_value("X-FIRST")
    second = await request.header_value("x-second")
    missing = await request.header_value("missing")
    headers["x-first"] = "changed"

    # Assert
    assert first == "one"
    assert second == "two"
    assert missing is None
    assert request.headers["x-first"] == "one"


def test_sync_request_full_header_helpers_bridge_the_async_snapshot() -> None:
    # Arrange
    async_request = Request(
        {
            "url": "https://example.test/",
            "method": "GET",
            "resource_type": "fetch",
            "headers": {"x-first": "one"},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": False,
        }
    )
    bridge = _LoopThread()
    request = SyncRequest(bridge, async_request)

    # Act
    try:
        headers = request.all_headers()
        value = request.header_value("X-FIRST")
        missing = request.header_value("missing")
    finally:
        bridge.close()

    # Assert
    assert headers == {"x-first": "one"}
    assert value == "one"
    assert missing is None


def test_route_snapshot_survives_native_request_merge_and_drives_response_url() -> None:
    # Arrange
    class BrowserOwner:
        def __init__(self) -> None:
            self._native = cast(Any, object())

    class ContextOwner:
        def __init__(self) -> None:
            self.browser = BrowserOwner()

    original = {
        "id": "request-effective",
        "url": "https://example.test/original",
        "method": "GET",
        "resource_type": "fetch",
        "headers": {"x-original": "yes"},
        "post_data": None,
        "post_data_buffer": None,
        "is_navigation_request": False,
    }
    page = Page(
        cast(Any, ContextOwner()),
        "page-effective",
        "frame-effective",
        "about:blank",
    )
    request = page._wrap_request(original)
    request._merge_route_snapshot(
        {
            "url": "https://example.test/effective",
            "method": "POST",
            "headers": {"x-effective": "yes"},
            "post_data": "updated",
            "post_data_buffer": b"updated",
        },
        cast(Any, object()),
        None,
    )
    page._remember_route_request_snapshot(request)
    page._requests.pop("request-effective")
    response = Response(
        {
            "url": "https://example.test/original",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        },
        request=page._wrap_request(original),
    )

    # Act
    restored = response.request

    # Assert
    assert restored.url == "https://example.test/effective"
    assert restored.method == "POST"
    assert restored.headers == {"x-effective": "yes"}
    assert restored.post_data == "updated"
    assert restored.post_data_buffer == b"updated"
    assert response.url == "https://example.test/effective"


@pytest.mark.asyncio
async def test_context_network_wait_uses_context_identity_and_converts_request() -> None:
    # Arrange
    history_owner = object()

    class Native:
        armed: Optional[tuple[str, str, int]] = None
        canceled: Optional[int] = None
        observed_history_owner: object = None

        def context_default_timeout(self, context_id: str, navigation: bool) -> int:
            assert context_id == "context-9"
            assert not navigation
            return 345

        async def arm_context_network_wait(
            self, context_id: str, kind: str, timeout_ms: int
        ) -> int:
            self.armed = (context_id, kind, timeout_ms)
            return 7

        async def next_network_event(self, waiter_id: int) -> dict[str, object]:
            assert waiter_id == 7
            return {
                "kind": "request",
                "url": "https://example.test/post",
                "method": "POST",
                "resource_type": "fetch",
                "headers": {},
                "post_data": "foo=bar",
                "post_data_buffer": b"foo=bar",
                "post_data_json": {"foo": "bar"},
                "is_navigation_request": False,
                "history_owner": history_owner,
            }

        async def request_history_response(self, owner: object) -> dict[str, object]:
            self.observed_history_owner = owner
            return {
                "url": "https://example.test/post",
                "status": 200,
                "status_text": "OK",
                "ok": True,
                "headers": {},
                "from_service_worker": False,
                "body_owner": None,
                "finished_owner": None,
            }

        async def cancel_network_wait(self, waiter_id: int) -> None:
            self.canceled = waiter_id

    class FakeBrowser:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, FakeBrowser(native)), "context-9", None)

    # Act
    async with context.expect_event("request") as request_info:
        request = cast(Request, await request_info.value)
    response = await request.response()

    # Assert
    assert isinstance(request, Request)
    assert response is not None
    assert response.status == 200
    assert request.post_data_json == {"foo": "bar"}
    assert native.observed_history_owner is history_owner
    assert native.armed == ("context-9", "request", 345)
    assert native.canceled == 7


@pytest.mark.asyncio
async def test_context_page_waiter_resolves_the_published_page() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-9", None)
    event_info = context.expect_event("page", timeout=0)
    await event_info.__aenter__()
    page = Page(context, "page-2", "frame-3", "about:blank")

    # Act
    context._emit_context_event("page", page)
    value = await event_info.value
    await event_info.__aexit__(None, None, None)

    # Assert
    assert value is page
    assert context._event_waiters == {}


@pytest.mark.asyncio
async def test_context_expect_page_reuses_the_lifecycle_event_waiter() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-9", None)
    page = Page(context, "page-2", "frame-3", "about:blank")
    observed: list[Page] = []
    event_info = context.expect_page(
        lambda value: observed.append(cast(Page, value)) is None,
        timeout=0,
    )
    await event_info.__aenter__()

    # Act
    context._emit_context_event("page", page)
    value = await event_info.value
    await event_info.__aexit__(None, None, None)

    # Assert
    assert value is page
    assert observed == [page]
    assert context._event_waiters == {}


@pytest.mark.asyncio
async def test_context_console_waiter_arms_before_lifecycle_fence() -> None:
    # Arrange
    flush_started = asyncio.Event()
    release_flush = asyncio.Event()

    class Native:
        released: list[int] = []

        async def flush_lifecycle_events(self) -> None:
            flush_started.set()
            await release_flush.wait()

        def subscribe_context_console(self, context_id: str) -> int:
            assert context_id == "context-1"
            return 23

        def release_console_subscription(self, token: int) -> None:
            self.released.append(token)

    native = Native()
    browser = object.__new__(Browser)
    browser._closed = False
    browser._contexts = {}
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-1", None)
    page = Page(context, "page-2", "frame-3", "about:blank")
    context._pages["page-2"] = page
    browser._contexts["context-1"] = context

    # Act
    waiter = asyncio.create_task(context.wait_for_event("console", timeout=0))
    await flush_started.wait()
    browser._receive_lifecycle_notice(
        6,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        ("log", "during fence", "[]", "", 0, 0, 9_000_000_000_000_000),
    )
    release_flush.set()
    message = cast(ConsoleMessage, await asyncio.wait_for(waiter, 1))

    # Assert
    assert message.text == "during fence"
    assert native.released == [23]
    assert context._event_waiters == {}


class _FrameEventNative:
    def __init__(self) -> None:
        self.rows: list[tuple[str, str | None, str, str]] = [
            ("frame-3", None, "", "about:blank"),
            ("frame-4", "frame-3", "child", "about:blank"),
        ]

    def frames(self, page_id: str) -> list[tuple[str, str | None, str, str]]:
        assert page_id == "page-2"
        return self.rows


def _frame_event_fixture() -> tuple[Browser, Page, _FrameEventNative]:
    native = _FrameEventNative()
    browser = object.__new__(Browser)
    browser._closed = False
    browser._contexts = {}
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-1", None)
    page = Page(context, "page-2", "frame-3", "about:blank")
    context._pages["page-2"] = page
    browser._contexts["context-1"] = context
    return browser, page, native


@pytest.mark.asyncio
async def test_page_frameattached_event_yields_canonical_child(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["frame-events"])
    browser, page, _native_fixture = _frame_event_fixture()
    info = page.expect_event("frameattached", timeout=0)
    await info.__aenter__()

    # Act
    browser._receive_lifecycle_notice(
        11,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        frame_id="frame-4",
    )
    frame = await info.value
    await info.__aexit__(None, None, None)

    # Assert
    assert isinstance(frame, Frame)
    assert frame is page.frame("child")
    assert frame.parent_frame is page.main_frame


@pytest.mark.asyncio
async def test_page_framenavigated_event_matches_existing_frame(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["frame-events"])
    browser, page, native = _frame_event_fixture()
    child = page.frame("child")
    assert child is not None
    info = page.expect_event("framenavigated", lambda frame: frame is child, timeout=0)
    await info.__aenter__()
    native.rows[1] = ("frame-4", "frame-3", "child", "https://example.test/child")

    # Act
    browser._receive_lifecycle_notice(
        12,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        frame_id="frame-4",
    )
    frame = await info.value
    await info.__aexit__(None, None, None)

    # Assert
    assert frame is child
    assert child.url == "https://example.test/child"
    assert page._event_waiters == {}


@pytest.mark.asyncio
async def test_batched_frame_notices_allow_sequential_waiter_registration(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["frame-events"])
    browser, page, _native_fixture = _frame_event_fixture()
    browser._loop = asyncio.get_running_loop()
    attached_info = page.expect_event("frameattached", timeout=100)
    await attached_info.__aenter__()

    # Act
    browser._schedule_lifecycle_notice(
        11,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        frame_id="frame-4",
    )
    browser._schedule_lifecycle_notice(
        12,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        frame_id="frame-4",
    )
    attached = await attached_info.value
    await attached_info.__aexit__(None, None, None)
    navigated_info = page.expect_event(
        "framenavigated", lambda frame: frame is attached, timeout=100
    )
    await navigated_info.__aenter__()
    navigated = await navigated_info.value
    await navigated_info.__aexit__(None, None, None)

    # Assert
    assert navigated is attached


@pytest.mark.asyncio
async def test_page_on_frameattached_receives_canonical_child(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["frame-events"])
    browser, page, _native_fixture = _frame_event_fixture()
    observed: list[Frame] = []
    page.on("frameattached", lambda value: observed.append(cast(Frame, value)))

    # Act
    browser._receive_lifecycle_notice(
        11,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        frame_id="frame-4",
    )

    # Assert
    assert observed == [page.frame("child")]


@pytest.mark.asyncio
async def test_unrelated_frame_navigation_is_delivered_immediately(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["frame-events"])
    browser, page, _native_fixture = _frame_event_fixture()
    observed: list[Frame] = []
    page.on("framenavigated", lambda value: observed.append(cast(Frame, value)))

    # Act
    browser._receive_lifecycle_notice(
        12,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        frame_id="frame-4",
    )

    # Assert
    assert observed == [page.frame("child")]


@pytest.mark.asyncio
async def test_frame_attach_delivery_marker_retires_on_page_close(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["frame-events"])
    browser, page, _native_fixture = _frame_event_fixture()
    browser._receive_lifecycle_notice(
        11,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        frame_id="frame-4",
    )
    assert page._frame_attaches_this_turn == {"frame-4"}

    # Act
    browser._receive_lifecycle_notice(
        0, "context-1", "page-2", None, None, "frame-3", "about:blank", None, None
    )

    # Assert
    assert page._frame_attaches_this_turn == set()
    assert page._closed


def test_sync_page_frame_event_predicate_receives_wrapped_identity(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(_native, "selected_features", lambda: ["frame-events"])
    bridge = _LoopThread()
    browser, async_page, _native_fixture = _frame_event_fixture()
    page = SyncPage(bridge, async_page, cast(Any, None))
    child = page.frame("child")
    assert child is not None

    # Act
    try:
        with page.expect_event("framenavigated", lambda frame: frame is child, timeout=0) as info:

            async def emit() -> None:
                browser._receive_lifecycle_notice(
                    12,
                    "context-1",
                    "page-2",
                    None,
                    None,
                    "frame-3",
                    "about:blank",
                    None,
                    None,
                    frame_id="frame-4",
                )

            bridge.run(emit())
        value = info.value
    finally:
        bridge.close()

    # Assert
    assert value is child


@pytest.mark.asyncio
async def test_page_error_fans_out_as_context_web_error() -> None:
    # Arrange
    browser = object.__new__(Browser)
    browser._closed = False
    browser._contexts = {}
    browser._native = cast(Any, object())
    context = BrowserContext(browser, "context-1", None)
    page = Page(context, "page-2", "frame-3", "about:blank")
    context._pages["page-2"] = page
    browser._contexts["context-1"] = context
    page_info = page.expect_event("pageerror", timeout=0)
    context_info = context.expect_event("weberror", timeout=0)
    await page_info.__aenter__()
    await context_info.__aenter__()

    # Act
    browser._receive_lifecycle_notice(
        8,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        ("TypeError", "broken", "TypeError: broken"),
        None,
    )
    page_error = await page_info.value
    web_error = cast(WebError, await context_info.value)
    await page_info.__aexit__(None, None, None)
    await context_info.__aexit__(None, None, None)

    # Assert
    assert web_error.page is page
    assert web_error.error is page_error
    assert web_error.error.name == "TypeError"
    assert web_error.error.message == "broken"


@pytest.mark.asyncio
async def test_dialog_fans_out_to_page_and_context_without_auto_handling() -> None:
    # Arrange
    class Native:
        handled: list[tuple[bool, Optional[str]]] = []

        async def handle_dialog(
            self, _dialog: object, accept: bool, prompt_text: Optional[str]
        ) -> None:
            self.handled.append((accept, prompt_text))

    native = Native()
    browser = object.__new__(Browser)
    browser._closed = False
    browser._contexts = {}
    browser._native = cast(Any, native)
    context = BrowserContext(browser, "context-1", None)
    page = Page(context, "page-2", "frame-3", "about:blank")
    context._pages["page-2"] = page
    browser._contexts["context-1"] = context
    page_info = page.expect_event("dialog", timeout=0)
    context_info = context.expect_event("dialog", timeout=0)
    await page_info.__aenter__()
    await context_info.__aenter__()

    # Act
    browser._receive_lifecycle_notice(
        9,
        "context-1",
        "page-2",
        None,
        None,
        "frame-3",
        "about:blank",
        None,
        None,
        (cast(Any, object()), "alert", "hello", ""),
    )
    page_dialog = cast(Dialog, await page_info.value)
    context_dialog = cast(Dialog, await context_info.value)
    await page_info.__aexit__(None, None, None)
    await context_info.__aexit__(None, None, None)
    await asyncio.sleep(0)

    # Assert
    assert context_dialog is page_dialog
    assert context_dialog.page is page
    assert context_dialog.message == "hello"
    assert native.handled == []


def test_sync_context_wraps_dialog_and_web_error() -> None:
    # Arrange
    bridge = _LoopThread()
    async_context = BrowserContext(cast(Any, object()), "context-1", None)
    async_page = Page(async_context, "page-2", "frame-3", "about:blank")
    async_context._pages["page-2"] = async_page
    context = SyncBrowserContext(bridge, async_context, cast(Any, None))
    async_dialog = Dialog(async_page, cast(Any, object()), "alert", "hello", "")
    error = pwright.Error("broken")
    async_web_error = WebError(async_page, error)

    # Act
    try:
        dialog = cast(SyncDialog, context._wrap_context_event(async_dialog))
        web_error = cast(SyncWebError, context._wrap_context_event(async_web_error))
    finally:
        bridge.close()

    # Assert
    assert dialog.page is context._wrap_page(async_page)
    assert dialog.message == "hello"
    assert web_error.page is dialog.page
    assert web_error.error is error


def test_sync_context_expect_page_wraps_predicate_and_value() -> None:
    # Arrange
    bridge = _LoopThread()
    async_context = BrowserContext(cast(Any, object()), "context-1", None)
    async_page = Page(async_context, "page-2", "frame-3", "about:blank")
    context = SyncBrowserContext(bridge, async_context, cast(Any, None))
    observed: list[SyncPage] = []

    # Act
    try:
        with context.expect_page(
            lambda page: observed.append(page) is None,
            timeout=0,
        ) as page_info:

            async def emit() -> None:
                async_context._emit_context_event("page", async_page)

            bridge.run(emit())
        page = cast(SyncPage, page_info.value)
    finally:
        bridge.close()

    # Assert
    assert observed == [page]
    assert page is context._wrap_page(async_page)


@pytest.mark.asyncio
async def test_context_close_listener_receives_the_context_once() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-9", None)
    observed: list[BrowserContext] = []
    context.once("close", lambda value: observed.append(cast(BrowserContext, value)))

    # Act
    context._emit_context_event("close", context)
    context._emit_context_event("close", context)

    # Assert
    assert observed == [context]
    assert context._event_handlers == {}


@pytest.mark.asyncio
async def test_context_tracing_stop_without_an_active_trace_is_a_noop() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-tracing", None)

    # Act
    await context.tracing.stop()

    # Assert
    assert context.tracing is context.tracing


@pytest.mark.asyncio
async def test_context_tracing_stop_with_a_path_remains_explicitly_unsupported() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-tracing", None)

    # Act
    with pytest.raises(pwright.UnsupportedError) as raised:
        await context.tracing.stop(path="trace.zip")

    # Assert
    assert str(raised.value) == "Tracing.stop(path=...) is not installed"


def test_sync_context_tracing_stop_without_an_active_trace_is_a_noop() -> None:
    # Arrange
    bridge = _LoopThread()
    async_context = BrowserContext(cast(Any, object()), "context-tracing", None)
    context = SyncBrowserContext(bridge, async_context, cast(Any, None))

    # Act
    try:
        context.tracing.stop()
    finally:
        bridge.close()

    # Assert
    assert context.tracing is context.tracing


def test_context_console_listener_owns_one_subscription_until_removed() -> None:
    # Arrange
    class Native:
        subscribed: list[str] = []
        released: list[int] = []

        def subscribe_context_console(self, context_id: str) -> int:
            self.subscribed.append(context_id)
            return 17

        def release_console_subscription(self, token: int) -> None:
            self.released.append(token)

    class Browser:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, Browser(native)), "context-9", None)

    def listener(_value: object) -> None:
        pass

    # Act
    context.on("console", listener)
    context.remove_listener("console", listener)

    # Assert
    assert native.subscribed == ["context-9"]
    assert native.released == [17]
    assert context._console_subscription_token is None


def test_sync_context_close_listener_receives_the_sync_context() -> None:
    # Arrange
    bridge = _LoopThread()
    async_context = BrowserContext(cast(Any, object()), "context-9", None)
    context = SyncBrowserContext(bridge, async_context, cast(Any, None))
    observed: list[SyncBrowserContext] = []

    # Act
    try:
        context.on("close", lambda value: observed.append(cast(SyncBrowserContext, value)))

        async def emit() -> None:
            async_context._emit_context_event("close", async_context)

        bridge.run(emit())
    finally:
        bridge.close()

    # Assert
    assert observed == [context]


def test_sync_context_once_listener_can_be_removed_by_public_identity() -> None:
    # Arrange
    bridge = _LoopThread()
    async_context = BrowserContext(cast(Any, object()), "context-9", None)
    context = SyncBrowserContext(bridge, async_context, cast(Any, None))
    observed: list[object] = []

    def listener(value: object) -> None:
        observed.append(value)

    # Act
    try:
        context.once("close", listener)
        context.remove_listener("close", listener)

        async def emit() -> None:
            async_context._emit_context_event("close", async_context)

        bridge.run(emit())
    finally:
        bridge.close()

    # Assert
    assert observed == []
    assert context._event_handlers == []


@pytest.mark.asyncio
async def test_context_cookies_forwards_urls_and_maps_the_native_shape() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[str, list[str]]] = None

        async def context_cookies(
            self, context_id: str, urls: list[str]
        ) -> list[tuple[str, str, str, str, float, bool, bool, str, Optional[str]]]:
            self.observed = (context_id, urls)
            return [
                (
                    "session",
                    "value",
                    "example.test",
                    "/",
                    -1.0,
                    False,
                    True,
                    "Lax",
                    None,
                )
            ]

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-9", None)

    # Act
    cookies = await context.cookies("https://example.test/path")

    # Assert
    assert native.observed == ("context-9", ["https://example.test/path"])
    assert cookies == [
        {
            "name": "session",
            "value": "value",
            "domain": "example.test",
            "path": "/",
            "expires": -1.0,
            "httpOnly": False,
            "secure": True,
            "sameSite": "Lax",
        }
    ]


@pytest.mark.asyncio
async def test_context_cookies_rejects_non_string_url_with_its_own_api_name() -> None:
    # Arrange
    class Native:
        called = False

        async def context_cookies(self, _context_id: str, _urls: list[str]) -> list[object]:
            self.called = True
            return []

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-9", None)

    # Act
    with pytest.raises(pwright.Error, match="BrowserContext.cookies url: expected string"):
        await context.cookies(cast(Any, [42]))

    # Assert
    assert not native.called


@pytest.mark.asyncio
async def test_context_add_cookies_passes_a_bounded_native_tuple() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[str, list[object]]] = None

        async def context_add_cookies(self, context_id: str, cookies: list[object]) -> None:
            self.observed = (context_id, cookies)

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-9", None)

    # Act
    await context.add_cookies(
        [
            {
                "url": "https://example.test/path",
                "name": "session",
                "value": "value",
                "httpOnly": True,
                "sameSite": "Strict",
            }
        ]
    )

    # Assert
    assert native.observed == (
        "context-9",
        [
            (
                "session",
                "value",
                "https://example.test/path",
                None,
                None,
                None,
                True,
                None,
                "Strict",
                None,
            )
        ],
    )


@pytest.mark.asyncio
async def test_context_add_cookies_rejects_an_oversized_field_before_native_dispatch() -> None:
    # Arrange
    class Native:
        called = False

        async def context_add_cookies(self, _context_id: str, _cookies: list[object]) -> None:
            self.called = True

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    context = BrowserContext(cast(Any, BrowserOwner(native)), "context-9", None)

    # Act
    with pytest.raises(pwright.ResourceExhaustedError, match="16384 UTF-8 bytes"):
        await context.add_cookies(
            [{"url": "https://example.test", "name": "x" * 16385, "value": "value"}]
        )

    # Assert
    assert not native.called


def test_sync_context_clear_cookies_preserves_regex_filters() -> None:
    # Arrange
    class Native:
        observed: Optional[tuple[object, ...]] = None

        async def context_clear_cookies(self, *values: object) -> None:
            self.observed = values

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    native = Native()
    bridge = _LoopThread()
    async_context = BrowserContext(cast(Any, BrowserOwner(native)), "context-9", None)
    context = SyncBrowserContext(bridge, async_context, cast(Any, None))

    # Act
    try:
        context.clear_cookies(name=re.compile("^sess", re.IGNORECASE), domain="example.test")
    finally:
        bridge.close()

    # Assert
    assert native.observed == (
        "context-9",
        "^sess",
        "i",
        "example.test",
        None,
        None,
        None,
    )


@pytest.mark.asyncio
async def test_page_event_waiter_maps_asyncio_timeout_to_playwright_timeout() -> None:
    # Arrange
    page = Page(cast(Any, object()), "page-2", "frame-3", "about:blank")
    event_info = page.expect_event("popup", timeout=1)
    await event_info.__aenter__()

    # Act
    with pytest.raises(
        pwright.TimeoutError,
        match='Timeout 1ms exceeded while waiting for event "popup"',
    ):
        await event_info.__aexit__(None, None, None)

    # Assert
    assert page._event_waiters == {}


@pytest.mark.asyncio
async def test_file_chooser_interception_tracks_listener_and_waiter_lifetime() -> None:
    # Arrange
    class Native:
        transitions: list[tuple[str, bool]] = []

        async def set_page_file_chooser_interception(self, page_id: str, enabled: bool) -> None:
            self.transitions.append((page_id, enabled))

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    native = Native()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(native))),
        "page-2",
        "frame-3",
        "about:blank",
    )
    observed: list[object] = []
    page.once("filechooser", observed.append)
    event = object()

    # Act
    async with page.expect_file_chooser(timeout=100):
        page._emit_page_event("filechooser", event)

    # Assert
    assert observed == [event]
    assert native.transitions == [("page-2", True), ("page-2", False)]
    assert page._file_chooser_interception_count == 0


@pytest.mark.asyncio
async def test_file_chooser_mutation_retains_interception_until_native_completion() -> None:
    # Arrange
    mutation_started = asyncio.Event()
    release_mutation = asyncio.Event()
    mutation_finished = asyncio.Event()

    class Native:
        def __init__(self) -> None:
            self.transitions: list[tuple[str, bool]] = []

        async def set_page_file_chooser_interception(self, page_id: str, enabled: bool) -> None:
            self.transitions.append((page_id, enabled))

        async def set_file_chooser_payloads(self, _chooser: object, _payloads: object) -> None:
            mutation_started.set()
            await release_mutation.wait()

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    class NativeChooser:
        def preview(self) -> str:
            return "JSHandle@<input/>"

        def is_element(self) -> bool:
            return True

    native = Native()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(native))),
        "page-2",
        "frame-3",
        "about:blank",
    )
    chooser = FileChooser(page, cast(Any, NativeChooser()), False)

    async def mutate(value: object) -> None:
        await cast(FileChooser, value).set_files([])
        mutation_finished.set()

    page.once("filechooser", mutate)
    disabled_during_mutation = True

    # Act
    async with page.expect_file_chooser(timeout=100):
        page._emit_page_event("filechooser", chooser)
        await mutation_started.wait()
        disabled_during_mutation = native.transitions[-1] == ("page-2", False)
        release_mutation.set()
    await mutation_finished.wait()
    await page._flush_file_chooser_interception()

    # Assert
    assert not disabled_during_mutation
    assert native.transitions == [("page-2", True), ("page-2", False)]
    assert page._file_chooser_interception_count == 0


def test_popup_navigation_request_frame_reports_the_official_unavailable_error() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-9", None)
    request = Request(
        {
            "page_id": "popup-page",
            "url": "https://example.test/empty.html",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "post_data_json": None,
            "is_navigation_request": True,
        },
        context=context,
    )

    # Act
    with pytest.raises(pwright.Error) as raised:
        _ = request.frame

    # Assert
    assert "Frame for this navigation request is not available" in raised.value.message


def test_request_frame_preserves_the_exact_portable_frame_identity() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-9", None)
    page = Page(context, "page-10", "frame-main", "about:blank")
    child = Frame(page, "frame-child", "frame-main", "child", "about:blank")
    page._frames["frame-child"] = child
    context._pages["page-10"] = page
    request = Request(
        {
            "page_id": "page-10",
            "frame_id": "frame-child",
            "frame_available": True,
            "url": "https://example.test/empty.html",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
        },
        context=context,
    )

    # Act
    observed = request.frame

    # Assert
    assert observed is child
    assert observed is not page._frames["frame-main"]


def test_response_frame_reuses_its_request_exact_frame_identity() -> None:
    # Arrange
    context = BrowserContext(cast(Any, object()), "context-9", None)
    page = Page(context, "page-10", "frame-main", "about:blank")
    child = Frame(page, "frame-child", "frame-main", "child", "about:blank")
    page._frames["frame-child"] = child
    context._pages["page-10"] = page
    request = Request(
        {
            "page_id": "page-10",
            "frame_id": "frame-child",
            "frame_available": True,
            "url": "https://example.test/empty.html",
            "method": "GET",
            "resource_type": "document",
            "headers": {},
            "post_data": None,
            "post_data_buffer": None,
            "is_navigation_request": True,
        },
        context=context,
    )
    response = Response(
        {
            "url": "https://example.test/empty.html",
            "status": 200,
            "status_text": "OK",
            "ok": True,
            "headers": {},
        },
        context=context,
        request=request,
    )

    # Act
    observed = response.frame

    # Assert
    assert observed is child
    assert observed is response.request.frame


@pytest.mark.asyncio
async def test_element_handle_content_frame_resolves_the_portable_frame_identity() -> None:
    # Arrange
    class Native:
        observed_handle: object = None

        async def element_handle_content_frame(self, handle: object) -> str:
            self.observed_handle = handle
            return "frame-child"

    class NativeHandle:
        def preview(self) -> str:
            return "JSHandle@node"

        def is_element(self) -> bool:
            return True

    context = BrowserContext(cast(Any, object()), "context-9", None)
    page = Page(context, "page-10", "frame-main", "about:blank")
    child = Frame(page, "frame-child", "frame-main", "child", "about:blank")
    page._frames["frame-child"] = child
    context._pages["page-10"] = page
    native = Native()
    native_handle = NativeHandle()
    handle = ElementHandle(cast(Any, native), cast(Any, native_handle), page)

    # Act
    observed = await handle.content_frame()

    # Assert
    assert native.observed_handle is native_handle
    assert observed is child


@pytest.mark.asyncio
async def test_page_close_rejects_a_pending_download_waiter() -> None:
    # Arrange
    page = Page(cast(Any, object()), "page", "frame", "about:blank")
    event_info = page.expect_download(timeout=0)
    await event_info.__aenter__()

    # Act
    page._emit_page_event("close", page)

    # Assert
    with pytest.raises(
        pwright.TargetClosedError,
        match="Target page, context or browser has been closed",
    ):
        await event_info.__aexit__(None, None, None)
    assert page._event_waiters == {}


@pytest.mark.asyncio
async def test_download_path_uses_the_exact_native_owner() -> None:
    # Arrange
    native_owner = cast(Any, object())

    class Native:
        observed: object = None

        async def download_path(self, owner: object) -> str:
            self.observed = owner
            return "/tmp/file.txt"

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    native = Native()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(native))),
        "page",
        "frame",
        "about:blank",
    )
    download = Download(page, "https://example.test/file", "file.txt", native_owner)

    # Act
    path = await download.path()

    # Assert
    assert path == Path("/tmp/file.txt")
    assert native.observed is native_owner


def test_sync_download_wraps_the_async_download_and_path() -> None:
    # Arrange
    class Native:
        def page_default_timeout(self, _page_id: str, _navigation: bool) -> int:
            return 30_000

        async def download_path(self, _owner: object) -> str:
            return "/tmp/file.txt"

    class BrowserOwner:
        def __init__(self) -> None:
            self._native = Native()

    class ContextOwner:
        def __init__(self) -> None:
            self.browser = BrowserOwner()

    page = Page(cast(Any, ContextOwner()), "page", "frame", "about:blank")
    download = Download(
        page,
        "https://example.test/file",
        "file.txt",
        cast(Any, object()),
    )
    bridge = _LoopThread()
    sync_page = cast(SyncPage, object())
    sync_download = SyncDownload(bridge, download, sync_page)

    # Act
    try:
        path = sync_download.path()
    finally:
        bridge.close()

    # Assert
    assert path == Path("/tmp/file.txt")
    assert sync_download.page is sync_page
    assert sync_download.suggested_filename == "file.txt"


def test_sync_download_event_info_wraps_the_async_download() -> None:
    # Arrange
    class Native:
        def page_default_timeout(self, _page_id: str, _navigation: bool) -> int:
            return 30_000

        async def download_path(self, _owner: object) -> str:
            return "/tmp/file.txt"

    class BrowserOwner:
        def __init__(self) -> None:
            self._native = Native()

    class ContextOwner:
        def __init__(self) -> None:
            self.browser = BrowserOwner()

    page = Page(cast(Any, ContextOwner()), "page", "frame", "about:blank")
    download = Download(
        page,
        "https://example.test/file",
        "file.txt",
        cast(Any, object()),
    )
    bridge = _LoopThread()
    sync_page = SyncPage(bridge, page, cast(SyncBrowserContext, object()))

    async def emit() -> None:
        page._emit_page_event("download", download)

    # Act
    try:
        with sync_page.expect_download() as event_info:
            bridge.run(emit())
        value = event_info.value
    finally:
        bridge.close()

    # Assert
    assert isinstance(value, SyncDownload)
    assert value.page is sync_page
    assert value.suggested_filename == "file.txt"


@pytest.mark.asyncio
async def test_worker_evaluate_uses_the_exact_native_owner() -> None:
    # Arrange
    native_owner = cast(Any, object())

    class Native:
        observed: Optional[tuple[object, str, object]] = None

        async def worker_evaluate(self, owner: object, expression: str, argument: object) -> object:
            self.observed = (owner, expression, argument)
            return "worker result"

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    native = Native()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(native))),
        "page",
        "frame",
        "about:blank",
    )
    worker = Worker(page, "worker-1", "https://example.test/worker.js", native_owner)

    # Act
    result = await worker.evaluate("value => value.answer", {"answer": 42})

    # Assert
    assert result == "worker result"
    assert native.observed == (native_owner, "value => value.answer", {"answer": 42})
    assert worker.url == "https://example.test/worker.js"
    assert repr(worker) == "<Worker url='https://example.test/worker.js'>"


@pytest.mark.asyncio
async def test_worker_evaluate_handle_and_property_preserve_native_owners() -> None:
    # Arrange
    native_owner = cast(Any, object())

    class NativeHandle:
        def __init__(self, preview: str) -> None:
            self._preview = preview

        def preview(self) -> str:
            return self._preview

        @staticmethod
        def is_element() -> bool:
            return False

    root = NativeHandle("JSHandle@object")
    child = NativeHandle("JSHandle@number")

    class Native:
        observed: list[tuple[object, ...]] = []

        async def worker_evaluate_handle(
            self, owner: object, expression: str, argument: object
        ) -> NativeHandle:
            self.observed.append(("evaluate_handle", owner, expression, argument))
            return root

        async def handle_get_property(self, handle: NativeHandle, name: str) -> NativeHandle:
            self.observed.append(("get_property", handle, name))
            return child

        async def handle_json_value(self, handle: NativeHandle) -> object:
            self.observed.append(("json_value", handle))
            return 42

    class BrowserOwner:
        def __init__(self, native: Native) -> None:
            self._native = native

    class ContextOwner:
        def __init__(self, browser: BrowserOwner) -> None:
            self.browser = browser

    native = Native()
    page = Page(
        cast(Any, ContextOwner(BrowserOwner(native))),
        "page",
        "frame",
        "about:blank",
    )
    worker = Worker(page, "worker-1", "https://example.test/worker.js", native_owner)

    # Act
    handle = await worker.evaluate_handle("value => value", {"answer": 42})
    property_handle = await handle.get_property("answer")
    value = await property_handle.json_value()

    # Assert
    assert native.observed == [
        ("evaluate_handle", native_owner, "value => value", {"answer": 42}),
        ("get_property", root, "answer"),
        ("json_value", child),
    ]
    assert repr(handle) == "<JSHandle preview=JSHandle@object>"
    assert repr(property_handle) == "<JSHandle preview=JSHandle@number>"
    assert handle._impl_obj is handle
    assert property_handle._impl_obj is property_handle
    assert value == 42


@pytest.mark.asyncio
async def test_js_handle_evaluate_unwraps_nested_handles_and_preserves_cycles() -> None:
    # Arrange
    class NativeHandle:
        def __init__(self, preview: str) -> None:
            self._preview = preview

        def preview(self) -> str:
            return self._preview

        @staticmethod
        def is_element() -> bool:
            return False

    class NativeSession:
        observed: Optional[tuple[object, str, object]] = None

        async def handle_evaluate(
            self, target: object, expression: str, argument: object
        ) -> object:
            self.observed = (target, expression, argument)
            return argument

    native_target = NativeHandle("object")
    native_child = NativeHandle("child")
    session = NativeSession()
    target = AsyncJSHandle(cast(Any, session), cast(Any, native_target))
    child = AsyncJSHandle(cast(Any, session), cast(Any, native_child))
    argument: dict[str, object] = {"nested": [{"child": child}]}
    argument["cycle"] = argument

    # Act
    result = await target.evaluate("value => value", argument)

    # Assert
    assert session.observed is not None
    observed = cast(dict[str, object], session.observed[2])
    nested = cast(list[object], observed["nested"])
    assert cast(dict[str, object], nested[0])["child"] is native_child
    assert observed["cycle"] is observed
    assert result is observed


@pytest.mark.asyncio
async def test_js_handle_get_properties_wraps_each_native_owner() -> None:
    # Arrange
    class NativeHandle:
        def __init__(self, preview: str) -> None:
            self._preview = preview

        def preview(self) -> str:
            return self._preview

        @staticmethod
        def is_element() -> bool:
            return False

    root = NativeHandle("object")
    first = NativeHandle("1")
    second = NativeHandle("two")

    class NativeSession:
        observed: object = None

        async def handle_get_properties(self, handle: object) -> dict[str, NativeHandle]:
            self.observed = handle
            return {"first": first, "second": second}

    session = NativeSession()
    handle = AsyncJSHandle(cast(Any, session), cast(Any, root))

    # Act
    properties = await handle.get_properties()

    # Assert
    assert session.observed is root
    assert set(properties) == {"first", "second"}
    assert properties["first"]._native is first
    assert properties["second"]._native is second
    assert str(properties["first"]) == "1"


@pytest.mark.asyncio
async def test_js_handle_argument_unwrap_rejects_overdepth_before_native_call() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def preview() -> str:
            return "object"

        @staticmethod
        def is_element() -> bool:
            return False

    class NativeSession:
        called = False

        async def handle_evaluate(
            self, _target: object, _expression: str, _argument: object
        ) -> object:
            self.called = True
            return None

    session = NativeSession()
    handle = AsyncJSHandle(cast(Any, session), cast(Any, NativeHandle()))
    argument: object = None
    for _ in range(66):
        argument = [argument]

    # Act
    with pytest.raises(pwright.InvalidArgumentError, match="depth exceeds limit"):
        await handle.evaluate("value => value", argument)

    # Assert
    assert session.called is False


def test_js_handle_preview_preserves_driver_text_without_node_prefix() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def preview() -> str:
            return "Window"

        @staticmethod
        def is_element() -> bool:
            return False

    handle = AsyncJSHandle(cast(Any, object()), cast(Any, NativeHandle()))

    # Act
    value = str(handle)

    # Assert
    assert value == "Window"
    assert repr(handle) == "<JSHandle preview=Window>"


def test_sync_js_handle_exposes_the_async_preview_compatibility_view() -> None:
    # Arrange
    class NativeHandle:
        @staticmethod
        def preview() -> str:
            return "JSHandle@object"

        @staticmethod
        def is_element() -> bool:
            return False

    async_handle = AsyncJSHandle(cast(Any, object()), cast(Any, NativeHandle()))

    # Act
    handle = SyncJSHandle(cast(Any, object()), async_handle)

    # Assert
    assert repr(handle) == "<JSHandle preview=JSHandle@object>"
    assert str(handle) == "JSHandle@object"
    assert handle._impl_obj is async_handle


@pytest.mark.asyncio
async def test_worker_console_waiter_and_listener_receive_one_message_identity(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Arrange
    monkeypatch.setattr(
        "pwright.async_api._native.selected_features",
        lambda: ["worker-events", "worker-console-events"],
    )
    listener_values: list[object] = []

    class Native:
        def page_default_timeout(self, _page_id: str, _navigation: bool) -> int:
            return 30_000

    class BrowserOwner:
        def __init__(self) -> None:
            self._native = Native()

    class ContextOwner:
        def __init__(self) -> None:
            self.browser = BrowserOwner()

    page = Page(cast(Any, ContextOwner()), "page", "frame", "about:blank")
    worker = Worker(
        page,
        "worker-1",
        "https://example.test/worker.js",
        cast(Any, object()),
    )
    message = ConsoleMessage(
        page,
        "log",
        "hello from worker",
        "[]",
        "",
        0,
        0,
        worker=worker,
    )
    worker.on("console", listener_values.append)

    # Act
    async with worker.expect_event("console") as event_info:
        worker._emit_worker_event("console", message)
    value = await event_info.value

    # Assert
    assert value is message
    assert listener_values == [message]
    assert message.worker is worker


@pytest.mark.asyncio
async def test_page_close_retires_worker_owner_before_session_close() -> None:
    # Arrange
    native_owner = cast(Any, object())
    close_events: list[Worker] = []

    class Native:
        async def close_page(self, _page_id: str) -> bool:
            return False

        async def worker_evaluate(
            self, _owner: object, _expression: str, _argument: object
        ) -> object:
            raise AssertionError("a retired Worker must not reach the native evaluator")

    class BrowserOwner:
        def __init__(self) -> None:
            self._native = Native()

    class ContextOwner:
        def __init__(self) -> None:
            self.browser = BrowserOwner()
            self._pages: dict[str, Page] = {}
            self._closed = False

    context = ContextOwner()
    page = Page(cast(Any, context), "page", "frame", "about:blank")
    context._pages["page"] = page
    worker = Worker(page, "worker-1", "https://example.test/worker.js", native_owner)
    page._workers["worker-1"] = worker
    worker.once("close", lambda value: close_events.append(cast(Worker, value)))

    # Act
    await page.close()
    worker._close()

    # Assert
    assert page._workers == {}
    assert worker._native is None
    assert close_events == [worker]
    with pytest.raises(pwright.TargetClosedError, match="has been closed"):
        await worker.evaluate("1+1")
    with pytest.raises(pwright.TargetClosedError, match="has been closed"):
        await worker.evaluate_handle("() => this")


def test_sync_worker_event_info_wraps_predicate_value_and_evaluation() -> None:
    # Arrange
    native_owner = cast(Any, object())

    class Native:
        def page_default_timeout(self, _page_id: str, _navigation: bool) -> int:
            return 30_000

        async def worker_evaluate(self, owner: object, expression: str, argument: object) -> object:
            assert owner is native_owner
            assert expression == "() => self.name"
            assert argument is None
            return "worker-name"

    class BrowserOwner:
        def __init__(self) -> None:
            self._native = Native()

    class ContextOwner:
        def __init__(self) -> None:
            self.browser = BrowserOwner()

    page = Page(cast(Any, ContextOwner()), "page", "frame", "about:blank")
    worker = Worker(
        page,
        "worker-1",
        "https://example.test/worker.js",
        native_owner,
    )
    bridge = _LoopThread()
    sync_page = SyncPage(bridge, page, cast(SyncBrowserContext, object()))
    predicate_values: list[SyncWorker] = []

    async def emit() -> None:
        page._emit_page_event("worker", worker)

    # Act
    try:
        with sync_page.expect_worker(
            lambda value: predicate_values.append(cast(SyncWorker, value)) is None
        ) as event_info:
            bridge.run(emit())
        value = cast(SyncWorker, event_info.value)
        result = value.evaluate("() => self.name")
    finally:
        bridge.close()

    # Assert
    assert predicate_values == [value]
    assert isinstance(value, SyncWorker)
    assert value.url == "https://example.test/worker.js"
    assert result == "worker-name"


@pytest.mark.asyncio
async def test_page_event_waiters_have_a_fixed_capacity() -> None:
    # Arrange
    page = Page(cast(Any, object()), "page", "frame", "about:blank")
    waiters = [page._arm_page_event("download", None) for _ in range(64)]

    # Act
    with pytest.raises(
        pwright.ResourceExhaustedError,
        match="Page event waiter capacity exceeded \\(64\\)",
    ):
        page._arm_page_event("download", None)

    # Assert
    assert len(page._event_waiters["download"]) == 64
    for waiter in waiters:
        waiter.cancel()
    page._event_waiters.clear()


@pytest.mark.asyncio
async def test_init_script_source_matches_playwright_missing_input_error() -> None:
    # Arrange
    invalid_script = {"foo": "bar"}

    # Act
    with pytest.raises(pwright.Error) as raised:
        await _init_script_source(invalid_script, None, source_url=True)

    # Assert
    assert str(raised.value) == "Either path or script parameter must be specified"


@pytest.mark.asyncio
async def test_async_environment_closes_with_zero_ffi_metrics() -> None:
    # Arrange
    manager = async_playwright()
    playwright = await manager.start()
    environment = playwright._environment

    # Act
    await playwright.stop()

    # Assert
    assert environment.metrics() == {
        "pending": 0,
        "queued": 0,
        "queued_commands": 0,
        "running": 0,
        "reserved_bytes": 0,
        "connects": 0,
        "commands": 0,
        "closes": 0,
        "watches": 0,
    }


@pytest.mark.asyncio
async def test_non_chromium_cdp_attach_is_explicitly_unsupported() -> None:
    # Arrange
    manager = async_playwright()
    playwright = await manager.start()

    # Act
    with pytest.raises(pwright.UnsupportedError, match="only available for Chromium") as raised:
        await playwright.firefox.connect_over_cdp("http://127.0.0.1:9222")

    # Assert
    assert raised.value.code == "PWRIGHT_UNSUPPORTED"
    await playwright.stop()


@pytest.mark.asyncio
async def test_invalid_timeout_is_rejected_before_native_work() -> None:
    # Arrange
    manager = async_playwright()
    playwright = await manager.start()

    # Act
    with pytest.raises(ValueError, match="finite"):
        await playwright.chromium.connect_over_cdp("http://127.0.0.1:9222", timeout=float("inf"))

    # Assert
    assert playwright._environment.metrics()["pending"] == 0
    await playwright.stop()


def test_sync_manager_joins_its_single_loop_thread() -> None:
    # Arrange
    before = {
        thread.ident for thread in threading.enumerate() if thread.name == "pwright-sync-loop"
    }

    # Act
    with sync_playwright() as playwright:
        assert playwright.chromium.name == "chromium"
        during = {
            thread.ident for thread in threading.enumerate() if thread.name == "pwright-sync-loop"
        }
        assert len(during - before) == 1
    for _ in range(10):
        if (
            not {
                thread.ident
                for thread in threading.enumerate()
                if thread.name == "pwright-sync-loop"
            }
            - before
        ):
            break
        asyncio.run(asyncio.sleep(0))

    # Assert
    after = {thread.ident for thread in threading.enumerate() if thread.name == "pwright-sync-loop"}
    assert after == before


def test_sync_bridge_maps_calls_after_stop_to_playwright_error() -> None:
    # Arrange
    bridge = _LoopThread()
    bridge.close()

    # Act
    with pytest.raises(
        pwright.Error,
        match="Event loop is closed! Is Playwright already stopped",
    ):
        bridge.run(asyncio.sleep(0))

    # Assert
    assert bridge._loop is None


def test_sync_bridge_error_stack_includes_the_calling_thread() -> None:
    # Arrange
    bridge = _LoopThread()

    async def fail_on_background_loop() -> None:
        error = pwright.Error("broken")
        error.stack = "native-stack"
        raise error

    # Act
    try:
        with pytest.raises(pwright.Error) as raised:
            bridge.run(fail_on_background_loop())
    finally:
        bridge.close()

    # Assert
    assert __file__ in raised.value.stack
    assert raised.value.stack.endswith("native-stack")


def test_sync_manager_rejects_a_second_active_owner_on_the_same_thread() -> None:
    # Arrange
    first = sync_playwright().start()

    # Act
    try:
        with pytest.raises(
            pwright.Error,
            match="using Playwright Sync API inside the asyncio loop",
        ):
            sync_playwright().start()
    finally:
        first.stop()

    # Assert
    with sync_playwright() as restarted:
        assert restarted.chromium.name == "chromium"


def test_sync_manager_exposes_and_drains_its_caller_dispatcher_loop() -> None:
    # Arrange
    playwright = sync_playwright().start()
    task = asyncio.create_task(asyncio.sleep(0, result="ready"))

    # Act
    while not task.done():
        playwright._dispatcher_fiber.switch()
    result = task.result()
    playwright.stop()

    # Assert
    assert result == "ready"
    with pytest.raises(RuntimeError, match="no running event loop"):
        asyncio.get_running_loop()


def test_sync_bridge_greenlets_overlap_background_coroutines() -> None:
    # Arrange
    playwright = sync_playwright().start()
    active = 0
    peak_active = 0
    results: dict[str, str] = {}
    errors: list[BaseException] = []
    root = greenlet.getcurrent()

    async def operation(label: str) -> str:
        nonlocal active, peak_active
        active += 1
        peak_active = max(peak_active, active)
        await asyncio.sleep(0.01)
        active -= 1
        return label

    def action(label: str) -> Callable[[], None]:
        def body() -> None:
            try:
                results[label] = playwright._bridge.run(operation(label))
            except BaseException as error:
                errors.append(error)
            root.switch()

        return body

    async def schedule_actions() -> None:
        for label in ("first", "second"):
            greenlet(action(label)).switch()

    # Act
    asyncio.create_task(schedule_actions())
    while len(results) + len(errors) < 2:
        playwright._dispatcher_fiber.switch()
    playwright.stop()

    # Assert
    assert errors == []
    assert results == {"first": "first", "second": "second"}
    assert peak_active == 2


@pytest.mark.asyncio
async def test_sync_manager_rejects_a_running_asyncio_loop() -> None:
    # Arrange
    manager = sync_playwright()

    # Act
    with pytest.raises(
        pwright.Error,
        match="using Playwright Sync API inside the asyncio loop",
    ):
        manager.start()

    # Assert
    assert manager._bridge is None
