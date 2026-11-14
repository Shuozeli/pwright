from collections.abc import Awaitable, Callable, Sequence
from re import Pattern
from typing import Any, Literal

class Error(Exception):
    code: str
    @property
    def message(self) -> str: ...
    name: str
    stack: str

class TimeoutError(Error):
    code: str

class TargetClosedError(Error):
    code: str

class ResourceExhaustedError(Error):
    code: str

class JavascriptError(Error):
    code: str

class InvalidArgumentError(ValueError):
    code: str

class UnsupportedError(NotImplementedError):
    code: str

class NativeDownload: ...
class NativeWorker: ...

NativeApiCookie = tuple[str, str, str, str, float, bool, bool, str]
NativeContextCookie = tuple[
    str,
    str,
    str,
    str,
    float,
    bool,
    bool,
    Literal["Lax", "None", "Strict"],
    str | None,
]
NativeContextCookieInput = tuple[
    str,
    str,
    str | None,
    str | None,
    str | None,
    float | None,
    bool | None,
    bool | None,
    str | None,
    str | None,
]

class NativeApiResponse:
    @property
    def url(self) -> str: ...
    @property
    def status(self) -> int: ...
    @property
    def status_text(self) -> str: ...
    @property
    def ok(self) -> bool: ...
    def headers_array(self) -> list[tuple[str, str]]: ...
    def body(self) -> bytes: ...
    def log(self) -> list[str]: ...
    def dispose(self) -> None: ...

class NativeApiRequestContext:
    def __init__(
        self,
        options: tuple[
            str | None,
            str,
            list[tuple[str, str]],
            int,
            bool,
            bool,
            int,
        ],
        credentials: tuple[str, str, str | None, bool] | None,
        client_certificates: list[tuple[str, bytes | None, bytes | None, bytes | None, str | None]],
        storage_origins_json: str,
    ) -> None: ...
    @property
    def storage_origins_json(self) -> str: ...
    def execute(
        self,
        method: str,
        url: str,
        headers: list[tuple[str, str]],
        query: tuple[str | None, list[tuple[str, str]]],
        body: tuple[
            int,
            bytes | None,
            str | None,
            list[tuple[str, str]],
            list[tuple[str, str | None, str | None, str | None, bytes | None]],
        ],
        controls: tuple[
            int | None,
            bool | None,
            bool | None,
            int | None,
            int,
            list[str] | None,
        ],
    ) -> Awaitable[NativeApiResponse]: ...
    def storage_cookies(self) -> Awaitable[list[NativeApiCookie]]: ...
    def add_cookies(self, cookies: list[NativeApiCookie]) -> Awaitable[None]: ...
    def dispose(self, reason: str | None) -> None: ...
    def metrics(self) -> dict[str, int]: ...

class NativeHandle:
    def is_element(self) -> bool: ...
    def preview(self) -> str: ...

