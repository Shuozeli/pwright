#!/usr/bin/env bash
# Extract response bodies from API calls matching a URL pattern.
#
# Usage:
#   ./extract-api-responses.sh https://example.com "/api/" 15
#
# Arguments:
#   $1 - URL to navigate to
#   $2 - URL pattern to filter (substring match)
#   $3 - Duration in seconds (default: 10)
#
# Output: Each matching API response body printed to stdout.
# Pipe to jq for formatting, or redirect to a file.

set -euo pipefail

URL="${1:?Usage: extract-api-responses.sh <url> <url-pattern> [duration]}"
PATTERN="${2:?Missing URL pattern}"
DURATION="${3:-10}"
CDP="${PWRIGHT_CDP:-http://localhost:9222}"
CAPTURE_FILE=$(mktemp /tmp/pwright-extract-XXXXXX.jsonl)

# Open and navigate
pwright --cdp "$CDP" open "$URL"

# Capture traffic
pwright --cdp "$CDP" network-listen \
  --filter "$PATTERN" \
  --duration "$DURATION" \
  > "$CAPTURE_FILE" 2>/dev/null

RESPONSE_COUNT=$(grep -c '"event":"response"' "$CAPTURE_FILE" 2>/dev/null || echo 0)

if [ "$RESPONSE_COUNT" -eq 0 ]; then
  echo "No API responses matching '${PATTERN}' captured in ${DURATION}s." >&2
  rm -f "$CAPTURE_FILE"
  exit 1
fi

echo "Found ${RESPONSE_COUNT} matching responses:" >&2

# Extract each response body
grep '"event":"response"' "$CAPTURE_FILE" | while IFS= read -r line; do
  REQID=$(echo "$line" | jq -r '.reqid')
  RESP_URL=$(echo "$line" | jq -r '.url')
  STATUS=$(echo "$line" | jq -r '.status')
  echo "  ${STATUS} ${RESP_URL}" >&2
  pwright --cdp "$CDP" network-get "$REQID" 2>/dev/null
done

rm -f "$CAPTURE_FILE"
