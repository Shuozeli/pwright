'use strict';

// Preload the upstream reporter-helper tests before the runner or fixture
// modules. The test bodies stay upstream-owned; no Pwright runtime is loaded.

const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const workerThreads = require('node:worker_threads');

const ROOT = __dirname;
const MANIFEST = path.join(ROOT, 'playwright-core-fit-to-width-source-only-v1-manifest.json');
const NO_IO_GUARD = path.join(ROOT, 'playwright-core-image-tools-source-only-guard.cjs');
const NO_IO_GUARD_SHA256 = '2d2f86427433f8567c049356727170cd628059f9fb9141ee916a6116f237fce2';
const COMMIT = '961381ec73a9fa2661d92504eab1a6bd6eaffc75';
const SUITE = 'core463-private-fit-to-width';
const CASE = 'private-fit-to-width-v1';
const RUNNER = 'tests/playwright-test/stable-test-runner/node_modules/@playwright/test/cli.js';
const WORKER = 'tests/playwright-test/stable-test-runner/node_modules/playwright/lib/common/process.js';
const CONFIG = 'tests/playwright-test/playwright.config.ts';
const TEST = 'tests/playwright-test/fit-to-width.spec.ts';
const ARGS = [TEST];
const PROJECTS = ['playwright-test'];
const SOURCE_FILES = [
  'packages/playwright/lib/reporters/base.js',
  'packages/playwright/src/reporters/base.ts',
  'tests/config/commonFixtures.ts',
  'tests/config/serverFixtures.ts',
  TEST,
  'tests/playwright-test/playwright-test-fixtures.ts',
  CONFIG,
  'tests/playwright-test/stable-test-runner/index.js',
  'tests/playwright-test/stable-test-runner/package-lock.json',
].sort();
const SOURCE_SHA256 = 'b79360d265bc9f66eda62558a41e6e62b1f8ea3fae94b976ebe936a4a975b0cf';
const TEST_SHA256 = 'ee10bc8218f91870611fbc097c27759838e2a2946bd434a2d1459a579d8662e3';
const IDS_SHA256 = '99b4845c0c87acbb50f4d5ba1caa0cc1d747c77443f95740211c45e6a5dd6282';
const FILE_SPEC_SHA256 = 'efa42c756737f3d2f704707742eca8f44c7febe189a57150e8131d69971959ba';
const INVOCATIONS = [
  { line: 20, projectId: 'playwright-test', specId: '43ecc55b07560c6b6ffc-c2e63de005eea391e61b' },
  { line: 25, projectId: 'playwright-test', specId: '43ecc55b07560c6b6ffc-9aca4b4e10409bacd54b' },
];

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function verifySuite(manifestPath, suiteId, upstream) {
  if (path.resolve(manifestPath) !== MANIFEST || suiteId !== SUITE ||
      sha256(NO_IO_GUARD) !== NO_IO_GUARD_SHA256)
    throw new Error('fit-to-width manifest, suite, or no-I/O guard changed');
  const manifest = JSON.parse(fs.readFileSync(MANIFEST, 'utf8'));
  assert.deepStrictEqual(Object.keys(manifest).sort(), ['commit', 'repository', 'schema', 'suites', 'tag']);
  if (manifest.schema !== 1 || manifest.repository !== 'https://github.com/microsoft/playwright.git' ||
      manifest.tag !== 'v1.58.0' || manifest.commit !== COMMIT || manifest.suites.length !== 1)
    throw new Error('fit-to-width source-only upstream revision changed');
  const suite = manifest.suites[0];
  assert.deepStrictEqual(Object.keys(suite).sort(), [
    'args', 'config', 'expectedFiles', 'expectedFileSpecIdsSha256', 'expectedInvocationIdsSha256',
    'expectedInvocations', 'expectedProjectIds', 'expectedTestsPerProject', 'fixtureMode', 'id',
    'requiresBrowser', 'role', 'runner', 'sourceOnlyAdapter', 'sourceOnlyCase',
    'upstreamSourceSetSha256',
  ].sort());
  if (suite.id !== SUITE || suite.role !== 'implementation-internal-source-only' ||
      suite.runner !== RUNNER || suite.config !== CONFIG ||
      suite.sourceOnlyAdapter !== 'upstream-private-source-only-no-pwright-runtime' ||
      suite.requiresBrowser !== false || suite.fixtureMode !== 'pinned-private-source-only-no-io' ||
      suite.sourceOnlyCase !== CASE || suite.upstreamSourceSetSha256 !== SOURCE_SHA256 ||
      suite.expectedInvocationIdsSha256 !== IDS_SHA256 ||
      suite.expectedFileSpecIdsSha256 !== FILE_SPEC_SHA256 ||
      suite.expectedTestsPerProject !== 2 || suite.expectedFiles !== 1)
    throw new Error('fit-to-width source-only suite changed');
  assert.deepStrictEqual(suite.args, ARGS);
  assert.deepStrictEqual(suite.expectedProjectIds, PROJECTS);
  assert.deepStrictEqual(suite.expectedInvocations, INVOCATIONS);
  if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE ||
      process.env.CI || process.env.GITHUB_ACTIONS || process.env.GITLAB_CI ||
      (process.env.JENKINS_URL && process.env.BUILD_URL) ||
      fs.existsSync(path.join(upstream, '.env')))
    throw new Error('fit-to-width source-only lane forbids runtime or environment overrides');

  const pairs = SOURCE_FILES.map(relative => {
    const file = path.join(upstream, relative);
    if (!fs.lstatSync(file).isFile())
      throw new Error(`fit-to-width input is not a regular file: ${relative}`);
    return [relative, sha256(file)];
  });
  const sourceSha256 = crypto.createHash('sha256').update(JSON.stringify(pairs)).digest('hex');
  if (sourceSha256 !== SOURCE_SHA256 || sha256(path.join(upstream, TEST)) !== TEST_SHA256)
    throw new Error('fit-to-width pinned source set changed');

  const ts = require(path.join(upstream, 'node_modules/typescript'));
  const tree = ts.createSourceFile(TEST, fs.readFileSync(path.join(upstream, TEST), 'utf8'), ts.ScriptTarget.Latest, true);
  const titles = new Map([[20, 'chinese characters'], [25, 'surrogate pairs']]);
  const found = new Set();
  let privateImport = false;
  for (const statement of tree.statements) {
    if (ts.isImportDeclaration(statement) &&
        statement.moduleSpecifier.text === 'packages/playwright/lib/reporters/base' &&
        ts.isNamedImports(statement.importClause?.namedBindings))
      privateImport = statement.importClause.namedBindings.elements.some(element =>
        element.name.text === 'fitToWidth' && !element.isTypeOnly);
  }
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'test') {
      const line = tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
      const callback = node.arguments.find(argument => ts.isArrowFunction(argument) || ts.isFunctionExpression(argument));
      if (found.has(line) || titles.get(line) !== node.arguments[0]?.text || !callback ||
          callback.parameters.length !== 0 || !callback.body || !ts.isBlock(callback.body))
        throw new Error('fit-to-width declaration is not one of two pinned zero-fixture tests');
      let used = false;
      function visitBody(bodyNode) {
        if (ts.isIdentifier(bodyNode) && bodyNode.text === 'fitToWidth') used = true;
        ts.forEachChild(bodyNode, visitBody);
      }
      visitBody(callback.body);
      if (!used)
        throw new Error('fit-to-width selected body stopped using the private reporter helper');
      found.add(line);
    }
    ts.forEachChild(node, visit);
  }
  visit(tree);
  if (!privateImport || found.size !== 2)
    throw new Error('fit-to-width private import or selected tests changed');
  return {
    case: CASE,
    sourceFiles: SOURCE_FILES,
    sourceSha256,
    testSha256: TEST_SHA256,
    expectedInvocationIdsSha256: IDS_SHA256,
    expectedFileSpecIdsSha256: FILE_SPEC_SHA256,
    expectedProjectIds: PROJECTS,
    expectedInvocations: INVOCATIONS,
    expectedTestsPerProject: 2,
  };
}

