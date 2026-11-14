"""Collection-only names that Pwright has not implemented yet.

The upstream Python suite imports public API classes in annotations while it is
being collected.  Keeping these explicit placeholders in the test adapter lets
collection reach the first executable discrepancy without pretending that the
production ``pwright`` package implements the corresponding capability.
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import Any

from pwright import UnsupportedError

PUBLIC_TYPE_NAMES = (
    "APIRequest",
    "APIRequestContext",
    "APIResponse",
    "CDPSession",
    "ChromiumBrowserContext",
    "ClientCertificate",
    "ConsoleMessage",
    "Cookie",
    "Dialog",
    "Download",
    "ElementHandle",
    "FileChooser",
    "FilePayload",
    "FloatRect",
    "Frame",
    "FrameLocator",
    "Geolocation",
    "HttpCredentials",
    "JSHandle",
    "Keyboard",
    "Locator",
    "Mouse",
    "PdfMargins",
    "Position",
    "ProxySettings",
    "Request",
    "ResourceTiming",
    "Route",
    "Selectors",
    "SourceLocation",
    "StorageState",
    "StorageStateCookie",
    "Touchscreen",
    "Video",
    "ViewportSize",
    "WebError",
    "WebSocket",
    "WebSocketRoute",
    "Worker",
)


class _UnsupportedPublicType:
    """Fail visibly if an annotation-only placeholder reaches execution."""

    def __init__(self, *_args: object, **_kwargs: object) -> None:
        raise UnsupportedError(
            f"official-suite adapter reached unimplemented public type {type(self).__name__}"
        )


def install_public_type_placeholders(
    namespace: dict[str, Any], names: Iterable[str] = PUBLIC_TYPE_NAMES
) -> list[str]:
    installed: list[str] = []
    for name in names:
        if name in namespace:
            continue
        namespace[name] = type(
            name,
            (_UnsupportedPublicType,),
            {"__module__": namespace["__name__"]},
        )
        installed.append(name)
    return installed


def expect(*_args: object, **_kwargs: object) -> None:
    raise UnsupportedError("official-suite adapter reached unimplemented expect assertions")


__all__ = ["PUBLIC_TYPE_NAMES", "expect", "install_public_type_placeholders"]
