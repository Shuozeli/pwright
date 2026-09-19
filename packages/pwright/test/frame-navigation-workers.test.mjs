import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { Worker } from 'node:worker_threads';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: observerApi } = oracleRequire('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const base = process.env.PWRIGHT_TEST_BASE;
const bind = process.env.PWRIGHT_TEST_BIND;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(base && new URL(base).hostname.endsWith('.ts.net'));
assert.ok(bind, 'explicit Tailscale bind required');
assert.ok(['frame-navigation', 'drag-frame-navigation', 'concurrent-close-drag-frame-extra-headers'].includes(process.env.PWRIGHT_NATIVE_PROFILE));

for (const mode of ['owned-running', 'borrowed-running', 'owned-queued']) {
  test(`Worker termination at ${mode} Frame navigation boundary`, { timeout: 25000 }, async () => {
    // Arrange: the observer is diagnostic-only, never the candidate driver.
    const observer = await observerApi.connectOverCDP(endpoint, { timeout: 5000 });
    let worker, control, borrowed, owned, server, releaseBody;
    let signalExecution;
    const execution = new Promise(resolve => { signalExecution = resolve; });
    const errors = [];
    try {
      server = createServer((request, response) => {
        if (request.url === '/executed' && request.method === 'POST') {
          response.writeHead(204).end();
          signalExecution();
          return;
        }
        if (request.url !== '/document') { response.writeHead(404).end(); return; }
        response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
        response.write(`<!--${' '.repeat(2048)}--><script>fetch('/executed', { method: 'POST' })</script>`);
        releaseBody = () => response.end('<script>window.workerNavigationCompleted = true</script>');
        if (mode === 'owned-queued') releaseBody();
      });
      server.listen(0, bind);
      await once(server, 'listening');
      const destination = `http://${new URL(base).hostname}:${server.address().port}/document`;
      const fixture = `${base}/empty.html#frame-worker-${mode}`;
      if (mode === 'borrowed-running') {
        borrowed = await observer.contexts()[0].newPage();
        await borrowed.goto(fixture, { timeout: 5000 });
      }
      worker = new Worker(new URL('./worker-frame-navigation.mjs', import.meta.url), {
        workerData: { endpoint, fixture, destination, mode },
      });
      worker.on('error', error => { if (errors.length < 2) errors.push(error); });
      const [ready] = await once(worker, 'message', { signal: AbortSignal.timeout(10000) });
      assert.equal(ready.phase, 'ready');
      control = await observer.newBrowserCDPSession();
      const initial = (await control.send('Target.getTargets')).targetInfos.filter(target => target.url === fixture);
      assert.equal(initial.length, 1);
      [owned] = initial;
      if (!borrowed) assert.ok(owned.browserContextId, 'Worker-created Context must have native identity');
      const next = once(worker, 'message', { signal: AbortSignal.timeout(10000) });
      worker.postMessage('navigate');
      const [state] = await next;
      assert.equal(state.phase, mode === 'owned-queued' ? 'queued' : 'started');
      assert.equal(state.ffi.commands, 1);
      assert.ok(state.ffi.reservedBytes >= 512 * 1024);
      if (mode === 'owned-queued') assert.equal(state.ffi.queuedCommands, 1);
      await diagnosticDeadline(execution, 'Worker navigation document execution');
      // Act: pending native work or a queued owned result exists at this point.
      const exit = await diagnosticDeadline(worker.terminate(), 'Frame Worker termination');
      // Assert: inspect exact identities, not counts of other tasks' Pages.
      assert.equal(exit, 1);
      assert.deepEqual(errors, []);
      const after = (await control.send('Target.getTargets')).targetInfos;
      if (borrowed) {
        assert.ok(after.some(target => target.targetId === owned.targetId));
        assert.equal(borrowed.isClosed(), false);
        assert.equal(borrowed.url(), destination);
        assert.equal(await borrowed.evaluate(() => document.readyState), 'loading');
        releaseBody();
        await borrowed.waitForFunction(() => window.workerNavigationCompleted === true, null, { timeout: 5000 });
      } else {
        assert.ok(!after.some(target => target.targetId === owned.targetId));
        const contexts = await control.send('Target.getBrowserContexts');
        assert.ok(!contexts.browserContextIds.includes(owned.browserContextId));
      }
      assert.ok(observer.isConnected());
    } finally {
      // Release only this fixture's body before attempting failure cleanup.
      server?.closeAllConnections();
      try { if (worker) await diagnosticDeadline(worker.terminate(), 'Frame Worker final cleanup'); }
      finally {
        try {
          if (borrowed) await diagnosticDeadline(borrowed.close(), 'borrowed fixture Page cleanup');
          else if (control && owned) {
            const contexts = await control.send('Target.getBrowserContexts');
            if (contexts.browserContextIds.includes(owned.browserContextId))
              await control.send('Target.disposeBrowserContext', { browserContextId: owned.browserContextId });
          }
        } finally {
          await control?.detach();
          await diagnosticDeadline(observer.close(), 'Frame Worker observer disconnect');
          if (server) await new Promise(resolve => server.close(resolve));
        }
      }
    }
  });
}