class NativeLocator:
    def locator(self, selector: str) -> NativeLocator: ...
    def nth(self, index: int) -> NativeLocator: ...
    def frame_locator(self, selector: str) -> NativeFrameLocator: ...
    def content_frame(self) -> NativeFrameLocator: ...
    def visible(self, visible: bool) -> NativeLocator: ...
    def filter_text(self, text: str, exclude: bool) -> NativeLocator: ...
    def filter_regex(self, source: str, flags: str, exclude: bool) -> NativeLocator: ...
    def combine(self, other: NativeLocator, kind: str) -> NativeLocator: ...
    def semantic(
        self,
        kind: str,
        source: str,
        exact: bool = ...,
        flags: str | None = ...,
    ) -> NativeLocator: ...
    def role(
        self,
        role: str,
        name: str | None = ...,
        name_flags: str | None = ...,
        exact: bool = ...,
        checked: bool | None = ...,
        disabled: bool | None = ...,
        expanded: bool | None = ...,
        include_hidden: bool = ...,
        level: int | None = ...,
        pressed: bool | None = ...,
        selected: bool | None = ...,
    ) -> NativeLocator: ...
    def click(
        self,
        timeout_ms: int = ...,
        steps: int | None = ...,
        force: bool | None = ...,
    ) -> Awaitable[None]: ...
    def tap(
        self,
        timeout_ms: int = ...,
        options: dict[str, object] | None = ...,
    ) -> Awaitable[None]: ...
    def dblclick(self, timeout_ms: int = ...) -> Awaitable[None]: ...
    def bounding_box(self, timeout_ms: int = ...) -> Awaitable[dict[str, float] | None]: ...
    def screenshot(
        self,
        format: int = ...,
        path: str | None = ...,
        quality: int | None = ...,
        scale: int = ...,
        timeout_ms: int = ...,
        mask: list[NativeLocator] | None = ...,
    ) -> Awaitable[bytes]: ...
    def set_input_files_paths(
        self,
        base: str,
        paths: list[str],
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def set_input_file_payloads(
        self,
        payloads: list[dict[str, object]],
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def element_handle(self, timeout_ms: int = ...) -> Awaitable[NativeHandle]: ...
    def highlight(self, description: str) -> Awaitable[None]: ...
    def dispatch_event(
        self,
        event_type: str,
        event_init: dict[str, object] | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def dispatch_event_owned(
        self,
        event_type: str,
        event_init: dict[str, object] | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def select_text(self, timeout_ms: int = ..., force: bool = ...) -> Awaitable[None]: ...
    def fill(self, value: str, timeout_ms: int = ..., force: bool = ...) -> Awaitable[None]: ...
    def press(
        self,
        key: str,
        timeout_ms: int = ...,
        delay_ms: int | None = ...,
        no_wait_after: bool = ...,
    ) -> Awaitable[None]: ...
    def set_checked(
        self,
        checked: bool,
        timeout_ms: int = ...,
        force: bool | None = ...,
        trial: bool | None = ...,
    ) -> Awaitable[None]: ...
    def focus(self, timeout_ms: int = ...) -> Awaitable[None]: ...
    def blur(self, timeout_ms: int = ...) -> Awaitable[None]: ...
    def hover(
        self,
        timeout_ms: int = ...,
        force: bool = ...,
        trial: bool = ...,
    ) -> Awaitable[None]: ...
    def scroll_into_view_if_needed(self, timeout_ms: int = ...) -> Awaitable[None]: ...
    def press_sequentially(
        self,
        text: str,
        timeout_ms: int = ...,
        delay_ms: int | None = ...,
    ) -> Awaitable[None]: ...
    def select_option(
        self,
        value: str,
        timeout_ms: int = ...,
        force: bool | None = ...,
    ) -> Awaitable[list[str]]: ...
    def select_options(
        self,
        values: list[object],
        timeout_ms: int = ...,
        force: bool | None = ...,
        element: NativeHandle | None = ...,
    ) -> Awaitable[list[str]]: ...
    def read(
        self,
        operation: str,
        timeout_ms: int = ...,
        attribute: str | None = ...,
    ) -> Awaitable[object]: ...
    def evaluate(
        self,
        source: str,
        argument: object | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[object]: ...
    def evaluate_all(
        self,
        source: str,
        argument: object | None = ...,
    ) -> Awaitable[object]: ...
    def evaluate_handle(
        self,
        source: str,
        argument: object | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[NativeHandle]: ...
    def wait_for(self, state: str = ..., timeout_ms: int = ...) -> Awaitable[None]: ...
    def wait_for_element(
        self, state: str = ..., timeout_ms: int = ...
    ) -> Awaitable[NativeHandle | None]: ...
    def expect_visibility(
        self,
        visible: bool = ...,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_text(
        self,
        expected: str,
        timeout_ms: int = ...,
        is_not: bool = ...,
        substring: bool = ...,
        normalize_whitespace: bool = ...,
        ignore_case: bool = ...,
        use_inner_text: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_text_regex(
        self,
        source: str,
        flags: str = ...,
        ignore_case: bool | None = ...,
        use_inner_text: bool = ...,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_texts(
        self,
        expected: list[str],
        timeout_ms: int = ...,
        is_not: bool = ...,
        substring: bool = ...,
        normalize_whitespace: bool = ...,
        ignore_case: bool = ...,
        use_inner_text: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_string(
        self,
        kind: str,
        expected: str,
        flags: str | None = ...,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_class_names(
        self,
        expected: list[str],
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_class_matchers(
        self,
        expected: list[tuple[str, str | None]],
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_string_property(
        self,
        name: str,
        expected: str,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_state(
        self,
        state: str,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_checked(
        self,
        checked: bool | None = ...,
        indeterminate: bool | None = ...,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_property(
        self,
        name: str,
        expected: object,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_attribute(
        self,
        name: str,
        expected: str,
        match_options: tuple[str | None, bool | None] = ...,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_css(
        self,
        name: str,
        expected: str,
        flags: str | None = ...,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def expect_count(
        self,
        expected: int,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...

class NativeFrameLocator:
    def locator(self, selector: str) -> NativeLocator: ...
    def locator_native(self, locator: NativeLocator) -> NativeLocator: ...
    def semantic(
        self,
        kind: str,
        source: str,
        exact: bool = ...,
        flags: str | None = ...,
    ) -> NativeLocator: ...
    def role(
        self,
        role: str,
        name: str | None = ...,
        name_flags: str | None = ...,
        exact: bool = ...,
        checked: bool | None = ...,
        disabled: bool | None = ...,
        expanded: bool | None = ...,
        include_hidden: bool = ...,
        level: int | None = ...,
        pressed: bool | None = ...,
        selected: bool | None = ...,
    ) -> NativeLocator: ...
    def frame_locator(self, selector: str) -> NativeFrameLocator: ...
    def owner(self) -> NativeLocator: ...
    def nth(self, index: int) -> NativeFrameLocator: ...

class NativeRequestSizeOwner: ...
class NativeResponseBody: ...
class NativeResponseFinishedOwner: ...
class NativeRequestHistoryOwner: ...
class NativeDialog: ...

class NativeFileChooser:
    def is_element(self) -> bool: ...
    def preview(self) -> str: ...

class NativeCdpSessionLease:
    def token(self) -> int: ...
    def claim(self) -> int: ...

class NativeCdpEventBatch:
    def peek(self) -> tuple[str, dict[str, object]] | None: ...
    def ack_next(self) -> None: ...
    def rollback(self) -> None: ...

class NativeSession:
    @property
    def version(self) -> str: ...
    def is_connected(self) -> bool: ...
    def context_request(self, context_id: str) -> NativeApiRequestContext: ...
    def cdp_session_create(
        self, context_id: str, page_id: str, frame_id: str | None
    ) -> Awaitable[NativeCdpSessionLease]: ...
    def cdp_session_send(
        self, token: int, method: str, params_json: str
    ) -> Awaitable[dict[str, object]]: ...
    def cdp_session_claim(self, token: int, max_events: int) -> Awaitable[NativeCdpEventBatch]: ...
    def next_cdp_session_event_generation(self, after: int) -> Awaitable[int]: ...
    def cdp_session_detach(self, token: int) -> Awaitable[None]: ...
    def locator(self, frame_id: str, selector: str) -> NativeLocator: ...
    def frame_locator(self, frame_id: str, selector: str) -> NativeFrameLocator: ...
    def semantic_locator(
        self,
        frame_id: str,
        kind: str,
        source: str,
        exact: bool = ...,
        flags: str | None = ...,
    ) -> NativeLocator: ...
    def role_locator(
        self,
        frame_id: str,
        role: str,
        name: str | None = ...,
        name_flags: str | None = ...,
        exact: bool = ...,
        checked: bool | None = ...,
        disabled: bool | None = ...,
        expanded: bool | None = ...,
        include_hidden: bool = ...,
        level: int | None = ...,
        pressed: bool | None = ...,
        selected: bool | None = ...,
    ) -> NativeLocator: ...
    def set_lifecycle_callback(
        self,
        callback: Callable[
            [
                int,
                str,
                str | None,
                str | None,
                str | None,
                str | None,
                str | None,
                tuple[str, str, str] | None,
                tuple[str, str, str, str, int, int, int, list[NativeHandle]] | None,
                tuple[NativeDialog, str, str, str] | None,
                tuple[NativeFileChooser, bool] | None,
                str | None,
            ],
            None,
        ],
    ) -> None: ...
    def next_lifecycle_notices(self) -> Awaitable[bool]: ...
    def flush_lifecycle_events(self) -> Awaitable[None]: ...
    def set_page_file_chooser_interception(
        self, page_id: str, enabled: bool
    ) -> Awaitable[None]: ...
    def set_page_console_listener(self, page_id: str, enabled: bool) -> None: ...
    def set_context_console_listener(self, context_id: str, enabled: bool) -> None: ...
    def subscribe_page_console(self, page_id: str) -> int: ...
    def subscribe_context_console(self, context_id: str) -> int: ...
    def release_console_subscription(self, token: int) -> None: ...
    def context_cookies(
        self, context_id: str, urls: list[str]
    ) -> Awaitable[list[NativeContextCookie]]: ...
    def context_add_cookies(
        self, context_id: str, cookies: list[NativeContextCookieInput]
    ) -> Awaitable[None]: ...
    def context_clear_cookies(
        self,
        context_id: str,
        name: str | None = ...,
        name_regex_flags: str | None = ...,
        domain: str | None = ...,
        domain_regex_flags: str | None = ...,
        path: str | None = ...,
        path_regex_flags: str | None = ...,
    ) -> Awaitable[None]: ...
    def context_storage_state(
        self, context_id: str, indexed_db: bool
    ) -> Awaitable[dict[str, Any]]: ...
    def context_set_storage_state(
        self, context_id: str, storage_state_json: str
    ) -> Awaitable[None]: ...
    def contexts(self) -> list[tuple[str, str | None]]: ...
    def pages(self, context_id: str) -> list[tuple[str, str, str, str]]: ...
    def frames(self, page_id: str) -> list[tuple[str, str | None, str, str]]: ...
    def page_errors(self, page_id: str) -> list[tuple[str, str, str]]: ...
    def console_messages(self, page_id: str) -> list[tuple[str, str, str, str, int, int]]: ...
    def page_requests(self, page_id: str) -> list[dict[str, Any]]: ...
    def set_context_default_timeout(
        self, context_id: str, navigation: bool, timeout_ms: int
    ) -> None: ...
    def context_default_timeout(self, context_id: str, navigation: bool) -> int: ...
    def set_page_default_timeout(self, page_id: str, navigation: bool, timeout_ms: int) -> None: ...
    def page_default_timeout(self, page_id: str, navigation: bool) -> int: ...
    def page_pause(self, page_id: str) -> Awaitable[None]: ...
    def wait_for_timeout(self, timeout_ms: int) -> Awaitable[None]: ...
    def page_request_gc(self, page_id: str) -> Awaitable[None]: ...
    def page_pdf(
        self,
        page_id: str,
        path: str | None = ...,
        tagged: bool = ...,
        outline: bool = ...,
    ) -> Awaitable[bytes]: ...
    def page_screenshot(
        self,
        page_id: str,
        format: int = ...,
        path: str | None = ...,
        quality: int | None = ...,
        full_page: bool = ...,
        scale: int = ...,
        timeout_ms: int = ...,
        mask: list[NativeLocator] | None = ...,
    ) -> Awaitable[bytes]: ...
    def element_screenshot(
        self,
        element: NativeHandle,
        format: int = ...,
        path: str | None = ...,
        quality: int | None = ...,
        scale: int = ...,
        timeout_ms: int = ...,
        mask: list[NativeLocator] | None = ...,
    ) -> Awaitable[bytes]: ...
    def page_set_viewport_size(self, page_id: str, width: int, height: int) -> Awaitable[None]: ...
    def request_history_response(
        self, owner: NativeRequestHistoryOwner
    ) -> Awaitable[dict[str, Any] | None]: ...
    def request_response_by_native_id(
        self, page_id: str, native_request_id: str
    ) -> Awaitable[dict[str, Any] | None]: ...
    def request_by_native_id(
        self, page_id: str, native_request_id: str
    ) -> Awaitable[dict[str, Any]]: ...
    def new_context(
        self,
        base_url: str | None = ...,
        device: tuple[
            str | None,
            tuple[int, int] | None,
            tuple[int, int] | None,
            float | None,
            bool | None,
            bool | None,
        ]
        | None = ...,
        locale: str | None = ...,
        timezone_id: str | None = ...,
        geolocation: tuple[float, float, float] | None = ...,
        geolocation_permission: bool = ...,
        http_credentials: tuple[str, str, str | None, bool] | None = ...,
        ignore_https_errors: bool = ...,
        offline: bool = ...,
        bypass_csp: bool = ...,
        javascript_enabled: bool = ...,
        accept_downloads: bool = ...,
        block_service_workers: bool = ...,
        forced_colors: str | None = ...,
        strict_selectors: bool = ...,
        extra_http_headers: list[tuple[str, str]] | None = ...,
    ) -> Awaitable[tuple[str, str | None]]: ...
    def new_page(self, context_id: str) -> Awaitable[tuple[str, str, str, str]]: ...
    def wait_for_navigation(
        self,
        page_id: str,
        frame_id: str,
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[dict[str, Any] | None]: ...
    def wait_for_navigation_matching(
        self,
        page_id: str,
        frame_id: str,
        url: str | Pattern[str] | Callable[[str], bool],
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[dict[str, Any] | None]: ...
    def wait_for_load_state(
        self,
        page_id: str,
        frame_id: str,
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def new_browser_page(
        self, base_url: str | None, accept_downloads: bool
    ) -> Awaitable[tuple[tuple[str, str | None], tuple[str, str, str, str]]]: ...
    def title(self, frame_id: str) -> Awaitable[str]: ...
    def content(self, frame_id: str) -> Awaitable[str]: ...
    def set_content(
        self,
        page_id: str,
        html: str,
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[str]: ...
    def emulate_media(
        self,
        page_id: str,
        options: tuple[str | None, str | None, str | None, str | None, str | None] | None = ...,
    ) -> Awaitable[None]: ...
    def click(self, frame_id: str, selector: str, timeout_ms: int = ...) -> Awaitable[None]: ...
    def click_with_options(
        self,
        frame_id: str,
        selector: str,
        options: dict[str, object],
        strict: bool = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def page_tap(
        self,
        frame_id: str,
        selector: str,
        options: dict[str, object],
        strict: bool = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def click_with_modifiers(
        self, frame_id: str, selector: str, modifiers: list[str], timeout_ms: int = ...
    ) -> Awaitable[None]: ...
    def mouse(
        self,
        page_id: str,
        operation: str,
        x: float,
        y: float,
        options: tuple[float, float, float, float] = ...,
    ) -> Awaitable[None]: ...
    def touchscreen_tap(self, page_id: str, x: float, y: float) -> Awaitable[None]: ...
    def keyboard(
        self,
        page_id: str,
        operation: str,
        value: str,
        delay_ms: float = ...,
    ) -> Awaitable[None]: ...
    def drag_and_drop(
        self,
        page_id: str,
        frame_id: str,
        source: str,
        target: str,
        timeout_ms: int = ...,
        options: tuple[
            tuple[float, float] | None,
            tuple[float, float] | None,
            float,
            bool,
            bool,
            bool,
        ]
        | None = ...,
    ) -> Awaitable[None]: ...
    def fill(
        self,
        frame_id: str,
        selector: str,
        value: str,
        timeout_ms: int = ...,
        force: bool = ...,
    ) -> Awaitable[None]: ...
    def press(
        self,
        frame_id: str,
        selector: str,
        key: str,
        timeout_ms: int = ...,
        options: tuple[int | None, bool] | None = ...,
    ) -> Awaitable[None]: ...
    def set_checked(
        self,
        frame_id: str,
        selector: str,
        checked: bool,
        timeout_ms: int = ...,
        options: tuple[bool | None, bool | None] | None = ...,
    ) -> Awaitable[None]: ...
    def focus(self, frame_id: str, selector: str, timeout_ms: int = ...) -> Awaitable[None]: ...
    def context_selector_is_strict(self, context_id: str, strict: bool | None = ...) -> bool: ...
    def locator_read(
        self,
        frame_id: str,
        selector: str,
        operation: str,
        timeout_ms: int = ...,
        attribute: str | None = ...,
        strict: bool = ...,
    ) -> Awaitable[object]: ...
    def locator_expect_count(
        self,
        frame_id: str,
        selector: str,
        expected: int,
        timeout_ms: int = ...,
        is_not: bool = ...,
    ) -> Awaitable[dict[str, object]]: ...
    def eval_on_selector(
        self,
        frame_id: str,
        selector: str,
        source: str,
        argument: object | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[object]: ...
    def eval_on_selector_all(
        self,
        frame_id: str,
        selector: str,
        source: str,
        argument: object | None = ...,
    ) -> Awaitable[object]: ...
    def evaluate(
        self, frame_id: str, source: str, argument: object | None = ...
    ) -> Awaitable[object]: ...
    def frame_evaluate_handle(
        self, frame_id: str, source: str, argument: object | None = ...
    ) -> Awaitable[NativeHandle]: ...
    def frame_wait_for_function(
        self,
        frame_id: str,
        source: str,
        argument: object | None = ...,
        polling_ms: int = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[NativeHandle]: ...
    def evaluate_handle_argument(
        self, frame_id: str, source: str, handle: NativeHandle
    ) -> Awaitable[object]: ...
    def handle_evaluate(
        self, handle: NativeHandle, source: str, argument: object | None = ...
    ) -> Awaitable[object]: ...
    def handle_evaluate_handle(
        self, handle: NativeHandle, source: str, argument: object | None = ...
    ) -> Awaitable[NativeHandle]: ...
    def handle_get_property(self, handle: NativeHandle, name: str) -> Awaitable[NativeHandle]: ...
    def handle_get_properties(self, handle: NativeHandle) -> Awaitable[dict[str, NativeHandle]]: ...
    def element_handle_dispatch_event(
        self,
        handle: NativeHandle,
        event_type: str,
        event_init: dict[str, object] | None = ...,
    ) -> Awaitable[None]: ...
    def element_handle_click(
        self,
        handle: NativeHandle,
        options: dict[str, object],
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def element_handle_tap(
        self,
        handle: NativeHandle,
        options: dict[str, object],
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def element_handle_wait_for_element_state(
        self, handle: NativeHandle, state: str, timeout_ms: int = ...
    ) -> Awaitable[None]: ...
    def element_handle_scroll_into_view_if_needed(
        self, handle: NativeHandle, timeout_ms: int = ...
    ) -> Awaitable[None]: ...
    def element_handle_select_text(
        self, handle: NativeHandle, force: bool = ..., timeout_ms: int = ...
    ) -> Awaitable[None]: ...
    def element_handle_state_predicate(
        self, handle: NativeHandle, operation: str
    ) -> Awaitable[bool]: ...
    def element_handle_dispatch_event_owned(
        self,
        handle: NativeHandle,
        event_type: str,
        event_init: dict[str, object] | None = ...,
    ) -> Awaitable[None]: ...
    def element_handle_inner_text(self, handle: NativeHandle) -> Awaitable[str]: ...
    def element_handle_dom_read(
        self, handle: NativeHandle, operation: str, attribute: str | None = ...
    ) -> Awaitable[str | bool | None]: ...
    def element_handle_bounding_box(
        self, handle: NativeHandle
    ) -> Awaitable[dict[str, float] | None]: ...
    def element_handle_query_selector(
        self, handle: NativeHandle, selector: str
    ) -> Awaitable[NativeHandle | None]: ...
    def element_handle_set_input_files_paths(
        self,
        handle: NativeHandle,
        base: str,
        paths: list[str],
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def element_handle_set_input_file_payloads(
        self,
        handle: NativeHandle,
        payloads: list[dict[str, object]],
        timeout_ms: int = ...,
    ) -> Awaitable[None]: ...
    def element_handle_owner_frame(self, handle: NativeHandle) -> Awaitable[str | None]: ...
    def element_handle_content_frame(self, handle: NativeHandle) -> Awaitable[str | None]: ...
    def element_handle_keyboard(
        self,
        handle: NativeHandle,
        page_id: str,
        operation: str,
        value: str,
        delay_ms: float = ...,
    ) -> Awaitable[None]: ...
    def handle_json_value(self, handle: NativeHandle) -> Awaitable[object]: ...
    def handle_dispose(self, handle: NativeHandle) -> Awaitable[None]: ...
    def page_expose_binding(
        self, page_id: str, name: str, needs_handle: bool
    ) -> Awaitable[None]: ...
    def context_expose_binding(
        self, context_id: str, name: str, needs_handle: bool
    ) -> Awaitable[None]: ...
    def next_page_binding_call(
        self,
    ) -> Awaitable[tuple[str, str, str, str, str, str, bool] | None]: ...
    def acknowledge_page_binding_call(self, call_id: str) -> Awaitable[None]: ...
    def take_page_binding_handle(self, call_id: str) -> Awaitable[NativeHandle]: ...
    def deliver_page_binding_result(self, call_id: str, value: object) -> Awaitable[None]: ...
    def deliver_page_binding_error(
        self,
        call_id: str,
        name: str,
        message: str,
        stack: str | None,
    ) -> Awaitable[None]: ...
    def set_page_extra_http_headers(
        self, page_id: str, headers: list[tuple[str, str]]
    ) -> Awaitable[None]: ...
    def set_context_extra_http_headers(
        self, context_id: str, headers: list[tuple[str, str]]
    ) -> Awaitable[None]: ...
    def set_context_offline(self, context_id: str, offline: bool) -> Awaitable[None]: ...
    def page_route(self, page_id: str, pattern: str, times: int | None = ...) -> Awaitable[str]: ...
    def context_route(
        self, context_id: str, pattern: str, times: int | None = ...
    ) -> Awaitable[str]: ...
    def page_unroute(self, page_id: str, registration_ids: Sequence[int]) -> Awaitable[None]: ...
    def context_unroute(
        self, context_id: str, registration_ids: Sequence[int]
    ) -> Awaitable[None]: ...
    def page_unroute_all(self, page_id: str, wait: bool) -> Awaitable[None]: ...
    def context_unroute_all(self, context_id: str, wait: bool) -> Awaitable[None]: ...
    def page_route_from_har(
        self,
        page_id: str,
        path: str,
        pattern: str | None,
        not_found: str | None,
    ) -> Awaitable[str]: ...
    def context_route_from_har(
        self,
        context_id: str,
        path: str,
        pattern: str | None,
        not_found: str | None,
    ) -> Awaitable[str]: ...
    def next_page_route_call(
        self,
    ) -> Awaitable[
        tuple[
            str,
            str,
            str,
            str,
            str | None,
            str,
            str,
            str,
            list[tuple[str, str]],
            bytes | None,
            bool,
            bool,
            str | None,
        ]
        | None
    ]: ...
    def page_route_continue(
        self,
        call_id: str,
        url: str | None,
        method: str | None,
        headers: list[tuple[str, str]] | None,
        post_data: bytes | None,
    ) -> Awaitable[None]: ...
    def page_route_abort(self, call_id: str, error_code: str | None = ...) -> Awaitable[None]: ...
    def page_route_fulfill(
        self,
        call_id: str,
        status: int,
        headers: list[tuple[str, str]],
        body: bytes,
    ) -> Awaitable[None]: ...
    def page_route_fulfill_response(
        self,
        call_id: str,
        response: NativeApiResponse,
        status: int | None,
        headers: list[tuple[str, str]] | None,
        content_type: str | None,
    ) -> Awaitable[None]: ...
    def page_route_fulfill_path(
        self,
        call_id: str,
        status: int,
        headers: list[tuple[str, str]],
        path: str,
    ) -> Awaitable[None]: ...
    def goto(
        self,
        page_id: str,
        url: str,
        wait_until: str = ...,
        referrer: str | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[dict[str, Any]]: ...
    def frame_goto(
        self,
        page_id: str,
        frame_id: str,
        url: str,
        wait_until: str = ...,
        referrer: str | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[dict[str, Any] | None]: ...
    def reload(
        self,
        page_id: str,
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[dict[str, Any]]: ...
    def go_back(
        self,
        page_id: str,
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[dict[str, Any] | None]: ...
    def go_forward(
        self,
        page_id: str,
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[dict[str, Any] | None]: ...
    def wait_for_url(
        self,
        page_id: str,
        url: object,
        wait_until: str = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[str]: ...
    def page_add_init_script(self, page_id: str, source: str) -> Awaitable[None]: ...
    def context_add_init_script(self, context_id: str, source: str) -> Awaitable[None]: ...
    def arm_network_wait(
        self, page_id: str, kind: str, timeout_ms: int = ...
    ) -> Awaitable[int]: ...
    def arm_context_network_wait(
        self, context_id: str, kind: str, timeout_ms: int = ...
    ) -> Awaitable[int]: ...
    def next_network_event(self, waiter_id: int) -> Awaitable[dict[str, Any]]: ...
    def next_request_notice(self) -> Awaitable[tuple[str, int, dict[str, Any]]]: ...
    def next_response_notice(self) -> Awaitable[tuple[str, int, dict[str, Any]]]: ...
    def network_notice_fence(self) -> tuple[int, int]: ...
    def next_download_notice(self) -> Awaitable[tuple[str, str, str, NativeDownload]]: ...
    def download_path(self, download: NativeDownload) -> Awaitable[str]: ...
    def next_worker_notice(
        self,
    ) -> Awaitable[tuple[int, str, str, str, NativeWorker]]: ...
    def next_worker_console_notice(
        self,
    ) -> Awaitable[
        tuple[str, str, str, NativeWorker, str, str, str, str, int, int, int]
        | tuple[
            str,
            str,
            str,
            NativeWorker,
            str,
            str,
            str,
            str,
            int,
            int,
            int,
            list[NativeHandle],
        ]
    ]: ...
    def page_route_fallback(
        self,
        call_id: str,
        url: str | None = ...,
        method: str | None = ...,
        headers: Sequence[tuple[str, str]] | None = ...,
        post_data: bytes | None = ...,
    ) -> Awaitable[None]: ...
    def page_workers(self, page_id: str) -> list[tuple[str, str, NativeWorker]]: ...
    def worker_evaluate(
        self, worker: NativeWorker, source: str, argument: object | None = ...
    ) -> Awaitable[object]: ...
    def worker_evaluate_handle(
        self, worker: NativeWorker, source: str, argument: object | None = ...
    ) -> Awaitable[NativeHandle]: ...
    def cancel_network_wait(self, waiter_id: int) -> Awaitable[None]: ...
    def validate_selector(self, frame_id: str, selector: str) -> None: ...
    def registered_selector_match_count(
        self, frame_id: str, selector: str
    ) -> Awaitable[int | None]: ...
    def request_sizes(self, owner: NativeRequestSizeOwner) -> Awaitable[dict[str, int]]: ...
    def response_body(self, owner: NativeResponseBody) -> Awaitable[bytes]: ...
    def frame_response_body(
        self, page_id: str, frame_id: str, request_id: str
    ) -> Awaitable[bytes]: ...
    def response_finished(self, owner: NativeResponseFinishedOwner) -> Awaitable[None]: ...
    def close_page(self, page_id: str) -> Awaitable[bool]: ...
    def run_before_unload(self, page_id: str) -> Awaitable[None]: ...
    def handle_dialog(
        self,
        dialog: NativeDialog,
        accept: bool,
        prompt_text: str | None = ...,
    ) -> Awaitable[None]: ...
    def set_file_chooser_payloads(
        self,
        chooser: NativeFileChooser,
        payloads: list[dict[str, object]],
    ) -> Awaitable[None]: ...
    def set_file_chooser_paths(
        self,
        chooser: NativeFileChooser,
        base: str,
        paths: list[str],
    ) -> Awaitable[None]: ...
    def close_context(self, context_id: str) -> Awaitable[None]: ...
    def close(self) -> Awaitable[None]: ...
    def metrics(self) -> dict[str, int | bool]: ...

class NativeRemoteBrowser:
    @property
    def guid(self) -> str: ...
    @property
    def version(self) -> str: ...
    def is_connected(self) -> bool: ...
    def live_contexts(self) -> list[tuple[str, int]]: ...
    def new_context(self) -> Awaitable[NativeRemoteContext]: ...
    def wait_disconnected(self) -> Awaitable[None]: ...
    def next_close_notice(self) -> Awaitable[tuple[str, str | None, int | None]]: ...
    def close(self) -> Awaitable[None]: ...

class NativeRemoteContext:
    @property
    def guid(self) -> str: ...
    @property
    def created_sequence(self) -> int: ...
    def is_open(self) -> bool: ...
    def live_pages(self) -> list[tuple[str, int]]: ...
    def new_page(self) -> Awaitable[NativeRemotePage]: ...
    def close(self) -> Awaitable[None]: ...

class NativeRemotePage:
    @property
    def guid(self) -> str: ...
    @property
    def created_sequence(self) -> int: ...
    def is_open(self) -> bool: ...
    def title(self) -> Awaitable[str]: ...

class NativeEnvironment:
    def __init__(self) -> None: ...
    def register_selector_engine(self, name: str, source: str, content_script: bool) -> None: ...
    def set_test_id_attribute(self, name: str) -> None: ...
    def connect_over_cdp(
        self,
        endpoint: str,
        timeout_ms: int = ...,
        headers: list[tuple[str, str]] | None = ...,
    ) -> Awaitable[NativeSession]: ...
    def launch_chromium(
        self,
        headless: bool = ...,
        executable_path: str | None = ...,
        channel: str | None = ...,
        timeout_ms: int = ...,
    ) -> Awaitable[NativeSession]: ...
    def observe_nonchromium_startup(
        self, engine: str, executable_path: str, timeout_ms: int = ...
    ) -> Awaitable[None]: ...
    def observe_playwright_server_initialize(
        self, endpoint_url: str, timeout_ms: int = ...
    ) -> Awaitable[None]: ...
    def connect_playwright_server(
        self, endpoint_url: str, browser_name: str, timeout_ms: int = ...
    ) -> Awaitable[NativeRemoteBrowser]: ...
    def launch_persistent_context(
        self,
        user_data_dir: str,
        headless: bool,
        options: tuple[
            str | None,
            tuple[
                str | None,
                str | None,
                tuple[float, float, float] | None,
                bool,
                tuple[str, str, str | None, bool] | None,
                bool,
                bool,
                bool,
                bool,
            ],
            tuple[
                str | None,
                tuple[int, int] | None,
                tuple[int, int] | None,
                float | None,
                bool | None,
                bool | None,
            ]
            | None,
            tuple[str | None, str | None, str | None] | None,
            list[tuple[str, str]] | None,
            list[str],
            str | None,
            str | None,
            int,
        ],
    ) -> Awaitable[NativeSession]: ...
    def close(self) -> Awaitable[None]: ...
    def metrics(self) -> dict[str, int]: ...
    def cleanup_report(self) -> dict[str, int | bool]: ...

def native_abi_version() -> int: ...
def package_version() -> str: ...
def device_descriptors_json() -> str: ...
def browser_executable_path(engine: str, package_root: str) -> str: ...
def build_profile() -> str: ...
def selected_features() -> list[str]: ...
def resource_limits() -> dict[str, int]: ...
