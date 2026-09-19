import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import artifactConfig from '../src/native-artifact.cjs';
import { collectDragKeyboard } from './drag-keyboard-cases.mjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_DRAG_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_DRAG_KEYBOARD_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
let oracle, candidate;
try {
  const session = await observer.newBrowserCDPSession();
  try {
    // At most one live fixture record; both sides are executed serially.
    const owned = new Map();
    const observeOwned = async url => {
      assert.equal(owned.size, 0);
      const targets = (await session.send('Target.getTargets')).targetInfos.filter(target => target.url === url);
      assert.equal(targets.length, 1);
      assert.equal(typeof targets[0].browserContextId, 'string');
      assert.ok(targets[0].browserContextId.length > 0);
      owned.set(url, { target: targets[0].targetId, context: targets[0].browserContextId });
    };
    const assertRemoved = async url => {
      const targets = (await session.send('Target.getTargets')).targetInfos;
      assert.ok(!targets.some(target => target.url === url), 'owned fixture target must be absent');
      const record = owned.get(url);
      if (record) {
        assert.ok(!targets.some(target => target.targetId === record.target || target.browserContextId === record.context));
        assert.ok(!(await session.send('Target.getBrowserContexts')).browserContextIds.includes(record.context));
        owned.delete(url);
      }
    };
    oracle = await collectDragKeyboard(upstream, { endpoint, fixture, label: 'oracle', observeOwned, assertRemoved });
    candidate = await collectDragKeyboard(await import('../src/index.mjs'), { endpoint, fixture, label: 'candidate', observeOwned, assertRemoved });
    assert.equal(owned.size, 0);
  } finally { await session.detach(); }
} finally { await diagnosticDeadline(observer.close(), 'drag-keyboard observer disconnect'); }
assert.equal(candidate.result.browser, oracle.result.browser);
const cases = oracle.result.observations.map((expected, index) => {
  const actual = candidate.result.observations[index];
  let match = false;
  try { assert.deepEqual(actual, expected); match = true; } catch { /* Retain the exact mismatch. */ }
  return { id: expected.id, match, oracle: expected, candidate: actual };
});
const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
const encoded = JSON.stringify({ oracle: 'playwright@1.58.2', artifact, nativeProfile: artifactConfig.nativeArtifact.profile, cases, metrics: candidate.metrics }, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 128 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
for (const sample of cases) console.log(`${sample.match ? 'MATCH' : 'FAIL'} ${sample.id}`);
assert.equal(cases.filter(sample => sample.match).length, cases.length);
console.log(`Public drag/Keyboard/Locator: ${cases.length}/${cases.length}; owned Contexts/targets absent and host owners zero`);
