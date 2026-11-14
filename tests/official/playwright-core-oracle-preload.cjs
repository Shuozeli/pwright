'use strict';

const Module = require('node:module');
const path = require('node:path');

const endpoint = process.env.PWRIGHT_ORACLE_CDP_ENDPOINT;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
if (!endpoint) throw new Error('PWRIGHT_ORACLE_CDP_ENDPOINT is required');
if (!upstream) throw new Error('PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM is required');

const testModeSource = path.join(path.resolve(upstream), 'tests', 'config', 'testMode.ts');
const originalLoad = Module._load;
let adaptedOracle;

function oracleForOfficialFixtures(request, parent, isMain) {
  if (adaptedOracle) return adaptedOracle;
  const implementation = originalLoad(request, parent, isMain);
  if (!implementation?.chromium)
    throw new Error('the pinned Playwright oracle has no chromium export');

  const chromium = Object.create(implementation.chromium);
  chromium.launch = async () => {
    process.stderr.write(
      `[playwright-oracle] browserType.launch fixture adapter -> connectOverCDP(${endpoint})\n`,
    );
    return implementation.chromium.connectOverCDP(endpoint);
  };
  adaptedOracle = Object.assign(Object.create(implementation), implementation, {
    chromium,
  });
  return adaptedOracle;
}

Module._load = function playwrightOracleLoad(request, parent, isMain) {
  if (request === 'playwright-core' && parent?.filename === testModeSource)
    return oracleForOfficialFixtures(request, parent, isMain);
  return originalLoad(request, parent, isMain);
};
