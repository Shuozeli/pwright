#!/usr/bin/env bash
# Discover API endpoints by navigating a site and capturing all XHR/Fetch traffic.
#
# Usage:
#   ./discover-api.sh https://example.com
#   ./discover-api.sh https://example.com 30    # capture for 30 seconds
#
# Output: api-discovery.jsonl (all captured API requests/responses)
#
# Workflow:
#   1. Opens the site
#   2. Starts network listener in background (filtered to XHR/Fetch)
#   3. Waits for user interactions or auto-timeout
#   4. Outputs structured JSONL of all API calls

set -euo pipefail

URL="${1:?Usage: discover-api.sh <url> [duration_seconds]}"
DURATION="${2:-15}"
OUTPUT="api-discovery.jsonl"
CDP="${PWRIGHT_CDP:-http://localhost:9222}"

echo "Opening ${URL}..."
pwright --cdp "$CDP" open "$URL"

echo "Capturing API traffic for ${DURATION}s..."
echo "Interact with the page in Chrome to trigger API calls."
echo ""

# Capture XHR and Fetch requests (the interesting API calls)
pwright --cdp "$CDP" network-listen \
  --type XHR,Fetch \
  --duration "$DURATION" \
  > "$OUTPUT" 2>/dev/null

REQUESTS=$(grep -c '"event":"request"' "$OUTPUT" 2>/dev/null || echo 0)
RESPONSES=$(grep -c '"event":"response"' "$OUTPUT" 2>/dev/null || echo 0)

echo ""
echo "Captured ${REQUESTS} requests, ${RESPONSES} responses -> ${OUTPUT}"

if [ "$REQUESTS" -gt 0 ]; then
  echo ""
  echo "Unique API endpoints:"
  grep '"event":"request"' "$OUTPUT" \
    | jq -r '[.method, .url] | join(" ")' 2>/dev/null \
    | sort -u \
    | head -20
fi
