"""Guard the exact public launch call in one pinned official Python test."""

from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any

ASSET_SHA256 = "9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f"
TEST_NAME = "test_browser_type_launch_should_reject_if_launched_browser_fails_immediately"
NONCE = re.compile(r"^[0-9a-f]{32}$")


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
        raise RuntimeError("strict dummy launch asset is not the pinned executable")
    return asset


def _common() -> dict[str, str]:
    api = os.environ["PWRIGHT_PYTHON_DUMMY_API"]
    mode = os.environ["PWRIGHT_OFFICIAL_MODE"]
    nonce = os.environ["PWRIGHT_PYTHON_DUMMY_NONCE"]
    if api not in {"async", "sync"} or mode not in {"oracle", "candidate"}:
        raise RuntimeError("strict dummy launch lane is invalid")
    if NONCE.fullmatch(nonce) is None:
        raise RuntimeError("strict dummy launch nonce is invalid")
    return {
        "api": api,
        "mode": mode,
        "runNonce": nonce,
        "executablePath": str(_asset()),
        "assetSha256": ASSET_SHA256,
    }


def _regular_receipt(name: str) -> Path:
    path = Path(os.environ[name])
    if not path.is_absolute() or path.is_symlink() or not path.is_file():
        raise RuntimeError(f"strict dummy launch receipt is missing or linked: {name}")
    return path


def _append_public(event: str, **fields: object) -> None:
    path = _regular_receipt("PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT")
    flags = os.O_WRONLY | os.O_APPEND | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        row = {"event": event, **_common(), **fields}
        os.write(descriptor, (json.dumps(row, sort_keys=True) + "\n").encode())
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _process_digest() -> str:
    path = _regular_receipt("PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT")
    data = path.read_bytes()
    if not data.endswith(b"\n"):
        raise RuntimeError("strict dummy process receipt is absent")
    try:
        rows = [json.loads(line) for line in data.splitlines()]
    except json.JSONDecodeError as error:
        raise RuntimeError("strict dummy process receipt is malformed") from error
    if len(rows) != 1 or rows[0].get("event") != "dummy-node-exited":
        raise RuntimeError("strict dummy process did not exit exactly once")
    if any(rows[0].get(key) != value for key, value in _common().items()):
        raise RuntimeError("strict dummy process receipt belongs to another launch")
    if rows[0].get("exitCode") != 1:
        raise RuntimeError("strict dummy process did not exit with status one")
    return hashlib.sha256(data).hexdigest()


def _validate(self: Any, args: tuple[object, ...], options: dict[str, object]) -> None:
    if getattr(self, "name", None) != "chromium" or args:
        raise RuntimeError("strict dummy launch requires Chromium keyword options")
    if set(options) != {"headless", "executable_path"}:
        raise RuntimeError("strict dummy launch options differ from the pinned test")
    if options["headless"] is not True or options["executable_path"] != _asset():
        raise RuntimeError("strict dummy launch rejected a non-pinned executable")


def pytest_collection_modifyitems(items: list[Any]) -> None:
    api = os.environ["PWRIGHT_PYTHON_DUMMY_API"]
    upstream = Path(os.environ["PWRIGHT_PYTHON_DUMMY_UPSTREAM"])
    expected = upstream / f"tests/{api}/test_launcher.py"
    if (
        api not in {"async", "sync"}
        or len(items) != 1
        or items[0].name != f"{TEST_NAME}[chromium]"
        or Path(items[0].path) != expected
    ):
        raise RuntimeError("strict dummy guard requires one exact Chromium test")


def pytest_configure() -> None:
    common = _common()
    _regular_receipt("PWRIGHT_PYTHON_DUMMY_LAUNCH_RECEIPT")
    _regular_receipt("PWRIGHT_PYTHON_DUMMY_PROCESS_RECEIPT")
    if common["api"] == "async":
        from playwright.async_api import BrowserType

        original = BrowserType.launch

        async def guarded(self: Any, *args: object, **options: object) -> Any:
            _validate(self, args, options)
            _append_public("public-launch-invoked")
            try:
                browser = await original(self, *args, **options)
            except Exception as error:
                _append_public(
                    "public-launch-failed",
                    errorClass=type(error).__name__,
                    errorMessage=str(error),
                    processReceiptSha256=_process_digest(),
                )
                raise
            _append_public("public-launch-succeeded")
            return browser

        BrowserType.launch = guarded
    else:
        from playwright.sync_api import BrowserType

        original_sync = BrowserType.launch

        def guarded_sync(self: Any, *args: object, **options: object) -> Any:
            _validate(self, args, options)
            _append_public("public-launch-invoked")
            try:
                browser = original_sync(self, *args, **options)
            except Exception as error:
                _append_public(
                    "public-launch-failed",
                    errorClass=type(error).__name__,
                    errorMessage=str(error),
                    processReceiptSha256=_process_digest(),
                )
                raise
            _append_public("public-launch-succeeded")
            return browser

        BrowserType.launch = guarded_sync
