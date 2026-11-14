"""Playwright-shaped async APIRequest facade over the Rust request owner."""

from __future__ import annotations

import asyncio
import inspect
import json
import math
import platform
import sys
from collections.abc import Awaitable, Callable, Mapping, Sequence
from os import PathLike
from pathlib import Path
from typing import Any, Optional, TypedDict, Union, cast
from urllib.parse import urljoin, urlparse

from . import _native
from ._errors import Error, UnsupportedError


class StorageStateCookie(TypedDict):
    name: str
    value: str
    domain: str
    path: str
    expires: float
    httpOnly: bool
    secure: bool
    sameSite: str


class StorageState(TypedDict):
    cookies: list[StorageStateCookie]
    origins: list[dict[str, Any]]


class ClientCertificate(TypedDict, total=False):
    origin: str
    certPath: str | PathLike[str]
    cert: bytes
    keyPath: str | PathLike[str]
    key: bytes
    pfxPath: str | PathLike[str]
    pfx: bytes
    passphrase: str


_UNSET = object()
_Resolver = Callable[[str], object]
_NativeClientCertificate = tuple[
    str,
    Optional[bytes],
    Optional[bytes],
    Optional[bytes],
    Optional[str],
]
_NativeQuery = tuple[Optional[str], list[tuple[str, str]]]
_NativeMultipartField = tuple[
    str,
    Optional[str],
    Optional[str],
    Optional[str],
    Optional[bytes],
]
_NativeBody = tuple[
    int,
    Optional[bytes],
    Optional[str],
    list[tuple[str, str]],
    list[_NativeMultipartField],
]


def _default_user_agent() -> str:
    system = platform.system() or "unknown"
    release = platform.release() or "unknown"
    machine = platform.machine() or "unknown"
    python = f"{sys.version_info.major}.{sys.version_info.minor}"
    return f"Playwright/1.58.0 ({machine}; {system} {release}) python/{python}"


def _milliseconds(value: object, name: str, default: int | None = None) -> int | None:
    if value is None:
        return default
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TypeError(f"{name} must be a number in milliseconds")
    if not math.isfinite(value) or value < 0 or value > 4_294_967_295:
        raise ValueError(f"{name} must be finite and between 0 and 4294967295 milliseconds")
    return int(value)


def _integer(value: object, name: str, default: int) -> int:
    if value is None:
        return default
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{name} must be an integer")
    if value < 0:
        raise AssertionError(f"'{name}' must be greater than or equal to '0'")
    if value > 4_294_967_295:
        raise ValueError(f"{name} must be at most 4294967295")
    return value


def _optional_string(value: object, api: str, name: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str):
        raise TypeError(f"{api}: {name} must be a string")
    return value


def _optional_boolean(value: object, api: str, name: str) -> bool | None:
    if value is None:
        return None
    if not isinstance(value, bool):
        raise TypeError(f"{api}: {name} must be a boolean")
    return value


def _headers(value: object, api: str) -> list[tuple[str, str]]:
    if value is None:
        return []
    entries: list[tuple[object, object]] = []
    if isinstance(value, Mapping):
        entries.extend(cast(Mapping[object, object], value).items())
    elif isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        for raw_item in cast(Sequence[object], value):
            if not isinstance(raw_item, Mapping):
                raise TypeError(f"{api}: header entries must be mappings")
            item = cast(Mapping[object, object], raw_item)
            entries.append((item.get("name"), item.get("value")))
    else:
        raise TypeError(f"{api}: headers must be a mapping or sequence")
    result: list[tuple[str, str]] = []
    for name, header_value in entries:
        if not isinstance(name, str) or not isinstance(header_value, str):
            raise TypeError(f"{api}: header name and value must be strings")
        result.append((name, header_value))
    return result


def _map_request_error(error: BaseException, api: str) -> Error:
    message = str(error)
    if message == "API request transport failed: connection closed before response headers":
        message = "socket hang up"
    prefix = "API request timed out after "
    if message.startswith(prefix) and message.endswith("ms"):
        timeout = message[len(prefix) : -2]
        if timeout.isdigit():
            return Error(f"{api}: Timeout {timeout}ms exceeded.")
    if message == "API response body is disposed":
        return Error(f"{api}: Response has been disposed")
    return Error(f"{api}: {message}")


