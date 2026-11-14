import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const candidate = require('../src/index.cjs');
const { nativeArtifact } = require('../src/native-artifact.cjs');
const selected = nativeArtifact.profile === 'persistent-context-preflight';
const api = 'browserType.launchPersistentContext';

const cases = [
  {
    name: 'Chromium boxed profile argument takes precedence over a boxed page argument',
    engine: 'chromium',
    options: { args: [new String('--user-data-dir=unused'), new String('https://example.invalid/')] },
    reason: `Pass userDataDir parameter to '${api}(userDataDir, options)' instead of specifying '--user-data-dir' argument`,
  },
  {
    name: 'Chromium boxed remote-debugging pipe takes precedence over a boxed page argument',
    engine: 'chromium',
    options: { args: [new String('--remote-debugging-pipe'), new String('https://example.invalid/')] },
    reason: 'Playwright manages remote debugging connection itself.',
  },
  {
    name: 'Chromium boxed page argument',
    engine: 'chromium',
    options: { args: [new String('https://example.invalid/')] },
    reason: 'Arguments can not specify page to be opened',
  },
  {
    name: 'WebKit boxed page argument',
    engine: 'webkit',
    options: { args: [new String('https://example.invalid/')] },
    reason: 'Arguments can not specify page to be opened',
  },
  {
    name: 'Firefox boxed profile argument',
    engine: 'firefox',
    options: { args: [new String('--profile=unused')] },
    reason: `Pass userDataDir parameter to '${api}(userDataDir, options)' instead of specifying '--profile' argument`,
  },
  {
    name: 'BiDi Firefox boxed channel and page argument',
    engine: 'firefox',
    options: { channel: new String('moz-firefox'), args: [new String('https://example.invalid/')] },
    reason: 'Arguments can not specify page to be opened',
  },
  {
    name: 'ignoreDefaultArgs array still validates a boxed page argument',
    engine: 'chromium',
    options: { ignoreDefaultArgs: [], args: [new String('https://example.invalid/')] },
    reason: 'Arguments can not specify page to be opened',
  },
];

const certificateCases = [
  {
    name: 'boxed empty certificate origin',
    engine: 'chromium',
    options: { clientCertificates: [{ origin: new String(''), cert: Buffer.from('cert'), key: Buffer.from('key') }] },
    reason: 'clientCertificates.origin is required',
  },
  {
    name: 'boxed empty certificate passphrase without material',
    engine: 'chromium',
    options: { clientCertificates: [{ origin: new String('https://example.invalid'), passphrase: new String('') }] },
    reason: 'None of cert, key, passphrase or pfx is specified',
  },
];

const firefoxJugglerCases = [
  {
    name: 'classic Firefox exact -juggler argument',
    engine: 'firefox',
    options: { args: ['-juggler'] },
    reason: 'Use the port parameter instead of -juggler argument',
  },
  {
    name: 'classic Firefox boxed -juggler-pipe argument',
    engine: 'firefox',
    options: { args: [new String('-juggler-pipe')] },
    reason: 'Use the port parameter instead of -juggler argument',
  },
  {
    name: 'classic Firefox -juggler suffix precedes an allowed page argument',
    engine: 'firefox',
    options: { args: ['https://example.invalid/', '-juggler-custom'] },
    reason: 'Use the port parameter instead of -juggler argument',
  },
  {
    name: 'classic Firefox profile argument precedes -juggler',
    engine: 'firefox',
    options: { args: [new String('-juggler-port=0'), new String('--profile=unused')] },
    reason: `Pass userDataDir parameter to '${api}(userDataDir, options)' instead of specifying '--profile' argument`,
  },
  {
    name: 'BiDi Firefox moz channel rejects a page but not -juggler',
    engine: 'firefox',
    options: { channel: new String('moz-firefox'), args: [new String('-juggler-pipe'), new String('https://example.invalid/')] },
    reason: 'Arguments can not specify page to be opened',
  },
  {
    name: 'classic Firefox ignoreDefaultArgs array still checks -juggler',
    engine: 'firefox',
    options: { ignoreDefaultArgs: [], args: [new String('-juggler-pipe')] },
    reason: 'Use the port parameter instead of -juggler argument',
  },
  {
    name: 'certificate origin precedes classic Firefox -juggler',
    engine: 'firefox',
    options: { clientCertificates: [{ origin: '', cert: Buffer.from('cert'), key: Buffer.from('key') }], args: [new String('-juggler-pipe')] },
    reason: 'clientCertificates.origin is required',
  },
];

