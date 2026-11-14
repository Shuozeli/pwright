'use strict';

// Exact-source instrumentation for the unchanged upstream :64 test. This
// wrapper records public launch calls and temporary-profile ownership; it does
// not replace the launch implementation or change an assertion.
const crypto = require('node:crypto');
const fs = require('node:fs');
const Module = require('node:module');
const path = require('node:path');

const mode = process.env.PWRIGHT_OTHER_DUMMY_MODE;
const engine = process.env.PWRIGHT_OTHER_DUMMY_ENGINE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const manifestPath = process.env.PWRIGHT_OTHER_DUMMY_MANIFEST;
const manifestSha256 = process.env.PWRIGHT_OTHER_DUMMY_MANIFEST_SHA256;
const launchReceipt = process.env.PWRIGHT_OTHER_DUMMY_LAUNCH_RECEIPT;
const profileReceipt = process.env.PWRIGHT_OTHER_DUMMY_PROFILE_RECEIPT;
const runId = process.env.PWRIGHT_OTHER_DUMMY_RUN_ID;
const candidate = process.env.PWRIGHT_NODE_CANDIDATE;
const candidateProfile = process.env.PWRIGHT_NATIVE_PROFILE;
const assetRelative = 'tests/assets/dummy_bad_browser_executable.js';
const assetSha256 = '9928a78e7de42f29dfc61783a2f31673b387d7d6ea27cda5bc4472e4338c1f7f';

function digest(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

if (!['candidate', 'oracle'].includes(mode) || !['firefox', 'webkit'].includes(engine) ||
    !upstream || !path.isAbsolute(upstream) || !manifestPath || !path.isAbsolute(manifestPath) ||
    !launchReceipt || !path.isAbsolute(launchReceipt) ||
    !profileReceipt || !path.isAbsolute(profileReceipt) ||
    !/^[0-9a-f]{32}$/.test(runId || '') || digest(manifestPath) !== manifestSha256 ||
    process.env.PWTEST_MODE !== 'default' || process.env.PWTEST_CHANNEL ||
    process.env.PWTEST_VIDEO || process.env.PWTEST_TRACE ||
    process.env.PWRIGHT_CANDIDATE_CDP_ENDPOINT || process.env.PWRIGHT_ORACLE_CDP_ENDPOINT ||
    process.env.PWRIGHT_CDP_ENDPOINT || process.env.PWRIGHT_CDP_WS_URL || process.env.CRPATH)
  throw new Error('other-engine dummy launch environment differs from strict policy');

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
const selectedCase = manifest.cases?.find(entry => entry.engine === engine);
if (manifest.schema !== 2 || manifest.executionPolicy !== 'pinned-dummy-node-other-engines-strict-v1' ||
    manifest.selector !== 'tests/library/browsertype-launch.spec.ts:64' ||
    manifest.asset !== assetRelative || !selectedCase ||
    selectedCase.project !== `${engine}-library` ||
    selectedCase.suite !== `library-browsertype-launch-dummy-node-failure-${engine}` ||
    (mode === 'candidate' && (!candidate || candidateProfile !== manifest.candidateProfile)) ||
    (mode === 'oracle' && (candidate || candidateProfile)))
  throw new Error('other-engine dummy launch manifest or runtime differs');

const asset = path.join(upstream, assetRelative);
if (fs.lstatSync(asset).isSymbolicLink() || !fs.statSync(asset).isFile() ||
    fs.realpathSync(asset) !== asset || digest(asset) !== assetSha256)
  throw new Error('other-engine dummy executable differs');

function append(receipt, event) {
  if (!fs.statSync(receipt).isFile() || fs.lstatSync(receipt).isSymbolicLink())
    throw new Error('other-engine dummy receipt is missing or linked');
  fs.appendFileSync(receipt, JSON.stringify(event) + '\n');
}

const originalMkdtemp = fs.promises.mkdtemp.bind(fs.promises);
fs.promises.mkdtemp = async function observedMkdtemp(prefix, ...args) {
  const created = await originalMkdtemp(prefix, ...args);
  if (typeof prefix === 'string' && path.basename(prefix) === `playwright_${engine}dev_profile-`) {
    const stat = fs.statSync(created);
    append(profileReceipt, {
      event: 'profile-created', mode, engine, runId,
      workerPid: process.pid, path: created,
      device: stat.dev, inode: stat.ino, uid: stat.uid,
    });
  }
  return created;
};

const testModeSource = path.join(upstream, 'tests/config/testMode.ts');
const originalLoad = Module._load;
let adapted;
Module._load = function guardedOtherEngineLoad(request, parent, isMain) {
  if (request !== 'playwright-core' || parent?.filename !== testModeSource)
    return originalLoad(request, parent, isMain);
  if (adapted) return adapted;
  const implementation = originalLoad(request, parent, isMain);
  const browserType = implementation?.[engine];
  if (typeof browserType?.launch !== 'function')
    throw new Error(`other-engine dummy launch lacks public ${engine}.launch`);
  let candidateInherited = null;
  let artifactSha256 = null;
  if (mode === 'candidate') {
    const artifact = path.join(candidate, 'native', candidateProfile, 'pwright.node');
    candidateInherited = !Object.prototype.hasOwnProperty.call(browserType, 'launch') &&
      Object.getPrototypeOf(browserType)?.launch === browserType.launch;
    if (implementation.compatibility?.nativeProfile !== candidateProfile ||
        !candidateInherited || fs.lstatSync(artifact).isSymbolicLink() ||
        !fs.statSync(artifact).isFile() || digest(artifact) !== manifest.candidateArtifactSha256)
      throw new Error('other-engine candidate launch/profile is not inherited from pinned package');
    artifactSha256 = digest(artifact);
  }
  const launch = browserType.launch.bind(browserType);
  browserType.launch = async options => {
    if (!options || typeof options !== 'object' || Array.isArray(options) ||
        Reflect.ownKeys(options).length !== 1 ||
        !Object.prototype.hasOwnProperty.call(options, 'executablePath') ||
        options.executablePath !== asset)
      throw new Error('other-engine dummy launch options differ from pinned source');
    const common = {
      mode, engine, runId, publicMethod: `${engine}.launch`,
      executablePath: asset, assetSha256, manifestSha256,
      workerPid: process.pid, workerParentPid: process.ppid,
      candidateProfile: mode === 'candidate' ? candidateProfile : null,
      candidateArtifactSha256: artifactSha256,
      candidatePublicLaunchInherited: candidateInherited,
    };
    append(launchReceipt, { event: 'launch-invoked', ...common });
    try {
      const browser = await launch(options);
      append(launchReceipt, { event: 'launch-succeeded', ...common });
      return browser;
    } catch (error) {
      append(launchReceipt, { event: 'launch-failed', ...common });
      throw error;
    }
  };
  adapted = implementation;
  return adapted;
};
