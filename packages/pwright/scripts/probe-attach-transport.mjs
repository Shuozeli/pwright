// ABBA diagnostic on separately built release probes, not a conformance shortcut.
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { isAbsolute } from 'node:path';
import { promisify } from 'node:util';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222', 'requires dedicated Chrome/Pwright preflight');
const [off, on] = process.argv.slice(2);
assert.ok(off && on && isAbsolute(off) && isAbsolute(on), 'provide absolute off/on probe artifacts');
const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium } = require('playwright');
const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
const rows = [];
try {
  const session = await diagnosticDeadline(browser.newBrowserCDPSession(), 'browser CDP session');
  try {
    const inventory = async () => {
      const { targetInfos } = await diagnosticDeadline(session.send('Target.getTargets'), 'target inventory');
      assert.ok(targetInfos.length <= 256, 'diagnostic target capacity exceeded');
      return targetInfos.map(target => ({ id: target.targetId, type: target.type })).sort((a, b) => a.id.localeCompare(b.id));
    };
    const baseline = await inventory();
    for (const [label, executable] of [['off', off], ['on', on], ['on', on], ['off', off]]) {
      const before = await inventory();
      let stdout, stderr, status = 0;
      try {
        ({ stdout, stderr } = await promisify(execFile)(executable, ['--ignored', '--nocapture'], {
          env: process.env, timeout: 15000, maxBuffer: 256 * 1024, killSignal: 'SIGKILL',
        }));
      } catch (error) {
        stdout = error.stdout; stderr = error.stderr; status = error.code;
      }
      assert.equal(typeof stdout, 'string');
      assert.equal(typeof stderr, 'string');
      const line = stdout.split('\n').find(line => line.startsWith('{"schema":"pwright.attach-command-probe.v1"'));
      assert.ok(line, 'probe must emit its bounded report, even on attach failure');
      const row = {
        tcpNodelay: label === 'on', status,
        artifactSha256: createHash('sha256').update(await readFile(executable)).digest('hex'),
        before, after: await inventory(), report: JSON.parse(line),
      };
      rows.push(row);
      console.log(JSON.stringify(row));
    }
    for (const row of rows) {
      assert.deepEqual(row.before, baseline, 'target set changed; this comparison is inconclusive');
      assert.deepEqual(row.after, baseline, 'target set changed; this comparison is inconclusive');
      assert.equal(row.report.overflow, false, 'diagnostic data overflowed');
    }
  } finally { await diagnosticDeadline(session.detach(), 'CDP detach'); }
} finally { await diagnosticDeadline(browser.close(), 'oracle cleanup'); }
// A reproduced timeout is recorded, not converted into a passing compatibility claim.
assert.ok(rows.every(row => row.status === 0), 'one or more diagnostic attachments failed');
