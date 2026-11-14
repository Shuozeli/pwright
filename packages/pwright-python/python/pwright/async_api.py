# pyright: reportPrivateUsage=false
"""Async Playwright-shaped facade with browser behavior delegated to Rust."""

from __future__ import annotations

import asyncio
import contextlib
import contextvars
import fnmatch
import inspect
import json
import math
import os
import re
import stat
import time
import traceback
import weakref
from collections.abc import Awaitable, Callable, Mapping, Sequence
from pathlib import Path
from re import Pattern
from typing import TYPE_CHECKING, Any, Generic, Literal, TypedDict, TypeVar, Union, cast
from urllib.parse import ParseResult, urljoin, urlsplit

from . import _native
from ._async_api_request import (
    APIRequest,
    APIRequestContext,
    APIResponse,
    ClientCertificate,
    StorageState,
    StorageStateCookie,
)
from ._errors import (
    Error,
    InvalidArgumentError,
    ResourceExhaustedError,
    TargetClosedError,
    TimeoutError,
    UnsupportedError,
    rewrite_api_error,
)

if TYPE_CHECKING:
    from typing_extensions import NotRequired
else:

    class NotRequired:
        def __class_getitem__(cls, item: object) -> object:
            return item


_MAX_PAGE_EVENT_WAITERS = 64
_MAX_WORKER_EVENT_WAITERS = 64
_MAX_SELECTOR_ENGINE_SOURCE_BYTES = 1024 * 1024
_MAX_COOKIE_FIELD_BYTES = 16 * 1024
_MAX_COOKIE_BATCH_BYTES = 4 * 1024 * 1024
_MAX_COOKIE_URL_BYTES = 64 * 1024
_MAX_COOKIE_URL_BATCH_BYTES = 1024 * 1024
_MAX_STORAGE_STATE_JSON_BYTES = 8 * 1024 * 1024
_MAX_EXTRA_HEADERS = 256
_MAX_EXTRA_HEADER_BYTES = 64 * 1024
_MAX_CDP_HANDSHAKE_HEADERS = 32
_MAX_CDP_HANDSHAKE_HEADER_NAME_BYTES = 256
_MAX_CDP_HANDSHAKE_HEADER_VALUE_BYTES = 4096
_MAX_CDP_HANDSHAKE_HEADER_BYTES = 8192
_MAX_BINDING_DISPATCHES = 64
_MAX_BINDING_REGISTRATIONS = 64
_BINDING_CLOSE_TIMEOUT_SECONDS = 5.0
_CURRENT_BINDING_DISPATCH: contextvars.ContextVar[asyncio.Task[None] | None] = (
    contextvars.ContextVar("pwright_binding_dispatch", default=None)
)
_TARGET_CLOSED_MESSAGE = "Target page, context or browser has been closed"
_REMOTE_CREATION_FENCE_TIMEOUT_SECONDS = 30.0


def _require_context_storage_state() -> None:
    if "context-storage-state" not in _native.selected_features():
        raise UnsupportedError(
            "BrowserContext.storage_state requires the context-storage-state profile"
        )


async def _browser_storage_state_json(
    value: object,
) -> str:
    if isinstance(value, (str, os.PathLike)):
        path = Path(cast("str | os.PathLike[str]", value))
        size = await asyncio.to_thread(lambda: path.stat().st_size)
        if size > _MAX_STORAGE_STATE_JSON_BYTES:
            raise ResourceExhaustedError("Browser.new_context storage_state exceeds 8 MiB")
        source = await asyncio.to_thread(path.read_text, "utf-8")
        if len(source.encode("utf-8")) > _MAX_STORAGE_STATE_JSON_BYTES:
            raise ResourceExhaustedError("Browser.new_context storage_state exceeds 8 MiB")
        value = cast(object, json.loads(source))
    if not isinstance(value, Mapping):
        raise TypeError("Browser.new_context storage_state must be a mapping or path")
    try:
        payload = json.dumps(
            dict(cast(Mapping[str, object], value)),
            ensure_ascii=True,
            allow_nan=False,
            separators=(",", ":"),
        )
    except (TypeError, ValueError) as error:
        raise TypeError("Browser.new_context storage_state must be JSON-serializable") from error
    if len(payload) > _MAX_STORAGE_STATE_JSON_BYTES:
        raise ResourceExhaustedError("Browser.new_context storage_state exceeds 8 MiB")
    return payload


async def _start_binding_awaitable(outcome: Awaitable[object]) -> asyncio.Task[object]:
    """Start a callback through its first suspension before publishing it."""
    started = asyncio.Event()

    async def run() -> object:
        started.set()
        return await outcome

    task = asyncio.create_task(run())
    try:
        await started.wait()
    except BaseException:
        task.cancel()
        with contextlib.suppress(BaseException):
            await task
        raise
    return task


async def _shield_binding_registration(
    browser: Browser,
    operation: Callable[[], Awaitable[None]],
    api_name: str,
) -> None:
    """Finish native registration and callback rollback after caller cancellation."""
    if browser._closed:
        raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
    registrations = browser._binding_registration_tasks
    if len(registrations) >= _MAX_BINDING_REGISTRATIONS:
        raise ResourceExhaustedError("pending binding registration capacity exceeded (64)")

    # Reserve before creating a transaction coroutine or task. A canceled caller
    # must not release this slot while its shielded native commit/rollback runs.
    async def run_registration() -> None:
        await operation()

    registration = asyncio.create_task(run_registration())
    registrations.add(registration)
    registration.add_done_callback(registrations.discard)
    try:
        await asyncio.shield(registration)
    except asyncio.CancelledError:
        loop = asyncio.get_running_loop()

        def report_detached_failure(done: asyncio.Future[None]) -> None:
            if done.cancelled():
                return
            error = done.exception()
            if error is not None:
                loop.call_exception_handler(
                    {
                        "message": f"{api_name} failed after caller cancellation",
                        "exception": error,
                    }
                )

        registration.add_done_callback(report_detached_failure)
        raise


def _wrap_handler(handler: Callable[..., object]) -> Callable[..., object]:
    """Match Playwright Python's public callback arity adaptation."""

    def invoke(*args: object) -> object:
        argument_count = len(inspect.signature(handler).parameters)
        return handler(*args[:argument_count])

    return invoke


class ViewportSize(TypedDict):
    width: int
    height: int


class DeviceDescriptor(TypedDict):
    user_agent: str
    viewport: ViewportSize
    screen: NotRequired[ViewportSize]
    device_scale_factor: float
    is_mobile: bool
    has_touch: bool
    default_browser_type: str


_DEFAULT_VIEWPORT_SIZE = (1280, 720)


class FilePayload(TypedDict):
    name: str
    mimeType: str
    buffer: bytes


class Cookie(TypedDict, total=False):
    name: str
    value: str
    domain: str
    path: str
    expires: float
    httpOnly: bool
    secure: bool
    sameSite: Literal["Lax", "None", "Strict"]
    partitionKey: str | None


class SetCookieParam(TypedDict, total=False):
    name: str
    value: str
    url: str | None
    domain: str | None
    path: str | None
    expires: float | None
    httpOnly: bool | None
    secure: bool | None
    sameSite: Literal["Lax", "None", "Strict"] | None
    partitionKey: str | None


FileInput = Union[
    str,
    Path,
    FilePayload,
    Sequence[Union[str, Path]],
    Sequence[FilePayload],
]


def _file_input_values(
    files: FileInput,
) -> tuple[list[str] | None, list[dict[str, object]] | None]:
    if isinstance(files, (str, Path, Mapping)):
        items: list[object] = [files]
    else:
        if len(files) > 64:
            raise ResourceExhaustedError("file-input count exceeds selected limit")
        items = list(cast(Sequence[object], files))

    path_items = [isinstance(item, (str, Path)) for item in items]
    if any(path_items):
        if not all(path_items):
            raise Error("File paths cannot be mixed with buffers")
        paths: list[str] = []
        directories = 0
        regular_files = 0
        for item in items:
            path = os.fspath(cast(Union[str, Path], item))
            observation = os.stat(path)
            if stat.S_ISDIR(observation.st_mode):
                directories += 1
            else:
                regular_files += 1
            paths.append(path)
        if directories > 1:
            raise Error("Multiple directories are not supported")
        if directories and regular_files:
            raise Error("File paths must be all files or a single directory")
        return paths, None

    payloads: list[dict[str, object]] = []
    for item in items:
        if not isinstance(item, Mapping):
            raise TypeError("file payload must be a mapping")
        mapping = cast(Mapping[str, object], item)
        name = mapping.get("name")
        mime_type = mapping.get("mimeType")
        buffer = mapping.get("buffer")
        if not isinstance(name, str):
            raise TypeError("file payload name must be a string")
        if not isinstance(mime_type, str):
            raise TypeError("file payload MIME must be a string")
        if not isinstance(buffer, bytes):
            raise TypeError("file payload buffer must be bytes")
        payloads.append({"name": name, "mimeType": mime_type, "buffer": buffer})
    return None, payloads


def _read_selector_engine_source(path: Path) -> str:
    with path.open("rb") as source_file:
        encoded = source_file.read(_MAX_SELECTOR_ENGINE_SOURCE_BYTES + 1)
    if len(encoded) > _MAX_SELECTOR_ENGINE_SOURCE_BYTES:
        raise ResourceExhaustedError(
            f"selector engine source exceeds {_MAX_SELECTOR_ENGINE_SOURCE_BYTES} UTF-8 bytes"
        )
    return encoded.decode()


def _timeout_ms(value: object) -> int:
    if value is None:
        return 30_000
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError("timeout must be a number in milliseconds")
    if not math.isfinite(value) or value < 0 or value > 4_294_967_295:
        raise ValueError("timeout must be finite and between 0 and 4294967295 milliseconds")
    return int(value)


def _playwright_server_endpoint(value: object) -> str:
    endpoint = _string_argument(value, "BrowserType.connect ws_endpoint")
    try:
        size = len(endpoint.encode("utf-8"))
    except UnicodeEncodeError as error:
        raise ValueError("BrowserType.connect requires a valid ws:// or wss:// endpoint") from error
    if size > 8_192:
        raise ResourceExhaustedError("BrowserType.connect ws_endpoint exceeds 8192 UTF-8 bytes")
    try:
        parsed = urlsplit(endpoint)
        host = parsed.hostname
        _ = parsed.port
        username = parsed.username
        password = parsed.password
    except ValueError as error:
        raise ValueError("BrowserType.connect requires a valid ws:// or wss:// endpoint") from error
    if (
        parsed.scheme not in {"ws", "wss"}
        or not host
        or username is not None
        or password is not None
        or parsed.fragment
        or any(ord(character) <= 32 or ord(character) == 127 for character in endpoint)
    ):
        raise ValueError("BrowserType.connect requires a valid ws:// or wss:// endpoint")
    return endpoint


def _playwright_server_connect_endpoint(value: object) -> str:
    endpoint = _string_argument(value, "BrowserType.connect ws_endpoint")
    try:
        size = len(endpoint.encode("utf-8"))
        parsed = urlsplit(endpoint)
        host = parsed.hostname
        _ = parsed.port
        username = parsed.username
        password = parsed.password
    except (UnicodeEncodeError, ValueError) as error:
        raise ValueError(
            "BrowserType.connect requires a valid ws(s) or http(s) endpoint"
        ) from error
    if size > 8_192:
        raise ResourceExhaustedError("BrowserType.connect ws_endpoint exceeds 8192 UTF-8 bytes")
    if (
        parsed.scheme not in {"ws", "wss", "http", "https"}
        or not host
        or username is not None
        or password is not None
        or parsed.fragment
        or any(ord(character) <= 32 or ord(character) == 127 for character in endpoint)
    ):
        raise ValueError("BrowserType.connect requires a valid ws(s) or http(s) endpoint")
    return endpoint


def _route_times(value: object, api_name: str) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{api_name}: times must be an integer")
    if value < 0 or value > 18_446_744_073_709_551_615:
        raise ValueError(f"{api_name}: times must be an unsigned 64-bit integer")
    return value


def _expected_count(value: object) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError("to_have_count expected an integer")
    if value < 0 or value > 4_294_967_295:
        raise ValueError("to_have_count expected a value between 0 and 4294967295")
    return value


def _expected_text(value: object, api: str) -> str | Pattern[str] | list[str]:
    if isinstance(value, str):
        return value
    if isinstance(value, re.Pattern):
        return cast(Pattern[str], value)
    if isinstance(value, Sequence):
        expected: list[str] = []
        total_bytes = 0
        for item in cast(Sequence[object], value):
            if isinstance(item, re.Pattern):
                raise UnsupportedError(
                    f"{api}: regular-expression collection expectations are not installed"
                )
            if not isinstance(item, str):
                raise TypeError(f"{api}: expected text items must be strings")
            total_bytes += len(item.encode())
            if len(expected) >= 1024 or total_bytes > 1024 * 1024:
                raise ResourceExhaustedError(
                    f"{api}: expected text collection exceeds 1024 items or 1048576 UTF-8 bytes"
                )
            expected.append(item)
        return expected
    raise TypeError(f"{api}: expected text must be a string or sequence of strings")


def _expected_string_matchers(
    value: object, api: str
) -> tuple[str, str | None] | list[tuple[str, str | None]]:
    if isinstance(value, (str, Pattern)):
        return _text_match(cast(object, value), api)
    if isinstance(value, Sequence):
        expected: list[tuple[str, str | None]] = []
        total_bytes = 0
        for item in cast(Sequence[object], value):
            source, flags = _text_match(item, f"{api} item")
            total_bytes += len(source.encode()) + (0 if flags is None else len(flags.encode()))
            if len(expected) >= 1024 or total_bytes > 1024 * 1024:
                raise ResourceExhaustedError(
                    f"{api}: expected collection exceeds 1024 items or 1048576 UTF-8 bytes"
                )
            expected.append((source, flags))
        return expected
    raise TypeError(f"{api}: expected a string, regular expression, or sequence")


