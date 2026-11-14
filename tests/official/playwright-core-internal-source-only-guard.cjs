'use strict';

// This preload is only for pinned upstream-private tests. It runs before the
// Playwright worker loads test or fixture modules and prevents accidental
// process, browser, CDP, or server activity in that worker.

const childProcess = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const workerThreads = require('node:worker_threads');

const FIXTURE_MODE = 'pinned-private-source-only-no-io';
const BATCH_CASE = 'private-internal-batch-v1';
const BATCH_ARGS = [
  'tests/library/unit',
  'tests/library/events',
  'tests/library/component-parser.spec.ts',
  'tests/library/css-parser.spec.ts',
  'tests/library/proxy-pattern.spec.ts',
  ...[514, 521, 528, 535, 553, 572, 584, 589, 615, 658]
      .map(line => `tests/library/locator-generator.spec.ts:${line}`),
];
const BATCH_SOURCE_DIGEST = 'd644bf46c87115333eece4fba6cbfde08ed5b92e066ae656f114e9e1195853d4';
const BATCH_INVOCATION_DIGESTS = {
  'tests/library/playwright.config.ts': '61624bddc42a9ec9aa964d1921fbf2258c964f2b974b67edc289ab73620c6ab5',
  'tests/bidi/playwright.config.ts': 'b655e9f2a151fc888d065ee374b6e9869ab2fc69763fac728339c253b328bd19',
};
const CASES = {
  'viewport-device-descriptors': {
    source: 'tests/library/browsercontext-viewport.spec.ts',
    line: 50,
    title: 'landscape viewport should have width larger than height',
    module: 'packages/playwright-core/lib/server/deviceDescriptors',
    symbols: ['deviceDescriptors'],
    libraryProjects: ['chromium-library', 'firefox-library', 'webkit-library'],
    bidiProjects: [
      'bidi-chrome-canary-library', 'bidi-chrome-stable-library', 'bidi-chromium-library',
      'moz-firefox-beta-library', 'moz-firefox-library', 'moz-firefox-nightly-library',
    ],
  },
  'glob-url-match': {
    source: 'tests/page/interception.spec.ts',
    line: 73,
    title: 'should work with glob',
    module: '../../packages/playwright-core/lib/utils/isomorphic/urlMatch',
    symbols: ['globToRegexPattern', 'urlMatches'],
    libraryProjects: ['chromium-page', 'firefox-page', 'webkit-page'],
    bidiProjects: [
      'bidi-chrome-canary-page', 'bidi-chrome-stable-page', 'bidi-chromium-page',
      'moz-firefox-beta-page', 'moz-firefox-nightly-page', 'moz-firefox-page',
    ],
  },
};

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function verifyBatchSuite(suite, upstream) {
  const config = suite.config;
  const invocationDigest = BATCH_INVOCATION_DIGESTS[config];
  const projectIds = config === 'tests/library/playwright.config.ts' ?
    CASES['viewport-device-descriptors'].libraryProjects :
    CASES['viewport-device-descriptors'].bidiProjects;
  if (!invocationDigest || suite.fixtureMode !== FIXTURE_MODE ||
      suite.role !== 'implementation-internal-source-only' || suite.requiresBrowser !== false ||
      suite.runner !== 'packages/playwright/cli.js' || suite.candidateAdapter !== 'test-mode-playwright-core' ||
      suite.project !== undefined || suite.oracleAdapter !== undefined ||
      suite.expectedTestsPerProject !== 284 || suite.expectedFiles !== 26 ||
      JSON.stringify(suite.args) !== JSON.stringify(BATCH_ARGS) ||
      suite.upstreamSourceSetSha256 !== BATCH_SOURCE_DIGEST ||
      suite.expectedInvocationIdsSha256 !== invocationDigest ||
      JSON.stringify(suite.expectedProjectIds) !== JSON.stringify(projectIds))
    throw new Error('source-only batch has an unapproved runner, fixture, or source selection');

  const sourceFiles = [
    ...fs.readdirSync(path.join(upstream, 'tests/library/unit'))
        .filter(name => name.endsWith('.spec.ts')).map(name => `tests/library/unit/${name}`),
    ...fs.readdirSync(path.join(upstream, 'tests/library/events'))
        .filter(name => name.endsWith('.spec.ts')).map(name => `tests/library/events/${name}`),
    'tests/library/component-parser.spec.ts',
    'tests/library/css-parser.spec.ts',
    'tests/library/proxy-pattern.spec.ts',
    'tests/library/locator-generator.spec.ts',
  ].sort();
  if (sourceFiles.length !== 26)
    throw new Error('source-only batch changed its exact 26-file selection');
  const sourceDigests = sourceFiles.map(relative => [relative, sha256(path.join(upstream, relative))]);
  const actualDigest = crypto.createHash('sha256').update(JSON.stringify(sourceDigests)).digest('hex');
  if (actualDigest !== BATCH_SOURCE_DIGEST)
    throw new Error('source-only batch pinned source SHA-256 mismatch');

  const ts = require(path.join(upstream, 'node_modules/typescript'));
  const relative = 'tests/library/locator-generator.spec.ts';
  const locatorSource = path.join(upstream, relative);
  const tree = ts.createSourceFile(locatorSource, fs.readFileSync(locatorSource, 'utf8'), ts.ScriptTarget.Latest, true);
  const selectedLines = new Set([514, 521, 528, 535, 553, 572, 584, 589, 615, 658]);
  const foundLines = new Set();
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'it') {
      const line = tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
      if (selectedLines.has(line)) {
        const callback = node.arguments.find(argument => ts.isArrowFunction(argument) || ts.isFunctionExpression(argument));
        if (!callback || callback.parameters.length !== 0 || !callback.body || !ts.isBlock(callback.body) ||
            !ts.isStringLiteral(node.arguments[0]) || foundLines.has(line))
          throw new Error('source-only batch locator selection is not a unique zero-fixture test');
        foundLines.add(line);
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(tree);
  if (foundLines.size !== selectedLines.size)
    throw new Error('source-only batch lost a pinned locator test declaration');
  return {
    case: BATCH_CASE,
    sourceFiles,
    sourceSha256: actualDigest,
    expectedInvocationIdsSha256: invocationDigest,
    expectedProjectIds: projectIds,
    expectedTestsPerProject: 284,
  };
}

function verifySuite(manifestPath, suiteId, upstream) {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const suite = manifest.suites?.find(item => item.id === suiteId);
  if (!suite || manifest.schema !== 1 || manifest.repository !== 'https://github.com/microsoft/playwright.git' ||
      manifest.tag !== 'v1.58.0' || manifest.commit !== '961381ec73a9fa2661d92504eab1a6bd6eaffc75')
    throw new Error('source-only guard requires the pinned Core v1.58.0 suite');
  if (suite.sourceOnlyCase === BATCH_CASE)
    return verifyBatchSuite(suite, upstream);
  const selected = CASES[suite.sourceOnlyCase];
  if (!selected || suite.fixtureMode !== FIXTURE_MODE || suite.role !== 'implementation-internal-source-only' ||
      suite.requiresBrowser !== false || suite.runner !== 'packages/playwright/cli.js' ||
      suite.candidateAdapter !== 'test-mode-playwright-core' || suite.project !== undefined ||
      suite.oracleAdapter !== undefined || suite.expectedTestsPerProject !== 1 || suite.expectedFiles !== 1 ||
      !['tests/library/playwright.config.ts', 'tests/bidi/playwright.config.ts'].includes(suite.config) ||
      JSON.stringify(suite.args) !== JSON.stringify([`${selected.source}:${selected.line}`]))
    throw new Error('source-only guard suite has an unapproved runner, fixture, or source selection');
  const expectedProjects = suite.config === 'tests/library/playwright.config.ts' ? 3 : 6;
  if (!Array.isArray(suite.expectedSpecIds) || suite.expectedSpecIds.length !== expectedProjects ||
      new Set(suite.expectedSpecIds).size !== expectedProjects ||
      !suite.expectedSpecIds.every(value => typeof value === 'string' && /^[a-f0-9]{20}-[a-f0-9]{20}$/.test(value)))
    throw new Error('source-only guard requires exact project-expanded spec IDs');
  const projectIds = suite.config === 'tests/library/playwright.config.ts' ?
    selected.libraryProjects : selected.bidiProjects;
  if (JSON.stringify(suite.expectedProjectIds) !== JSON.stringify(projectIds))
    throw new Error('source-only guard requires exact project IDs');

  const sourcePath = path.join(upstream, selected.source);
  if (!/^[a-f0-9]{64}$/.test(suite.upstreamTestSourceSha256) ||
      sha256(sourcePath) !== suite.upstreamTestSourceSha256)
    throw new Error('source-only guard pinned test source SHA-256 mismatch');
  const ts = require(path.join(upstream, 'node_modules/typescript'));
  const tree = ts.createSourceFile(sourcePath, fs.readFileSync(sourcePath, 'utf8'), ts.ScriptTarget.Latest, true);
  const imported = new Set();
  for (const statement of tree.statements) {
    if (!ts.isImportDeclaration(statement) || statement.moduleSpecifier.text !== selected.module ||
        !statement.importClause || statement.importClause.isTypeOnly ||
        !statement.importClause.namedBindings || !ts.isNamedImports(statement.importClause.namedBindings))
      continue;
    for (const element of statement.importClause.namedBindings.elements)
      if (!element.isTypeOnly && element.name.text === (element.propertyName?.text || element.name.text))
        imported.add(element.name.text);
  }
  if (!selected.symbols.every(symbol => imported.has(symbol)))
    throw new Error('source-only guard private import changed');

  const selectedTests = [];
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'it' &&
        tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1 === selected.line &&
        node.arguments.length >= 2 && ts.isStringLiteral(node.arguments[0]) &&
        node.arguments[0].text === selected.title)
      selectedTests.push(node);
    ts.forEachChild(node, visit);
  }
  visit(tree);
  if (selectedTests.length !== 1)
    throw new Error('source-only guard did not find exactly one pinned test declaration');
  const callback = selectedTests[0].arguments.find(argument => ts.isArrowFunction(argument) || ts.isFunctionExpression(argument));
  if (!callback || callback.parameters.length !== 0 || !callback.body || !ts.isBlock(callback.body))
    throw new Error('source-only guard requires a zero-fixture test body');
  const used = new Set();
  function visitBody(node) {
    if (ts.isIdentifier(node)) used.add(node.text);
    ts.forEachChild(node, visitBody);
  }
  visitBody(callback.body);
  if (!selected.symbols.every(symbol => used.has(symbol)))
    throw new Error('source-only guard private utility is no longer used by the selected body');
  return {
    case: suite.sourceOnlyCase,
    sourceFile: selected.source,
    sourceLine: selected.line,
    sourceSha256: suite.upstreamTestSourceSha256,
    expectedSpecIds: suite.expectedSpecIds,
    expectedProjectIds: suite.expectedProjectIds,
  };
}

