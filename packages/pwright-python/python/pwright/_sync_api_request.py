"""Sync wrappers for the Rust-backed APIRequest facade."""

from __future__ import annotations

import asyncio
import concurrent.futures
import threading
from collections.abc import Coroutine, Mapping, Sequence
from os import PathLike
from typing import Any, TypeVar

from . import async_api
from ._async_api_request import ClientCertificate, StorageState, StorageStateCookie

T = TypeVar("T")


def _run_callback_safe(
    bridge: Any,
    operation: Coroutine[Any, Any, T],
    *,
    worker_name: str,
) -> T:
    if threading.get_ident() != bridge._thread_id:
        return bridge.run(operation)
    result: concurrent.futures.Future[T] = concurrent.futures.Future()

    def run_operation() -> None:
        try:
            result.set_result(asyncio.run(operation))
        except BaseException as error:
            result.set_exception(error)

    worker = threading.Thread(target=run_operation, name=worker_name)
    worker.start()
    worker.join()
    return result.result()


class APIResponse:
    def __init__(self, bridge: Any, value: async_api.APIResponse) -> None:
        self._bridge = bridge
        self._value = value

    @property
    def url(self) -> str:
        return self._value.url

    @property
    def status(self) -> int:
        return self._value.status

    @property
    def status_text(self) -> str:
        return self._value.status_text

    @property
    def ok(self) -> bool:
        return self._value.ok

    @property
    def headers(self) -> dict[str, str]:
        return self._value.headers

    @property
    def headers_array(self) -> list[dict[str, str]]:
        return self._value.headers_array

    def body(self) -> bytes:
        return _run_callback_safe(
            self._bridge,
            self._value.body(),
            worker_name="pwright-sync-api-response",
        )

    def text(self) -> str:
        return _run_callback_safe(
            self._bridge,
            self._value.text(),
            worker_name="pwright-sync-api-response",
        )

    def json(self) -> Any:
        return _run_callback_safe(
            self._bridge,
            self._value.json(),
            worker_name="pwright-sync-api-response",
        )

    def dispose(self) -> None:
        _run_callback_safe(
            self._bridge,
            self._value.dispose(),
            worker_name="pwright-sync-api-response",
        )

    def log(self) -> list[str]:
        return _run_callback_safe(
            self._bridge,
            self._value.log(),
            worker_name="pwright-sync-api-response",
        )


class APIRequestContext:
    def __init__(self, bridge: Any, value: async_api.APIRequestContext) -> None:
        self._bridge = bridge
        self._value = value

    def _call(self, method: str, url: str, kwargs: dict[str, object]) -> APIResponse:
        operation = getattr(self._value, method)(url, **kwargs)
        return APIResponse(self._bridge, self._bridge.run(operation))

    def fetch(self, url_or_request: str, **kwargs: object) -> APIResponse:
        return self._call("fetch", url_or_request, kwargs)

    def delete(self, url: str, **kwargs: object) -> APIResponse:
        return self._call("delete", url, kwargs)

    def get(self, url: str, **kwargs: object) -> APIResponse:
        return self._call("get", url, kwargs)

    def head(self, url: str, **kwargs: object) -> APIResponse:
        return self._call("head", url, kwargs)

    def patch(self, url: str, **kwargs: object) -> APIResponse:
        return self._call("patch", url, kwargs)

    def post(self, url: str, **kwargs: object) -> APIResponse:
        return self._call("post", url, kwargs)

    def put(self, url: str, **kwargs: object) -> APIResponse:
        return self._call("put", url, kwargs)

    def dispose(self, *, reason: str | None = None) -> None:
        self._bridge.run(self._value.dispose(reason=reason))

    def storage_state(
        self,
        *,
        path: str | PathLike[str] | None = None,
        indexed_db: bool = False,
    ) -> StorageState:
        return self._bridge.run(self._value.storage_state(path=path, indexed_db=indexed_db))

    def pwright_metrics(self) -> dict[str, int]:
        return self._value.pwright_metrics()


class APIRequest:
    def __init__(self, bridge: Any, value: async_api.APIRequest) -> None:
        self._bridge = bridge
        self._value = value

    def new_context(
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
        value = self._bridge.run(
            self._value.new_context(
                base_url=base_url,
                client_certificates=client_certificates,
                extra_http_headers=extra_http_headers,
                fail_on_status_code=fail_on_status_code,
                http_credentials=http_credentials,
                ignore_https_errors=ignore_https_errors,
                max_redirects=max_redirects,
                proxy=proxy,
                storage_state=storage_state,
                timeout=timeout,
                user_agent=user_agent,
            )
        )
        return APIRequestContext(self._bridge, value)


__all__ = [
    "APIRequest",
    "APIRequestContext",
    "APIResponse",
    "ClientCertificate",
    "StorageState",
    "StorageStateCookie",
]
