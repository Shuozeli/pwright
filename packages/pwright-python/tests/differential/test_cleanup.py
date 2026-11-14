# pyright: reportPrivateUsage=false

import asyncio
import gc
import os
import resource
import sys

import pytest
from pwright.async_api import async_playwright

ENDPOINT = os.environ.get("PWRIGHT_CDP_ENDPOINT", "")

ZERO_FFI = {
    "pending": 0,
    "queued": 0,
    "queued_commands": 0,
    "running": 0,
    "reserved_bytes": 0,
    "connects": 0,
    "commands": 0,
    "closes": 0,
    "watches": 0,
}

ZERO_SESSION = {
    "connected": False,
    "live_objects": 0,
    "pending_calls": 0,
    "topology_drivers": 0,
    "active_leases": 0,
    "live_scopes": 0,
    "retained_event_bytes": 0,
    "active_operations": 0,
    "cleanup_in_flight": 0,
}


async def _wait_for_zero(environment: object) -> None:
    for _ in range(100):
        if environment.metrics() == ZERO_FFI:  # type: ignore[attr-defined]
            return
        await asyncio.sleep(0.01)
    assert environment.metrics() == ZERO_FFI  # type: ignore[attr-defined]


def _maximum_rss_bytes() -> int:
    value = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return int(value if sys.platform == "darwin" else value * 1024)


@pytest.mark.skipif(not ENDPOINT, reason="PWRIGHT_CDP_ENDPOINT is required")
@pytest.mark.asyncio
async def test_repeated_attach_failure_cancellation_and_owner_stop_reclaim_everything() -> None:
    # Arrange
    manager = async_playwright()
    playwright = await manager.start()
    environment = playwright._environment

    # Act: Rust-owned Context/Page creation and explicit child-first cleanup.
    browser = await playwright.chromium.connect_over_cdp(ENDPOINT)
    native = browser._native
    context = await browser.new_context()
    page = await context.new_page()
    assert page.context is context
    assert context.browser is browser
    assert context in browser.contexts
    assert page in context.pages
    await page.close()
    assert page.is_closed()
    await context.close()
    await browser.close()
    assert native.metrics() == ZERO_SESSION
    assert environment.metrics() == ZERO_FFI

    # Act: repeated explicit detach.
    for _ in range(3):
        browser = await playwright.chromium.connect_over_cdp(ENDPOINT)
        assert browser.contexts
        native = browser._native
        await browser.close()
        assert native.metrics() == ZERO_SESSION
        assert environment.metrics() == ZERO_FFI

    # Act: failed attachment settles its admission.
    with pytest.raises(Exception):
        await playwright.chromium.connect_over_cdp("http://127.0.0.1:1", timeout=100)
    assert environment.metrics() == ZERO_FFI

    # Act: cancelling the Python awaiter cannot strand a native ticket.
    pending = asyncio.create_task(
        playwright.chromium.connect_over_cdp("http://10.255.255.1:9", timeout=30_000)
    )
    await asyncio.sleep(0)
    if not pending.done():
        pending.cancel()
        with pytest.raises(asyncio.CancelledError):
            await pending
    else:
        with pytest.raises(Exception):
            await pending
    await _wait_for_zero(environment)

    # Act: repeated sessions have a bounded process-memory high-water trend.
    baseline_rss = _maximum_rss_bytes()
    for _ in range(20):
        browser = await playwright.chromium.connect_over_cdp(ENDPOINT)
        assert browser.contexts
        await browser.close()
    gc.collect()
    rss_growth = _maximum_rss_bytes() - baseline_rss
    print(f"pwright-python-memory baseline={baseline_rss} growth={rss_growth} iterations=20")
    assert rss_growth < 64 * 1024 * 1024
    assert environment.metrics() == ZERO_FFI

    # Act: owner stop closes an attached session without Browser.close().
    await playwright.chromium.connect_over_cdp(ENDPOINT)
    await playwright.stop()

    # Assert
    assert environment.metrics() == ZERO_FFI
    assert environment.cleanup_report() == {
        "sessions_closed": 1,
        "sessions_zero": True,
        "ffi_zero": True,
    }
