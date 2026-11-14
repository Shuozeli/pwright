'use strict';

// This preload permits one upstream-private, zero-fixture Android project
// invocation. It installs no-network/no-spawn hooks before fixture modules.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const workerThreads = require('node:worker_threads');

const MANIFEST = path.join(__dirname, 'playwright-core-android-glob-source-only-v1-manifest.json');
const NO_IO_GUARD = path.join(__dirname, 'playwright-core-image-tools-source-only-guard.cjs');
const NO_IO_GUARD_SHA256 = '2d2f86427433f8567c049356727170cd628059f9fb9141ee916a6116f237fce2';
const COMMIT = '961381ec73a9fa2661d92504eab1a6bd6eaffc75';
const SUITE = 'core464-private-android-glob';
const CASE = 'private-android-glob-v1';
const RUNNER = 'packages/playwright/cli.js';
const WORKER = 'packages/playwright/lib/common/process.js';
const CONFIG = 'tests/android/playwright.config.ts';
const TEST = 'tests/page/interception.spec.ts';
const ARGS = [`${TEST}:73`];
const PROJECT = 'android-page';
const SPEC_ID = 'c569f641042bf346bcbd-ff0d174e7981f09583b5';
const SOURCE_FILES = [
  'packages/playwright-core/lib/utils/isomorphic/urlMatch.js',
  'packages/playwright-core/src/utils/isomorphic/urlMatch.ts',
  RUNNER,
  WORKER,
  'tests/android/androidTest.ts',
  CONFIG,
  'tests/config/baseTest.ts',
  'tests/config/commonFixtures.ts',
  'tests/config/platformFixtures.ts',
  'tests/config/serverFixtures.ts',
  'tests/config/testModeFixtures.ts',
  'tests/electron/electronTest.ts',
  TEST,
  'tests/page/pageTest.ts',
].sort();
const SOURCE_SHA256 = '11f3507c4440e13baddbc535544bf737061b20ff622cc3888f40b9b74b8ac291';
const TEST_SHA256 = '1f0d3089d80ef3b9529f7bfd056e21435d78698a2ed4021bf8cf380d417b9676';
const IDS_SHA256 = '72d82784c03962b8ad3974f25d022aef3f5b33ea2591e0df6ba3327273ea2155';
const FILE_SPEC_SHA256 = 'b73728a8e39ede506d8cb4fe49eeae4e44af8a35484a79e511df1bdc5bd2662c';
const INVOCATIONS = [{ line: 73, projectId: PROJECT, specId: SPEC_ID }];

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function verifySuite(manifestPath, suiteId, upstream) {
  if (path.resolve(manifestPath) !== MANIFEST || suiteId !== SUITE ||
      sha256(NO_IO_GUARD) !== NO_IO_GUARD_SHA256)
    throw new Error('Android glob manifest, suite, or no-I/O helper changed');
  const manifest = JSON.parse(fs.readFileSync(MANIFEST, 'utf8'));
  assert.deepStrictEqual(Object.keys(manifest).sort(), ['commit', 'repository', 'schema', 'suites', 'tag']);
  if (manifest.schema !== 1 || manifest.repository !== 'https://github.com/microsoft/playwright.git' ||
      manifest.tag !== 'v1.58.0' || manifest.commit !== COMMIT || manifest.suites.length !== 1)
    throw new Error('Android glob upstream revision changed');
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
      suite.expectedTestsPerProject !== 1 || suite.expectedFiles !== 1)
    throw new Error('Android glob source-only suite changed');
  assert.deepStrictEqual(suite.args, ARGS);
  assert.deepStrictEqual(suite.expectedProjectIds, [PROJECT]);
  assert.deepStrictEqual(suite.expectedInvocations, INVOCATIONS);
  if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE ||
      process.env.CI || process.env.GITHUB_ACTIONS || process.env.GITLAB_CI ||
      (process.env.JENKINS_URL && process.env.BUILD_URL) ||
      fs.existsSync(path.join(upstream, '.env')))
    throw new Error('Android glob source-only lane forbids runtime or environment overrides');

  const pairs = SOURCE_FILES.map(relative => {
    const file = path.join(upstream, relative);
    if (!fs.lstatSync(file).isFile() || fs.lstatSync(file).isSymbolicLink())
      throw new Error(`Android glob input is not a regular file: ${relative}`);
    return [relative, sha256(file)];
  });
  const sourceSha256 = crypto.createHash('sha256').update(JSON.stringify(pairs)).digest('hex');
  if (sourceSha256 !== SOURCE_SHA256 || sha256(path.join(upstream, TEST)) !== TEST_SHA256)
    throw new Error('Android glob pinned source set changed');

  const config = fs.readFileSync(path.join(upstream, CONFIG), 'utf8');
  const pageTest = fs.readFileSync(path.join(upstream, 'tests/page/pageTest.ts'), 'utf8');
  const androidTest = fs.readFileSync(path.join(upstream, 'tests/android/androidTest.ts'), 'utf8');
  if (!config.includes("process.env.PWPAGE_IMPL = 'android'") ||
      !config.includes("name: 'android-page'") ||
      !config.includes("testDir: path.join(testDir, 'page')") ||
      /\b(?:globalSetup|globalTeardown|webServer)\s*:/.test(config) ||
      !pageTest.includes("if (process.env.PWPAGE_IMPL === 'android')") ||
      !pageTest.includes('impl = androidTest') ||
      !androidTest.includes('baseTest.extend<') ||
      SOURCE_FILES.filter(file => file.startsWith('tests/config/') ||
        file === 'tests/android/androidTest.ts' || file === 'tests/page/pageTest.ts')
          .some(file => /\bauto\s*:/.test(fs.readFileSync(path.join(upstream, file), 'utf8'))))
    throw new Error('Android fixture ancestry gained automatic device, server, or setup activity');

  const ts = require(path.join(upstream, 'node_modules/typescript'));
  const tree = ts.createSourceFile(TEST, fs.readFileSync(path.join(upstream, TEST), 'utf8'), ts.ScriptTarget.Latest, true);
  let privateImport = false;
  for (const statement of tree.statements) {
    if (ts.isImportDeclaration(statement) &&
        statement.moduleSpecifier.text === '../../packages/playwright-core/lib/utils/isomorphic/urlMatch' &&
        ts.isNamedImports(statement.importClause?.namedBindings)) {
      const symbols = new Set(statement.importClause.namedBindings.elements
          .filter(element => !element.isTypeOnly).map(element => element.name.text));
      privateImport = symbols.has('globToRegexPattern') && symbols.has('urlMatches');
    }
  }
  const selected = [];
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'it' &&
        tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1 === 73 &&
        node.arguments[0]?.text === 'should work with glob')
      selected.push(node);
    ts.forEachChild(node, visit);
  }
  visit(tree);
  if (!privateImport || selected.length !== 1)
    throw new Error('Android glob private import or selected test changed');
  const callback = selected[0].arguments.find(argument => ts.isArrowFunction(argument) || ts.isFunctionExpression(argument));
  if (!callback || callback.parameters.length !== 0 || !callback.body || !ts.isBlock(callback.body))
    throw new Error('Android glob selected body is not a zero-fixture test');
  const used = new Set();
  function visitBody(node) {
    if (ts.isIdentifier(node)) used.add(node.text);
    ts.forEachChild(node, visitBody);
  }
  visitBody(callback.body);
  if (!used.has('globToRegexPattern') || !used.has('urlMatches'))
    throw new Error('Android glob selected body stopped using private utilities');
  return {
    case: CASE,
    sourceFiles: SOURCE_FILES,
    sourceSha256,
    testSha256: TEST_SHA256,
    expectedInvocationIdsSha256: IDS_SHA256,
    expectedFileSpecIdsSha256: FILE_SPEC_SHA256,
    expectedProjectIds: [PROJECT],
    expectedInvocations: INVOCATIONS,
    expectedTestsPerProject: 1,
  };
}

