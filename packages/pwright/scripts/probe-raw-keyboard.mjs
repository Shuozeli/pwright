// Pinned raw Keyboard oracle, not a public Node-FFI conformance claim.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_INPUT_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_INPUT_ORACLE;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const cases = JSON.parse(await readFile(new URL('../../../tests/data/raw-keyboard-cases.json', import.meta.url), 'utf8'));
assert.equal(cases.length, 14);
const browser = await requireOracle('playwright').chromium.connectOverCDP(endpoint, { timeout: 5000 });
let result;
try {
  const observations = [];
  for (const sample of cases) {
    assert.ok(sample.actions.length <= 8);
    // A fresh owned Context isolates held logical keys even after a test error.
    const context = await diagnosticDeadline(browser.newContext(), 'keyboard Context');
    try {
      const page = await diagnosticDeadline(context.newPage(), 'keyboard Page');
      await page.goto(fixture.href, { timeout: 5000 });
      await page.evaluate(() => window.resetTyping('#text'));
      const outcomes = [];
      for (const action of sample.actions) {
        assert.ok(['press', 'down', 'up'].includes(action.kind));
        assert.ok(typeof action.value === 'string' && action.value.length <= 128);
        try {
          await diagnosticDeadline(page.keyboard[action.kind](action.value), `${sample.id}/${action.kind}`);
          outcomes.push('ok');
        } catch (error) {
          if (!error.message.includes('Unknown key:')) throw error;
          outcomes.push('unknown-key');
        }
      }
      const snapshot = await page.evaluate(() => window.typingSnapshot());
      assert.equal(snapshot.overflow, false);
      observations.push({ id: sample.id, outcomes, snapshot });
    } finally { await diagnosticDeadline(context.close(), 'keyboard Context cleanup'); }
  }
  result = { version: '1.58.2', browser: browser.version(), isMac: process.platform === 'darwin', observations };
} finally { await diagnosticDeadline(browser.close(), 'keyboard oracle disconnect'); }
const encoded = JSON.stringify(result, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 1024 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
console.log('Recorded 14 raw Keyboard cases after exact owned Context cleanup');