def _int32(value: object, name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{name}: value must be an integer")
    if value < -(2**31) or value > 2**31 - 1:
        raise ValueError(f"{name}: value must fit in int32")
    return value


def _optional_milliseconds(value: object, name: str) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{name} must be a number in milliseconds")
    if not math.isfinite(value) or value < 0 or value > 4_294_967_295:
        raise ValueError(f"{name} must be finite and between 0 and 4294967295 milliseconds")
    return int(value)


def _optional_fractional_milliseconds(value: object, name: str) -> float | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{name} must be a number in milliseconds")
    if not math.isfinite(value) or value < 0 or value > 4_294_967_295:
        raise ValueError(f"{name} must be finite and between 0 and 4294967295 milliseconds")
    return float(value)


def _drag_position(value: object, name: str) -> tuple[float, float] | None:
    if value is None:
        return None
    if not isinstance(value, Mapping):
        raise TypeError(f"{name} must be a mapping with numeric x and y")
    position = cast(Mapping[object, object], value)
    x = position.get("x")
    y = position.get("y")
    if (
        isinstance(x, bool)
        or not isinstance(x, (int, float))
        or isinstance(y, bool)
        or not isinstance(y, (int, float))
    ):
        raise TypeError(f"{name} must contain numeric x and y")
    if not math.isfinite(x) or not math.isfinite(y):
        raise ValueError(f"{name} x and y must be finite")
    return float(x), float(y)


def _drag_steps(value: object) -> float:
    if value is None:
        return 1.0
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError("steps must be an integer")
    return float(value)


def _click_steps(value: object) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError("Locator.click steps must be an integer")
    if value < 1 or value > 10_000:
        raise ValueError("Locator.click steps must be between 1 and 10000")
    return value


def _mouse_number(value: object, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{name} must be a number")
    if not math.isfinite(value):
        raise ValueError(f"{name} must be finite")
    return float(value)


def _tap_options(
    api_name: str,
    *,
    modifiers: Sequence[str] | None,
    position: Mapping[str, float] | None,
    force: bool | None,
    no_wait_after: bool | None,
    trial: bool | None,
) -> dict[str, object]:
    options: dict[str, object] = {}
    if modifiers is not None:
        if (
            not isinstance(cast(object, modifiers), Sequence)
            or isinstance(modifiers, (str, bytes))
            or len(modifiers) > 5
        ):
            raise InvalidArgumentError(
                f"{api_name} modifiers must be a sequence of at most five names"
            )
        if any(
            not isinstance(cast(object, name), str)
            or name not in ("Alt", "Control", "ControlOrMeta", "Meta", "Shift")
            for name in modifiers
        ):
            raise InvalidArgumentError(f"{api_name} modifiers contain an unknown name")
        options["modifiers"] = list(modifiers)
    if position is not None:
        if (
            not isinstance(cast(object, position), Mapping)
            or len(position) != 2
            or "x" not in position
            or "y" not in position
        ):
            raise InvalidArgumentError(f"{api_name} position must contain x and y")
        options["position"] = {
            "x": _mouse_number(position["x"], f"{api_name} position.x"),
            "y": _mouse_number(position["y"], f"{api_name} position.y"),
        }
    for name, value in (
        ("force", force),
        ("no_wait_after", no_wait_after),
        ("trial", trial),
    ):
        if value is not None:
            options[name] = _boolean_argument(value, f"{api_name} {name}")
    return options


def _mouse_button(value: object) -> float:
    if value == "left":
        return 0.0
    if value == "right":
        return 1.0
    if value == "middle":
        return 2.0
    raise Error(f"Unknown button: {value}")


def _mouse_options(
    *,
    button: object = "left",
    steps: object = 1,
    click_count: object = 1,
    delay: object = None,
) -> tuple[float, float, float, float]:
    delay_ms = _optional_milliseconds(delay, "delay")
    return (
        _mouse_button(button),
        _drag_steps(steps),
        float(_int32(click_count, "click_count")),
        0.0 if delay_ms is None else float(delay_ms),
    )


def _polling_ms(value: object) -> int:
    if value is None or value == "raf":
        return 0
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise Error(f"Unknown polling option: {value}")
    if not math.isfinite(value) or value <= 0 or value > 4_294_967_295:
        raise Error(f"Cannot poll with non-positive interval: {value}")
    return int(value)


def _unsupported_option(name: str, value: object) -> None:
    if value is not None:
        raise UnsupportedError(f"connect_over_cdp option {name!r} is not installed")


def _persistent_context_launch_arguments(args: object) -> list[str]:
    if args is None:
        return []
    if isinstance(args, (str, bytes)) or not isinstance(args, Sequence):
        raise Error("BrowserType.launch_persistent_context args must be a sequence of strings")
    values = cast(Sequence[object], args)
    if len(values) > 256:
        raise ResourceExhaustedError("persistent Context launch argument count exceeds 256")
    total_bytes = 0
    checked: list[str] = []
    for value in values:
        argument = _string_argument(value, "BrowserType.launch_persistent_context args")
        if len(argument) > 65_536:
            raise ResourceExhaustedError(
                "persistent Context launch argument exceeds 65536 UTF-8 bytes"
            )
        size = len(argument.encode("utf-8"))
        if size > 65_536:
            raise ResourceExhaustedError(
                "persistent Context launch argument exceeds 65536 UTF-8 bytes"
            )
        total_bytes += size
        if total_bytes > 1_048_576:
            raise ResourceExhaustedError(
                "persistent Context launch argument batch exceeds 1048576 UTF-8 bytes"
            )
        checked.append(argument)
    if any(argument.startswith("--user-data-dir") for argument in checked):
        raise Error("Pass user_data_dir parameter instead of --user-data-dir argument")
    if any(argument.startswith("--remote-debugging-pipe") for argument in checked):
        raise Error("Pwright manages remote debugging connection itself.")
    if any(not argument.startswith("-") for argument in checked):
        raise Error("Arguments can not specify page to be opened")
    return checked


def _geolocation_options(value: object, name: str) -> tuple[float, float, float] | None:
    if value is None:
        return None
    if not isinstance(value, Mapping):
        raise TypeError(f"{name} must be a mapping")
    geolocation = cast(Mapping[object, object], value)
    longitude = geolocation.get("longitude")
    latitude = geolocation.get("latitude")
    accuracy = geolocation.get("accuracy", 0)

    def number(field: str, coordinate: object) -> float:
        if isinstance(coordinate, bool) or not isinstance(coordinate, (int, float)):
            raise TypeError(f"{name} {field} must be numeric")
        return float(coordinate)

    return (
        number("longitude", longitude),
        number("latitude", latitude),
        number("accuracy", accuracy),
    )


def _geolocation_permission(value: object, name: str) -> bool:
    if value is None:
        return False
    if isinstance(value, (str, bytes)) or not isinstance(value, Sequence):
        raise TypeError(f"{name} must be a sequence of strings")
    values = cast(Sequence[object], value)
    permissions = [_string_argument(permission, name) for permission in values]
    unsupported = sorted({permission for permission in permissions if permission != "geolocation"})
    if unsupported:
        joined = ", ".join(unsupported)
        raise UnsupportedError(f"{name} permissions are not installed: {joined}")
    return "geolocation" in permissions


def _http_credentials_options(value: object, name: str) -> tuple[str, str, str | None, bool] | None:
    if value is None:
        return None
    if not isinstance(value, Mapping):
        raise TypeError(f"{name} must be a mapping")
    credentials = cast(Mapping[object, object], value)
    username = credentials.get("username")
    password = credentials.get("password")
    origin = credentials.get("origin")
    send = credentials.get("send", "unauthorized")
    if not isinstance(username, str) or not isinstance(password, str):
        raise TypeError(f"{name} username and password must be strings")
    if origin is not None and not isinstance(origin, str):
        raise TypeError(f"{name} origin must be a string")
    if send not in ("always", "unauthorized"):
        raise TypeError(f'{name} send must be "always" or "unauthorized"')
    return username, password, origin, send == "always"


def _string_argument(value: object, api: str) -> str:
    if isinstance(value, str):
        return value
    if value is None:
        actual = "null"
    elif isinstance(value, bool):
        actual = "boolean"
    elif isinstance(value, (int, float)):
        actual = "number"
    else:
        actual = type(value).__name__
    raise Error(f"{api}: expected string, got {actual}")


def _text_match(value: object, api: str) -> tuple[str, str | None]:
    if not isinstance(value, Pattern):
        return _string_argument(value, api), None
    pattern = cast(Pattern[str], value)
    pattern_source: object = cast(Any, pattern).pattern
    if not isinstance(pattern_source, str):
        raise Error(f"{api}: expected a string regular expression")
    supported = int(re.IGNORECASE | re.MULTILINE | re.DOTALL | re.UNICODE)
    if pattern.flags & ~supported:
        raise Error(
            f"{api}: only IGNORECASE, MULTILINE, DOTALL and UNICODE regex flags are supported"
        )
    # Rust validates and retains canonical ECMAScript flags. Python's implicit
    # UNICODE flag has no separate JavaScript equivalent.
    flags = ""
    if pattern.flags & int(re.IGNORECASE):
        flags += "i"
    if pattern.flags & int(re.MULTILINE):
        flags += "m"
    if pattern.flags & int(re.DOTALL):
        flags += "s"
    return pattern_source, flags


def _viewport(value: object, name: str) -> tuple[int, int] | None:
    if value is None:
        return None
    if not isinstance(value, Mapping):
        raise TypeError(f"{name} must be a mapping with integer width and height")
    mapping = cast(Mapping[object, object], value)
    width = mapping.get("width")
    height = mapping.get("height")
    if (
        isinstance(width, bool)
        or not isinstance(width, int)
        or isinstance(height, bool)
        or not isinstance(height, int)
    ):
        raise TypeError(f"{name} must contain integer width and height")
    if width <= 0 or height <= 0 or width > 10_000_000 or height > 10_000_000:
        raise ValueError(f"{name} width and height must be between 1 and 10000000")
    return width, height


def _device_options(
    *,
    user_agent: object = None,
    viewport: object = None,
    screen: object = None,
    device_scale_factor: object = None,
    is_mobile: object = None,
    has_touch: object = None,
) -> (
    tuple[
        str | None,
        tuple[int, int] | None,
        tuple[int, int] | None,
        float | None,
        bool | None,
        bool | None,
    ]
    | None
):
    values = (user_agent, viewport, screen, device_scale_factor, is_mobile, has_touch)
    if all(value is None for value in values):
        return None
    if user_agent is not None and not isinstance(user_agent, str):
        raise TypeError("user_agent must be a string")
    if device_scale_factor is not None:
        if isinstance(device_scale_factor, bool) or not isinstance(
            device_scale_factor, (int, float)
        ):
            raise TypeError("device_scale_factor must be a number")
        if not math.isfinite(device_scale_factor) or device_scale_factor < 0:
            raise ValueError("device_scale_factor must be finite and non-negative")
    if is_mobile is not None and not isinstance(is_mobile, bool):
        raise TypeError("is_mobile must be a boolean")
    if has_touch is not None and not isinstance(has_touch, bool):
        raise TypeError("has_touch must be a boolean")
    return (
        user_agent,
        _viewport(viewport, "viewport"),
        _viewport(screen, "screen"),
        None if device_scale_factor is None else float(device_scale_factor),
        is_mobile,
        has_touch,
    )


def _persistent_context_media_options(
    *,
    color_scheme: object = None,
    reduced_motion: object = None,
    contrast: object = None,
) -> tuple[str | None, str | None, str | None] | None:
    def checked(value: object, name: str, allowed: set[str]) -> str | None:
        if value is None:
            return None
        text = _string_argument(value, f"BrowserType.launch_persistent_context {name}")
        if text not in allowed:
            choices = ", ".join(sorted(allowed))
            raise Error(f"{name}: expected one of {choices}, got {text!r}")
        return text

    values = (
        checked(color_scheme, "color_scheme", {"dark", "light", "no-preference"}),
        checked(reduced_motion, "reduced_motion", {"no-preference", "reduce"}),
        checked(contrast, "contrast", {"more", "no-preference"}),
    )
    return None if all(value is None for value in values) else values


def _persistent_context_extra_http_headers(
    value: object,
) -> list[tuple[str, str]] | None:
    if value is None:
        return None
    if not isinstance(value, Mapping):
        raise TypeError("extra_http_headers must be a mapping of strings")
    mapping = cast(Mapping[object, object], value)
    if len(mapping) > _MAX_EXTRA_HEADERS:
        raise ResourceExhaustedError("persistent Context extra header count exceeds 256")
    total_bytes = 0
    headers: list[tuple[str, str]] = []
    for raw_name, raw_value in mapping.items():
        name = _string_argument(
            raw_name, "BrowserType.launch_persistent_context extra_http_headers name"
        )
        header_value = _string_argument(
            raw_value, "BrowserType.launch_persistent_context extra_http_headers value"
        )
        total_bytes += len(name.encode("utf-8")) + len(header_value.encode("utf-8"))
        if total_bytes > _MAX_EXTRA_HEADER_BYTES:
            raise ResourceExhaustedError("persistent Context extra header bytes exceed 65536")
        headers.append((name, header_value))
    return headers


def _extra_http_headers(
    value: object, api_name: str, *, indexed_values: bool = False
) -> list[tuple[str, str]]:
    if not isinstance(value, Mapping):
        raise TypeError(f"{api_name}: headers must be a mapping of strings")
    mapping = cast(Mapping[object, object], value)
    if len(mapping) > _MAX_EXTRA_HEADERS:
        raise ResourceExhaustedError(f"{api_name}: extra header count exceeds 256")
    total_bytes = 0
    headers: list[tuple[str, str]] = []
    for index, (raw_name, raw_value) in enumerate(mapping.items()):
        name = _string_argument(raw_name, f"{api_name} header name")
        value_location = (
            f"{api_name}: headers[{index}].value" if indexed_values else f"{api_name} header value"
        )
        header_value = _string_argument(raw_value, value_location)
        total_bytes += len(name.encode("utf-8")) + len(header_value.encode("utf-8"))
        if total_bytes > _MAX_EXTRA_HEADER_BYTES:
            raise ResourceExhaustedError(f"{api_name}: extra header bytes exceed 65536")
        headers.append((name, header_value))
    return headers


def _cdp_handshake_headers(value: object) -> list[tuple[str, str]]:
    api_name = "BrowserType.connect_over_cdp"
    if not isinstance(value, Mapping):
        raise TypeError(f"{api_name}: headers must be a mapping of strings")
    mapping = cast(Mapping[object, object], value)
    if len(mapping) > _MAX_CDP_HANDSHAKE_HEADERS:
        raise ResourceExhaustedError(f"{api_name}: handshake header count exceeds 32")
    total_bytes = 0
    headers: list[tuple[str, str]] = []
    for raw_name, raw_value in mapping.items():
        name = _string_argument(raw_name, f"{api_name} header name")
        header_value = _string_argument(raw_value, f"{api_name} header value")
        name_bytes = len(name.encode("utf-8"))
        value_bytes = len(header_value.encode("utf-8"))
        if name_bytes > _MAX_CDP_HANDSHAKE_HEADER_NAME_BYTES:
            raise ResourceExhaustedError(f"{api_name}: handshake header name exceeds 256 bytes")
        if value_bytes > _MAX_CDP_HANDSHAKE_HEADER_VALUE_BYTES:
            raise ResourceExhaustedError(f"{api_name}: handshake header value exceeds 4096 bytes")
        total_bytes += name_bytes + value_bytes
        if total_bytes > _MAX_CDP_HANDSHAKE_HEADER_BYTES:
            raise ResourceExhaustedError(f"{api_name}: handshake header bytes exceed 8192")
        headers.append((name, header_value))
    return headers


def _context_creation_extra_http_headers(
    value: object,
) -> list[tuple[str, str]] | None:
    if value is None:
        return None
    if not isinstance(value, Mapping):
        raise TypeError("Browser.new_context extra_http_headers must be a mapping of strings")
    mapping = cast(Mapping[object, object], value)
    total_bytes = 0
    headers: list[tuple[str, str]] = []
    for raw_name, raw_value in mapping.items():
        # Playwright omits null-valued entries when serializing creation headers.
        if raw_value is None:
            continue
        if len(headers) == _MAX_EXTRA_HEADERS:
            raise ResourceExhaustedError("Browser.new_context extra header count exceeds 256")
        name = _string_argument(raw_name, "Browser.new_context extra_http_headers name")
        header_value = _string_argument(raw_value, "Browser.new_context extra_http_headers value")
        total_bytes += len(name.encode("utf-8")) + len(header_value.encode("utf-8"))
        if total_bytes > _MAX_EXTRA_HEADER_BYTES:
            raise ResourceExhaustedError("Browser.new_context extra header bytes exceed 65536")
        headers.append((name, header_value))
    return headers


def _owned_context_viewport(
    device: (
        tuple[
            str | None,
            tuple[int, int] | None,
            tuple[int, int] | None,
            float | None,
            bool | None,
            bool | None,
        ]
        | None
    ),
) -> tuple[int, int]:
    if device is None or device[1] is None:
        return _DEFAULT_VIEWPORT_SIZE
    return device[1]


def _device_descriptors() -> dict[str, DeviceDescriptor]:
    loaded: object = json.loads(_native.device_descriptors_json())
    if not isinstance(loaded, dict):
        raise RuntimeError("native device descriptor catalog is not an object")
    raw = cast(dict[object, object], loaded)
    result: dict[str, DeviceDescriptor] = {}
    for raw_name, raw_descriptor in raw.items():
        if not isinstance(raw_name, str) or not isinstance(raw_descriptor, dict):
            raise RuntimeError("native device descriptor catalog contains an invalid entry")
        name = raw_name
        values = cast(dict[str, object], raw_descriptor)
        user_agent = values.get("userAgent")
        viewport = _viewport(values.get("viewport"), "catalog viewport")
        scale = values.get("deviceScaleFactor")
        mobile = values.get("isMobile")
        touch = values.get("hasTouch")
        browser_type = values.get("defaultBrowserType")
        if (
            not isinstance(user_agent, str)
            or viewport is None
            or isinstance(scale, bool)
            or not isinstance(scale, (int, float))
            or not isinstance(mobile, bool)
            or not isinstance(touch, bool)
            or not isinstance(browser_type, str)
        ):
            raise RuntimeError(f"native device descriptor {name!r} is invalid")
        descriptor: DeviceDescriptor = {
            "user_agent": user_agent,
            "viewport": {"width": viewport[0], "height": viewport[1]},
            "device_scale_factor": float(scale),
            "is_mobile": mobile,
            "has_touch": touch,
            "default_browser_type": browser_type,
        }
        screen = _viewport(values.get("screen"), "catalog screen")
        if screen is not None:
            descriptor["screen"] = {"width": screen[0], "height": screen[1]}
        result[name] = descriptor
    return result


async def _init_script_source(
    script: object,
    path: str | Path | None,
    *,
    source_url: bool,
) -> str:
    if path:
        source = (await asyncio.to_thread(Path(path).read_bytes)).decode()
        if source_url:
            return source + "\n//# sourceURL=" + str(path).replace("\n", "")
        return source
    if not isinstance(script, str):
        raise Error("Either path or script parameter must be specified")
    return script


async def _tag_source(
    *,
    url: str | None,
    path: str | Path | None,
    content: str | None,
    source_url_style: str,
) -> tuple[str | None, str | None]:
    if path is not None:
        content = (await asyncio.to_thread(Path(path).read_bytes)).decode()
        clean_path = str(path).replace("\n", "")
        if source_url_style == "script":
            content += f"\n//# sourceURL={clean_path}"
        else:
            content += f"\n/*# sourceURL={clean_path} */"
    if url is None and content is None:
        raise Error("Either path, url, or content parameter must be specified")
    return url, content


_ADD_SCRIPT_TAG_SOURCE = """async ({ url, content, type }) => {
    const script = document.createElement('script');
    if (url !== null) script.src = url;
    if (type !== null) script.type = type;
    if (content !== null) script.text = content;
    if (url !== null) {
        const completion = new Promise((resolve, reject) => {
            script.onload = resolve;
            script.onerror = () => reject(new Error(`Failed to load script at ${script.src}`));
        });
        document.head.appendChild(script);
        await completion;
        return script;
    }
    let cspError = null;
    const onViolation = event => {
        if (event.blockedURI === 'inline' || event.blockedURI === '')
            cspError = new Error(
                `Refused to execute inline script because it violates ` +
                `${event.violatedDirective || 'Content Security Policy'}`
            );
    };
    document.addEventListener('securitypolicyviolation', onViolation);
    script.onerror = event => {
        cspError ??= new Error(`Failed to add inline script: ${event.type}`);
    };
    document.head.appendChild(script);
    await new Promise(resolve => setTimeout(resolve, 0));
    document.removeEventListener('securitypolicyviolation', onViolation);
    if (cspError) throw cspError;
    return script;
}"""


_ADD_STYLE_TAG_SOURCE = """async ({ url, content }) => {
    const element = document.createElement(url === null ? 'style' : 'link');
    if (url !== null) {
        element.rel = 'stylesheet';
        element.href = url;
        const completion = new Promise((resolve, reject) => {
            element.onload = resolve;
            element.onerror = () => reject(new Error(`Failed to load style at ${element.href}`));
        });
        document.head.appendChild(element);
        await completion;
        return element;
    }
    element.textContent = content;
    let cspError = null;
    const onViolation = event => {
        if (event.blockedURI === 'inline' || event.blockedURI === '')
            cspError = new Error(
                `Refused to apply inline style because it violates ` +
                `${event.violatedDirective || 'Content Security Policy'}`
            );
    };
    document.addEventListener('securitypolicyviolation', onViolation);
    document.head.appendChild(element);
    await new Promise(resolve => setTimeout(resolve, 0));
    document.removeEventListener('securitypolicyviolation', onViolation);
    if (cspError) throw cspError;
    return element;
}"""


def _binding_value(value: object, references: dict[int, object]) -> object:
    if isinstance(value, (bool, int, float, str)) or value is None:
        return value
    if not isinstance(value, dict):
        raise Error("invalid serialized binding value")
    mapping = cast(dict[str, object], value)
    if "ref" in mapping:
        reference = mapping["ref"]
        if not isinstance(reference, int) or reference not in references:
            raise Error("unknown serialized binding reference")
        return references[reference]
    if "v" in mapping:
        primitive = mapping["v"]
        if primitive in {"undefined", "null"}:
            return None
        if primitive == "NaN":
            return math.nan
        if primitive == "Infinity":
            return math.inf
        if primitive == "-Infinity":
            return -math.inf
        if primitive == "-0":
            return -0.0
        raise Error("unknown serialized binding primitive")
    if "bi" in mapping:
        return int(cast(str, mapping["bi"]))
    if "d" in mapping:
        return mapping["d"]
    if "u" in mapping:
        return mapping["u"]
    if "r" in mapping:
        regexp = mapping["r"]
        if not isinstance(regexp, dict):
            raise Error("invalid serialized binding regexp")
        regexp_mapping = cast(dict[str, object], regexp)
        pattern = regexp_mapping.get("p")
        if not isinstance(pattern, str):
            raise Error("invalid serialized binding regexp")
        return pattern
    if "e" in mapping:
        payload = mapping["e"]
        if not isinstance(payload, dict):
            raise Error("invalid serialized binding error")
        payload_mapping = cast(dict[str, object], payload)
        return Error(str(payload_mapping.get("m", "")))
    if "a" in mapping:
        reference = mapping.get("id")
        items = mapping["a"]
        if not isinstance(reference, int) or not isinstance(items, list):
            raise Error("invalid serialized binding array")
        array_result: list[object] = []
        if reference in references:
            raise Error("duplicate serialized binding reference")
        references[reference] = array_result
        array_result.extend(_binding_value(item, references) for item in cast(list[object], items))
        return array_result
    if "o" in mapping:
        reference = mapping.get("id")
        properties = mapping["o"]
        if not isinstance(reference, int) or not isinstance(properties, list):
            raise Error("invalid serialized binding object")
        object_result: dict[str, object] = {}
        if reference in references:
            raise Error("duplicate serialized binding reference")
        references[reference] = object_result
        for property_value in cast(list[object], properties):
            if not isinstance(property_value, dict):
                raise Error("invalid serialized binding property")
            property_mapping = cast(dict[str, object], property_value)
            key = property_mapping.get("k")
            if not isinstance(key, str) or "v" not in property_mapping:
                raise Error("invalid serialized binding property")
            if key != "__proto__":
                object_result[key] = _binding_value(property_mapping["v"], references)
        return object_result
    raise Error("unknown serialized binding value")


def _binding_arguments(arguments_json: str) -> list[object]:
    values = cast(object, json.loads(arguments_json))
    if not isinstance(values, list):
        raise Error(
            "serializedArgs is not an array. This can happen when "
            "Array.prototype.toJSON is defined incorrectly"
        )
    return [_binding_value(value, {}) for value in cast(list[object], values)]


def _json_body(value: object) -> bytes:
    return json.dumps(value).encode()


class RequestSizes(TypedDict):
    requestBodySize: int
    requestHeadersSize: int
    responseBodySize: int
    responseHeadersSize: int
    transferSize: int


class SecurityDetails(TypedDict):
    issuer: str
    protocol: str
    subjectName: str
    validFrom: float
    validTo: float


class ResourceTiming(TypedDict):
    startTime: float
    domainLookupStart: float
    domainLookupEnd: float
    connectStart: float
    secureConnectionStart: float
    connectEnd: float
    requestStart: float
    responseStart: float
    responseEnd: float


def _resource_timing(value: Mapping[str, Any]) -> ResourceTiming:
    return ResourceTiming(
        startTime=float(value["startTime"]),
        domainLookupStart=float(value["domainLookupStart"]),
        domainLookupEnd=float(value["domainLookupEnd"]),
        connectStart=float(value["connectStart"]),
        secureConnectionStart=float(value["secureConnectionStart"]),
        connectEnd=float(value["connectEnd"]),
        requestStart=float(value["requestStart"]),
        responseStart=float(value["responseStart"]),
        responseEnd=float(value["responseEnd"]),
    )


def _merge_resource_timing(
    current: Mapping[str, Any], incoming: Mapping[str, Any]
) -> ResourceTiming:
    previous = _resource_timing(current)
    replacement = _resource_timing(incoming)
    for field in (
        "domainLookupStart",
        "domainLookupEnd",
        "connectStart",
        "secureConnectionStart",
        "connectEnd",
        "requestStart",
        "responseStart",
        "responseEnd",
    ):
        if replacement[field] == -1 and previous[field] != -1:
            replacement[field] = previous[field]
    if replacement["startTime"] == 0 and previous["startTime"] != 0:
        replacement["startTime"] = previous["startTime"]
    return replacement


class Request:
    def __init__(
        self,
        value: Mapping[str, Any],
        native: _native.NativeSession | None = None,
        context: BrowserContext | None = None,
    ) -> None:
        self._value = dict(value)
        self._native = native
        self._context = context
        page_id = self._value.get("page_id")
        frame_available = bool(self._value.get("frame_available", False))
        if (
            context is not None
            and page_id is not None
            and frame_available
            and str(page_id) not in context._pages
        ):
            context.pages
        self._owner_page = (
            None if context is None or page_id is None else context._pages.get(str(page_id))
        )
        self._page = self._owner_page if frame_available else None
        self._sizes_owner = cast(
            "_native.NativeRequestSizeOwner | None", self._value.get("sizes_owner")
        )
        self._history_owner = cast(
            "_native.NativeRequestHistoryOwner | None", self._value.get("history_owner")
        )
        self._redirected_from: Request | None = None
        self._redirected_to: Request | None = None
        self._route_snapshot: dict[str, Any] | None = None
        timing = self._value.get("timing")
        if isinstance(timing, Mapping):
            self._merge_timing(cast(Mapping[str, Any], timing))

    def _merge_timing(self, value: Mapping[str, Any]) -> None:
        current = self._value.get("timing")
        self._value["timing"] = (
            _merge_resource_timing(cast(Mapping[str, Any], current), value)
            if isinstance(current, Mapping)
            else _resource_timing(value)
        )

    def _merge(
        self,
        value: Mapping[str, Any],
        native: _native.NativeSession | None,
        context: BrowserContext | None,
    ) -> None:
        current_timing = self._value.get("timing")
        self._value.update(value)
        timing = value.get("timing")
        if isinstance(timing, Mapping):
            self._value["timing"] = (
                _merge_resource_timing(
                    cast(Mapping[str, Any], current_timing),
                    cast(Mapping[str, Any], timing),
                )
                if isinstance(current_timing, Mapping)
                else _resource_timing(cast(Mapping[str, Any], timing))
            )
        if native is not None:
            self._native = native
        if context is not None:
            self._context = context
        if "page_id" in value or "frame_available" in value:
            page_id = self._value.get("page_id")
            frame_available = bool(self._value.get("frame_available", False))
            if (
                self._context is not None
                and page_id is not None
                and frame_available
                and str(page_id) not in self._context._pages
            ):
                self._context.pages
            self._owner_page = (
                None
                if self._context is None or page_id is None
                else self._context._pages.get(str(page_id))
            )
            self._page = self._owner_page if frame_available else None
        sizes_owner = cast("_native.NativeRequestSizeOwner | None", value.get("sizes_owner"))
        if sizes_owner is not None:
            self._sizes_owner = sizes_owner
        history_owner = cast("_native.NativeRequestHistoryOwner | None", value.get("history_owner"))
        if history_owner is not None:
            self._history_owner = history_owner
        if self._route_snapshot is not None:
            self._value.update(self._route_snapshot)

    def _merge_route_snapshot(
        self,
        value: Mapping[str, Any],
        native: _native.NativeSession,
        context: BrowserContext | None,
    ) -> None:
        self._merge(value, native, context)
        self._route_snapshot = {
            "url": str(value["url"]),
            "method": str(value["method"]),
            "headers": dict(value["headers"]),
            "post_data": value.get("post_data"),
            "post_data_buffer": value.get("post_data_buffer"),
        }
        self._value.update(self._route_snapshot)

    @property
    def url(self) -> str:
        return str(self._value["url"])

    @property
    def method(self) -> str:
        return str(self._value["method"])

    @property
    def resource_type(self) -> str:
        return str(self._value["resource_type"])

    @property
    def headers(self) -> dict[str, str]:
        return {str(name).lower(): str(value) for name, value in self._value["headers"].items()}

    async def all_headers(self) -> dict[str, str]:
        return self.headers

    async def header_value(self, name: str) -> str | None:
        normalized_name = str(name).lower()
        return next(
            (
                value
                for header_name, value in self.headers.items()
                if header_name.lower() == normalized_name
            ),
            None,
        )

    @property
    def post_data(self) -> str | None:
        value = self._value.get("post_data")
        return None if value is None else str(value)

    @property
    def post_data_buffer(self) -> bytes | None:
        value = self._value.get("post_data_buffer")
        return None if value is None else bytes(value)

    @property
    def post_data_json(self) -> object | None:
        if "post_data_json" not in self._value:
            raise UnsupportedError("Request.post_data_json capability is not available")
        error = self._value.get("post_data_json_error")
        if error is not None:
            raise Error(str(error))
        return self._value["post_data_json"]

    @property
    def frame(self) -> Frame:
        if "page_id" not in self._value:
            raise UnsupportedError("Request.frame capability is not available")
        if not bool(self._value.get("frame_available", False)):
            if self.is_navigation_request():
                raise Error(
                    "Frame for this navigation request is not available, because the request\n"
                    "was issued before the frame is created. You can check whether the request\n"
                    "is a navigation request by calling isNavigationRequest() method."
                )
            raise Error("Service Worker requests do not have an associated frame.")
        if self._page is None:
            raise Error("Request frame Page is no longer available.")
        frame_id = self._value.get("frame_id")
        if frame_id is None:
            return self._page.main_frame
        frame = self._page._frames.get(str(frame_id))
        if frame is None:
            self._page._refresh_frames()
            frame = self._page._frames.get(str(frame_id))
        if frame is None:
            raise Error("Request frame is no longer available.")
        return frame

    def is_navigation_request(self) -> bool:
        return bool(self._value["is_navigation_request"])

    @property
    def failure(self) -> str | None:
        value = self._value.get("failure_text")
        return None if value is None else str(value)

    @property
    def timing(self) -> ResourceTiming:
        value = self._value.get("timing")
        if not isinstance(value, Mapping):
            raise UnsupportedError("Request.timing capability is not available")
        return _resource_timing(cast(Mapping[str, Any], value))

    @property
    def redirected_from(self) -> Request | None:
        if self._owner_page is not None:
            self._owner_page._sync_request_history()
        return self._redirected_from

    @property
    def redirected_to(self) -> Request | None:
        if self._owner_page is not None:
            self._owner_page._sync_request_history()
        return self._redirected_to

    async def sizes(self) -> RequestSizes:
        if self._native is None or self._sizes_owner is None:
            raise UnsupportedError("Request.sizes capability is not available for this Request")
        value = await self._native.request_sizes(self._sizes_owner)
        return RequestSizes(
            requestBodySize=int(value["requestBodySize"]),
            requestHeadersSize=int(value["requestHeadersSize"]),
            responseBodySize=int(value["responseBodySize"]),
            responseHeadersSize=int(value["responseHeadersSize"]),
            transferSize=int(value["transferSize"]),
        )

    async def response(self) -> Response | None:
        if self._native is None:
            raise UnsupportedError("Request.response capability is not available for this Request")
        if self._history_owner is not None:
            value = await self._native.request_history_response(self._history_owner)
        else:
            page_id = self._value.get("page_id")
            native_request_id = self._value.get("native_request_id")
            if page_id is None or native_request_id is None:
                raise UnsupportedError(
                    "Request.response capability is not available for this Request"
                )
            value = await self._native.request_response_by_native_id(
                str(page_id), str(native_request_id)
            )
        if value is None:
            return None
        if self._owner_page is not None:
            return self._owner_page._wrap_response(value, request=self)
        return Response(value, self._native, self._context, request=self)


class Route:
    def __init__(self, native: _native.NativeSession, call_id: str, request: Request) -> None:
        self._native = native
        self._call_id = call_id
        self._request = request
        self._handled = False
        self._retire_registration: Callable[[], Awaitable[None]] | None = None
        self._dispatch_task: asyncio.Task[object] | None = None
        self._scheduled_handling_task: asyncio.Task[object] | None = None

    def _track_handling_task(self) -> None:
        task = asyncio.current_task()
        if task is not None and task is not self._dispatch_task:
            self._scheduled_handling_task = cast(asyncio.Task[object], task)

    @property
    def request(self) -> Request:
        return self._request

    async def fallback(
        self,
        *,
        url: str | None = None,
        method: str | None = None,
        headers: Mapping[str, str | None] | None = None,
        post_data: Any = None,
    ) -> None:
        if self._handled:
            raise Error("Route is already handled")
        self._track_handling_task()
        header_items = (
            None
            if headers is None
            else [(str(name), str(value)) for name, value in headers.items() if value is not None]
        )
        if isinstance(post_data, str):
            post_data_buffer = post_data.encode()
        elif isinstance(post_data, bytes) or post_data is None:
            post_data_buffer = post_data
        else:
            post_data_buffer = json.dumps(post_data).encode()
        await self._native.page_route_fallback(
            self._call_id,
            url,
            method,
            header_items,
            post_data_buffer,
        )
        self._handled = True

    async def continue_(
        self,
        *,
        url: str | None = None,
        method: str | None = None,
        headers: Mapping[str, str | None] | None = None,
        post_data: Any = None,
    ) -> None:
        if self._handled:
            raise Error("Route is already handled")
        self._track_handling_task()
        header_items = (
            None
            if headers is None
            else [(str(name), str(value)) for name, value in headers.items() if value is not None]
        )
        if isinstance(post_data, str):
            post_data_buffer = post_data.encode()
        elif isinstance(post_data, bytes) or post_data is None:
            post_data_buffer = post_data
        else:
            post_data_buffer = json.dumps(post_data).encode()
        await self._native.page_route_continue(
            self._call_id,
            url,
            method,
            header_items,
            post_data_buffer,
        )
        self._handled = True

    async def abort(self, error_code: str | None = None) -> None:
        if self._handled:
            raise Error("Route is already handled")
        self._track_handling_task()
        await self._native.page_route_abort(self._call_id, error_code)
        self._handled = True

    async def fetch(
        self,
        *,
        url: str | None = None,
        method: str | None = None,
        headers: Mapping[str, str] | None = None,
        post_data: object = None,
        timeout: float | None = None,
        max_redirects: int | None = None,
        max_retries: int | None = None,
    ) -> APIResponse:
        context = self._request._context
        if context is None:
            raise UnsupportedError("Route.fetch requires its BrowserContext request owner")
        options: dict[str, object] = {
            "headers": self._request.headers if headers is None else headers,
            "timeout": timeout,
            "max_redirects": max_redirects,
            "max_retries": max_retries,
        }
        request_body = self._request.post_data_buffer if post_data is None else post_data
        if request_body is not None:
            options["data"] = request_body
        return await context.request._send_kwargs(
            "Route.fetch",
            self._request.method if method is None else method,
            self._request.url if url is None else url,
            options,
        )

    async def fulfill(
        self,
        *,
        status: int | None = None,
        headers: Mapping[str, str] | None = None,
        body: str | bytes | None = None,
        json: object = None,
        path: str | Path | None = None,
        content_type: str | None = None,
        response: APIResponse | None = None,
    ) -> None:
        if self._handled:
            raise Error("Route is already handled")
        self._track_handling_task()
        if json is not None:
            if body is not None or path is not None:
                raise Error("Route.fulfill json, body, and path are mutually exclusive")
            body = _json_body(json)
            if content_type is None:
                content_type = "application/json"
        if response is not None and not isinstance(cast(Any, response), APIResponse):
            raise TypeError("Route.fulfill response must be an APIResponse")
        response_headers = [] if headers is None else [(str(k), str(v)) for k, v in headers.items()]
        if response is not None and body is None and path is None:
            if "route-fulfill-api-response" not in _native.selected_features():
                raise UnsupportedError(
                    "Route.fulfill response requires the route-fulfill-api-response profile"
                )
            await self._native.page_route_fulfill_response(
                self._call_id,
                response._native,
                status,
                None if not headers else response_headers,
                content_type,
            )
            self._handled = True
            return
        if response is not None:
            if status is None:
                status = response.status
            if not headers:
                response_headers = list(response.headers.items())
        if content_type is not None:
            response_headers = [
                (name, value) for name, value in response_headers if name.lower() != "content-type"
            ]
            response_headers.append(("content-type", content_type))
        if path is not None:
            if body is not None:
                raise Error("Route.fulfill body and path are mutually exclusive")
            await self._native.page_route_fulfill_path(
                self._call_id,
                200 if status is None else status,
                response_headers,
                os.fspath(path),
            )
            self._handled = True
            return
        if body is None:
            response_body = b""
        elif isinstance(body, str):
            response_body = body.encode()
        else:
            response_body = body
        await self._native.page_route_fulfill(
            self._call_id,
            200 if status is None else status,
            response_headers,
            response_body,
        )
        self._handled = True

    async def _finish_registration_retirement(self) -> None:
        retire = self._retire_registration
        self._retire_registration = None
        if retire is not None:
            await retire()

    async def _finish_scheduled_handling(self) -> None:
        # A synchronous async-API callback can deliberately schedule a route
        # command with create_task(). Give that task one turn to identify itself,
        # then wait only for the command that actually started. An unhandled
        # route must not stall the global route watcher indefinitely.
        await asyncio.sleep(0)
        task = self._scheduled_handling_task
        if task is not None and task is not asyncio.current_task():
            await task


RouteURLMatcher = Union[str, Pattern[str], Callable[[str], bool]]


class _BindingRouteHandler:
    """Evaluate binding-owned URL matchers while Rust owns route traversal."""

    def __init__(
        self,
        matcher: Pattern[str] | Callable[[str], bool],
        handler: Callable[..., object],
        times: int | None,
        discard: Callable[[str], None],
        retire: Callable[[str], Awaitable[None]],
    ) -> None:
        self._matcher = matcher
        self._handler = handler
        self._remaining = times if times is not None and times > 0 else None
        self._discard = discard
        self._retire = retire
        self._registration: str | None = None

    def bind_registration(self, registration: str) -> None:
        self._registration = registration

    def _matches(self, url: str) -> bool:
        if isinstance(self._matcher, Pattern):
            return self._matcher.search(url) is not None
        return bool(self._matcher(url))

    async def __call__(self, route: Route, request: Request) -> None:
        if not self._matches(request.url):
            await route.fallback()
            return

        if self._remaining is not None:
            self._remaining -= 1
            if self._remaining == 0:
                registration = self._registration
                if registration is None:
                    raise Error("Route registration is not available")
                self._discard(registration)
                route._retire_registration = lambda: self._retire(registration)

        try:
            argument_count = len(inspect.signature(self._handler).parameters)
        except (TypeError, ValueError):
            argument_count = 2
        outcome = self._handler(*[route, request][:argument_count])
        if inspect.isawaitable(outcome):
            await cast(Awaitable[object], outcome)


class Response:
    def __init__(
        self,
        value: Mapping[str, Any],
        native: _native.NativeSession | None = None,
        context: BrowserContext | None = None,
        *,
        request: Request | None = None,
        navigation_body: tuple[str, str, str] | None = None,
        frame_override: Frame | None = None,
    ) -> None:
        self._value = dict(value)
        self._native = native
        self._body_owner = cast("_native.NativeResponseBody | None", self._value.get("body_owner"))
        self._finished_owner: _native.NativeResponseFinishedOwner | None = self._value.get(
            "finished_owner"
        )
        self._navigation_body = navigation_body
        self._frame_override = frame_override
        raw_request = self._value.get("request")
        self._request = (
            request
            if request is not None
            else Request(cast(Mapping[str, Any], raw_request), native, context)
            if isinstance(raw_request, Mapping)
            else None
        )
        self._merge_request_timing()

    def _merge_request_timing(self) -> None:
        timing = self._value.get("timing")
        if self._request is not None and isinstance(timing, Mapping):
            self._request._merge_timing(cast(Mapping[str, Any], timing))

    def _merge(self, value: Mapping[str, Any], request: Request | None) -> None:
        previous_security_details = self._value.get("security_details")
        self._value.update(value)
        if previous_security_details is not None and value.get("security_details") is None:
            self._value["security_details"] = previous_security_details
        body_owner = cast("_native.NativeResponseBody | None", value.get("body_owner"))
        if body_owner is not None:
            self._body_owner = body_owner
        finished_owner = cast(
            "_native.NativeResponseFinishedOwner | None", value.get("finished_owner")
        )
        if finished_owner is not None:
            self._finished_owner = finished_owner
        if request is not None:
            self._request = request
        self._merge_request_timing()

    @property
    def url(self) -> str:
        if self._request is not None:
            return self._request.url
        return str(self._value["url"])

    @property
    def status(self) -> int:
        return int(self._value["status"])

    @property
    def status_text(self) -> str:
        return str(self._value["status_text"])

    @property
    def ok(self) -> bool:
        return bool(self._value["ok"])

    @property
    def headers(self) -> dict[str, str]:
        return dict(self._value["headers"])

    @property
    def from_service_worker(self) -> bool:
        return bool(self._value.get("from_service_worker", False))

    @property
    def request(self) -> Request:
        if self._request is None:
            raise UnsupportedError("Response.request capability is not available for this Response")
        return self._request

    @property
    def frame(self) -> Frame:
        if self._frame_override is not None:
            return self._frame_override
        return self.request.frame

    async def body(self) -> bytes:
        if self._native is not None and self._body_owner is not None:
            return bytes(await self._native.response_body(self._body_owner))
        if self._native is not None and self._navigation_body is not None:
            return bytes(await self._native.frame_response_body(*self._navigation_body))
        raise UnsupportedError("Response.body capability is not available for this Response")

    async def text(self) -> str:
        return (await self.body()).decode()

    async def json(self) -> Any:
        return json.loads(await self.text())

    async def finished(self) -> None:
        if self._native is None or self._finished_owner is None:
            raise UnsupportedError(
                "Response.finished capability is not available for this Response"
            )
        await self._native.response_finished(self._finished_owner)

    async def security_details(self) -> SecurityDetails | None:
        if "security_details" not in self._value:
            raise UnsupportedError(
                "Response.security_details capability is not available for this Response"
            )
        details = self._value["security_details"]
        if details is None:
            return None
        if not isinstance(details, Mapping):
            raise Error("Invalid native Response.security_details value")
        return cast(SecurityDetails, dict(details))


NetworkValue = Union[Request, Response]
NetworkMatcher = Union[str, Pattern[str], Callable[[NetworkValue], object]]
TNetwork = TypeVar("TNetwork", bound=NetworkValue)


def _network_value(
    native: _native.NativeSession,
    value: Mapping[str, Any],
    context: BrowserContext | None = None,
) -> NetworkValue:
    if context is not None:
        return context._wrap_network_value(value)
    if value.get("kind") == "response":
        return Response(value, native, context)
    return Request(value, native, context)


async def _network_matches(
    matcher: NetworkMatcher | None,
    value: NetworkValue,
    base_url: str | None,
) -> bool:
    if matcher is None:
        return True
    if isinstance(matcher, str):
        expected = (
            urljoin(base_url, matcher)
            if base_url and not any(character in matcher for character in "*?[")
            else matcher
        )
        if any(c in expected for c in "*?["):
            return fnmatch.fnmatchcase(value.url, expected)
        return value.url == expected
    if isinstance(matcher, Pattern):
        return matcher.search(value.url) is not None
    outcome = matcher(value)
    if inspect.isawaitable(outcome):
        outcome = await cast(Awaitable[object], outcome)
    return bool(outcome)


class EventContextManager(Generic[TNetwork]):
    def __init__(
        self,
        native: _native.NativeSession,
        arm: Callable[[str, str, int], Awaitable[int]],
        scope_id: str,
        context: BrowserContext,
        kind: str,
        matcher: NetworkMatcher | None,
        timeout_ms: int,
    ) -> None:
        self._native = native
        self._arm = arm
        self._scope_id = scope_id
        self._context = context
        self._kind = kind
        self._matcher = matcher
        self._timeout_ms = timeout_ms
        self._waiter_id: int | None = None
        self._task: asyncio.Task[TNetwork] | None = None

    @property
    def value(self) -> asyncio.Future[TNetwork]:
        if self._task is None:
            raise RuntimeError("event context manager has not been entered")
        return self._task

    async def __aenter__(self) -> EventContextManager[TNetwork]:
        self._waiter_id = int(await self._arm(self._scope_id, self._kind, self._timeout_ms))
        self._task = asyncio.create_task(self._consume())
        return self

    async def _consume(self) -> TNetwork:
        waiter_id = self._waiter_id
        if waiter_id is None:
            raise RuntimeError("network waiter is not armed")
        while True:
            raw = cast(
                Mapping[str, Any],
                await self._native.next_network_event(waiter_id),
            )
            value = _network_value(self._native, raw, self._context)
            if await _network_matches(self._matcher, value, self._context._base_url):
                return cast(TNetwork, value)

    async def _cancel(self) -> None:
        waiter_id = self._waiter_id
        if waiter_id is None:
            return
        self._waiter_id = None
        await self._native.cancel_network_wait(waiter_id)

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        _exc: BaseException | None,
        _traceback: object,
    ) -> bool:
        task = self._task
        if task is None:
            return False
        if exc_type is not None:
            task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await task
            with contextlib.suppress(BaseException):
                await self._cancel()
            return False
        try:
            await task
        except BaseException:
            with contextlib.suppress(BaseException):
                await self._cancel()
            raise
        await self._cancel()
        return False


PageEventPredicate = Callable[[object], object]


class PageEventContextManager:
    def __init__(
        self,
        page: Page,
        event: str,
        predicate: PageEventPredicate | None,
        timeout_ms: int,
    ) -> None:
        self._page = page
        self._event = event
        self._predicate = predicate
        self._timeout_ms = timeout_ms
        self._waiter: asyncio.Future[object] | None = None
        self._task: asyncio.Task[object] | None = None
        self._console_subscription_token: int | None = None

    @property
    def value(self) -> asyncio.Future[object]:
        if self._task is None:
            raise RuntimeError("event context manager has not been entered")
        return self._task

    async def __aenter__(self) -> PageEventContextManager:
        not_before_micros = 0
        try:
            if self._event == "console":
                not_before_micros = time.time_ns() // 1_000
                self._console_subscription_token = (
                    self._page._context.browser._native.subscribe_page_console(
                        self._page._object_id
                    )
                )
            self._waiter = self._page._arm_page_event(
                self._event, self._predicate, not_before_micros
            )
            self._task = asyncio.create_task(self._wait())
            if self._event == "filechooser":
                await self._page._flush_file_chooser_interception()
            if self._event == "console":
                # Arm synchronously before the first await so a waiter started
                # beside the action cannot lose the action's console event.
                # The timestamp cutoff keeps events published by the fence
                # itself from satisfying this newly armed waiter.
                await self._page._context.browser._native.flush_lifecycle_events()
                await asyncio.sleep(0)
        except BaseException:
            task = self._task
            if task is not None:
                task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await task
            waiter = self._waiter
            if waiter is not None:
                self._page._remove_page_event_waiter(self._event, waiter)
            self._release_console_subscription()
            raise
        return self

    def _release_console_subscription(self) -> None:
        token = self._console_subscription_token
        if token is None:
            return
        self._console_subscription_token = None
        with contextlib.suppress(Error):
            self._page._context.browser._native.release_console_subscription(token)

    async def _wait(self) -> object:
        waiter = self._waiter
        if waiter is None:
            raise RuntimeError("Page event waiter is not armed")
        if self._timeout_ms == 0:
            return await waiter

        def expire() -> None:
            if not waiter.done():
                waiter.set_exception(
                    TimeoutError(
                        f'Timeout {self._timeout_ms}ms exceeded while waiting for event "{self._event}"'
                    )
                )

        timeout_handle = asyncio.get_running_loop().call_later(self._timeout_ms / 1000, expire)
        try:
            return await waiter
        finally:
            timeout_handle.cancel()

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        _exc: BaseException | None,
        _traceback: object,
    ) -> bool:
        try:
            task = self._task
            waiter = self._waiter
            if task is None or waiter is None:
                return False
            if exc_type is not None:
                task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await task
                self._page._remove_page_event_waiter(self._event, waiter)
                return False
            try:
                await task
            finally:
                self._page._remove_page_event_waiter(self._event, waiter)
            return False
        finally:
            self._release_console_subscription()
            if self._event == "filechooser":
                await self._page._flush_file_chooser_interception()


class NavigationContextManager:
    def __init__(
        self,
        page: Page,
        wait_until: str,
        timeout_ms: int,
        frame: Frame | None = None,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
    ) -> None:
        self._page = page
        self._frame = frame
        self._url = url
        self._wait_until = wait_until
        self._timeout_ms = timeout_ms
        self._task: asyncio.Task[Response | None] | None = None

    @property
    def value(self) -> asyncio.Future[Response | None]:
        if self._task is None:
            raise RuntimeError("navigation context manager has not been entered")
        return self._task

    def is_done(self) -> bool:
        task = self._task
        return task is not None and task.done()

    async def __aenter__(self) -> NavigationContextManager:
        native = self._page._context.browser._native
        frame_id = self._page._main_frame_id if self._frame is None else self._frame._object_id
        if self._url is None:
            pending = native.wait_for_navigation(
                self._page._object_id,
                frame_id,
                self._wait_until,
                self._timeout_ms,
            )
        else:
            pending = native.wait_for_navigation_matching(
                self._page._object_id,
                frame_id,
                self._url,
                self._wait_until,
                self._timeout_ms,
            )
        self._task = asyncio.create_task(self._wait(pending))
        return self

    async def _wait(self, pending: Awaitable[Mapping[str, Any] | None]) -> Response | None:
        try:
            value = await pending
        except TargetClosedError as error:
            if self._frame is not None and str(error).lower() == "frame was detached":
                raise TargetClosedError("frame was detached") from None
            raise
        self._page._refresh_snapshot()
        if self._frame is not None:
            self._page._refresh_frames()
        if value is None:
            return None
        if self._frame is not None:
            return await self._page._frame_navigation_response(value, self._frame)
        return self._page._navigation_response(value)

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        _exc: BaseException | None,
        _traceback: object,
    ) -> bool:
        task = self._task
        if task is None:
            return False
        if exc_type is not None:
            task.cancel()
            with contextlib.suppress(BaseException):
                await task
            return False
        await task
        return False


WorkerEventPredicate = Callable[[object], object]


class WorkerEventContextManager:
    def __init__(
        self,
        worker: Worker,
        event: str,
        predicate: WorkerEventPredicate | None,
        timeout_ms: int,
    ) -> None:
        self._worker = worker
        self._event = event
        self._predicate = predicate
        self._timeout_ms = timeout_ms
        self._waiter: asyncio.Future[object] | None = None
        self._task: asyncio.Task[object] | None = None

    @property
    def value(self) -> asyncio.Future[object]:
        if self._task is None:
            raise RuntimeError("event context manager has not been entered")
        return self._task

    async def __aenter__(self) -> WorkerEventContextManager:
        self._waiter = self._worker._arm_worker_event(self._event, self._predicate)
        self._task = asyncio.create_task(self._wait())
        return self

    async def _wait(self) -> object:
        waiter = self._waiter
        if waiter is None:
            raise RuntimeError("Worker event waiter is not armed")
        if self._timeout_ms == 0:
            return await waiter
        try:
            return await asyncio.wait_for(waiter, self._timeout_ms / 1000)
        except asyncio.TimeoutError as error:
            raise TimeoutError(
                f'Timeout {self._timeout_ms}ms exceeded while waiting for event "{self._event}"'
            ) from error

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        _exc: BaseException | None,
        _traceback: object,
    ) -> bool:
        task = self._task
        waiter = self._waiter
        if task is None or waiter is None:
            return False
        if exc_type is not None:
            task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await task
            self._worker._remove_worker_event_waiter(self._event, waiter)
            return False
        try:
            await task
        finally:
            self._worker._remove_worker_event_waiter(self._event, waiter)
        return False


class BrowserContextEventContextManager:
    def __init__(
        self,
        context: BrowserContext,
        event: str,
        predicate: PageEventPredicate | None,
        timeout_ms: int,
    ) -> None:
        self._context = context
        self._event = event
        self._predicate = predicate
        self._timeout_ms = timeout_ms
        self._waiter: asyncio.Future[object] | None = None
        self._task: asyncio.Task[object] | None = None
        self._console_subscription_token: int | None = None

    @property
    def value(self) -> asyncio.Future[object]:
        if self._task is None:
            raise RuntimeError("event context manager has not been entered")
        return self._task

    async def __aenter__(self) -> BrowserContextEventContextManager:
        not_before_micros = 0
        try:
            if self._event == "console":
                not_before_micros = time.time_ns() // 1_000
                self._console_subscription_token = (
                    self._context.browser._native.subscribe_context_console(
                        self._context._object_id
                    )
                )
            self._waiter = self._context._arm_context_event(
                self._event, self._predicate, not_before_micros
            )
            self._task = asyncio.create_task(self._wait())
            if self._event == "console":
                # See PageEventContextManager: the waiter must exist before
                # yielding, while the timestamp cutoff rejects fenced history.
                await self._context.browser._native.flush_lifecycle_events()
                await asyncio.sleep(0)
        except BaseException:
            task = self._task
            if task is not None:
                task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await task
            waiter = self._waiter
            if waiter is not None:
                self._context._remove_context_event_waiter(self._event, waiter)
            self._release_console_subscription()
            raise
        return self

    def _release_console_subscription(self) -> None:
        token = self._console_subscription_token
        if token is None:
            return
        self._console_subscription_token = None
        with contextlib.suppress(Error):
            self._context.browser._native.release_console_subscription(token)

    async def _wait(self) -> object:
        waiter = self._waiter
        if waiter is None:
            raise RuntimeError("BrowserContext event waiter is not armed")
        if self._timeout_ms == 0:
            return await waiter
        try:
            return await asyncio.wait_for(waiter, self._timeout_ms / 1000)
        except asyncio.TimeoutError as error:
            raise TimeoutError(
                f'Timeout {self._timeout_ms}ms exceeded while waiting for event "{self._event}"'
            ) from error

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        _exc: BaseException | None,
        _traceback: object,
    ) -> bool:
        try:
            task = self._task
            waiter = self._waiter
            if task is None or waiter is None:
                return False
            if exc_type is not None:
                task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await task
                self._context._remove_context_event_waiter(self._event, waiter)
                return False
            try:
                await task
            finally:
                self._context._remove_context_event_waiter(self._event, waiter)
            return False
        finally:
            self._release_console_subscription()


def _handle_preview(value: str, is_element: bool) -> str:
    if value.startswith("JSHandle@"):
        return value
    if is_element and re.fullmatch(r"[a-z][a-z0-9:-]*", value):
        void_elements = {
            "area",
            "base",
            "br",
            "col",
            "command",
            "embed",
            "hr",
            "img",
            "input",
            "keygen",
            "link",
            "menuitem",
            "meta",
            "param",
            "source",
            "track",
            "wbr",
        }
        return f"JSHandle@<{value}/>" if value in void_elements else f"JSHandle@<{value}></{value}>"
    if not value:
        return "JSHandle@node" if is_element else "JSHandle@object"
    return value


_MAX_EVALUATION_ARGUMENT_DEPTH = 64
_MAX_EVALUATION_ARGUMENT_NODES = 4096
_MAX_EVALUATION_ARGUMENT_ENTRIES = 4096


class _NativeEvaluationArgumentConverter:
    def __init__(self) -> None:
        self._memo: dict[int, object] = {}
        self._nodes = 0
        self._entries = 0
        self.has_handles = False

    def _charge_node(self, depth: int) -> None:
        if depth > _MAX_EVALUATION_ARGUMENT_DEPTH:
            raise InvalidArgumentError("evaluation argument depth exceeds limit")
        self._nodes += 1
        if self._nodes > _MAX_EVALUATION_ARGUMENT_NODES:
            raise InvalidArgumentError("evaluation argument node count exceeds limit")

    def _charge_entries(self, count: int) -> None:
        self._entries += count
        if self._entries > _MAX_EVALUATION_ARGUMENT_ENTRIES:
            raise InvalidArgumentError("evaluation argument entry count exceeds limit")

    def convert(self, item: object, depth: int = 0) -> object:
        self._charge_node(depth)
        if isinstance(item, JSHandle):
            self.has_handles = True
            return item._native
        if isinstance(item, list):
            source = cast(list[object], item)
            identity = id(source)
            if identity in self._memo:
                return self._memo[identity]
            self._charge_entries(len(source))
            output: list[object] = []
            self._memo[identity] = output
            output.extend(self.convert(child, depth + 1) for child in source)
            return output
        if isinstance(item, ParseResult):
            # Preserve this tuple subtype for the Rust evaluation value codec.
            # It owns URL serialization and the corresponding byte budget.
            return item
        if isinstance(item, tuple):
            source = cast(tuple[object, ...], item)
            identity = id(source)
            if identity in self._memo:
                return self._memo[identity]
            self._charge_entries(len(source))
            output = []
            self._memo[identity] = output
            output.extend(self.convert(child, depth + 1) for child in source)
            return output
        if isinstance(item, dict):
            source = cast(dict[object, object], item)
            identity = id(source)
            if identity in self._memo:
                return self._memo[identity]
            self._charge_entries(len(source))
            output_dict: dict[object, object] = {}
            self._memo[identity] = output_dict
            for key, child in source.items():
                output_dict[key] = self.convert(child, depth + 1)
            return output_dict
        return item


def _native_evaluation_argument(value: object) -> object:
    """Replace public Handle wrappers while preserving bounded container identity."""

    return _NativeEvaluationArgumentConverter().convert(value)


class JSHandle:
    def __init__(
        self,
        session: _native.NativeSession,
        value: _native.NativeHandle,
        page: Page | None = None,
    ) -> None:
        self._session = session
        self._native = value
        self._page = page
        self._preview = _handle_preview(value.preview(), value.is_element())

    @property
    def _impl_obj(self) -> JSHandle:
        """Return the read-only compatibility view used by upstream assertions."""

        return self

    def __repr__(self) -> str:
        return f"<JSHandle preview={self._preview}>"

    def __str__(self) -> str:
        return self._preview

    def as_element(self) -> ElementHandle | None:
        return None

    async def evaluate(self, expression: str, arg: object = None) -> object:
        return await self._session.handle_evaluate(
            self._native, expression, _native_evaluation_argument(arg)
        )

    async def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        value = await self._session.handle_evaluate_handle(
            self._native, expression, _native_evaluation_argument(arg)
        )
        return _wrap_handle(self._session, value, self._page)

    async def get_property(self, property_name: str) -> JSHandle:
        value = await self._session.handle_get_property(self._native, property_name)
        return _wrap_handle(self._session, value, self._page)

    async def get_properties(self) -> dict[str, JSHandle]:
        values = await self._session.handle_get_properties(self._native)
        return {
            name: _wrap_handle(self._session, value, self._page) for name, value in values.items()
        }

    async def json_value(self) -> object:
        return await self._session.handle_json_value(self._native)

    async def dispose(self) -> None:
        await self._session.handle_dispose(self._native)


