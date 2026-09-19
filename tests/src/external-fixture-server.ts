import type { TestServer } from "./test-server.js";

// Static fixtures are served exclusively by the pre-registered serve-lib route.
// There is no local HTTP listener, file-serving fallback or URL rewriting.
export function externalFixtureServer(base: string): TestServer {
  const url = new URL(base);
  if (!["http:", "https:"].includes(url.protocol) || url.username || url.password || url.search || url.hash)
    throw new Error("fixture base must be an HTTP(S) directory URL without credentials, query or fragment");
  if (!url.hostname.endsWith(".ts.net"))
    throw new Error("fixture base must use the verified full Tailscale MagicDNS hostname");
  const PREFIX = url.href.replace(/\/$/, "");
  return {
    PREFIX, EMPTY_PAGE: `${PREFIX}/empty.html`,
    port: Number(url.port || (url.protocol === "https:" ? 443 : 80)),
    setRoute() { throw new Error("shared fixture infrastructure does not provide dynamic routes"); },
    setRedirect() { throw new Error("shared fixture infrastructure does not provide dynamic redirects"); },
    async close() { /* The caller owns the shared serve-lib registration. */ },
  };
}
