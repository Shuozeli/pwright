// Source-of-truth public Frame lifetime observations, before implementing views.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
assert.equal(process.env.PWRIGHT_TEST_CDP, 'http://127.0.0.1:9222');
assert.ok(isAbsolute(process.env.PWRIGHT_FRAME_IDENTITY_REPORT));
const fixture = new URL(process.env.PWRIGHT_DRAG_FIXTURE);
assert.ok(fixture.hostname.endsWith('.ts.net'));
const browser = await requireOracle('playwright').chromium.connectOverCDP(process.env.PWRIGHT_TEST_CDP, { timeout: 5000 });
const observations = [];
try {
  for (const mode of ['local', 'sandbox']) {
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      const url = new URL(fixture); url.searchParams.set('frame-mode', mode);
      await page.goto(url.href, { timeout: 5000 });
      const main = page.mainFrame();
      const child = main.childFrames()[0];
      assert.ok(child);
      await child.waitForFunction(() => typeof frameSnapshot === 'function', null, { timeout: 5000 });
      const before = { count: page.frames().length, parent: child.parentFrame() === main, owner: child.page() === page, same: page.frames().includes(child), detached: child.isDetached(), mainDetached: main.isDetached() };
      const detachedEvent = page.waitForEvent('framedetached', { predicate: value => value === child, timeout: 5000 });
      await page.evaluate(() => document.querySelector('iframe').remove());
      const beforeDetachEvent = { count: page.frames().length, detached: child.isDetached() };
      await detachedEvent;
      const removed = { count: page.frames().length, parent: child.parentFrame() === main, children: main.childFrames().length, detached: child.isDetached(), retainedURL: child.url().includes('raw-drag-frame.html') };
      await page.evaluate(() => { const f = document.createElement('iframe'); f.srcdoc = '<title>new child</title>'; document.body.append(f); });
      await page.waitForFunction(() => document.querySelector('iframe').contentDocument.title === 'new child', null, { timeout: 5000 });
      const replacement = main.childFrames()[0];
      assert.ok(replacement && replacement !== child);
      await page.close();
      const closed = { count: page.frames().length, children: main.childFrames().length, mainDetached: main.isDetached(), childDetached: replacement.isDetached(), parent: replacement.parentFrame() === main, owner: replacement.page() === page };
      assert.deepEqual(removed, { count: 1, parent: true, children: 0, detached: true, retainedURL: true });
      assert.deepEqual(closed, { count: 2, children: 1, mainDetached: false, childDetached: false, parent: true, owner: true });
      observations.push({ mode, before, beforeDetachEvent, removed, closed });
    } finally { await diagnosticDeadline(context.close(), 'Frame oracle Context cleanup'); }
  }
} finally { await diagnosticDeadline(browser.close(), 'Frame oracle disconnect'); }
await writeFile(process.env.PWRIGHT_FRAME_IDENTITY_REPORT, JSON.stringify({ version: '1.58.2', observations }, null, 2), { flag: 'wx' });
console.log(JSON.stringify(observations));