def _cookie_from_mapping(
    value: object, api: str
) -> tuple[str, str, str, str, float, bool, bool, str]:
    if not isinstance(value, Mapping):
        raise TypeError(f"{api}: storage_state cookie must be a mapping")
    cookie = cast(Mapping[object, object], value)
    string_values: list[str] = []
    for field in ("name", "value", "domain", "path"):
        item = cookie.get(field)
        if not isinstance(item, str):
            raise TypeError(f"{api}: storage_state cookie {field} must be a string")
        string_values.append(item)
    expires = cookie.get("expires")
    if (
        isinstance(expires, bool)
        or not isinstance(expires, (int, float))
        or not math.isfinite(expires)
    ):
        raise TypeError(f"{api}: storage_state cookie expires must be a finite number")
    http_only = cookie.get("httpOnly")
    secure = cookie.get("secure")
    if not isinstance(http_only, bool) or not isinstance(secure, bool):
        raise TypeError(f"{api}: storage_state cookie flags must be booleans")
    same_site = cookie.get("sameSite")
    if same_site not in ("Strict", "Lax", "None"):
        raise TypeError(f"{api}: storage_state cookie sameSite must be Strict, Lax, or None")
    return (
        string_values[0],
        string_values[1],
        string_values[2],
        string_values[3],
        float(expires),
        http_only,
        secure,
        cast(str, same_site),
    )


async def _normalize_storage_state(
    value: object,
) -> tuple[list[tuple[str, str, str, str, float, bool, bool, str]], list[dict[str, Any]]]:
    api = "request.new_context"
    if value is None:
        return [], []
    if isinstance(value, (str, PathLike)):
        path_value = cast(Union[str, PathLike[str]], value)
        source = await asyncio.to_thread(Path(path_value).read_text, "utf-8")
        value = cast(object, json.loads(source))
    if not isinstance(value, Mapping):
        raise TypeError(f"{api}: storage_state must be a mapping or path")
    state = cast(Mapping[object, object], value)
    cookies = state.get("cookies", [])
    origins = state.get("origins", [])
    if not isinstance(cookies, list) or not isinstance(origins, list):
        raise TypeError(f"{api}: storage_state cookies and origins must be lists")
    normalized_origins = cast(object, json.loads(json.dumps(origins)))
    if not isinstance(normalized_origins, list):
        raise TypeError(f"{api}: storage_state origins must be a list")
    return [_cookie_from_mapping(cookie, api) for cookie in cast(list[object], cookies)], cast(
        list[dict[str, Any]], normalized_origins
    )


async def _certificate_material(
    certificate: Mapping[object, object],
    content_name: str,
    path_name: str,
    api: str,
) -> bytes | None:
    content = certificate.get(content_name)
    path = certificate.get(path_name)
    if content is not None and path is not None:
        raise TypeError(
            f"{api}: client certificate cannot specify both {content_name} and {path_name}"
        )
    if content is not None:
        if not isinstance(content, bytes):
            raise TypeError(f"{api}: client certificate {content_name} must be bytes")
        return bytes(content)
    if path is not None:
        if not isinstance(path, (str, PathLike)):
            raise TypeError(f"{api}: client certificate {path_name} must be a path")
        path_value = cast(Union[str, PathLike[str]], path)
        return await asyncio.to_thread(Path(path_value).read_bytes)
    return None


async def _normalize_client_certificates(
    value: object,
    api: str,
) -> list[_NativeClientCertificate]:
    if value is None:
        return []
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes, bytearray)):
        raise TypeError(f"{api}: client_certificates must be a sequence")
    result: list[_NativeClientCertificate] = []
    for raw_certificate in cast(Sequence[object], value):
        if not isinstance(raw_certificate, Mapping):
            raise TypeError(f"{api}: client certificate entries must be mappings")
        certificate = cast(Mapping[object, object], raw_certificate)
        origin = certificate.get("origin")
        if not isinstance(origin, str) or not origin:
            raise TypeError(f"{api}: client certificate origin must be a non-empty string")
        passphrase = certificate.get("passphrase")
        if passphrase is not None and not isinstance(passphrase, str):
            raise TypeError(f"{api}: client certificate passphrase must be a string")
        cert = await _certificate_material(certificate, "cert", "certPath", api)
        key = await _certificate_material(certificate, "key", "keyPath", api)
        pfx = await _certificate_material(certificate, "pfx", "pfxPath", api)
        if (cert is None) != (key is None):
            raise TypeError(f"{api}: client certificate cert and key must be specified together")
        if pfx is not None and cert is not None:
            raise TypeError(f"{api}: client certificate pfx cannot be combined with cert and key")
        if pfx is None and cert is None:
            raise TypeError(f"{api}: client certificate must contain either cert/key or pfx")
        result.append((origin, cert, key, pfx, passphrase))
    return result


