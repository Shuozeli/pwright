#!/bin/sh
# Firefox binds --remote-debugging-port to 127.0.0.1 only.
# We use socat on the container's external IP (same port 9222) to expose it
# to the Docker network. Port must match so Firefox accepts the Host header.
#
# NOTE: --remote-allow-hosts/origins are intentionally permissive for testing.
# Do NOT use this configuration outside of ephemeral CI/test containers.

# Clean stale state from previous runs
rm -f /tmp/firefox-cdp/ws-path

# Get the container's non-loopback IP
CONTAINER_IP=$(hostname -i | awk '{print $1}')
echo "Container IP: $CONTAINER_IP"

# Start Firefox with CDP on 127.0.0.1:9222
firefox-esr \
    --headless \
    --no-remote \
    --profile /tmp/firefox-profile \
    --remote-debugging-port 9222 \
    --remote-allow-hosts "*" \
    --remote-allow-origins "*" \
    2>&1 | while IFS= read -r line; do
    echo "$line"
    case "$line" in
        *"DevTools listening on"*)
            # Extract WS path using POSIX-compatible sed (no grep -P)
            ws_path=$(echo "$line" | sed -n 's|.*\(/devtools/browser/[^ ]*\).*|\1|p')
            echo "$ws_path" > /tmp/firefox-cdp/ws-path
            echo "Firefox CDP ready at path: $ws_path"
            # Start socat AFTER Firefox is listening (avoids race condition)
            socat TCP-LISTEN:9222,fork,bind="$CONTAINER_IP",reuseaddr TCP:127.0.0.1:9222 &
            echo "socat forwarding $CONTAINER_IP:9222 -> 127.0.0.1:9222"
            ;;
    esac
done
