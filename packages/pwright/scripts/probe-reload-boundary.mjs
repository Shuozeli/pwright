// Dynamic causal fixture only. Static files remain served by serve-lib.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';

const bind = process.env.PWRIGHT_TEST_BIND;
const host = process.env.PWRIGHT_TEST_HOST;
assert.ok(bind && /^100\./.test(bind));
assert.ok(host && host.endsWith('.ts.net'));
assert.equal(process.env.PWRIGHT_TEST_CDP, 'http://127.0.0.1:9222');
const lane = process.argv[2] ?? 'host';
assert.ok(lane === 'host' || lane === 'raw', 'lane must be host or raw');
const modes = ['native-close', 'native-close-at-response', 'stop-before-close'];
if (lane === 'host') modes.unshift('baseline');
const counts = new Map(modes.map(mode => [mode, 0]));
const cargoTarget = lane === 'host'
  ? ['-p', 'pwright-node-host', '--lib', 'reload::boundary_probe::trace_short_response_close_boundary']
  : ['-p', 'pwright-cdp', '--test', 'reload_close_boundary', 'raw_reload_close_boundary'];
let child;
const server = createServer((request, response) => {
  const url = new URL(request.url, 'http://fixture.invalid');
  const mode = url.searchParams.get('mode');
  if (!counts.has(mode)) { response.writeHead(404).end(); return; }
  if (url.pathname !== '/document') { response.writeHead(404).end(); return; }
  const count = counts.get(mode) + 1;
  counts.set(mode, count);
  if (count > 2) { response.writeHead(409).end(); return; }
  response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
  response.write('<title>held reload</title>');
  if (count === 1) response.end('<p>initial</p>');
  else {
    response.once('close', () => console.log(`held-response-closed ${mode}`));
    child.stdin.write(`${mode}\n`);
  }
});
server.listen(0, bind);
await once(server, 'listening');
try {
  child = spawn('cargo', ['test', '--release', '--locked', ...cargoTarget, '--', '--ignored', '--exact', '--nocapture'], {
    cwd: fileURLToPath(new URL('../../../', import.meta.url)),
    env: { ...process.env, CARGO_INCREMENTAL: '0', PWRIGHT_RELOAD_PROBE_FIXTURE: `http://${host}:${server.address().port}` },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  let bytes = 0;
  let overflow = false;
  for (const [stream, output] of [[child.stdout, process.stdout], [child.stderr, process.stderr]]) stream.on('data', chunk => {
    bytes += chunk.byteLength;
    if (bytes > 1024 * 1024) { overflow = true; child.kill('SIGKILL'); }
    else output.write(chunk);
  });
  const timer = setTimeout(() => child.kill('SIGKILL'), 120000);
  try {
    const [status, signal] = await once(child, 'close');
    assert.equal(status, 0, `probe exit=${status}, signal=${signal}`);
    assert.equal(overflow, false);
  } finally { clearTimeout(timer); }
} finally {
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
}
