'use strict';

// Keep the selected upstream test source untouched. The pinned shared preload
// substitutes only RunServer's process command in remoteServer.ts.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const mode = process.env.PWRIGHT_CORE_CHILD_CLOSE_SYNTHETIC_MODE;
const receipt = process.env.PWRIGHT_CORE_CHILD_CLOSE_SYNTHETIC_RECEIPT;
const server = process.env.PWRIGHT_CORE_CHILD_CLOSE_SYNTHETIC_SERVER;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
if (!['oracle', 'candidate'].includes(mode) || !receipt || !server || !upstream)
  throw new Error('child-close fixture requires its exact mode, receipt, server, and upstream');
for (const [key, value] of [
  ['PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_MODE', mode],
  ['PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_RECEIPT', receipt],
  ['PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_SERVER', server],
]) {
  if (process.env[key] && process.env[key] !== value)
    throw new Error(`child-close fixture forbids conflicting ${key}`);
}

const source = fs.readFileSync(path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts'), 'utf8');
const digest = crypto.createHash('sha256').update(source).digest('hex');
if (digest !== '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc')
  throw new Error('pinned upstream child-close test source changed');
const selected = `    test('should emit close events on pages and contexts', async ({ connect, startRemoteServer }) => {
      const remoteServer = await startRemoteServer(kind);
      const browser = await connect(remoteServer.wsEndpoint());
      const context = await browser.newContext();
      const page = await context.newPage();
      let pageClosed = false;
      page.on('close', () => pageClosed = true);
      await Promise.all([
        new Promise(f => context.on('close', f)),
        remoteServer.close()
      ]);
      expect(pageClosed).toBeTruthy();
    });`;
if (!source.includes(selected) || source.split(selected).length !== 2)
  throw new Error('selected upstream child-close assertion changed or duplicated');

process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_MODE = mode;
process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_RECEIPT = receipt;
process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_SERVER = server;
require('./playwright-core-server-connect-lifecycle-synthetic-v1-preload.cjs');
fs.appendFileSync(receipt, JSON.stringify({
  kind: 'selected-child-close-source', mode, pid: process.pid,
  workerIndex: process.env.TEST_WORKER_INDEX ?? null,
  upstreamSourceSha256: digest,
  selectedBodySha256: crypto.createHash('sha256').update(selected).digest('hex'),
  testSourceAdapted: false,
}) + '\n');
