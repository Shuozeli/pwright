# Firefox ESR with CDP enabled for integration tests.
# Firefox binds --remote-debugging-port to 127.0.0.1 only,
# so we use socat to expose it on 0.0.0.0 for Docker networking.
#
# Firefox does NOT expose Chrome-style /json/version HTTP discovery.
# The entrypoint captures the WS URL from stderr and writes the path
# to a shared volume so the test-runner can construct the full WS URL.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    firefox-esr \
    socat \
    --no-install-recommends && rm -rf /var/lib/apt/lists/*

# Create a profile directory with CDP enabled (protocol 2=CDP-only)
RUN mkdir -p /tmp/firefox-profile /tmp/firefox-cdp && \
    printf '%s\n' \
      'user_pref("remote.active-protocols", 2);' \
      'user_pref("remote.allow-hosts", "localhost,0.0.0.0,127.0.0.1");' \
      'user_pref("remote.allow-origins", "null");' \
      'user_pref("fission.bfcacheInParent", false);' \
      'user_pref("fission.webContentIsolationStrategy", 0);' \
      > /tmp/firefox-profile/user.js

COPY firefox-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 9222

HEALTHCHECK --interval=2s --timeout=3s --retries=30 \
    CMD test -f /tmp/firefox-cdp/ws-path

CMD ["/entrypoint.sh"]
