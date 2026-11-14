"""Guard one pinned public BrowserType.launch of the inert upstream Node asset."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
from typing import Any

ASSET_SHA256 = "9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f"
TEST_NAME = "test_browser_type_launch_should_reject_if_launched_browser_fails_immediately"


def _asset() -> Path:
    upstream = Path(os.environ["PWRIGHT_PYTHON_DUMMY_UPSTREAM"])
    asset = upstream / "tests/assets/dummy_bad_browser_executable.js"
    if (
        not upstream.is_absolute()
        or asset.is_symlink()
        or not asset.is_file()
        or asset.resolve() != asset
        or hashlib.sha256(asset.read_bytes()).hexdigest() != ASSET_SHA256
        or not os.access(asset, os.X_OK)
    ):
        raise RuntimeError("dummy launch asset is not the pinned executable")
    return asset


def _record(event: str) -> None:
    receipt = Path(os.environ["PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT"])
    if receipt.is_symlink() or not receipt.is_file():
        raise RuntimeError("dummy launch receipt is missing or linked")
    with receipt.open("a", encoding="utf-8") as stream:
        stream.write(
            json.dumps(
                {
                    "event": event,
                    "api": os.environ["PWRIGHT_PYTHON_DUMMY_API"],
                    "mode": os.environ["PWRIGHT_OFFICIAL_MODE"],
                    "executablePath": str(_asset()),
                    "assetSha256": ASSET_SHA256,
                },
                sort_keys=True,
            )
            + "\n"
        )


def _validate(self: Any, args: tuple[object, ...], options: dict[str, object]) -> None:
    if getattr(self, "name", None) != "chromium" or args:
        raise RuntimeError("dummy launch guard requires Chromium and keyword options")
    if set(options) != {"headless", "executable_path"}:
        raise RuntimeError("dummy launch guard requires only headless and executable_path")
    if options["headless"] is not True or options["executable_path"] != _asset():
        raise RuntimeError("dummy launch guard rejected a non-pinned launch")


def pytest_collection_modifyitems(items: list[Any]) -> None:
    api = os.environ["PWRIGHT_PYTHON_DUMMY_API"]
    expected = f"tests/{api}/test_launcher.py"
    upstream = Path(os.environ["PWRIGHT_PYTHON_DUMMY_UPSTREAM"])
    if (
        api not in {"async", "sync"}
        or len(items) != 1
        or items[0].name != f"{TEST_NAME}[chromium]"
        or Path(items[0].path) != upstream / expected
    ):
        raise RuntimeError("dummy launch guard requires exactly one pinned Chromium test")


def pytest_configure() -> None:
    api = os.environ["PWRIGHT_PYTHON_DUMMY_API"]
    mode = os.environ["PWRIGHT_OFFICIAL_MODE"]
    if api not in {"async", "sync"} or mode not in {"oracle", "candidate"}:
        raise RuntimeError("dummy launch guard mode or API is invalid")
    _asset()
    if api == "async":
        from playwright.async_api import BrowserType

        original = BrowserType.launch

        async def guarded(self: Any, *args: object, **options: object) -> Any:
            _validate(self, args, options)
            _record("public-launch-invoked")
            try:
                browser = await original(self, *args, **options)
            except BaseException:
                _record("public-launch-failed")
                raise
            _record("public-launch-succeeded")
            return browser

        BrowserType.launch = guarded
    else:
        from playwright.sync_api import BrowserType

        original_sync = BrowserType.launch

        def guarded_sync(self: Any, *args: object, **options: object) -> Any:
            _validate(self, args, options)
            _record("public-launch-invoked")
            try:
                browser = original_sync(self, *args, **options)
            except BaseException:
                _record("public-launch-failed")
                raise
            _record("public-launch-succeeded")
            return browser

        BrowserType.launch = guarded_sync
