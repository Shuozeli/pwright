"""Native error taxonomy; classification originates in Rust."""

from __future__ import annotations

from ._native import (
    Error,
    InvalidArgumentError,
    JavascriptError,
    ResourceExhaustedError,
    TargetClosedError,
    TimeoutError,
    UnsupportedError,
)


def _message(error: BaseException) -> str:
    return str(error)


def _name(error: BaseException) -> str:
    override = getattr(error, "_pwright_name", None)
    return type(error).__name__ if override is None else str(override)


def _set_name(error: BaseException, value: str) -> None:
    setattr(error, "_pwright_name", value)


def rewrite_api_error(error: Exception, api_name: str, *, message: str | None = None) -> Exception:
    prefix = f"{api_name}:"
    if message is None and str(error).startswith(prefix):
        return error

    rewritten = type(error)(f"{prefix} {str(error) if message is None else message}")
    if hasattr(error, "_pwright_name"):
        setattr(rewritten, "_pwright_name", getattr(error, "_pwright_name"))
    if hasattr(error, "stack"):
        setattr(rewritten, "stack", getattr(error, "stack"))
    return rewritten


for _error_type in (
    Error,
    TimeoutError,
    TargetClosedError,
    ResourceExhaustedError,
    JavascriptError,
    InvalidArgumentError,
    UnsupportedError,
):
    setattr(_error_type, "message", property(_message))
    setattr(_error_type, "name", property(_name, _set_name))

__all__ = [
    "Error",
    "InvalidArgumentError",
    "JavascriptError",
    "ResourceExhaustedError",
    "TargetClosedError",
    "TimeoutError",
    "UnsupportedError",
]