for (const { name, engine, options, reason } of [...cases, ...certificateCases]) {
  test(`${name} rejects through the public adapter before native acquisition`, async () => {
    // Arrange
    const browserType = candidate[engine];

    // Act
    const launch = browserType.launchPersistentContext('unused-profile', options);

    // Assert
    await assert.rejects(launch, error => error.constructor === Error
      && error.message === `${api}: ${reason}`);
  });
}

for (const { name, engine, options, reason } of firefoxJugglerCases) {
  test(`${name} matches the opt-in Rust-backed public preflight`, { skip: !selected }, async () => {
    // Arrange
    const browserType = candidate[engine];

    // Act
    const launch = browserType.launchPersistentContext('unused-profile', options);

    // Assert
    await assert.rejects(launch, error => error.constructor === Error
      && error.message === `${api}: ${reason}`);
  });
}

test('default JS profile keeps its existing Firefox -juggler boundary', { skip: selected }, async () => {
  // Arrange
  const options = { args: ['-juggler-pipe'] };

  // Act
  const launch = candidate.firefox.launchPersistentContext('unused-profile', options);

  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('moz Firefox and ignored default arguments do not gain a juggler rejection', { skip: !selected }, async () => {
  // One independent Arrange/Act/Assert case for each permitted preflight.
  for (const options of [
    { channel: 'moz-firefox', args: ['-juggler-pipe'] },
    { ignoreDefaultArgs: true, args: ['-juggler-pipe'] },
    { args: ['--juggler-pipe'] },
  ]) {
    // Arrange
    const browserType = candidate.firefox;

    // Act
    const launch = browserType.launchPersistentContext('unused-profile', options);

    // Assert
    await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
  }
});

test('ignoreDefaultArgs true does not turn boxed page arguments into a preflight rejection', async () => {
  // Arrange
  const options = { ignoreDefaultArgs: true, args: [new String('https://example.invalid/')] };

  // Act
  const launch = candidate.chromium.launchPersistentContext('unused-profile', options);

  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('pinned Playwright Core public API agrees on boxed-string pre-spawn reasons', {
  skip: !process.env.PWRIGHT_PINNED_PLAYWRIGHT_CORE,
}, async () => {
  // Arrange
  const root = process.env.PWRIGHT_PINNED_PLAYWRIGHT_CORE;
  const metadata = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));
  assert.equal(metadata.name, 'playwright-core');
  assert.equal(metadata.version, '1.58.0');
  assert.equal(
    execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    '961381ec73a9fa2661d92504eab1a6bd6eaffc75',
  );
  const scratch = await mkdtemp(join(tmpdir(), 'pwright-boxed-preflight-'));
  const previousTmpdir = process.env.TMPDIR;
  const previousBrowsersPath = process.env.PLAYWRIGHT_BROWSERS_PATH;

  try {
    // Keep even unexpected launch paths browserless: the explicit executable
    // cannot exist, and the registry points at this test-owned empty folder.
    process.env.TMPDIR = scratch;
    process.env.PLAYWRIGHT_BROWSERS_PATH = join(scratch, 'no-browsers');
    const upstream = require(join(root, 'index.js'));

    const compared = [...cases, ...certificateCases, ...(selected ? firefoxJugglerCases : [])];
    for (const [index, { name, engine, options, reason }] of compared.entries()) {
      const guardedOptions = { ...options, executablePath: join(scratch, 'no-browser-executable') };
      const userDataDir = join(scratch, `profile-${index}`);

      // Act
      const upstreamError = await upstream[engine].launchPersistentContext(userDataDir, guardedOptions).catch(error => error);
      const candidateError = await candidate[engine].launchPersistentContext(userDataDir, guardedOptions).catch(error => error);

      // Assert
      assert.ok(upstreamError instanceof Error, `${name}: upstream must reject`);
      assert.ok(candidateError instanceof Error, `${name}: candidate must reject`);
      assert.ok(upstreamError.message.includes(reason), `${name}: upstream error was ${upstreamError.message}`);
      assert.equal(candidateError.message, `${api}: ${reason}`, name);
    }
  } finally {
    if (previousTmpdir === undefined) delete process.env.TMPDIR;
    else process.env.TMPDIR = previousTmpdir;
    if (previousBrowsersPath === undefined) delete process.env.PLAYWRIGHT_BROWSERS_PATH;
    else process.env.PLAYWRIGHT_BROWSERS_PATH = previousBrowsersPath;
    await rm(scratch, { recursive: true, force: true });
  }
});

test('pinned Core and opt-in Rust both pass through BiDi and ignoreDefaultArgs juggler preflight', {
  skip: !selected || !process.env.PWRIGHT_PINNED_PLAYWRIGHT_CORE,
}, async () => {
  // Arrange
  const root = process.env.PWRIGHT_PINNED_PLAYWRIGHT_CORE;
  const metadata = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));
  assert.equal(metadata.name, 'playwright-core');
  assert.equal(metadata.version, '1.58.0');
  assert.equal(
    execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    '961381ec73a9fa2661d92504eab1a6bd6eaffc75',
  );
  const scratch = await mkdtemp(join(tmpdir(), 'pwright-firefox-juggler-preflight-'));
  const previousTmpdir = process.env.TMPDIR;
  const previousBrowsersPath = process.env.PLAYWRIGHT_BROWSERS_PATH;

  try {
    process.env.TMPDIR = scratch;
    process.env.PLAYWRIGHT_BROWSERS_PATH = join(scratch, 'no-browsers');
    const upstream = require(join(root, 'index.js'));
    const acceptedCases = [
      { name: 'BiDi moz channel', options: { channel: 'moz-firefox', args: ['-juggler-pipe'] } },
      { name: 'ignored default arguments', options: { ignoreDefaultArgs: true, args: ['-juggler-pipe'] } },
      { name: 'double-dash juggler spelling', options: { args: ['--juggler-pipe'] } },
    ];

    for (const [index, { name, options }] of acceptedCases.entries()) {
      // Arrange
      const executablePath = join(scratch, 'no-browser-executable');
      const guardedOptions = { ...options, executablePath };
      const userDataDir = join(scratch, `profile-${index}`);

      // Act
      const upstreamError = await upstream.firefox.launchPersistentContext(userDataDir, guardedOptions).catch(error => error);
      const candidateError = await candidate.firefox.launchPersistentContext(userDataDir, guardedOptions).catch(error => error);

      // Assert
      assert.ok(upstreamError instanceof Error, `${name}: upstream must reject the missing executable`);
      assert.ok(candidateError instanceof Error, `${name}: candidate must stop at unsupported launch`);
      assert.ok(upstreamError.message.includes(`executable doesn't exist at ${executablePath}`),
        `${name}: upstream error was ${upstreamError.message}`);
      assert.equal(candidateError.code, 'PWRIGHT_UNSUPPORTED', name);
    }
  } finally {
    if (previousTmpdir === undefined) delete process.env.TMPDIR;
    else process.env.TMPDIR = previousTmpdir;
    if (previousBrowsersPath === undefined) delete process.env.PLAYWRIGHT_BROWSERS_PATH;
    else process.env.PLAYWRIGHT_BROWSERS_PATH = previousBrowsersPath;
    await rm(scratch, { recursive: true, force: true });
  }
});
