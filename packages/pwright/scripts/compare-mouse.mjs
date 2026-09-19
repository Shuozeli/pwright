// Public Mouse subset against the pinned oracle, on the same external Chrome.
import assert from 'node:assert/strict';
import artifactConfig from '../src/native-artifact.cjs';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';
import { mouseConcurrencyCases } from './mouse-concurrency-cases.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_MOUSE_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_MOUSE_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
const setup = process.env.PWRIGHT_MOUSE_SETUP ?? 'original';
assert.ok(['original', 'background'].includes(setup), 'Unknown Mouse setup');
const raw = JSON.parse(await readFile(new URL('../../../tests/data/raw-mouse-cases.json', import.meta.url), 'utf8'));
assert.equal(raw.length, 25);

async function observe(page, predicate, argument) {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (await page.evaluate(predicate, argument)) return;
  }
  throw new Error('Mouse diagnostic observation did not arrive');
}
const snapshot = page => page.evaluate(() => window.mouseSnapshot());
const samples = raw.map(sample => ({ id: sample.id, async run(page) {
  const outcomes = [];
  let wheels = 0;
  for (const action of sample.actions) {
    let outcome = 'ok';
    try {
      switch (action.kind) {
        case 'move': await page.mouse.move(action.x, action.y, { steps: action.steps ?? 1 }); break;
        case 'click': await page.mouse.click(action.x, action.y, { button: action.button ?? 'left', clickCount: action.clickCount ?? 1, delay: action.delay ?? 0 }); break;
        case 'dblclick': await page.mouse.dblclick(action.x, action.y, { button: action.button ?? 'left', delay: action.delay ?? 0 }); break;
        case 'down': case 'up': await page.mouse[action.kind]({ button: action.button ?? 'left', clickCount: action.clickCount ?? 1 }); break;
        case 'wheel': await page.mouse.wheel(action.deltaX, action.deltaY); break;
        case 'keyDown': await page.keyboard.down(action.key); break;
        case 'keyUp': await page.keyboard.up(action.key); break;
        default: throw new Error(`Unknown Mouse action: ${action.kind}`);
      }
    } catch (error) {
      // This catalog compares rejection and unchanged state, not error wording.
      if (action.error !== 'invalid-steps' || !/steps.*integer/.test(error.message)) throw error;
      outcome = 'invalid-steps';
    }
    assert.equal(outcome, action.error ?? 'ok');
    outcomes.push(outcome);
    if (action.kind === 'wheel') {
      wheels++;
      await observe(page, count => window.mouseSnapshot().wheels === count, wheels);
    }
  }
  return { outcomes, snapshot: await snapshot(page) };
} }));
samples.push({ id: 'stable-mouse-across-navigation', async run(page, _context, prepare) {
  const mouse = page.mouse;
  await mouse.move(100, 120);
  await page.keyboard.down('Shift');
  await page.goto(page.url(), { timeout: 5000 });
  await prepare(page);
  assert.equal(page.mouse, mouse);
  await mouse.down();
  await mouse.up();
  await page.keyboard.up('Shift');
  return snapshot(page);
} });
samples.push({ id: 'two-pages-isolate-mouse-state', async run(page, context, prepare) {
  await page.mouse.move(100, 120);
  await page.keyboard.down('Shift');
  const other = await context.newPage();
  try {
    await other.goto(page.url(), { timeout: 5000 });
    await prepare(other);
    await other.mouse.down();
    await other.mouse.up();
    const second = await snapshot(other);
    await page.mouse.down();
    await page.mouse.up();
    await page.keyboard.up('Shift');
    return { first: await snapshot(page), second };
  } finally { await other.close(); }
} });
samples.push({ id: 'closed-page-rejects', async run(page) {
  const mouse = page.mouse;
  await page.close();
  await assert.rejects(mouse.click(10, 20), /closed/i);
  return 'target-closed';
} });
samples.push({ id: 'close-page-during-click-delay', async run(page) {
  const pending = page.mouse.click(100, 120, { delay: 60000 }).then(
    () => 'unexpected-success', error => /closed/i.test(error.message) ? 'target-closed' : Promise.reject(error));
  await observe(page, () => window.mouseSnapshot().records.some(event => event.type === 'mousedown'));
  await page.close();
  return pending;
} });

samples.push(...mouseConcurrencyCases());

