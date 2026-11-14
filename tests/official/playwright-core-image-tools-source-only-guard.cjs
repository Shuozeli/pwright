'use strict';

// Dedicated preload for the pinned upstream-private image-tools shard. Keep
// this separate from the historical Core source-only guard: its hash is part
// of already-recorded provenance.

const childProcess = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const workerThreads = require('node:worker_threads');

const CASE = 'private-image-tools-batch-v1';
const FIXTURE_MODE = 'pinned-private-source-only-no-io';
const RUNNER = 'tests/playwright-test/stable-test-runner/node_modules/@playwright/test/cli.js';
const WORKER = 'tests/playwright-test/stable-test-runner/node_modules/playwright/lib/common/process.js';
const CONFIG = 'tests/playwright-test/playwright.config.ts';
const ARGS = ['tests/image_tools/fixtures.spec.ts', 'tests/image_tools/unit.spec.ts'];
const SOURCE_FILES = [
  ...ARGS,
  'tests/image_tools/utils.ts',
  CONFIG,
  'tests/playwright-test/stable-test-runner/index.js',
  'tests/playwright-test/stable-test-runner/package-lock.json',
];
const SOURCE_DIGEST = '850aaf28f80268352daaeaed5f58fd20af593956eabd551dae6412804f5353bb';
const INVOCATION_DIGEST = '59b6aa848f07603093d68b74b2c9ccfaa6f10ea7ab239863a6e34b69a581ee74';
const FILE_SPEC_DIGEST = '909053ffba200a5d4db0ed53adaacfac8acb51be3abfcff0a4fad13df4a0f311';
const PINNED_COMMIT = '961381ec73a9fa2661d92504eab1a6bd6eaffc75';

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function verifySuite(manifestPath, suiteId, upstream) {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const suite = manifest.suites?.find(item => item.id === suiteId);
  if (manifest.schema !== 1 || manifest.repository !== 'https://github.com/microsoft/playwright.git' ||
      manifest.tag !== 'v1.58.0' || manifest.commit !== PINNED_COMMIT ||
      manifest.suites?.length !== 1 || !suite || suite.id !== 'core461-private-image-tools' ||
      suite.role !== 'implementation-internal-source-only' || suite.sourceOnlyCase !== CASE ||
      suite.fixtureMode !== FIXTURE_MODE || suite.requiresBrowser !== false ||
      suite.runner !== RUNNER || suite.config !== CONFIG ||
      suite.candidateAdapter !== 'test-mode-playwright-core' ||
      suite.project !== undefined || suite.oracleAdapter !== undefined ||
      suite.expectedTestsPerProject !== 25 || suite.expectedFiles !== 2 ||
      JSON.stringify(suite.args) !== JSON.stringify(ARGS) ||
      JSON.stringify(suite.expectedProjectIds) !== JSON.stringify(['image_tools']) ||
      suite.upstreamSourceSetSha256 !== SOURCE_DIGEST ||
      suite.expectedInvocationIdsSha256 !== INVOCATION_DIGEST ||
      suite.expectedFileSpecIdsSha256 !== FILE_SPEC_DIGEST)
    throw new Error('image-tools source-only suite has an unapproved runner, fixture, or source selection');
  if (process.env.IMAGE_TOOLS_FIXTURES)
    throw new Error('image-tools source-only suite forbids custom IMAGE_TOOLS_FIXTURES');
  if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE)
    throw new Error('image-tools source-only suite forbids loading the Pwright candidate runtime');
  if (fs.existsSync(path.join(upstream, '.env')))
    throw new Error('image-tools source-only suite forbids upstream .env overrides');
  if (process.env.GITHUB_ACTIONS || process.env.GITLAB_CI ||
      (process.env.JENKINS_URL && process.env.BUILD_URL))
    throw new Error('image-tools source-only suite forbids CI Git metadata subprocesses');

  const pngFiles = [];
  function visitPngs(relativeDirectory) {
    if (!fs.lstatSync(path.join(upstream, relativeDirectory)).isDirectory())
      throw new Error('image-tools source-only suite forbids symlink fixture directories');
    for (const entry of fs.readdirSync(path.join(upstream, relativeDirectory), { withFileTypes: true })) {
      const relative = `${relativeDirectory}/${entry.name}`;
      if (entry.isSymbolicLink())
        throw new Error('image-tools source-only suite forbids symlink fixtures');
      if (entry.isDirectory())
        visitPngs(relative);
      else if (entry.isFile() && /-(actual|expected)\.png$/.test(entry.name))
        pngFiles.push(relative);
    }
  }
  visitPngs('tests/image_tools/fixtures');
  if (pngFiles.length !== 42)
    throw new Error('image-tools source-only suite changed its 42 pinned PNG fixtures');
  const sourceFiles = [...SOURCE_FILES, ...pngFiles].sort();
  const digests = sourceFiles.map(relative => {
    const file = path.join(upstream, relative);
    if (!fs.lstatSync(file).isFile())
      throw new Error(`image-tools source-only suite source is not a regular file: ${relative}`);
    return [relative, sha256(file)];
  });
  const actualDigest = crypto.createHash('sha256').update(JSON.stringify(digests)).digest('hex');
  if (actualDigest !== SOURCE_DIGEST)
    throw new Error('image-tools source-only suite pinned source or PNG SHA-256 mismatch');

  // Verify the selected bodies are still fixture-free or use only testInfo.
  const ts = require(path.join(upstream, 'node_modules/typescript'));
  const unitTests = new Map([
    [24, 'srgb to lab conversion should work'],
    [38, 'colorDeltaE94 should work'],
    [45, 'fast stats and naive computation should match'],
    [66, 'ssim + fastStats should match "weber" algorithm from ssim.js'],
  ]);
  for (const relative of ARGS) {
    const file = path.join(upstream, relative);
    const tree = ts.createSourceFile(file, fs.readFileSync(file, 'utf8'), ts.ScriptTarget.Latest, true);
    const found = new Set();
    function visit(node) {
      if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'test') {
        const line = tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
        const callback = node.arguments.find(argument => ts.isArrowFunction(argument) || ts.isFunctionExpression(argument));
        if (!callback || !callback.body || !ts.isBlock(callback.body) || found.has(line))
          throw new Error('image-tools source-only suite changed a test body');
        if (relative.endsWith('/unit.spec.ts')) {
          if (unitTests.get(line) !== node.arguments[0]?.text || callback.parameters.length !== 0)
            throw new Error('image-tools source-only suite changed a zero-fixture unit test');
        } else if (line !== 37 || callback.parameters.length !== 2 ||
                   !ts.isObjectBindingPattern(callback.parameters[0].name) ||
                   callback.parameters[0].name.elements.length !== 0 ||
                   !ts.isIdentifier(callback.parameters[1].name) ||
                   callback.parameters[1].name.text !== 'testInfo') {
          throw new Error('image-tools source-only suite changed its PNG fixture test');
        }
        found.add(line);
      }
      ts.forEachChild(node, visit);
    }
    visit(tree);
    if (relative.endsWith('/unit.spec.ts') ?
        (found.size !== unitTests.size || [...found].some(line => !unitTests.has(line))) :
        (found.size !== 1 || !found.has(37)))
      throw new Error('image-tools source-only suite changed its test declarations');
  }
  return {
    case: CASE,
    sourceFiles: ARGS,
    sourceSha256: actualDigest,
    expectedInvocationIdsSha256: INVOCATION_DIGEST,
    expectedFileSpecIdsSha256: FILE_SPEC_DIGEST,
    expectedProjectIds: ['image_tools'],
    expectedTestsPerProject: 25,
  };
}

