'use strict';

// Audit the unchanged public BrowserType.launch call. The selected executable
// is Playwright's pinned exit-1 Node asset, never a browser binary.
const crypto = require('node:crypto');
const fs = require('node:fs');
const Module = require('node:module');
const path = require('node:path');

const mode = process.env.PWRIGHT_BIDI_DUMMY_MODE;
const project = process.env.PWRIGHT_BIDI_DUMMY_PROJECT;
const engine = process.env.PWRIGHT_BIDI_DUMMY_ENGINE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const manifestPath = process.env.PWRIGHT_BIDI_DUMMY_MANIFEST;
const manifestSha256 = process.env.PWRIGHT_BIDI_DUMMY_MANIFEST_SHA256;
const runId = process.env.PWRIGHT_BIDI_DUMMY_RUN_ID;
const launchReceipt = process.env.PWRIGHT_BIDI_DUMMY_LAUNCH_RECEIPT;
const profileReceipt = process.env.PWRIGHT_BIDI_DUMMY_PROFILE_RECEIPT;
const candidate = process.env.PWRIGHT_NODE_CANDIDATE;
const candidateProfile = process.env.PWRIGHT_NATIVE_PROFILE;
const assetRelative = 'tests/assets/dummy_bad_browser_executable.js';
const assetSha256 = '9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f';

function digest(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function append(file, event) {
  if (!path.isAbsolute(file) || fs.lstatSync(file).isSymbolicLink() || !fs.statSync(file).isFile())
    throw new Error('BiDi dummy receipt is missing, linked or not regular');
  fs.appendFileSync(file, JSON.stringify(event) + '\n');
}

if (!['candidate', 'oracle'].includes(mode) || !['chromium', 'firefox'].includes(engine) ||
    !project || !upstream || !path.isAbsolute(upstream) ||
    !manifestPath || !path.isAbsolute(manifestPath) || !/^[0-9a-f]{64}$/.test(manifestSha256 || '') ||
    !/^[0-9a-f]{32}$/.test(runId || '') || !launchReceipt || !profileReceipt ||
    process.env.PWTEST_MODE !== 'default' || process.env.PWTEST_CHANNEL ||
    process.env.PWTEST_VIDEO || process.env.PWTEST_TRACE || process.env.PWTEST_USE_BIDI_EXPECTATIONS ||
    process.env.PWRIGHT_CANDIDATE_CDP_ENDPOINT || process.env.PWRIGHT_ORACLE_CDP_ENDPOINT ||
    process.env.PWRIGHT_CDP_ENDPOINT || process.env.PWRIGHT_CDP_WS_URL ||
    process.env.BIDI_CRPATH || process.env.BIDI_FFPATH || process.env.NODE_PATH ||
    process.env.PLAYWRIGHT_SERVICE_OS || process.env.PWTEST_FIREFOX_USER_PREFS ||
    digest(manifestPath) !== manifestSha256)
  throw new Error('BiDi dummy launch environment or manifest differs');

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
const selected = manifest.cases?.find(item => item.project === project);
if (manifest.schema !== 1 ||
    manifest.executionPolicy !== 'pinned-bidi-dummy-node-public-launch-strict-v1' ||
    manifest.selector !== 'tests/library/browsertype-launch.spec.ts:64' ||
    manifest.asset !== assetRelative ||
    manifest.config !== 'tests/bidi/playwright.config.ts' ||
    !selected || selected.engine !== engine ||
    (mode === 'candidate' && (!candidate || candidateProfile !== manifest.candidateProfile)) ||
    (mode === 'oracle' && (candidate || candidateProfile)))
  throw new Error('BiDi dummy launch case/profile differs');

const asset = path.join(upstream, assetRelative);
if (fs.lstatSync(asset).isSymbolicLink() || !fs.statSync(asset).isFile() ||
    fs.realpathSync(asset) !== asset || digest(asset) !== assetSha256)
  throw new Error('BiDi dummy asset differs from pinned source');

const originalMkdtemp = fs.promises.mkdtemp.bind(fs.promises);
fs.promises.mkdtemp = async function observedMkdtemp(prefix, ...args) {
  const created = await originalMkdtemp(prefix, ...args);
  if (typeof prefix === 'string' && path.basename(prefix) === `playwright_${engine}dev_profile-`) {
    const stat = fs.statSync(created);
    append(profileReceipt, {
      event: 'profile-created', mode, project, engine, runId,
      workerPid: process.pid, path: created,
      device: stat.dev, inode: stat.ino, uid: stat.uid,
    });
  }
  return created;
};

const testModeSource = path.join(upstream, 'tests/config/testMode.ts');
const originalLoad = Module._load;
let adapted;
Module._load = function bidiDummyLaunchLoad(request, parent, isMain) {
  if (request !== 'playwright-core' || parent?.filename !== testModeSource)
    return originalLoad(request, parent, isMain);
  if (adapted)
    return adapted;
  const implementation = originalLoad(request, parent, isMain);
  const browserType = implementation?.[engine];
  if (typeof browserType?.launch !== 'function')
    throw new Error(`BiDi dummy capture lacks public ${engine}.launch`);
  let inherited = null;
  let artifactSha256 = null;
  if (mode === 'candidate') {
    const artifact = path.join(candidate, 'native', candidateProfile, 'pwright.node');
    inherited = !Object.prototype.hasOwnProperty.call(browserType, 'launch') &&
      Object.getPrototypeOf(browserType)?.launch === browserType.launch;
    if (implementation.compatibility?.nativeProfile !== candidateProfile ||
        !inherited || fs.lstatSync(artifact).isSymbolicLink() ||
        !fs.statSync(artifact).isFile() ||
        digest(artifact) !== manifest.candidateArtifactSha256)
      throw new Error('BiDi dummy public candidate launch is not inherited from pinned package');
    artifactSha256 = digest(artifact);
  }
  const launch = browserType.launch.bind(browserType);
  browserType.launch = async options => {
    if (!options || typeof options !== 'object' || Array.isArray(options) ||
        Reflect.ownKeys(options).length !== 1 ||
        !Object.prototype.hasOwnProperty.call(options, 'executablePath') ||
        options.executablePath !== asset)
      throw new Error('BiDi dummy public launch options differ from official test');
    const event = {
      mode, project, engine, runId, publicMethod: `${engine}.launch`,
      executablePath: asset, assetSha256, manifestSha256,
      workerPid: process.pid, workerParentPid: process.ppid,
      candidateProfile: mode === 'candidate' ? candidateProfile : null,
      candidateArtifactSha256: artifactSha256,
      candidatePublicLaunchInherited: inherited,
    };
    append(launchReceipt, { event: 'launch-invoked', ...event });
    try {
      const browser = await launch(options);
      append(launchReceipt, { event: 'launch-succeeded', ...event });
      return browser;
    } catch (error) {
      append(launchReceipt, { event: 'launch-failed', ...event });
      throw error;
    }
  };
  adapted = implementation;
  return adapted;
};
