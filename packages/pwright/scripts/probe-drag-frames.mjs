// Real same-process and sandboxed OOPIF oracle. Fail rather than infer isolation.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

assert.equal(process.env.PWRIGHT_TEST_CDP, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_DRAG_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_DRAG_FRAME_ORACLE;
assert.ok(typeof output === 'string' && isAbsolute(output));
const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const cases = JSON.parse(await readFile(new URL('../../../tests/data/raw-drag-frame-cases.json', import.meta.url), 'utf8'));
assert.equal(cases.length, 18);
const browser = await require('playwright').chromium.connectOverCDP(process.env.PWRIGHT_TEST_CDP, { timeout: 5000 });
const observations = [];
try {
  const native = await browser.newBrowserCDPSession();
  try {
    for (const { id, mode, operation, oopif } of cases) {
      assert.ok(['local', 'sandbox'].includes(mode));
      assert.ok(['escape', 'chord', 'sequential', 'timeout', 'detach', 'detach-finish', 'page-close', 'context-close', 'before-focus'].includes(operation));
      assert.equal(id, `${mode}-${operation}`);
      assert.equal(oopif, mode === 'sandbox');
      const context = await diagnosticDeadline(browser.newContext(), id);
      try {
        const page = await diagnosticDeadline(context.newPage(), id);
        const url = new URL(fixture);
        url.searchParams.set('frame-mode', mode);
        url.searchParams.set('frame-case', id);
        await page.goto(url.href, { timeout: 5000 });
        const childURL = new URL('raw-drag-frame.html', url); childURL.search = url.search;
        const child = page.frames().find(frame => frame.url() === childURL.href);
        assert.ok(child);
        await child.waitForFunction(() => typeof frameSnapshot === 'function', null, { timeout: 5000 });
        const targets = (await native.send('Target.getTargets')).targetInfos.filter(target => target.type === 'iframe' && target.url === childURL.href);
        assert.equal(targets.length, mode === 'sandbox' ? 1 : 0, `${id}: native process isolation proof`);
        await page.mouse.move(40, 40);
        if (!['sequential', 'before-focus'].includes(operation)) {
          await page.mouse.down();
          await diagnosticDeadline(page.mouse.move(70, 40), `${id}: capture`);
          assert.ok(await page.evaluate(() => dragFixture.events.includes('dragstart')));
        }
        const locator = child.locator(operation === 'before-focus' ? '#absent' : '#keyboard');
        let detachedSnapshot;
        if (operation === 'page-close' || operation === 'context-close') {
          const pending = locator.press('Escape', { timeout: 5000, delay: 60000, noWaitAfter: true }).then(() => ({ resolved: true }), error => ({ error }));
          await page.waitForFunction(() => dragFixture.events.includes('dragend'), null, { timeout: 5000 });
          const frame = await child.evaluate(() => frameSnapshot());
          const drag = await page.evaluate(() => dragFixture);
          assert.deepEqual(frame.keys, [], `${id}: destruction must precede keyup`);
          assert.equal(frame.overflow, false); assert.equal(drag.overflow, false);
          if (operation === 'page-close') await diagnosticDeadline(page.close(), `${id}: Page close`);
          else await diagnosticDeadline(context.close(), `${id}: Context close`);
          const result = await diagnosticDeadline(pending, `${id}: closed press`);
          assert.equal(result.error?.name, 'Error');
          assert.match(result.error.message, /Target page, context or browser has been closed/);
          assert.equal(page.isClosed(), true);
          const remaining = (await native.send('Target.getTargets')).targetInfos;
          assert.ok(!remaining.some(target => target.url === url.href || target.url === childURL.href));
          observations.push({ id, mode, operation, oopif: targets.length === 1, drag, frame });
          continue;
        }
        if (operation === 'before-focus') {
          const pending = locator.press('Escape', { timeout: 5000, noWaitAfter: true }).then(() => ({ resolved: true }), error => ({ error }));
          detachedSnapshot = await child.evaluate(() => frameSnapshot());
          assert.equal(detachedSnapshot.focused, false);
          assert.deepEqual(detachedSnapshot.keys, []);
          await page.evaluate(() => document.querySelector('iframe').remove());
          const result = await diagnosticDeadline(pending, `${id}: pre-focus detach`);
          assert.equal(result.error?.name, 'Error');
          assert.match(result.error.message, /Frame was detached/);
          assert.equal(child.isDetached(), true);
          await page.keyboard.down('Escape'); await page.keyboard.up('Escape');
        }
        else if (operation === 'sequential') await locator.pressSequentially('a中', { timeout: 5000 });
        else if (operation === 'detach' || operation === 'detach-finish') {
          const finish = operation === 'detach-finish';
          const pending = locator.press('Escape', { timeout: 5000, delay: finish ? 1500 : 60000, noWaitAfter: !finish }).then(() => ({ resolved: true }), error => ({ error }));
          await page.waitForFunction(() => dragFixture.events.includes('dragend'), null, { timeout: 5000 });
          detachedSnapshot = await child.evaluate(() => frameSnapshot());
          assert.deepEqual(detachedSnapshot.keys, [], `${id}: keyup must not precede Frame removal`);
          await page.evaluate(() => document.querySelector('iframe').remove());
          const result = await diagnosticDeadline(pending, `${id}: detached press`);
          if (finish) assert.equal(result.resolved, true);
          else assert.equal(result.error?.name, 'TimeoutError');
          assert.equal(child.isDetached(), true);
          await page.keyboard.down('Escape'); await page.keyboard.up('Escape');
        }
        else if (operation === 'timeout') {
          await assert.rejects(locator.press('Escape', { timeout: 2000, delay: 60000, noWaitAfter: true }), { name: 'TimeoutError' });
          await page.keyboard.down('Escape'); await page.keyboard.up('Escape');
        } else await locator.press(operation === 'chord' ? 'Control+Escape' : 'Escape', { timeout: 5000, noWaitAfter: true });
        await page.mouse.move(220, 40); await page.mouse.up();
        const drag = await page.evaluate(() => dragFixture);
        const frame = detachedSnapshot ?? await child.evaluate(() => frameSnapshot());
        assert.equal(drag.overflow, false); assert.equal(frame.overflow, false);
        observations.push({ id, mode, operation, oopif: targets.length === 1, drag, frame });
      } finally { await diagnosticDeadline(context.close(), `${id}: cleanup`); }
    }
  } finally { await native.detach(); }
} finally { await diagnosticDeadline(browser.close(), 'disconnect'); }
const encoded = JSON.stringify({ version: '1.58.2', browser: browser.version(), observations }, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 128 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
console.log(`Recorded ${observations.length} Frame cases with native OOPIF proof and owned cleanup`);
