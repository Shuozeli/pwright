// Public-API diagnostic for native owned-Page recursive attachment. This is not
// a substitute for child Frame identity or command conformance.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import pwright from '../src/index.cjs';
import artifact from '../src/native-artifact.cjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

assert.equal(process.env.PWRIGHT_TEST_CDP, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_DRAG_FIXTURE);
assert.ok(fixture.hostname.endsWith('.ts.net'));
const selection = artifact.nativeArtifact;
console.log({ selection, sha256: createHash('sha256').update(await readFile(selection.path)).digest('hex') });
const browser = await pwright.chromium.connectOverCDP(process.env.PWRIGHT_TEST_CDP, { timeout: 15000 });
try {
  const context = await browser.newContext();
  let primary;
  try {
    const page = await context.newPage();
    fixture.searchParams.set('frame-mode', 'sandbox');
    fixture.searchParams.set('frame-case', `old-addon-owned-oopif-${process.pid}`);
    await page.goto(fixture.href, { timeout: 5000 });
    console.log({ navigated: true, title: await page.title() });
  } catch (error) {
    primary = error;
    console.error('Owned OOPIF primary error:', error);
  } finally {
    try { await diagnosticDeadline(context.close(), 'owned OOPIF Context cleanup'); }
    catch (cleanup) { throw primary ? new AggregateError([primary, cleanup], 'owned OOPIF action and cleanup failed') : cleanup; }
  }
  if (primary) throw primary;
} finally {
  await diagnosticDeadline(browser.close(), 'owned OOPIF Browser disconnect');
}
