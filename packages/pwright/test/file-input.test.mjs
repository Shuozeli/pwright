import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { relative } from 'node:path';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { once } from 'node:events';
import { Worker } from 'node:worker_threads';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = oracleRequire('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(process.env.PWRIGHT_NATIVE_PROFILE?.startsWith('file-input'));
const fixture = fileURLToPath(new URL('../../../crates/pwright-file-input-source/tests/fixture.txt', import.meta.url));
const payload = (name = 'bytes.bin', mimeType = 'application/octet-stream', buffer = Buffer.from([0, 255, 1])) => ({ name, mimeType, buffer });
const html = `<label id="label" for="single">Upload</label><input id="single" type="file">
<input id="multiple" type="file" multiple><input id="directory" type="file" webkitdirectory>
<input id="hidden" type="file" disabled style="display:none"><input id="text" type="text"><div id="not-input"></div>
<script>window.fileEvents=[];for(const type of ['input','change'])document.addEventListener(type,e=>window.fileEvents.push({type:e.type,composed:e.composed,bubbles:e.bubbles,trusted:e.isTrusted}));</script>`;

async function withPage(type, exercise) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    // Shared fixture setup only; Page.setContent is not in the candidate API.
    await page.evaluate(markup => {
      document.open();
      document.write(markup);
      document.close();
    }, html);
    return await exercise(page, context, browser);
  } finally {
    try { await diagnosticDeadline(context?.close(), 'upload owned Context cleanup'); }
    finally { await diagnosticDeadline(browser.close(), 'upload Browser disconnect'); }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
  }
}
async function observed(page, selector) {
  return page.evaluate(async selector => {
    const element = document.querySelector(selector);
    const files = element.files === null ? null : await Promise.all([...element.files].map(async file => ({
      name: file.name, type: file.type, size: file.size,
      bytes: [...new Uint8Array(await file.arrayBuffer())],
    })));
    return { files, events: window.fileEvents };
  }, selector);
}
function paired(name, exercise) {
  test(name, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise);
    // Act
    const actual = await withPage(candidate, exercise);
    // Assert
    assert.deepEqual(actual, expected);
  });
}
for (const sample of [
  { name: 'binary Buffer', files: () => payload() },
  { name: 'offset Buffer view', files: () => payload('offset.bin', undefined, Buffer.from([9, 0, 255, 1, 9]).subarray(1, 4)) },
  { name: 'empty Buffer', files: () => payload('empty.txt', '', Buffer.alloc(0)) },
  { name: 'empty MIME inference', files: () => payload('guess.txt', '') },
  { name: 'missing MIME inference', files: () => ({ name: 'guess.txt', buffer: Buffer.from('text') }) },
  { name: 'multiple payloads', selector: '#multiple', files: () => [payload(), payload('second.txt', 'text/plain')] },
  { name: 'empty array clear', files: () => [] },
  { name: 'label retargeting', selector: '#label', observed: '#single', files: () => payload() },
  { name: 'hidden disabled input', selector: '#hidden', files: () => payload() },
  { name: 'text input no-op', selector: '#text', files: () => payload() },
  { name: 'absolute path fixture', files: () => fixture },
  { name: 'relative path fixture', files: () => relative(process.cwd(), fixture) },
  { name: 'metadata getter mutates earlier bytes before materialization', selector: '#multiple', files: () => {
    const first = payload();
    return [first, { get name() { first.buffer[0] = 17; return 'second.txt'; }, mimeType: 'text/plain', buffer: Buffer.from('text') }];
  } },
]) {
  paired(`upload: ${sample.name}`, async page => {
    // Arrange
    const files = sample.files();
    const selector = sample.selector ?? '#single';
    if (sample.name === 'empty array clear') {
      await page.evaluate(() => {
        const transfer = new DataTransfer();
        transfer.items.add(new File(['old'], 'old.txt'));
        document.querySelector('#single').files = transfer.files;
      });
    }
    // Act
    await page.locator(selector).setInputFiles(files, { timeout: 5000, noWaitAfter: true });
    // Assert on actual native bytes, metadata and input/change delivery.
    return observed(page, sample.observed ?? selector);
  });
}

