/**
 * Local HTTP server for test fixtures.
 * Serves static HTML files and supports dynamic route registration.
 */

import http from "http";
import { readFileSync, existsSync } from "fs";
import { resolve, extname } from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const FIXTURES_DIR = resolve(__dirname, "../fixtures");

const MIME_TYPES: Record<string, string> = {
  ".html": "text/html",
  ".js": "application/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".svg": "image/svg+xml",
};

export interface TestServer {
  /** Base URL, e.g. http://localhost:8787 */
  PREFIX: string;
  /** Empty page URL */
  EMPTY_PAGE: string;
  /** Port number */
  port: number;
  /** Register a custom route handler */
  setRoute(path: string, handler: (req: http.IncomingMessage, res: http.ServerResponse) => void): void;
  /** Register a redirect */
  setRedirect(from: string, to: string): void;
  /** Stop the server */
  close(): Promise<void>;
}

export async function createTestServer(): Promise<TestServer> {
  const routes = new Map<string, (req: http.IncomingMessage, res: http.ServerResponse) => void>();

  const server = http.createServer((req, res) => {
    const url = req.url || "/";
    const pathname = url.split("?")[0];

    // Check custom routes first
    const handler = routes.get(pathname);
    if (handler) {
      handler(req, res);
      return;
    }

    // Serve static fixtures
    const filePath = resolve(FIXTURES_DIR, pathname.slice(1));
    if (!filePath.startsWith(FIXTURES_DIR)) {
      res.writeHead(403);
      res.end("Forbidden");
      return;
    }

    if (existsSync(filePath)) {
      const ext = extname(filePath);
      const mime = MIME_TYPES[ext] || "application/octet-stream";
      const content = readFileSync(filePath);
      res.writeHead(200, { "Content-Type": mime });
      res.end(content);
    } else {
      res.writeHead(404);
      res.end("Not Found");
    }
  });

  // Find a free port
  await new Promise<void>((resolve, reject) => {
    server.listen(0, "127.0.0.1", () => resolve());
    server.on("error", reject);
  });

  const address = server.address() as { port: number };
  const port = address.port;
  const PREFIX = `http://127.0.0.1:${port}`;

  return {
    PREFIX,
    EMPTY_PAGE: `${PREFIX}/empty.html`,
    port,
    setRoute(path, handler) {
      routes.set(path, handler);
    },
    setRedirect(from, to) {
      routes.set(from, (_req, res) => {
        res.writeHead(302, { Location: to });
        res.end();
      });
    },
    async close() {
      return new Promise((resolve) => server.close(() => resolve()));
    },
  };
}