function activate() {
  const installed = Symbol.for('pwright.coreFitToWidthSourceOnlyV1GuardInstalled');
  if (globalThis[installed]) return;
  const manifest = process.env.PWRIGHT_CORE_SOURCE_ONLY_MANIFEST;
  const suite = process.env.PWRIGHT_CORE_SOURCE_ONLY_SUITE;
  const mode = process.env.PWRIGHT_CORE_SOURCE_ONLY_MODE;
  const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
  const receipt = process.env.PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT;
  if (!manifest || !suite || !['candidate', 'oracle'].includes(mode) || !upstream || !receipt)
    throw new Error('fit-to-width source-only guard environment is incomplete');
  const source = verifySuite(manifest, suite, upstream);
  const isWorker = !workerThreads.isMainThread || typeof process.send === 'function' ||
      process.env.PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER === '1';
  const entrypoint = path.join(upstream, isWorker ? WORKER : RUNNER);
  if (process.argv[1] && path.resolve(process.argv[1]) !== entrypoint)
    throw new Error(`fit-to-width unapproved Node entrypoint: ${process.argv[1]}`);

  // Reuse the audited no-network/no-process hooks, without activating that
  // module's image-tools selection against this manifest.
  const active = process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE;
  delete process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE;
  let noIo;
  try {
    noIo = require(NO_IO_GUARD);
  } finally {
    process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE = active;
  }
  noIo.installNoIoGuard({ allowRunnerFork: !isWorker, workerScript: path.join(upstream, WORKER) });
  globalThis[installed] = true;
  fs.appendFileSync(receipt, JSON.stringify({
    kind: !workerThreads.isMainThread ? 'loader-thread-no-io' : isWorker ? 'worker-no-io' : 'runner-no-io',
    pid: process.pid, threadId: workerThreads.threadId, suite, mode, case: source.case,
  }) + '\n', { flag: 'a' });
}

if (require.main === module) {
  if (process.argv[2] !== '--check' || process.argv.length !== 6)
    throw new Error('usage: fit-to-width guard --check MANIFEST SUITE UPSTREAM');
  process.stdout.write(JSON.stringify(verifySuite(process.argv[3], process.argv[4], process.argv[5])) + '\n');
} else if (process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE === '1') {
  activate();
}

module.exports = { verifySuite };