function installNoIoGuard({ allowRunnerFork = false, workerScript = '' } = {}) {
  const fail = operation => () => {
    throw new Error(`source-only worker forbids ${operation}`);
  };
  const originalFork = childProcess.fork;
  for (const method of ['spawn', 'spawnSync', 'exec', 'execSync', 'execFile', 'execFileSync', 'fork'])
    childProcess[method] = fail(`child_process.${method}`);
  if (allowRunnerFork) {
    childProcess.fork = function sourceOnlyRunnerFork(modulePath, argsOrOptions, options) {
      const forkOptions = Array.isArray(argsOrOptions) ? options : argsOrOptions;
      if (modulePath !== workerScript || Array.isArray(argsOrOptions) ||
          !forkOptions || forkOptions.detached !== false ||
          !Array.isArray(forkOptions.stdio) || forkOptions.stdio[3] !== 'ipc' ||
          !forkOptions.env || forkOptions.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE !== '1' ||
          forkOptions.env.NODE_OPTIONS !== process.env.NODE_OPTIONS)
        throw new Error('source-only runner forbids unpinned child_process.fork');
      return originalFork.call(this, modulePath, argsOrOptions);
    };
  }
  const net = require('node:net');
  net.Socket.prototype.connect = fail('socket connect');
  net.Server.prototype.listen = fail('socket listen');
  net.connect = net.createConnection = fail('net connect');
  const tls = require('node:tls');
  tls.connect = fail('TLS connect');
  for (const name of ['node:http', 'node:https']) {
    const transport = require(name);
    transport.request = transport.get = fail(`${name} request`);
  }
  require('node:dgram').createSocket = fail('UDP socket');
  globalThis.fetch = fail('fetch');
  if (globalThis.WebSocket)
    globalThis.WebSocket = fail('WebSocket');
}

