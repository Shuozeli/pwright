// Pinned primitive oracle; not a public Mouse API conformance claim.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

assert.equal(process.env.PWRIGHT_TEST_CDP, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_MOUSE_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_MOUSE_ORACLE;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const cases = JSON.parse(await readFile(new URL('../../../tests/data/raw-mouse-cases.json', import.meta.url), 'utf8'));
assert.equal(cases.length, 25);
const browser = await requireOracle('playwright').chromium.connectOverCDP(process.env.PWRIGHT_TEST_CDP, { timeout: 5000 });
let result;
try {
  const observations = [];
  for (const sample of cases) {
    assert.ok(sample.actions.length <= 12);
    const context = await diagnosticDeadline(browser.newContext(), 'mouse Context');
    try {
      const page = await diagnosticDeadline(context.newPage(), 'mouse Page');
      await page.goto(fixture.href, { timeout: 5000 });
      await page.evaluate(() => window.resetMouse());
      let wheels = 0;
      const outcomes = [];
      for (const action of sample.actions) {
        let pending;
        switch (action.kind) {
          case 'move': pending = page.mouse.move(action.x, action.y, { steps: action.steps ?? 1 }); break;
          case 'click': case 'dblclick': pending = page.mouse[action.kind](action.x, action.y, { button: action.button ?? 'left', clickCount: action.clickCount ?? 1, delay: action.delay ?? 0 }); break;
          case 'down': case 'up': pending = page.mouse[action.kind]({ button: action.button ?? 'left', clickCount: action.clickCount ?? 1 }); break;
          case 'wheel': pending = page.mouse.wheel(action.deltaX, action.deltaY); break;
          case 'keyDown': pending = page.keyboard.down(action.key); break;
          case 'keyUp': pending = page.keyboard.up(action.key); break;
          default: throw new Error(`invalid mouse action: ${action.kind}`);
        }
        let outcome = 'ok';
        try { await diagnosticDeadline(pending, `${sample.id}/${action.kind}`); }
        catch (error) {
          if (action.error !== 'invalid-steps' || !/mouse\.move: steps: expected integer, got float/.test(error.message)) throw error;
          outcome = 'invalid-steps';
        }
        assert.equal(outcome, action.error ?? 'ok');
        outcomes.push(outcome);
        if (action.kind === 'wheel') {
          wheels++;
          // Observe delivered wheel input, not scroll completion or API timing.
          await page.waitForFunction(count => window.mouseSnapshot().wheels === count, wheels, { timeout: 5000 });
        }
      }
      const snapshot = await page.evaluate(() => window.mouseSnapshot());
      assert.equal(snapshot.overflow, false);
      observations.push({ id: sample.id, outcomes, snapshot });
    } finally { await diagnosticDeadline(context.close(), 'mouse Context cleanup'); }
  }
  result = { version: '1.58.2', browser: browser.version(), isMac: process.platform === 'darwin', observations };
} finally { await diagnosticDeadline(browser.close(), 'mouse oracle disconnect'); }
const encoded = JSON.stringify(result, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 1024 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
console.log(`Recorded ${cases.length} raw Mouse cases after owned Context cleanup`);
