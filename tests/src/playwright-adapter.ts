/** Playwright oracle adapter for the unchanged shared API fixtures. */
import { chromium, type BrowserContext, type Page } from "playwright";
import type { RunContext } from "./harness.js";
import type { TestServer } from "./test-server.js";
import { closeAll, rejectAfterCleanup, type SharedSession } from "./shared-lifecycle.js";

export interface OracleOptions {
  connectTimeoutMs?: number;
  actionTimeoutMs?: number;
  resolveInputFile?: (path: string) => string;
}

export async function createPlaywrightContext(
  cdpUrl: string, server: TestServer, options: OracleOptions = {},
): Promise<SharedSession<RunContext>> {
  const browser = await chromium.connectOverCDP(cdpUrl, { timeout: options.connectTimeoutMs });
  let ownedContext: BrowserContext | undefined;
  let ownedPage: Page | undefined;
  const cleanup = () => closeAll([
    async () => { await ownedPage?.close(); },
    async () => { await ownedContext?.close(); },
    async () => { await browser.close(); },
  ]);
  try {
    const context = ownedContext = await browser.newContext();
    const page = ownedPage = await context.newPage();
    if (options.actionTimeoutMs !== undefined) {
      page.setDefaultTimeout(options.actionTimeoutMs);
      page.setDefaultNavigationTimeout(options.actionTimeoutMs);
    }
    const ctx: RunContext = {
      server,
      goto: async url => { await page.goto(url); },
      reload: async () => { await page.reload(); },
      goBack: async () => { await page.goBack(); },
      goForward: async () => { await page.goForward(); },
      url: async () => page.url(), title: () => page.title(), content: () => page.content(),
      evaluate: expr => page.evaluate(expr),
      screenshot: async format => {
        if (format !== undefined && format !== "png" && format !== "jpeg")
          throw new Error(`Unsupported screenshot format: ${format}`);
        return page.screenshot({ type: format ?? "png" });
      },
      textContent: sel => page.textContent(sel), getAttribute: (sel, name) => page.getAttribute(sel, name),
      innerText: sel => page.innerText(sel), innerHTML: sel => page.innerHTML(sel),
      inputValue: sel => page.inputValue(sel), click: sel => page.click(sel), dblclick: sel => page.dblclick(sel),
      fill: (sel, value) => page.fill(sel, value), type: (sel, text) => page.locator(sel).pressSequentially(text),
      press: key => page.keyboard.press(key), hover: sel => page.hover(sel), focus: sel => page.focus(sel),
      blur: sel => page.evaluate(s => {
        const element = document.querySelector(s);
        if (element instanceof HTMLElement || element instanceof SVGElement) element.blur();
      }, sel),
      check: sel => page.check(sel), uncheck: sel => page.uncheck(sel),
      locatorCount: sel => page.locator(sel).count(), isVisible: sel => page.isVisible(sel),
      isChecked: sel => page.isChecked(sel), mouseClick: (x, y) => page.mouse.click(x, y),
      mouseDblclick: (x, y) => page.mouse.dblclick(x, y), mouseMove: (x, y) => page.mouse.move(x, y),
      keyboardPress: key => page.keyboard.press(key), keyboardType: text => page.keyboard.type(text),
      setInputFiles: (selector, files) => page.locator(selector).setInputFiles(
        options.resolveInputFile ? files.map(options.resolveInputFile) : files,
      ),
      getByText: (text, queryOptions) => page.getByText(text, queryOptions).textContent(),
      // Keep the legacy fixture's tag-name observation, not an input-value assertion.
      getByLabel: async text => {
        const loc = page.getByLabel(text);
        await loc.inputValue().catch(() => null);
        return loc.evaluate(el => el.tagName.toLowerCase()).catch(() => null);
      },
      getByRole: (role, queryOptions) => page.getByRole(
        role as Parameters<Page["getByRole"]>[0], queryOptions,
      ).textContent(),
      waitForDownload: async action => {
        // Observe rejection immediately so action failure cannot leave an unhandled promise.
        const download = page.waitForEvent("download").then(
          value => ({ value }), (error: unknown) => ({ error }),
        );
        await action();
        const outcome = await download;
        if ("error" in outcome) throw outcome.error;
        // Historical corpus checks an event only; this is not artifact-content parity.
        return "download_path_mocked_for_golden_tests.txt";
      },
    };
    return { ctx, cleanup, browserVersion: browser.version() };
  } catch (error) { return rejectAfterCleanup(error, cleanup); }
}
