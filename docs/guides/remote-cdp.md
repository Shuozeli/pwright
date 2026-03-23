# Connecting to Chrome on a Remote Machine

> **Security warning.** CDP gives full control over the browser --
> navigate to any page, read cookies, execute JS, access local files
> via `file://`. Anyone who can reach the CDP port has the same access
> as the logged-in user. Never expose CDP to the public internet. Always
> restrict access via VPN, firewall, or SSH tunnel.

Chrome's `--remote-debugging-port` binds to localhost by default. To
connect from another machine, you need either `--remote-debugging-address=0.0.0.0`
(exposes CDP to the network, no auth) or a reverse proxy that forwards
the connection securely.

## Option 1: Direct Binding (simple, no auth)

Start Chrome with:
```bash
chrome --remote-debugging-port=9222 --remote-debugging-address=0.0.0.0
```

Connect from another machine:
```bash
pwright --cdp http://remote-host:9222 health
```

This works but exposes full browser control to anyone who can reach
port 9222. Only use this on a trusted network (e.g., Tailscale, LAN).

## Option 2: Reverse Proxy with Caddy

Caddy can proxy both the HTTP API and WebSocket connections. Chrome
binds to localhost; Caddy handles TLS and access control.

### Chrome (on the remote machine)

```bash
chrome --remote-debugging-port=9222
# Binds to 127.0.0.1:9222 only
```

### Caddyfile

```
cdp.example.com {
    reverse_proxy localhost:9222
}
```

Caddy automatically provisions TLS via Let's Encrypt. The
`reverse_proxy` directive handles WebSocket upgrade transparently.

Connect from another machine:
```bash
pwright --cdp https://cdp.example.com health
```

### With Basic Auth

```
cdp.example.com {
    basicauth {
        admin $2a$14$... # caddy hash-password
    }
    reverse_proxy localhost:9222
}
```

Note: pwright's `connect_http` uses `reqwest` for the initial
`/json/version` fetch, which does not send basic auth headers.
You would need to configure auth at the network level (VPN, firewall)
rather than HTTP basic auth, or extend pwright to support auth headers.

## Option 3: Reverse Proxy with nginx

### nginx config

```nginx
server {
    listen 443 ssl;
    server_name cdp.example.com;

    ssl_certificate     /etc/ssl/certs/cdp.pem;
    ssl_certificate_key /etc/ssl/private/cdp.key;

    location / {
        proxy_pass http://127.0.0.1:9222;

        # WebSocket support (required for CDP)
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;

        # Timeouts for long-lived WebSocket connections
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
    }
}
```

The key lines are `proxy_set_header Upgrade` and `Connection "upgrade"` --
without these, WebSocket connections fail and you get HTTP 404 errors.

### With IP allowlist

```nginx
    allow 10.0.0.0/24;     # LAN
    allow 100.64.0.0/10;   # Tailscale
    deny all;
```

## Option 4: SSH Tunnel (no proxy needed)

Forward the port over SSH:

```bash
ssh -L 9222:localhost:9222 user@remote-host
```

Then connect locally:
```bash
pwright --cdp http://localhost:9222 health
```

Simple, encrypted, no proxy config. Works well for development.

## How pwright Resolves the Connection

`Browser::connect_http(url)` does:
1. GET `{url}/json/version` to discover the `webSocketDebuggerUrl`
2. Connect to that WebSocket URL for CDP communication

The WebSocket URL returned by Chrome contains the hostname Chrome sees
(usually `localhost` or `0.0.0.0`). When behind a proxy, Chrome still
returns `ws://localhost:9222/devtools/browser/...` in its response.
pwright rewrites this to use the original HTTP host, so the WebSocket
connection goes through the proxy correctly.

If you use `Browser::connect(config)` directly, you must provide the
full WebSocket URL yourself (e.g., from a prior `/json/version` call).

## Recommendation

| Setup | Use case |
|-------|----------|
| SSH tunnel | Development, single user |
| Tailscale + direct bind | Home lab, trusted machines |
| Caddy reverse proxy | Production, TLS, multiple users |
| nginx reverse proxy | Production, existing nginx infrastructure |
