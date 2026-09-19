// Native reload primitive oracle. This does not exercise the Node FFI package.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_RELOAD_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_RELOAD_ORACLE;
assert.ok(typeof output === 'string' && isAbsolute(output));
const oracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracle('playwright/package.json').version, '1.58.2');
const browser = await oracle('playwright').chromium.connectOverCDP(endpoint, { timeout: 5000 });
let context;
const observations = [];
try {
  context = await diagnosticDeadline(browser.newContext(), 'reload oracle context');
  const page = await diagnosticDeadline(context.newPage(), 'reload oracle page');
  for (const url of ['about:blank', 'data:text/html,<title>native-reload</title>', fixture.href]) {
    for (const waitUntil of ['commit', 'domcontentloaded', 'load', 'networkidle']) {
      await page.goto(url, { timeout: 5000 });
      const before = await page.evaluate(() => history.length);
      const response = await page.reload({ waitUntil, timeout: 5000 });
      const after = await page.evaluate(() => history.length);
      assert.equal(after, before, 'reload must not append a history entry');
      observations.push({ url, waitUntil, result: {
        url: page.url(), historyUnchanged: after === before,
        response: response ? { url: response.url(), status: response.status(), statusText: response.statusText() } : null,
      } });
    }
  }
} finally {
  try { if (context) await diagnosticDeadline(context.close(), 'reload oracle context cleanup'); }
  finally { await diagnosticDeadline(browser.close(), 'reload oracle disconnect'); }
}
assert.equal(observations.length, 12);
const encoded = JSON.stringify({ version: '1.58.2', observations }, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 64 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
console.log('Recorded 12 pinned Playwright reload observations after fixture cleanup');
