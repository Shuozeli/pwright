// Same public API scenarios on pinned Playwright and the locally built Rust FFI.
import assert from 'node:assert/strict';
import artifactConfig from '../src/native-artifact.cjs';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_INPUT_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_KEYBOARD_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const raw = JSON.parse(await readFile(new URL('../../../tests/data/raw-keyboard-cases.json', import.meta.url), 'utf8'));
assert.equal(raw.length, 14);
const samples = raw.map(sample => ({ id: sample.id, async run(page) {
  const outcomes = [];
  for (const action of sample.actions) {
    try { await page.keyboard[action.kind](action.value); outcomes.push('ok'); }
    catch (error) { if (!error.message.includes('Unknown key:')) throw error; outcomes.push({ error: error.message }); }
  }
  return { outcomes, snapshot: await page.evaluate(() => window.typingSnapshot()) };
} }));
for (const [id, method, text, options] of [
  ['insert-unicode', 'insertText', '你好🙂\nA'],
  ['type-unicode', 'type', 'Ab🙂\n'],
  ['type-fractional-delay', 'type', 'aB', { delay: 0.5 }],
  ['insert-empty', 'insertText', ''],
  ['type-empty', 'type', ''],
]) samples.push({ id, async run(page) {
  await page.keyboard[method](text, options);
  return page.evaluate(() => window.typingSnapshot());
} });
samples.push({ id: 'shared-locator-modifier', async run(page) {
  await page.keyboard.down('Shift');
  await page.locator('#text').press('KeyA');
  await page.keyboard.up('Shift');
  return page.evaluate(() => window.typingSnapshot());
} });
samples.push({ id: 'navigation-keeps-page-input', async run(page) {
  const keyboard = page.keyboard;
  await keyboard.down('Shift');
  await page.goto(fixture.href, { timeout: 5000 });
  await page.evaluate(() => window.resetTyping('#text'));
  assert.equal(page.keyboard, keyboard);
  await keyboard.press('KeyA');
  await keyboard.up('Shift');
  return page.evaluate(() => window.typingSnapshot());
} });
samples.push({ id: 'two-pages-isolate-input', async run(page, context) {
  await page.keyboard.down('Shift');
  const other = await context.newPage();
  try {
    await other.goto(fixture.href, { timeout: 5000 });
    await other.evaluate(() => window.resetTyping('#text'));
    await other.keyboard.press('KeyA');
    const second = await other.evaluate(() => window.typingSnapshot());
    await page.keyboard.press('KeyB');
    await page.keyboard.up('Shift');
    return { first: await page.evaluate(() => window.typingSnapshot()), second };
  } finally { await other.close(); }
} });
samples.push({ id: 'closed-page-rejects', async run(page) {
  const keyboard = page.keyboard;
  await page.close();
  try { await keyboard.press('a'); return 'unexpected-success'; }
  catch (error) { return /closed/i.test(error.message) ? 'target-closed' : { name: error.name, message: error.message }; }
} });
samples.push({ id: 'close-during-delayed-press', async run(page) {
  // Attach rejection handling immediately; close only after native keydown is visible.
  const pressed = page.keyboard.press('Shift', { delay: 5000 }).then(() => 'unexpected-success', error => /closed/i.test(error.message) ? 'target-closed' : { name: error.name, message: error.message });
  let observed = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    observed = await page.evaluate(() => window.typingSnapshot().events.some(event => event.type === 'keydown'));
    if (observed) break;
  }
  assert.equal(observed, true, 'native keydown must precede close');
  await page.close();
  return pressed;
} });

async function collect(provider) {
  const browser = await provider.chromium.connectOverCDP(endpoint, { timeout: 5000 });
  const observations = [];
  let metrics;
  try {
    for (const sample of samples) {
      const context = await diagnosticDeadline(browser.newContext(), 'Keyboard Context');
      try {
        const page = await diagnosticDeadline(context.newPage(), 'Keyboard Page');
        await page.goto(fixture.href, { timeout: 5000 });
        await page.evaluate(() => window.resetTyping('#text'));
        // Diagnostic watchdog failures are errors, never parity outcomes.
        try { observations.push({ id: sample.id, value: await diagnosticDeadline(sample.run(page, context), sample.id) }); }
        catch (error) { observations.push({ id: sample.id, error: { name: error.name, message: error.message } }); }
      } finally { await diagnosticDeadline(context.close(), 'Keyboard Context cleanup'); }
    }
  } finally {
    await diagnosticDeadline(browser.close(), 'Keyboard disconnect');
    metrics = browser.pwrightMetrics?.();
  }
  if (metrics) {
    assert.equal(metrics.activeLeases, 0);
    assert.equal(metrics.activeOperations, 0);
    assert.equal(metrics.cleanupInFlight, 0);
  }
  return { observations, metrics };
}
const oracle = await collect(requireOracle('playwright'));
const candidate = await collect(await import('../src/index.mjs'));
const cases = oracle.observations.map((expected, index) => {
  const actual = candidate.observations[index];
  let match = false;
  if (!expected.error && !actual.error) {
    try { assert.deepEqual(actual, expected); match = true; } catch { /* Record the exact discrepancy below. */ }
  }
  return { id: expected.id, match, oracle: expected, candidate: actual };
});
const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
const report = { oracle: 'playwright@1.58.2', artifact, nativeProfile: artifactConfig.nativeArtifact.profile, cases, metrics: candidate.metrics };
const encoded = JSON.stringify(report, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 4 * 1024 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
for (const sample of cases) console.log(`${sample.match ? 'MATCH' : 'FAIL'} ${sample.id}`);
assert.equal(cases.filter(sample => sample.match).length, cases.length);
console.log(`Public Keyboard: ${cases.length}/${cases.length}, zero host leases/operations/cleanup after close`);
