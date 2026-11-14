import os

import pytest
from playwright.async_api import async_playwright as oracle_playwright
from playwright.sync_api import sync_playwright as oracle_sync_playwright
from pwright.async_api import async_playwright as candidate_playwright
from pwright.sync_api import sync_playwright as candidate_sync_playwright

ENDPOINT = os.environ.get("PWRIGHT_CDP_ENDPOINT", "")
TEST_PAGE_URL = os.environ.get("PWRIGHT_TEST_PAGE_URL", "")
TEST_PAGE_TOKEN = "pwright-python-owned-marker-20260922"
NAVIGATION_URL = (
    "data:text/html,<title>pwright-python-navigation</title>"
    "<main id='result'>Rust owned browser behavior</main>"
)


async def _observe_oracle() -> dict[str, object]:
    async with oracle_playwright() as playwright:
        browser = await playwright.chromium.connect_over_cdp(ENDPOINT)
        try:
            contexts = browser.contexts
            pages = [page for context in contexts for page in context.pages]
            return {
                "connected": browser.is_connected(),
                "version": browser.version,
                "contexts_nonempty": bool(contexts),
                "pages_nonempty": bool(pages),
                "title": await pages[0].title() if pages else None,
                "browser_type_identity": browser.browser_type is playwright.chromium,
                "context_identity": contexts[0].browser is browser if contexts else None,
                "context_repeat_identity": (
                    contexts[0] is browser.contexts[0] if contexts else None
                ),
                "page_context_identity": pages[0].context is contexts[0] if pages else None,
                "page_repeat_identity": (pages[0] is contexts[0].pages[0] if pages else None),
            }
        finally:
            await browser.close()


async def _observe_candidate() -> dict[str, object]:
    async with candidate_playwright() as playwright:
        browser = await playwright.chromium.connect_over_cdp(ENDPOINT)
        try:
            contexts = browser.contexts
            pages = [page for context in contexts for page in context.pages]
            return {
                "connected": browser.is_connected(),
                "version": browser.version,
                "contexts_nonempty": bool(contexts),
                "pages_nonempty": bool(pages),
                "title": await pages[0].title() if pages else None,
                "browser_type_identity": browser.browser_type is playwright.chromium,
                "context_identity": contexts[0].browser is browser if contexts else None,
                "context_repeat_identity": (
                    contexts[0] is browser.contexts[0] if contexts else None
                ),
                "page_context_identity": pages[0].context is contexts[0] if pages else None,
                "page_repeat_identity": (pages[0] is contexts[0].pages[0] if pages else None),
            }
        finally:
            await browser.close()


@pytest.mark.skipif(not ENDPOINT, reason="PWRIGHT_CDP_ENDPOINT is required")
@pytest.mark.asyncio
async def test_attach_enumeration_matches_pinned_playwright() -> None:
    # Arrange
    oracle = await _observe_oracle()

    # Act
    candidate = await _observe_candidate()

    # Assert
    assert candidate == oracle


def _observe_oracle_sync() -> dict[str, object]:
    with oracle_sync_playwright() as playwright:
        browser = playwright.chromium.connect_over_cdp(ENDPOINT)
        try:
            contexts = browser.contexts
            pages = [page for context in contexts for page in context.pages]
            return {
                "connected": browser.is_connected(),
                "version": browser.version,
                "contexts_nonempty": bool(contexts),
                "pages_nonempty": bool(pages),
                "title": pages[0].title() if pages else None,
                "browser_type_identity": browser.browser_type is playwright.chromium,
                "context_identity": contexts[0].browser is browser if contexts else None,
                "context_repeat_identity": (
                    contexts[0] is browser.contexts[0] if contexts else None
                ),
                "page_context_identity": pages[0].context is contexts[0] if pages else None,
                "page_repeat_identity": (pages[0] is contexts[0].pages[0] if pages else None),
            }
        finally:
            browser.close()