const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
const inventory = await observer.newBrowserCDPSession();
let guardContext;
let guardOwner;
let guardTarget;
async function prepare(page) {
  // Optional diagnostic setup only. Route ambient native pointer events to an
  // owned blank Page; do not filter fixture events or alter input API calls.
  if (guardTarget) await inventory.send('Target.activateTarget', { targetId: guardTarget });
  await page.evaluate(() => window.resetMouse());
}
async function collect(provider, label) {
  const browser = await provider.chromium.connectOverCDP(endpoint, { timeout: 5000 });
  const observations = [];
  let metrics;
  try {
    for (const [index, sample] of samples.entries()) {
      const context = await diagnosticDeadline(browser.newContext(), 'Mouse Context');
      let nativeContext;
      try {
        const page = await diagnosticDeadline(context.newPage(), 'Mouse Page');
        const url = new URL(fixture);
        url.searchParams.set('mouse-public-probe', `${label}-${index}-${process.pid}`);
        await page.goto(url.href, { timeout: 5000 });
        const { targetInfos } = await inventory.send('Target.getTargets');
        const targets = targetInfos.filter(target => target.type === 'page' && target.url === url.href);
        assert.equal(targets.length, 1);
        nativeContext = targets[0].browserContextId;
        assert.ok(nativeContext);
        await prepare(page);
        try {
          const value = await diagnosticDeadline(sample.run(page, context, prepare), `${label}/${sample.id}`);
          if (value?.snapshot) assert.equal(value.snapshot.overflow, false);
          observations.push({ id: sample.id, value });
        } catch (error) {
          observations.push({ id: sample.id, error: { name: error.name, message: error.message } });
        }
      } finally {
        await diagnosticDeadline(context.close(), 'Mouse Context cleanup');
        if (nativeContext) {
          const { browserContextIds } = await inventory.send('Target.getBrowserContexts');
          assert.equal(browserContextIds.includes(nativeContext), false, 'native Context must be absent');
          const { targetInfos } = await inventory.send('Target.getTargets');
          assert.equal(targetInfos.some(target => target.browserContextId === nativeContext), false, 'owned native targets must be absent');
        }
      }
    }
  } finally {
    await diagnosticDeadline(browser.close(), 'Mouse disconnect');
    metrics = browser.pwrightMetrics?.();
  }
  if (metrics) {
    assert.equal(metrics.activeLeases, 0);
    assert.equal(metrics.activeOperations, 0);
    assert.equal(metrics.cleanupInFlight, 0);
  }
  return { observations, metrics };
}
let oracle;
let candidate;
try {
  if (setup === 'background') {
    guardContext = await observer.newContext();
    const page = await guardContext.newPage();
    const url = `about:blank#pwright-mouse-input-guard-${process.pid}`;
    await page.goto(url, { timeout: 5000 });
    const targets = (await inventory.send('Target.getTargets')).targetInfos.filter(target => target.url === url);
    assert.equal(targets.length, 1);
    guardOwner = targets[0].browserContextId;
    guardTarget = targets[0].targetId;
    assert.equal(typeof guardOwner, 'string');
  }
  oracle = await collect(upstream, 'oracle');
  candidate = await collect(await import('../src/index.mjs'), 'candidate');
} finally {
  try {
    if (guardContext) await diagnosticDeadline(guardContext.close(), 'Mouse guard Context cleanup');
    if (guardOwner) {
      assert.ok(!(await inventory.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === guardOwner));
      assert.ok(!(await inventory.send('Target.getBrowserContexts')).browserContextIds.includes(guardOwner));
    }
  } finally { await diagnosticDeadline(observer.close(), 'Mouse observer disconnect'); }
}
const cases = oracle.observations.map((expected, index) => {
  const actual = candidate.observations[index];
  let match = false;
  if (!expected.error && !actual.error) {
    try { assert.deepEqual(actual, expected); match = true; } catch { /* Retain exact discrepancy. */ }
  }
  return { id: expected.id, match, oracle: expected, candidate: actual };
});
const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
const encoded = JSON.stringify({ oracle: 'playwright@1.58.2', artifact, nativeProfile: artifactConfig.nativeArtifact.profile, setup, cases, metrics: candidate.metrics }, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 4 * 1024 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
for (const sample of cases) console.log(`${sample.match ? 'MATCH' : 'FAIL'} ${sample.id}`);
assert.equal(cases.filter(sample => sample.match).length, cases.length);
console.log(`Public Mouse: ${cases.length}/${cases.length}; owned native Contexts/targets absent; host leases/operations/cleanup zero`);