class ElementHandle(JSHandle):
    def as_element(self) -> ElementHandle:
        return self

    async def _state_predicate(self, operation: str, api_name: str) -> bool:
        if "element-handle-state-predicates" not in _native.selected_features():
            raise UnsupportedError(
                f"ElementHandle.{api_name} requires the element-handle-state-predicates profile"
            )
        try:
            return await self._session.element_handle_state_predicate(self._native, operation)
        except Error as error:
            raise rewrite_api_error(error, f"ElementHandle.{api_name}") from None

    async def is_disabled(self) -> bool:
        return await self._state_predicate("isDisabled", "is_disabled")

    async def is_editable(self) -> bool:
        return await self._state_predicate("isEditable", "is_editable")

    async def is_enabled(self) -> bool:
        return await self._state_predicate("isEnabled", "is_enabled")

    async def is_hidden(self) -> bool:
        return await self._state_predicate("isHidden", "is_hidden")

    async def is_visible(self) -> bool:
        return await self._state_predicate("isVisible", "is_visible")

    async def wait_for_element_state(self, state: str, *, timeout: float | None = None) -> None:
        if "element-handle-state-wait" not in _native.selected_features():
            raise UnsupportedError(
                "ElementHandle.wait_for_element_state requires the "
                "element-handle-state-wait profile"
            )
        state_value = _string_argument(state, "ElementHandle.wait_for_element_state state")
        timeout_ms = (
            self._page._effective_timeout(timeout)
            if self._page is not None
            else _timeout_ms(timeout)
        )
        try:
            await self._session.element_handle_wait_for_element_state(
                self._native, state_value, timeout_ms
            )
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.wait_for_element_state") from None

    async def scroll_into_view_if_needed(self, *, timeout: float | None = None) -> None:
        if "element-handle-scroll-into-view" not in _native.selected_features():
            raise UnsupportedError(
                "ElementHandle.scroll_into_view_if_needed requires the "
                "element-handle-scroll-into-view profile"
            )
        timeout_ms = (
            self._page._effective_timeout(timeout)
            if self._page is not None
            else _timeout_ms(timeout)
        )
        try:
            await self._session.element_handle_scroll_into_view_if_needed(self._native, timeout_ms)
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.scroll_into_view_if_needed") from None

    async def select_text(self, *, force: bool | None = None, timeout: float | None = None) -> None:
        if "element-handle-select-text" not in _native.selected_features():
            raise UnsupportedError(
                "ElementHandle.select_text requires the element-handle-select-text profile"
            )
        force_value = (
            False if force is None else _boolean_argument(force, "ElementHandle.select_text force")
        )
        timeout_ms = (
            self._page._effective_timeout(timeout)
            if self._page is not None
            else _timeout_ms(timeout)
        )
        try:
            await self._session.element_handle_select_text(self._native, force_value, timeout_ms)
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.select_text") from None

    async def click(
        self,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
        delay: float | None = None,
        force: bool | None = None,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        no_wait_after: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        steps: int | None = None,
    ) -> None:
        if "element-handle-click" not in _native.selected_features():
            raise UnsupportedError("ElementHandle.click requires the element-handle-click profile")
        if self._page is None:
            raise UnsupportedError("ElementHandle.click requires its Page owner")
        options: dict[str, object] = {}
        if button is not None:
            if button not in ("left", "right", "middle"):
                raise InvalidArgumentError(
                    "ElementHandle.click button must be left, right, or middle"
                )
            options["button"] = button
        if click_count is not None:
            if (
                isinstance(click_count, bool)
                or not isinstance(cast(object, click_count), int)
                or not 1 <= click_count <= 255
            ):
                raise InvalidArgumentError(
                    "ElementHandle.click click_count must be an integer in [1, 255]"
                )
            options["click_count"] = click_count
        if delay is not None:
            options["delay"] = _optional_fractional_milliseconds(delay, "delay")
        if force is not None:
            options["force"] = _boolean_argument(force, "ElementHandle.click force")
        if modifiers is not None:
            if "locator-click-modifiers" not in _native.selected_features():
                raise UnsupportedError(
                    "ElementHandle.click modifiers require the locator-click-modifiers profile"
                )
            if isinstance(modifiers, (str, bytes)) or len(modifiers) > 5:
                raise InvalidArgumentError(
                    "ElementHandle.click modifiers must be a sequence of at most five names"
                )
            if any(
                not isinstance(cast(object, name), str)
                or name not in ("Alt", "Control", "ControlOrMeta", "Meta", "Shift")
                for name in modifiers
            ):
                raise InvalidArgumentError("ElementHandle.click modifiers contain an unknown name")
            options["modifiers"] = list(modifiers)
        if no_wait_after is not None:
            options["no_wait_after"] = _boolean_argument(
                no_wait_after, "ElementHandle.click no_wait_after"
            )
        if position is not None:
            if (
                not isinstance(cast(object, position), Mapping)
                or len(position) != 2
                or "x" not in position
                or "y" not in position
            ):
                raise InvalidArgumentError("ElementHandle.click position must contain x and y")
            options["position"] = {
                "x": _mouse_number(position["x"], "ElementHandle.click position.x"),
                "y": _mouse_number(position["y"], "ElementHandle.click position.y"),
            }
        if trial is not None:
            options["trial"] = _boolean_argument(trial, "ElementHandle.click trial")
        if steps is not None:
            if (
                isinstance(steps, bool)
                or not isinstance(cast(object, steps), int)
                or not 1 <= steps <= 10_000
            ):
                raise InvalidArgumentError(
                    "ElementHandle.click steps must be an integer in [1, 10000]"
                )
            options["steps"] = steps
        try:
            await self._session.element_handle_click(
                self._native, options, self._page._effective_timeout(timeout)
            )
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.click") from None
        self._page._refresh_snapshot()

    async def tap(
        self,
        *,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        if "element-handle-tap" not in _native.selected_features():
            raise UnsupportedError("ElementHandle.tap requires the element-handle-tap profile")
        options = _tap_options(
            "ElementHandle.tap",
            modifiers=modifiers,
            position=position,
            force=force,
            no_wait_after=no_wait_after,
            trial=trial,
        )
        timeout_ms = (
            self._page._effective_timeout(timeout)
            if self._page is not None
            else _timeout_ms(timeout)
        )
        try:
            await self._session.element_handle_tap(self._native, options, timeout_ms)
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.tap") from None
        if self._page is not None:
            self._page._refresh_snapshot()

    async def dispatch_event(
        self, type: str, event_init: Mapping[str, object] | None = None
    ) -> None:
        event_type = _string_argument(type, "ElementHandle.dispatch_event")
        if event_init is not None and not isinstance(cast(object, event_init), Mapping):
            raise TypeError("ElementHandle.dispatch_event event_init must be a mapping or None")
        init = None if event_init is None else dict(event_init)
        has_handles = False

        def handle_leaf(value: object) -> None:
            nonlocal has_handles
            if not isinstance(value, JSHandle):
                raise TypeError("unsupported event initializer value")
            has_handles = True

        try:
            json.dumps(init, allow_nan=False, default=handle_leaf)
        except (TypeError, ValueError) as error:
            raise TypeError(
                "ElementHandle.dispatch_event event_init must be JSON-serializable or JSHandles"
            ) from error
        if has_handles:
            await self._session.element_handle_dispatch_event_owned(
                self._native,
                event_type,
                cast(dict[str, object], _native_evaluation_argument(init)),
            )
            return
        await self._session.element_handle_dispatch_event(self._native, event_type, init)

    async def inner_text(self) -> str:
        return await self._session.element_handle_inner_text(self._native)

    async def _dom_read(
        self, operation: str, api_name: str, attribute: str | None = None
    ) -> str | bool | None:
        if "element-handle-dom-reads" not in _native.selected_features():
            raise UnsupportedError(
                f"ElementHandle.{api_name} requires the element-handle-dom-reads profile"
            )
        try:
            return await self._session.element_handle_dom_read(self._native, operation, attribute)
        except Error as error:
            raise rewrite_api_error(error, f"ElementHandle.{api_name}") from None

    async def get_attribute(self, name: str) -> str | None:
        attribute = _string_argument(name, "ElementHandle.get_attribute")
        return cast("str | None", await self._dom_read("getAttribute", "get_attribute", attribute))

    async def inner_html(self) -> str:
        return cast(str, await self._dom_read("innerHTML", "inner_html"))

    async def input_value(self, *, timeout: float | None = None) -> str:
        if timeout is not None:
            _timeout_ms(timeout)
        return cast(str, await self._dom_read("inputValue", "input_value"))

    async def is_checked(self) -> bool:
        return cast(bool, await self._dom_read("isChecked", "is_checked"))

    async def bounding_box(self) -> dict[str, float] | None:
        try:
            return await self._session.element_handle_bounding_box(self._native)
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.bounding_box") from None

    async def query_selector(self, selector: str) -> ElementHandle | None:
        value = await self._session.element_handle_query_selector(self._native, selector)
        if value is None:
            return None
        return ElementHandle(self._session, value, self._page)

    async def press(
        self,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        del timeout, no_wait_after
        if self._page is None:
            raise UnsupportedError("ElementHandle.press requires its Page owner")
        delay_ms = _optional_fractional_milliseconds(delay, "delay")
        try:
            await self._session.element_handle_keyboard(
                self._native,
                self._page._object_id,
                "press",
                _string_argument(key, "ElementHandle.press"),
                0.0 if delay_ms is None else delay_ms,
            )
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.press") from None

    async def type(
        self,
        text: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        del timeout, no_wait_after
        if self._page is None:
            raise UnsupportedError("ElementHandle.type requires its Page owner")
        delay_ms = _optional_fractional_milliseconds(delay, "delay")
        try:
            await self._session.element_handle_keyboard(
                self._native,
                self._page._object_id,
                "type",
                _string_argument(text, "ElementHandle.type"),
                0.0 if delay_ms is None else delay_ms,
            )
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.type") from None

    async def owner_frame(self) -> Frame | None:
        if "element-handle-dom" not in _native.selected_features():
            raise UnsupportedError(
                "ElementHandle.owner_frame requires the element-handle-dom profile"
            )
        if self._page is None:
            raise UnsupportedError("ElementHandle.owner_frame requires its Page owner")
        try:
            frame_id = await self._session.element_handle_owner_frame(self._native)
        except Error as error:
            raise rewrite_api_error(error, "ElementHandle.owner_frame") from None
        if frame_id is None:
            return None
        for page in self._page._context.pages:
            for frame in page._refresh_frames():
                if frame._object_id == frame_id:
                    return frame
        raise Error("ElementHandle owner Frame is no longer available.")

    async def content_frame(self) -> Frame | None:
        if self._page is None:
            raise UnsupportedError("ElementHandle.content_frame requires its Page owner")
        frame_id = await self._session.element_handle_content_frame(self._native)
        if frame_id is None:
            return None
        frame = self._page._frames.get(frame_id)
        if frame is None:
            self._page._refresh_frames()
            frame = self._page._frames.get(frame_id)
        if frame is None:
            raise Error("ElementHandle content Frame is no longer available.")
        return frame

    async def screenshot(
        self,
        *,
        type: str | None = None,
        path: str | Path | None = None,
        quality: int | None = None,
        scale: str = "device",
        timeout: float | None = None,
        mask: Sequence[Locator] | None = None,
    ) -> bytes:
        format_value, output_path, quality_value, scale_value = _screenshot_arguments(
            "ElementHandle.screenshot",
            type=type,
            path=path,
            quality=quality,
            scale=scale,
        )
        return await self._session.element_screenshot(
            self._native,
            format_value,
            output_path,
            quality_value,
            scale_value,
            30_000 if timeout is None else _timeout_ms(timeout),
            _screenshot_masks(mask, "ElementHandle.screenshot"),
        )

    async def set_input_files(
        self,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        del no_wait_after
        paths, payloads = _file_input_values(files)
        timeout_ms = (
            self._page._effective_timeout(timeout)
            if self._page is not None
            else _timeout_ms(timeout)
        )
        if paths is not None:
            await self._session.element_handle_set_input_files_paths(
                self._native, os.getcwd(), paths, timeout_ms
            )
            return
        assert payloads is not None
        await self._session.element_handle_set_input_file_payloads(
            self._native, payloads, timeout_ms
        )


def _wrap_handle(
    session: _native.NativeSession,
    value: _native.NativeHandle,
    page: Page | None = None,
) -> JSHandle:
    if value.is_element():
        return ElementHandle(session, value, page)
    return JSHandle(session, value, page)


def _drag_target(value: object) -> Locator:
    if not isinstance(value, Locator):
        raise TypeError("Locator.drag_to target must be a Locator")
    return value


def _locator_argument(value: object, api: str) -> Locator:
    if not isinstance(value, Locator):
        raise TypeError(f"{api} requires a Locator")
    return value


def _screenshot_arguments(
    api: str,
    *,
    type: str | None,
    path: str | Path | None,
    quality: int | None,
    scale: str,
) -> tuple[int, str | None, int | None, int]:
    if type not in (None, "png", "jpeg"):
        raise TypeError(f"{api} type must be 'png' or 'jpeg'")
    output_path = None if path is None else cast("str | bytes", os.fspath(path))
    if output_path is not None and not isinstance(output_path, str):
        raise TypeError(f"{api} path must resolve to a string")
    quality_value = cast(object, quality)
    if quality is not None and (
        isinstance(quality_value, bool)
        or not isinstance(quality_value, int)
        or not 0 <= quality_value <= 100
    ):
        raise TypeError(f"{api} quality must be an integer in [0, 100]")
    if scale not in ("device", "css"):
        raise TypeError(f"{api} scale must be 'device' or 'css'")
    return (
        -1 if type is None else 1 if type == "jpeg" else 0,
        output_path,
        quality,
        1 if scale == "css" else 0,
    )


def _screenshot_masks(
    mask: Sequence[Locator] | None,
    api: str,
) -> list[_native.NativeLocator]:
    if mask is None:
        return []
    values = list(cast(Sequence[object], mask))
    if len(values) > 64:
        raise ResourceExhaustedError(f"{api} mask exceeds 64 Locators")
    native: list[_native.NativeLocator] = []
    for value in values:
        if not isinstance(value, Locator):
            raise TypeError(f"{api} mask must contain Locators")
        if value._native_locator is None:
            raise UnsupportedError(f"{api} mask requires Rust-owned Locators")
        native.append(value._native_locator)
    return native


def _boolean_argument(value: object, api: str) -> bool:
    if not isinstance(value, bool):
        raise TypeError(f"{api} must be a boolean")
    return value


def _optional_boolean_argument(value: object, api: str) -> bool | None:
    if value is None:
        return None
    return _boolean_argument(value, api)


def _bounded_cookie_string(value: object, api: str, field: str, maximum: int) -> tuple[str, int]:
    value = _string_argument(value, api)
    # One Unicode scalar is at least one UTF-8 byte. Rejecting on character
    # count first bounds the temporary encoding used for the exact byte check.
    if len(value) > maximum:
        raise ResourceExhaustedError(f"{field} exceeds {maximum} UTF-8 bytes")
    size = len(value.encode("utf-8"))
    if size > maximum:
        raise ResourceExhaustedError(f"{field} exceeds {maximum} UTF-8 bytes")
    return value, size


def _optional_cookie_string(
    value: object, api: str, field: str, maximum: int
) -> tuple[str | None, int]:
    if value is None:
        return None, 0
    return _bounded_cookie_string(value, api, field, maximum)


def _cookie_inputs(
    cookies: Sequence[SetCookieParam],
) -> list[_native.NativeContextCookieInput]:
    if len(cookies) > 1024:
        raise ResourceExhaustedError("cookie input count exceeds 1024")
    values: list[_native.NativeContextCookieInput] = []
    total_bytes = 0
    for cookie in cast(Sequence[object], cookies):
        if not isinstance(cookie, Mapping):
            raise TypeError("BrowserContext.add_cookies cookie must be a mapping")
        mapping = cast(Mapping[str, object], cookie)
        name, name_bytes = _bounded_cookie_string(
            mapping.get("name"),
            "BrowserContext.add_cookies name",
            "cookie name",
            _MAX_COOKIE_FIELD_BYTES,
        )
        value, value_bytes = _bounded_cookie_string(
            mapping.get("value"),
            "BrowserContext.add_cookies value",
            "cookie value",
            _MAX_COOKIE_FIELD_BYTES,
        )
        url, url_bytes = _optional_cookie_string(
            mapping.get("url"),
            "BrowserContext.add_cookies url",
            "cookie URL",
            _MAX_COOKIE_URL_BYTES,
        )
        domain, domain_bytes = _optional_cookie_string(
            mapping.get("domain"),
            "BrowserContext.add_cookies domain",
            "cookie domain",
            _MAX_COOKIE_FIELD_BYTES,
        )
        path, path_bytes = _optional_cookie_string(
            mapping.get("path"),
            "BrowserContext.add_cookies path",
            "cookie path",
            _MAX_COOKIE_FIELD_BYTES,
        )
        same_site, same_site_bytes = _optional_cookie_string(
            mapping.get("sameSite"),
            "BrowserContext.add_cookies sameSite",
            "cookie sameSite",
            _MAX_COOKIE_FIELD_BYTES,
        )
        partition_key, partition_key_bytes = _optional_cookie_string(
            mapping.get("partitionKey"),
            "BrowserContext.add_cookies partitionKey",
            "cookie partition key",
            _MAX_COOKIE_URL_BYTES,
        )
        total_bytes += (
            name_bytes
            + value_bytes
            + url_bytes
            + domain_bytes
            + path_bytes
            + same_site_bytes
            + partition_key_bytes
        )
        if total_bytes > _MAX_COOKIE_BATCH_BYTES:
            raise ResourceExhaustedError(
                f"cookie input batch exceeds {_MAX_COOKIE_BATCH_BYTES} bytes"
            )
        expires_value = mapping.get("expires")
        if expires_value is not None and (
            isinstance(expires_value, bool)
            or not isinstance(expires_value, (int, float))
            or not math.isfinite(expires_value)
        ):
            raise TypeError("BrowserContext.add_cookies expires must be a finite number")
        values.append(
            (
                name,
                value,
                url,
                domain,
                path,
                None if expires_value is None else float(expires_value),
                _optional_boolean_argument(
                    mapping.get("httpOnly"), "BrowserContext.add_cookies httpOnly"
                ),
                _optional_boolean_argument(
                    mapping.get("secure"), "BrowserContext.add_cookies secure"
                ),
                same_site,
                partition_key,
            )
        )
    return values


def _role_level(value: object) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError("get_by_role level must be an integer")
    if value < 1 or value > 4_294_967_295:
        raise ValueError("get_by_role level must be between 1 and 4294967295")
    return value


def _role_arguments(
    role: object,
    *,
    checked: object = None,
    disabled: object = None,
    expanded: object = None,
    include_hidden: object = None,
    level: object = None,
    name: object = None,
    pressed: object = None,
    selected: object = None,
    exact: object = None,
) -> tuple[Any, ...]:
    role_value = _string_argument(role, "get_by_role")
    name_source, name_flags = (
        (None, None) if name is None else _text_match(name, "get_by_role name")
    )
    exact_value = _optional_boolean_argument(exact, "get_by_role exact") or False
    include_hidden_value = (
        _optional_boolean_argument(include_hidden, "get_by_role include_hidden") or False
    )
    return (
        role_value,
        name_source,
        name_flags,
        exact_value,
        _optional_boolean_argument(checked, "get_by_role checked"),
        _optional_boolean_argument(disabled, "get_by_role disabled"),
        _optional_boolean_argument(expanded, "get_by_role expanded"),
        include_hidden_value,
        _role_level(level),
        _optional_boolean_argument(pressed, "get_by_role pressed"),
        _optional_boolean_argument(selected, "get_by_role selected"),
    )


class Locator:
    def __init__(
        self,
        page: Page,
        selector: str | None,
        description: str | None = None,
        native_locator: _native.NativeLocator | None = None,
    ) -> None:
        self._page = page
        self._selector = (
            _string_argument(selector, "Page.locator") if selector is not None else None
        )
        if self._selector is None and native_locator is None:
            raise UnsupportedError("Locator requires a Rust-owned query")
        self._native_locator = native_locator
        self._description = description

    def __repr__(self) -> str:
        return f"<Locator frame={self._page.main_frame!r} selector={self._selector!r}>"

    def __str__(self) -> str:
        return self.__repr__()

    @property
    def page(self) -> Page:
        return self._page

    @property
    def description(self) -> str | None:
        return self._description

    def describe(self, description: str) -> Locator:
        return Locator(
            self._page,
            self._selector,
            _string_argument(description, "Locator.describe"),
            self._native_locator,
        )

    def locator(
        self,
        selector_or_locator: str | Locator,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.locator requires a Rust-owned query")
        if isinstance(selector_or_locator, Locator):
            if selector_or_locator._native_locator is None:
                raise UnsupportedError("Locator.locator requires a Rust-owned inner query")
            native = self._native_locator.combine(selector_or_locator._native_locator, "within")
            child_selector = selector_or_locator._selector
        else:
            value = _string_argument(selector_or_locator, "Locator.locator")
            native = self._native_locator.locator(value)
            child_selector = value
        selector = (
            f"{self._selector} >> {child_selector}"
            if self._selector is not None and child_selector is not None
            else None
        )
        result = Locator(self._page, selector, None, native)
        if any(value is not None for value in (has_text, has_not_text, has, has_not)):
            return result.filter(
                has_text=has_text,
                has_not_text=has_not_text,
                has=has,
                has_not=has_not,
            )
        return result

    def frame_locator(self, selector: str) -> FrameLocator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.frame_locator requires a Rust-owned query")
        value = _string_argument(selector, "Locator.frame_locator")
        owner_selector = f"{self._selector} >> {value}" if self._selector is not None else value
        return FrameLocator(
            self._page,
            owner_selector,
            self._native_locator.frame_locator(value),
        )

    @property
    def content_frame(self) -> FrameLocator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.content_frame requires a Rust-owned query")
        return FrameLocator(
            self._page,
            self._selector,
            self._native_locator.content_frame(),
        )

    def filter(
        self,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
        visible: bool | None = None,
    ) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.filter requires a Rust-owned query")
        native = self._native_locator
        for text, exclude, api in (
            (has_text, False, "Locator.filter has_text"),
            (has_not_text, True, "Locator.filter has_not_text"),
        ):
            if text is None:
                continue
            source, flags = _text_match(text, api)
            native = (
                native.filter_text(source, exclude)
                if flags is None
                else native.filter_regex(source, flags, exclude)
            )
        for inner, kind in ((has, "has"), (has_not, "hasNot")):
            if inner is None:
                continue
            inner_value = _locator_argument(inner, "Locator.filter inner query")
            if inner_value._native_locator is None:
                raise UnsupportedError("Locator.filter requires a Rust-owned inner query")
            native = native.combine(inner_value._native_locator, kind)
        if visible is not None:
            visible_value = _boolean_argument(visible, "Locator.filter visible")
            native = native.visible(visible_value)
        return Locator(self._page, None, None, native)

    def and_(self, locator: Locator) -> Locator:
        locator_value = _locator_argument(locator, "Locator.and_")
        if self._native_locator is None or locator_value._native_locator is None:
            raise UnsupportedError("Locator.and_ requires Rust-owned queries")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.combine(locator_value._native_locator, "and"),
        )

    def or_(self, locator: Locator) -> Locator:
        locator_value = _locator_argument(locator, "Locator.or_")
        if self._native_locator is None or locator_value._native_locator is None:
            raise UnsupportedError("Locator.or_ requires Rust-owned queries")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.combine(locator_value._native_locator, "or"),
        )

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.get_by_text requires a Rust-owned query")
        source, flags = _text_match(text, "Locator.get_by_text")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.semantic("text", source, bool(exact), flags),
        )

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.get_by_label requires a Rust-owned query")
        source, flags = _text_match(text, "Locator.get_by_label")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.semantic("label", source, bool(exact), flags),
        )

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.get_by_placeholder requires a Rust-owned query")
        source, flags = _text_match(text, "Locator.get_by_placeholder")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.semantic("placeholder", source, bool(exact), flags),
        )

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.get_by_alt_text requires a Rust-owned query")
        source, flags = _text_match(text, "Locator.get_by_alt_text")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.semantic("altText", source, bool(exact), flags),
        )

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.get_by_title requires a Rust-owned query")
        source, flags = _text_match(text, "Locator.get_by_title")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.semantic("title", source, bool(exact), flags),
        )

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.get_by_test_id requires a Rust-owned query")
        source, flags = _text_match(test_id, "Locator.get_by_test_id")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.semantic("testId", source, False, flags),
        )

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        if self._native_locator is None:
            raise UnsupportedError("Locator.get_by_role requires a Rust-owned query")
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.role(
                *_role_arguments(
                    role,
                    checked=checked,
                    disabled=disabled,
                    expanded=expanded,
                    include_hidden=include_hidden,
                    level=level,
                    name=name,
                    pressed=pressed,
                    selected=selected,
                    exact=exact,
                )
            ),
        )

    @property
    def first(self) -> Locator:
        return self.nth(0)

    @property
    def last(self) -> Locator:
        return self.nth(-1)

    def nth(self, index: int) -> Locator:
        value = _int32(index, "Locator.nth")
        if self._native_locator is None:
            raise UnsupportedError("Locator.nth requires a Rust-owned query")
        return Locator(self._page, None, None, self._native_locator.nth(value))

    def _selector_for(self, api: str) -> str:
        if self._selector is None:
            raise UnsupportedError(f"{api} requires a native Locator operation")
        return self._selector

    async def _read(
        self,
        operation: str,
        *,
        timeout: float | None = None,
        attribute: str | None = None,
    ) -> object:
        if self._native_locator is not None:
            return await self._native_locator.read(
                operation,
                self._page._effective_timeout(timeout),
                attribute,
            )
        selector = self._selector_for(f"Locator.{operation}")
        if "context-strict-selectors" in _native.selected_features():
            return await self._page._locator_read(
                selector, operation, timeout=timeout, attribute=attribute, strict=True
            )
        return await self._page._locator_read(
            selector, operation, timeout=timeout, attribute=attribute
        )

    async def click(
        self,
        *,
        timeout: float | None = None,
        steps: int | None = None,
        force: bool | None = None,
    ) -> None:
        checked_steps = None if steps is None else _click_steps(steps)
        checked_force = _optional_boolean_argument(force, "Locator.click force")
        if self._native_locator is not None:
            timeout_ms = self._page._effective_timeout(timeout)
            if checked_force is not None:
                await self._native_locator.click(timeout_ms, checked_steps, checked_force)
            elif checked_steps is None:
                await self._native_locator.click(timeout_ms)
            else:
                await self._native_locator.click(timeout_ms, checked_steps)
            return
        if checked_steps is not None:
            raise UnsupportedError("Locator.click steps requires a Rust-owned query")
        await self._page.click(
            self._selector_for("Locator.click"), timeout=timeout, force=checked_force
        )

    async def tap(
        self,
        *,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        if "touchscreen-input" not in _native.selected_features():
            raise UnsupportedError("Locator.tap requires the touchscreen-input profile")
        options = _tap_options(
            "Locator.tap",
            modifiers=modifiers,
            position=position,
            force=force,
            no_wait_after=no_wait_after,
            trial=trial,
        )
        if self._native_locator is not None:
            await self._native_locator.tap(self._page._effective_timeout(timeout), options)
            return
        await self._page.tap(
            self._selector_for("Locator.tap"),
            modifiers=modifiers,
            position=position,
            timeout=timeout,
            force=force,
            no_wait_after=no_wait_after,
            strict=True,
            trial=trial,
        )

    async def dblclick(self, *, timeout: float | None = None) -> None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.dblclick requires a Rust-owned query")
        await self._native_locator.dblclick(self._page._effective_timeout(timeout))

    async def bounding_box(self, *, timeout: float | None = None) -> dict[str, float] | None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.bounding_box requires a Rust-owned query")
        return await self._native_locator.bounding_box(self._page._effective_timeout(timeout))

    async def screenshot(
        self,
        *,
        type: str | None = None,
        path: str | Path | None = None,
        quality: int | None = None,
        scale: str = "device",
        timeout: float | None = None,
        mask: Sequence[Locator] | None = None,
    ) -> bytes:
        if self._native_locator is None:
            raise UnsupportedError("Locator.screenshot requires a Rust-owned query")
        format_value, output_path, quality_value, scale_value = _screenshot_arguments(
            "Locator.screenshot",
            type=type,
            path=path,
            quality=quality,
            scale=scale,
        )
        return await self._native_locator.screenshot(
            format_value,
            output_path,
            quality_value,
            scale_value,
            self._page._effective_timeout(timeout),
            _screenshot_masks(mask, "Locator.screenshot"),
        )

    async def set_input_files(
        self,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        del no_wait_after
        if self._native_locator is None:
            raise UnsupportedError("Locator.set_input_files requires a Rust-owned query")
        paths, payloads = _file_input_values(files)
        timeout_ms = self._page._effective_timeout(timeout)
        if paths is not None:
            await self._native_locator.set_input_files_paths(os.getcwd(), paths, timeout_ms)
            return
        assert payloads is not None
        await self._native_locator.set_input_file_payloads(payloads, timeout_ms)

    async def element_handle(self, *, timeout: float | None = None) -> ElementHandle:
        if self._native_locator is None:
            raise UnsupportedError("Locator.element_handle requires a Rust-owned query")
        native = await self._native_locator.element_handle(self._page._effective_timeout(timeout))
        handle = _wrap_handle(self._page._context.browser._native, native, self._page)
        if not isinstance(handle, ElementHandle):
            await handle.dispose()
            raise Error("Locator.element_handle resolved to a non-element")
        return handle

    async def element_handles(self) -> list[ElementHandle]:
        count = await self.count()
        if count > 64:
            raise ResourceExhaustedError(
                "Locator.element_handles result exceeds the fixed 64-handle boundary"
            )
        return [await self.nth(index).element_handle() for index in range(count)]

    async def highlight(self) -> None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.highlight requires a Rust-owned query")
        await self._native_locator.highlight(self._description or "")

    async def dispatch_event(
        self,
        type: str,
        event_init: Mapping[str, object] | None = None,
        *,
        timeout: float | None = None,
    ) -> None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.dispatch_event requires a Rust-owned query")
        event_type = _string_argument(type, "Locator.dispatch_event")
        init = None if event_init is None else dict(event_init)
        timeout_ms = self._page._effective_timeout(timeout)
        converter = _NativeEvaluationArgumentConverter()
        native_init = converter.convert(init)
        if converter.has_handles:
            await self._native_locator.dispatch_event_owned(
                event_type, cast(dict[str, object], native_init), timeout_ms
            )
        else:
            await self._native_locator.dispatch_event(event_type, init, timeout_ms)

    async def evaluate_handle(
        self,
        expression: str,
        arg: object = None,
        *,
        timeout: float | None = None,
    ) -> JSHandle:
        if self._native_locator is None:
            raise UnsupportedError("Locator.evaluate_handle requires a Rust-owned query")
        native = await self._native_locator.evaluate_handle(
            _string_argument(expression, "Locator.evaluate_handle"),
            arg,
            self._page._effective_timeout(timeout),
        )
        return _wrap_handle(self._page._context.browser._native, native, self._page)

    async def select_text(
        self,
        *,
        force: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.select_text requires a Rust-owned query")
        await self._native_locator.select_text(
            self._page._effective_timeout(timeout),
            force or False,
        )

    async def fill(
        self,
        value: str,
        *,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        if no_wait_after is not None:
            raise UnsupportedError("Locator.fill(no_wait_after=...) is not installed")
        if self._native_locator is not None:
            await self._native_locator.fill(
                _string_argument(value, "Locator.fill"),
                self._page._effective_timeout(timeout),
                force or False,
            )
            return
        await self._page.fill(
            self._selector_for("Locator.fill"),
            value,
            timeout=timeout,
            force=force,
            no_wait_after=no_wait_after,
        )

    async def press(
        self,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        if self._native_locator is not None:
            await self._native_locator.press(
                _string_argument(key, "Locator.press"),
                self._page._effective_timeout(timeout),
                _optional_milliseconds(delay, "delay"),
                no_wait_after or False,
            )
            return
        await self._page.press(
            self._selector_for("Locator.press"),
            key,
            delay=delay,
            timeout=timeout,
            no_wait_after=no_wait_after,
        )

    async def set_checked(
        self,
        checked: bool,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        if position is not None:
            raise UnsupportedError("Locator.set_checked(position=...) is not installed")
        if no_wait_after is not None:
            raise UnsupportedError("Locator.set_checked(no_wait_after=...) is not installed")
        if self._native_locator is not None:
            await self._native_locator.set_checked(
                checked,
                self._page._effective_timeout(timeout),
                force,
                trial,
            )
            return
        await self._page.set_checked(
            self._selector_for("Locator.set_checked"),
            checked,
            force=force,
            position=position,
            timeout=timeout,
            trial=trial,
            no_wait_after=no_wait_after,
        )

    async def check(
        self,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        await self.set_checked(
            True,
            force=force,
            position=position,
            timeout=timeout,
            trial=trial,
            no_wait_after=no_wait_after,
        )

    async def uncheck(
        self,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        await self.set_checked(
            False,
            force=force,
            position=position,
            timeout=timeout,
            trial=trial,
            no_wait_after=no_wait_after,
        )

    async def focus(self, *, timeout: float | None = None) -> None:
        if self._native_locator is not None:
            await self._native_locator.focus(self._page._effective_timeout(timeout))
            return
        await self._page.focus(self._selector_for("Locator.focus"), timeout=timeout)

    async def blur(self, *, timeout: float | None = None) -> None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.blur requires a Rust-owned query")
        await self._native_locator.blur(self._page._effective_timeout(timeout))

    async def hover(
        self,
        *,
        force: bool | None = None,
        modifiers: list[str] | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
    ) -> None:
        if modifiers is not None or position is not None:
            raise UnsupportedError("Locator.hover modifiers and position are not installed")
        if self._native_locator is None:
            raise UnsupportedError("Locator.hover requires a Rust-owned query")
        await self._native_locator.hover(
            self._page._effective_timeout(timeout), force or False, trial or False
        )

    async def clear(
        self,
        *,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        await self.fill(
            "",
            force=force,
            no_wait_after=no_wait_after,
            timeout=timeout,
        )

    async def select_option(
        self,
        value: str,
        *,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> list[str]:
        if no_wait_after is not None:
            raise UnsupportedError("Locator.select_option(no_wait_after=...) is not installed")
        if self._native_locator is None:
            raise UnsupportedError("Locator.select_option requires a Rust-owned query")
        return await self._native_locator.select_option(
            _string_argument(value, "Locator.select_option"),
            self._page._effective_timeout(timeout),
            force,
        )

    async def scroll_into_view_if_needed(self, *, timeout: float | None = None) -> None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.scroll_into_view_if_needed requires a Rust-owned query")
        await self._native_locator.scroll_into_view_if_needed(
            self._page._effective_timeout(timeout)
        )

    async def press_sequentially(
        self,
        text: str,
        *,
        delay: float | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        _ = no_wait_after
        if self._native_locator is None:
            raise UnsupportedError("Locator.press_sequentially requires a Rust-owned query")
        await self._native_locator.press_sequentially(
            _string_argument(text, "Locator.press_sequentially"),
            self._page._effective_timeout(timeout),
            _optional_milliseconds(delay, "delay"),
        )

    async def type(
        self,
        text: str,
        *,
        delay: float | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        await self.press_sequentially(
            text,
            delay=delay,
            no_wait_after=no_wait_after,
            timeout=timeout,
        )

    async def input_value(self, *, timeout: float | None = None) -> str:
        return cast(str, await self._read("inputValue", timeout=timeout))

    async def is_hidden(self, *, timeout: float | None = None) -> bool:
        return cast(bool, await self._read("isHidden", timeout=timeout))

    async def is_visible(self, *, timeout: float | None = None) -> bool:
        return cast(bool, await self._read("isVisible", timeout=timeout))

    async def is_checked(self, *, timeout: float | None = None) -> bool:
        return cast(bool, await self._read("isChecked", timeout=timeout))

    async def is_enabled(self, *, timeout: float | None = None) -> bool:
        return cast(bool, await self._read("isEnabled", timeout=timeout))

    async def is_disabled(self, *, timeout: float | None = None) -> bool:
        return cast(bool, await self._read("isDisabled", timeout=timeout))

    async def is_editable(self, *, timeout: float | None = None) -> bool:
        return cast(bool, await self._read("isEditable", timeout=timeout))

    async def text_content(self, *, timeout: float | None = None) -> str | None:
        return cast("str | None", await self._read("textContent", timeout=timeout))

    async def inner_text(self, *, timeout: float | None = None) -> str:
        return cast(str, await self._read("innerText", timeout=timeout))

    async def inner_html(self, *, timeout: float | None = None) -> str:
        return cast(str, await self._read("innerHTML", timeout=timeout))

    async def get_attribute(self, name: str, *, timeout: float | None = None) -> str | None:
        return cast(
            "str | None",
            await self._read(
                "getAttribute",
                timeout=timeout,
                attribute=_string_argument(name, "Locator.get_attribute"),
            ),
        )

    async def count(self) -> int:
        value = await self._read("count")
        return cast(int, value)

    async def all_inner_texts(self) -> list[str]:
        value = await self._read("allInnerTexts")
        return cast(list[str], value)

    async def all_text_contents(self) -> list[str]:
        value = await self._read("allTextContents")
        return cast(list[str], value)

    async def all(self) -> list[Locator]:
        count = await self.count()
        if count > 1024:
            raise Error("Locator.all: result exceeds the fixed 1024-item boundary")
        return [self.nth(index) for index in range(count)]

    async def wait_for(self, *, timeout: float | None = None, state: str | None = None) -> None:
        if self._native_locator is None:
            raise UnsupportedError("Locator.wait_for requires a Rust-owned query")
        state_value = "visible" if state is None else _string_argument(state, "Locator.wait_for")
        if state_value not in {"attached", "detached", "hidden", "visible"}:
            raise ValueError(
                "Locator.wait_for: state must be attached, detached, hidden, or visible"
            )
        await self._native_locator.wait_for(state_value, self._page._effective_timeout(timeout))

    async def evaluate(
        self, expression: str, arg: object = None, *, timeout: float | None = None
    ) -> object:
        if self._native_locator is not None:
            return await self._native_locator.evaluate(
                expression, arg, self._page._effective_timeout(timeout)
            )
        return await self._page.eval_on_selector(
            self._selector_for("Locator.evaluate"), expression, arg, timeout=timeout
        )

    async def evaluate_all(self, expression: str, arg: object = None) -> object:
        if self._native_locator is not None:
            return await self._native_locator.evaluate_all(expression, arg)
        return await self._page.eval_on_selector_all(
            self._selector_for("Locator.evaluate_all"), expression, arg
        )

    async def _expect_count(
        self, expected: int, timeout_ms: int, is_not: bool
    ) -> dict[str, object]:
        if self._native_locator is not None:
            return await self._native_locator.expect_count(expected, timeout_ms, is_not)
        return await self._page._locator_expect_count(
            self._selector_for("expect(locator).to_have_count"),
            expected,
            timeout_ms,
            is_not,
        )

    async def _expect_visibility(
        self, visible: bool, timeout_ms: int, is_not: bool
    ) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("visibility expectations require a Rust-owned query")
        return await self._native_locator.expect_visibility(visible, timeout_ms, is_not)

    async def _expect_text(
        self,
        expected: str | Pattern[str] | list[str],
        timeout_ms: int,
        is_not: bool,
        *,
        substring: bool = False,
        use_inner_text: bool = False,
        ignore_case: bool | None = None,
    ) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("expect(locator).to_have_text requires a Rust-owned query")
        if isinstance(expected, Pattern):
            source, flags = _text_match(expected, "expect(locator) text")
            return await self._native_locator.expect_text_regex(
                source,
                flags or "",
                ignore_case,
                use_inner_text,
                timeout_ms,
                is_not,
            )
        ignore_case_value = False if ignore_case is None else ignore_case
        if isinstance(expected, str):
            return await self._native_locator.expect_text(
                expected,
                timeout_ms,
                is_not,
                substring,
                True,
                ignore_case_value,
                use_inner_text,
            )
        return await self._native_locator.expect_texts(
            expected,
            timeout_ms,
            is_not,
            substring,
            True,
            ignore_case_value,
            use_inner_text,
        )

    async def _expect_class(
        self,
        expected: tuple[str, str | None] | list[tuple[str, str | None]],
        timeout_ms: int,
        is_not: bool,
    ) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("expect(locator).to_have_class requires a Rust-owned query")
        if isinstance(expected, list):
            return await self._native_locator.expect_class_matchers(expected, timeout_ms, is_not)
        source, flags = expected
        return await self._native_locator.expect_string("class", source, flags, timeout_ms, is_not)

    async def _expect_state(self, state: str, timeout_ms: int, is_not: bool) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("state expectations require a Rust-owned query")
        return await self._native_locator.expect_state(state, timeout_ms, is_not)

    async def _expect_checked(
        self,
        checked: bool | None,
        indeterminate: bool | None,
        timeout_ms: int,
        is_not: bool,
    ) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("checked expectations require a Rust-owned query")
        return await self._native_locator.expect_checked(checked, indeterminate, timeout_ms, is_not)

    async def _expect_property(
        self, name: str, expected: object, timeout_ms: int, is_not: bool
    ) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("property expectations require a Rust-owned query")
        return await self._native_locator.expect_property(name, expected, timeout_ms, is_not)

    async def _expect_attribute(
        self,
        name: str,
        expected: str,
        flags: str | None,
        ignore_case: bool | None,
        timeout_ms: int,
        is_not: bool,
    ) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("expect(locator).to_have_attribute requires a Rust-owned query")
        return await self._native_locator.expect_attribute(
            name,
            expected,
            (flags, ignore_case),
            timeout_ms,
            is_not,
        )

    async def _expect_css(
        self,
        name: str,
        expected: str,
        flags: str | None,
        timeout_ms: int,
        is_not: bool,
    ) -> dict[str, object]:
        if self._native_locator is None:
            raise UnsupportedError("expect(locator).to_have_css requires a Rust-owned query")
        return await self._native_locator.expect_css(name, expected, flags, timeout_ms, is_not)

    async def drag_to(
        self,
        target: Locator,
        *,
        source_position: Mapping[str, float] | None = None,
        target_position: Mapping[str, float] | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        steps: int | None = None,
    ) -> None:
        del no_wait_after
        target_value = _drag_target(target)
        if target_value._page is not self._page:
            raise Error("Locator.drag_to: target must belong to the same Page")
        await self._page._context.browser._native.drag_and_drop(
            self._page._object_id,
            self._page._main_frame_id,
            self._selector_for("Locator.drag_to"),
            target_value._selector_for("Locator.drag_to"),
            self._page._effective_timeout(timeout),
            (
                _drag_position(source_position, "source_position"),
                _drag_position(target_position, "target_position"),
                _drag_steps(steps),
                trial or False,
                True,
                force or False,
            ),
        )


class FrameLocator:
    def __init__(
        self,
        page: Page,
        selector: str | None,
        native_locator: _native.NativeFrameLocator,
    ) -> None:
        self._page = page
        self._selector = selector
        self._native_locator = native_locator

    def __repr__(self) -> str:
        return f"<FrameLocator frame={self._page.main_frame!r} selector={self._selector!r}>"

    def __str__(self) -> str:
        return self.__repr__()

    def locator(
        self,
        selector_or_locator: str | Locator,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        if isinstance(selector_or_locator, Locator):
            if selector_or_locator._native_locator is None:
                raise UnsupportedError("FrameLocator.locator requires a Rust-owned inner query")
            native = self._native_locator.locator_native(selector_or_locator._native_locator)
            selector = selector_or_locator._selector
        else:
            selector = _string_argument(selector_or_locator, "FrameLocator.locator")
            native = self._native_locator.locator(selector)
        result = Locator(self._page, selector, None, native)
        if any(value is not None for value in (has_text, has_not_text, has, has_not)):
            return result.filter(
                has_text=has_text,
                has_not_text=has_not_text,
                has=has,
                has_not=has_not,
            )
        return result

    def frame_locator(self, selector: str) -> FrameLocator:
        value = _string_argument(selector, "FrameLocator.frame_locator")
        return FrameLocator(
            self._page,
            value,
            self._native_locator.frame_locator(value),
        )

    def _semantic(
        self,
        kind: str,
        text: str | Pattern[str],
        exact: bool | None,
        api: str,
    ) -> Locator:
        source, flags = _text_match(text, api)
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.semantic(kind, source, bool(exact), flags),
        )

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return self._semantic("text", text, exact, "FrameLocator.get_by_text")

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return self._semantic("label", text, exact, "FrameLocator.get_by_label")

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return self._semantic("placeholder", text, exact, "FrameLocator.get_by_placeholder")

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return self._semantic("altText", text, exact, "FrameLocator.get_by_alt_text")

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        return self._semantic("title", text, exact, "FrameLocator.get_by_title")

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        return self._semantic("testId", test_id, False, "FrameLocator.get_by_test_id")

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        return Locator(
            self._page,
            None,
            None,
            self._native_locator.role(
                *_role_arguments(
                    role,
                    checked=checked,
                    disabled=disabled,
                    expanded=expanded,
                    include_hidden=include_hidden,
                    level=level,
                    name=name,
                    pressed=pressed,
                    selected=selected,
                    exact=exact,
                )
            ),
        )

    @property
    def owner(self) -> Locator:
        return Locator(
            self._page,
            self._selector,
            None,
            self._native_locator.owner(),
        )

    @property
    def first(self) -> FrameLocator:
        return self.nth(0)

    @property
    def last(self) -> FrameLocator:
        return self.nth(-1)

    def nth(self, index: int) -> FrameLocator:
        value = _int32(index, "FrameLocator.nth")
        return FrameLocator(
            self._page,
            self._selector,
            self._native_locator.nth(value),
        )


class LocatorAssertions:
    def __init__(
        self,
        locator: Locator,
        timeout_ms: int,
        is_not: bool = False,
        message: str | None = None,
    ) -> None:
        self._locator = locator
        self._timeout_ms = timeout_ms
        self._is_not = is_not
        self._message = message

    @property
    def not_to(self) -> LocatorAssertions:
        return LocatorAssertions(
            self._locator,
            self._timeout_ms,
            not self._is_not,
            self._message,
        )

    async def to_have_count(self, count: int, *, timeout: float | None = None) -> None:
        expected = _expected_count(count)
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_count(expected, timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}have count {expected}, received {received}"
            )

    async def to_be_visible(self, *, timeout: float | None = None) -> None:
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_visibility(True, timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}be visible, received {received}"
            )

    async def to_be_hidden(self, *, timeout: float | None = None) -> None:
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_visibility(False, timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}be hidden, received {received}"
            )

    async def to_be_checked(
        self,
        *,
        timeout: float | None = None,
        checked: bool | None = None,
        indeterminate: bool | None = None,
    ) -> None:
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_checked(
            checked, indeterminate, timeout_ms, self._is_not
        )
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}be checked, received {received!r}"
            )

    async def to_be_empty(self, *, timeout: float | None = None) -> None:
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_state("empty", timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}be empty, received {received!r}"
            )

    async def to_have_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        expected_value = _expected_text(expected, "to_have_text")
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_text(
            expected_value,
            timeout_ms,
            self._is_not,
            use_inner_text=bool(use_inner_text),
            ignore_case=ignore_case,
        )
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}have text {expected_value!r}, "
                f"received {received!r}"
            )

    async def to_contain_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        timeout: float | None = None,
        ignore_case: bool | None = None,
    ) -> None:
        expected_value = _expected_text(expected, "to_contain_text")
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_text(
            expected_value,
            timeout_ms,
            self._is_not,
            substring=True,
            use_inner_text=bool(use_inner_text),
            ignore_case=ignore_case,
        )
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}contain text {expected_value!r}, "
                f"received {received!r}"
            )

    async def to_have_value(
        self,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        expected, flags = _text_match(value, "to_have_value value")
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        if self._locator._native_locator is None:
            raise UnsupportedError("expect(locator).to_have_value requires a Rust-owned query")
        result = await self._locator._native_locator.expect_string(
            "value", expected, flags, timeout_ms, self._is_not
        )
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}have value {value!r}, received {received!r}"
            )

    async def to_have_class(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        timeout: float | None = None,
    ) -> None:
        expected_value = _expected_string_matchers(expected, "to_have_class")
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_class(expected_value, timeout_ms, self._is_not)
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}have class {expected!r}, "
                f"received {received!r}"
            )

    async def to_have_attribute(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        name_value = _string_argument(name, "to_have_attribute name")
        source, flags = _text_match(value, "to_have_attribute value")
        ignore_case_value = _optional_boolean_argument(ignore_case, "to_have_attribute ignore_case")
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_attribute(
            name_value,
            source,
            flags,
            ignore_case_value,
            timeout_ms,
            self._is_not,
        )
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}have attribute "
                f"{name_value!r}={value!r}, received {received!r}"
            )

    async def to_have_css(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        name_value = _string_argument(name, "to_have_css name")
        source, flags = _text_match(value, "to_have_css value")
        timeout_ms = self._timeout_ms if timeout is None else _timeout_ms(timeout)
        result = await self._locator._expect_css(
            name_value,
            source,
            flags,
            timeout_ms,
            self._is_not,
        )
        if result.get("timed_out"):
            received = result.get("received")
            prefix = f"{self._message}\n" if self._message else ""
            polarity = "not " if self._is_not else ""
            raise AssertionError(
                f"{prefix}Expected locator to {polarity}have CSS "
                f"{name_value!r}={value!r}, received {received!r}"
            )

    async def to_have_id(
        self,
        id: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        await self.to_have_attribute("id", id, timeout=timeout)

    async def not_to_have_count(self, count: int, *, timeout: float | None = None) -> None:
        await self.not_to.to_have_count(count, timeout=timeout)

    async def not_to_be_visible(self, *, timeout: float | None = None) -> None:
        await self.not_to.to_be_visible(timeout=timeout)

    async def not_to_be_hidden(self, *, timeout: float | None = None) -> None:
        await self.not_to.to_be_hidden(timeout=timeout)

    async def not_to_be_checked(
        self,
        *,
        timeout: float | None = None,
        checked: bool | None = None,
        indeterminate: bool | None = None,
    ) -> None:
        await self.not_to.to_be_checked(
            timeout=timeout,
            checked=checked,
            indeterminate=indeterminate,
        )

    async def not_to_be_empty(self, *, timeout: float | None = None) -> None:
        await self.not_to.to_be_empty(timeout=timeout)

    async def not_to_have_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        await self.not_to.to_have_text(
            expected,
            use_inner_text=use_inner_text,
            ignore_case=ignore_case,
            timeout=timeout,
        )

    async def not_to_contain_text(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        use_inner_text: bool | None = None,
        timeout: float | None = None,
        ignore_case: bool | None = None,
    ) -> None:
        await self.not_to.to_contain_text(
            expected,
            use_inner_text=use_inner_text,
            timeout=timeout,
            ignore_case=ignore_case,
        )

    async def not_to_have_value(
        self,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        await self.not_to.to_have_value(value, timeout=timeout)

    async def not_to_have_class(
        self,
        expected: str | Pattern[str] | Sequence[str | Pattern[str]],
        *,
        timeout: float | None = None,
    ) -> None:
        await self.not_to.to_have_class(expected, timeout=timeout)

    async def not_to_have_attribute(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        ignore_case: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        await self.not_to.to_have_attribute(
            name,
            value,
            ignore_case=ignore_case,
            timeout=timeout,
        )

    async def not_to_have_css(
        self,
        name: str,
        value: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        await self.not_to.to_have_css(name, value, timeout=timeout)

    async def not_to_have_id(
        self,
        id: str | Pattern[str],
        *,
        timeout: float | None = None,
    ) -> None:
        await self.not_to.to_have_id(id, timeout=timeout)


class Expect:
    def __init__(self) -> None:
        self._timeout_ms = 5_000

    def set_options(self, *, timeout: float | None = None) -> None:
        if timeout is not None:
            self._timeout_ms = _timeout_ms(timeout)

    def __call__(self, actual: object, message: str | None = None) -> LocatorAssertions:
        if not isinstance(actual, Locator):
            raise UnsupportedError("expect currently requires a Locator")
        return LocatorAssertions(actual, self._timeout_ms, message=message)


expect = Expect()


class Frame:
    def __init__(
        self,
        page: Page,
        object_id: str,
        parent_id: str | None,
        name: str,
        url: str,
    ) -> None:
        self._page = page
        self._object_id = object_id
        self._parent_id = parent_id
        self._name = name
        self._url = url

    def __repr__(self) -> str:
        return f"<Frame name={self._name} url={self._url!r}>"

    def __str__(self) -> str:
        return self.__repr__()

    @property
    def page(self) -> Page:
        return self._page

    @property
    def name(self) -> str:
        return self._name

    @property
    def url(self) -> str:
        return self._url

    @property
    def parent_frame(self) -> Frame | None:
        if self._parent_id is None:
            return None
        return self._page._frames.get(self._parent_id)

    @property
    def child_frames(self) -> list[Frame]:
        return [frame for frame in self._page.frames if frame._parent_id == self._object_id]

    def locator(
        self,
        selector: str,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        value = _string_argument(selector, "Frame.locator")
        native_locator = self._page._context.browser._native.locator(self._object_id, value)
        locator = Locator(self._page, value, None, native_locator)
        if any(value is not None for value in (has_text, has_not_text, has, has_not)):
            return locator.filter(
                has_text=has_text,
                has_not_text=has_not_text,
                has=has,
                has_not=has_not,
            )
        return locator

    def frame_locator(self, selector: str) -> FrameLocator:
        value = _string_argument(selector, "Frame.frame_locator")
        native = self._page._context.browser._native.frame_locator(self._object_id, value)
        return FrameLocator(self._page, value, native)

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Frame.get_by_text")
        native_locator = self._page._context.browser._native.semantic_locator(
            self._object_id, "text", source, bool(exact), flags
        )
        return Locator(self._page, None, None, native_locator)

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Frame.get_by_label")
        native_locator = self._page._context.browser._native.semantic_locator(
            self._object_id, "label", source, bool(exact), flags
        )
        return Locator(self._page, None, None, native_locator)

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Frame.get_by_placeholder")
        native_locator = self._page._context.browser._native.semantic_locator(
            self._object_id, "placeholder", source, bool(exact), flags
        )
        return Locator(self._page, None, None, native_locator)

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Frame.get_by_alt_text")
        native_locator = self._page._context.browser._native.semantic_locator(
            self._object_id, "altText", source, bool(exact), flags
        )
        return Locator(self._page, None, None, native_locator)

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Frame.get_by_title")
        native_locator = self._page._context.browser._native.semantic_locator(
            self._object_id, "title", source, bool(exact), flags
        )
        return Locator(self._page, None, None, native_locator)

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        source, flags = _text_match(test_id, "Frame.get_by_test_id")
        native_locator = self._page._context.browser._native.semantic_locator(
            self._object_id, "testId", source, False, flags
        )
        return Locator(self._page, None, None, native_locator)

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        native_locator = self._page._context.browser._native.role_locator(
            self._object_id,
            *_role_arguments(
                role,
                checked=checked,
                disabled=disabled,
                expanded=expanded,
                include_hidden=include_hidden,
                level=level,
                name=name,
                pressed=pressed,
                selected=selected,
                exact=exact,
            ),
        )
        return Locator(self._page, None, None, native_locator)

    async def evaluate(self, expression: str, arg: object = None) -> object:
        if isinstance(arg, JSHandle):
            return await self._page._context.browser._native.evaluate_handle_argument(
                self._object_id, expression, arg._native
            )
        return await self._page._context.browser._native.evaluate(
            self._object_id, expression, _native_evaluation_argument(arg)
        )

    async def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        value = await self._page._context.browser._native.frame_evaluate_handle(
            self._object_id, expression, _native_evaluation_argument(arg)
        )
        return _wrap_handle(self._page._context.browser._native, value, self._page)

    async def add_script_tag(
        self,
        *,
        content: str | None = None,
        path: str | Path | None = None,
        type: str | None = None,
        url: str | None = None,
    ) -> ElementHandle:
        url, content = await _tag_source(
            url=url, path=path, content=content, source_url_style="script"
        )
        handle = await self.evaluate_handle(
            _ADD_SCRIPT_TAG_SOURCE,
            {"url": url, "content": content, "type": type},
        )
        element = handle.as_element()
        if element is None:
            await handle.dispose()
            raise Error("Frame.add_script_tag did not create an ElementHandle")
        return element

    async def wait_for_function(
        self,
        expression: str,
        arg: object = None,
        *,
        polling: float | str | None = None,
        timeout: float | None = None,
    ) -> JSHandle:
        value = await self._page._context.browser._native.frame_wait_for_function(
            self._object_id,
            expression,
            arg,
            _polling_ms(polling),
            self._page._effective_timeout(timeout),
        )
        return _wrap_handle(self._page._context.browser._native, value, self._page)

    async def wait_for_load_state(
        self,
        state: str | None = None,
        *,
        timeout: float | None = None,
    ) -> None:
        await self._page._context.browser._native.wait_for_load_state(
            self._page._object_id,
            self._object_id,
            state or "load",
            self._page._effective_timeout(timeout, navigation=True),
        )

    async def goto(
        self,
        url: str,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
        referer: str | None = None,
    ) -> Response | None:
        self._page._context.browser._raise_deferred_route_error()
        result = await self._page._context.browser._native.frame_goto(
            self._page._object_id,
            self._object_id,
            url,
            wait_until or "load",
            referer,
            self._page._effective_timeout(timeout, navigation=True),
        )
        await self._page._context.browser._flush_network_notices()
        await self._page._context._drain_cdp_sessions(self._page._object_id)
        self._page._refresh_snapshot()
        self._page._refresh_frames()
        if result is None:
            return None
        return await self._page._frame_navigation_response(result, self)

    def expect_navigation(
        self,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        wait_until: str | None = None,
        timeout: float | None = None,
    ) -> NavigationContextManager:
        if url is not None:
            raise UnsupportedError("Frame.expect_navigation(url=...) is not installed")
        return NavigationContextManager(
            self._page,
            wait_until or "load",
            self._page._effective_timeout(timeout, navigation=True),
            frame=self,
        )

    async def query_selector(
        self,
        selector: str,
        *,
        strict: bool | None = None,
    ) -> ElementHandle | None:
        locator = self.locator(_string_argument(selector, "Frame.query_selector"))
        count = await locator.count()
        if count == 0:
            return None
        if not self._page._selector_is_strict(strict, "Frame.query_selector"):
            locator = locator.first
        return await locator.element_handle()

    async def eval_on_selector(
        self,
        selector: str,
        expression: str,
        arg: object = None,
        *,
        strict: bool | None = None,
    ) -> object:
        selector = _string_argument(selector, "Frame.eval_on_selector")
        if self._page._selector_is_strict(strict, "Frame.eval_on_selector"):
            count = await self.locator(selector).count()
            if count != 1:
                raise Error(
                    "Frame.eval_on_selector: strict mode violation: "
                    f"selector resolved to {count} elements"
                )
        value = await self._page._context.browser._native.eval_on_selector(
            self._object_id,
            selector,
            _string_argument(expression, "Frame.eval_on_selector"),
            arg,
            self._page._effective_timeout(None),
        )
        self._page._refresh_snapshot()
        return value

    async def eval_on_selector_all(
        self,
        selector: str,
        expression: str,
        arg: object = None,
    ) -> object:
        value = await self._page._context.browser._native.eval_on_selector_all(
            self._object_id,
            _string_argument(selector, "Frame.eval_on_selector_all"),
            _string_argument(expression, "Frame.eval_on_selector_all"),
            arg,
        )
        self._page._refresh_snapshot()
        return value

    async def focus(
        self,
        selector: str,
        *,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        value = _string_argument(selector, "Frame.focus")
        if self._page._selector_is_strict(strict, "Frame.focus"):
            await self._page._context.browser._native.focus(
                self._object_id, value, self._page._effective_timeout(timeout)
            )
            return
        await self.locator(value).first.focus(timeout=timeout)

    async def press(
        self,
        selector: str,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        await self._page._context.browser._native.press(
            self._object_id,
            selector,
            key,
            self._page._effective_timeout(timeout),
            (_optional_milliseconds(delay, "delay"), no_wait_after or False),
        )

    async def type(
        self,
        selector: str,
        text: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
    ) -> None:
        locator = self.locator(_string_argument(selector, "Frame.type"))
        if not self._page._selector_is_strict(strict, "Frame.type"):
            locator = locator.first
        await locator.press_sequentially(
            _string_argument(text, "Frame.type"),
            delay=delay,
            no_wait_after=no_wait_after,
            timeout=timeout,
        )


class Dialog:
    def __init__(
        self,
        page: Page,
        native: _native.NativeDialog,
        dialog_type: str,
        message: str,
        default_value: str,
    ) -> None:
        self._page = page
        self._native = native
        self._type = dialog_type
        self._message = message
        self._default_value = default_value

    @property
    def page(self) -> Page:
        return self._page

    @property
    def type(self) -> str:
        return self._type

    @property
    def message(self) -> str:
        return self._message

    @property
    def default_value(self) -> str:
        return self._default_value

    async def accept(self, prompt_text: str | None = None) -> None:
        await self._page._context.browser._native.handle_dialog(self._native, True, prompt_text)

    async def dismiss(self) -> None:
        await self._page._context.browser._native.handle_dialog(self._native, False, None)

    async def _auto_handle(self) -> None:
        try:
            if self._type == "beforeunload":
                await self.accept()
            else:
                await self.dismiss()
        except BaseException:
            # The Page or browser may have disappeared before the queued
            # default action reaches Chromium.
            return

    def __repr__(self) -> str:
        return (
            f"<Dialog type={self.type} message={self.message} default_value={self.default_value}>"
        )


class WebError:
    def __init__(self, page: Page | None, error: Error) -> None:
        self._page = page
        self._error = error

    @property
    def page(self) -> Page | None:
        return self._page

    @property
    def error(self) -> Error:
        return self._error


class FileChooser:
    def __init__(
        self,
        page: Page,
        native: _native.NativeFileChooser,
        is_multiple: bool,
    ) -> None:
        self._page = page
        self._native = native
        self._is_multiple = is_multiple
        self._element = ElementHandle(
            page._context.browser._native,
            cast(_native.NativeHandle, native),
            page,
        )

    @property
    def page(self) -> Page:
        return self._page

    def is_multiple(self) -> bool:
        return self._is_multiple

    @property
    def element(self) -> ElementHandle:
        return self._element

    async def set_files(
        self,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        del timeout, no_wait_after
        self._page._retain_file_chooser_interception()
        try:
            await self._page._flush_file_chooser_interception()
            paths, payloads = _file_input_values(files)
            native_session = self._page._context.browser._native
            if paths is not None:
                await native_session.set_file_chooser_paths(self._native, os.getcwd(), paths)
                return
            assert payloads is not None
            await native_session.set_file_chooser_payloads(self._native, payloads)
        finally:
            self._page._release_file_chooser_interception()

    def __repr__(self) -> str:
        return f"<FileChooser page={self.page} element={self.element}>"


class Keyboard:
    def __init__(self, page: Page) -> None:
        self._page = page

    async def down(self, key: str) -> None:
        try:
            await self._page._context.browser._native.keyboard(
                self._page._object_id, "down", key, 0.0
            )
        except Error as error:
            raise rewrite_api_error(error, "Keyboard.down") from None

    async def up(self, key: str) -> None:
        try:
            await self._page._context.browser._native.keyboard(
                self._page._object_id, "up", key, 0.0
            )
        except Error as error:
            raise rewrite_api_error(error, "Keyboard.up") from None

    async def insert_text(self, text: str) -> None:
        try:
            await self._page._context.browser._native.keyboard(
                self._page._object_id, "insert_text", text, 0.0
            )
        except Error as error:
            raise rewrite_api_error(error, "Keyboard.insert_text") from None

    async def type(self, text: str, *, delay: float | None = None) -> None:
        delay_ms = _optional_fractional_milliseconds(delay, "delay")
        try:
            await self._page._context.browser._native.keyboard(
                self._page._object_id,
                "type",
                text,
                0.0 if delay_ms is None else delay_ms,
            )
        except Error as error:
            raise rewrite_api_error(error, "Keyboard.type") from None

    async def press(self, key: str, *, delay: float | None = None) -> None:
        delay_ms = _optional_fractional_milliseconds(delay, "delay")
        try:
            await self._page._context.browser._native.keyboard(
                self._page._object_id,
                "press",
                key,
                0.0 if delay_ms is None else delay_ms,
            )
        except Error as error:
            raise rewrite_api_error(error, "Keyboard.press") from None


class Touchscreen:
    def __init__(self, page: Page) -> None:
        self._page = page

    async def tap(self, x: float, y: float) -> None:
        if "touchscreen-input" not in _native.selected_features():
            raise UnsupportedError("Touchscreen.tap requires the touchscreen-input profile")
        try:
            await self._page._context.browser._native.touchscreen_tap(
                self._page._object_id,
                _mouse_number(x, "Touchscreen.tap x"),
                _mouse_number(y, "Touchscreen.tap y"),
            )
        except Error as error:
            raise rewrite_api_error(error, "Touchscreen.tap") from None


class Mouse:
    def __init__(self, page: Page) -> None:
        self._page = page

    async def move(self, x: float, y: float, *, steps: int | None = None) -> None:
        await self._page._context.browser._native.mouse(
            self._page._object_id,
            "move",
            _mouse_number(x, "x"),
            _mouse_number(y, "y"),
            _mouse_options(steps=1 if steps is None else steps),
        )

    async def down(
        self,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
    ) -> None:
        await self._page._context.browser._native.mouse(
            self._page._object_id,
            "down",
            0.0,
            0.0,
            _mouse_options(
                button="left" if button is None else button,
                click_count=1 if click_count is None else click_count,
            ),
        )

    async def up(
        self,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
    ) -> None:
        await self._page._context.browser._native.mouse(
            self._page._object_id,
            "up",
            0.0,
            0.0,
            _mouse_options(
                button="left" if button is None else button,
                click_count=1 if click_count is None else click_count,
            ),
        )

    async def click(
        self,
        x: float,
        y: float,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
        delay: float | None = None,
    ) -> None:
        await self._page._context.browser._native.mouse(
            self._page._object_id,
            "click",
            _mouse_number(x, "x"),
            _mouse_number(y, "y"),
            _mouse_options(
                button="left" if button is None else button,
                click_count=1 if click_count is None else click_count,
                delay=delay,
            ),
        )

    async def dblclick(
        self,
        x: float,
        y: float,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        delay: float | None = None,
    ) -> None:
        await self._page._context.browser._native.mouse(
            self._page._object_id,
            "dblclick",
            _mouse_number(x, "x"),
            _mouse_number(y, "y"),
            _mouse_options(button="left" if button is None else button, delay=delay),
        )

    async def wheel(self, delta_x: float, delta_y: float) -> None:
        await self._page._context.browser._native.mouse(
            self._page._object_id,
            "wheel",
            _mouse_number(delta_x, "delta_x"),
            _mouse_number(delta_y, "delta_y"),
            _mouse_options(),
        )


class Download:
    def __init__(
        self,
        page: Page,
        url: str,
        suggested_filename: str,
        native: _native.NativeDownload,
    ) -> None:
        self._page = page
        self._url = url
        self._suggested_filename = suggested_filename
        self._native = native

    @property
    def page(self) -> Page:
        return self._page

    @property
    def url(self) -> str:
        return self._url

    @property
    def suggested_filename(self) -> str:
        return self._suggested_filename

    async def path(self) -> Path:
        value = await self._page._context.browser._native.download_path(self._native)
        return Path(value)

    def __repr__(self) -> str:
        return f"<Download url={self.url!r} suggested_filename={self.suggested_filename!r}>"


class SourceLocation(TypedDict):
    url: str
    lineNumber: int
    columnNumber: int


class ConsoleMessage:
    def __init__(
        self,
        page: Page,
        message_type: str,
        text: str,
        arguments_json: str,
        location_url: str,
        line_number: int,
        column_number: int,
        argument_handles: list[_native.NativeHandle] | None = None,
        timestamp_micros: int = 0,
        worker: Worker | None = None,
    ) -> None:
        self._page = page
        self._worker = worker
        self._type = message_type
        self._text = text
        self._arguments_json = arguments_json
        self._args = [
            _wrap_handle(page._context.browser._native, handle, page)
            for handle in (argument_handles or [])
        ]
        self._location = SourceLocation(
            url=location_url,
            lineNumber=line_number,
            columnNumber=column_number,
        )
        self._timestamp_micros = timestamp_micros

    @property
    def page(self) -> Page:
        return self._page

    @property
    def worker(self) -> Worker | None:
        return self._worker

    @property
    def type(self) -> str:
        return self._type

    @property
    def text(self) -> str:
        return self._text

    @property
    def args(self) -> list[JSHandle]:
        return list(self._args)

    @property
    def location(self) -> SourceLocation:
        return self._location.copy()

    def __str__(self) -> str:
        return self._text

    def __repr__(self) -> str:
        return f"<ConsoleMessage type={self.type} text={self.text}>"


class Worker:
    def __init__(
        self,
        page: Page,
        worker_id: str,
        url: str,
        native: _native.NativeWorker,
    ) -> None:
        self._page = page
        self._worker_id = worker_id
        self._url = url
        self._native: _native.NativeWorker | None = native
        self._closed = False
        self._event_handlers: dict[str, list[Callable[[object], object]]] = {}
        self._event_waiters: dict[
            str, list[tuple[asyncio.Future[object], WorkerEventPredicate | None]]
        ] = {}
        self._close_emitted = False

    @property
    def url(self) -> str:
        return self._url

    async def evaluate(self, expression: str, arg: object = None) -> object:
        native = self._native
        if self._closed or native is None:
            raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
        return await self._page._context.browser._native.worker_evaluate(native, expression, arg)

    async def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        native = self._native
        if self._closed or native is None:
            raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
        value = await self._page._context.browser._native.worker_evaluate_handle(
            native, expression, arg
        )
        return _wrap_handle(self._page._context.browser._native, value, self._page)

    def _arm_worker_event(
        self, event: str, predicate: WorkerEventPredicate | None
    ) -> asyncio.Future[object]:
        if sum(len(waiters) for waiters in self._event_waiters.values()) >= (
            _MAX_WORKER_EVENT_WAITERS
        ):
            raise ResourceExhaustedError(
                f"Worker event waiter capacity exceeded ({_MAX_WORKER_EVENT_WAITERS})"
            )
        waiter = asyncio.get_running_loop().create_future()
        self._event_waiters.setdefault(event, []).append((waiter, predicate))
        return waiter

    def _remove_worker_event_waiter(self, event: str, waiter: asyncio.Future[object]) -> None:
        values = self._event_waiters.get(event)
        if values is None:
            return
        self._event_waiters[event] = [value for value in values if value[0] is not waiter]
        if not self._event_waiters[event]:
            self._event_waiters.pop(event, None)

    async def _match_worker_event(
        self,
        event: str,
        waiter: asyncio.Future[object],
        predicate: WorkerEventPredicate,
        value: object,
    ) -> None:
        if waiter.done():
            return
        try:
            matches = predicate(value)
            if inspect.isawaitable(matches):
                matches = await cast(Awaitable[object], matches)
            if bool(matches) and not waiter.done():
                waiter.set_result(value)
                self._remove_worker_event_waiter(event, waiter)
        except BaseException as error:
            if not waiter.done():
                waiter.set_exception(error)
                self._remove_worker_event_waiter(event, waiter)

    def _emit_worker_event(self, event: str, value: object) -> None:
        if event == "close":
            if self._close_emitted:
                return
            self._close_emitted = True
            for pending_event, pending_waiters in list(self._event_waiters.items()):
                if pending_event == "close":
                    continue
                for waiter, _predicate in pending_waiters:
                    if not waiter.done():
                        waiter.set_exception(TargetClosedError(_TARGET_CLOSED_MESSAGE))
                self._event_waiters.pop(pending_event, None)
        for waiter, predicate in list(self._event_waiters.get(event, [])):
            if waiter.done():
                self._remove_worker_event_waiter(event, waiter)
            elif predicate is None:
                waiter.set_result(value)
                self._remove_worker_event_waiter(event, waiter)
            else:
                asyncio.create_task(self._match_worker_event(event, waiter, predicate, value))
        loop = asyncio.get_running_loop()
        for handler in list(self._event_handlers.get(event, [])):
            try:
                outcome = handler(value)
                if inspect.isawaitable(outcome):
                    asyncio.ensure_future(cast(Awaitable[object], outcome))
            except BaseException as error:
                loop.call_exception_handler(
                    {"message": f"Worker {event!r} event handler failed", "exception": error}
                )

    def _close(self) -> None:
        if self._closed and self._native is None:
            return
        self._closed = True
        self._native = None
        self._emit_worker_event("close", self)

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        installed = {"close"}
        if "worker-console-events" in _native.selected_features():
            installed.add("console")
        if event not in installed:
            raise UnsupportedError(f"Worker event {event!r} is not installed")
        self._event_handlers.setdefault(event, []).append(handler)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self.remove_listener(event, invoke)
            return handler(value)

        self.on(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        handlers = self._event_handlers.get(event)
        if handlers is None:
            return
        self._event_handlers[event] = [
            registered for registered in handlers if registered != handler
        ]
        if not self._event_handlers[event]:
            self._event_handlers.pop(event, None)

    def expect_event(
        self,
        event: str,
        predicate: WorkerEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> WorkerEventContextManager:
        installed = {"close"}
        if "worker-console-events" in _native.selected_features():
            installed.add("console")
        if event not in installed:
            raise UnsupportedError(f"Worker event {event!r} is not installed")
        if predicate is not None and not callable(predicate):
            raise TypeError("Worker event predicate must be callable")
        return WorkerEventContextManager(
            self,
            event,
            predicate,
            self._page._effective_timeout(timeout),
        )

    async def wait_for_event(
        self,
        event: str,
        predicate: WorkerEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> object:
        event_info = self.expect_event(event, predicate, timeout=timeout)
        await event_info.__aenter__()
        try:
            value = await event_info.value
        except BaseException as error:
            await event_info.__aexit__(type(error), error, error.__traceback__)
            raise
        await event_info.__aexit__(None, None, None)
        return value

    def __repr__(self) -> str:
        return f"<Worker url={self.url!r}>"


class Page:
    _MAX_ROUTE_REQUEST_SNAPSHOTS = 64

    def __init__(
        self,
        context: BrowserContext,
        object_id: str,
        main_frame_id: str,
        url: str,
    ) -> None:
        self._context = context
        self._object_id = object_id
        self._main_frame_id = main_frame_id
        self._url = url
        self._viewport_size = getattr(context, "_viewport_size", None)
        self._closed = False
        self._closes_context = False
        self._frames: dict[str, Frame] = {main_frame_id: Frame(self, main_frame_id, None, "", url)}
        self._bindings: dict[str, tuple[Callable[..., object], bool]] = {}
        self._routes: dict[str, Callable[..., object]] = {}
        self._route_patterns: dict[str, RouteURLMatcher] = {}
        self._route_handler_keys: dict[str, object] = {}
        self._workers: dict[str, Worker] = {}
        self._requests: weakref.WeakValueDictionary[str, Request] = weakref.WeakValueDictionary()
        self._responses: weakref.WeakValueDictionary[str, Response] = weakref.WeakValueDictionary()
        self._route_request_snapshots: dict[str, dict[str, Any]] = {}
        self._retired_worker_ids: set[str] = set()
        self._opener: Page | None = None
        self._event_waiters: dict[
            str,
            list[tuple[asyncio.Future[object], PageEventPredicate | None, int]],
        ] = {}
        self._event_handlers: dict[str, list[Callable[[object], object]]] = {}
        self._console_subscription_token: int | None = None
        self._frame_attaches_this_turn: set[str] | None = (
            set() if "frame-events" in _native.selected_features() else None
        )
        self._frame_attach_clear_scheduled = False
        self._file_chooser_interception_count = 0
        self._file_chooser_interception_enabled = False
        self._file_chooser_interception_task: asyncio.Task[None] | None = None
        self._file_chooser_waiters: set[asyncio.Future[object]] = set()
        self._close_emitted = False
        self._keyboard = Keyboard(self)
        self._mouse = Mouse(self)
        self._touchscreen = Touchscreen(self)

    def _queue_file_chooser_interception(self, enabled: bool) -> None:
        previous = self._file_chooser_interception_task

        async def update() -> None:
            if previous is not None:
                await previous
            if not enabled:
                # Event delivery can consume the last once-listener/waiter
                # immediately before scheduling its async FileChooser.set_files
                # callback. Give that callback one turn to acquire its mutation
                # lease, then discard this stale disable when a new lease exists.
                await asyncio.sleep(0)
                if self._file_chooser_interception_count != 0:
                    return
            if self._file_chooser_interception_enabled == enabled:
                return
            await self._context.browser._native.set_page_file_chooser_interception(
                self._object_id, enabled
            )
            self._file_chooser_interception_enabled = enabled

        task = asyncio.create_task(update())
        self._file_chooser_interception_task = task

        def report_error(completed: asyncio.Task[None]) -> None:
            if completed.cancelled():
                return
            try:
                completed.result()
            except TargetClosedError:
                return
            except BaseException as error:
                asyncio.get_running_loop().call_exception_handler(
                    {
                        "message": "Page file chooser interception update failed",
                        "exception": error,
                    }
                )

        task.add_done_callback(report_error)

    async def _flush_file_chooser_interception(self) -> None:
        task = self._file_chooser_interception_task
        if task is not None:
            await asyncio.shield(task)

    def _retain_file_chooser_interception(self) -> None:
        self._file_chooser_interception_count += 1
        if self._file_chooser_interception_count == 1:
            self._queue_file_chooser_interception(True)

    def _release_file_chooser_interception(self) -> None:
        if self._file_chooser_interception_count == 0:
            return
        self._file_chooser_interception_count -= 1
        if self._file_chooser_interception_count == 0:
            self._queue_file_chooser_interception(False)

    def _wrap_request(self, value: Mapping[str, Any]) -> Request:
        request_id = str(value["id"])
        request = self._requests.get(request_id)
        if request is None:
            request = Request(value, self._context.browser._native, self._context)
            self._requests[request_id] = request
        else:
            request._merge(value, self._context.browser._native, self._context)
        route_snapshot = self._route_request_snapshots.get(request_id)
        if route_snapshot is not None:
            request._route_snapshot = {
                **route_snapshot,
                "headers": dict(route_snapshot["headers"]),
            }
            request._value.update(request._route_snapshot)
        redirected_from_id = value.get("redirected_from_id")
        if redirected_from_id is not None:
            redirected_from = self._requests.get(str(redirected_from_id))
            if redirected_from is not None:
                request._redirected_from = redirected_from
                redirected_from._redirected_to = request
        return request

    def _remember_route_request_snapshot(self, request: Request) -> None:
        request_id = request._value.get("id")
        snapshot = request._route_snapshot
        if request_id is None or snapshot is None:
            return
        key = str(request_id)
        self._route_request_snapshots.pop(key, None)
        if len(self._route_request_snapshots) == self._MAX_ROUTE_REQUEST_SNAPSHOTS:
            self._route_request_snapshots.pop(next(iter(self._route_request_snapshots)))
        self._route_request_snapshots[key] = {
            **snapshot,
            "headers": dict(snapshot["headers"]),
        }

    def _forget_route_request_snapshot(self, value: Mapping[str, Any]) -> None:
        request_id = value.get("id")
        if request_id is not None:
            self._route_request_snapshots.pop(str(request_id), None)

    def _sync_request_history(self) -> list[Request]:
        if "request-history" not in _native.selected_features():
            return []
        return [
            self._wrap_request(value)
            for value in self._context.browser._native.page_requests(self._object_id)
        ]

    def _wrap_response(
        self,
        value: Mapping[str, Any],
        *,
        request: Request | None = None,
    ) -> Response:
        raw_request = value.get("request")
        if request is None and isinstance(raw_request, Mapping):
            request = self._wrap_request(cast(Mapping[str, Any], raw_request))
        if request is None:
            request = self._requests.get(str(value.get("request_id", "")))
        response_id = str(value["id"])
        response = self._responses.get(response_id)
        if response is None:
            response = Response(
                value,
                self._context.browser._native,
                self._context,
                request=request,
            )
            self._responses[response_id] = response
        else:
            response._merge(value, request)
        return response

    async def __aenter__(self) -> Page:
        return self

    async def __aexit__(self, *_exc: object) -> None:
        await self.close()

    def __repr__(self) -> str:
        return f"<Page url={self.url!r}>"

    @property
    def context(self) -> BrowserContext:
        return self._context

    @property
    def request(self) -> APIRequestContext:
        return self._context.request

    @property
    def keyboard(self) -> Keyboard:
        return self._keyboard

    @property
    def mouse(self) -> Mouse:
        return self._mouse

    @property
    def touchscreen(self) -> Touchscreen:
        return self._touchscreen

    @property
    def url(self) -> str:
        return self._url

    @property
    def workers(self) -> list[Worker]:
        if "worker-events" not in _native.selected_features():
            return []
        live_ids: set[str] = set()
        for worker_id, url, native in self._context.browser._native.page_workers(self._object_id):
            live_ids.add(worker_id)
            worker = self._workers.get(worker_id)
            if worker is None:
                self._workers[worker_id] = Worker(self, worker_id, url, native)
                self._retired_worker_ids.discard(worker_id)
            else:
                worker._url = url
        for worker_id in set(self._workers) - live_ids:
            self._retired_worker_ids.add(worker_id)
            self._workers.pop(worker_id)._close()
        return list(self._workers.values())

    def _retire_workers(self) -> None:
        workers = list(self._workers.values())
        self._retired_worker_ids.update(self._workers)
        self._workers.clear()
        for worker in workers:
            worker._close()

    def is_closed(self) -> bool:
        return self._closed or not self._context.browser.is_connected()

    def _arm_page_event(
        self,
        event: str,
        predicate: PageEventPredicate | None,
        not_before_micros: int = 0,
    ) -> asyncio.Future[object]:
        if sum(len(waiters) for waiters in self._event_waiters.values()) >= (
            _MAX_PAGE_EVENT_WAITERS
        ):
            raise ResourceExhaustedError(
                f"Page event waiter capacity exceeded ({_MAX_PAGE_EVENT_WAITERS})"
            )
        waiter = asyncio.get_running_loop().create_future()
        self._event_waiters.setdefault(event, []).append((waiter, predicate, not_before_micros))
        if event == "filechooser":
            self._retain_file_chooser_interception()
            self._file_chooser_waiters.add(waiter)
        return waiter

    def _remove_page_event_waiter(self, event: str, waiter: asyncio.Future[object]) -> None:
        if event == "filechooser" and waiter in self._file_chooser_waiters:
            self._file_chooser_waiters.remove(waiter)
            self._release_file_chooser_interception()
        values = self._event_waiters.get(event)
        if values is None:
            return
        self._event_waiters[event] = [value for value in values if value[0] is not waiter]
        if not self._event_waiters[event]:
            self._event_waiters.pop(event, None)

    async def _match_page_event(
        self,
        event: str,
        waiter: asyncio.Future[object],
        predicate: PageEventPredicate,
        value: object,
    ) -> None:
        if waiter.done():
            return
        try:
            matches = predicate(value)
            if inspect.isawaitable(matches):
                matches = await cast(Awaitable[object], matches)
            if bool(matches) and not waiter.done():
                waiter.set_result(value)
                self._remove_page_event_waiter(event, waiter)
        except BaseException as error:
            if not waiter.done():
                waiter.set_exception(error)
                self._remove_page_event_waiter(event, waiter)

    def _emit_page_event(self, event: str, value: object) -> None:
        if event == "close":
            if self._close_emitted:
                return
            self._close_emitted = True
            if self._frame_attaches_this_turn is not None:
                self._frame_attaches_this_turn.clear()
            self._frame_attach_clear_scheduled = False
            for pending_event, pending_waiters in list(self._event_waiters.items()):
                if pending_event == "close":
                    continue
                for waiter, _predicate, _not_before_micros in pending_waiters:
                    if not waiter.done():
                        waiter.set_exception(TargetClosedError(_TARGET_CLOSED_MESSAGE))
                self._event_waiters.pop(pending_event, None)
        for waiter, predicate, not_before_micros in list(self._event_waiters.get(event, [])):
            if waiter.done():
                self._remove_page_event_waiter(event, waiter)
            elif (
                event == "console"
                and isinstance(value, ConsoleMessage)
                and value._timestamp_micros < not_before_micros
            ):
                continue
            elif predicate is None:
                waiter.set_result(value)
                self._remove_page_event_waiter(event, waiter)
            else:
                asyncio.create_task(self._match_page_event(event, waiter, predicate, value))
        loop = asyncio.get_running_loop()
        for handler in list(self._event_handlers.get(event, [])):
            try:
                outcome = handler(value)
                if inspect.isawaitable(outcome):
                    asyncio.ensure_future(cast(Awaitable[object], outcome))
            except BaseException as error:
                loop.call_exception_handler(
                    {"message": f"Page {event!r} event handler failed", "exception": error}
                )

    def _mark_frame_attached_this_turn(self, frame_id: str) -> None:
        markers = self._frame_attaches_this_turn
        if markers is None:
            raise UnsupportedError("Frame events capability is not installed")
        if frame_id not in markers and len(markers) >= 128:
            raise ResourceExhaustedError("Frame attach delivery marker capacity exceeded (128)")
        markers.add(frame_id)
        if not self._frame_attach_clear_scheduled:
            self._frame_attach_clear_scheduled = True
            asyncio.get_running_loop().call_soon(self._clear_frame_attach_markers)

    def _clear_frame_attach_markers(self) -> None:
        if self._frame_attaches_this_turn is not None:
            self._frame_attaches_this_turn.clear()
        self._frame_attach_clear_scheduled = False

    def _emit_deferred_frame_navigation(self, frame: Frame) -> None:
        if self._frame_attaches_this_turn is not None:
            self._frame_attaches_this_turn.discard(frame._object_id)
        if not self._closed and self._frames.get(frame._object_id) is frame:
            self._emit_page_event("framenavigated", frame)

    def _has_page_event_subscribers(self, event: str) -> bool:
        return any(
            not waiter.done()
            for waiter, _predicate, _not_before_micros in self._event_waiters.get(event, [])
        ) or bool(self._event_handlers.get(event))

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        installed = {"load", "domcontentloaded", "close", "popup", "pageerror", "dialog"}
        if "frame-events" in _native.selected_features():
            installed.update({"frameattached", "framenavigated"})
        if "worker-events" in _native.selected_features():
            installed.add("worker")
        if "console-events" in _native.selected_features():
            installed.add("console")
        if "file-chooser-events" in _native.selected_features():
            installed.add("filechooser")
        if "request-events" in _native.selected_features():
            installed.update({"request", "requestfinished", "requestfailed"})
        if "response-events" in _native.selected_features():
            installed.add("response")
        if "download-events" in _native.selected_features():
            installed.add("download")
        if event not in installed:
            raise UnsupportedError(f"Page event {event!r} is not installed")
        if event == "console" and not self._event_handlers.get(event):
            self._context.browser._native.set_page_console_listener(self._object_id, True)
        if event == "filechooser":
            self._retain_file_chooser_interception()
        self._event_handlers.setdefault(event, []).append(handler)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self.remove_listener(event, invoke)
            return handler(value)

        self.on(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        handlers = self._event_handlers.get(event)
        if handlers is None:
            return
        retained = [registered for registered in handlers if registered != handler]
        removed = len(handlers) - len(retained)
        self._event_handlers[event] = retained
        if event == "filechooser":
            for _ in range(removed):
                self._release_file_chooser_interception()
        if not self._event_handlers[event]:
            self._event_handlers.pop(event, None)
            if event == "console":
                with contextlib.suppress(Error):
                    self._context.browser._native.set_page_console_listener(self._object_id, False)

    async def opener(self) -> Page | None:
        opener = self._opener
        if opener is None or opener.is_closed():
            return None
        return opener

    def _effective_timeout(self, value: object, *, navigation: bool = False) -> int:
        if value is not None:
            return _timeout_ms(value)
        return int(self._context.browser._native.page_default_timeout(self._object_id, navigation))

    def set_default_timeout(self, timeout: float) -> None:
        self._context.browser._native.set_page_default_timeout(
            self._object_id, False, _timeout_ms(timeout)
        )

    def set_default_navigation_timeout(self, timeout: float) -> None:
        self._context.browser._native.set_page_default_timeout(
            self._object_id, True, _timeout_ms(timeout)
        )

    async def pause(self) -> None:
        await self._context.browser._native.page_pause(self._object_id)

    async def wait_for_timeout(self, timeout: float) -> None:
        await self._context.browser._native.wait_for_timeout(_timeout_ms(timeout))

    async def wait_for_selector(
        self,
        selector: str,
        *,
        state: str | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> ElementHandle | None:
        state_value = (
            "visible" if state is None else _string_argument(state, "Page.wait_for_selector state")
        )
        if state_value not in {"attached", "detached", "hidden", "visible"}:
            raise ValueError(
                "Page.wait_for_selector: state must be attached, detached, hidden, or visible"
            )
        locator = self.locator(_string_argument(selector, "Page.wait_for_selector"))
        if _optional_boolean_argument(strict, "Page.wait_for_selector strict") is not True:
            locator = locator.first
        native_locator = locator._native_locator
        assert native_locator is not None
        native = await native_locator.wait_for_element(
            state_value, self._effective_timeout(timeout)
        )
        if native is None:
            return None
        handle = _wrap_handle(self._context.browser._native, native, self)
        if not isinstance(handle, ElementHandle):
            await handle.dispose()
            raise Error("Page.wait_for_selector resolved to a non-element")
        return handle

    async def request_gc(self) -> None:
        await self._context.browser._native.page_request_gc(self._object_id)

    async def pdf(
        self,
        *,
        path: str | Path | None = None,
        tagged: bool | None = None,
        outline: bool | None = None,
        **kwargs: object,
    ) -> bytes:
        if kwargs:
            options = ", ".join(sorted(kwargs))
            raise UnsupportedError(f"Page.pdf options are not installed: {options}")
        output_path = None if path is None else os.fspath(path)
        checked_tagged = _optional_boolean_argument(tagged, "Page.pdf tagged")
        checked_outline = _optional_boolean_argument(outline, "Page.pdf outline")
        return await self._context.browser._native.page_pdf(
            self._object_id,
            output_path,
            checked_tagged is True,
            checked_outline is True,
        )

    async def screenshot(
        self,
        *,
        timeout: float | None = None,
        type: str | None = None,
        path: str | Path | None = None,
        quality: int | None = None,
        full_page: bool | None = None,
        scale: str = "device",
        mask: Sequence[Locator] | None = None,
        **kwargs: object,
    ) -> bytes:
        if kwargs:
            options = ", ".join(sorted(kwargs))
            raise UnsupportedError(f"Page.screenshot options are not installed: {options}")
        format_value, output_path, quality_value, scale_value = _screenshot_arguments(
            "Page.screenshot",
            type=type,
            path=path,
            quality=quality,
            scale=scale,
        )
        checked_full_page = _optional_boolean_argument(full_page, "Page.screenshot full_page")
        return await self._context.browser._native.page_screenshot(
            self._object_id,
            format_value,
            output_path,
            quality_value,
            checked_full_page is True,
            scale_value,
            self._effective_timeout(timeout),
            _screenshot_masks(mask, "Page.screenshot"),
        )

    async def set_viewport_size(self, viewport_size: ViewportSize) -> None:
        viewport = _viewport(viewport_size, "viewport_size")
        assert viewport is not None
        await self._context.browser._native.page_set_viewport_size(
            self._object_id, viewport[0], viewport[1]
        )
        self._viewport_size = viewport

    @property
    def viewport_size(self) -> ViewportSize | None:
        if self._viewport_size is None:
            return None
        return {"width": self._viewport_size[0], "height": self._viewport_size[1]}

    async def page_errors(self) -> list[Error]:
        values = self._context.browser._native.page_errors(self._object_id)
        errors: list[Error] = []
        for name, message, stack in values:
            error = Error(message)
            setattr(error, "name", name)
            setattr(error, "stack", stack)
            errors.append(error)
        return errors

    async def console_messages(self) -> list[ConsoleMessage]:
        values = self._context.browser._native.console_messages(self._object_id)
        return [
            ConsoleMessage(
                self,
                message_type,
                text,
                arguments_json,
                location_url,
                line_number,
                column_number,
            )
            for (
                message_type,
                text,
                arguments_json,
                location_url,
                line_number,
                column_number,
            ) in values
        ]

    async def requests(self) -> list[Request]:
        return self._sync_request_history()

    def _refresh_snapshot(self) -> None:
        for object_id, _context_id, main_frame_id, url in self._context.browser._native.pages(
            self._context._object_id
        ):
            if object_id == self._object_id:
                self._main_frame_id = main_frame_id
                self._url = url
                return

    def _refresh_frames(self) -> list[Frame]:
        seen: dict[str, Frame] = {}
        for object_id, parent_id, name, url in self._context.browser._native.frames(
            self._object_id
        ):
            frame = self._frames.get(object_id)
            if frame is None:
                frame = Frame(self, object_id, parent_id, name, url)
            else:
                frame._parent_id = parent_id
                frame._name = name
                frame._url = url
            seen[object_id] = frame
        self._frames = seen
        return list(seen.values())

    @property
    def frames(self) -> list[Frame]:
        return self._refresh_frames()

    @property
    def main_frame(self) -> Frame:
        frames = self._refresh_frames()
        for frame in frames:
            if frame._parent_id is None:
                return frame
        raise TargetClosedError("Page has no main Frame")

    def frame(
        self,
        name: str | None = None,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
    ) -> Frame | None:
        if name is None and url is None:
            raise TypeError("Page.frame requires name or url")
        for frame in self._refresh_frames():
            if name is not None and frame.name == name:
                return frame
            if isinstance(url, str) and frame.url == url:
                return frame
            if isinstance(url, Pattern):
                if url.search(frame.url):
                    return frame
            if callable(url):
                if url(frame.url):
                    return frame
        return None

    async def title(self) -> str:
        return str(await self._context.browser._native.title(self._main_frame_id))

    async def content(self) -> str:
        return str(await self._context.browser._native.content(self._main_frame_id))

    async def add_init_script(
        self,
        script: str | None = None,
        *,
        path: str | Path | None = None,
    ) -> None:
        source = await _init_script_source(script, path, source_url=True)
        await self._context.browser._native.page_add_init_script(self._object_id, source)

    async def _register_binding(
        self,
        name: str,
        callback: Callable[..., object],
        needs_handle: bool,
        api_name: str,
    ) -> None:
        if not name:
            raise TypeError(f"{api_name}: name must be a non-empty string")

        async def register() -> None:
            registration_lock = self._context._binding_registration_lock
            if registration_lock is None:
                registration_lock = asyncio.Lock()
                self._context._binding_registration_lock = registration_lock
            async with registration_lock:
                if name in self._bindings:
                    raise Error(f'{api_name}: Function "{name}" has been already registered')
                binding = (callback, needs_handle)
                self._bindings[name] = binding
                try:
                    await self._context.browser._native.page_expose_binding(
                        self._object_id, name, needs_handle
                    )
                except BaseException:
                    if self._bindings.get(name) is binding:
                        self._bindings.pop(name, None)
                    raise

        await _shield_binding_registration(self._context.browser, register, api_name)

    async def expose_binding(
        self,
        name: str,
        callback: Callable[..., object],
        *,
        handle: bool | None = None,
    ) -> None:
        if not callable(callback):
            raise TypeError("Page.expose_binding: callback must be callable")
        await self._register_binding(
            name,
            _wrap_handler(callback),
            handle is True,
            "Page.expose_binding",
        )

    async def expose_function(self, name: str, callback: Callable[..., object]) -> None:
        if not callable(callback):
            raise TypeError("Page.expose_function: callback must be callable")
        wrapped_callback = _wrap_handler(callback)

        def invoke(_source: object, *args: object) -> object:
            return wrapped_callback(*args)

        await self._register_binding(name, invoke, False, "Page.expose_function")

    async def set_extra_http_headers(self, headers: Mapping[str, str]) -> None:
        await self._context.browser._native.set_page_extra_http_headers(
            self._object_id,
            _extra_http_headers(headers, "Page.set_extra_http_headers", indexed_values=True),
        )

    async def route(
        self,
        url: RouteURLMatcher,
        handler: Callable[..., object],
        *,
        times: int | None = None,
    ) -> None:
        await self._route_with_key(url, handler, handler, times=times)

    async def _route_with_key(
        self,
        url: RouteURLMatcher,
        handler: Callable[..., object],
        handler_key: object,
        *,
        times: int | None = None,
    ) -> None:
        if not callable(handler):
            raise TypeError("Page.route: handler must be callable")
        if not isinstance(url, (str, Pattern)) and not callable(url):
            raise TypeError("Page.route: url must be a string, compiled regex, or callable")
        times = _route_times(times, "Page.route")

        route_handler: Callable[..., object] = handler
        native_pattern = url
        native_times = times
        binding_handler: _BindingRouteHandler | None = None
        if not isinstance(url, str):
            native_pattern = "**/*"
            native_times = None

            def discard(registration: str) -> None:
                self._routes.pop(registration, None)
                self._route_patterns.pop(registration, None)
                self._route_handler_keys.pop(registration, None)

            async def retire(registration: str) -> None:
                await self._context.browser._native.page_unroute(
                    self._object_id,
                    [int(registration)],
                )

            binding_handler = _BindingRouteHandler(
                url,
                handler,
                times,
                discard,
                retire,
            )
            route_handler = binding_handler
        registration = str(
            await self._context.browser._native.page_route(
                self._object_id,
                cast(str, native_pattern),
                native_times,
            )
        )
        if binding_handler is not None:
            binding_handler.bind_registration(registration)
        self._routes[registration] = route_handler
        self._route_patterns[registration] = url
        self._route_handler_keys[registration] = handler_key

    async def unroute(
        self,
        url: RouteURLMatcher,
        handler: Callable[..., object] | None = None,
    ) -> None:
        if not isinstance(url, (str, Pattern)) and not callable(url):
            raise TypeError("Page.unroute: url must be a string, compiled regex, or callable")
        registrations = [
            registration
            for registration, pattern in self._route_patterns.items()
            if pattern == url
            and (handler is None or self._route_handler_keys.get(registration) is handler)
        ]
        if registrations:
            await self._context.browser._native.page_unroute(
                self._object_id,
                [int(registration) for registration in registrations],
            )
        for registration in registrations:
            self._routes.pop(registration, None)
            self._route_patterns.pop(registration, None)
            self._route_handler_keys.pop(registration, None)

    async def route_from_har(
        self,
        har: str | Path,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        not_found: Literal["abort", "fallback"] | None = None,
        update: bool | None = None,
        update_content: Literal["attach", "embed"] | None = None,
        update_mode: Literal["full", "minimal"] | None = None,
    ) -> None:
        if update:
            raise UnsupportedError("Page.route_from_har(update=True) is not installed")
        if update_content is not None or update_mode is not None:
            raise UnsupportedError(
                "Page.route_from_har update_content and update_mode require update=True"
            )
        if url is not None and not isinstance(url, str):
            raise UnsupportedError(
                "Page.route_from_har regex and predicate URL filters are not installed"
            )
        await self._context.browser._native.page_route_from_har(
            self._object_id,
            str(har),
            url,
            not_found,
        )

    async def unroute_all(
        self,
        *,
        behavior: Literal["default", "ignoreErrors", "wait"] | None = None,
    ) -> None:
        if behavior not in (None, "default", "wait"):
            raise UnsupportedError("Page.unroute_all(behavior='ignoreErrors') is not installed")
        await self._context.browser._native.page_unroute_all(
            self._object_id,
            behavior == "wait",
        )
        self._routes.clear()
        self._route_patterns.clear()
        self._route_handler_keys.clear()

    async def add_script_tag(
        self,
        *,
        content: str | None = None,
        path: str | Path | None = None,
        type: str | None = None,
        url: str | None = None,
    ) -> ElementHandle:
        url, content = await _tag_source(
            url=url, path=path, content=content, source_url_style="script"
        )
        handle = await self.main_frame.evaluate_handle(
            _ADD_SCRIPT_TAG_SOURCE,
            {"url": url, "content": content, "type": type},
        )
        element = handle.as_element()
        if element is None:
            await handle.dispose()
            raise Error("Page.add_script_tag did not create an ElementHandle")
        return element

    async def add_style_tag(
        self,
        *,
        content: str | None = None,
        path: str | Path | None = None,
        url: str | None = None,
    ) -> ElementHandle:
        url, content = await _tag_source(
            url=url, path=path, content=content, source_url_style="style"
        )
        handle = await self.main_frame.evaluate_handle(
            _ADD_STYLE_TAG_SOURCE,
            {"url": url, "content": content},
        )
        element = handle.as_element()
        if element is None:
            await handle.dispose()
            raise Error("Page.add_style_tag did not create an ElementHandle")
        return element

    async def set_content(
        self,
        html: str,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> None:
        self._url = str(
            await self._context.browser._native.set_content(
                self._object_id,
                html,
                wait_until or "load",
                self._effective_timeout(timeout, navigation=True),
            )
        )
        await self._context.browser._flush_network_notices()

    async def emulate_media(
        self,
        *,
        media: str | None = None,
        color_scheme: str | None = None,
        reduced_motion: str | None = None,
        forced_colors: str | None = None,
        contrast: str | None = None,
    ) -> None:
        await self._context.browser._native.emulate_media(
            self._object_id,
            (media, color_scheme, reduced_motion, forced_colors, contrast),
        )

    async def click(
        self,
        selector: str,
        *,
        button: Literal["left", "right", "middle"] | None = None,
        click_count: int | None = None,
        force: bool | None = None,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        selector = _string_argument(selector, "Page.click")
        strict_selection = self._selector_is_strict(strict, "Page.click")
        options: dict[str, object] = {}
        if button is not None:
            if button not in ("left", "right", "middle"):
                raise InvalidArgumentError("Page.click button must be left, right, or middle")
            options["button"] = button
        if click_count is not None:
            if (
                isinstance(click_count, bool)
                or not isinstance(cast(object, click_count), int)
                or not 1 <= click_count <= 255
            ):
                raise InvalidArgumentError("Page.click click_count must be an integer in [1, 255]")
            options["click_count"] = click_count
        if force is not None:
            options["force"] = _boolean_argument(force, "Page.click force")
        if position is not None:
            if (
                not isinstance(cast(object, position), Mapping)
                or len(position) != 2
                or "x" not in position
                or "y" not in position
            ):
                raise InvalidArgumentError("Page.click position must contain x and y")
            options["position"] = {
                "x": _mouse_number(position["x"], "Page.click position.x"),
                "y": _mouse_number(position["y"], "Page.click position.y"),
            }
        if modifiers is not None:
            if "locator-click-modifiers" not in _native.selected_features():
                raise UnsupportedError(
                    "Page.click modifiers require the locator-click-modifiers profile"
                )
            if isinstance(modifiers, (str, bytes)) or len(modifiers) > 5:
                raise InvalidArgumentError(
                    "Page.click modifiers must be a sequence of at most five names"
                )
            if any(
                not isinstance(cast(object, name), str)
                or name not in ("Alt", "Control", "ControlOrMeta", "Meta", "Shift")
                for name in modifiers
            ):
                raise InvalidArgumentError("Page.click modifiers contain an unknown name")
            options["modifiers"] = list(modifiers)
        timeout_ms = self._effective_timeout(timeout)
        await self._context.browser._native.click_with_options(
            self._main_frame_id, selector, options, strict_selection, timeout_ms
        )
        self._refresh_snapshot()

    async def tap(
        self,
        selector: str,
        *,
        modifiers: Sequence[Literal["Alt", "Control", "ControlOrMeta", "Meta", "Shift"]]
        | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        if "touchscreen-input" not in _native.selected_features():
            raise UnsupportedError("Page.tap requires the touchscreen-input profile")
        selector = _string_argument(selector, "Page.tap")
        options = _tap_options(
            "Page.tap",
            modifiers=modifiers,
            position=position,
            force=force,
            no_wait_after=no_wait_after,
            trial=trial,
        )
        try:
            await self._context.browser._native.page_tap(
                self._main_frame_id,
                selector,
                options,
                self._selector_is_strict(strict, "Page.tap"),
                self._effective_timeout(timeout),
            )
        except Error as error:
            raise rewrite_api_error(error, "Page.tap") from None
        self._refresh_snapshot()

    async def dblclick(
        self,
        selector: str,
        *,
        timeout: float | None = None,
        strict: bool | None = None,
    ) -> None:
        locator = self._selector_locator(
            _string_argument(selector, "Page.dblclick"), strict, "Page.dblclick"
        )
        await locator.dblclick(timeout=timeout)
        self._refresh_snapshot()

    async def dispatch_event(
        self,
        selector: str,
        type: str,
        event_init: Mapping[str, object] | None = None,
        *,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> None:
        locator = self._selector_locator(selector, strict, "Page.dispatch_event")
        await locator.dispatch_event(type, event_init, timeout=timeout)

    async def set_input_files(
        self,
        selector: str,
        files: FileInput,
        *,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
    ) -> None:
        del strict
        await self.locator(selector).set_input_files(
            files,
            timeout=timeout,
            no_wait_after=no_wait_after,
        )

    def locator(
        self,
        selector: str,
        *,
        has_text: str | Pattern[str] | None = None,
        has_not_text: str | Pattern[str] | None = None,
        has: Locator | None = None,
        has_not: Locator | None = None,
    ) -> Locator:
        value = _string_argument(selector, "Page.locator")
        native_locator = self._context.browser._native.locator(self._main_frame_id, value)
        locator = Locator(self, value, None, native_locator)
        if any(value is not None for value in (has_text, has_not_text, has, has_not)):
            return locator.filter(
                has_text=has_text,
                has_not_text=has_not_text,
                has=has,
                has_not=has_not,
            )
        return locator

    def _selector_is_strict(self, strict: bool | None, api: str) -> bool:
        explicit = _optional_boolean_argument(strict, f"{api} strict")
        if "context-strict-selectors" not in _native.selected_features():
            return explicit is True
        return bool(
            self._context.browser._native.context_selector_is_strict(
                self._context._object_id, explicit
            )
        )

    def _selector_locator(self, selector: str, strict: bool | None, api: str) -> Locator:
        locator = self.locator(selector)
        return locator if self._selector_is_strict(strict, api) else locator.first

    def frame_locator(self, selector: str) -> FrameLocator:
        value = _string_argument(selector, "Page.frame_locator")
        native = self._context.browser._native.frame_locator(self._main_frame_id, value)
        return FrameLocator(self, value, native)

    def get_by_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Page.get_by_text")
        native_locator = self._context.browser._native.semantic_locator(
            self._main_frame_id, "text", source, bool(exact), flags
        )
        return Locator(self, None, None, native_locator)

    def get_by_label(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Page.get_by_label")
        native_locator = self._context.browser._native.semantic_locator(
            self._main_frame_id, "label", source, bool(exact), flags
        )
        return Locator(self, None, None, native_locator)

    def get_by_placeholder(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Page.get_by_placeholder")
        native_locator = self._context.browser._native.semantic_locator(
            self._main_frame_id, "placeholder", source, bool(exact), flags
        )
        return Locator(self, None, None, native_locator)

    def get_by_alt_text(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Page.get_by_alt_text")
        native_locator = self._context.browser._native.semantic_locator(
            self._main_frame_id, "altText", source, bool(exact), flags
        )
        return Locator(self, None, None, native_locator)

    def get_by_title(self, text: str | Pattern[str], *, exact: bool | None = None) -> Locator:
        source, flags = _text_match(text, "Page.get_by_title")
        native_locator = self._context.browser._native.semantic_locator(
            self._main_frame_id, "title", source, bool(exact), flags
        )
        return Locator(self, None, None, native_locator)

    def get_by_test_id(self, test_id: str | Pattern[str]) -> Locator:
        source, flags = _text_match(test_id, "Page.get_by_test_id")
        native_locator = self._context.browser._native.semantic_locator(
            self._main_frame_id, "testId", source, False, flags
        )
        return Locator(self, None, None, native_locator)

    def get_by_role(
        self,
        role: str,
        *,
        checked: bool | None = None,
        disabled: bool | None = None,
        expanded: bool | None = None,
        include_hidden: bool | None = None,
        level: int | None = None,
        name: str | Pattern[str] | None = None,
        pressed: bool | None = None,
        selected: bool | None = None,
        exact: bool | None = None,
    ) -> Locator:
        native_locator = self._context.browser._native.role_locator(
            self._main_frame_id,
            *_role_arguments(
                role,
                checked=checked,
                disabled=disabled,
                expanded=expanded,
                include_hidden=include_hidden,
                level=level,
                name=name,
                pressed=pressed,
                selected=selected,
                exact=exact,
            ),
        )
        return Locator(self, None, None, native_locator)

    async def drag_and_drop(
        self,
        source: str,
        target: str,
        *,
        source_position: Mapping[str, float] | None = None,
        target_position: Mapping[str, float] | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        steps: int | None = None,
    ) -> None:
        del no_wait_after
        await self._context.browser._native.drag_and_drop(
            self._object_id,
            self._main_frame_id,
            _string_argument(source, "Page.drag_and_drop"),
            _string_argument(target, "Page.drag_and_drop"),
            self._effective_timeout(timeout),
            (
                _drag_position(source_position, "source_position"),
                _drag_position(target_position, "target_position"),
                _drag_steps(steps),
                trial or False,
                strict or False,
                force or False,
            ),
        )

    async def fill(
        self,
        selector: str,
        value: str,
        *,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        if no_wait_after is not None:
            raise UnsupportedError("Page.fill(no_wait_after=...) is not installed")
        await self._context.browser._native.fill(
            self._main_frame_id,
            selector,
            _string_argument(value, "Page.fill"),
            self._effective_timeout(timeout),
            force or False,
        )

    async def select_option(
        self,
        selector: str,
        value: str | Sequence[str] | None = None,
        *,
        index: int | None = None,
        label: str | None = None,
        element: ElementHandle | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> list[str]:
        if "page-select-option" not in _native.selected_features():
            raise UnsupportedError("Page.select_option requires the page-select-option profile")
        # The upstream parameter is deprecated and has no effect.
        _optional_boolean_argument(no_wait_after, "Page.select_option no_wait_after")
        values: list[object] = []
        if isinstance(value, str):
            values.append(value)
        elif value is not None:
            if isinstance(cast(object, value), Sequence) and not isinstance(
                value, (bytes, bytearray)
            ):
                values.extend(value)
            else:
                values.append(value)
        if label is not None:
            values.append({"label": label})
        if index is not None:
            values.append({"index": index})
        if element is not None:
            if not isinstance(cast(object, element), ElementHandle) or element._page is not self:
                raise InvalidArgumentError("Page.select_option element must belong to this Page")
            native_element = element._native
        else:
            native_element = None
        locator = self._selector_locator(
            _string_argument(selector, "Page.select_option"), strict, "Page.select_option"
        )
        if locator._native_locator is None:
            raise UnsupportedError("Page.select_option requires a Rust-owned query")
        try:
            return await locator._native_locator.select_options(
                values,
                self._effective_timeout(timeout),
                force,
                native_element,
            )
        except InvalidArgumentError as error:
            raise Error(f"Page.select_option: {error}") from None
        except Error as error:
            raise rewrite_api_error(error, "Page.select_option") from None

    async def press(
        self,
        selector: str,
        key: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        await self._context.browser._native.press(
            self._main_frame_id,
            selector,
            key,
            self._effective_timeout(timeout),
            (_optional_milliseconds(delay, "delay"), no_wait_after or False),
        )
        self._refresh_snapshot()

    async def type(
        self,
        selector: str,
        text: str,
        *,
        delay: float | None = None,
        timeout: float | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
    ) -> None:
        locator = self._selector_locator(
            _string_argument(selector, "Page.type"), strict, "Page.type"
        )
        await locator.press_sequentially(
            _string_argument(text, "Page.type"),
            delay=delay,
            no_wait_after=no_wait_after,
            timeout=timeout,
        )

    async def set_checked(
        self,
        selector: str,
        checked: bool,
        *,
        force: bool | None = None,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        trial: bool | None = None,
        no_wait_after: bool | None = None,
    ) -> None:
        if position is not None:
            raise UnsupportedError("Page.set_checked(position=...) is not installed")
        if no_wait_after is not None:
            raise UnsupportedError("Page.set_checked(no_wait_after=...) is not installed")
        await self._context.browser._native.set_checked(
            self._main_frame_id,
            selector,
            checked,
            self._effective_timeout(timeout),
            (force, trial),
        )

    async def check(
        self,
        selector: str,
        *,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        locator = self._selector_locator(selector, strict, "Page.check")
        await locator.set_checked(
            True,
            position=position,
            timeout=timeout,
            force=force,
            no_wait_after=no_wait_after,
            trial=trial,
        )

    async def uncheck(
        self,
        selector: str,
        *,
        position: Mapping[str, float] | None = None,
        timeout: float | None = None,
        force: bool | None = None,
        no_wait_after: bool | None = None,
        strict: bool | None = None,
        trial: bool | None = None,
    ) -> None:
        locator = self._selector_locator(selector, strict, "Page.uncheck")
        await locator.set_checked(
            False,
            position=position,
            timeout=timeout,
            force=force,
            no_wait_after=no_wait_after,
            trial=trial,
        )

    async def focus(self, selector: str, *, timeout: float | None = None) -> None:
        await self._context.browser._native.focus(
            self._main_frame_id, selector, self._effective_timeout(timeout)
        )

    async def _locator_read(
        self,
        selector: str,
        operation: str,
        *,
        timeout: float | None = None,
        attribute: str | None = None,
        strict: bool | None = None,
    ) -> object:
        native = self._context.browser._native
        args = (
            self._main_frame_id,
            selector,
            operation,
            self._effective_timeout(timeout),
            attribute,
        )
        if "context-strict-selectors" in _native.selected_features():
            return await native.locator_read(
                *args, self._selector_is_strict(strict, f"Page.{operation}")
            )
        if strict is False:
            first = self.locator(selector).first
            return await first._read(operation, timeout=timeout, attribute=attribute)
        return await native.locator_read(*args)

    async def _locator_expect_count(
        self, selector: str, expected: int, timeout_ms: int, is_not: bool
    ) -> dict[str, object]:
        value = await self._context.browser._native.locator_expect_count(
            self._main_frame_id,
            selector,
            expected,
            timeout_ms,
            is_not,
        )
        return value

    async def input_value(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> str:
        return str(await self._locator_read(selector, "inputValue", timeout=timeout, strict=strict))

    async def is_hidden(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> bool:
        return bool(await self._locator_read(selector, "isHidden", timeout=timeout, strict=strict))

    async def is_visible(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> bool:
        return bool(await self._locator_read(selector, "isVisible", timeout=timeout, strict=strict))

    async def is_checked(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> bool:
        return bool(await self._locator_read(selector, "isChecked", timeout=timeout, strict=strict))

    async def is_enabled(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> bool:
        return bool(await self._locator_read(selector, "isEnabled", timeout=timeout, strict=strict))

    async def is_disabled(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> bool:
        return bool(
            await self._locator_read(selector, "isDisabled", timeout=timeout, strict=strict)
        )

    async def is_editable(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> bool:
        return bool(
            await self._locator_read(selector, "isEditable", timeout=timeout, strict=strict)
        )

    async def text_content(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> str | None:
        value = await self._locator_read(selector, "textContent", timeout=timeout, strict=strict)
        return None if value is None else str(value)

    async def inner_text(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> str:
        return str(await self._locator_read(selector, "innerText", timeout=timeout, strict=strict))

    async def inner_html(
        self, selector: str, *, strict: bool | None = None, timeout: float | None = None
    ) -> str:
        return str(await self._locator_read(selector, "innerHTML", timeout=timeout, strict=strict))

    async def get_attribute(
        self,
        selector: str,
        name: str,
        *,
        strict: bool | None = None,
        timeout: float | None = None,
    ) -> str | None:
        value = await self._locator_read(
            selector, "getAttribute", timeout=timeout, attribute=name, strict=strict
        )
        return None if value is None else str(value)

    async def evaluate(self, expression: str, arg: object = None) -> object:
        try:
            if isinstance(arg, JSHandle):
                value = await self._context.browser._native.evaluate_handle_argument(
                    self._main_frame_id, expression, arg._native
                )
            else:
                value = await self._context.browser._native.evaluate(
                    self._main_frame_id, expression, _native_evaluation_argument(arg)
                )
        except Error as error:
            replacement = None
            if type(error) is Error:
                if isinstance(arg, JSHandle) and str(error) == "JSHandle is disposed":
                    replacement = "no object with guid: JSHandle is disposed"
                elif (
                    str(error) == "Backend: CDP error -32000: Inspected target navigated or closed"
                ):
                    replacement = (
                        "Execution context was destroyed, most likely because of a navigation."
                    )
            raise rewrite_api_error(error, "Page.evaluate", message=replacement) from None
        self._refresh_snapshot()
        return value

    async def evaluate_handle(self, expression: str, arg: object = None) -> JSHandle:
        value = await self._context.browser._native.frame_evaluate_handle(
            self._main_frame_id, expression, _native_evaluation_argument(arg)
        )
        return _wrap_handle(self._context.browser._native, value, self)

    async def wait_for_function(
        self,
        expression: str,
        arg: object = None,
        *,
        polling: float | str | None = None,
        timeout: float | None = None,
    ) -> JSHandle:
        return await self.main_frame.wait_for_function(
            expression, arg, polling=polling, timeout=timeout
        )

    async def wait_for_load_state(
        self,
        state: str | None = None,
        *,
        timeout: float | None = None,
    ) -> None:
        wait_until = state or "load"
        timeout_ms = self._effective_timeout(timeout, navigation=True)
        try:
            await self._context.browser._native.wait_for_load_state(
                self._object_id,
                self._main_frame_id,
                wait_until,
                timeout_ms,
            )
        except InvalidArgumentError as error:
            if str(error) == "wait_until must be commit, domcontentloaded, load, or networkidle":
                raise InvalidArgumentError(
                    "state: expected one of (load|domcontentloaded|networkidle|commit)"
                ) from None
            raise

    async def query_selector(
        self,
        selector: str,
        *,
        strict: bool | None = None,
    ) -> ElementHandle | None:
        self._context.browser._native.validate_selector(self._main_frame_id, selector)
        program_count = await self._context.browser._native.registered_selector_match_count(
            self._main_frame_id, selector
        )
        if program_count == 0:
            return None
        if program_count is not None:
            raise UnsupportedError(
                "Page.query_selector cannot publish a browser-program selector handle yet"
            )
        locator = self.locator(selector)
        if await locator.count() == 0:
            return None
        if not self._selector_is_strict(strict, "Page.query_selector"):
            locator = locator.first
        return await locator.element_handle()

    async def query_selector_all(self, selector: str) -> list[ElementHandle]:
        return await self.locator(selector).element_handles()

    async def eval_on_selector(
        self,
        selector: str,
        expression: str,
        arg: object = None,
        *,
        timeout: float | None = None,
    ) -> object:
        value = await self._context.browser._native.eval_on_selector(
            self._main_frame_id,
            selector,
            expression,
            arg,
            self._effective_timeout(timeout),
        )
        self._refresh_snapshot()
        return value

    async def eval_on_selector_all(
        self,
        selector: str,
        expression: str,
        arg: object = None,
    ) -> object:
        value = await self._context.browser._native.eval_on_selector_all(
            self._main_frame_id,
            selector,
            expression,
            arg,
        )
        self._refresh_snapshot()
        return value

    async def goto(
        self,
        url: str,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
        referer: str | None = None,
    ) -> Response | None:
        # Route callback errors currently have a Page.goto preflight only;
        # other public commands do not yet share a command boundary.
        self._context.browser._raise_deferred_route_error()
        call_stack = "".join(traceback.format_stack())
        navigation_wait_until = wait_until or "load"
        try:
            result = await self._context.browser._native.goto(
                self._object_id,
                url,
                navigation_wait_until,
                referer,
                self._effective_timeout(timeout, navigation=True),
            )
        except Error as error:
            message = str(error).replace(
                "waitUntil: expected one of", "wait_until: expected one of"
            )
            if getattr(error, "code", None) in {"PWRIGHT_TIMEOUT", "PWRIGHT_BACKEND"} or (
                '"referer" is already specified as extra HTTP header' in message
            ):
                message += (
                    f'\nCall log:\n  - navigating to "{url}", '
                    f'waiting until "{navigation_wait_until}"'
                )
            rewritten = rewrite_api_error(error, "Page.goto", message=message)
            setattr(rewritten, "stack", call_stack)
            raise rewritten from None
        await self._context.browser._flush_network_notices()
        await self._context._drain_cdp_sessions(self._object_id)
        self._url = str(result["url"])
        response = result["response"]
        return None if response is None else self._navigation_response(response)

    async def reload(
        self,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> Response | None:
        result = await self._context.browser._native.reload(
            self._object_id,
            wait_until or "load",
            self._effective_timeout(timeout, navigation=True),
        )
        await self._context.browser._flush_network_notices()
        self._url = str(result["url"])
        response = result["response"]
        return None if response is None else self._navigation_response(response)

    async def go_back(
        self,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> Response | None:
        result = await self._context.browser._native.go_back(
            self._object_id,
            wait_until or "load",
            self._effective_timeout(timeout, navigation=True),
        )
        if result is None:
            return None
        await self._context.browser._flush_network_notices()
        self._url = str(result["url"])
        self._refresh_frames()
        response = result["response"]
        return None if response is None else self._navigation_response(response)

    async def go_forward(
        self,
        *,
        timeout: float | None = None,
        wait_until: str | None = None,
    ) -> Response | None:
        result = await self._context.browser._native.go_forward(
            self._object_id,
            wait_until or "load",
            self._effective_timeout(timeout, navigation=True),
        )
        if result is None:
            return None
        await self._context.browser._flush_network_notices()
        self._url = str(result["url"])
        self._refresh_frames()
        response = result["response"]
        return None if response is None else self._navigation_response(response)

    def _navigation_response(
        self,
        value: Mapping[str, Any],
        *,
        frame: Frame | None = None,
        request: Request | None = None,
        lookup_history: bool = True,
    ) -> Response:
        request_id = value.get("request_id")
        frame_id = self._main_frame_id if frame is None else frame._object_id
        # A navigation result already identifies its Frame even when the
        # independent request-history capability is not installed.
        if frame is None:
            frame = self._frames.get(frame_id)
        navigation_body = (
            (self._object_id, frame_id, request_id)
            if isinstance(request_id, str) and request_id
            else None
        )
        if (
            request is None
            and lookup_history
            and navigation_body is not None
            and "request-history" in _native.selected_features()
        ):
            for candidate in reversed(self._sync_request_history()):
                if str(candidate._value.get("native_request_id", "")) == request_id:
                    request = candidate
                    break
        return Response(
            value,
            self._context.browser._native,
            self._context,
            request=request,
            navigation_body=navigation_body,
            frame_override=frame,
        )

    async def _frame_navigation_response(self, value: Mapping[str, Any], frame: Frame) -> Response:
        request_id = value.get("request_id")
        request = None
        if (
            isinstance(request_id, str)
            and request_id
            and "request-history" in _native.selected_features()
        ):
            for candidate in tuple(self._requests.values()):
                if str(candidate._value.get("native_request_id", "")) == request_id:
                    request = candidate
                    break
            if request is None:
                try:
                    raw_request = await asyncio.wait_for(
                        self._context.browser._native.request_by_native_id(
                            self._object_id, request_id
                        ),
                        timeout=1,
                    )
                except asyncio.TimeoutError:
                    pass
                else:
                    request = self._wrap_request(raw_request)
        return self._navigation_response(value, frame=frame, request=request, lookup_history=False)

    async def wait_for_url(
        self,
        url: str | Pattern[str] | Callable[[str], bool],
        *,
        wait_until: str | None = None,
        timeout: float | None = None,
    ) -> None:
        self._url = str(
            await self._context.browser._native.wait_for_url(
                self._object_id,
                url,
                wait_until or "load",
                self._effective_timeout(timeout, navigation=True),
            )
        )

    def expect_navigation(
        self,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        wait_until: str | None = None,
        timeout: float | None = None,
    ) -> NavigationContextManager:
        return NavigationContextManager(
            self,
            wait_until or "load",
            self._effective_timeout(timeout, navigation=True),
            url=url,
        )

    def expect_event(
        self,
        event: str,
        predicate: NetworkMatcher | PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[NetworkValue] | PageEventContextManager:
        if event in {"request", "response", "requestfinished", "requestfailed"}:
            return EventContextManager(
                self._context.browser._native,
                self._context.browser._native.arm_network_wait,
                self._object_id,
                self._context,
                event,
                cast("NetworkMatcher | None", predicate),
                self._effective_timeout(timeout),
            )
        if event in {
            "load",
            "domcontentloaded",
            "close",
            "popup",
            "pageerror",
            "dialog",
            "filechooser",
        }:
            if predicate is not None and not callable(predicate):
                raise TypeError("Page lifecycle event predicate must be callable")
            return PageEventContextManager(
                self,
                event,
                cast("PageEventPredicate | None", predicate),
                self._effective_timeout(timeout),
            )
        if (
            event in {"frameattached", "framenavigated"}
            and "frame-events" in _native.selected_features()
        ):
            if predicate is not None and not callable(predicate):
                raise TypeError("Page Frame event predicate must be callable")
            return PageEventContextManager(
                self,
                event,
                cast("PageEventPredicate | None", predicate),
                self._effective_timeout(timeout),
            )
        if event == "worker" and "worker-events" in _native.selected_features():
            if predicate is not None and not callable(predicate):
                raise TypeError("Page Worker event predicate must be callable")
            return PageEventContextManager(
                self,
                event,
                cast("PageEventPredicate | None", predicate),
                self._effective_timeout(timeout),
            )
        if event == "console" and "console-events" in _native.selected_features():
            if predicate is not None and not callable(predicate):
                raise TypeError("Page console event predicate must be callable")
            return PageEventContextManager(
                self,
                event,
                cast("PageEventPredicate | None", predicate),
                self._effective_timeout(timeout),
            )
        if event == "download" and "download-events" in _native.selected_features():
            if predicate is not None and not callable(predicate):
                raise TypeError("Page download event predicate must be callable")
            return PageEventContextManager(
                self,
                event,
                cast("PageEventPredicate | None", predicate),
                self._effective_timeout(timeout),
            )
        raise UnsupportedError(f"Page event {event!r} is not installed")

    def expect_popup(
        self,
        predicate: PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        return PageEventContextManager(self, "popup", predicate, self._effective_timeout(timeout))

    def expect_worker(
        self,
        predicate: PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        if "worker-events" not in _native.selected_features():
            raise UnsupportedError("Page Worker event capability is not installed")
        return PageEventContextManager(self, "worker", predicate, self._effective_timeout(timeout))

    def expect_console_message(
        self,
        predicate: PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        if "console-events" not in _native.selected_features():
            raise UnsupportedError("Page console event capability is not installed")
        if predicate is not None and not callable(predicate):
            raise TypeError("Page console event predicate must be callable")
        return PageEventContextManager(self, "console", predicate, self._effective_timeout(timeout))

    def expect_download(
        self,
        predicate: PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        if "download-events" not in _native.selected_features():
            raise UnsupportedError("Page download event capability is not installed")
        if predicate is not None and not callable(predicate):
            raise TypeError("Page download event predicate must be callable")
        return PageEventContextManager(
            self, "download", predicate, self._effective_timeout(timeout)
        )

    def expect_file_chooser(
        self,
        predicate: PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> PageEventContextManager:
        if "file-chooser-events" not in _native.selected_features():
            raise UnsupportedError("Page file chooser capability is not installed")
        if predicate is not None and not callable(predicate):
            raise TypeError("Page file chooser predicate must be callable")
        return PageEventContextManager(
            self, "filechooser", predicate, self._effective_timeout(timeout)
        )

    def expect_request(
        self,
        url_or_predicate: NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[Request]:
        return EventContextManager(
            self._context.browser._native,
            self._context.browser._native.arm_network_wait,
            self._object_id,
            self._context,
            "request",
            url_or_predicate,
            self._effective_timeout(timeout),
        )

    def expect_response(
        self,
        url_or_predicate: NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[Response]:
        return EventContextManager(
            self._context.browser._native,
            self._context.browser._native.arm_network_wait,
            self._object_id,
            self._context,
            "response",
            url_or_predicate,
            self._effective_timeout(timeout),
        )

    async def _wait_for_network_event(self, event_info: EventContextManager[TNetwork]) -> TNetwork:
        await event_info.__aenter__()
        try:
            value = await event_info.value
        except BaseException as error:
            await event_info.__aexit__(type(error), error, error.__traceback__)
            raise
        await event_info.__aexit__(None, None, None)
        return value

    async def wait_for_event(
        self,
        event: str,
        predicate: NetworkMatcher | PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> NetworkValue | object:
        event_info = self.expect_event(event, predicate, timeout=timeout)
        if isinstance(event_info, PageEventContextManager):
            await event_info.__aenter__()
            try:
                value = await event_info.value
            except BaseException as error:
                await event_info.__aexit__(type(error), error, error.__traceback__)
                raise
            await event_info.__aexit__(None, None, None)
            return value
        return await self._wait_for_network_event(event_info)

    async def wait_for_request(
        self,
        url_or_predicate: NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> Request:
        return await self._wait_for_network_event(
            self.expect_request(url_or_predicate, timeout=timeout)
        )

    async def wait_for_response(
        self,
        url_or_predicate: NetworkMatcher,
        *,
        timeout: float | None = None,
    ) -> Response:
        return await self._wait_for_network_event(
            self.expect_response(url_or_predicate, timeout=timeout)
        )

    async def close(self, *, run_before_unload: bool = False, reason: str | None = None) -> None:
        if reason is not None:
            raise UnsupportedError("Page.close(reason=...) is not installed")
        if self._closed:
            return
        if self._closes_context and not run_before_unload:
            await self._context.close(reason=reason)
            return
        if run_before_unload:
            await self._context.browser._native.run_before_unload(self._object_id)
            return
        closed_context = await self._context.browser._native.close_page(self._object_id)
        if isinstance(cast(object, self._context), BrowserContext):
            self._context._retire_cdp_sessions(self._object_id)
        self._retire_workers()
        self._closed = True
        if "page-events" not in _native.selected_features():
            self._context._pages.pop(self._object_id, None)
        if closed_context:
            self._context._closed = True
            if isinstance(cast(object, self._context), BrowserContext):
                self._context._retire_all_cdp_sessions()
            for page in self._context._pages.values():
                page._closed = True
            if "page-events" not in _native.selected_features():
                self._context._pages.clear()
                self._context.browser._contexts.pop(self._context._object_id, None)


class Tracing:
    async def stop(self, *, path: str | Path | None = None) -> None:
        if path is not None:
            raise UnsupportedError("Tracing.stop(path=...) is not installed")


class CDPSession:
    """One Rust-owned, auxiliary Chromium DevTools session."""

    def __init__(self, context: BrowserContext, page_id: str, token: int) -> None:
        self._context = context
        self._page_id = page_id
        self._token = token
        self._closed = False
        self._event_handlers: dict[str, list[Callable[[dict[str, object]], object]]] = {}
        self._once_handlers: dict[
            str,
            list[
                tuple[
                    Callable[[dict[str, object]], object],
                    Callable[[dict[str, object]], object],
                ]
            ],
        ] = {}
        self._delivery_lock = asyncio.Lock()
        self._pending_event_handlers: set[asyncio.Future[object]] = set()
        self._owned_event_handler_tasks: set[asyncio.Task[object]] = set()
        self._registration_flush_task: asyncio.Task[None] | None = None
        self._detach_task: asyncio.Task[None] | None = None

    def on(self, event: str, handler: Callable[[dict[str, object]], object]) -> None:
        if not isinstance(cast(object, event), str) or not event:
            raise TypeError("CDPSession.on event must be a nonempty string")
        if sum(map(len, self._event_handlers.values())) >= 64:
            raise ResourceExhaustedError("CDPSession event handler limit exceeded")
        self._event_handlers.setdefault(event, []).append(handler)
        schedule = getattr(self._context.browser, "_schedule_cdp_registration_flush", None)
        if callable(schedule):
            schedule(self)

    def once(self, event: str, handler: Callable[[dict[str, object]], object]) -> None:
        def invoke(params: dict[str, object]) -> object:
            self.remove_listener(event, invoke)
            return handler(params)

        self.on(event, invoke)
        self._once_handlers.setdefault(event, []).append((handler, invoke))

    def remove_listener(self, event: str, handler: Callable[[dict[str, object]], object]) -> None:
        handlers = self._event_handlers.get(event)
        if handlers is None:
            return
        once_handlers = self._once_handlers.get(event, [])
        remove = [handler]
        remove.extend(invoke for original, invoke in once_handlers if original is handler)
        self._event_handlers[event] = [
            value for value in handlers if all(value is not target for target in remove)
        ]
        self._once_handlers[event] = [
            (original, invoke)
            for original, invoke in once_handlers
            if original is not handler and invoke is not handler
        ]
        if not self._once_handlers[event]:
            self._once_handlers.pop(event, None)
        if not self._event_handlers[event]:
            self._event_handlers.pop(event, None)

    def _schedule_event_handler(self, outcome: object) -> None:
        if len(self._pending_event_handlers) >= 64:
            if inspect.iscoroutine(outcome):
                outcome.close()
            raise ResourceExhaustedError("CDPSession pending async event handler limit exceeded")
        if isinstance(outcome, asyncio.Future):
            future = cast(asyncio.Future[object], outcome)
        else:
            future = asyncio.ensure_future(cast(Awaitable[object], outcome))
            self._owned_event_handler_tasks.add(future)
        self._pending_event_handlers.add(future)
        future.add_done_callback(self._event_handler_done)

    def _event_handler_done(self, future: asyncio.Future[object]) -> None:
        self._pending_event_handlers.discard(future)
        if isinstance(future, asyncio.Task):
            self._owned_event_handler_tasks.discard(future)
        if future.cancelled():
            return
        error = future.exception()
        if error is not None and not self._closed:
            future.get_loop().call_exception_handler(
                {
                    "message": "CDPSession async event handler failed",
                    "exception": error,
                    "cdp_session": self,
                }
            )

    async def _drain_events(self) -> None:
        async with self._delivery_lock:
            if self._closed or self._detach_task is not None:
                return
            native = self._context.browser._native
            for _ in range(8):
                batch = await native.cdp_session_claim(self._token, 64)
                delivered = 0
                try:
                    if self._closed:
                        return
                    while event := batch.peek():
                        method, params = event
                        for handler in list(self._event_handlers.get(method, [])):
                            try:
                                outcome = handler(params)
                            except asyncio.CancelledError as error:
                                # A callback-raised cancellation is a handler
                                # failure, not cancellation of the Watch task.
                                raise Error("CDPSession event handler cancelled") from error
                            if inspect.isawaitable(outcome):
                                self._schedule_event_handler(outcome)
                        batch.ack_next()
                        delivered += 1
                finally:
                    batch.rollback()
                if delivered < 64:
                    return

    async def send(
        self, method: str, params: Mapping[str, object] | None = None
    ) -> dict[str, object]:
        if self._closed or self._detach_task is not None:
            raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
        if not isinstance(cast(object, method), str) or not method:
            raise TypeError("CDPSession.send method must be a nonempty string")
        if params is not None and not isinstance(cast(object, params), Mapping):
            raise TypeError("CDPSession.send params must be a mapping")
        try:
            params_json = json.dumps(
                {} if params is None else dict(params),
                ensure_ascii=True,
                allow_nan=False,
                separators=(",", ":"),
            )
        except (TypeError, ValueError) as error:
            raise TypeError("CDPSession.send params must be JSON-serializable") from error
        if len(params_json) > 1024 * 1024:
            raise ResourceExhaustedError("CDPSession.send params exceed 1 MiB")
        result = await self._context.browser._native.cdp_session_send(
            self._token, method, params_json
        )
        await self._drain_events()
        return result

    async def detach(self) -> None:
        async with self._delivery_lock:
            if self._closed:
                raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
            task = self._detach_task
            if task is None:

                async def dispatch_detach() -> None:
                    await self._context.browser._native.cdp_session_detach(self._token)

                task = asyncio.create_task(dispatch_detach())
                self._detach_task = task
                browser = self._context.browser
                browser._cdp_detach_tasks.add(task)
                task.add_done_callback(browser._cdp_detach_tasks.discard)
                task.add_done_callback(self._finish_detach)
            try:
                # Caller cancellation must not discard a detach that may
                # already have retired the native child.
                await asyncio.shield(task)
            finally:
                if task.done():
                    self._finish_detach(task)

    def _finish_detach(self, task: asyncio.Task[None]) -> None:
        if self._detach_task is not task:
            return
        self._detach_task = None
        if task.cancelled():
            return
        try:
            task.result()
        except TargetClosedError:
            self._context._retire_cdp_session(self._token)
        except BaseException:
            # A failed native detach remains retryable. Recheck its event
            # queue because a pending detach can have invalidated delivery.
            schedule = getattr(self._context.browser, "_schedule_cdp_registration_flush", None)
            if callable(schedule):
                schedule(self)
        else:
            self._context._retire_cdp_session(self._token)


class BrowserContext:
    def __init__(
        self,
        browser: Browser,
        object_id: str,
        base_url: str | None,
        viewport_size: tuple[int, int] | None = None,
    ) -> None:
        self._browser = browser
        self._object_id = object_id
        self._base_url = base_url
        self._viewport_size = viewport_size
        self._pages: dict[str, Page] = {}
        self._cdp_sessions: dict[int, CDPSession] = {}
        self._bindings: dict[str, tuple[Callable[..., object], bool]] = {}
        self._binding_registration_lock: asyncio.Lock | None = None
        self._routes: dict[str, Callable[..., object]] = {}
        self._route_patterns: dict[str, RouteURLMatcher] = {}
        self._route_handler_keys: dict[str, object] = {}
        self._closed = False
        self._request_context: APIRequestContext | None = None
        self._event_waiters: dict[
            str,
            list[tuple[asyncio.Future[object], PageEventPredicate | None, int]],
        ] = {}
        self._event_handlers: dict[str, list[Callable[[object], object]]] = {}
        self._close_emitted = False
        self._tracing = Tracing()

    async def __aenter__(self) -> BrowserContext:
        return self

    async def __aexit__(self, *_exc: object) -> None:
        await self.close()

    def __repr__(self) -> str:
        return f"<BrowserContext browser={self.browser}>"

    @property
    def browser(self) -> Browser:
        return self._browser

    @property
    def tracing(self) -> Tracing:
        return self._tracing

    @property
    def request(self) -> APIRequestContext:
        self._ensure_request_context()
        if self._request_context is None:
            raise UnsupportedError(
                "BrowserContext.request requires the context-api-request profile"
            )
        return self._request_context

    async def new_cdp_session(self, page: Page | Frame) -> CDPSession:
        if "cdp-session" not in _native.selected_features():
            raise UnsupportedError("BrowserContext.new_cdp_session requires cdp-session")
        if not isinstance(cast(object, page), (Page, Frame)):
            raise TypeError("BrowserContext.new_cdp_session requires a Page or Frame")
        if isinstance(page, Frame):
            frame_id: str | None = page._object_id
            owner = page.page
        else:
            frame_id = None
            owner = page
        if owner.context is not self:
            raise InvalidArgumentError("Page or Frame belongs to a different BrowserContext")
        if frame_id is not None and frame_id != owner._main_frame_id:
            raise UnsupportedError("CDPSession for non-main Frame is not installed")
        lease = await self._browser._native.cdp_session_create(
            self._object_id, owner._object_id, frame_id
        )
        if self._closed or owner._closed or getattr(self._browser, "_closed", False):
            # The unclaimed native lease owns cleanup if closure won the
            # create race. Never register a wrapper for a retired owner.
            del lease
            raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
        token = lease.token()
        session = CDPSession(self, owner._object_id, token)
        self._cdp_sessions[token] = session
        # No await separates registry insertion from claiming the native
        # owner. If cancellation discards an unread result, its lease drops
        # and detaches the auxiliary child instead.
        lease.claim()
        ensure_watcher = getattr(self._browser, "_ensure_cdp_event_watcher", None)
        if callable(ensure_watcher):
            ensure_watcher()
        return session

    async def _drain_cdp_sessions(self, page_id: str) -> None:
        for session in list(self._cdp_sessions.values()):
            if session._page_id == page_id:
                await session._drain_events()

    def _retire_cdp_session(self, token: int) -> None:
        session = self._cdp_sessions.pop(token, None)
        if session is None:
            return
        session._closed = True
        session._event_handlers.clear()
        session._once_handlers.clear()
        for future in session._pending_event_handlers:
            if future in session._owned_event_handler_tasks:
                future.cancel()
            else:
                future.remove_done_callback(session._event_handler_done)
        session._pending_event_handlers.clear()
        session._owned_event_handler_tasks.clear()
        task = session._registration_flush_task
        if task is not None and task is not asyncio.current_task():
            task.cancel()
        maybe_stop = getattr(self._browser, "_maybe_stop_cdp_event_watcher", None)
        if callable(maybe_stop):
            maybe_stop()

    def _retire_cdp_sessions(self, page_id: str) -> None:
        for token, session in list(self._cdp_sessions.items()):
            if session._page_id == page_id:
                self._retire_cdp_session(token)
        maybe_stop = getattr(self._browser, "_maybe_stop_cdp_event_watcher", None)
        if callable(maybe_stop):
            maybe_stop()

    def _retire_all_cdp_sessions(self) -> None:
        for token in list(self._cdp_sessions):
            self._retire_cdp_session(token)
        maybe_stop = getattr(self._browser, "_maybe_stop_cdp_event_watcher", None)
        if callable(maybe_stop):
            maybe_stop()

    def _ensure_request_context(self) -> None:
        if self._request_context is not None:
            return
        native = getattr(self._browser, "_native", None)
        factory_value = getattr(native, "context_request", None)
        if "context-api-request" in _native.selected_features() and callable(factory_value):
            factory = cast(
                Callable[[str], _native.NativeApiRequestContext],
                factory_value,
            )
            self._request_context = APIRequestContext(
                None,
                factory(self._object_id),
                self._base_url,
            )

    def _wrap_network_value(self, value: Mapping[str, Any]) -> NetworkValue:
        raw_request_value: object = (
            value.get("request") if value.get("kind") == "response" else value
        )
        raw_request = (
            cast(Mapping[str, Any], raw_request_value)
            if isinstance(raw_request_value, Mapping)
            else None
        )
        page_id_value: object = None if raw_request is None else raw_request.get("page_id")
        page_id = None if page_id_value is None else str(page_id_value)
        page = None if page_id is None else self._pages.get(page_id)
        if page is None and page_id is not None:
            self.pages
            page = self._pages.get(page_id)
        if page is not None:
            if value.get("kind") == "response":
                return page._wrap_response(value)
            return page._wrap_request(value)
        if value.get("kind") == "response":
            return Response(value, self._browser._native, self)
        return Request(value, self._browser._native, self)

    def _effective_timeout(self, value: object) -> int:
        if value is not None:
            return _timeout_ms(value)
        return int(self._browser._native.context_default_timeout(self._object_id, False))

    def _arm_context_event(
        self,
        event: str,
        predicate: PageEventPredicate | None,
        not_before_micros: int = 0,
    ) -> asyncio.Future[object]:
        if sum(len(waiters) for waiters in self._event_waiters.values()) >= (
            _MAX_PAGE_EVENT_WAITERS
        ):
            raise ResourceExhaustedError(
                f"BrowserContext event waiter capacity exceeded ({_MAX_PAGE_EVENT_WAITERS})"
            )
        waiter = asyncio.get_running_loop().create_future()
        self._event_waiters.setdefault(event, []).append((waiter, predicate, not_before_micros))
        return waiter

    def _remove_context_event_waiter(self, event: str, waiter: asyncio.Future[object]) -> None:
        values = self._event_waiters.get(event)
        if values is None:
            return
        self._event_waiters[event] = [value for value in values if value[0] is not waiter]
        if not self._event_waiters[event]:
            self._event_waiters.pop(event, None)

    async def _match_context_event(
        self,
        event: str,
        waiter: asyncio.Future[object],
        predicate: PageEventPredicate,
        value: object,
    ) -> None:
        if waiter.done():
            return
        try:
            matches = predicate(value)
            if inspect.isawaitable(matches):
                matches = await cast(Awaitable[object], matches)
            if bool(matches) and not waiter.done():
                waiter.set_result(value)
                self._remove_context_event_waiter(event, waiter)
        except BaseException as error:
            if not waiter.done():
                waiter.set_exception(error)
                self._remove_context_event_waiter(event, waiter)

    def _emit_context_event(self, event: str, value: object) -> None:
        if event == "close":
            if self._close_emitted:
                return
            self._close_emitted = True
            for pending_event, pending_waiters in list(self._event_waiters.items()):
                if pending_event == "close":
                    continue
                for waiter, _predicate, _not_before_micros in pending_waiters:
                    if not waiter.done():
                        waiter.set_exception(TargetClosedError(_TARGET_CLOSED_MESSAGE))
                self._event_waiters.pop(pending_event, None)
        for waiter, predicate, not_before_micros in list(self._event_waiters.get(event, [])):
            if waiter.done():
                self._remove_context_event_waiter(event, waiter)
            elif (
                event == "console"
                and isinstance(value, ConsoleMessage)
                and value._timestamp_micros < not_before_micros
            ):
                continue
            elif predicate is None:
                waiter.set_result(value)
                self._remove_context_event_waiter(event, waiter)
            else:
                asyncio.create_task(self._match_context_event(event, waiter, predicate, value))
        loop = asyncio.get_running_loop()
        for handler in list(self._event_handlers.get(event, [])):
            try:
                outcome = handler(value)
                if inspect.isawaitable(outcome):
                    asyncio.ensure_future(cast(Awaitable[object], outcome))
            except BaseException as error:
                loop.call_exception_handler(
                    {
                        "message": f"BrowserContext {event!r} event handler failed",
                        "exception": error,
                    }
                )

    def _has_context_event_subscribers(self, event: str) -> bool:
        return any(
            not waiter.done()
            for waiter, _predicate, _not_before_micros in self._event_waiters.get(event, [])
        ) or bool(self._event_handlers.get(event))

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        installed = {"page", "close"}
        if "console-events" in _native.selected_features():
            installed.add("console")
        if "page-dialogs" in _native.selected_features():
            installed.add("dialog")
        if "page-errors" in _native.selected_features():
            installed.add("weberror")
        if event not in installed:
            raise UnsupportedError(f"BrowserContext event {event!r} is not installed")
        if event == "console" and not self._event_handlers.get(event):
            self._console_subscription_token = self._browser._native.subscribe_context_console(
                self._object_id
            )
        self._event_handlers.setdefault(event, []).append(handler)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self.remove_listener(event, invoke)
            return handler(value)

        self.on(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        handlers = self._event_handlers.get(event)
        if handlers is None:
            return
        self._event_handlers[event] = [
            registered for registered in handlers if registered != handler
        ]
        if not self._event_handlers[event]:
            self._event_handlers.pop(event, None)
            token = self._console_subscription_token if event == "console" else None
            if token is not None:
                self._console_subscription_token = None
                with contextlib.suppress(Error):
                    self._browser._native.release_console_subscription(token)

    def expect_event(
        self,
        event: str,
        predicate: NetworkMatcher | PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> EventContextManager[NetworkValue] | BrowserContextEventContextManager:
        timeout_ms = self._effective_timeout(timeout)
        if event in {"request", "response", "requestfinished", "requestfailed"}:
            return EventContextManager(
                self._browser._native,
                self._browser._native.arm_context_network_wait,
                self._object_id,
                self,
                event,
                cast("NetworkMatcher | None", predicate),
                timeout_ms,
            )
        lifecycle_events = {"page", "close"}
        if "console-events" in _native.selected_features():
            lifecycle_events.add("console")
        if "page-dialogs" in _native.selected_features():
            lifecycle_events.add("dialog")
        if "page-errors" in _native.selected_features():
            lifecycle_events.add("weberror")
        if event in lifecycle_events:
            if predicate is not None and not callable(predicate):
                raise TypeError("BrowserContext lifecycle event predicate must be callable")
            return BrowserContextEventContextManager(
                self,
                event,
                cast("PageEventPredicate | None", predicate),
                timeout_ms,
            )
        raise UnsupportedError(f"BrowserContext event {event!r} is not installed")

    def expect_console_message(
        self,
        predicate: PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> BrowserContextEventContextManager:
        if "console-events" not in _native.selected_features():
            raise UnsupportedError("BrowserContext console event capability is not installed")
        if predicate is not None and not callable(predicate):
            raise TypeError("BrowserContext console event predicate must be callable")
        return BrowserContextEventContextManager(
            self, "console", predicate, self._effective_timeout(timeout)
        )

    def expect_page(
        self,
        predicate: PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> BrowserContextEventContextManager:
        event_info = self.expect_event("page", predicate, timeout=timeout)
        if not isinstance(event_info, BrowserContextEventContextManager):
            raise RuntimeError("BrowserContext page event returned an invalid waiter")
        return event_info

    async def wait_for_event(
        self,
        event: str,
        predicate: NetworkMatcher | PageEventPredicate | None = None,
        *,
        timeout: float | None = None,
    ) -> NetworkValue | object:
        event_info = self.expect_event(event, predicate, timeout=timeout)
        await event_info.__aenter__()
        try:
            value = await event_info.value
        except BaseException as error:
            await event_info.__aexit__(type(error), error, error.__traceback__)
            raise
        await event_info.__aexit__(None, None, None)
        return value

    @property
    def pages(self) -> list[Page]:
        if self._closed:
            return []
        seen: dict[str, Page] = {}
        for object_id, _context_id, main_frame_id, url in self._browser._native.pages(
            self._object_id
        ):
            page = self._pages.get(object_id)
            if page is None:
                page = Page(self, object_id, main_frame_id, url)
            else:
                page._main_frame_id = main_frame_id
                page._url = url
            seen[object_id] = page
        for object_id, page in self._pages.items():
            if object_id not in seen:
                page._closed = True
        self._pages = seen
        return list(seen.values())

    async def new_page(self) -> Page:
        object_id, _context_id, main_frame_id, url = await self._browser._native.new_page(
            self._object_id
        )
        page = self._pages.get(object_id)
        if page is None:
            page = Page(self, object_id, main_frame_id, url)
        else:
            page._main_frame_id = main_frame_id
            page._url = url
        self._pages[object_id] = page
        return page

    def set_default_timeout(self, timeout: float) -> None:
        self._browser._native.set_context_default_timeout(
            self._object_id, False, _timeout_ms(timeout)
        )

    def set_default_navigation_timeout(self, timeout: float) -> None:
        self._browser._native.set_context_default_timeout(
            self._object_id, True, _timeout_ms(timeout)
        )

    async def cookies(self, urls: str | Sequence[str] | None = None) -> list[Cookie]:
        if urls is None:
            url_values: list[str] = []
        elif isinstance(urls, str):
            value, _size = _bounded_cookie_string(
                urls,
                "BrowserContext.cookies url",
                "cookie URL",
                _MAX_COOKIE_URL_BYTES,
            )
            url_values = [value]
        else:
            if len(urls) > 256:
                raise ResourceExhaustedError("cookie URL count exceeds 256")
            url_values = []
            total_bytes = 0
            for url in urls:
                value, size = _bounded_cookie_string(
                    url,
                    "BrowserContext.cookies url",
                    "cookie URL",
                    _MAX_COOKIE_URL_BYTES,
                )
                total_bytes += size
                if total_bytes > _MAX_COOKIE_URL_BATCH_BYTES:
                    raise ResourceExhaustedError(
                        f"cookie URL batch exceeds {_MAX_COOKIE_URL_BATCH_BYTES} bytes"
                    )
                url_values.append(value)
        native_values = await self._browser._native.context_cookies(self._object_id, url_values)
        values: list[Cookie] = []
        for (
            name,
            value,
            domain,
            path,
            expires,
            http_only,
            secure,
            same_site,
            partition_key,
        ) in native_values:
            cookie: Cookie = {
                "name": name,
                "value": value,
                "domain": domain,
                "path": path,
                "expires": expires,
                "httpOnly": http_only,
                "secure": secure,
                "sameSite": same_site,
            }
            if partition_key is not None:
                cookie["partitionKey"] = partition_key
            values.append(cookie)
        return values

    async def add_cookies(self, cookies: Sequence[SetCookieParam]) -> None:
        await self._browser._native.context_add_cookies(self._object_id, _cookie_inputs(cookies))

    async def storage_state(
        self,
        *,
        path: str | os.PathLike[str] | None = None,
        indexed_db: bool = False,
    ) -> StorageState:
        checked_indexed_db = _boolean_argument(
            indexed_db, "BrowserContext.storage_state indexed_db"
        )
        _require_context_storage_state()
        state = cast(
            StorageState,
            await self._browser._native.context_storage_state(self._object_id, checked_indexed_db),
        )
        if path is not None:
            await asyncio.to_thread(Path(path).write_text, json.dumps(state, indent=2), "utf-8")
        return state

    async def clear_cookies(
        self,
        *,
        name: str | Pattern[str] | None = None,
        domain: str | Pattern[str] | None = None,
        path: str | Pattern[str] | None = None,
    ) -> None:
        name_source, name_flags = (
            (None, None) if name is None else _text_match(name, "BrowserContext.clear_cookies name")
        )
        domain_source, domain_flags = (
            (None, None)
            if domain is None
            else _text_match(domain, "BrowserContext.clear_cookies domain")
        )
        path_source, path_flags = (
            (None, None) if path is None else _text_match(path, "BrowserContext.clear_cookies path")
        )
        await self._browser._native.context_clear_cookies(
            self._object_id,
            name_source,
            name_flags,
            domain_source,
            domain_flags,
            path_source,
            path_flags,
        )

    async def add_init_script(
        self,
        script: str | None = None,
        *,
        path: str | Path | None = None,
    ) -> None:
        source = await _init_script_source(script, path, source_url=False)
        await self._browser._native.context_add_init_script(self._object_id, source)

    async def _register_binding(
        self,
        name: str,
        callback: Callable[..., object],
        needs_handle: bool,
        api_name: str,
    ) -> None:
        if "context-bindings" not in _native.selected_features():
            raise UnsupportedError("BrowserContext bindings capability is not installed")
        if not name:
            raise TypeError(f"{api_name}: name must be a non-empty string")

        async def register() -> None:
            registration_lock = self._binding_registration_lock
            if registration_lock is None:
                registration_lock = asyncio.Lock()
                self._binding_registration_lock = registration_lock
            async with registration_lock:
                if name in self._bindings:
                    raise Error(f'Function "{name}" has been already registered')
                binding = (callback, needs_handle)
                self._bindings[name] = binding
                try:
                    await self._browser._native.context_expose_binding(
                        self._object_id, name, needs_handle
                    )
                except BaseException:
                    if self._bindings.get(name) is binding:
                        self._bindings.pop(name, None)
                    raise

        await _shield_binding_registration(self._browser, register, api_name)

    async def expose_binding(
        self,
        name: str,
        callback: Callable[..., object],
        *,
        handle: bool | None = None,
    ) -> None:
        if not callable(callback):
            raise TypeError("BrowserContext.expose_binding: callback must be callable")
        await self._register_binding(
            name,
            _wrap_handler(callback),
            handle is True,
            "BrowserContext.expose_binding",
        )

    async def expose_function(self, name: str, callback: Callable[..., object]) -> None:
        if not callable(callback):
            raise TypeError("BrowserContext.expose_function: callback must be callable")
        wrapped_callback = _wrap_handler(callback)

        def invoke(_source: object, *args: object) -> object:
            return wrapped_callback(*args)

        await self._register_binding(name, invoke, False, "BrowserContext.expose_function")

    async def set_extra_http_headers(self, headers: Mapping[str, str]) -> None:
        await self._browser._native.set_context_extra_http_headers(
            self._object_id,
            _extra_http_headers(headers, "BrowserContext.set_extra_http_headers"),
        )

    async def set_offline(self, offline: bool) -> None:
        await self._browser._native.set_context_offline(
            self._object_id,
            _boolean_argument(offline, "BrowserContext.set_offline offline"),
        )

    async def route(
        self,
        url: RouteURLMatcher,
        handler: Callable[..., object],
        *,
        times: int | None = None,
    ) -> None:
        await self._route_with_key(url, handler, handler, times=times)

    async def _route_with_key(
        self,
        url: RouteURLMatcher,
        handler: Callable[..., object],
        handler_key: object,
        *,
        times: int | None = None,
    ) -> None:
        if not callable(handler):
            raise TypeError("BrowserContext.route: handler must be callable")
        if not isinstance(url, (str, Pattern)) and not callable(url):
            raise TypeError(
                "BrowserContext.route: url must be a string, compiled regex, or callable"
            )
        times = _route_times(times, "BrowserContext.route")

        route_handler: Callable[..., object] = handler
        native_pattern = url
        native_times = times
        binding_handler: _BindingRouteHandler | None = None
        if not isinstance(url, str):
            native_pattern = "**/*"
            native_times = None

            def discard(registration: str) -> None:
                self._routes.pop(registration, None)
                self._route_patterns.pop(registration, None)
                self._route_handler_keys.pop(registration, None)

            async def retire(registration: str) -> None:
                await self._browser._native.context_unroute(
                    self._object_id,
                    [int(registration)],
                )

            binding_handler = _BindingRouteHandler(
                url,
                handler,
                times,
                discard,
                retire,
            )
            route_handler = binding_handler
        registration = str(
            await self._browser._native.context_route(
                self._object_id,
                cast(str, native_pattern),
                native_times,
            )
        )
        if binding_handler is not None:
            binding_handler.bind_registration(registration)
        self._routes[registration] = route_handler
        self._route_patterns[registration] = url
        self._route_handler_keys[registration] = handler_key

    async def unroute(
        self,
        url: RouteURLMatcher,
        handler: Callable[..., object] | None = None,
    ) -> None:
        if not isinstance(url, (str, Pattern)) and not callable(url):
            raise TypeError(
                "BrowserContext.unroute: url must be a string, compiled regex, or callable"
            )
        registrations = [
            registration
            for registration, pattern in self._route_patterns.items()
            if pattern == url
            and (handler is None or self._route_handler_keys.get(registration) is handler)
        ]
        if registrations:
            await self._browser._native.context_unroute(
                self._object_id,
                [int(registration) for registration in registrations],
            )
        for registration in registrations:
            self._routes.pop(registration, None)
            self._route_patterns.pop(registration, None)
            self._route_handler_keys.pop(registration, None)

    async def route_from_har(
        self,
        har: str | Path,
        *,
        url: str | Pattern[str] | Callable[[str], bool] | None = None,
        not_found: Literal["abort", "fallback"] | None = None,
        update: bool | None = None,
        update_content: Literal["attach", "embed"] | None = None,
        update_mode: Literal["full", "minimal"] | None = None,
    ) -> None:
        if update:
            raise UnsupportedError("BrowserContext.route_from_har(update=True) is not installed")
        if update_content is not None or update_mode is not None:
            raise UnsupportedError(
                "BrowserContext.route_from_har update_content and update_mode require update=True"
            )
        if url is not None and not isinstance(url, str):
            raise UnsupportedError(
                "BrowserContext.route_from_har regex and predicate URL filters are not installed"
            )
        await self._browser._native.context_route_from_har(
            self._object_id,
            str(har),
            url,
            not_found,
        )

    async def unroute_all(
        self,
        *,
        behavior: Literal["default", "ignoreErrors", "wait"] | None = None,
    ) -> None:
        if behavior not in (None, "default", "wait"):
            raise UnsupportedError(
                "BrowserContext.unroute_all(behavior='ignoreErrors') is not installed"
            )
        await self._browser._native.context_unroute_all(
            self._object_id,
            behavior == "wait",
        )
        self._routes.clear()
        self._route_patterns.clear()
        self._route_handler_keys.clear()

    async def close(self, *, reason: str | None = None) -> None:
        if self._closed:
            return
        self._ensure_request_context()
        if self._request_context is not None:
            self._request_context._dispose_from_owner(reason)
        await self._browser._native.close_context(self._object_id)
        self._retire_all_cdp_sessions()
        self._closed = True
        self._routes.clear()
        self._route_patterns.clear()
        self._route_handler_keys.clear()
        self._bindings.clear()
        for page in self._pages.values():
            page._retire_workers()
            page._closed = True
        if "page-events" not in _native.selected_features():
            self._pages.clear()
            self._browser._contexts.pop(self._object_id, None)
        if self._browser._persistent_context:
            await self._browser.close()


class RemotePage(Page):
    """A Page identity backed by a typed remote GUID, not the CDP Page host."""

    def __init__(self, context: RemoteBrowserContext, native: _native.NativeRemotePage) -> None:
        self._context = context
        self._object_id = native.guid
        self._remote_native = native
        self._remote_close_handlers: list[Callable[[object], object]] = []
        self._remote_close_emitted = False
        self._closes_context = False
        context._browser._register_remote_page(self)

    def __getattribute__(self, name: str) -> Any:
        if not name.startswith("_") and name not in {
            "context",
            "is_closed",
            "close",
            "title",
            "on",
            "once",
            "remove_listener",
        }:
            if inspect.iscoroutinefunction(inspect.getattr_static(Page, name, None)):

                async def reject_remote_operation(*_args: object, **_kwargs: object) -> None:
                    browser = self._context._browser
                    native = browser._remote_native
                    if native is not None and not native.is_connected():
                        await browser._flush_remote_disconnect()
                    if browser._closed or self.is_closed():
                        raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
                    raise UnsupportedError(f"Page.{name} remote operation is not installed")

                return reject_remote_operation
            if self.is_closed():
                raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
            raise UnsupportedError(f"Page.{name} remote operation is not installed")
        return object.__getattribute__(self, name)

    def __repr__(self) -> str:
        return f"<RemotePage guid={self._object_id!r}>"

    @property
    def context(self) -> BrowserContext:
        return self._context

    def is_closed(self) -> bool:
        return self._remote_close_emitted or not self._remote_native.is_open()

    async def title(self) -> str:
        browser = self._context._browser
        native_browser = browser._remote_native
        if native_browser is not None and not native_browser.is_connected():
            await browser._flush_remote_disconnect()
        if browser._closed or self.is_closed():
            raise TargetClosedError(_TARGET_CLOSED_MESSAGE)
        if "playwright-server-frame-title" not in _native.selected_features():
            raise UnsupportedError("Page.title remote operation is not installed")
        try:
            return str(await self._remote_native.title())
        except Exception as error:
            if native_browser is not None and not native_browser.is_connected():
                await browser._flush_remote_disconnect()
            if browser._closed or self.is_closed():
                raise TargetClosedError(_TARGET_CLOSED_MESSAGE) from error
            raise

    def _mark_remote_closed(self) -> None:
        if self._remote_close_emitted:
            return
        self._remote_close_emitted = True
        self._context._browser._unregister_remote_page(self)
        for handler in list(self._remote_close_handlers):
            try:
                outcome = handler(self)
                if inspect.isawaitable(outcome):
                    asyncio.ensure_future(cast(Awaitable[object], outcome))
            except BaseException as error:
                asyncio.get_running_loop().call_exception_handler(
                    {"message": "Page 'close' handler failed", "exception": error}
                )

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        if event != "close":
            raise UnsupportedError(f"Page event {event!r} remote operation is not installed")
        if len(self._remote_close_handlers) >= 64:
            raise ResourceExhaustedError("Page close handler capacity exceeded (64)")
        self._remote_close_handlers.append(handler)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self.remove_listener(event, invoke)
            return handler(value)

        self.on(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        if event == "close":
            self._remote_close_handlers = [
                registered for registered in self._remote_close_handlers if registered != handler
            ]

    async def close(self, *, run_before_unload: bool = False, reason: str | None = None) -> None:
        if run_before_unload or reason is not None:
            raise UnsupportedError("Page.close remote options are not installed")
        if not self._closes_context:
            raise UnsupportedError("Page.close on a remote Context-owned Page is not installed")
        await self._context.close()


class RemoteBrowserContext(BrowserContext):
    """A Context identity whose liveness comes from the remote GUID graph."""

    def __init__(self, browser: Browser, native: _native.NativeRemoteContext) -> None:
        self._browser = browser
        self._object_id = native.guid
        self._remote_native = native
        self._remote_pages: dict[str, RemotePage] = {}
        self._remote_page_registry: dict[tuple[str, int], RemotePage] = {}
        self._remote_close_handlers: list[Callable[[object], object]] = []
        self._remote_close_emitted = False
        browser._register_remote_context(self)

    def __getattribute__(self, name: str) -> Any:
        if not name.startswith("_") and name not in {
            "browser",
            "pages",
            "new_page",
            "is_closed",
            "close",
            "on",
            "once",
            "remove_listener",
        }:
            raise UnsupportedError(f"BrowserContext.{name} remote operation is not installed")
        return object.__getattribute__(self, name)

    def __repr__(self) -> str:
        return f"<RemoteBrowserContext guid={self._object_id!r}>"

    @property
    def browser(self) -> Browser:
        return self._browser

    def is_closed(self) -> bool:
        return self._remote_close_emitted or not self._remote_native.is_open()

    def _mark_remote_closed(self) -> None:
        if self._remote_close_emitted:
            return
        self._remote_close_emitted = True
        if self._browser._remote_contexts.get(self._object_id) is self:
            del self._browser._remote_contexts[self._object_id]
        for page in sorted(
            self._remote_page_registry.values(),
            key=lambda current: current._remote_native.created_sequence,
        ):
            page._mark_remote_closed()
        self._browser._unregister_remote_context(self)
        for handler in list(self._remote_close_handlers):
            try:
                outcome = handler(self)
                if inspect.isawaitable(outcome):
                    asyncio.ensure_future(cast(Awaitable[object], outcome))
            except BaseException as error:
                asyncio.get_running_loop().call_exception_handler(
                    {"message": "BrowserContext 'close' handler failed", "exception": error}
                )

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        if event != "close":
            raise UnsupportedError(
                f"BrowserContext event {event!r} remote operation is not installed"
            )
        if len(self._remote_close_handlers) >= 64:
            raise ResourceExhaustedError("BrowserContext close handler capacity exceeded (64)")
        self._remote_close_handlers.append(handler)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self.remove_listener(event, invoke)
            return handler(value)

        self.on(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        if event == "close":
            self._remote_close_handlers = [
                registered for registered in self._remote_close_handlers if registered != handler
            ]

    @property
    def pages(self) -> list[Page]:
        live = self._remote_native.live_pages()
        known = self._remote_pages
        pages: list[RemotePage] = []
        for guid, sequence in live:
            page = known.get(guid)
            if page is None or page._remote_native.created_sequence != sequence or page.is_closed():
                raise UnsupportedError(
                    "BrowserContext.pages contains a remote Page without an installed wrapper"
                )
            pages.append(page)
        self._remote_pages = {page._object_id: page for page in pages}
        return cast(list[Page], pages)

    async def new_page(self) -> Page:
        if self.is_closed():
            raise TargetClosedError("BrowserContext has been closed")
        browser = self._browser
        browser._remote_creation_started("page")
        try:
            native = await self._remote_native.new_page()
            page = RemotePage(self, native)
        finally:
            browser._remote_creation_finished("page")
        self._remote_pages = {
            guid: current for guid, current in self._remote_pages.items() if not current.is_closed()
        }
        self._remote_pages[page._object_id] = page
        return page

    async def close(self, *, reason: str | None = None) -> None:
        if reason is not None:
            raise UnsupportedError(
                "BrowserContext.close(reason=...) remote operation is not installed"
            )
        if self._remote_close_emitted:
            return
        if self._remote_native.is_open():
            native_close = getattr(self._remote_native, "close", None)
            if not callable(native_close):
                raise UnsupportedError("BrowserContext.close requires a rebuilt native extension")
            try:
                await cast(Callable[[], Awaitable[None]], native_close)()
            except BaseException:
                if not self._remote_close_emitted:
                    raise
        self._mark_remote_closed()


class Browser:
    def __init__(
        self,
        browser_type: BrowserType,
        native: _native.NativeSession | _native.NativeRemoteBrowser,
        *,
        remote: bool = False,
        fresh_owned: bool = False,
        persistent_context: bool = False,
        persistent_context_viewport: tuple[int, int] | None = None,
    ) -> None:
        self._browser_type = browser_type
        self._native = cast(_native.NativeSession, native)
        self._remote = remote
        self._remote_native = cast("_native.NativeRemoteBrowser", native) if remote else None
        self._remote_event_handlers: dict[str, list[Callable[[object], object]]] = {}
        self._remote_disconnect_task: asyncio.Task[None] | None = None
        self._remote_disconnect_emitted = False
        self._remote_close_notices = remote and (
            "playwright-server-close-notices" in _native.selected_features()
        )
        self._remote_context_registry: dict[tuple[str, int], RemoteBrowserContext] = {}
        self._remote_page_registry: dict[tuple[str, int], RemotePage] = {}
        self._remote_pending_context_creations = 0
        self._remote_pending_page_creations = 0
        self._remote_creation_event = asyncio.Event()
        self._remote_terminal_event = asyncio.Event()
        self._fresh_owned = fresh_owned
        self._persistent_context = persistent_context
        self._persistent_context_viewport = persistent_context_viewport
        self._contexts: dict[str, BrowserContext] = {}
        self._remote_contexts: dict[str, RemoteBrowserContext] = {}
        self._pending_context_ids: set[str] = set()
        self._pending_context_cleanup_tasks: dict[str, asyncio.Task[None]] = {}
        self._unidentified_storage_state_creations = 0
        self._closed = False
        self._binding_watch_error: BaseException | None = None
        self._binding_task: asyncio.Task[None] | None = None
        self._binding_dispatch_tasks: set[asyncio.Task[None]] = set()
        self._binding_registration_tasks: set[asyncio.Task[None]] = set()
        self._binding_closing_dispatch: asyncio.Task[None] | None = None
        self._route_watch_error: BaseException | None = None
        self._deferred_route_error: BaseException | None = None
        self._route_task: asyncio.Task[None] | None = None
        self._lifecycle_watch_error: BaseException | None = None
        self._lifecycle_task: asyncio.Task[None] | None = None
        self._request_watch_error: BaseException | None = None
        self._request_task: asyncio.Task[None] | None = None
        self._response_watch_error: BaseException | None = None
        self._response_task: asyncio.Task[None] | None = None
        self._request_notice_sequence = 0
        self._response_notice_sequence = 0
        self._network_notice_condition = asyncio.Condition()
        self._download_watch_error: BaseException | None = None
        self._download_task: asyncio.Task[None] | None = None
        self._worker_watch_error: BaseException | None = None
        self._worker_task: asyncio.Task[None] | None = None
        self._worker_console_watch_error: BaseException | None = None
        self._worker_console_task: asyncio.Task[None] | None = None
        self._cdp_watch_error: BaseException | None = None
        self._cdp_event_task: asyncio.Task[None] | None = None
        self._cdp_registration_tasks: set[asyncio.Task[None]] = set()
        self._cdp_detach_tasks: set[asyncio.Task[None]] = set()
        self._loop = asyncio.get_running_loop()
        features = [] if remote else _native.selected_features()
        if "page-events" in features:
            self._native.set_lifecycle_callback(self._schedule_lifecycle_notice)
            self._lifecycle_task = asyncio.create_task(self._watch_lifecycle_notices())
        if "page-bindings" in features:
            self._binding_task = asyncio.create_task(self._watch_page_bindings())
        if "page-routes" in features:
            self._route_task = asyncio.create_task(self._watch_page_routes())
        if "request-events" in features:
            self._request_task = asyncio.create_task(self._watch_request_notices())
        if "response-events" in features:
            self._response_task = asyncio.create_task(self._watch_response_notices())
        if "download-events" in features:
            self._download_task = asyncio.create_task(self._watch_download_notices())
        if "worker-events" in features:
            self._worker_task = asyncio.create_task(self._watch_worker_notices())
        if "worker-console-events" in features:
            self._worker_console_task = asyncio.create_task(self._watch_worker_console_notices())
        if remote:
            watch = (
                self._watch_remote_close_notices()
                if self._remote_close_notices
                else self._watch_remote_disconnect()
            )
            self._remote_disconnect_task = asyncio.create_task(watch)

    def _register_remote_context(self, context: RemoteBrowserContext) -> None:
        key = (context._object_id, context._remote_native.created_sequence)
        previous = self._remote_context_registry.get(key)
        if previous is not None and previous is not context:
            raise RuntimeError("Duplicate remote Context identity")
        self._remote_context_registry[key] = context

    def _unregister_remote_context(self, context: RemoteBrowserContext) -> None:
        key = (context._object_id, context._remote_native.created_sequence)
        if self._remote_context_registry.get(key) is context:
            del self._remote_context_registry[key]

    def _register_remote_page(self, page: RemotePage) -> None:
        key = (page._object_id, page._remote_native.created_sequence)
        previous = self._remote_page_registry.get(key)
        if previous is not None and previous is not page:
            raise RuntimeError("Duplicate remote Page identity")
        self._remote_page_registry[key] = page
        page._context._remote_page_registry[key] = page

    def _unregister_remote_page(self, page: RemotePage) -> None:
        key = (page._object_id, page._remote_native.created_sequence)
        if self._remote_page_registry.get(key) is page:
            del self._remote_page_registry[key]
        if page._context._remote_page_registry.get(key) is page:
            del page._context._remote_page_registry[key]
        if page._context._remote_pages.get(page._object_id) is page:
            del page._context._remote_pages[page._object_id]

    def _remote_creation_started(self, kind: Literal["page", "context"]) -> None:
        if kind == "page":
            self._remote_pending_page_creations += 1
        else:
            self._remote_pending_context_creations += 1

    def _remote_creation_finished(self, kind: Literal["page", "context"]) -> None:
        if kind == "page":
            self._remote_pending_page_creations -= 1
        else:
            self._remote_pending_context_creations -= 1
        self._remote_creation_event.set()
        self._remote_creation_event = asyncio.Event()

    async def _await_remote_wrapper(
        self, kind: Literal["page", "context"], key: tuple[str, int]
    ) -> RemotePage | RemoteBrowserContext | None:
        registry: Mapping[tuple[str, int], RemotePage | RemoteBrowserContext] = (
            self._remote_page_registry if kind == "page" else self._remote_context_registry
        )
        deadline = self._loop.time() + _REMOTE_CREATION_FENCE_TIMEOUT_SECONDS
        while True:
            wrapper = registry.get(key)
            if wrapper is not None:
                return wrapper
            pending = (
                self._remote_pending_page_creations
                if kind == "page"
                else self._remote_pending_context_creations
            )
            if pending == 0:
                return None
            await asyncio.wait_for(
                self._remote_creation_event.wait(),
                timeout=max(0.0, deadline - self._loop.time()),
            )

    def _close_remote_children(self) -> None:
        for context in sorted(
            self._remote_context_registry.values(),
            key=lambda current: current._remote_native.created_sequence,
        ):
            context._mark_remote_closed()
        self._remote_contexts.clear()

    def _mark_remote_browser_closed(self) -> None:
        self._closed = True
        self._close_remote_children()

    def _emit_remote_event(self, event: str, value: object) -> None:
        for handler in list(self._remote_event_handlers.get(event, [])):
            try:
                outcome = handler(value)
                if inspect.isawaitable(outcome):
                    asyncio.ensure_future(cast(Awaitable[object], outcome))
            except BaseException as error:
                self._loop.call_exception_handler(
                    {"message": f"Browser {event!r} handler failed", "exception": error}
                )

    def _mark_remote_disconnected(self) -> None:
        if self._remote_disconnect_emitted:
            return
        self._remote_disconnect_emitted = True
        self._closed = True
        self._close_remote_children()
        self._remote_terminal_event.set()
        with contextlib.suppress(ValueError):
            self._browser_type._playwright._browsers.remove(self)
        self._emit_remote_event("disconnected", self)

    async def _watch_remote_disconnect(self) -> None:
        native = self._remote_native
        assert native is not None
        try:
            await native.wait_disconnected()
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._loop.call_exception_handler(
                {"message": "Browser disconnected watch failed", "exception": error}
            )
            if native.is_connected():
                return
        self._mark_remote_disconnected()

    async def _watch_remote_close_notices(self) -> None:
        native = self._remote_native
        assert native is not None
        try:
            while not self._remote_disconnect_emitted:
                kind, guid, generation = await native.next_close_notice()
                if kind == "disconnected":
                    if guid is not None or generation is not None:
                        raise ValueError("Malformed remote disconnected notice")
                    self._mark_remote_disconnected()
                    return
                if not isinstance(guid, str) or not isinstance(generation, int):
                    raise ValueError("Malformed remote close notice identity")
                if kind == "browser":
                    if guid != native.guid:
                        raise ValueError("Remote Browser close notice has a foreign identity")
                    self._mark_remote_browser_closed()
                elif kind in ("page", "context"):
                    wrapper = await self._await_remote_wrapper(kind, (guid, generation))
                    if wrapper is not None:
                        wrapper._mark_remote_closed()
                else:
                    raise ValueError(f"Unknown remote close notice kind: {kind!r}")
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._loop.call_exception_handler(
                {"message": "Browser remote close notice watch failed", "exception": error}
            )
            self._mark_remote_disconnected()

    async def _flush_remote_disconnect(self) -> None:
        if not self._remote_close_notices:
            self._mark_remote_disconnected()
            return
        if self._remote_terminal_event.is_set():
            return
        task = self._remote_disconnect_task
        if task is None or task.done():
            self._mark_remote_disconnected()
            return
        try:
            await asyncio.wait_for(
                self._remote_terminal_event.wait(),
                timeout=_REMOTE_CREATION_FENCE_TIMEOUT_SECONDS,
            )
        except asyncio.TimeoutError:
            self._mark_remote_disconnected()

    async def _cancel_remote_disconnect_watcher(self) -> None:
        task = self._remote_disconnect_task
        self._remote_disconnect_task = None
        if task is None or task is asyncio.current_task():
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    def on(self, event: str, handler: Callable[[object], object]) -> None:
        if not self._remote or event != "disconnected":
            raise UnsupportedError(f"Browser event {event!r} is not installed")
        if len(self._remote_event_handlers.get(event, [])) >= 64:
            raise ResourceExhaustedError("Browser disconnected handler capacity exceeded (64)")
        self._remote_event_handlers.setdefault(event, []).append(handler)

    def once(self, event: str, handler: Callable[[object], object]) -> None:
        def invoke(value: object) -> object:
            self.remove_listener(event, invoke)
            return handler(value)

        self.on(event, invoke)

    def remove_listener(self, event: str, handler: Callable[[object], object]) -> None:
        handlers = self._remote_event_handlers.get(event)
        if handlers is None:
            return
        remaining = [registered for registered in handlers if registered != handler]
        if remaining:
            self._remote_event_handlers[event] = remaining
        else:
            self._remote_event_handlers.pop(event, None)

    def _has_cdp_sessions(self) -> bool:
        return any(context._cdp_sessions for context in self._contexts.values())

    def _ensure_cdp_event_watcher(self) -> None:
        if self._closed or self._cdp_watch_error is not None or not self._has_cdp_sessions():
            return
        task = self._cdp_event_task
        if task is not None and not task.done():
            return
        task = asyncio.create_task(self._watch_cdp_session_events())
        self._cdp_event_task = task
        task.add_done_callback(self._cdp_event_watcher_done)

    def _cdp_event_watcher_done(self, task: asyncio.Task[None]) -> None:
        if self._cdp_event_task is task:
            self._cdp_event_task = None
        if not task.cancelled():
            error = task.exception()
            if error is not None:
                self._record_cdp_watch_error(error)
        # Final-session retirement may cancel the old Watch just as a new
        # child is registered. Restart only after the old task has exited.
        self._ensure_cdp_event_watcher()

    def _maybe_stop_cdp_event_watcher(self) -> None:
        if self._has_cdp_sessions():
            return
        task = self._cdp_event_task
        if task is not None:
            task.cancel()

    def _record_cdp_watch_error(self, error: BaseException) -> None:
        if self._cdp_watch_error is not None or self._closed:
            return
        self._cdp_watch_error = error
        task = self._cdp_event_task
        if task is not None and task is not asyncio.current_task():
            task.cancel()
        self._loop.call_exception_handler(
            {"message": "CDPSession event delivery stopped", "exception": error}
        )

    def _schedule_cdp_registration_flush(self, session: CDPSession) -> None:
        if self._closed or self._cdp_watch_error is not None or session._closed:
            return
        task = session._registration_flush_task
        if task is not None and not task.done():
            return
        task = asyncio.create_task(self._flush_registered_cdp_session(session))
        session._registration_flush_task = task
        self._cdp_registration_tasks.add(task)

        def done(completed: asyncio.Task[None]) -> None:
            self._cdp_registration_tasks.discard(completed)
            if session._registration_flush_task is completed:
                session._registration_flush_task = None

        task.add_done_callback(done)

    async def _flush_registered_cdp_session(self, session: CDPSession) -> None:
        try:
            await session._drain_events()
        except TargetClosedError:
            if session._detach_task is None:
                session._context._retire_cdp_session(session._token)
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._record_cdp_watch_error(error)

    async def _watch_cdp_session_events(self) -> None:
        generation = 0
        try:
            while not self._closed and self._has_cdp_sessions():
                try:
                    next_generation = await self._native.next_cdp_session_event_generation(
                        generation
                    )
                except TargetClosedError as error:
                    self._cdp_watch_error = error
                    return
                if next_generation == generation:
                    raise Error("CDPSession event generation did not advance")
                generation = next_generation
                if self._closed:
                    return
                for context in tuple(self._contexts.values()):
                    for session in tuple(context._cdp_sessions.values()):
                        try:
                            await session._drain_events()
                        except TargetClosedError:
                            # A retired child must not keep the Python watcher
                            # alive or stop delivery to siblings.
                            if session._detach_task is None:
                                context._retire_cdp_session(session._token)
                            continue
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._record_cdp_watch_error(error)

    async def _cancel_cdp_event_watcher(self) -> None:
        tasks = set(self._cdp_registration_tasks)
        if self._cdp_event_task is not None:
            tasks.add(self._cdp_event_task)
        for task in tasks:
            task.cancel()
        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)
        self._cdp_event_task = None
        self._cdp_registration_tasks.clear()

    async def _settle_cdp_detach_tasks(self) -> None:
        tasks = tuple(self._cdp_detach_tasks)
        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)

    async def __aenter__(self) -> Browser:
        return self

    async def __aexit__(self, *_exc: object) -> None:
        await self.close()

    def __repr__(self) -> str:
        return f"<Browser type={self.browser_type} version={self.version}>"

    def _schedule_lifecycle_notice(
        self,
        kind: int,
        context_id: str,
        page_id: str | None,
        opener_id: str | None,
        base_url: str | None,
        main_frame_id: str | None,
        url: str | None,
        error_descriptor: tuple[str, str, str] | None,
        console_descriptor: tuple[
            str,
            str,
            str,
            str,
            int,
            int,
            int,
            list[_native.NativeHandle],
        ]
        | None,
        dialog_descriptor: tuple[_native.NativeDialog, str, str, str] | None = None,
        file_chooser_descriptor: tuple[_native.NativeFileChooser, bool] | None = None,
        frame_id: str | None = None,
    ) -> None:
        self._loop.call_soon_threadsafe(
            self._receive_lifecycle_notice,
            kind,
            context_id,
            page_id,
            opener_id,
            base_url,
            main_frame_id,
            url,
            error_descriptor,
            console_descriptor,
            dialog_descriptor,
            file_chooser_descriptor,
            frame_id,
        )

    def _notice_context(
        self, context_id: str, base_url: str | None, *, create: bool
    ) -> BrowserContext | None:
        if context_id in getattr(self, "_pending_context_ids", set[str]()):
            return None
        context = self._contexts.get(context_id)
        if context is None and create and not self._closed:
            context = BrowserContext(self, context_id, base_url)
            self._contexts[context_id] = context
        return context

    @staticmethod
    def _notice_page(
        context: BrowserContext,
        page_id: str,
        main_frame_id: str | None,
        url: str | None,
        *,
        create: bool,
    ) -> Page | None:
        page = context._pages.get(page_id)
        if page is None and create and main_frame_id is not None and url is not None:
            page = Page(context, page_id, main_frame_id, url)
            context._pages[page_id] = page
        elif page is not None:
            if main_frame_id is not None:
                page._main_frame_id = main_frame_id
            if url is not None:
                page._url = url
        return page

    def _receive_lifecycle_notice(
        self,
        kind: int,
        context_id: str,
        page_id: str | None,
        opener_id: str | None,
        base_url: str | None,
        main_frame_id: str | None,
        url: str | None,
        error_descriptor: tuple[str, str, str] | None,
        console_descriptor: (
            tuple[str, str, str, str, int, int]
            | tuple[str, str, str, str, int, int, int]
            | tuple[
                str,
                str,
                str,
                str,
                int,
                int,
                int,
                list[_native.NativeHandle],
            ]
            | None
        ),
        dialog_descriptor: tuple[_native.NativeDialog, str, str, str] | None = None,
        file_chooser_descriptor: tuple[_native.NativeFileChooser, bool] | None = None,
        frame_id: str | None = None,
    ) -> None:
        context = self._notice_context(context_id, base_url, create=kind in {2, 3})
        if kind == 1:
            if context is None:
                return
            context._ensure_request_context()
            for page in list(context._pages.values()):
                page._retire_workers()
                page._closed = True
                page._emit_page_event("close", page)
            context._pages.clear()
            context._closed = True
            if context._request_context is not None:
                context._request_context._dispose_from_owner(None)
            context._emit_context_event("close", context)
            self._contexts.pop(context_id, None)
            return
        if context is None or page_id is None:
            return
        if kind == 0:
            page = self._notice_page(context, page_id, main_frame_id, url, create=False)
            if page is None:
                return
            page._retire_workers()
            page._closed = True
            page._emit_page_event("close", page)
            context._pages.pop(page_id, None)
            return
        if kind == 2:
            page = self._notice_page(context, page_id, main_frame_id, url, create=True)
            if page is not None:
                context._emit_context_event("page", page)
            return
        if kind == 3:
            popup = self._notice_page(context, page_id, main_frame_id, url, create=True)
            opener = context._pages.get(opener_id) if opener_id is not None else None
            if popup is None:
                return
            context._emit_context_event("page", popup)
            if opener is None:
                return
            popup._opener = opener
            opener._emit_page_event("popup", popup)
            return
        page = self._notice_page(context, page_id, main_frame_id, url, create=False)
        if page is None:
            return
        if kind == 4:
            page._emit_page_event("domcontentloaded", page)
        elif kind == 5:
            page._emit_page_event("load", page)
        elif kind == 7 and "frame-events" in _native.selected_features():
            page._emit_page_event("framenavigated", page.main_frame)
        elif (
            kind in {11, 12}
            and frame_id is not None
            and "frame-events" in _native.selected_features()
        ):
            frame = next(
                (value for value in page._refresh_frames() if value._object_id == frame_id), None
            )
            if frame is not None:
                if kind == 11:
                    page._mark_frame_attached_this_turn(frame_id)
                    page._emit_page_event("frameattached", frame)
                elif (
                    page._frame_attaches_this_turn is not None
                    and frame_id in page._frame_attaches_this_turn
                ):
                    # Native can publish attach and navigate in one drain. Let
                    # the attach waiter arm its next waiter before delivery.
                    self._loop.call_soon(
                        self._loop.call_soon, page._emit_deferred_frame_navigation, frame
                    )
                else:
                    page._emit_page_event("framenavigated", frame)
        elif kind == 6 and console_descriptor is not None:
            if len(console_descriptor) == 6:
                (
                    message_type,
                    text,
                    arguments_json,
                    location_url,
                    line_number,
                    column_number,
                ) = console_descriptor
                argument_handles: list[_native.NativeHandle] = []
                timestamp_micros = 0
            elif len(console_descriptor) == 7:
                (
                    message_type,
                    text,
                    arguments_json,
                    location_url,
                    line_number,
                    column_number,
                    timestamp_micros,
                ) = console_descriptor
                argument_handles = []
            else:
                (
                    message_type,
                    text,
                    arguments_json,
                    location_url,
                    line_number,
                    column_number,
                    timestamp_micros,
                    argument_handles,
                ) = console_descriptor
            message = ConsoleMessage(
                page,
                message_type,
                text,
                arguments_json,
                location_url,
                line_number,
                column_number,
                argument_handles,
                timestamp_micros,
            )
            page._emit_page_event("console", message)
            context._emit_context_event("console", message)
        elif kind == 8 and error_descriptor is not None:
            error_name, error_message, error_stack = error_descriptor
            error = Error(error_message)
            setattr(error, "name", error_name or "")
            setattr(error, "stack", error_stack or "")
            page._emit_page_event("pageerror", error)
            context._emit_context_event("weberror", WebError(page, error))
        elif kind == 9 and dialog_descriptor is not None:
            native_dialog, dialog_type, dialog_message, dialog_default_value = dialog_descriptor
            dialog = Dialog(
                page,
                native_dialog,
                dialog_type,
                dialog_message,
                dialog_default_value,
            )
            has_subscribers = page._has_page_event_subscribers(
                "dialog"
            ) or context._has_context_event_subscribers("dialog")
            page._emit_page_event("dialog", dialog)
            context._emit_context_event("dialog", dialog)
            if not has_subscribers:
                asyncio.create_task(dialog._auto_handle())
        elif kind == 10 and file_chooser_descriptor is not None:
            native_file_chooser, is_multiple = file_chooser_descriptor
            page._emit_page_event(
                "filechooser", FileChooser(page, native_file_chooser, is_multiple)
            )

    async def _watch_lifecycle_notices(self) -> None:
        try:
            while not self._closed:
                if not bool(await self._native.next_lifecycle_notices()):
                    return
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._lifecycle_watch_error = error

    async def _finish_lifecycle_watcher(self) -> None:
        task = self._lifecycle_task
        self._lifecycle_task = None
        if task is None:
            return
        try:
            await asyncio.wait_for(task, timeout=5)
        except TimeoutError:
            task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await task

    async def _cancel_lifecycle_watcher(self) -> None:
        task = self._lifecycle_task
        self._lifecycle_task = None
        if task is None:
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    def _network_page(self, page_id: str) -> Page | None:
        for context in self._contexts.values():
            page = context._pages.get(page_id)
            if page is not None:
                return page
        self.contexts
        for context in self._contexts.values():
            context.pages
            page = context._pages.get(page_id)
            if page is not None:
                return page
        return None

    async def _watch_request_notices(self) -> None:
        try:
            while not self._closed:
                page_id, sequence, raw = await self._native.next_request_notice()
                page = self._network_page(page_id)
                if page is None:
                    raise Error(f"Request notice references unknown Page {page_id}")
                event = str(raw.get("kind", "request"))
                if event not in {"request", "requestfinished", "requestfailed"}:
                    raise Error(f"Request notice has unknown event kind {event!r}")
                request = page._wrap_request(raw)
                page._emit_page_event(event, request)
                if event in {"requestfinished", "requestfailed"}:
                    page._forget_route_request_snapshot(raw)
                async with self._network_notice_condition:
                    self._request_notice_sequence = sequence
                    self._network_notice_condition.notify_all()
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._request_watch_error = error
        finally:
            async with self._network_notice_condition:
                self._network_notice_condition.notify_all()

    async def _cancel_request_watcher(self) -> None:
        task = self._request_task
        self._request_task = None
        if task is None:
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    async def _watch_response_notices(self) -> None:
        try:
            while not self._closed:
                page_id, sequence, raw = await self._native.next_response_notice()
                page = self._network_page(page_id)
                if page is None:
                    raise Error(f"Response notice references unknown Page {page_id}")
                page._emit_page_event("response", page._wrap_response(raw))
                async with self._network_notice_condition:
                    self._response_notice_sequence = sequence
                    self._network_notice_condition.notify_all()
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._response_watch_error = error
        finally:
            async with self._network_notice_condition:
                self._network_notice_condition.notify_all()

    async def _flush_network_notices(self) -> None:
        request_target, response_target = self._native.network_notice_fence()

        def reached() -> bool:
            return (
                self._request_notice_sequence >= request_target
                and self._response_notice_sequence >= response_target
            )

        async with self._network_notice_condition:
            await self._network_notice_condition.wait_for(
                lambda: (
                    reached()
                    or self._request_watch_error is not None
                    or self._response_watch_error is not None
                    or self._closed
                )
            )
        if self._request_watch_error is not None:
            raise self._request_watch_error
        if self._response_watch_error is not None:
            raise self._response_watch_error
        if not reached():
            raise TargetClosedError(_TARGET_CLOSED_MESSAGE)

    async def _cancel_response_watcher(self) -> None:
        task = self._response_task
        self._response_task = None
        if task is None:
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    async def _watch_download_notices(self) -> None:
        try:
            while not self._closed:
                page_id, url, suggested_filename, native = await self._native.next_download_notice()
                page = self._network_page(page_id)
                if page is None:
                    raise Error(f"Download notice references unknown Page {page_id}")
                page._emit_page_event("download", Download(page, url, suggested_filename, native))
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._download_watch_error = error

    async def _cancel_download_watcher(self) -> None:
        task = self._download_task
        self._download_task = None
        if task is None:
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    async def _watch_worker_notices(self) -> None:
        try:
            while not self._closed:
                kind, page_id, worker_id, url, native = await self._native.next_worker_notice()
                page = self._network_page(page_id)
                if page is None:
                    # A Page terminal notice can retire the language object
                    # before Chromium's child Worker detach is delivered.
                    # The Page path has already revoked every published Worker
                    # owner, so this exact late notice is safe to discard.
                    continue
                if kind == 0:
                    worker = page._workers.get(worker_id)
                    if worker is None:
                        worker = Worker(page, worker_id, url, native)
                        page._workers[worker_id] = worker
                        page._retired_worker_ids.discard(worker_id)
                        page._emit_page_event("worker", worker)
                    else:
                        worker._url = url
                else:
                    page._retired_worker_ids.add(worker_id)
                    worker = page._workers.pop(worker_id, None)
                    if worker is not None:
                        worker._close()
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._worker_watch_error = error

    async def _cancel_worker_watcher(self) -> None:
        task = self._worker_task
        self._worker_task = None
        if task is None:
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    async def _watch_worker_console_notices(self) -> None:
        try:
            while not self._closed:
                notice = await self._native.next_worker_console_notice()
                if len(notice) == 11:
                    (
                        page_id,
                        worker_id,
                        worker_url,
                        native,
                        message_type,
                        text,
                        arguments_json,
                        location_url,
                        line_number,
                        column_number,
                        timestamp_micros,
                    ) = notice
                    argument_handles: list[_native.NativeHandle] = []
                else:
                    (
                        page_id,
                        worker_id,
                        worker_url,
                        native,
                        message_type,
                        text,
                        arguments_json,
                        location_url,
                        line_number,
                        column_number,
                        timestamp_micros,
                        argument_handles,
                    ) = notice
                page = self._network_page(page_id)
                if page is None or page.is_closed() or worker_id in page._retired_worker_ids:
                    continue
                worker = page._workers.get(worker_id)
                if worker is None:
                    worker = Worker(page, worker_id, worker_url, native)
                    page._workers[worker_id] = worker
                    page._emit_page_event("worker", worker)
                else:
                    worker._url = worker_url
                message = ConsoleMessage(
                    page,
                    message_type,
                    text,
                    arguments_json,
                    location_url,
                    line_number,
                    column_number,
                    argument_handles=argument_handles,
                    timestamp_micros=timestamp_micros,
                    worker=worker,
                )
                worker._emit_worker_event("console", message)
                page._emit_page_event("console", message)
                page.context._emit_context_event("console", message)
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._worker_console_watch_error = error

    async def _cancel_worker_console_watcher(self) -> None:
        task = self._worker_console_task
        self._worker_console_task = None
        if task is None:
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    def _binding_source(
        self, context_id: str, page_id: str, frame_id: str
    ) -> tuple[Page, Frame] | None:
        context = self._contexts.get(context_id)
        if context is None:
            self.contexts
            context = self._contexts.get(context_id)
        if context is None:
            return None
        page = context._pages.get(page_id)
        if page is None:
            context.pages
            page = context._pages.get(page_id)
        if page is None:
            return None
        frame = page._frames.get(frame_id)
        if frame is None:
            page._refresh_frames()
            frame = page._frames.get(frame_id)
        if frame is None:
            return None
        return page, frame

    async def _deliver_binding_error(
        self, call_id: str, error: BaseException, stack: str | None = None
    ) -> None:
        if stack is None:
            stack = "".join(traceback.format_exception(type(error), error, error.__traceback__))
        with contextlib.suppress(BaseException):
            await self._native.deliver_page_binding_error(
                call_id, "Error", str(error), stack or None
            )

    async def _dispatch_page_binding(self, call: tuple[str, str, str, str, str, str, bool]) -> None:
        dispatch = cast("asyncio.Task[None] | None", asyncio.current_task())
        token = _CURRENT_BINDING_DISPATCH.set(dispatch)
        try:
            await self._run_page_binding(call)
        finally:
            if getattr(self, "_binding_closing_dispatch", None) is dispatch:
                self._binding_closing_dispatch = None
            _CURRENT_BINDING_DISPATCH.reset(token)

    async def _run_page_binding(self, call: tuple[str, str, str, str, str, str, bool]) -> None:
        call_id, context_id, page_id, frame_id, name, arguments_json, needs_handle = call
        source = self._binding_source(context_id, page_id, frame_id)
        if source is None:
            # Page retirement may have drained this call after the watcher
            # dequeued it. Do not terminate the watcher for other live Pages.
            with contextlib.suppress(TargetClosedError, InvalidArgumentError):
                await self._native.acknowledge_page_binding_call(call_id)
            await self._deliver_binding_error(
                call_id, Error("Binding source frame has been closed"), None
            )
            return
        page, frame = source
        binding = page._bindings.get(name)
        if binding is None:
            binding = page.context._bindings.get(name)
        if binding is None:
            with contextlib.suppress(TargetClosedError, InvalidArgumentError):
                await self._native.acknowledge_page_binding_call(call_id)
            await self._deliver_binding_error(
                call_id, Error(f'Function "{name}" is not exposed'), None
            )
            return
        callback, registered_with_handle = binding
        acknowledged = False
        callback_task: asyncio.Task[object] | None = None
        try:
            if needs_handle != registered_with_handle:
                raise Error("Page binding Handle mode does not match its registration")
            if needs_handle:
                native_handle = await self._native.take_page_binding_handle(call_id)
                arguments: list[object] = [_wrap_handle(self._native, native_handle, page)]
            else:
                arguments = _binding_arguments(arguments_json)
            outcome = callback({"context": page.context, "page": page, "frame": frame}, *arguments)
            callback_task = (
                await _start_binding_awaitable(cast(Awaitable[object], outcome))
                if inspect.isawaitable(outcome)
                else None
            )
            await self._native.acknowledge_page_binding_call(call_id)
            acknowledged = True
            if callback_task is not None:
                outcome = await callback_task
            await self._native.deliver_page_binding_result(call_id, outcome)
        except BaseException as error:
            if not acknowledged:
                with contextlib.suppress(BaseException):
                    await self._native.acknowledge_page_binding_call(call_id)
            await self._deliver_binding_error(call_id, error)
        finally:
            if callback_task is not None:
                # The callback can be running while acknowledgement is still
                # pending. Keep ownership even if that await fails or is canceled.
                # A callback initiating Browser.close must finish its own close
                # cleanup before this dispatcher is allowed to retire.
                if (
                    not callback_task.done()
                    and getattr(self, "_binding_closing_dispatch", None)
                    is not _CURRENT_BINDING_DISPATCH.get()
                ):
                    callback_task.cancel()
                with contextlib.suppress(BaseException):
                    await callback_task

    async def _watch_page_bindings(self) -> None:
        try:
            while not self._closed:
                call = await self._native.next_page_binding_call()
                if call is None:
                    return
                # A callback may evaluate another exposed binding. Keep the
                # sole queue consumer moving while each call owns its own
                # acknowledgement/result lifecycle. Native Page retirement can
                # release a Rust pending slot before its callback completes, so
                # independently cap Python dispatchers across Page/Context churn.
                if len(self._binding_dispatch_tasks) >= _MAX_BINDING_DISPATCHES:
                    with contextlib.suppress(TargetClosedError, InvalidArgumentError):
                        await self._native.acknowledge_page_binding_call(call[0])
                    await self._deliver_binding_error(
                        call[0],
                        ResourceExhaustedError("pending binding dispatch capacity exceeded (64)"),
                    )
                    continue
                dispatch = asyncio.create_task(self._dispatch_page_binding(call))
                self._binding_dispatch_tasks.add(dispatch)
                dispatch.add_done_callback(self._binding_dispatch_tasks.discard)
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._binding_watch_error = error

    async def _finish_binding_watcher(self) -> None:
        task = self._binding_task
        self._binding_task = None
        if task is not None:
            try:
                await asyncio.wait_for(task, timeout=_BINDING_CLOSE_TIMEOUT_SECONDS)
            except asyncio.TimeoutError:
                task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await task
        closing_dispatch = getattr(self, "_binding_closing_dispatch", None)
        dispatches = tuple(
            dispatch
            for dispatch in self._binding_dispatch_tasks
            if dispatch is not closing_dispatch
        )
        for dispatch in dispatches:
            dispatch.cancel()
        if dispatches:
            _done, pending = await asyncio.wait(dispatches, timeout=_BINDING_CLOSE_TIMEOUT_SECONDS)
            if pending:
                self._binding_watch_error = Error(
                    "Binding callbacks did not stop after Browser closure"
                )

    async def _dispatch_page_route(
        self,
        call: tuple[
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
        ],
    ) -> None:
        (
            call_id,
            registration_id,
            context_id,
            page_id,
            frame_id,
            url,
            method,
            resource_type,
            headers,
            post_data,
            is_navigation_request,
            expires,
            native_request_id,
        ) = call
        context = self._contexts.get(context_id)
        page = context._pages.get(page_id) if context is not None else None
        handler = None if page is None else page._routes.get(registration_id)
        if handler is not None and expires and page is not None:
            page._routes.pop(registration_id, None)
            getattr(page, "_route_patterns", {}).pop(registration_id, None)
            getattr(page, "_route_handler_keys", {}).pop(registration_id, None)
        if handler is None and context is not None:
            handler = context._routes.get(registration_id)
            if handler is not None and expires:
                context._routes.pop(registration_id, None)
                getattr(context, "_route_patterns", {}).pop(registration_id, None)
                getattr(context, "_route_handler_keys", {}).pop(registration_id, None)
        if (
            page is not None
            and native_request_id is not None
            and "route-redirects" in _native.selected_features()
        ):
            request = page._wrap_request(
                await self._native.request_by_native_id(page_id, native_request_id)
            )
            request._merge_route_snapshot(
                {
                    "url": url,
                    "method": method,
                    "resource_type": resource_type,
                    "headers": dict(headers),
                    "post_data": None if post_data is None else post_data.decode(errors="replace"),
                    "post_data_buffer": post_data,
                    "is_navigation_request": is_navigation_request,
                    "failure_text": None,
                    "page_id": page_id,
                    "frame_id": frame_id,
                    "frame_available": frame_id is not None,
                    "native_request_id": native_request_id,
                },
                self._native,
                context,
            )
            page._remember_route_request_snapshot(request)
        else:
            request = Request(
                {
                    "url": url,
                    "method": method,
                    "resource_type": resource_type,
                    "headers": dict(headers),
                    "post_data": None if post_data is None else post_data.decode(errors="replace"),
                    "post_data_buffer": post_data,
                    "is_navigation_request": is_navigation_request,
                    "failure_text": None,
                    "page_id": page_id,
                    "frame_id": frame_id,
                    "frame_available": frame_id is not None,
                    "native_request_id": native_request_id,
                },
                self._native,
                context,
            )
        route = Route(self._native, call_id, request)
        route._dispatch_task = cast("asyncio.Task[object] | None", asyncio.current_task())
        try:
            if handler is None:
                await route.fallback()
            else:
                try:
                    argument_count = len(inspect.signature(handler).parameters)
                except (TypeError, ValueError):
                    argument_count = 2
                outcome = handler(*[route, request][:argument_count])
                if inspect.isawaitable(outcome):
                    await cast(Awaitable[object], outcome)
                await route._finish_scheduled_handling()
        except BaseException as error:
            if not route._handled:
                with contextlib.suppress(BaseException):
                    await route.abort("failed")
            if isinstance(error, asyncio.CancelledError):
                raise
            if self._deferred_route_error is None:
                self._deferred_route_error = error
            asyncio.get_running_loop().call_exception_handler(
                {"message": "Route handler failed", "exception": error}
            )
        finally:
            try:
                await route._finish_registration_retirement()
            except BaseException as error:
                asyncio.get_running_loop().call_exception_handler(
                    {"message": "Route registration retirement failed", "exception": error}
                )

    def _raise_deferred_route_error(self) -> None:
        # Route callbacks can fail after their request has already continued.
        # The next navigation owns the one-shot error, not the completed one.
        error = self._deferred_route_error
        self._deferred_route_error = None
        if error is not None:
            raise error

    async def _watch_page_routes(self) -> None:
        try:
            while not self._closed:
                call = await self._native.next_page_route_call()
                if call is None:
                    return
                await self._dispatch_page_route(call)
        except TargetClosedError:
            return
        except asyncio.CancelledError:
            raise
        except BaseException as error:
            self._route_watch_error = error

    async def _finish_route_watcher(self) -> None:
        task = self._route_task
        self._route_task = None
        if task is None:
            return
        try:
            await asyncio.wait_for(task, timeout=5)
        except TimeoutError:
            task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await task

    async def _cancel_route_watcher(self) -> None:
        task = self._route_task
        self._route_task = None
        if task is None:
            return
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task

    @property
    def browser_type(self) -> BrowserType:
        return self._browser_type

    @property
    def version(self) -> str:
        return str(self._native.version)

    def is_connected(self) -> bool:
        return not self._closed and bool(self._native.is_connected())

    @property
    def contexts(self) -> list[BrowserContext]:
        if getattr(self, "_remote", False):
            native = self._remote_native
            assert native is not None
            live = native.live_contexts()
            known = self._remote_contexts
            contexts: list[RemoteBrowserContext] = []
            for guid, sequence in live:
                context = known.get(guid)
                if (
                    context is None
                    or context._remote_native.created_sequence != sequence
                    or context.is_closed()
                ):
                    raise UnsupportedError(
                        "Browser.contexts contains a remote Context without an installed wrapper"
                    )
                contexts.append(context)
            self._remote_contexts = {context._object_id: context for context in contexts}
            return cast(list[BrowserContext], contexts)
        if self._closed:
            return []
        seen: dict[str, BrowserContext] = {}
        for object_id, base_url in self._native.contexts():
            if object_id in getattr(self, "_pending_context_ids", set[str]()):
                continue
            context = self._contexts.get(object_id)
            if context is None:
                if getattr(self, "_unidentified_storage_state_creations", 0):
                    continue
                context = BrowserContext(
                    self,
                    object_id,
                    base_url,
                    self._persistent_context_viewport,
                )
            seen[object_id] = context
        for object_id, context in self._contexts.items():
            if object_id not in seen:
                context._closed = True
        self._contexts = seen
        return list(seen.values())

    async def new_context(
        self,
        *,
        base_url: str | None = None,
        viewport: Mapping[str, int] | None = None,
        no_viewport: bool = False,
        screen: Mapping[str, int] | None = None,
        user_agent: str | None = None,
        device_scale_factor: float | None = None,
        is_mobile: bool | None = None,
        has_touch: bool | None = None,
        default_browser_type: str | None = None,
        locale: str | None = None,
        timezone_id: str | None = None,
        geolocation: Mapping[str, float] | None = None,
        permissions: Sequence[str] | None = None,
        extra_http_headers: Mapping[str, str | None] | None = None,
        http_credentials: Mapping[str, object] | None = None,
        ignore_https_errors: bool = False,
        offline: bool = False,
        bypass_csp: bool = False,
        java_script_enabled: bool = True,
        accept_downloads: bool = False,
        service_workers: Literal["allow", "block"] = "allow",
        storage_state: Mapping[str, object] | str | os.PathLike[str] | None = None,
        forced_colors: Literal["active", "none"] | None = None,
        strict_selectors: bool = False,
        **kwargs: object,
    ) -> BrowserContext:
        checked_no_viewport = _boolean_argument(no_viewport, "Browser.new_context no_viewport")
        if checked_no_viewport and device_scale_factor is not None:
            raise Error(
                'Browser.new_context: "deviceScaleFactor" option is not supported '
                'with null "viewport"'
            )
        if checked_no_viewport and is_mobile is True:
            raise Error(
                'Browser.new_context: "isMobile" option is not supported with null "viewport"'
            )
        if checked_no_viewport:
            raise UnsupportedError(
                "Browser.new_context no_viewport requires a null-viewport profile"
            )
        if getattr(self, "_remote", False):
            remote_options = {
                "base_url": base_url is not None,
                "viewport": viewport is not None,
                "no_viewport": no_viewport is not False,
                "screen": screen is not None,
                "user_agent": user_agent is not None,
                "device_scale_factor": device_scale_factor is not None,
                "is_mobile": is_mobile is not None,
                "has_touch": has_touch is not None,
                "default_browser_type": default_browser_type is not None,
                "locale": locale is not None,
                "timezone_id": timezone_id is not None,
                "geolocation": geolocation is not None,
                "permissions": permissions is not None,
                "extra_http_headers": extra_http_headers is not None,
                "http_credentials": http_credentials is not None,
                "ignore_https_errors": ignore_https_errors is not False,
                "offline": offline is not False,
                "bypass_csp": bypass_csp is not False,
                "java_script_enabled": java_script_enabled is not True,
                "accept_downloads": accept_downloads is not False,
                "service_workers": service_workers != "allow",
                "storage_state": storage_state is not None,
                "forced_colors": forced_colors is not None,
                "strict_selectors": strict_selectors is not False,
                **{name: True for name in kwargs},
            }
            unsupported = [name for name, supplied in remote_options.items() if supplied]
            if unsupported:
                raise UnsupportedError(
                    "Browser.new_context remote options are not installed: "
                    + ", ".join(sorted(unsupported))
                )
            native = self._remote_native
            assert native is not None
            self._remote_creation_started("context")
            try:
                context = RemoteBrowserContext(self, await native.new_context())
            finally:
                self._remote_creation_finished("context")
            self._remote_contexts = {
                guid: current
                for guid, current in self._remote_contexts.items()
                if not current.is_closed()
            }
            self._remote_contexts[context._object_id] = context
            return context
        if kwargs:
            options = ", ".join(sorted(kwargs))
            raise UnsupportedError(f"Browser.new_context options are not installed: {options}")
        _ = default_browser_type
        device = _device_options(
            user_agent=user_agent,
            viewport=viewport,
            screen=screen,
            device_scale_factor=device_scale_factor,
            is_mobile=is_mobile,
            has_touch=has_touch,
        )
        checked_ignore_https_errors = _boolean_argument(
            ignore_https_errors, "Browser.new_context ignore_https_errors"
        )
        checked_geolocation = _geolocation_options(geolocation, "Browser.new_context geolocation")
        geolocation_permission = _geolocation_permission(
            permissions, "Browser.new_context permissions"
        )
        checked_http_credentials = _http_credentials_options(
            http_credentials, "Browser.new_context http_credentials"
        )
        checked_extra_http_headers = _context_creation_extra_http_headers(extra_http_headers)
        if (
            checked_extra_http_headers is not None
            and "extra-headers" not in _native.selected_features()
        ):
            raise UnsupportedError(
                "Browser.new_context extra_http_headers requires the extra-headers profile"
            )
        checked_offline = _boolean_argument(offline, "Browser.new_context offline")
        checked_bypass_csp = _boolean_argument(bypass_csp, "Browser.new_context bypass_csp")
        checked_java_script_enabled = _boolean_argument(
            java_script_enabled, "Browser.new_context java_script_enabled"
        )
        checked_accept_downloads = _boolean_argument(
            accept_downloads, "Browser.new_context accept_downloads"
        )
        checked_service_workers = _string_argument(
            service_workers, "Browser.new_context service_workers"
        )
        if checked_service_workers not in ("allow", "block"):
            raise Error(
                f"service_workers: expected one of allow, block, got {checked_service_workers!r}"
            )
        checked_forced_colors: str | None = None
        if forced_colors is not None:
            checked_forced_colors = _string_argument(
                forced_colors, "Browser.new_context forced_colors"
            )
            if checked_forced_colors not in ("active", "none"):
                raise Error("Browser.new_context forced_colors must be 'active' or 'none'")
            if "context-forced-colors" not in _native.selected_features():
                raise UnsupportedError(
                    "Browser.new_context forced_colors requires the context-forced-colors profile"
                )
        checked_strict_selectors = _boolean_argument(
            strict_selectors, "Browser.new_context strict_selectors"
        )
        if (
            checked_strict_selectors
            and "context-strict-selectors" not in _native.selected_features()
        ):
            raise UnsupportedError(
                "Browser.new_context strict_selectors requires the context-strict-selectors profile"
            )
        storage_state_json: str | None = None
        if storage_state is not None:
            _require_context_storage_state()
            storage_state_json = await _browser_storage_state_json(storage_state)
            self._unidentified_storage_state_creations += 1
        unidentified_storage_context = storage_state_json is not None
        pending_context_id: str | None = None
        retain_pending_context = False
        try:
            native_options = (
                base_url,
                device,
                locale,
                timezone_id,
                checked_geolocation,
                geolocation_permission,
                checked_http_credentials,
                checked_ignore_https_errors,
                checked_offline,
                checked_bypass_csp,
                checked_java_script_enabled,
                checked_accept_downloads,
                checked_service_workers == "block",
            )
            if checked_extra_http_headers is not None:
                object_id, published_base_url = await self._native.new_context(
                    *native_options,
                    forced_colors=checked_forced_colors,
                    strict_selectors=checked_strict_selectors,
                    extra_http_headers=checked_extra_http_headers,
                )
            elif checked_forced_colors is None and not checked_strict_selectors:
                object_id, published_base_url = await self._native.new_context(*native_options)
            elif checked_forced_colors is None:
                object_id, published_base_url = await self._native.new_context(
                    *native_options, strict_selectors=True
                )
            elif not checked_strict_selectors:
                object_id, published_base_url = await self._native.new_context(
                    *native_options, forced_colors=checked_forced_colors
                )
            else:
                object_id, published_base_url = await self._native.new_context(
                    *native_options,
                    forced_colors=checked_forced_colors,
                    strict_selectors=True,
                )
            if storage_state_json is not None:
                self._pending_context_ids.add(object_id)
                pending_context_id = object_id
                self._unidentified_storage_state_creations -= 1
                unidentified_storage_context = False
                try:
                    await self._native.context_set_storage_state(object_id, storage_state_json)
                except BaseException as restore_error:

                    async def close_pending_context() -> None:
                        await self._native.close_context(object_id)

                    close_task = asyncio.create_task(close_pending_context())
                    self._pending_context_cleanup_tasks[object_id] = close_task
                    try:
                        await asyncio.shield(close_task)
                    except asyncio.CancelledError:
                        retain_pending_context = True
                        close_task.add_done_callback(
                            lambda done: self._finish_pending_context_cleanup(object_id, done)
                        )
                        raise
                    except Exception as cleanup_error:
                        retain_pending_context = True
                        raise Error(
                            "Browser.new_context: storage state restore failed: "
                            f"{restore_error}; Context cleanup failed: {cleanup_error}"
                        ) from restore_error
                    except BaseException:
                        retain_pending_context = True
                        raise
                    finally:
                        if not retain_pending_context:
                            self._pending_context_cleanup_tasks.pop(object_id, None)
                    raise
            context = BrowserContext(
                self,
                object_id,
                published_base_url,
                _owned_context_viewport(device),
            )
            self._contexts[object_id] = context
            return context
        finally:
            if unidentified_storage_context:
                self._unidentified_storage_state_creations -= 1
            if pending_context_id is not None and not retain_pending_context:
                self._pending_context_ids.discard(pending_context_id)

    def _finish_pending_context_cleanup(self, context_id: str, task: asyncio.Task[None]) -> None:
        self._pending_context_cleanup_tasks.pop(context_id, None)
        if task.cancelled():
            task.get_loop().call_exception_handler(
                {
                    "message": "Browser.new_context: detached Context cleanup was cancelled",
                    "exception": asyncio.CancelledError(),
                    "task": task,
                }
            )
            return
        error = task.exception()
        if error is None:
            self._pending_context_ids.discard(context_id)
        else:
            task.get_loop().call_exception_handler(
                {
                    "message": "Browser.new_context: detached Context cleanup failed",
                    "exception": error,
                    "task": task,
                }
            )

    async def new_page(
        self,
        *,
        base_url: str | None = None,
        accept_downloads: bool = False,
        **kwargs: object,
    ) -> Page:
        if getattr(self, "_remote", False):
            if base_url is not None or accept_downloads or kwargs:
                raise UnsupportedError("Browser.new_page remote options are not installed")
            remote_context_type = getattr(_native, "NativeRemoteContext", None)
            if remote_context_type is not None and not callable(
                getattr(remote_context_type, "close", None)
            ):
                raise UnsupportedError(
                    "Browser.new_page requires a rebuilt native extension "
                    "for implicit Context cleanup"
                )
            context = await self.new_context()
            try:
                page = await context.new_page()
                cast(RemotePage, page)._closes_context = True
                return page
            except BaseException:
                try:
                    await context.close()
                except BaseException as cleanup_error:
                    raise RuntimeError(
                        "Browser.new_page failed and implicit Context cleanup failed"
                    ) from cleanup_error
                raise
        if kwargs:
            context_factory = cast(
                Callable[..., Awaitable[BrowserContext]],
                self.new_context,
            )
            context = await context_factory(
                base_url=base_url,
                accept_downloads=accept_downloads,
                **kwargs,
            )
            page = await context.new_page()
            page._closes_context = True
            return page
        checked_accept_downloads = _boolean_argument(
            accept_downloads, "Browser.new_page accept_downloads"
        )
        context_value, page_value = await self._native.new_browser_page(
            base_url, checked_accept_downloads
        )
        context_id, published_base_url = context_value
        page_id, _context_id, main_frame_id, url = page_value
        context = self._contexts.get(context_id)
        if context is None:
            context = BrowserContext(
                self,
                context_id,
                published_base_url,
                _DEFAULT_VIEWPORT_SIZE,
            )
        page = context._pages.get(page_id)
        if page is None:
            page = Page(context, page_id, main_frame_id, url)
        else:
            page._main_frame_id = main_frame_id
            page._url = url
        context._pages[page_id] = page
        self._contexts[context_id] = context
        return page

    async def close(self, *, reason: str | None = None) -> None:
        if reason is not None:
            raise UnsupportedError("Browser.close(reason=...) is not installed")
        if self._closed:
            return
        if getattr(self, "_remote", False):
            native = self._remote_native
            assert native is not None
            self._closed = True
            native_closed = False
            try:
                await native.close()
                native_closed = True
            finally:
                try:
                    if native_closed and self._remote_close_notices:
                        task = self._remote_disconnect_task
                        if task is not None and not task.done():
                            with contextlib.suppress(asyncio.TimeoutError):
                                await asyncio.wait_for(
                                    asyncio.shield(task),
                                    timeout=_REMOTE_CREATION_FENCE_TIMEOUT_SECONDS,
                                )
                finally:
                    await self._cancel_remote_disconnect_watcher()
                    self._mark_remote_disconnected()
            return
        dispatch = _CURRENT_BINDING_DISPATCH.get()
        self._binding_closing_dispatch = (
            dispatch if dispatch in self._binding_dispatch_tasks else None
        )
        self._closed = True
        await self._cancel_cdp_event_watcher()
        await self._cancel_worker_console_watcher()
        for context in self._contexts.values():
            for page in context._pages.values():
                page._retire_workers()
        await self._cancel_lifecycle_watcher()
        await self._cancel_route_watcher()
        await self._cancel_request_watcher()
        await self._cancel_response_watcher()
        await self._cancel_download_watcher()
        await self._cancel_worker_watcher()
        for context in self._contexts.values():
            context._ensure_request_context()
        await self._native.close()
        await self._settle_cdp_detach_tasks()
        await self._finish_binding_watcher()
        contexts = list(self._contexts.values())
        for context in contexts:
            context._retire_all_cdp_sessions()
            if context._request_context is not None:
                context._request_context._dispose_from_owner(reason)
            context._closed = True
        if self._fresh_owned:
            for context in contexts:
                context._emit_context_event("close", context)
        self._contexts.clear()
        with contextlib.suppress(ValueError):
            self._browser_type._playwright._browsers.remove(self)


class BrowserType:
    def __init__(self, playwright: Playwright, name: str) -> None:
        self._playwright = playwright
        self.name = name

    def __repr__(self) -> str:
        return f"<BrowserType name={self.name} executable_path={self.executable_path}>"

    @property
    def executable_path(self) -> str:
        return _native.browser_executable_path(self.name, str(Path(__file__).resolve().parent))

    async def connect(
        self,
        ws_endpoint: str,
        *,
        timeout: float | None = None,
        slow_mo: float | None = None,
        headers: Mapping[str, str] | None = None,
        expose_network: str | None = None,
    ) -> Browser:
        features = _native.selected_features()
        if not (
            {"playwright-server-initialize", "playwright-server-connect-session"} & set(features)
        ):
            raise UnsupportedError(
                "BrowserType.connect requires the playwright-server-initialize profile"
            )
        for name, value in (
            ("slow_mo", slow_mo),
            ("headers", headers),
            ("expose_network", expose_network),
        ):
            if value is not None:
                raise UnsupportedError(f"BrowserType.connect option {name!r} is not installed")
        if "playwright-server-connect-session" in features:
            endpoint = _playwright_server_connect_endpoint(ws_endpoint)
            checked_timeout = _timeout_ms(timeout)
            if checked_timeout == 0 or checked_timeout > 120_000:
                raise ValueError(
                    "BrowserType.connect timeout must be between 1 and 120000 milliseconds"
                )
            native = await self._playwright._environment.connect_playwright_server(
                endpoint, self.name, checked_timeout
            )
            browser = Browser(self, native, remote=True)
            self._playwright._browsers.append(browser)
            return browser
        endpoint = _playwright_server_endpoint(ws_endpoint)
        checked_timeout = _timeout_ms(timeout)
        await self._playwright._environment.observe_playwright_server_initialize(
            endpoint, checked_timeout
        )
        raise UnsupportedError("BrowserType.connect remote object graph is not installed")

    async def connect_over_cdp(
        self,
        endpoint_url: str,
        *,
        timeout: float | None = None,
        slow_mo: float | None = None,
        headers: Mapping[str, str] | None = None,
        is_local: bool | None = None,
    ) -> Browser:
        if self.name != "chromium":
            raise UnsupportedError("connect_over_cdp is only available for Chromium")
        _unsupported_option("slow_mo", slow_mo)
        _unsupported_option("is_local", is_local)
        checked_headers = None if headers is None else _cdp_handshake_headers(headers)
        if checked_headers:
            if "cdp-handshake-headers" not in _native.selected_features():
                raise UnsupportedError(
                    "BrowserType.connect_over_cdp headers require the cdp-handshake-headers profile"
                )
            native = await self._playwright._environment.connect_over_cdp(
                endpoint_url, _timeout_ms(timeout), checked_headers
            )
        else:
            native = await self._playwright._environment.connect_over_cdp(
                endpoint_url, _timeout_ms(timeout)
            )
        browser = Browser(self, native)
        self._playwright._browsers.append(browser)
        return browser

    async def launch(
        self,
        *,
        executable_path: str | Path | None = None,
        channel: str | None = None,
        headless: bool | None = None,
        timeout: float | None = None,
        args: Sequence[str] | None = None,
        **kwargs: object,
    ) -> Browser:
        checked_args = _persistent_context_launch_arguments(args)
        if checked_args:
            raise UnsupportedError("BrowserType.launch args are not installed")
        checked_path = None if executable_path is None else Path(executable_path).resolve()
        if checked_path is not None and not checked_path.is_file():
            raise Error(f"executable doesn't exist at {checked_path}")
        proxy_options = kwargs.get("proxy")
        if isinstance(proxy_options, Mapping) and "server" in proxy_options:
            _string_argument(cast(Mapping[str, object], proxy_options)["server"], "proxy.server")
        if self.name != "chromium":
            if (
                self.name in {"firefox", "webkit"}
                and checked_path is not None
                and "owned-process-startup" in _native.selected_features()
            ):
                if channel is not None:
                    raise UnsupportedError(
                        "BrowserType.launch channel is not installed for Firefox/WebKit"
                    )
                if kwargs:
                    options = ", ".join(sorted(kwargs))
                    raise UnsupportedError(
                        "BrowserType.launch options are not installed "
                        f"for Firefox/WebKit: {options}"
                    )
                _boolean_argument(
                    True if headless is None else headless, "BrowserType.launch headless"
                )
                await self._playwright._environment.observe_nonchromium_startup(
                    self.name, str(checked_path), _timeout_ms(timeout)
                )
                raise UnsupportedError(
                    "BrowserType.launch Firefox/WebKit protocol is not installed"
                )
            raise UnsupportedError("launch is only available for Chromium")
        if "fresh-launch" not in _native.selected_features():
            raise UnsupportedError("BrowserType.launch requires the fresh-launch profile")
        if kwargs:
            options = ", ".join(sorted(kwargs))
            raise UnsupportedError(f"BrowserType.launch options are not installed: {options}")
        checked_headless = _boolean_argument(
            True if headless is None else headless, "BrowserType.launch headless"
        )
        native = await self._playwright._environment.launch_chromium(
            checked_headless,
            None if checked_path is None else str(checked_path),
            channel,
            _timeout_ms(timeout),
        )
        try:
            browser = Browser(self, native, fresh_owned=True)
        except BaseException:
            with contextlib.suppress(Exception):
                await native.close()
            raise
        self._playwright._browsers.append(browser)
        return browser

    async def launch_persistent_context(
        self,
        user_data_dir: str | Path,
        *,
        headless: bool | None = None,
        base_url: str | None = None,
        extra_http_headers: Mapping[str, str] | None = None,
        locale: str | None = None,
        timezone_id: str | None = None,
        geolocation: Mapping[str, float] | None = None,
        permissions: Sequence[str] | None = None,
        http_credentials: Mapping[str, object] | None = None,
        ignore_https_errors: bool = False,
        offline: bool = False,
        bypass_csp: bool = False,
        java_script_enabled: bool = True,
        viewport: Mapping[str, int] | None = None,
        screen: Mapping[str, int] | None = None,
        user_agent: str | None = None,
        device_scale_factor: float | None = None,
        is_mobile: bool | None = None,
        has_touch: bool | None = None,
        color_scheme: Literal["dark", "light", "no-preference"] | None = None,
        reduced_motion: Literal["no-preference", "reduce"] | None = None,
        contrast: Literal["more", "no-preference"] | None = None,
        args: Sequence[str] | None = None,
        executable_path: str | Path | None = None,
        channel: str | None = None,
        timeout: float | None = None,
        **kwargs: object,
    ) -> BrowserContext:
        if self.name == "webkit":
            # Match WebKit's pre-spawn argument validation without enabling launch.
            _persistent_context_launch_arguments(args)
        if self.name != "chromium":
            raise UnsupportedError("launch_persistent_context is only available for Chromium")
        if isinstance(user_data_dir, str) and user_data_dir == "":
            raise UnsupportedError(
                "BrowserType.launch_persistent_context with an empty user_data_dir "
                "requires a temporary profile, which is not installed"
            )
        if kwargs:
            options = ", ".join(sorted(kwargs))
            raise UnsupportedError(
                f"BrowserType.launch_persistent_context options are not installed: {options}"
            )
        checked_launch_arguments = _persistent_context_launch_arguments(args)
        checked_geolocation = _geolocation_options(
            geolocation, "BrowserType.launch_persistent_context geolocation"
        )
        geolocation_permission = _geolocation_permission(
            permissions, "BrowserType.launch_persistent_context permissions"
        )
        checked_http_credentials = _http_credentials_options(
            http_credentials, "BrowserType.launch_persistent_context http_credentials"
        )
        checked_ignore_https_errors = _boolean_argument(
            ignore_https_errors,
            "BrowserType.launch_persistent_context ignore_https_errors",
        )
        checked_offline = _boolean_argument(
            offline, "BrowserType.launch_persistent_context offline"
        )
        checked_bypass_csp = _boolean_argument(
            bypass_csp, "BrowserType.launch_persistent_context bypass_csp"
        )
        checked_java_script_enabled = _boolean_argument(
            java_script_enabled,
            "BrowserType.launch_persistent_context java_script_enabled",
        )
        device = _device_options(
            user_agent=user_agent,
            viewport=viewport,
            screen=screen,
            device_scale_factor=device_scale_factor,
            is_mobile=is_mobile,
            has_touch=has_touch,
        )
        media = _persistent_context_media_options(
            color_scheme=color_scheme,
            reduced_motion=reduced_motion,
            contrast=contrast,
        )
        native = await self._playwright._environment.launch_persistent_context(
            str(Path(user_data_dir).resolve()),
            True if headless is None else headless,
            (
                base_url,
                (
                    locale,
                    timezone_id,
                    checked_geolocation,
                    geolocation_permission,
                    checked_http_credentials,
                    checked_ignore_https_errors,
                    checked_offline,
                    checked_bypass_csp,
                    checked_java_script_enabled,
                ),
                device,
                media,
                _persistent_context_extra_http_headers(extra_http_headers),
                checked_launch_arguments,
                None if executable_path is None else str(Path(executable_path).resolve()),
                channel,
                _timeout_ms(timeout),
            ),
        )
        browser = Browser(
            self,
            native,
            persistent_context=True,
            persistent_context_viewport=_owned_context_viewport(device),
        )
        self._playwright._browsers.append(browser)
        contexts = browser.contexts
        if len(contexts) != 1:
            await browser.close()
            raise Error(
                "launch_persistent_context expected exactly one default BrowserContext, "
                f"observed {len(contexts)}"
            )
        return contexts[0]


class Selectors:
    def __init__(self, environment: _native.NativeEnvironment) -> None:
        self._environment = environment

    async def register(
        self,
        name: str,
        script: str | None = None,
        *,
        path: Path | str | None = None,
        content_script: bool | None = None,
    ) -> None:
        if path:
            source = await asyncio.to_thread(_read_selector_engine_source, Path(path))
        elif not script:
            raise Error("Either source or path should be specified")
        else:
            source = _string_argument(script, "Selectors.register")
        self._environment.register_selector_engine(name, source, bool(content_script))

    def set_test_id_attribute(self, attribute_name: str) -> None:
        self._environment.set_test_id_attribute(
            _string_argument(attribute_name, "Selectors.set_test_id_attribute")
        )


class Playwright:
    def __init__(self) -> None:
        self._environment = _native.NativeEnvironment()
        self._browsers: list[Browser] = []
        self._stopped = False
        self.chromium = BrowserType(self, "chromium")
        self.firefox = BrowserType(self, "firefox")
        self.webkit = BrowserType(self, "webkit")
        self.selectors = Selectors(self._environment)
        self.devices = _device_descriptors()
        self.request = APIRequest()

    def __getitem__(self, name: str) -> BrowserType:
        if name == "chromium":
            return self.chromium
        if name == "firefox":
            return self.firefox
        if name == "webkit":
            return self.webkit
        raise KeyError(name)

    async def stop(self) -> None:
        if self._stopped:
            return
        self._stopped = True
        browsers = list(self._browsers)
        for browser in browsers:
            browser._closed = True
        self._browsers.clear()
        await self.request._dispose_all()
        for browser in browsers:
            await browser._cancel_remote_disconnect_watcher()
            await browser._cancel_cdp_event_watcher()
            await browser._cancel_lifecycle_watcher()
            await browser._cancel_route_watcher()
            await browser._cancel_response_watcher()
            await browser._cancel_worker_watcher()
            await browser._cancel_worker_console_watcher()
            for context in browser._contexts.values():
                context._retire_all_cdp_sessions()
        await self._environment.close()
        for browser in browsers:
            await browser._settle_cdp_detach_tasks()
            await browser._finish_lifecycle_watcher()
            await browser._finish_binding_watcher()
            await browser._finish_route_watcher()
            if browser._remote:
                browser._mark_remote_disconnected()


class PlaywrightContextManager:
    def __init__(self) -> None:
        self._playwright: Playwright | None = None

    async def start(self) -> Playwright:
        if self._playwright is not None:
            return self._playwright
        self._playwright = Playwright()
        return self._playwright

    async def __aenter__(self) -> Playwright:
        return await self.start()

    async def __aexit__(self, *_exc: object) -> None:
        if self._playwright is not None:
            await self._playwright.stop()


def async_playwright() -> PlaywrightContextManager:
    return PlaywrightContextManager()


__all__ = [
    "APIRequest",
    "APIRequestContext",
    "APIResponse",
    "ClientCertificate",
    "Cookie",
    "ConsoleMessage",
    "Dialog",
    "WebError",
    "FileChooser",
    "DeviceDescriptor",
    "Browser",
    "BrowserContext",
    "BrowserType",
    "Frame",
    "FrameLocator",
    "JSHandle",
    "ElementHandle",
    "FileInput",
    "FilePayload",
    "Locator",
    "LocatorAssertions",
    "Keyboard",
    "Mouse",
    "Touchscreen",
    "Page",
    "Playwright",
    "PlaywrightContextManager",
    "Request",
    "RequestSizes",
    "SecurityDetails",
    "ResourceTiming",
    "Response",
    "Selectors",
    "SetCookieParam",
    "StorageState",
    "StorageStateCookie",
    "ViewportSize",
    "SourceLocation",
    "EventContextManager",
    "PageEventContextManager",
    "Worker",
    "TargetClosedError",
    "Tracing",
    "async_playwright",
    "expect",
]
