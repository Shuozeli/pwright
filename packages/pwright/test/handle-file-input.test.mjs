import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

// Public contract lane, not a substitute Locator implementation. Oracle-only
// mode characterizes prerequisites; the default paired mode must stay red until
// the candidate implements the same retained-node and lifecycle semantics.
const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const provider = process.env.PWRIGHT_HANDLE_PROVIDER ?? 'both';
assert.ok(['both', 'oracle', 'candidate'].includes(provider));
const candidate = provider === 'oracle' ? undefined : (await import('@shuozeli/pwright')).chromium;
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = fileURLToPath(new URL('../../../crates/pwright-file-input-source/tests/fixture.txt', import.meta.url));
const payload = () => ({ name: 'bytes.bin', mimeType: 'application/octet-stream', buffer: Buffer.from([0, 255, 1]) });

async function withPage(type, exercise) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  let primary;
  const handles = [];
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    await page.evaluate(() => {
      document.body.innerHTML = '<label id="label" for="input">Upload</label><input id="input" type="file"><input id="second" type="file"><input id="text" type="text"><div id="other"></div>';
      window.inputEvents = [];
      for (const input of document.querySelectorAll('input')) {
        for (const type of ['input', 'change']) input.addEventListener(type, event => {
          if (window.inputEvents.length >= 16) throw Error('fixture event capacity exceeded');
          window.inputEvents.push({ id: input.id, type: event.type, trusted: event.isTrusted });
        });
      }
    });
    const retain = async (source, argument) => {
      const handle = await page.evaluateHandle(source, argument);
      handles.push(handle);
      assert.equal(handle.asElement(), handle);
      return handle;
    };
    return await exercise({ page, retain, handles });
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    const failures = [];
    // A failed disposal must not prevent the owned Context and connection from
    // being closed. No shared target/browser process is stopped here.
    for (const handle of handles) {
      try { await diagnosticDeadline(handle.dispose(), 'retained upload handle disposal'); }
      catch (error) { failures.push(error); }
    }
    try { if (context) await diagnosticDeadline(context.close(), 'handle upload Context cleanup'); }
    catch (error) { failures.push(error); }
    try { await diagnosticDeadline(browser.close(), 'handle upload Browser disconnect'); }
    catch (error) { failures.push(error); }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
    if (failures.length) throw new AggregateError(primary ? [primary, ...failures] : failures, 'handle upload cleanup failed');
  }
}

async function snapshot(handle) {
  return handle.evaluate(async input => ({
    id: input.id,
    connected: input.isConnected,
    files: input.files === null ? null : await Promise.all([...input.files].map(async file => {
      if (file.size > 4096) throw Error('fixture file exceeds observation bound');
      return { name: file.name, bytes: [...new Uint8Array(await file.arrayBuffer())] };
    })),
  }));
}
function paired(name, exercise) {
  test(`${provider}: handle upload ${name}`, { timeout: 25000 }, async () => {
    // Arrange: independently owned pages and retained handles per provider.
    const expected = provider === 'candidate' ? undefined : await withPage(oracle, exercise);
    // Act: the exact same public workload, without selector emulation.
    const actual = provider === 'oracle' ? undefined : await withPage(candidate, exercise);
    // Assert: exercise asserts intended behavior; paired mode also compares values.
    if (provider === 'both') assert.deepEqual(actual, expected);
  });
}

paired('keeps original node after selector identity changes', async ({ page, retain }) => {
  const handle = await retain(() => document.querySelector('#input'));
  await page.evaluate(() => {
    const original = document.querySelector('#input');
    original.id = 'original';
    const replacement = document.createElement('input');
    replacement.type = 'file';
    replacement.id = 'input';
    original.after(replacement);
  });
  await handle.setInputFiles(payload(), { timeout: 2000 });
  const result = await snapshot(handle);
  assert.equal(result.id, 'original');
  assert.deepEqual(result.files, [{ name: 'bytes.bin', bytes: [0, 255, 1] }]);
  assert.equal(await page.evaluate(() => document.querySelector('#input').files.length), 0);
  return result;
});