def _name_value_pairs(value: object, api: str, option: str) -> list[tuple[str, str]]:
    if not isinstance(value, Mapping):
        raise TypeError(f"{api}: {option} must be a mapping")
    result: list[tuple[str, str]] = []
    for name, field_value in cast(Mapping[object, object], value).items():
        if not isinstance(name, str):
            raise TypeError(f"{api}: {option} names must be strings")
        if not isinstance(field_value, (str, bool, int, float)):
            raise TypeError(f"{api}: {option} values must be strings, numbers, or booleans")
        result.append((name, str(field_value)))
    return result


def _query(params: object, api: str) -> _NativeQuery:
    if params is None:
        return (None, [])
    if isinstance(params, str):
        return (params, [])
    return (None, _name_value_pairs(params, api, "params"))


def _body_and_headers(
    data: object,
    form: object,
    multipart: object,
    request_headers: list[tuple[str, str]],
) -> tuple[_NativeBody, list[tuple[str, str]]]:
    supplied = sum(value is not _UNSET and value is not None for value in (data, form, multipart))
    if supplied > 1:
        raise AssertionError("Only one of 'data', 'form' or 'multipart' can be specified")
    if form is not _UNSET and form is not None:
        return (
            3,
            None,
            None,
            _name_value_pairs(form, "APIRequestContext", "form"),
            [],
        ), request_headers
    if multipart is not _UNSET and multipart is not None:
        if not isinstance(multipart, Mapping):
            raise TypeError("APIRequestContext: multipart must be a mapping")
        fields: list[_NativeMultipartField] = []
        for name, value in cast(Mapping[object, object], multipart).items():
            if not isinstance(name, str):
                raise TypeError("APIRequestContext: multipart names must be strings")
            if isinstance(value, Mapping):
                payload = cast(Mapping[object, object], value)
                file_name = payload.get("name")
                mime_type = payload.get("mimeType")
                buffer = payload.get("buffer")
                if (
                    not isinstance(file_name, str)
                    or not isinstance(mime_type, str)
                    or not isinstance(buffer, bytes)
                ):
                    raise TypeError(f"APIRequestContext: invalid multipart file payload {name}")
                fields.append((name, None, file_name, mime_type, buffer))
            elif isinstance(value, bytes):
                fields.append((name, None, name, None, value))
            elif isinstance(value, (str, bool, int, float)):
                fields.append((name, str(value), None, None, None))
            else:
                raise TypeError(f"APIRequestContext: unsupported multipart field {name}")
        return (4, None, None, [], fields), request_headers
    if data is _UNSET or data is None:
        return (0, None, None, [], []), request_headers
    content_type = next(
        (value for name, value in request_headers if name.lower() == "content-type"), None
    )
    is_json = (
        content_type is not None
        and content_type.split(";", 1)[0].strip().lower() == "application/json"
    )
    if isinstance(data, bytes):
        return (1, data, None, [], []), request_headers
    if isinstance(data, bytearray):
        return (1, bytes(data), None, [], []), request_headers
    if isinstance(data, memoryview):
        return (1, data.tobytes(), None, [], []), request_headers
    if isinstance(data, str):
        if is_json:
            try:
                json.loads(data)
                return (2, None, data, [], []), request_headers
            except json.JSONDecodeError:
                return (2, None, json.dumps(data), [], []), request_headers
        return (1, data.encode(), None, [], []), request_headers
    if isinstance(data, (Mapping, list, int, float, bool)):
        return (2, None, json.dumps(data), [], []), request_headers
    raise Error(f"Unsupported 'data' type: {type(data)}")


