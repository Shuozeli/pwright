import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const provider = process.env.PWRIGHT_HANDLE_PROVIDER ?? 'both';
assert.ok(['both', 'oracle', 'candidate'].includes(provider));
const candidate = provider === 'oracle' ? undefined : (await import('@shuozeli/pwright')).chromium;
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const phaseTrace = process.env.PWRIGHT_HANDLE_OWNER_FRAME_TRACE === '1';

async function traced(label, phase, action) {
  const start = performance.now();
  const emit = (event, error) => {
    if (phaseTrace) process.stderr.write(`${JSON.stringify({
      diagnostic: 'handle-owner-frame-phase', timestamp: new Date().toISOString(),
      label, phase, event, elapsedMs: Math.round(performance.now() - start),
      ...(error === undefined ? {} : { error: String(error).slice(0, 512) }),
    })}\n`);
  };
  emit('start');
  try { const value = await action(); emit('ok'); return value; }
  catch (error) { emit('error', error); throw error; }
}

async function withPage(type, exercise, label) {
  const trace = (phase, action) => traced(label, phase, action);
  const browser = await trace('Browser connect', () => type.connectOverCDP(endpoint, { timeout: 5000 }));
  const held = [];
  let context, primary;
  try {
    context = await trace('Context create', () => browser.newContext());
    const page = await trace('Page create', () => context.newPage());
    const keep = async pending => {
      const handle = await pending;
      held.push(handle);
      return handle;
    };
    const retain = (source, argument) => keep(page.evaluateHandle(source, argument));
    return await trace('Exercise', () => exercise({ browser, context, page, keep, retain, label }));
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    const failures = [];
    const attempt = async (phase, action) => {
      try { await trace(phase, () => diagnosticDeadline(action(), phase)); }
      catch (error) { failures.push(error); }
    };
    // Keep every wrapper strongly reachable while checking explicit cleanup.
    for (const [index, handle] of held.entries()) await attempt(`Cleanup Handle ${index + 1}`, () => handle.dispose());
    if (context) await attempt('Cleanup Context', () => context.close());
    await attempt('Cleanup Browser', () => browser.close());
    if (browser.pwrightMetrics) await attempt('Zero ownership', async () => {
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().reservedBytes; turn++) await setImmediate();
      const host = browser.pwrightMetrics();
      assert.equal(host.activeLeases, 0);
      assert.equal(host.cleanupInFlight, 0);
      assert.equal(host.activeOperations, 0);
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    });
    if (failures.length) throw new AggregateError(primary ? [primary, ...failures] : failures, 'owner Frame cleanup failed');
  }
}

function paired(name, exercise, options = {}) {
  test(`${provider}: ElementHandle.ownerFrame ${name}`, { timeout: 25000, ...options }, async () => {
    // Arrange: independent Contexts, Pages and retained owners for each provider.
    const expected = provider === 'candidate' ? undefined : await withPage(oracle, exercise, `oracle: ${name}`);
    // Act
    const actual = provider === 'oracle' ? undefined : await withPage(candidate, exercise, `candidate: ${name}`);
    // Assert
    if (provider === 'both') assert.deepEqual(actual, expected);
  });
}

async function addChild(page) {
  await page.evaluate(async () => {
    const iframe = document.createElement('iframe');
    iframe.name = 'owner-frame-child';
    iframe.srcdoc = '<title>owner child</title><body><div id="child-node">child</div></body>';
    const loaded = new Promise(resolve => iframe.addEventListener('load', resolve, { once: true }));
    document.body.append(iframe);
    await loaded;
  });
}

paired('returns the existing main Frame wrapper repeatedly', async ({ page, retain }) => {
  // Arrange
  const handle = await retain(() => document.body);
  const expected = page.mainFrame();
  // Act
  const first = await handle.ownerFrame();
  const second = await handle.ownerFrame();
  // Assert
  assert.equal(handle.asElement(), handle);
  assert.equal(first, expected);
  assert.equal(second, first);
  assert.equal(first.page(), page);
  return { main: first === expected, repeat: second === first };
});

paired('keeps the owner of a removed DOM node', async ({ page, retain }) => {
  // Arrange
  const handle = await retain(() => {
    const node = document.createElement('div');
    document.body.append(node);
    return node;
  });
  await handle.evaluate(node => node.remove());
  // Act
  const owner = await handle.ownerFrame();
  // Assert
  assert.equal(await handle.evaluate(node => node.isConnected), false);
  assert.equal(owner, page.mainFrame());
  return { connected: false, main: owner === page.mainFrame() };
});