paired('retains a removed input rather than querying its replacement', async ({ page, retain }) => {
  const handle = await retain(() => document.querySelector('#input'));
  await handle.evaluate(input => {
    const replacement = input.cloneNode();
    input.replaceWith(replacement);
  });
  // A removed DOM node can still have a live owner document and native handle.
  assert.equal(await handle.ownerFrame(), page.mainFrame());
  await handle.setInputFiles(payload(), { timeout: 2000 });
  const result = await snapshot(handle);
  assert.equal(result.connected, false);
  assert.deepEqual(result.files, [{ name: 'bytes.bin', bytes: [0, 255, 1] }]);
  assert.equal(await page.evaluate(() => document.querySelector('#input').files.length), 0);
  return result;
});

paired('follows the retained label current control', async ({ page, retain }) => {
  const handle = await retain(() => document.querySelector('#label'));
  await handle.evaluate(label => { label.htmlFor = 'second'; });
  await handle.setInputFiles(payload(), { timeout: 2000 });
  const result = await page.evaluate(() => [...document.querySelectorAll('input[type=file]')].map(input => input.files.length));
  assert.deepEqual(result, [0, 1]);
  return result;
});

paired('uploads to hidden disabled input without a pointer action', async ({ page, retain }) => {
  const handle = await retain(() => document.querySelector('#input'));
  await handle.evaluate(input => { input.hidden = true; input.disabled = true; });
  await handle.setInputFiles(payload(), { timeout: 2000 });
  const events = await page.evaluate(() => window.inputEvents);
  assert.deepEqual(events, [{ id: 'input', type: 'input', trusted: false }, { id: 'input', type: 'change', trusted: false }]);
  return { node: await snapshot(handle), events };
});

paired('clears only the retained node', async ({ retain }) => {
  const handle = await retain(() => document.querySelector('#input'));
  await handle.evaluate(input => {
    const data = new DataTransfer();
    data.items.add(new File(['old'], 'old.txt'));
    input.files = data.files;
  });
  await handle.setInputFiles([], { timeout: 2000 });
  const result = await snapshot(handle);
  assert.deepEqual(result.files, []);
  return result;
});

paired('reads a caller path', async ({ retain }) => {
  const handle = await retain(() => document.querySelector('#input'));
  await handle.setInputFiles(fixture, { timeout: 2000 });
  const result = await snapshot(handle);
  assert.equal(result.files[0].name, 'fixture.txt');
  assert.ok(result.files[0].bytes.length > 0);
  return result;
});

paired('preserves text input success without mutation', async ({ retain }) => {
  const handle = await retain(() => document.querySelector('#text'));
  await handle.setInputFiles(payload(), { timeout: 2000 });
  const result = await snapshot(handle);
  assert.equal(result.files, null);
  return result;
});

paired('rejects non-input element without a selector retry', async ({ retain }) => {
  const handle = await retain(() => document.querySelector('#other'));
  await assert.rejects(handle.setInputFiles(payload(), { timeout: 2000 }), /Node is not an HTMLInputElement/);
  return 'non-input rejected';
});

