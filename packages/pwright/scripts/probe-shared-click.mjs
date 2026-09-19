// Preserve the shared fixture's goto/click/read sequence; inspect only afterward.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { isAbsolute } from 'node:path';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import artifact from '../src/native-artifact.cjs';
import { diagnosticDeadline } from './probe-deadline.mjs';
import { CloseTrace } from '../test/native-close-trace.mjs';
import { withCdpCommandRelay } from '../test/cdp-command-relay.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const oracle = require('playwright').chromium;
const endpoint = process.env.PWRIGHT_TEST_CDP;
const fixture = process.env.PWRIGHT_CLICK_FIXTURE;
const output = process.env.PWRIGHT_CLICK_REPORT;
const observeEvents = process.env.PWRIGHT_CLICK_OBSERVE_EVENTS === '1';
const mode = process.env.PWRIGHT_CLICK_MODE ?? 'original';
assert.ok(['original', 'serial-buttons', 'delayed-click', 'painted-batch'].includes(mode));
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(fixture && new URL(fixture).hostname.endsWith('.ts.net'));
assert.ok(output && isAbsolute(output));
const report = { schema: 'pwright.shared-click-probe.v2', fixture, observeEvents, mode, profile: artifact.nativeArtifact.profile,
  sha256: createHash('sha256').update(await readFile(artifact.nativeArtifact.path)).digest('hex'), cases: [] };

class MouseTrace extends CloseTrace {
  inputs = [];
  command(raw, client, upstream) {
    const message = JSON.parse(raw.toString());
    if (message.method === 'Input.dispatchMouseEvent') {
      assert.ok(this.inputs.length < 8);
      const p = message.params;
      this.inputs.push({ id: message.id, session: message.sessionId, type: p.type,
        x: p.x, y: p.y, button: p.button, buttons: p.buttons, clickCount: p.clickCount,
        modifiers: p.modifiers, pointerType: p.pointerType, force: p.force });
    }
    super.command(raw, client, upstream);
  }
}

async function run(side, iteration, traced) {
  const type = side === 'candidate' ? candidate : oracle;
  const record = { side, iteration, traced, errors: [] };
  report.cases.push(record);
  const trace = new MouseTrace(() => performance.now(), m => m.startsWith('Input.') || m.startsWith('Target.') || m === 'Page.navigate');
  const action = async url => {
    const browser = await type.connectOverCDP(url, { timeout: 5000 });
    let context, page;
    try {
      context = await browser.newContext();
      page = await context.newPage();
      // Default remains exactly the original shared case. Explicit diagnostic
      // modes change only dispatch scheduling and are NOT parity-gate fixes.
      // Original has no evaluation, focus, paint wait, retry or reset here;
      // only the separately labelled painted-batch control adds a paint wait.
      await page.goto(fixture, { timeout: 5000 });
      switch (mode) {
        case 'original': await page.mouse.click(100, 100); break;
        case 'serial-buttons':
          await page.mouse.move(100, 100);
          await page.mouse.down();
          await page.mouse.up();
          break;
        case 'delayed-click': await page.mouse.click(100, 100, { delay: 1 }); break;
        case 'painted-batch':
          // Explicit experimental precondition, never used in original mode.
          await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(null)))));
          await page.mouse.click(100, 100);
          break;
        default: throw new Error(`Unknown click diagnostic mode: ${mode}`);
      }
      record.result = await page.evaluate("document.getElementById('result').textContent");
      record.after = await page.evaluate(() => {
        const r = document.getElementById('target').getBoundingClientRect();
        return { rect: { x: r.x, y: r.y, width: r.width, height: r.height },
          hit: document.elementFromPoint(100, 100)?.id, ready: document.readyState,
          focused: document.hasFocus(), width: innerWidth, height: innerHeight,
          scale: devicePixelRatio, result: document.getElementById('result').textContent };
      });
      if (observeEvents) {
        record.events = await page.evaluate(() => window.clickSnapshot());
        assert.equal(record.events.overflow, false);
        assert.ok(record.events.records.length <= 64);
      }
    } catch (error) { record.errors.push({ phase: 'operation', message: String(error?.message ?? error).slice(0, 4096) }); }
    finally {
      for (const [phase, close] of [['page', () => page?.close()], ['context', () => context?.close()], ['browser', () => browser.close()]]) {
        try { await diagnosticDeadline(close(), `click ${phase} cleanup`); }
        catch (error) { record.errors.push({ phase, message: String(error?.message ?? error).slice(0, 4096) }); }
      }
      if (browser.pwrightMetrics) {
        record.ffiAtClose = browser.pwrightFfiMetrics();
        // The close Promise can publish before a preexisting terminal watch's
        // queued JS settlement. Preserve that first observation, then use the
        // same bounded delivery drain as the shared/public differential gates.
        for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
        record.resources = { host: browser.pwrightMetrics(), ffi: browser.pwrightFfiMetrics() };
        for (const [key, value] of Object.entries(record.resources.host)) assert.equal(value, key === 'connected' ? false : 0);
        for (const value of Object.values(record.resources.ffi)) assert.equal(value, 0);
      }
    }
  };
  if (traced) await withCdpCommandRelay(endpoint, trace, action);
  else await action(endpoint);
  if (traced) { record.inputs = trace.inputs; record.trace = trace.report(); }
  process.stdout.write(`${JSON.stringify(record)}\n`);
  assert.equal(record.errors.length, 0, 'do not continue after a setup or cleanup failure');
  if (record.result !== 'touched') process.exitCode = 1;
}

try {
  for (const traced of [false, true]) for (let iteration = 0; iteration < 5; iteration++) {
    for (const side of ['oracle', 'candidate']) await run(side, iteration, traced);
  }
} catch (error) { report.failure = String(error?.message ?? error).slice(0, 4096); process.exitCode = 1; }
finally {
  const text = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(text) <= 1024 * 1024);
  await writeFile(output, text, { flag: 'wx' });
}