function installNoIoGuard({ allowRunnerFork = false, workerScript = '' } = {}) {
  const fail = operation => () => {
    throw new Error(`image-tools source-only worker forbids ${operation}`);
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
        throw new Error('image-tools source-only runner forbids unpinned child_process.fork');
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
  const installed = Symbol.for('pwright.coreImageToolsSourceOnlyGuardInstalled');
  if (globalThis[installed]) return;
  const manifest = process.env.PWRIGHT_CORE_SOURCE_ONLY_MANIFEST;
  const suite = process.env.PWRIGHT_CORE_SOURCE_ONLY_SUITE;
  const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
  const receipt = process.env.PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT;
  if (!manifest || !suite || !upstream || !receipt)
    throw new Error('image-tools source-only guard environment is incomplete');
  const verified = verifySuite(manifest, suite, upstream);
  const workerScript = path.join(upstream, WORKER);
  const isWorker = !workerThreads.isMainThread || typeof process.send === 'function' ||
      process.env.PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER === '1';
  const runnerScript = path.join(upstream, RUNNER);
  // Node preloads run before argv[1] is populated on supported Node releases.
  if (process.argv[1] && path.resolve(process.argv[1]) !== (isWorker ? workerScript : runnerScript))
    throw new Error(`image-tools source-only guard is attached to an unapproved Node entrypoint: ${process.argv[1]}`);
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
