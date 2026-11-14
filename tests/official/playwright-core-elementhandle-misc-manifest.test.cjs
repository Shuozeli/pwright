'use strict';

const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const readline = require('node:readline');
const test = require('node:test');

const root = path.resolve(__dirname, '../..');
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM
  || path.join(os.homedir(), 'projects/thirdparty/playwright-v1.58.0');
const relativeSource = 'tests/page/elementhandle-misc.spec.ts';
const manifest = JSON.parse(fs.readFileSync(path.join(
  __dirname, 'playwright-core-elementhandle-misc-public-chromium-v1-manifest.json',
), 'utf8'));
const suite = manifest.suites[0];
const ledgers = [
  'playwright-core-candidate-page-wait-for-selector-1-17-20261011-v1.jsonl',
  'playwright-core-oracle-page-wait-for-selector-1-17-20261011-v1.jsonl',
].map(name => path.join(root, '.work/official-conformance/outcomes', name));

function sha256(contents) {
  return createHash('sha256').update(contents).digest('hex');
}

function selectedSpecs(report) {
  const specs = [];
  function visit(node) {
    for (const spec of node.specs || []) specs.push(spec);
    for (const child of node.suites || []) visit(child);
  }
  for (const rootSuite of report.suites || []) visit(rootSuite);
  return specs;
}

async function matchingRows(ledger) {
  const rows = [];
  const stream = readline.createInterface({ input: fs.createReadStream(ledger) });
  for await (const line of stream) {
    if (line.includes(`"sourceFile":"${relativeSource}"`) && line.includes('"projectId":"chromium-page"'))
      rows.push(JSON.parse(line));
  }
  return rows;
}

test('manifest pins the complete official public Chromium ElementHandle file', {
  skip: !fs.existsSync(path.join(upstream, relativeSource)),
}, () => {
  // Arrange
  const source = fs.readFileSync(path.join(upstream, relativeSource));
  const head = execFileSync('git', ['-C', upstream, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();

  // Act
  const report = JSON.parse(execFileSync('node', [
    path.join(upstream, suite.runner), 'test', '--list', '--reporter=json',
    `--config=${suite.config}`, `--project=${suite.project}`, ...suite.args,
  ], { cwd: upstream, encoding: 'utf8' }));
  const specs = selectedSpecs(report);
  const ids = specs.map(spec => spec.id).sort();

  // Assert
  assert.equal(manifest.schema, 1);
  assert.equal(manifest.tag, 'v1.58.0');
  assert.equal(manifest.commit, head);
  assert.equal(manifest.suites.length, 1);
  assert.equal(suite.id, 'library-elementhandle-misc-public-chromium');
  assert.equal(suite.role, 'browser-backed-official-file');
  assert.deepEqual(suite.args, [relativeSource]);
  assert.equal(suite.candidateAdapter, 'test-mode-playwright-core');
  assert.equal(suite.oracleAdapter, 'test-mode-playwright-core-cdp');
  assert.equal(suite.requiresBrowser, true);
  assert.equal(suite.expectedFiles, 1);
  assert.equal(suite.expectedTestsPerProject, 10);
  assert.equal(sha256(source), suite.upstreamTestSourceSha256);
  assert.equal((source.toString().match(/^it\('/gm) || []).length, 10);
  assert.equal(report.errors.length, 0);
  assert.equal(specs.length, 10);
  assert.equal(new Set(ids).size, 10);
  assert.ok(specs.every(spec => spec.file === 'page/elementhandle-misc.spec.ts'));
  assert.ok(specs.every(spec => spec.tests.length === 1 && spec.tests[0].projectId === 'chromium-page'));
  assert.equal(sha256(ids.map(id => `${id}\n`).join('')), suite.expectedSpecIdsSha256);
});

test('both frozen strict ledgers leave all ten public Chromium cases unrun', {
  skip: !ledgers.every(fs.existsSync),
}, async () => {
  // Arrange
  const expectedDigest = suite.expectedSpecIdsSha256;

  // Act
  const selections = await Promise.all(ledgers.map(matchingRows));

  // Assert
  for (const rows of selections) {
    const ids = rows.map(row => row.specId).sort();
    assert.equal(rows.length, 10);
    assert.equal(new Set(ids).size, 10);
    assert.equal(sha256(ids.map(id => `${id}\n`).join('')), expectedDigest);
    assert.ok(rows.every(row => row.sourceClassification === 'public-api-candidate'));
    assert.ok(rows.every(row => row.expectedStatus === 'passed'));
    assert.ok(rows.every(row => row.outcome === 'not-run' && row.attempts === 0));
  }
});