def _observe_candidate_sync() -> dict[str, object]:
    with candidate_sync_playwright() as playwright:
        browser = playwright.chromium.connect_over_cdp(ENDPOINT)
        try:
            contexts = browser.contexts
            pages = [page for context in contexts for page in context.pages]
            return {
                "connected": browser.is_connected(),
                "version": browser.version,
                "contexts_nonempty": bool(contexts),
                "pages_nonempty": bool(pages),
                "title": pages[0].title() if pages else None,
                "browser_type_identity": browser.browser_type is playwright.chromium,
                "context_identity": contexts[0].browser is browser if contexts else None,
                "context_repeat_identity": (
                    contexts[0] is browser.contexts[0] if contexts else None
                ),
                "page_context_identity": pages[0].context is contexts[0] if pages else None,
                "page_repeat_identity": (pages[0] is contexts[0].pages[0] if pages else None),
            }
        finally:
            browser.close()


@pytest.mark.skipif(not ENDPOINT, reason="PWRIGHT_CDP_ENDPOINT is required")
def test_sync_attach_enumeration_matches_pinned_playwright() -> None:
    # Arrange
    oracle = _observe_oracle_sync()

    # Act
    candidate = _observe_candidate_sync()

    # Assert
    assert candidate == oracle


async def _observe_oracle_navigation() -> dict[str, object]:
    async with oracle_playwright() as playwright:
        browser = await playwright.chromium.connect_over_cdp(ENDPOINT)
        pages = [page for context in browser.contexts for page in context.pages]
        page = next(page for page in pages if TEST_PAGE_TOKEN in page.url)
        try:
            response = await page.goto(NAVIGATION_URL)
            return {
                "response_is_none": response is None,
                "url": page.url,
                "title": await page.title(),
                "content": await page.content(),
            }
        finally:
            await page.goto(TEST_PAGE_URL)
            await browser.close()


async def _observe_candidate_navigation() -> dict[str, object]:
    async with candidate_playwright() as playwright:
        browser = await playwright.chromium.connect_over_cdp(ENDPOINT)
        pages = [page for context in browser.contexts for page in context.pages]
        page = next(page for page in pages if TEST_PAGE_TOKEN in page.url)
        try:
            response = await page.goto(NAVIGATION_URL)
            return {
                "response_is_none": response is None,
                "url": page.url,
                "title": await page.title(),
                "content": await page.content(),
            }
        finally:
            await page.goto(TEST_PAGE_URL)
            await browser.close()


@pytest.mark.skipif(
    not ENDPOINT or not TEST_PAGE_URL,
    reason="PWRIGHT_CDP_ENDPOINT and PWRIGHT_TEST_PAGE_URL are required",
)
@pytest.mark.asyncio
async def test_navigation_and_content_match_pinned_playwright() -> None:
    # Arrange
    oracle = await _observe_oracle_navigation()

    # Act
    candidate = await _observe_candidate_navigation()

    # Assert
    assert candidate == oracle


def _observe_oracle_navigation_sync() -> dict[str, object]:
    with oracle_sync_playwright() as playwright:
        browser = playwright.chromium.connect_over_cdp(ENDPOINT)
        pages = [page for context in browser.contexts for page in context.pages]
        page = next(page for page in pages if TEST_PAGE_TOKEN in page.url)
        try:
            response = page.goto(NAVIGATION_URL)
            return {
                "response_is_none": response is None,
                "url": page.url,
                "title": page.title(),
                "content": page.content(),
            }
        finally:
            page.goto(TEST_PAGE_URL)
            browser.close()


def _observe_candidate_navigation_sync() -> dict[str, object]:
    with candidate_sync_playwright() as playwright:
        browser = playwright.chromium.connect_over_cdp(ENDPOINT)
        pages = [page for context in browser.contexts for page in context.pages]
        page = next(page for page in pages if TEST_PAGE_TOKEN in page.url)
        try:
            response = page.goto(NAVIGATION_URL)
            return {
                "response_is_none": response is None,
                "url": page.url,
                "title": page.title(),
                "content": page.content(),
            }
        finally:
            page.goto(TEST_PAGE_URL)
            browser.close()


@pytest.mark.skipif(
    not ENDPOINT or not TEST_PAGE_URL,
    reason="PWRIGHT_CDP_ENDPOINT and PWRIGHT_TEST_PAGE_URL are required",
)
def test_sync_navigation_and_content_match_pinned_playwright() -> None:
    # Arrange
    oracle = _observe_oracle_navigation_sync()

    # Act
    candidate = _observe_candidate_navigation_sync()

    # Assert
    assert candidate == oracle