paired('keeps the source world through same-document navigation', async ({ page, retain }) => {
  // Arrange
  const handle = await retain(() => document.body);
  await page.evaluate(() => { location.hash = 'owner-frame-same-document'; });
  // Act
  const owner = await handle.ownerFrame();
  // Assert
  assert.equal(owner, page.mainFrame());
  assert.equal(await handle.evaluate(node => node === document.body), true);
  return owner === page.mainFrame();
});

paired('works with all persistent Handle slots occupied and releases temporary admission', async ({ browser, page, retain, keep }) => {
  // Arrange: one source plus 63 independently published children fills the
  // candidate's selected 64-result capacity without depending on JavaScript GC.
  const source = await retain(() => Object.fromEntries(
    Array.from({ length: 63 }, (_, index) => [String(index), document.body]),
  ));
  const properties = await source.getProperties();
  for (const handle of properties.values()) await keep(handle);
  assert.equal(properties.size, 63);
  const target = properties.get('0');
  const settle = async () => {
    if (!browser.pwrightMetrics) return undefined;
    for (let turn = 0; turn < 100; turn++) {
      const host = browser.pwrightMetrics();
      if (host.activeOperations === 0 && host.pendingCalls === 0 && host.cleanupInFlight === 0)
        return { activeLeases: host.activeLeases, liveObjects: host.liveObjects, liveScopes: host.liveScopes };
      await setImmediate();
    }
    throw new Error('owner Frame temporary ownership did not settle');
  };
  const baseline = await settle();
  // Act
  const owners = [];
  for (let index = 0; index < 8; index++) owners.push(await target.ownerFrame());
  // Assert
  assert.ok(owners.every(owner => owner === page.mainFrame()));
  assert.deepEqual(await settle(), baseline);
  assert.equal(await target.evaluate(node => node.tagName), 'BODY');
  return { owners: owners.length, children: properties.size };
});

paired('resolves a child document rather than the creation Frame', async ({ page, retain }) => {
  // Arrange: the retained object lives in the main evaluation world, but its
  // ownerDocument belongs to a different same-process Frame.
  await addChild(page);
  const handle = await retain(() => document.querySelector('iframe').contentDocument.getElementById('child-node'));
  // Act
  const owner = await handle.ownerFrame();
  // Assert: lookup after native settlement also covers newly published topology.
  const child = page.frames().find(frame => frame.name() === 'owner-frame-child');
  assert.ok(child);
  assert.equal(owner, child);
  assert.equal(owner.parentFrame(), page.mainFrame());
  return { name: owner.name(), parent: owner.parentFrame() === page.mainFrame() };
});

paired('follows adoption into a newly created child Frame', async ({ page, retain }) => {
  // Arrange
  const handle = await retain(() => {
    const node = document.createElement('div');
    node.id = 'adopted-owner-node';
    document.body.append(node);
    return node;
  });
  assert.equal(await handle.ownerFrame(), page.mainFrame());
  await addChild(page);
  await page.evaluate(() => document.querySelector('iframe').contentDocument.body.append(document.getElementById('adopted-owner-node')));
  // Act
  const owner = await handle.ownerFrame();
  // Assert
  assert.equal(owner, page.frames().find(frame => frame.name() === 'owner-frame-child'));
  assert.equal(await handle.evaluate(node => node.ownerDocument.defaultView.frameElement.name), 'owner-frame-child');
  return { name: owner.name(), sameNode: await handle.evaluate(node => node.id) };
});

for (const node of ['element', 'document']) {
  paired(`returns null for an orphan ${node}`, async ({ retain }) => {
    // Arrange
    const handle = await retain(kind => {
      const orphan = document.implementation.createHTMLDocument('orphan');
      return kind === 'document' ? orphan : orphan.body;
    }, node);
    // Act
    const owner = await handle.ownerFrame();
    // Assert
    assert.equal(handle.asElement(), handle);
    assert.equal(owner, null);
    return owner;
  });
}

paired('resolves a retained Document node to the canonical main Frame', async ({ page, retain }) => {
  // Arrange
  const handle = await retain(() => document);
  // Act
  const owner = await handle.ownerFrame();
  // Assert
  assert.equal(handle.asElement(), handle);
  assert.equal(owner, page.mainFrame());
  assert.equal(await handle.evaluate(node => node.nodeType), 9);
  return { document: true, main: owner === page.mainFrame() };
});

for (const producer of ['evaluateHandle', 'getProperty', 'getProperties']) {
  paired(`preserves Frame identity through ${producer} after source disposal`, async ({ page, retain, keep }) => {
    // Arrange
    const source = await retain(() => ({ node: document.body }));
    let handle;
    if (producer === 'getProperties') {
      const properties = await source.getProperties();
      for (const child of properties.values()) await keep(child);
      assert.deepEqual([...properties.keys()], ['node']);
      handle = properties.get('node');
    } else {
      handle = await keep(producer === 'evaluateHandle'
        ? source.evaluateHandle(value => value.node) : source.getProperty('node'));
    }
    await source.dispose();
    // Act
    const owner = await handle.ownerFrame();
    // Assert
    assert.equal(handle.asElement(), handle);
    assert.equal(owner, page.mainFrame());
    return { main: owner === page.mainFrame(), tag: await handle.evaluate(node => node.tagName) };
  });
}

