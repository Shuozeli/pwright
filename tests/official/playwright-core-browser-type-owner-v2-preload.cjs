'use strict';

// Official Playwright fixtures inspect BrowserType._playwright before calling
// the public executablePath() API. Supply only that client-object backpointer
// in this versioned test adapter; browser path policy stays in the candidate.
const path = require('node:path');
const fs = require('node:fs');

const candidate = process.env.PWRIGHT_NODE_CANDIDATE;
if (!candidate)
  throw new Error('PWRIGHT_NODE_CANDIDATE is required');

const implementation = require(path.resolve(candidate));
for (const name of ['chromium', 'firefox', 'webkit']) {
  const browserType = implementation[name];
  if (!browserType || typeof browserType.executablePath !== 'function')
    throw new Error(`Pwright candidate is missing ${name}.executablePath`);
  if (Object.hasOwn(browserType, '_playwright'))
    throw new Error(`Pwright candidate already owns ${name}._playwright`);
  Object.defineProperty(browserType, '_playwright', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: implementation,
  });
}

const receipt = process.env.PWRIGHT_CORE_BROWSER_TYPE_OWNER_RECEIPT;
if (receipt) {
  fs.appendFileSync(receipt, JSON.stringify({
    pid: process.pid,
    workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    candidate: path.resolve(candidate),
    ownerBound: ['chromium', 'firefox', 'webkit'].every(
      name => implementation[name]._playwright === implementation),
  }) + '\n');
}
