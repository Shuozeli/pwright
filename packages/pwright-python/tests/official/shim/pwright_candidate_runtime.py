"""Candidate-only fixture adapter for upstream Playwright Python execution."""

from __future__ import annotations

import threading
from collections.abc import Generator
from contextlib import contextmanager
from importlib import import_module
from typing import Any


class _ThreadSafeRequestWaiter:
    """Minimal future shape consumed by the upstream Twisted request hook."""

    def __init__(self) -> None:
        self._loop = self
        self._event = threading.Event()
        self._value: Any = None

    def call_soon_threadsafe(self, callback: Any, *args: object) -> None:
        callback(*args)

    def set_result(self, value: Any) -> None:
        self._value = value
        self._event.set()

    def result(self, timeout: float = 30) -> Any:
        if not self._event.wait(timeout):
            raise TimeoutError("candidate upstream server request waiter timed out")
        return self._value


def pytest_configure() -> None:
    upstream_server = import_module("tests.server")
    upstream_errors = import_module("playwright._impl._errors")
    candidate = import_module("pwright")
    setattr(upstream_errors, "Error", getattr(candidate, "Error"))

    @contextmanager
    def expect_request(server: Any, path: str) -> Generator[Any, None, None]:
        """Bridge the upstream sync helper to Pwright's dedicated loop thread."""

        waiter = _ThreadSafeRequestWaiter()
        server.request_subscribers[path] = waiter
        wrapper = upstream_server.ExpectResponse()
        yield wrapper
        wrapper._value = waiter.result()

    upstream_server.Server.expect_request = expect_request
