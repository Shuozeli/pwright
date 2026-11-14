'use strict';

// Loaded by the dedicated schema-2 runner, never by the historical Core runner.
// This guards the public launch boundary. Strict OS ancestry is captured by
// dummy-core-guard/node and checked by the independent strict verifier.
const crypto = require('node:crypto');
const fs = require('node:fs');
const Module = require('node:module');
const path = require('node:path');

const mode = process.env.PWRIGHT_DUMMY_LAUNCH_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const manifestPath = process.env.PWRIGHT_DUMMY_LAUNCH_MANIFEST;
const manifestSha256 = process.env.PWRIGHT_DUMMY_LAUNCH_MANIFEST_SHA256;
const receipt = process.env.PWRIGHT_DUMMY_LAUNCH_RECEIPT;
const strictRunId = process.env.PWRIGHT_DUMMY_STRICT_RUN_ID;
const candidateProfile = process.env.PWRIGHT_NATIVE_PROFILE;
const candidate = process.env.PWRIGHT_NODE_CANDIDATE;
const assetRelative = 'tests/assets/dummy_bad_browser_executable.js';
const assetSha256 = '9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f';

function digest(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function assertEnvironment() {
  if (!['candidate', 'oracle'].includes(mode) || !upstream || !path.isAbsolute(upstream) ||
      !manifestPath || !path.isAbsolute(manifestPath) || !manifestSha256 ||
      !receipt || !path.isAbsolute(receipt))
    throw new Error('dummy-node Core launch needs pinned mode, upstream, manifest and receipt');
  for (const name of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
    'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL', 'CRPATH', 'NODE_PATH']) {
    if (process.env[name]) throw new Error(`dummy-node Core launch forbids ${name}`);
  }
  if (process.env.PWTEST_MODE !== 'default' || process.env.PWTEST_CHANNEL ||
      process.env.PWTEST_VIDEO || process.env.PWTEST_TRACE)
    throw new Error('dummy-node Core launch requires default headless mode without channel/video/trace');
  if (digest(manifestPath) !== manifestSha256)
    throw new Error('dummy-node Core manifest changed after wrapper preflight');
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const strict = manifest.executionPolicy === 'pinned-dummy-node-launch-strict-v1';
  if (manifest.schema !== 2 ||
      !['pinned-dummy-node-launch-v1', 'pinned-dummy-node-launch-strict-v1'].includes(manifest.executionPolicy) ||
      manifest.suite?.fixtureMode !== 'dummy-node-launch' ||
      manifest.suite?.selector !== 'tests/library/browsertype-launch.spec.ts:64' ||
      manifest.suite?.executable !== assetRelative)
    throw new Error('dummy-node Core manifest policy or selector differs');
  if (strict && (!strictRunId || !/^[0-9a-f]{32}$/.test(strictRunId) ||
      (mode === 'candidate' && (!candidate || candidateProfile !== manifest.suite.candidateProfile)) ||
      (mode === 'oracle' && (candidate || candidateProfile))))
    throw new Error('dummy-node strict mode/profile/run identity differs');
  if (fs.existsSync(path.join(upstream, '.env')))
    throw new Error('dummy-node Core launch forbids upstream .env overrides');
}

function pinnedExecutable(options) {
  if (!options || typeof options !== 'object' || Array.isArray(options) ||
      Reflect.ownKeys(options).length !== 1 ||
      !Object.prototype.hasOwnProperty.call(options, 'executablePath'))
    throw new Error('dummy-node Core launch requires only explicit executablePath');
  const executable = path.join(upstream, assetRelative);
  if (options.executablePath !== executable || !path.isAbsolute(executable))
    throw new Error('dummy-node Core launch executablePath differs from pinned asset');
  const stat = fs.lstatSync(executable);
  if (!stat.isFile() || stat.isSymbolicLink() || fs.realpathSync(executable) !== executable ||
      digest(executable) !== assetSha256)
    throw new Error('dummy-node Core launch asset is not the pinned regular file');
  fs.accessSync(executable, fs.constants.X_OK);
  return executable;
}

assertEnvironment();
const testModeSource = path.join(upstream, 'tests/config/testMode.ts');
const originalLoad = Module._load;
let adapted;

function record(event, executable, runtime) {
  const target = fs.lstatSync(receipt);
  if (!target.isFile() || target.isSymbolicLink())
    throw new Error('dummy-node Core receipt is not a regular file');
  fs.appendFileSync(receipt, JSON.stringify({
    event,
    mode,
    publicMethod: 'chromium.launch',
    executablePath: executable,
    assetSha256,
    manifestSha256,
    ...(strictRunId ? {
      runId: strictRunId,
      launcherPid: process.pid,
      launcherParentPid: process.ppid,
      runtimeProfile: runtime.profile,
      runtimeArtifactSha256: runtime.artifactSha256,
      candidatePublicLaunchInherited: runtime.inherited,
    } : {}),
  }) + '\n');
}

Module._load = function pwrightDummyNodeLaunchLoad(request, parent, isMain) {
  if (request !== 'playwright-core' || parent?.filename !== testModeSource)
    return originalLoad(request, parent, isMain);
  assertEnvironment();
  if (adapted) return adapted;
  const implementation = originalLoad(request, parent, isMain);
  if (typeof implementation?.chromium?.launch !== 'function')
    throw new Error('dummy-node Core launch requires public chromium.launch');
  const runtime = { profile: null, artifactSha256: null, inherited: null };
  if (strictRunId && mode === 'candidate') {
    const artifact = path.join(candidate, 'native', candidateProfile, 'pwright.node');
    if (implementation.compatibility?.nativeProfile !== candidateProfile ||
        Object.prototype.hasOwnProperty.call(implementation.chromium, 'launch') ||
        Object.getPrototypeOf(implementation.chromium)?.launch !== implementation.chromium.launch ||
        !fs.statSync(artifact).isFile() || fs.lstatSync(artifact).isSymbolicLink())
      throw new Error('dummy-node strict candidate launch/profile is not inherited from the selected package');
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    if (digest(artifact) !== manifest.suite.candidateArtifactSha256)
      throw new Error('dummy-node strict candidate artifact differs from the selected manifest');
    runtime.profile = candidateProfile;
    runtime.artifactSha256 = digest(artifact);
    runtime.inherited = true;
  }
  const launch = implementation.chromium.launch.bind(implementation.chromium);
  implementation.chromium.launch = async options => {
    assertEnvironment();
    const executable = pinnedExecutable(options);
    record('launch-invoked', executable, runtime);
    try {
      const browser = await launch(options);
      record('launch-succeeded', executable, runtime);
      return browser;
    } catch (error) {
      record('launch-failed', executable, runtime);
      throw error;
    }
  };
  adapted = implementation;
  return adapted;
};
