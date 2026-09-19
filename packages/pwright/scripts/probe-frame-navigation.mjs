// Records final-state observations, not a claim that native begin == Frame.goto.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_FRAME_NAVIGATION_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_FRAME_NAVIGATION_ORACLE;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const browser = await requireOracle('playwright').chromium.connectOverCDP(endpoint, { timeout: 5000 });
let context;
const observations = [];
try {
  context = await diagnosticDeadline(browser.newContext(), 'Frame navigation oracle Context');
  const page = await context.newPage();
  for (const [mode, depth] of [['local', 1], ['local', 2], ['sandbox', 1]]) {
    await page.goto(new URL(`frame-navigation.html?mode=${mode}`, fixture).href, { timeout: 5000 });
    const parent = page.mainFrame().childFrames()[0];
    const frame = depth === 1 ? parent : parent.childFrames()[0];
    const sibling = page.mainFrame().childFrames()[1];
    const parentUrl = parent.url();
    const rootUrl = page.url(), siblingUrl = sibling.url();
    for (const phase of ['document', 'hash']) {
      // The nested fixture already loads empty.html: a distinct query is needed
      // to exercise a new document before the subsequent hash-only navigation.
      const url = new URL(`empty.html?frame-navigation=${depth}#${phase}-${depth}`, fixture).href;
      const referer = 'http://referrer.example/path?q=1';
      const response = await frame.goto(url, { referer, timeout: 5000 });
      assert.equal(response === null, phase === 'hash');
      observations.push({ mode, depth, phase, url, referer, result: {
        rootUnchanged: page.url() === rootUrl,
        siblingUnchanged: sibling.url() === siblingUrl,
        parentUnchanged: depth === 1 ? null : parent.url() === parentUrl,
        selectedUrl: frame.url(), referrer: await frame.evaluate(() => document.referrer),
        sameDocument: response === null,
        sameIdentity: (depth === 1 ? page.mainFrame().childFrames()[0] : parent.childFrames()[0]) === frame,
      } });
    }
  }
} finally {
  try { if (context) await diagnosticDeadline(context.close(), 'Frame navigation oracle Context cleanup'); }
  finally { await diagnosticDeadline(browser.close(), 'Frame navigation oracle disconnect'); }
}
const encoded = JSON.stringify({ version: '1.58.2', fixture: fixture.href, observations }, null, 2);
assert.equal(observations.length, 6);
assert.ok(Buffer.byteLength(encoded) < 64 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
console.log('Recorded six pinned Frame navigation final-state observations after cleanup');
