/** Diagnostic only. Never imported by the paired conformance gate or candidate. */
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { isAbsolute } from "node:path";
import { chromium, type BrowserContext, type Page, type CDPSession } from "playwright";
import { closeAll } from "./shared-lifecycle.js";
import { externalFixtureServer } from "./external-fixture-server.js";
import { diagnostic } from "./shared-differential.js";

async function main(): Promise<void> {
  const endpoint = process.env.PWRIGHT_TEST_CDP;
  const base = process.env.PWRIGHT_SHARED_FIXTURES;
  const output = process.env.PWRIGHT_SHARED_HISTORY_OUTPUT;
  assert.equal(endpoint, "http://127.0.0.1:9222", "use the dedicated Chrome profile");
  assert.ok(base && output && isAbsolute(output), "explicit fixture URL and absolute new history output file required");
  const server = externalFixtureServer(base);
  const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
  let context: BrowserContext | undefined, page: Page | undefined, cdp: CDPSession | undefined;
  let phase = "setup", overflow = false;
  const events: object[] = [];
  const record = (value: object) => {
    if (events.length < 128) events.push({ phase, ...value });
    else overflow = true;
  };
  try {
    context = await browser.newContext();
    page = await context.newPage();
    page.setDefaultTimeout(5000);
    page.setDefaultNavigationTimeout(5000);
    page.on("load", () => record({ kind: "public-load" }));
    page.on("domcontentloaded", () => record({ kind: "public-domcontentloaded" }));
    cdp = await context.newCDPSession(page);
    cdp.on("Page.frameNavigated", (event: { type: string; frame: { url: string; loaderId: string } }) => record({
      kind: "frameNavigated", navigationType: event.type, url: event.frame.url, loaderId: event.frame.loaderId,
    }));
    cdp.on("Page.lifecycleEvent", (event: { name: string; loaderId: string }) => record({ kind: "lifecycle", name: event.name, loaderId: event.loaderId }));
    await cdp.send("Page.enable");
    await cdp.send("Page.setLifecycleEventsEnabled", { enabled: true });
    phase = "first";
    await page.goto(`${server.PREFIX}/navigation-extras.html`);
    phase = "second";
    await page.goto(`${server.PREFIX}/navigation-extras-page2.html`);
    phase = "back";
    try { await page.goBack(); record({ kind: "result", result: "resolved" }); }
    catch (error) { record({ kind: "result", result: "rejected", error: diagnostic(error) }); }
    record({ kind: "after-back", url: page.url(), title: await page.title(), readyState: await page.evaluate("document.readyState") });
  } finally {
    const cleanup = await closeAll([
      async () => { await cdp?.detach(); }, async () => { await page?.close(); },
      async () => { await context?.close(); }, async () => { await browser.close(); },
    ]).then(() => undefined, (error: unknown) => diagnostic(error));
    await writeFile(output, JSON.stringify({
      diagnosticOnly: true, oracle: "playwright", browserVersion: browser.version(),
      timestamp: new Date().toISOString(), endpoint, fixtures: server.PREFIX, overflow, cleanup, events,
    }, null, 2) + "\n", { flag: "wx" });
    assert.equal(overflow, false, "history probe exceeded event budget");
    assert.equal(cleanup, undefined, "history probe cleanup failed");
  }
  console.log(`History diagnostic saved to ${output}; this is not a conformance pass.`);
}

main().catch((error: unknown) => { console.error(error); process.exitCode = 2; });
