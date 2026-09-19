// Oracle for the Rust input primitive, not a public Node-FFI parity claim.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222', 'dedicated Chrome/Pwright preflight required');
const fixture = new URL(process.env.PWRIGHT_INPUT_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'), 'use the verified full Tailscale fixture URL');
const output = process.env.PWRIGHT_INPUT_ORACLE;
assert.ok(typeof output === 'string' && isAbsolute(output), 'explicit absolute oracle output required');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium } = requireOracle('playwright');
const cases = JSON.parse(await readFile(new URL('../../../tests/data/sequential-typing-cases.json', import.meta.url), 'utf8'));
assert.equal(cases.length, 13);
const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
let context;
const observations = [];
try {
  context = await diagnosticDeadline(browser.newContext(), 'oracle context');
  const page = await diagnosticDeadline(context.newPage(), 'oracle page');
  await page.goto(fixture.href, { waitUntil: 'load', timeout: 10000 });
  const isMac = await page.evaluate(() => navigator.platform.includes('Mac'));
  for (const sample of cases) {
    await page.evaluate(selector => window.resetTyping(selector), sample.selector);
    if (sample.modifier !== null) await page.keyboard.down(sample.modifier);
    try {
      await diagnosticDeadline(page.keyboard.type(sample.text, { delay: sample.delayMs }), `typing ${sample.id}`);
    } finally {
      if (sample.modifier !== null) await page.keyboard.up(sample.modifier);
    }
    const snapshot = await page.evaluate(() => window.typingSnapshot());
    assert.equal(snapshot.overflow, false, `event bound: ${sample.id}`);
    observations.push({ id: sample.id, snapshot });
  }
  const result = { version: '1.58.2', browser: browser.version(), isMac, observations };
  const encoded = JSON.stringify(result, null, 2);
  assert.ok(Buffer.byteLength(encoded) <= 1024 * 1024, 'bounded oracle file');
  // Refuse to overwrite evidence from a prior run.
  await writeFile(output, encoded, { flag: 'wx' });
  console.log(`Recorded ${observations.length} Playwright keyboard.type cases (${result.browser})`);
} finally {
  try {
    if (context) await diagnosticDeadline(context.close(), 'oracle context cleanup');
  } finally {
    await diagnosticDeadline(browser.close(), 'oracle disconnect');
  }
}