for (const sample of [
  { name: 'multiple files on single input', files: () => [payload(), payload()], error: /Non-multiple file input can only accept single file/ },
  { name: 'mixed paths and payloads', files: () => [fixture, payload()], error: /File paths cannot be mixed with buffers/ },
  { name: 'non-input element', selector: '#not-input', files: () => payload(), error: /Node is not an HTMLInputElement/ },
  { name: 'directory input clear', selector: '#directory', files: () => [], error: /\[webkitdirectory\] input requires passing a path to a directory/ },
  { name: 'missing file precedes selector', selector: '#missing', files: () => `${fixture}.absent`, error: /ENOENT|No such file/ },
  { name: 'strict duplicate selector', selector: 'input', files: () => payload(), error: /strict mode violation/ },
  { name: 'missing selector deadline', selector: '#missing', files: () => payload(), error: /Timeout 50ms exceeded/, timeout: 50 },
  { name: '50 MiB rejection', files: () => payload('limit.bin', undefined, Buffer.alloc(50 * 1024 * 1024)), error: /Cannot set buffer larger than 50Mb/ },
  { name: 'Uint8Array is not a Buffer', files: () => payload('array.bin', undefined, new Uint8Array([0, 255])), error: /Buffer|buffer/ },
  { name: 'numeric file name', files: () => payload(7), error: /string/ },
  { name: 'numeric MIME', files: () => payload('file.txt', 7), error: /string/ },
]) {
  paired(`upload error: ${sample.name}`, async page => {
    // Arrange
    const files = sample.files();
    // Act
    const result = await page.locator(sample.selector ?? '#single').setInputFiles(files, { timeout: sample.timeout ?? 5000 })
      .then(() => { throw new Error('expected upload failure'); }, error => error);
    // Assert
    assert.match(result.message, sample.error);
    assert.equal(await page.locator('#single').evaluate(element => element.files.length), 0);
    return result.name;
  });
}

paired('upload: 8 MiB succeeds beyond ordinary command ceiling', async page => {
  // Arrange
  const files = payload('large.bin', undefined, Buffer.alloc(8 * 1024 * 1024, 73));
  // Act
  await page.locator('#single').setInputFiles(files, { timeout: 10000 });
  // Assert in browser without copying megabytes back through evaluation.
  return page.locator('#single').evaluate(async element => {
    const file = element.files[0];
    const bytes = new Uint8Array(await file.arrayBuffer());
    return { size: file.size, allBytesMatch: bytes.every(byte => byte === 73) };
  });
});

test('upload rejects shared backing before browser mutation', { timeout: 15000 }, async () => {
  await withPage(candidate, async page => {
    // Arrange
    const files = payload('shared.bin', undefined, Buffer.from(new SharedArrayBuffer(4)));
    // Act
    const result = await page.locator('#single').setInputFiles(files).catch(error => error);
    // Assert: deliberate unsupported boundary, not a compatibility claim.
    assert.equal(result.name, 'UnsupportedOperationError');
    assert.match(result.message, /shared file-input Buffer backing/);
    assert.equal(await page.locator('#single').evaluate(element => element.files.length), 0);
  });
});

test('upload detects an earlier Buffer detached by a later metadata getter', { timeout: 15000 }, async () => {
  await withPage(candidate, async page => {
    // Arrange
    const backing = new ArrayBuffer(4);
    const first = payload('first.bin', undefined, Buffer.from(backing));
    const files = [first, { get name() {
      structuredClone(backing, { transfer: [backing] });
      return 'second.txt';
    }, mimeType: 'text/plain', buffer: Buffer.from('next') }];
    // Act
    const result = await page.locator('#multiple').setInputFiles(files).catch(error => error);
    // Assert: no raw view survives getter reentry or reaches browser mutation.
    assert.equal(result.name, 'TypeError');
    assert.match(result.message, /detached ArrayBuffer/);
    assert.equal(await page.locator('#multiple').evaluate(element => element.files.length), 0);
  });
});

