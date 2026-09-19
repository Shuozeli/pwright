// Pinned public API characterization; no candidate implementation is implied.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { writeFile } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { withCdpCommandRelay } from '../test/cdp-command-relay.mjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium } = require('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const directory = process.env.PWRIGHT_FILE_INPUT_FIXTURES;
const output = process.env.PWRIGHT_FILE_INPUT_REPORT;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(directory && isAbsolute(directory));
assert.ok(output && isAbsolute(output));
const path = name => join(directory, name);
const payload = (name = 'buffer.txt', mimeType = 'text/plain') => ({ name, mimeType, buffer: Buffer.from('memory file') });
const cases = [
  { name: 'one caller path uses payload transfer', files: () => path('input.txt'), native: 0, count: 1 },
  { name: 'multiple caller paths', selector: '#multiple', files: () => [path('input.txt'), path('second.txt')], native: 0, count: 2 },
  { name: 'single memory payload', files: () => payload(), native: 0, count: 1 },
  { name: 'multiple memory payloads', selector: '#multiple', files: () => [payload(), payload('second.txt')], native: 0, count: 2 },
  { name: 'binary memory payload', files: () => ({ name: 'bytes.bin', mimeType: 'application/octet-stream', buffer: Buffer.from([0, 255, 1]) }), native: 0, count: 1, bytes: [0, 255, 1] },
  { name: 'empty array clears input', files: () => [], native: 0, count: 0 },
  { name: 'label retargets to file input', selector: '#label', files: () => payload(), native: 0, count: 1 },
  { name: 'hidden disabled input is not a pointer action', selector: '#hidden', files: () => payload(), native: 0, count: 1 },
  { name: 'empty MIME uses filename lookup', files: () => payload('guess.txt', ''), native: 0, count: 1, mime: 'text/plain' },
  { name: 'native directory traversal', selector: '#directory', files: () => path('tree'), native: 1, count: 2, directory: true },
  { name: 'multiple files rejected on single input', files: () => [payload(), payload()], error: /Non-multiple file input can only accept single file/ },
  { name: 'directory rejected on regular input', files: () => path('tree'), error: /File input does not support directories/ },
  { name: 'directory input rejects regular paths', selector: '#directory', files: () => path('input.txt'), error: /\[webkitdirectory\] input requires passing a path to a directory/ },
  { name: 'directory input rejects empty clearing array', selector: '#directory', files: () => [], error: /\[webkitdirectory\] input requires passing a path to a directory/ },
  { name: 'multiple directories rejected', selector: '#directory', files: () => [path('tree'), path('tree')], error: /Multiple directories are not supported/ },
  { name: 'file and directory mixing rejected', selector: '#directory', files: () => [path('input.txt'), path('tree')], error: /File paths must be all files or a single directory/ },
  { name: 'path and payload mixing rejected', files: () => [path('input.txt'), payload()], error: /File paths cannot be mixed with buffers/ },
  { name: 'non-input element rejected', selector: '#not-input', files: () => payload(), error: /Node is not an HTMLInputElement/ },
  { name: 'missing path fails before missing selector', selector: '#missing', files: () => path('does-not-exist.txt'), error: /ENOENT/ },
  { name: 'exact 50 MiB payload rejected', files: () => ({ ...payload(), buffer: Buffer.alloc(50 * 1024 * 1024) }), error: /Cannot set buffer larger than 50Mb/ },
  // This branch's success/no-op behavior must be observed, not inferred from
  // the native-path branch or replaced by a stricter universal type check.
  { name: 'payload on text input is a no-op', selector: '#text', files: () => payload(), native: 0, noFiles: true },
];

const report = { schema: 'pwright.file-input-oracle.v1', oracle: 'playwright@1.58.2', cases: [] };
let failure;
const native = [];
const gate = {
  command(raw, _client, upstream) {
    const value = JSON.parse(raw.toString());
    if (value.method === 'DOM.setFileInputFiles') {
      assert.ok(native.length < 8);
      assert.ok(value.params.files.length <= 8);
      assert.ok(value.params.files.every(p => typeof p === 'string' && p.length <= 4096));
      native.push({ files: value.params.files, objectId: value.params.objectId });
    }
    upstream.send(raw, { binary: false });
  },
  response(raw, client) { client.send(raw, { binary: false }); },
  fail(error) { failure ??= error; },
  check() { if (failure) throw failure; },
  release() {},
};
const html = `<label id="label" for="single">Upload</label><input id="single" type="file">
<input id="multiple" type="file" multiple><input id="directory" type="file" webkitdirectory>
<input id="hidden" type="file" disabled style="display:none"><input id="text" type="text"><div id="not-input"></div>
<script>window.fileEvents=[];for(const type of ['input','change'])document.addEventListener(type,e=>window.fileEvents.push({type:e.type,composed:e.composed,bubbles:e.bubbles,trusted:e.isTrusted}));</script>`;

try {
  await withCdpCommandRelay(endpoint, gate, async relay => {
    const browser = await chromium.connectOverCDP(relay, { timeout: 5000 });
    let context;
    try {
      context = await browser.newContext();
      const page = await context.newPage();
      for (const sample of cases) {
        await page.setContent(html);
        const before = native.length;
        const selector = sample.selector ?? '#single';
        const row = { name: sample.name };
        report.cases.push(row);
        try { await page.locator(selector).setInputFiles(sample.files(), { timeout: 2000 }); }
        catch (error) { row.error = { name: error.name, message: String(error.message).slice(0, 4096) }; }
        row.nativeFileCommands = native.length - before;
        if (sample.error) {
          assert.match(row.error?.message ?? '', sample.error);
          assert.equal(row.nativeFileCommands, 0);
        } else {
          assert.equal(row.error, undefined);
          const actualSelector = selector === '#label' ? '#single' : selector;
          row.observation = await page.locator(actualSelector).evaluate(async input => {
            const files = input.files === null ? null : await Promise.all([...input.files].map(async f => {
              if (f.size > 4096) throw Error('diagnostic file exceeds bounded fixture size');
              return { name: f.name, type: f.type, size: f.size, relative: f.webkitRelativePath,
                bytes: [...new Uint8Array(await f.arrayBuffer())] };
            }));
            return { files, events: window.fileEvents };
          });
          assert.equal(row.nativeFileCommands, sample.native);
          if (sample.noFiles) assert.equal(row.observation.files, null);
          else {
            assert.equal(row.observation.files.length, sample.count);
            if (sample.bytes) assert.deepEqual(row.observation.files[0].bytes, sample.bytes);
            if (sample.mime) assert.equal(row.observation.files[0].type, sample.mime);
            if (sample.directory) assert.deepEqual(row.observation.files.map(f => f.relative).sort(), ['tree/first.txt', 'tree/nested/second.txt']);
            assert.deepEqual(row.observation.events.map(e => e.type), ['input', 'change']);
          }
        }
        row.passed = true;
        process.stdout.write(`${JSON.stringify({ name: row.name, passed: true, nativeFileCommands: row.nativeFileCommands })}\n`);
      }
    } finally {
      try { if (context) await diagnosticDeadline(context.close(), 'oracle upload Context cleanup'); }
      finally { await diagnosticDeadline(browser.close(), 'oracle upload Browser cleanup'); }
    }
  });
} catch (error) { report.failure = String(error?.message ?? error).slice(0, 4096); process.exitCode = 1; }
finally {
  report.nativeFileCommands = native;
  const text = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(text) <= 256 * 1024);
  await writeFile(output, text, { flag: 'wx' });
}