class APIResponse:
    def __init__(self, native: _native.NativeApiResponse) -> None:
        self._native = native

    @property
    def url(self) -> str:
        return self._native.url

    @property
    def status(self) -> int:
        return self._native.status

    @property
    def status_text(self) -> str:
        return self._native.status_text

    @property
    def ok(self) -> bool:
        return self._native.ok

    @property
    def headers_array(self) -> list[dict[str, str]]:
        return [{"name": name, "value": value} for name, value in self._native.headers_array()]

    @property
    def headers(self) -> dict[str, str]:
        result: dict[str, str] = {}
        for header in self.headers_array:
            name = header["name"].lower()
            value = header["value"]
            result[name] = value if name not in result else f"{result[name]}, {value}"
        return result

    async def body(self) -> bytes:
        try:
            return bytes(self._native.body())
        except BaseException as error:
            raise _map_request_error(error, "APIResponse.body") from error

    async def text(self) -> str:
        return (await self.body()).decode()

    async def json(self) -> Any:
        return json.loads(await self.text())

    async def dispose(self) -> None:
        self._native.dispose()

    async def log(self) -> list[str]:
        try:
            return list(self._native.log())
        except BaseException as error:
            raise _map_request_error(error, "APIResponse.log") from error


class APIRequestContext:
    def __init__(
        self,
        owner: APIRequest | None,
        native: _native.NativeApiRequestContext,
        base_url: str | None,
    ) -> None:
        self._owner = owner
        self._native = native
        self._base_url = base_url
        self._disposed = False
        self._dispose_reason: str | None = None

    async def _send(
        self,
        api: str,
        method: str,
        url: str,
        *,
        headers: Mapping[str, str] | Sequence[Mapping[str, str]] | None = None,
        data: object = _UNSET,
        form: object = _UNSET,
        multipart: object = _UNSET,
        params: object = None,
        timeout: float | None = None,
        fail_on_status_code: bool | None = None,
        ignore_https_errors: bool | None = None,
        max_redirects: int | None = None,
        max_retries: int | None = None,
        resolver: _Resolver | None = None,
    ) -> APIResponse:
        request_url = _optional_string(url, api, "url")
        if request_url is None:
            raise TypeError(f"{api}: url must be a string")
        if self._disposed:
            message = self._dispose_reason or "Target page, context or browser has been closed"
            raise Error(f"{api}: {message}")
        fail_on_status_code = _optional_boolean(fail_on_status_code, api, "fail_on_status_code")
        ignore_https_errors = _optional_boolean(ignore_https_errors, api, "ignore_https_errors")
        request_headers = _headers(headers, api)
        body, request_headers = _body_and_headers(data, form, multipart, request_headers)
        query = _query(params, api)
        redirect_limit = None
        if max_redirects is not None:
            redirect_limit = _integer(max_redirects, "max_redirects", 0)
        retry_limit = _integer(max_retries, "max_retries", 0)
        lookup_addresses: list[str] | None = None
        if resolver is not None:
            target = (
                urljoin(self._base_url, request_url) if self._base_url is not None else request_url
            )
            hostname = urlparse(target).hostname
            if hostname is None:
                raise Error(f"{api}: invalid URL")
            values = resolver(hostname)
            if inspect.isawaitable(values):
                values = await cast(Awaitable[object], values)
            if not isinstance(values, list):
                raise TypeError(f"{api}: __test_hook_lookup must return a list")
            lookup_addresses = []
            for value in cast(list[object], values):
                if not isinstance(value, Mapping):
                    raise TypeError(f"{api}: resolver entries must contain an address string")
                resolver_value = cast(Mapping[object, object], value)
                if not isinstance(resolver_value.get("address"), str):
                    raise TypeError(f"{api}: resolver entries must contain an address string")
                lookup_addresses.append(cast(str, resolver_value["address"]))
        try:
            native = await self._native.execute(
                method,
                request_url,
                request_headers,
                query,
                body,
                (
                    _milliseconds(timeout, f"{api}: timeout"),
                    fail_on_status_code,
                    ignore_https_errors,
                    redirect_limit,
                    retry_limit,
                    lookup_addresses,
                ),
            )
            return APIResponse(native)
        except BaseException as error:
            raise _map_request_error(error, api) from error

    async def fetch(self, url_or_request: object, **kwargs: object) -> APIResponse:
        options = dict(kwargs)
        method = options.pop("method", None)
        resolver = options.pop("__test_hook_lookup", None)
        if isinstance(url_or_request, str):
            request_url = url_or_request
            default_method = "GET"
        else:
            request_url = getattr(url_or_request, "url", None)
            default_method = getattr(url_or_request, "method", None)
            request_headers = getattr(url_or_request, "headers", None)
            request_body = getattr(url_or_request, "post_data_buffer", None)
            if (
                not isinstance(request_url, str)
                or not isinstance(default_method, str)
                or not isinstance(request_headers, Mapping)
            ):
                raise TypeError("APIRequestContext.fetch: first argument must be a URL or Request")
            options.setdefault("headers", cast(Mapping[str, str], request_headers))
            if not any(name in options for name in ("data", "form", "multipart")):
                if request_body is not None:
                    if not isinstance(request_body, bytes):
                        raise TypeError("APIRequestContext.fetch: Request post data must be bytes")
                    options["data"] = request_body
        if method is None:
            method = default_method
        if not isinstance(method, str):
            raise TypeError("APIRequestContext.fetch: method must be a string")
        if resolver is not None and not callable(resolver):
            raise TypeError("APIRequestContext.fetch: __test_hook_lookup must be callable")
        return await self._send_kwargs(
            "APIRequestContext.fetch",
            method,
            request_url,
            options,
            cast(Union[_Resolver, None], resolver),
        )

    async def delete(self, url: str, **kwargs: object) -> APIResponse:
        return await self._send_kwargs("APIRequestContext.delete", "DELETE", url, kwargs)

    async def get(self, url: str, **kwargs: object) -> APIResponse:
        return await self._send_kwargs("APIRequestContext.get", "GET", url, kwargs)

    async def head(self, url: str, **kwargs: object) -> APIResponse:
        return await self._send_kwargs("APIRequestContext.head", "HEAD", url, kwargs)

    async def patch(self, url: str, **kwargs: object) -> APIResponse:
        return await self._send_kwargs("APIRequestContext.patch", "PATCH", url, kwargs)

    async def post(self, url: str, **kwargs: object) -> APIResponse:
        return await self._send_kwargs("APIRequestContext.post", "POST", url, kwargs)

    async def put(self, url: str, **kwargs: object) -> APIResponse:
        return await self._send_kwargs("APIRequestContext.put", "PUT", url, kwargs)

    async def _send_kwargs(
        self,
        api: str,
        method: str,
        url: str,
        values: Mapping[str, object],
        resolver: _Resolver | None = None,
    ) -> APIResponse:
        options = dict(values)
        supported = {
            "headers",
            "data",
            "form",
            "multipart",
            "params",
            "timeout",
            "fail_on_status_code",
            "ignore_https_errors",
            "max_redirects",
            "max_retries",
        }
        unknown = sorted(set(options) - supported)
        if unknown:
            raise TypeError(f"{api}: unexpected option {unknown[0]}")
        return await self._send(
            api,
            method,
            url,
            headers=cast(
                Union[Mapping[str, str], Sequence[Mapping[str, str]], None],
                options.get("headers"),
            ),
            data=options.get("data", _UNSET),
            form=options.get("form", _UNSET),
            multipart=options.get("multipart", _UNSET),
            params=options.get("params"),
            timeout=cast(Union[float, None], options.get("timeout")),
            fail_on_status_code=cast(Union[bool, None], options.get("fail_on_status_code")),
            ignore_https_errors=cast(Union[bool, None], options.get("ignore_https_errors")),
            max_redirects=cast(Union[int, None], options.get("max_redirects")),
            max_retries=cast(Union[int, None], options.get("max_retries")),
            resolver=resolver,
        )

    async def dispose(self, *, reason: str | None = None) -> None:
        reason = _optional_string(reason, "APIRequestContext.dispose", "reason")
        self._dispose_from_owner(reason)
        if self._owner is not None:
            self._owner.discard_context(self)

    def _dispose_from_owner(self, reason: str | None) -> None:
        if self._disposed:
            return
        self._native.dispose(reason)
        self._disposed = True
        self._dispose_reason = reason

    async def storage_state(
        self,
        *,
        path: str | PathLike[str] | None = None,
        indexed_db: bool = False,
    ) -> StorageState:
        checked_indexed_db = _optional_boolean(
            indexed_db, "APIRequestContext.storage_state", "indexed_db"
        )
        if checked_indexed_db is None:
            raise TypeError("APIRequestContext.storage_state: indexed_db must be a boolean")
        origins = json.loads(self._native.storage_origins_json)
        if not checked_indexed_db:
            origins = [
                {key: value for key, value in origin.items() if key != "indexedDB"}
                for origin in origins
            ]
        native_cookies = await self._native.storage_cookies()
        cookies: list[StorageStateCookie] = []
        for cookie in native_cookies:
            name, value, domain, cookie_path, expires, http_only, secure, same_site = cookie
            cookies.append(
                StorageStateCookie(
                    name=name,
                    value=value,
                    domain=domain,
                    path=cookie_path,
                    expires=expires,
                    httpOnly=http_only,
                    secure=secure,
                    sameSite=same_site,
                )
            )
        state = StorageState(cookies=cookies, origins=origins)
        if path is not None:
            target = Path(path)
            await asyncio.to_thread(target.write_text, json.dumps(state, indent=2), "utf-8")
        return state

    def pwright_metrics(self) -> dict[str, int]:
        return dict(self._native.metrics())