function activate() {
  const installed = Symbol.for('pwright.coreAndroidGlobSourceOnlyV1GuardInstalled');
  if (globalThis[installed]) return;
  const manifest = process.env.PWRIGHT_CORE_SOURCE_ONLY_MANIFEST;
  const suite = process.env.PWRIGHT_CORE_SOURCE_ONLY_SUITE;
  const mode = process.env.PWRIGHT_CORE_SOURCE_ONLY_MODE;
  const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
  const receipt = process.env.PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT;
  if (!manifest || !suite || !['candidate', 'oracle'].includes(mode) || !upstream || !receipt)
    throw new Error('Android glob source-only guard environment is incomplete');
  const source = verifySuite(manifest, suite, upstream);
  const isWorker = !workerThreads.isMainThread || typeof process.send === 'function' ||
      process.env.PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER === '1';
  const entrypoint = path.join(upstream, isWorker ? WORKER : RUNNER);
  if (process.argv[1] && path.resolve(process.argv[1]) !== entrypoint)
    throw new Error(`Android glob unapproved Node entrypoint: ${process.argv[1]}`);
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
    throw new Error('usage: Android glob guard --check MANIFEST SUITE UPSTREAM');
  process.stdout.write(JSON.stringify(verifySuite(process.argv[3], process.argv[4], process.argv[5])) + '\n');
} else if (process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE === '1') {
  activate();
}

module.exports = { verifySuite };
