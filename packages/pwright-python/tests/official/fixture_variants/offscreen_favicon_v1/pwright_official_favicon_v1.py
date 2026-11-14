"""Case-scoped fixture variant for the shared Chromium favicon 404.

This module is opt-in and lives outside the historical official/runtime tree.
It does not change the upstream test assertion or suppress console messages.
"""

from __future__ import annotations

import base64
import hashlib
from pathlib import Path
from threading import Event
from typing import Any, cast

import pytest

VARIANT_ID = "offscreen-favicon-v1"
CASE_ID = "tests/async/test_click.py::test_click_offscreen_buttons[chromium]"
ICON_PATH = Path(__file__).with_name("favicon.ico.b64")
ICON_ROUTE = "/favicon.ico"
PROPERTY_VARIANT = "pwright.fixtureVariant"
PROPERTY_ICON_SHA256 = "pwright.fixtureVariantIconSha256"
PROPERTY_SERVED = "pwright.fixtureVariantServed"


def _icon_bytes() -> bytes:
    if ICON_PATH.is_symlink() or not ICON_PATH.is_file():
        raise pytest.UsageError("versioned favicon asset is missing or a symlink")
    try:
        icon = base64.b64decode(ICON_PATH.read_bytes().strip(), validate=True)
    except (OSError, ValueError) as error:
        raise pytest.UsageError("versioned favicon asset is not valid base64") from error
    # A complete 1x1, 32-bit Windows ICO with one opaque red pixel.
    if (
        len(icon) != 70
        or icon[:6] != b"\x00\x00\x01\x00\x01\x00"
        or icon[6:10] != b"\x01\x01\x00\x00"
        or int.from_bytes(icon[14:18], "little") != 48
        or int.from_bytes(icon[18:22], "little") != 22
    ):
        raise pytest.UsageError("versioned favicon asset is not the expected ICO")
    return icon


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    if len(items) != 1 or items[0].nodeid != CASE_ID:
        raise pytest.UsageError("offscreen favicon v1 may run only the exact pinned Chromium case")


@pytest.fixture(autouse=True)
def serve_offscreen_favicon_v1(request: pytest.FixtureRequest, server: Any):
    node = cast(pytest.Item, getattr(request, "node"))
    if node.nodeid != CASE_ID:
        raise pytest.UsageError("offscreen favicon v1 fixture reached another case")
    icon = _icon_bytes()
    if ICON_ROUTE in server.routes:
        raise pytest.UsageError("favicon route already exists before variant setup")

    node.user_properties.append((PROPERTY_VARIANT, VARIANT_ID))
    node.user_properties.append((PROPERTY_ICON_SHA256, hashlib.sha256(icon).hexdigest()))
    served = Event()

    def respond(http_request: Any) -> None:
        http_request.setResponseCode(200)
        http_request.setHeader(b"content-type", b"image/x-icon")
        http_request.setHeader(b"content-length", str(len(icon)).encode("ascii"))
        if http_request.method != b"HEAD":
            http_request.write(icon)
        http_request.finish()
        if not served.is_set():
            served.set()
            node.user_properties.append((PROPERTY_SERVED, ICON_ROUTE))

    server.set_route(ICON_ROUTE, respond)
    try:
        yield
    finally:
        if server.routes.get(ICON_ROUTE) is respond:
            del server.routes[ICON_ROUTE]