for (const retire of ['dispose', 'page-close', 'navigation', 'frame-detach']) {
  paired(`rejects after ${retire} before reading payload getters`, async ({ page, retain }) => {
    if (retire === 'frame-detach') await page.evaluate(async () => {
      const iframe = document.createElement('iframe');
      const loaded = new Promise(resolve => iframe.addEventListener('load', resolve, { once: true }));
      iframe.srcdoc = '<input type="file">';
      document.body.append(iframe);
      await loaded;
    });
    const handle = await retain(retire === 'frame-detach'
      ? () => document.querySelector('iframe').contentDocument.querySelector('input')
      : () => document.querySelector('#input'));
    if (retire === 'dispose') await handle.dispose();
    if (retire === 'page-close') await page.close();
    if (retire === 'navigation') await page.goto('data:text/html,<input type=file id=input>');
    if (retire === 'frame-detach') await page.evaluate(() => document.querySelector('iframe').remove());
    let getters = 0;
    const files = { get name() { getters++; return 'bytes.bin'; }, mimeType: 'text/plain', buffer: Buffer.from('new') };
    let failure;
    try { await handle.setInputFiles(files, { timeout: 2000 }); }
    catch (error) { failure = { name: error.name, message: String(error.message).slice(0, 4096) }; }
    assert.ok(failure, 'retired handle must reject');
    process.stdout.write(`${JSON.stringify({ provider, retire, failure, getters })}\n`);
    const message = retire === 'navigation'
      ? 'elementHandle.setInputFiles: Execution context was destroyed, most likely because of a navigation'
      : retire === 'frame-detach' ? 'Cannot set input files to detached element'
      : 'elementHandle.setInputFiles: Target page, context or browser has been closed';
    assert.deepEqual(failure, { name: 'Error', message });
    assert.equal(getters, 0);
    return { failure, getters };
  });
}

paired('reports adopted node owner frame without changing handle identity', async ({ page, retain }) => {
  const handle = await retain(() => document.querySelector('#input'));
  await page.evaluate(async () => {
    const iframe = document.createElement('iframe');
    iframe.name = 'upload-owner';
    const loaded = new Promise(resolve => iframe.addEventListener('load', resolve, { once: true }));
    iframe.srcdoc = '<body></body>';
    document.body.append(iframe);
    await loaded;
    iframe.contentDocument.body.append(document.querySelector('#input'));
  });
  assert.equal(await handle.ownerFrame(), page.frames().find(frame => frame.name() === 'upload-owner'));
  await handle.setInputFiles(payload(), { timeout: 2000 });
  const result = await snapshot(handle);
  assert.deepEqual(result.files, [{ name: 'bytes.bin', bytes: [0, 255, 1] }]);
  assert.equal(await page.evaluate(() => document.querySelector('#input')), null);
  return result;
});

paired('keeps primitive handles distinct from element handles', async ({ page, handles }) => {
  const handle = await page.evaluateHandle(() => 42);
  handles.push(handle);
  assert.equal(handle.asElement(), null);
  const value = await handle.jsonValue();
  assert.equal(value, 42);
  await handle.dispose();
  await handle.dispose();
  return value;
});

for (const surface of ['page', 'frame', 'locator']) {
  paired(`accepts a retained element from ${surface} selector API`, async ({ page, handles }) => {
    const handle = surface === 'locator' ? await page.locator('#input').elementHandle()
      : surface === 'frame' ? await page.mainFrame().$('#input') : await page.$('#input');
    assert.ok(handle);
    handles.push(handle);
    assert.equal(handle.asElement(), handle);
    await handle.setInputFiles(payload(), { timeout: 2000 });
    const result = await snapshot(handle);
    assert.deepEqual(result.files, [{ name: 'bytes.bin', bytes: [0, 255, 1] }]);
    return result;
  });
}

paired('rejects a node without a browsing-context owner before payload getters', async ({ retain }) => {
  const handle = await retain(() => {
    const document = window.document.implementation.createHTMLDocument('detached');
    const input = document.createElement('input');
    input.type = 'file';
    document.body.append(input);
    return input;
  });
  assert.equal(await handle.ownerFrame(), null);
  let getters = 0;
  const files = { get name() { getters++; return 'bytes.bin'; }, mimeType: 'text/plain', buffer: Buffer.from('new') };
  await assert.rejects(handle.setInputFiles(files, { timeout: 2000 }), /Cannot set input files to detached element/);
  assert.equal(getters, 0);
  return { owner: null, getters };
});
