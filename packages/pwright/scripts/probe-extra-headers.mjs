// Pinned public characterization. No production header policy lives here.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
const bind = process.env.PWRIGHT_TEST_BIND;
const host = process.env.PWRIGHT_TEST_HOST;
const output = process.env.PWRIGHT_HEADERS_ORACLE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(bind && /^100\./.test(bind));
assert.ok(host && host.endsWith('.ts.net'));
assert.ok(output && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium } = requireOracle('playwright');
let requests = [];
const server = createServer((request, response) => {
  if (request.url !== '/document') { response.writeHead(404).end(); return; }
  assert.ok(requests.length < 4);
  requests.push(Object.fromEntries(Object.entries(request.headers).filter(([name]) => name.startsWith('x-') || name === 'referer')));
  response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
  response.end('<title>headers</title>');
});
server.listen(0, bind);
await once(server, 'listening');
const origin = `http://${host}:${server.address().port}`;
const contextReferer = `${origin}/context`;
const pageReferer = `${origin}/page`;
const optionReferer = `${origin}/option`;
const cases = [
  { id: 'empty' },
  { id: 'context-create', initial: { 'X-A': 'context', 'X-B': 'context-only' } },
  { id: 'page-over-context', initial: { 'X-A': 'context', 'X-B': 'context-only' }, page: { 'x-a': 'page' } },
  { id: 'context-update-keeps-page', page: { 'X-A': 'page' }, context: { 'x-a': 'context', 'X-B': 'new' } },
  { id: 'clear-page-reveals-context', initial: { 'X-A': 'context' }, page: { 'X-A': 'page' }, clearPage: true },
  { id: 'clear-context-keeps-page', initial: { 'X-A': 'context' }, page: { 'X-B': 'page' }, context: {} },
  { id: 'context-referer', initial: { Referer: contextReferer } },
  { id: 'context-referer-option', initial: { Referer: contextReferer }, referer: optionReferer },
  { id: 'page-referer', page: { Referer: pageReferer } },
  { id: 'page-referer-same-option', page: { Referer: pageReferer }, referer: pageReferer },
  { id: 'page-referer-conflict', page: { Referer: pageReferer }, referer: optionReferer },
  { id: 'duplicate-page-referer', page: { Referer: pageReferer, referer: optionReferer } },
  { id: 'duplicate-page-referer-first-option', page: { Referer: pageReferer, referer: optionReferer }, referer: pageReferer },
  { id: 'duplicate-page-referer-last-option', page: { Referer: pageReferer, referer: optionReferer }, referer: optionReferer },
  { id: 'empty-page-referer', page: { Referer: '' }, referer: optionReferer },
  { id: 'context-referer-page-referer', initial: { Referer: contextReferer }, page: { referer: pageReferer } },
  { id: 'duplicate-page-name', page: { 'X-A': 'first', 'x-a': 'last' } },
  { id: 'unicode-name', page: { 'X-Ä': 'value' } },
  { id: 'space-name', page: { 'X A': 'value' } },
  { id: 'empty-name', page: { '': 'value' } },
  { id: 'newline-value', page: { 'X-A': 'first\nsecond' } },
  { id: 'undefined-value', page: { 'X-A': undefined } },
  { id: 'number-value', page: { 'X-A': 3 } },
  { id: 'context-create-invalid-value', initial: { 'X-A': 'bad\nvalue' } },
  { id: 'context-create-invalid-name', initial: { 'bad name': 'v' } },
  { id: 'empty-context-set-invalid-value', beforePage: { 'X-A': 'bad\nvalue' } },
  { id: 'empty-context-set-clear-invalid-value', beforePage: { 'X-A': 'bad\nvalue' }, clearBeforePage: true },
  { id: 'page-shadows-invalid-context', page: { 'X-A': 'page' }, context: { 'X-A': 'bad\nvalue' } },
  { id: 'last-page-duplicate-shadows-invalid-value', page: { 'X-A': 'bad\nvalue', 'x-a': 'valid' } },
];
let browser;
const observations = [];
try {
  browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
  for (const sample of cases) {
    let context, phase = 'create-context';
    requests = [];
    let outcome;
    try {
      context = await diagnosticDeadline(browser.newContext(sample.initial ? { extraHTTPHeaders: sample.initial } : {}), phase);
      if (sample.beforePage) { phase = 'empty-context-set'; await diagnosticDeadline(context.setExtraHTTPHeaders(sample.beforePage), phase); }
      if (sample.clearBeforePage) { phase = 'empty-context-clear'; await diagnosticDeadline(context.setExtraHTTPHeaders({}), phase); }
      phase = 'new-page';
      const page = await diagnosticDeadline(context.newPage(), phase);
      if (sample.page) { phase = 'page-set'; await diagnosticDeadline(page.setExtraHTTPHeaders(sample.page), phase); }
      if (sample.context) { phase = 'context-set'; await diagnosticDeadline(context.setExtraHTTPHeaders(sample.context), phase); }
      if (sample.clearPage) { phase = 'page-clear'; await diagnosticDeadline(page.setExtraHTTPHeaders({}), phase); }
      phase = 'frame-goto';
      const response = await page.mainFrame().goto(`${origin}/document`, { timeout: 5000, ...(sample.referer !== undefined ? { referer: sample.referer } : {}) });
      outcome = { status: response?.status(), requests };
    } catch (error) {
      assert.ok(error.message.length <= 4096);
      outcome = { phase, error: { name: error.name, message: error.message }, requests };
    } finally {
      if (context) await diagnosticDeadline(context.close(), 'header Context cleanup');
    }
    observations.push({ id: sample.id, ...outcome });
    console.log(`${sample.id}: ${outcome.error ? `${outcome.phase}: ${outcome.error.message.split('\n')[0]}` : JSON.stringify(outcome.requests)}`);
  }
} finally {
  server.closeAllConnections();
  try {
    if (browser) await diagnosticDeadline(browser.close(), 'header oracle disconnect');
  } finally {
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  }
}
const encoded = JSON.stringify({ version: '1.58.2', origin, observations }, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 128 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
