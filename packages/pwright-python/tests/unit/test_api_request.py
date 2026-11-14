from __future__ import annotations

from pathlib import Path

import pytest
from pwright import Error
from pwright.async_api import StorageState, async_playwright
from pwright.sync_api import sync_playwright

STORAGE_STATE: StorageState = {
    "cookies": [
        {
            "name": "a",
            "value": "b",
            "domain": "a.b.one.com",
            "path": "/",
            "expires": -1,
            "httpOnly": False,
            "secure": False,
            "sameSite": "Lax",
        }
    ],
    "origins": [],
}


@pytest.mark.asyncio
async def test_async_request_storage_state_round_trips_through_native_owner(
    tmp_path: Path,
) -> None:
    # Arrange
    playwright = await async_playwright().start()
    context = await playwright.request.new_context(storage_state=STORAGE_STATE)
    path = tmp_path / "state.json"

    # Act
    state = await context.storage_state(path=path)

    # Assert
    assert state == STORAGE_STATE
    assert path.is_file()
    assert context.pwright_metrics()["live_responses"] == 0
    await playwright.stop()


@pytest.mark.asyncio
async def test_async_request_dispose_preserves_custom_reason() -> None:
    # Arrange
    playwright = await async_playwright().start()
    context = await playwright.request.new_context()

    # Act
    await context.dispose(reason="My reason")

    # Assert
    with pytest.raises(Error, match="My reason"):
        await context.get("http://invalid.test/")
    await playwright.stop()


@pytest.mark.asyncio
async def test_async_client_certificate_paths_are_retained_lazily(tmp_path: Path) -> None:
    # Arrange
    certificate = tmp_path / "client.pem"
    private_key = tmp_path / "client-key.pem"
    certificate.write_bytes(b"not-decoded-for-an-unmatched-origin")
    private_key.write_bytes(b"not-decoded-for-an-unmatched-origin")
    playwright = await async_playwright().start()

    # Act
    context = await playwright.request.new_context(
        client_certificates=[
            {
                "origin": "https://unmatched.example",
                "certPath": certificate,
                "keyPath": private_key,
            }
        ]
    )

    # Assert
    assert context.pwright_metrics()["live_responses"] == 0
    await playwright.stop()


@pytest.mark.asyncio
async def test_async_client_certificate_rejects_unpaired_pem_before_native_work() -> None:
    # Arrange
    playwright = await async_playwright().start()
    raised: TypeError | None = None

    # Act
    try:
        await playwright.request.new_context(
            client_certificates=[
                {
                    "origin": "https://example.test",
                    "cert": b"certificate-only",
                }
            ]
        )
    except TypeError as error:
        raised = error
    await playwright.stop()

    # Assert
    assert raised is not None
    assert "cert and key must be specified together" in str(raised)


@pytest.mark.asyncio
async def test_async_client_certificate_rejects_mixed_pfx_before_native_work() -> None:
    # Arrange
    playwright = await async_playwright().start()
    raised: TypeError | None = None

    # Act
    try:
        await playwright.request.new_context(
            client_certificates=[
                {
                    "origin": "https://example.test",
                    "cert": b"certificate",
                    "key": b"key",
                    "pfx": b"archive",
                }
            ]
        )
    except TypeError as error:
        raised = error
    await playwright.stop()

    # Assert
    assert raised is not None
    assert "pfx cannot be combined with cert and key" in str(raised)


def test_sync_request_storage_state_uses_bounded_loop_owner(tmp_path: Path) -> None:
    # Arrange
    path = tmp_path / "state.json"

    # Act
    with sync_playwright() as playwright:
        context = playwright.request.new_context(storage_state=STORAGE_STATE)
        state = context.storage_state(path=path)

    # Assert
    assert state == STORAGE_STATE
    assert path.is_file()