function activate() {
  const installed = Symbol.for('pwright.coreSourceOnlyGuardInstalled');
  if (globalThis[installed]) return;
  const manifest = process.env.PWRIGHT_CORE_SOURCE_ONLY_MANIFEST;
  const suite = process.env.PWRIGHT_CORE_SOURCE_ONLY_SUITE;
  const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
  const receipt = process.env.PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT;
  if (!manifest || !suite || !upstream || !receipt)
    throw new Error('source-only guard environment is incomplete');
  const verified = verifySuite(manifest, suite, upstream);
  const workerScript = path.join(upstream, 'packages/playwright/lib/common/process.js');
  // Node preloads run before argv[1] is populated on supported Node releases.
  // Playwright uses child_process.fork with IPC for every loader/test worker.
  const isWorker = !workerThreads.isMainThread || typeof process.send === 'function' ||
      process.env.PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER === '1';
  const runnerScript = path.join(upstream, 'packages/playwright/cli.js');
  if (process.argv[1] && path.resolve(process.argv[1]) !== (isWorker ? workerScript : runnerScript))
    throw new Error(`source-only guard is attached to an unapproved Node entrypoint: ${process.argv[1]}`);
  installNoIoGuard({ allowRunnerFork: !isWorker, workerScript });
  globalThis[installed] = true;
  fs.appendFileSync(receipt, JSON.stringify({
    kind: !workerThreads.isMainThread ? 'loader-thread-no-io' :
      isWorker ? 'worker-no-io' : 'runner-no-io',
    pid: process.pid, threadId: workerThreads.threadId, suite, case: verified.case,
  }) + '\n', { flag: 'a' });
}

if (require.main === module) {
  if (process.argv[2] !== '--check' || process.argv.length !== 6)
    throw new Error('usage: guard --check MANIFEST SUITE UPSTREAM');
  process.stdout.write(JSON.stringify(verifySuite(process.argv[3], process.argv[4], process.argv[5])) + '\n');
} else if (process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE === '1') {
  activate();
}

module.exports = { installNoIoGuard, verifySuite };