test('upload preserves a thrown getter without retaining source or FFI admission', { timeout: 15000 }, async () => {
  await withPage(candidate, async page => {
    // Arrange
    const original = new Error('getter failure');
    const files = { get name() { throw original; } };
    // Act
    const result = await page.locator('#single').setInputFiles(files).catch(error => error);
    // Assert
    assert.equal(result, original);
    assert.equal(await page.locator('#single').evaluate(element => element.files.length), 0);
  });
});

test('upload source capacity is bounded even for empty batches and recovers without GC', { timeout: 15000 }, async () => {
  await withPage(candidate, async (page, context) => {
    // Arrange: callback-scoped source admission happens before Promise return.
    const outcomes = ['#missing-one', '#missing-two'].map(selector =>
      page.locator(selector).setInputFiles([], { timeout: 5000 }).catch(error => error));
    // Act
    const result = await page.locator('#single').setInputFiles([]).catch(error => error);
    // Assert
    assert.match(result.message, /file-input source capacity exhausted/);
    await context.close();
    for (const outcome of await Promise.all(outcomes))
      assert.match(outcome.message, /closed/);
  });
});

test('completed uploads release source slots without awaiting JavaScript GC', { timeout: 15000 }, async () => {
  await withPage(candidate, async page => {
    // Arrange: retain all JavaScript payloads; no GC is requested.
    const inputs = Array.from({ length: 4 }, (_, index) => payload(`${index}.bin`));
    // Act
    for (const input of inputs) await page.locator('#single').setInputFiles(input);
    // Assert: more completions than the two-slot source capacity.
    assert.equal(await page.locator('#single').evaluate(element => element.files[0].name), '3.bin');
    assert.equal(inputs.length, 4);
  });
});

for (const phase of ['resolving', 'queued']) {
  test(`upload Worker termination during ${phase} retires only its owned native Context`, { timeout: 25000 }, async () => {
    // Arrange: an independent diagnostic connection identifies only this fixture.
    const fixtureUrl = `about:blank#node149-upload-worker-${phase}`;
    let worker, control, target, observer;
    const errors = [];
    try {
      worker = new Worker(new URL('./worker-file-input.mjs', import.meta.url), {
        workerData: { endpoint, fixtureUrl, phase },
      });
      worker.on('error', error => { if (errors.length < 2) errors.push(error); });
      const [ready] = await once(worker, 'message', { signal: AbortSignal.timeout(10000) });
      assert.equal(ready.phase, 'ready');
      observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
      control = await observer.newBrowserCDPSession();
      const targets = (await control.send('Target.getTargets')).targetInfos.filter(info => info.url === fixtureUrl);
      assert.equal(targets.length, 1);
      [target] = targets;
      assert.ok(target.browserContextId);
      const receipt = once(worker, 'message', { signal: AbortSignal.timeout(10000) });
      worker.postMessage('upload');
      const [state] = await receipt;
      assert.equal(state.phase, phase);
      assert.equal(state.ffi.commands, 1);
      assert.ok(state.ffi.reservedBytes >= 1024 * 1024);
      if (phase === 'queued') {
        assert.equal(state.ffi.queuedCommands, 1);
        const pages = observer.contexts().flatMap(context => context.pages()).filter(page => page.url() === fixtureUrl);
        assert.equal(pages.length, 1);
        assert.deepEqual(await pages[0].locator('#file').evaluate(async element => {
          const file = element.files[0];
          return { size: file.size, allBytesMatch: new Uint8Array(await file.arrayBuffer()).every(byte => byte === 73) };
        }), { size: 1024 * 1024, allBytesMatch: true });
      }
      // Act: source or an undelivered result still belongs to the Worker Env.
      assert.equal(await diagnosticDeadline(worker.terminate(), 'upload Worker termination'), 1);
      // Assert: no blanket Browser.close, and exact owned identities retired.
      assert.deepEqual(errors, []);
      assert.equal(observer.isConnected(), true);
      assert.ok(!(await control.send('Target.getTargets')).targetInfos.some(info => info.targetId === target.targetId));
      assert.ok(!(await control.send('Target.getBrowserContexts')).browserContextIds.includes(target.browserContextId));
    } finally {
      try { if (worker) await diagnosticDeadline(worker.terminate(), 'upload Worker final cleanup'); }
      finally {
        try {
          if (control && target) {
            const contexts = await control.send('Target.getBrowserContexts');
            if (contexts.browserContextIds.includes(target.browserContextId))
              await control.send('Target.disposeBrowserContext', { browserContextId: target.browserContextId });
          }
        } finally {
          await control?.detach();
          await diagnosticDeadline(observer?.close(), 'upload observer disconnect');
        }
      }
    }
  });
}

