/** Shared fixture adapter: public Node API only, no browser logic or JS fallback. */
import { chromium, errors, type BrowserContext, type Page } from "@shuozeli/pwright";
import assert from "node:assert/strict";
import { setImmediate } from "node:timers/promises";
import type { RunContext } from "./harness.js";
import type { TestServer } from "./test-server.js";
import { closeAll, rejectAfterCleanup, type SharedSession } from "./shared-lifecycle.js";
import { UnsupportedRunOperation, type RunOperation } from "./shared-differential.js";

const unsupported = (operation: RunOperation) => async (..._args: unknown[]): Promise<never> => {
  throw new UnsupportedRunOperation(operation);
};

export async function createNodeFfiContext(cdp: string, server: TestServer,
  options: { resolveInputFile?: (path: string) => string } = {},
): Promise<SharedSession<RunContext>> {
  const browser = await chromium.connectOverCDP(cdp, { timeout: 5000 });
  let ownedContext: BrowserContext | undefined;
  let ownedPage: Page | undefined;
  const cleanup = () => closeAll([
    async () => { await ownedPage?.close(); },
    async () => { await ownedContext?.close(); },
    async () => { await browser.close(); },
    async () => {
      assert.deepEqual(browser.pwrightMetrics(), {
        connected: false, liveObjects: 0, pendingCalls: 0, topologyDrivers: 0,
        activeLeases: 0, liveScopes: 0, retainedEventBytes: 0,
        activeOperations: 0, cleanupInFlight: 0,
      });
      const deadline = Date.now() + 2000;
      while (browser.pwrightFfiMetrics().pending !== 0 && Date.now() < deadline) await setImmediate();
      assert.deepEqual(browser.pwrightFfiMetrics(), {
        pending: 0, queued: 0, queuedCommands: 0, running: 0, reservedBytes: 0,
        connects: 0, commands: 0, closes: 0, watches: 0,
      });
      assert.deepEqual(browser.pwrightConversionMetrics(), { pending: 0, binaryReferences: 0, backingBytes: 0 });
      assert.deepEqual(browser.pwrightLocatorConversionMetrics(), { pending: 0, rawReferences: 0, binaryReferences: 0, backingBytes: 0, exceptions: 0 });
    },
  ]);
  try {
    const context = ownedContext = await browser.newContext();
    const page = ownedPage = await context.newPage();
    const ctx: RunContext = {
      server,
      goto: async url => { await page.goto(url, { timeout: 5000 }); },
      url: async () => page.url(), title: () => page.title(), content: () => page.content(),
      reload: async () => { await page.reload({ timeout: 5000 }); },
      goBack: unsupported("goBack"), goForward: unsupported("goForward"),
      evaluate: expr => page.evaluate(expr),
      screenshot: format => {
        if (format !== undefined && format !== 'png' && format !== 'jpeg') throw new Error(`Unsupported screenshot format: ${format}`);
        return page.screenshot({ type: format, timeout: 5000 });
      },
      // The shared oracle adapter uses legacy Page selector reads, whose
      // default is first-match rather than Locator strictness. Select nth(0)
      // through the public Rust-backed value; do not evaluate a JS fallback.
      textContent: selector => page.locator(selector).first().textContent({ timeout: 5000 }),
      getAttribute: (selector, name) => page.locator(selector).first().getAttribute(name, { timeout: 5000 }),
      innerText: selector => page.locator(selector).first().innerText({ timeout: 5000 }),
      innerHTML: selector => page.locator(selector).first().innerHTML({ timeout: 5000 }),
      inputValue: selector => page.locator(selector).first().inputValue({ timeout: 5000 }),
      click: selector => page.locator(selector).first().click({ timeout: 5000 }),
      dblclick: selector => page.locator(selector).first().dblclick({ timeout: 5000 }),
      fill: (selector, value) => page.locator(selector).first().fill(value, { timeout: 5000 }),
      type: (selector, text) => page.locator(selector).first().type(text, { timeout: 5000 }),
      press: unsupported("press"),
      hover: selector => page.locator(selector).first().hover({ timeout: 5000 }),
      focus: selector => page.locator(selector).first().focus({ timeout: 5000 }),
      blur: selector => page.locator(selector).first().blur({ timeout: 5000 }),
      check: selector => page.locator(selector).first().check({ timeout: 5000 }),
      uncheck: selector => page.locator(selector).first().uncheck({ timeout: 5000 }),
      locatorCount: selector => page.locator(selector).count(),
      isVisible: selector => page.locator(selector).first().isVisible(),
      isChecked: selector => page.locator(selector).first().isChecked({ timeout: 5000 }),
      mouseClick: (x, y) => page.mouse.click(x, y), mouseDblclick: (x, y) => page.mouse.dblclick(x, y), mouseMove: (x, y) => page.mouse.move(x, y),
      keyboardPress: key => page.keyboard.press(key), keyboardType: text => page.keyboard.type(text),
      setInputFiles: async (selector, files) => {
        try {
          await page.locator(selector).setInputFiles(
            options.resolveInputFile ? files.map(options.resolveInputFile) : files,
            { timeout: 5000 },
          );
        } catch (error) {
          if (error instanceof errors.UnsupportedOperationError)
            throw new UnsupportedRunOperation("setInputFiles");
          throw error;
        }
      },
      getByText: (text, options) => page.getByText(text, options).textContent({ timeout: 5000 }),
      // Preserve the oracle's existing two observations through public APIs.
      getByLabel: async text => {
        const loc = page.getByLabel(text);
        await loc.inputValue({ timeout: 5000 }).catch(() => null);
        return loc.evaluate(el => el.tagName.toLowerCase(), undefined, { timeout: 5000 }).catch(() => null);
      },
      getByRole: (role, options) => page.getByRole(role as Parameters<Page["getByRole"]>[0], options).textContent({ timeout: 5000 }),
      waitForDownload: unsupported("waitForDownload"),
    };
    return { ctx, cleanup, browserVersion: browser.version() };
  } catch (error) { return rejectAfterCleanup(error, cleanup); }
}
