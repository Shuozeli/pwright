'use strict';

const Module = require('node:module');
const path = require('node:path');

const candidate = process.env.PWRIGHT_NODE_CANDIDATE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
if (!candidate) throw new Error('PWRIGHT_NODE_CANDIDATE is required');
if (!upstream) throw new Error('PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM is required');

const candidateEntry = require.resolve(path.resolve(candidate));
const testModeSource = path.join(path.resolve(upstream), 'tests', 'config', 'testMode.ts');
const originalLoad = Module._load;
let adaptedCandidate;

function publishOfficialBuiltins() {
  const global = globalThis;
  global.builtins = {
    setTimeout: global.setTimeout?.bind(global),
    clearTimeout: global.clearTimeout?.bind(global),
    setInterval: global.setInterval?.bind(global),
    clearInterval: global.clearInterval?.bind(global),
    requestAnimationFrame: global.requestAnimationFrame?.bind(global),
    cancelAnimationFrame: global.cancelAnimationFrame?.bind(global),
    requestIdleCallback: global.requestIdleCallback?.bind(global),
    cancelIdleCallback: global.cancelIdleCallback?.bind(global),
    performance: global.performance,
    Intl: global.Intl,
    Date: global.Date,
  };
}

async function installOfficialBuiltins(page) {
  // Playwright's own UtilityScript exposes these only while its upstream test
  // build isUnderTest. Reproduce that fixture input here; it is not a Pwright
  // public API and is never installed outside this official-test preload.
  await page.evaluate(publishOfficialBuiltins);
  return page;
}

function candidateForOfficialFixtures(parent, isMain) {
  if (adaptedCandidate) return adaptedCandidate;
  const implementation = originalLoad(candidateEntry, parent, isMain);
  if (!implementation?.chromium)
    throw new Error(`Pwright candidate has no chromium export: ${candidateEntry}`);

  // @playwright/test's fixture recorder consumes these Playwright-client
  // internals even when trace and screenshot fixtures are unused. Keep the
  // browser adapter test-only, but preserve the candidate-owned APIRequest
  // implementation so official tests exercise it rather than a placeholder.
  const listeners = new Set();
  const instrumentation = new Proxy({}, {
    get(target, property) {
      if (typeof property !== 'string') return target[property];
      if (property === 'addListener') return listener => listeners.add(listener);
      if (property === 'removeListener') return listener => listeners.delete(listener);
      if (property === 'removeAllListeners') return () => listeners.clear();
      if (property.startsWith('run')) {
        return async (...args) => {
          for (const listener of listeners) await listener[property]?.(...args);
        };
      }
      if (property.startsWith('on')) {
        return (...args) => {
          for (const listener of listeners) listener[property]?.(...args);
        };
      }
      return target[property];
    },
  });
  const contexts = new Set();
  const request = Object.create(implementation.request);
  Object.defineProperty(request, 'newContext', {
    configurable: false,
    enumerable: true,
    writable: false,
    value: async (...args) => {
      const context = await implementation.request.newContext(...args);
      // The upstream @playwright/test artifact recorder reaches into these
      // client internals for request contexts that a test intentionally leaves
      // open. They are test-harness protocol, not public request behavior.
      context._wrapApiCall ??= async callback => callback({ internal: true });
      context._tracing ??= {
        async start() {},
        async startChunk() {},
        async stop() {},
        async stopChunk() {},
      };
      return context;
    },
  });
  const endpoint = process.env.PWRIGHT_CANDIDATE_CDP_ENDPOINT;
  const chromium = Object.create(implementation.chromium);
  if (endpoint) {
    chromium.launch = async () => {
      process.stderr.write(
        `[pwright-candidate] browserType.launch fixture adapter -> connectOverCDP(${endpoint})\n`,
      );
      const browser = await implementation.chromium.connectOverCDP(endpoint);
      browser._connection = { headers: [] };
      const closeBrowser = browser.close.bind(browser);
      browser.close = () => closeBrowser();
      const newContext = browser.newContext.bind(browser);
      browser.newContext = async options => {
        // @playwright/test applies per-test context options through this
        // client-instrumentation hook before Browser.newContext. The candidate
        // has no upstream ChannelOwner, so the official adapter must drive the
        // same already-registered fixture hook explicitly.
        const preparedOptions = { ...(options ?? {}) };
        await instrumentation.runBeforeCreateBrowserContext(preparedOptions);
        const context = await newContext(preparedOptions);
        contexts.add(context);
        // UtilityScript creates this test-only global for every document. An
        // initial Page evaluation alone is lost on navigation, so register the
        // same fixture input through the candidate's normal init-script API.
        await context.addInitScript(publishOfficialBuiltins);
        const newPage = context.newPage.bind(context);
        context.newPage = async (...args) =>
          installOfficialBuiltins(await newPage(...args));
        const closeContext = context.close.bind(context);
        context.close = async () => {
          try {
            return await closeContext();
          } finally {
            contexts.delete(context);
          }
        };
        return context;
      };
      return browser;
    };
  }
  adaptedCandidate = Object.assign(Object.create(implementation), implementation, {
    chromium,
    _allContexts: () => [...contexts],
    _allPages: () => [...contexts].flatMap(context => context.pages()),
    _instrumentation: instrumentation,
    request,
  });
  return adaptedCandidate;
}

Module._load = function pwrightCandidateLoad(request, parent, isMain) {
  if (request === 'playwright-core' && parent?.filename === testModeSource) {
    process.stderr.write(
      `[pwright-candidate] ${testModeSource} -> ${candidateEntry}\n`,
    );
    return candidateForOfficialFixtures(parent, isMain);
  }
  return originalLoad(request, parent, isMain);
};