for (const retire of ['dispose', 'page-close', 'navigation', 'navigation-then-close']) {
  paired(`rejects a handle after ${retire}`, async ({ page, retain }) => {
    // Arrange
    const handle = await retain(() => document.body);
    if (retire === 'dispose') await handle.dispose();
    if (retire === 'page-close') await page.close();
    if (retire === 'navigation' || retire === 'navigation-then-close') await page.goto('data:text/html,<title>new owner document</title>');
    if (retire === 'navigation-then-close') await page.close();
    // Act: attach rejection handling immediately; a diagnostic deadline fails
    // this test and is never interpreted as the API's own rejection.
    const result = await diagnosticDeadline(handle.ownerFrame().then(
      () => ({ rejected: false }), error => ({ rejected: true, name: error.name, message: error.message }),
    ), `owner Frame after ${retire}`);
    // Assert
    assert.equal(result.rejected, true);
    assert.equal(result.name, 'Error');
    assert.equal(result.message, retire === 'navigation'
      ? 'elementHandle.ownerFrame: Execution context was destroyed, most likely because of a navigation'
      : 'elementHandle.ownerFrame: Target page, context or browser has been closed');
    return result;
  });
}

paired('returns null after the owner child detaches while source world survives', async ({ page, retain }) => {
  // Arrange
  await addChild(page);
  const handle = await retain(() => document.querySelector('iframe').contentDocument.body);
  await page.evaluate(() => document.querySelector('iframe').remove());
  // Act
  const owner = await handle.ownerFrame();
  // Assert
  assert.equal(owner, null);
  assert.equal(await page.evaluate(() => document.querySelector('iframe')), null);
  return owner;
});

paired('does not expose ElementHandle methods on primitive handles', async ({ retain }) => {
  // Arrange
  const handle = await retain(() => 42);
  // Act
  const method = typeof handle.ownerFrame;
  // Assert
  assert.equal(method, 'undefined');
  assert.equal(handle.asElement(), null);
  assert.equal(await handle.jsonValue(), 42);
  return method;
});

paired('resolves the existing sandbox OOPIF child Frame', async ({ page, keep, label }) => {
  // Arrange: reuse the separately served existing fixture. The independent
  // observer only reads target inventory, never evaluates or implements the API.
  const url = new URL(process.env.PWRIGHT_DRAG_FIXTURE);
  assert.equal(url.protocol, 'http:');
  assert.ok(url.hostname.endsWith('.ts.net'));
  url.searchParams.set('frame-mode', 'sandbox');
  url.searchParams.set('owner-frame-run', `${process.pid}-${label}`);
  const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let inventory;
  try {
    inventory = await observer.newBrowserCDPSession();
    await page.goto(url.href, { timeout: 5000 });
    let observing = true;
    let child;
    try {
      child = await diagnosticDeadline((async () => {
        while (observing) {
          const frame = page.frames().find(value => value.name() === 'keyboard-frame');
          if (frame) return frame;
          await setImmediate();
        }
        throw new Error('owner Frame observation cancelled');
      })(), 'sandbox child topology');
    } finally { observing = false; }
    const childUrl = new URL('raw-drag-frame.html', url);
    childUrl.search = url.search;
    const targets = (await inventory.send('Target.getTargets')).targetInfos;
    const parent = targets.filter(target => target.type === 'page' && target.url === url.href);
    assert.equal(parent.length, 1, 'exact owned parent target');
    assert.equal(targets.filter(target => target.type === 'iframe' && target.url === childUrl.href
      && target.browserContextId === parent[0].browserContextId).length, 1, 'native iframe target proves OOPIF routing');
    const handle = await keep(child.evaluateHandle(() => document.querySelector('#keyboard')));
    // Act
    const owner = await handle.ownerFrame();
    // Assert
    assert.equal(owner, child);
    assert.equal(owner.parentFrame(), page.mainFrame());
    return { name: owner.name(), child: owner === child, parent: owner.parentFrame() === page.mainFrame() };
  } finally {
    try { if (inventory) await diagnosticDeadline(inventory.detach(), 'owner Frame observer session cleanup'); }
    finally { await diagnosticDeadline(observer.close(), 'owner Frame observer disconnect'); }
  }
}, { skip: process.env.PWRIGHT_DRAG_FIXTURE ? false : 'requires the existing served PWRIGHT_DRAG_FIXTURE for native OOPIF proof' });