for (const scope of ['Page', 'Frame']) {
  const owner = page => scope === 'Page' ? page : page.mainFrame();
  paired(`${scope}.setInputFiles selects the first match by default`, async page => {
    // Arrange
    const files = payload();
    // Act
    await owner(page).setInputFiles('input[type=file]', files, { timeout: 5000 });
    // Assert
    const state = await page.evaluate(() => [...document.querySelectorAll('input[type=file]')].map(input => input.files.length));
    assert.deepEqual(state, [1, 0, 0, 0]);
    return observed(page, '#single');
  });
  paired(`${scope}.setInputFiles strict option rejects multiple matches without mutation`, async page => {
    // Arrange
    const files = payload();
    // Act
    const result = await owner(page).setInputFiles('input[type=file]', files, { strict: true, timeout: 5000 }).catch(error => error);
    // Assert
    assert.match(result.message, /strict mode violation/);
    assert.equal(await page.evaluate(() => document.querySelector('#single').files.length), 0);
    return result.name;
  });
  paired(`${scope}.setInputFiles forwards paths and retargets labels`, async page => {
    // Arrange
    const path = fixture;
    // Act
    await owner(page).setInputFiles('#label', path, { noWaitAfter: true, timeout: 5000 });
    // Assert
    return observed(page, '#single');
  });
  paired(`${scope}.setInputFiles retains the public timeout error attribution`, async page => {
    // Arrange
    const timeout = 50;
    // Act
    const result = await owner(page).setInputFiles('#missing', [], { timeout }).catch(error => error);
    // Assert
    assert.match(result.message, new RegExp(`^${scope.toLowerCase()}\\.setInputFiles:`));
    assert.match(result.message, /Timeout 50ms exceeded/);
    return result.name;
  });
}

paired('child Frame.setInputFiles uses its own document rather than Page mainFrame', async page => {
  // Arrange
  await page.evaluate(() => new Promise(resolve => {
    const child = document.createElement('iframe');
    child.name = 'upload-child';
    child.srcdoc = '<input id="single" type="file"><input id="second" type="file">';
    child.onload = () => resolve(null);
    document.body.append(child);
  }));
  const child = page.frame('upload-child');
  assert.ok(child);
  // Act
  await child.setInputFiles('input', payload(), { timeout: 5000 });
  // Assert
  const state = {
    main: await page.evaluate(() => document.querySelector('#single').files.length),
    child: await child.evaluate(() => [...document.querySelectorAll('input')].map(input => input.files.length)),
  };
  assert.deepEqual(state, { main: 0, child: [1, 0] });
  return state;
});

paired('Page.setInputFiles after Page.close rejects without creating browser work', async page => {
  // Arrange
  await page.close();
  // Act
  const result = await page.setInputFiles('input', [], { timeout: 5000 }).catch(error => error);
  // Assert
  assert.match(result.message, /closed/);
  return result.name;
});

test('Page and Frame selector uploads release temporary Rust query recipes without GC', { timeout: 15000 }, async () => {
  await withPage(candidate, async (page, _context, browser) => {
    // Arrange
    const before = browser.pwrightLocatorMetrics();
    // Act
    for (let index = 0; index < 8; index++) {
      await page.setInputFiles('input[type=file]', payload());
      await page.mainFrame().setInputFiles('#single', [], { strict: true });
    }
    // Assert: no JS Locator or GC-finalized temporary query is involved.
    assert.deepEqual(browser.pwrightLocatorMetrics(), before);
  });
});
