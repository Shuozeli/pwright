"""Playwright-shaped Python facade backed by the Pwright Rust host."""

from ._abi import NATIVE_ABI_VERSION, native_capabilities
from ._errors import (
    Error,
    InvalidArgumentError,
    JavascriptError,
    ResourceExhaustedError,
    TargetClosedError,
    TimeoutError,
    UnsupportedError,
)

__all__ = [
    "Error",
    "InvalidArgumentError",
    "JavascriptError",
    "NATIVE_ABI_VERSION",
    "ResourceExhaustedError",
    "TargetClosedError",
    "TimeoutError",
    "UnsupportedError",
    "native_capabilities",
]
