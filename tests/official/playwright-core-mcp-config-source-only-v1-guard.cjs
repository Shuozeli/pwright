'use strict';

// Preload before upstream runner/worker imports. This shard executes exactly
// two fixture-free MCP configuration tests, not MCP or Pwright runtime tests.

const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const workerThreads = require('node:worker_threads');

const ROOT = __dirname;
const MANIFEST = path.join(ROOT, 'playwright-core-mcp-config-source-only-v1-manifest.json');
const SHARED_GUARD = path.join(ROOT, 'playwright-core-internal-source-only-guard.cjs');
const SHARED_GUARD_SHA256 = 'f80debd1433efa271ddd063af799332aac4cefc6386876b16775d38d448fde6a';
const COMMIT = '961381ec73a9fa2661d92504eab1a6bd6eaffc75';
const SUITE = 'core462-private-mcp-config-sandbox';
const CASE = 'private-mcp-config-sandbox-v1';
const RUNNER = 'packages/playwright/cli.js';
const WORKER = 'packages/playwright/lib/common/process.js';
const CONFIG = 'tests/mcp/playwright.config.ts';
const ARGS = ['tests/mcp/config.spec.ts:82', 'tests/mcp/config.spec.ts:87'];
const SOURCE_FILES = [
  'package-lock.json',
  RUNNER,
  WORKER,
  'packages/playwright/lib/mcp/browser/config.js',
  'packages/playwright/src/mcp/browser/config.ts',
  'tests/config/serverFixtures.ts',
  'tests/mcp/config.spec.ts',
  'tests/mcp/fixtures.ts',
  CONFIG,
].sort();
const SOURCE_SHA256 = 'f50fe7dec188ab2957f19b4681df621181821e296df251d940747e7fd297b0a8';
const IDS_SHA256 = '7be17705f68f69aca3afec656c3fe4142c71a49583c59a31b296d9c6a296b00c';
const PROJECTS = ['chrome', 'chromium', 'firefox', 'webkit'];
const INVOCATIONS = [
  [82, 'chrome', '9196cb1f5e62547dc5c9-69a718c5f51fb8dc7faa'],
  [82, 'chromium', '9196cb1f5e62547dc5c9-fd4a6ec16bc60a1e0666'],
  [82, 'firefox', '9196cb1f5e62547dc5c9-f6eabc76476719c34bf5'],
  [82, 'webkit', '9196cb1f5e62547dc5c9-ecaf9642b768357b001c'],
  [87, 'chrome', '9196cb1f5e62547dc5c9-95d0ab28bd553b34dec5'],
  [87, 'chromium', '9196cb1f5e62547dc5c9-28c2d94b2411e26d5592'],
  [87, 'firefox', '9196cb1f5e62547dc5c9-bd861023e2299e369470'],
  [87, 'webkit', '9196cb1f5e62547dc5c9-a52d59b5d1439fd414fb'],
];

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function verifySuite(manifestPath, suiteId, upstream) {
  if (path.resolve(manifestPath) !== MANIFEST || suiteId !== SUITE ||
      sha256(SHARED_GUARD) !== SHARED_GUARD_SHA256)
    throw new Error('MCP source-only manifest, suite, or no-I/O dependency changed');
  const manifest = JSON.parse(fs.readFileSync(MANIFEST, 'utf8'));
  assert.deepStrictEqual(Object.keys(manifest).sort(), ['commit', 'repository', 'schema', 'suites', 'tag']);
  if (manifest.schema !== 1 || manifest.repository !== 'https://github.com/microsoft/playwright.git' ||
      manifest.tag !== 'v1.58.0' || manifest.commit !== COMMIT || manifest.suites.length !== 1)
    throw new Error('MCP source-only upstream revision changed');
  const suite = manifest.suites[0];
  assert.deepStrictEqual(Object.keys(suite).sort(), [
    'args', 'config', 'expectedFiles', 'expectedInvocationIdsSha256', 'expectedInvocations',
    'expectedProjectIds', 'expectedTestsPerProject', 'fixtureMode', 'id', 'requiresBrowser',
    'role', 'runner', 'sourceOnlyAdapter', 'sourceOnlyCase', 'upstreamSourceSetSha256',
  ].sort());
  if (suite.id !== SUITE || suite.role !== 'implementation-internal-source-only' ||
      suite.runner !== RUNNER || suite.config !== CONFIG ||
      suite.sourceOnlyAdapter !== 'upstream-private-source-only-no-pwright-runtime' ||
      suite.requiresBrowser !== false || suite.fixtureMode !== 'pinned-private-source-only-no-io' ||
      suite.sourceOnlyCase !== CASE || suite.upstreamSourceSetSha256 !== SOURCE_SHA256 ||
      suite.expectedInvocationIdsSha256 !== IDS_SHA256 || suite.expectedTestsPerProject !== 2 ||
      suite.expectedFiles !== 1)
    throw new Error('MCP source-only suite changed');
  assert.deepStrictEqual(suite.args, ARGS);
  assert.deepStrictEqual(suite.expectedProjectIds, PROJECTS);
  assert.deepStrictEqual(suite.expectedInvocations, INVOCATIONS.map(([line, projectId, specId]) => ({ line, projectId, specId })));
  if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE ||
      process.env.CI || process.env.GITHUB_ACTIONS || process.env.GITLAB_CI ||
      (process.env.JENKINS_URL && process.env.BUILD_URL) || fs.existsSync(path.join(upstream, '.env')))
    throw new Error('MCP source-only lane forbids candidate runtime and environment overrides');

  const pairs = SOURCE_FILES.map(relative => {
    const file = path.join(upstream, relative);
    if (!fs.lstatSync(file).isFile())
      throw new Error(`MCP source-only input is not a regular file: ${relative}`);
    return [relative, sha256(file)];
  });
  const actual = crypto.createHash('sha256').update(JSON.stringify(pairs)).digest('hex');
  if (actual !== SOURCE_SHA256)
    throw new Error('MCP source-only pinned source SHA-256 mismatch');

  const ts = require(path.join(upstream, 'node_modules/typescript'));
  const spec = path.join(upstream, 'tests/mcp/config.spec.ts');
  const tree = ts.createSourceFile(spec, fs.readFileSync(spec, 'utf8'), ts.ScriptTarget.Latest, true);
  const expectedTitles = new Map([
    [82, 'should enable sandbox by default (no --no-sandbox flag)'],
    [87, 'should disable sandbox when --no-sandbox flag is passed'],
  ]);
  const found = new Set();
  let privateImport = false;
  for (const statement of tree.statements) {
    if (ts.isImportDeclaration(statement) && statement.moduleSpecifier.text ===
        '../../packages/playwright/lib/mcp/browser/config' &&
        ts.isNamedImports(statement.importClause?.namedBindings))
      privateImport = statement.importClause.namedBindings.elements.some(element =>
        element.name.text === 'configFromCLIOptions' && !element.isTypeOnly);
  }
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'test') {
      const line = tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
      if (expectedTitles.has(line)) {
        const callback = node.arguments.find(argument => ts.isArrowFunction(argument) || ts.isFunctionExpression(argument));
        if (found.has(line) || node.arguments[0]?.text !== expectedTitles.get(line) ||
            !callback || callback.parameters.length !== 0 || !ts.isBlock(callback.body))
          throw new Error('MCP source-only selection is no longer a unique zero-fixture test');
        found.add(line);
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(tree);
  if (!privateImport || found.size !== 2)
    throw new Error('MCP source-only private import or test declarations changed');
  return {
    case: CASE, sourceFiles: SOURCE_FILES, sourceSha256: actual,
    expectedInvocationIdsSha256: IDS_SHA256, expectedProjectIds: PROJECTS,
    expectedTestsPerProject: 2, expectedInvocations: suite.expectedInvocations,
  };
}

function activate() {
  const installed = Symbol.for('pwright.coreMcpConfigSourceOnlyV1GuardInstalled');
  if (globalThis[installed]) return;
  const manifest = process.env.PWRIGHT_CORE_SOURCE_ONLY_MANIFEST;
  const suite = process.env.PWRIGHT_CORE_SOURCE_ONLY_SUITE;
  const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
  const receipt = process.env.PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT;
  if (!manifest || !suite || !upstream || !receipt)
    throw new Error('MCP source-only guard environment is incomplete');
  const source = verifySuite(manifest, suite, upstream);
  const isWorker = !workerThreads.isMainThread || typeof process.send === 'function';
  const entrypoint = path.join(upstream, isWorker ? WORKER : RUNNER);
  if (process.argv[1] && path.resolve(process.argv[1]) !== entrypoint)
    throw new Error(`MCP source-only unapproved Node entrypoint: ${process.argv[1]}`);

  // Reuse the already audited no-network/no-process hooks without activating
  // the older suite selector. Its exact bytes are pinned above.
  const active = process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE;
  delete process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE;
  let shared;
  try {
    shared = require(SHARED_GUARD);
  } finally {
    process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE = active;
  }
  shared.installNoIoGuard({ allowRunnerFork: !isWorker, workerScript: path.join(upstream, WORKER) });
  globalThis[installed] = true;
  fs.appendFileSync(receipt, JSON.stringify({
    kind: !workerThreads.isMainThread ? 'loader-thread-no-io' : isWorker ? 'worker-no-io' : 'runner-no-io',
    pid: process.pid, threadId: workerThreads.threadId, suite, case: source.case,
  }) + '\n', { flag: 'a' });
}

if (require.main === module) {
  if (process.argv[2] !== '--check' || process.argv.length !== 6)
    throw new Error('usage: MCP guard --check MANIFEST SUITE UPSTREAM');
  process.stdout.write(JSON.stringify(verifySuite(process.argv[3], process.argv[4], process.argv[5])) + '\n');
} else if (process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE === '1') {
  activate();
}

module.exports = { verifySuite };
