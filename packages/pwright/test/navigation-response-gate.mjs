import { createServer } from 'node:http';
import { once } from 'node:events';

// A dynamic, caller-released HTTP response, not a static file server. The
// initial document is committed while its unfinished body prevents load.
export async function withNavigationResponseGate(bind, hostname, action) {
  let arrived;
  const started = new Promise(resolve => { arrived = resolve; });
  let pending;
  let requests = 0;
  const server = createServer((request, response) => {
    if (request.url !== '/held-document' || ++requests !== 1) {
      response.writeHead(404).end();
      return;
    }
    pending = response;
    response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' });
    response.write('<!doctype html><title>Caller-held document</title><body><p id="marker">committed</p>');
    arrived();
  });
  server.listen(0, bind);
  await once(server, 'listening');
  const url = `http://${hostname}:${server.address().port}/held-document`;
  const release = () => {
    if (!pending || pending.writableEnded) throw new Error('Expected one pending fixture response');
    pending.end('</body></html>');
  };
  try { return await action({ url, started, release }); }
  finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
}