class APIRequest:
    def __init__(self) -> None:
        self._contexts: set[APIRequestContext] = set()

    async def new_context(
        self,
        *,
        base_url: str | None = None,
        client_certificates: Sequence[ClientCertificate] | None = None,
        extra_http_headers: Mapping[str, str] | None = None,
        fail_on_status_code: bool = False,
        http_credentials: Mapping[str, object] | None = None,
        ignore_https_errors: bool = False,
        max_redirects: int = 20,
        proxy: object = None,
        storage_state: StorageState | str | PathLike[str] | None = None,
        timeout: float = 30_000,
        user_agent: str | None = None,
    ) -> APIRequestContext:
        api = "request.new_context"
        if proxy is not None:
            raise UnsupportedError(f"{api} option proxy is not installed")
        base_url = _optional_string(base_url, api, "base_url")
        user_agent = _optional_string(user_agent, api, "user_agent")
        checked_fail_on_status_code = _optional_boolean(
            fail_on_status_code, api, "fail_on_status_code"
        )
        checked_ignore_https_errors = _optional_boolean(
            ignore_https_errors, api, "ignore_https_errors"
        )
        if checked_fail_on_status_code is None:
            raise TypeError(f"{api}: fail_on_status_code must be a boolean")
        if checked_ignore_https_errors is None:
            raise TypeError(f"{api}: ignore_https_errors must be a boolean")
        credentials: tuple[str, str, str | None, bool] | None = None
        if http_credentials is not None:
            username = http_credentials.get("username")
            password = http_credentials.get("password")
            origin = http_credentials.get("origin")
            send = http_credentials.get("send", "unauthorized")
            if not isinstance(username, str) or not isinstance(password, str):
                raise TypeError(f"{api}: http_credentials username and password must be strings")
            if origin is not None and not isinstance(origin, str):
                raise TypeError(f"{api}: http_credentials origin must be a string")
            if send not in ("unauthorized", "always"):
                raise TypeError(f'{api}: http_credentials send must be "unauthorized" or "always"')
            credentials = (username, password, origin, send == "always")
        native_client_certificates = await _normalize_client_certificates(client_certificates, api)
        cookies, origins = await _normalize_storage_state(storage_state)
        native_type = getattr(_native, "NativeApiRequestContext", None)
        if native_type is None:
            raise UnsupportedError("request.new_context requires the api-request native profile")
        try:
            native = native_type(
                (
                    base_url,
                    user_agent or _default_user_agent(),
                    _headers(extra_http_headers, api),
                    cast(int, _milliseconds(timeout, f"{api}: timeout", 30_000)),
                    checked_fail_on_status_code,
                    checked_ignore_https_errors,
                    _integer(max_redirects, "max_redirects", 20),
                ),
                credentials,
                native_client_certificates,
                json.dumps(origins),
            )
            if cookies:
                await native.add_cookies(cookies)
        except BaseException as error:
            raise _map_request_error(error, api) from error
        context = APIRequestContext(self, native, base_url)
        self._contexts.add(context)
        return context

    def discard_context(self, context: APIRequestContext) -> None:
        self._contexts.discard(context)

    async def _dispose_all(self) -> None:
        contexts = tuple(self._contexts)
        for context in contexts:
            await context.dispose()


__all__ = [
    "APIRequest",
    "APIRequestContext",
    "APIResponse",
    "ClientCertificate",
    "StorageState",
    "StorageStateCookie",
]
