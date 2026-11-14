#!/usr/bin/env python3
"""Tests for per-invocation Playwright Core classification."""

from __future__ import annotations

import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from audit_playwright_official_invocations import (
    _classify_invocation,
    _source_segments,
)

SELECTOR_PARSER_IMPORT = (
    "runtime-source-import:packages/playwright-core/lib/utils/isomorphic/selectorParser"
)
HOST_PLATFORM_IMPORT = (
    "runtime-source-import:packages/playwright-core/src/server/utils/hostPlatform"
)


class ClassifyInvocationTests(unittest.TestCase):
    def test_public_api_without_private_hook_remains_public(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }

        # Act
        classification, reasons = _classify_invocation(
            record, "test('uses public API', async ({ page }) => page.click('button'));"
        )

        # Assert
        self.assertEqual(classification, "public-api-candidate")
        self.assertEqual(reasons, [])

    def test_public_api_with_private_hook_is_explicit(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "page.click('button', { __testHookAfterStable } as any);",
        )

        # Assert
        self.assertEqual(classification, "public-api-with-implementation-private-hook")
        self.assertEqual(
            reasons,
            ["implementation-private-test-hook:__testHookAfterStable"],
        )

    def test_local_implementation_keeps_source_and_hook_reasons(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/component-parser.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [SELECTOR_PARSER_IMPORT],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "page.pause({ __testHookKeepTestTimeout: true });",
        )

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            [
                "implementation-private-test-hook:__testHookKeepTestTimeout",
                (
                    "local-implementation-under-test:packages/playwright-core/"
                    "lib/utils/isomorphic/selectorParser"
                ),
                SELECTOR_PARSER_IMPORT,
            ],
        )

    def test_runtime_import_supporting_public_behavior_is_private_helper(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/page/page-keyboard.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [HOST_PLATFORM_IMPORT],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('types', async ({ page }) => page.keyboard.type('hello'));",
        )

        # Assert
        self.assertEqual(classification, "public-api-with-private-test-helper")
        self.assertEqual(reasons, [HOST_PLATFORM_IMPORT])

    def test_runtime_import_outside_public_corpus_is_product(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/playwright-test/reporter-html.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [
                "runtime-source-import:packages/playwright/lib/reporters/html"
            ],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('renders report', async () => renderReport());",
        )

        # Assert
        self.assertEqual(classification, "product-infrastructure")
        self.assertEqual(
            reasons,
            [
                "product-infrastructure-source:playwright-test-runner",
                "runtime-source-import:packages/playwright/lib/reporters/html",
            ],
        )

    def test_unrelated_file_level_private_access_does_not_hide_public_test(
        self,
    ) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/browser.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": ["private-client-access"],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('version', ({ browser }) => expect(browser.version()).toBeTruthy());",
        )

        # Assert
        self.assertEqual(classification, "public-api-candidate")
        self.assertEqual(reasons, [])

    def test_direct_private_client_access_remains_internal(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/browser.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": ["private-client-access"],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('contexts', ({ browser, toImpl }) => toImpl(browser).contexts());",
        )

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(reasons, ["private-client-access"])

    def test_direct_private_launch_options_in_public_source_is_internal(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/browsertype-basic.spec.ts",
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }
        source = (
            "test.skip(!!(browserType as any)._playwright."
            "_defaultLaunchOptions.executablePath);\n"
            "expect(fs.existsSync(browserType.executablePath())).toBe(true);"
        )

        # Act
        classification, reasons = _classify_invocation(record, source)

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(reasons, ["private-client-access"])

    def test_request_private_tracing_is_internal_even_with_file_level_helpers(
        self,
    ) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/tracing.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [
                "runtime-internal-import:playwright-core/lib/utilsBundle",
                "runtime-source-import:packages/playwright/src/isomorphic/folders",
            ],
        }
        source = (
            "test('stores request traces', async ({ request }) => {\n"
            "  await (request as any)._tracing.start({ snapshots: true });\n"
            "  await (request as any)._tracing.stop();\n"
            "});"
        )

        # Act
        classification, reasons = _classify_invocation(record, source)

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            [
                "private-client-access",
                "runtime-internal-import:playwright-core/lib/utilsBundle",
                "runtime-source-import:packages/playwright/src/isomorphic/folders",
            ],
        )

    def test_public_context_tracing_does_not_become_internal(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/tracing.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [
                "runtime-internal-import:playwright-core/lib/utilsBundle",
            ],
        }

        # Act
        classification, reasons = _classify_invocation(
            record, "await context.tracing.start({ snapshots: true });"
        )

        # Assert
        self.assertEqual(classification, "public-api-with-private-test-helper")
        self.assertEqual(
            reasons, ["runtime-internal-import:playwright-core/lib/utilsBundle"]
        )

    def test_direct_private_access_precedes_public_private_hook(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/page/page-click.spec.ts",
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }
        source = "page._channel.click({ __testHookAfterStable: true });"

        # Act
        classification, reasons = _classify_invocation(record, source)

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            [
                "implementation-private-test-hook:__testHookAfterStable",
                "private-client-access",
            ],
        )

    def test_shared_private_fixture_remains_internal(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/page-event-crash.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": ["private-client-access"],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('crashes', async ({ page, crash }) => { crash(); });",
        )

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(reasons, ["shared-private-fixture:crash"])

    def test_non_public_source_without_direct_private_access_is_product(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/playwright-test/playwright.reuse.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": ["private-client-access"],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('runner behavior', async ({ page }) => page.goto('about:blank'));",
        )

        # Assert
        self.assertEqual(classification, "product-infrastructure")
        self.assertEqual(
            reasons, ["product-infrastructure-source:playwright-test-runner"]
        )

    def test_product_source_without_prior_reason_gets_explicit_reason(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/stress/watchdog.spec.ts",
            "sourceClassification": "product-infrastructure",
            "classificationReasons": [],
        }

        # Act
        classification, reasons = _classify_invocation(
            record, "test('stress', () => {});"
        )

        # Assert
        self.assertEqual(classification, "product-infrastructure")
        self.assertEqual(reasons, ["product-infrastructure-source:stress-harness"])

    def test_unrecognized_product_source_fails_closed(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/new-product/example.spec.ts",
            "sourceClassification": "product-infrastructure",
            "classificationReasons": [],
        }

        # Act / Assert
        with self.assertRaisesRegex(SystemExit, "no explicit reason"):
            _classify_invocation(record, "test('new product', () => {});")

    def test_local_event_emitter_implementation_is_internal(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/events/events-list.spec.ts",
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "const emitter = new EventEmitter(); emitter.on('ready', () => {});",
        )

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            ["local-implementation-under-test:tests/library/events/utils"],
        )

    def test_private_device_descriptors_test_is_internal(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/browsercontext-viewport.spec.ts",
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }
        source = (
            "it('landscape viewport', async () => {\n"
            "  for (const device in deviceDescriptors) {\n"
            "    const configuration = deviceDescriptors[device];\n"
            "    expect(configuration.viewport.width).toBeGreaterThan(0);\n"
            "  }\n"
            "});"
        )

        # Act
        classification, reasons = _classify_invocation(record, source)

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            [
                (
                    "local-implementation-under-test:packages/playwright-core/"
                    "lib/server/deviceDescriptors"
                )
            ],
        )

    def test_other_viewport_public_api_test_remains_public(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceFile": "tests/library/browsercontext-viewport.spec.ts",
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "it('viewport size', async ({ page }) => "
            "expect(page.viewportSize()).toEqual({ width: 1280, height: 720 }));",
        )

        # Assert
        self.assertEqual(classification, "public-api-candidate")
        self.assertEqual(reasons, [])

    def test_private_url_matcher_test_is_internal(self) -> None:
        # Arrange
        import_reason = (
            "runtime-source-import:packages/playwright-core/"
            "lib/utils/isomorphic/urlMatch"
        )
        record: dict[str, object] = {
            "sourceFile": "tests/page/interception.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [import_reason],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "it('should work with glob', async () => { "
            "expect(globToRegexPattern('**/*.js')).toBeTruthy(); "
            "expect(urlMatches(undefined, '/foo.js', '**/*.js')).toBeTruthy(); });",
        )

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            [
                (
                    "local-implementation-under-test:packages/playwright-core/"
                    "lib/utils/isomorphic/urlMatch"
                ),
                import_reason,
            ],
        )

    def test_interception_sibling_without_private_matcher_stays_helper(self) -> None:
        # Arrange
        import_reason = (
            "runtime-source-import:packages/playwright-core/"
            "lib/utils/isomorphic/urlMatch"
        )
        record: dict[str, object] = {
            "sourceFile": "tests/page/interception.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [import_reason],
        }

        # Act
        classification, reasons = _classify_invocation(
            record, "it('intercepts', async ({ page }) => page.route('**/*.js', cb));"
        )

        # Assert
        self.assertEqual(classification, "public-api-with-private-test-helper")
        self.assertEqual(reasons, [import_reason])

    def test_private_locator_conversion_test_is_internal(self) -> None:
        # Arrange
        import_reason = "runtime-internal-import:playwright-core/lib/utils"
        record: dict[str, object] = {
            "sourceFile": "tests/page/page-aria-snapshot-ai.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [import_reason],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "it('stitches frames', async ({ page }) => { "
            "const selector = await page.locator('aria-ref=e1')._resolveSelector(); "
            "expect(asLocator('javascript', selector)).toBe('locator'); });",
        )

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            [
                "local-implementation-under-test:playwright-core/lib/utils",
                import_reason,
            ],
        )

    def test_snapshot_sibling_without_private_conversion_stays_helper(self) -> None:
        # Arrange
        import_reason = "runtime-internal-import:playwright-core/lib/utils"
        record: dict[str, object] = {
            "sourceFile": "tests/page/page-aria-snapshot-ai.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [import_reason],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "it('takes snapshot', async ({ page }) => "
            "expect(await snapshotForAI(page)).toContain('iframe'));",
        )

        # Assert
        self.assertEqual(classification, "public-api-with-private-test-helper")
        self.assertEqual(reasons, [import_reason])

    def test_private_outofprocess_driver_test_is_internal(self) -> None:
        # Arrange
        import_reason = (
            "runtime-source-import:packages/playwright-core/lib/outofprocess"
        )
        record: dict[str, object] = {
            "sourceFile": "tests/library/browsertype-launch-selenium.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [import_reason],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('run-driver', async () => { "
            "const { playwright, stop } = await start({ SELENIUM_REMOTE_URL }); "
            "await playwright.chromium.launch(); await stop(); });",
        )

        # Assert
        self.assertEqual(classification, "implementation-internal")
        self.assertEqual(
            reasons,
            [
                (
                    "local-implementation-under-test:packages/playwright-core/"
                    "lib/outofprocess"
                ),
                import_reason,
            ],
        )

    def test_selenium_sibling_message_does_not_imply_private_driver(self) -> None:
        # Arrange
        import_reason = (
            "runtime-source-import:packages/playwright-core/lib/outofprocess"
        )
        record: dict[str, object] = {
            "sourceFile": "tests/library/browsertype-launch-selenium.spec.ts",
            "sourceClassification": "implementation-internal",
            "classificationReasons": [import_reason],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('error', async ({ browserType }) => { "
            "expect(error.message).toContain('Could not start a new session'); "
            "await browserType.launch(); });",
        )

        # Assert
        self.assertEqual(classification, "public-api-with-private-test-helper")
        self.assertEqual(reasons, [import_reason])

    def test_official_example_role_is_product_infrastructure(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "role": "official-example",
            "sourceFile": "examples/todomvc/tests/adding-todos.spec.ts",
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }

        # Act
        classification, reasons = _classify_invocation(
            record,
            "test('adds a todo', async ({ page }) => page.click('button'));",
        )

        # Assert
        self.assertEqual(classification, "product-infrastructure")
        self.assertEqual(
            reasons,
            ["product-infrastructure-role:example-application-validation"],
        )

    def test_duplicate_private_hook_reason_is_deduplicated(self) -> None:
        # Arrange
        record: dict[str, object] = {
            "sourceClassification": "public-api-candidate",
            "classificationReasons": [],
        }
        source = (
            "const x = __testHookLookup; request.get(url, { __testHookLookup: x });"
        )

        # Act
        classification, reasons = _classify_invocation(record, source)

        # Assert
        self.assertEqual(classification, "public-api-with-implementation-private-hook")
        self.assertEqual(
            reasons,
            ["implementation-private-test-hook:__testHookLookup"],
        )

    def test_source_segments_stop_at_the_next_collected_test(self) -> None:
        # Arrange
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "tests/page/sample.spec.ts"
            source.parent.mkdir(parents=True)
            source.write_text(
                "header\n"
                "test('first', () => {\n"
                "  page.click('button', { __testHookAfterStable } as any);\n"
                "});\n"
                "test('second', () => page.click('button'));\n"
            )

            # Act
            segments = _source_segments(root, {"tests/page/sample.spec.ts": (2, 5)})

            # Assert
            self.assertEqual(segments[("tests/page/sample.spec.ts", 2)][0], 4)
            self.assertIn(
                "__testHookAfterStable",
                segments[("tests/page/sample.spec.ts", 2)][1],
            )
            self.assertNotIn(
                "__testHookAfterStable",
                segments[("tests/page/sample.spec.ts", 5)][1],
            )


if __name__ == "__main__":
    unittest.main()
