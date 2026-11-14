'use strict';

// Opt-in official-test adapter. It wraps the real public launch method; the
// historical candidate/oracle CDP adapters remain separate and unchanged.
const fs = require('node:fs');
const Module = require('node:module');
const path = require('node:path');

const mode = process.env.PWRIGHT_OFFICIAL_OWNED_LAUNCH_MODE;
const executable = process.env.CRPATH;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_OFFICIAL_OWNED_LAUNCH_RECEIPT;
if (!['candidate', 'oracle'].includes(mode) || !executable || !path.isAbsolute(executable) || !upstream || !receipt)
  throw new Error('owned Core launch needs mode, absolute CRPATH, upstream and receipt');
if (process.env.PWRIGHT_CANDIDATE_CDP_ENDPOINT || process.env.PWRIGHT_ORACLE_CDP_ENDPOINT)
  throw new Error('owned Core launch forbids the historical CDP adapter endpoints');

const testModeSource = path.join(path.resolve(upstream), 'tests', 'config', 'testMode.ts');
const originalLoad = Module._load;
let adapted;

function record(event, options) {
  fs.appendFileSync(receipt, JSON.stringify({
    event,
    mode,
    publicMethod: 'chromium.launch',
    executablePath: options.executablePath,
    headless: options.headless,
  }) + '\n');
}

Module._load = function pwrightOwnedLaunchLoad(request, parent, isMain) {
  if (request !== 'playwright-core' || parent?.filename !== testModeSource)
    return originalLoad(request, parent, isMain);
  if (adapted) return adapted;
  const implementation = originalLoad(request, parent, isMain);
  if (!implementation?.chromium?.launch)
    throw new Error('owned Core launch requires a public chromium.launch method');
  const launch = implementation.chromium.launch.bind(implementation.chromium);
  implementation.chromium.launch = async options => {
    const defaults = implementation._defaultLaunchOptions;
    if (!defaults || defaults.executablePath !== executable || typeof defaults.headless !== 'boolean')
      throw new Error('owned Core launch fixture defaults are absent or differ from CRPATH');
    if (options && Object.keys(options).length)
      throw new Error('owned Core launch one-spec lane forbids explicit launch options');
    // The candidate supports only this narrow public option subset. The
    // official oracle still merges its own complete fixture defaults.
    const selected = { executablePath: defaults.executablePath, headless: defaults.headless };
    if (defaults.timeout !== undefined) selected.timeout = defaults.timeout;
    record('launch-invoked', selected);
    try {
      const browser = await launch(mode === 'candidate' ? selected : options);
      record('launch-succeeded', selected);
      return browser;
    } catch (error) {
      record('launch-failed', selected);
      throw error;
    }
  };
  adapted = implementation;
  return adapted;
};
