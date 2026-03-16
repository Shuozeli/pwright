#!/usr/bin/env bash
# Capture the API call made when submitting a form.
#
# Usage:
#   ./capture-form-submit.sh https://example.com/search "#search-input" "query text" "#submit-btn"
#
# Arguments:
#   $1 - URL of the page with the form
#   $2 - CSS selector for the input field (use pwright snapshot to find refs)
#   $3 - Text to fill in the input
#   $4 - CSS selector or ref for the submit button
#
# Output: Prints the API request/response that the form submission triggers.

set -euo pipefail

URL="${1:?Usage: capture-form-submit.sh <url> <input-selector> <text> <submit-selector>}"
INPUT="${2:?Missing input selector}"
TEXT="${3:?Missing text to fill}"
SUBMIT="${4:?Missing submit selector}"
CDP="${PWRIGHT_CDP:-http://localhost:9222}"
CAPTURE_FILE=$(mktemp /tmp/pwright-capture-XXXXXX.jsonl)

# Open page
pwright --cdp "$CDP" open "$URL"

# Start network listener in background
pwright --cdp "$CDP" network-listen --type XHR,Fetch > "$CAPTURE_FILE" &
LISTENER_PID=$!
sleep 1

# Take snapshot to get refs
pwright --cdp "$CDP" snapshot

# Fill the form and submit
echo "Filling '${INPUT}' with '${TEXT}'..."
pwright --cdp "$CDP" eval "document.querySelector('${INPUT}').focus()"
pwright --cdp "$CDP" type "$TEXT"
echo "Clicking '${SUBMIT}'..."
pwright --cdp "$CDP" eval "document.querySelector('${SUBMIT}').click()"

# Wait for API call
sleep 3
kill "$LISTENER_PID" 2>/dev/null || true
wait "$LISTENER_PID" 2>/dev/null || true

echo ""
echo "=== Captured API Calls ==="
cat "$CAPTURE_FILE"

# Try to get response body for each response
echo ""
echo "=== Response Bodies ==="
grep '"event":"response"' "$CAPTURE_FILE" | while IFS= read -r line; do
  REQID=$(echo "$line" | jq -r '.reqid')
  URL=$(echo "$line" | jq -r '.url')
  STATUS=$(echo "$line" | jq -r '.status')
  echo ""
  echo "--- ${STATUS} ${URL} (${REQID}) ---"
  pwright --cdp "$CDP" network-get "$REQID" 2>/dev/null || echo "(body not available)"
done

rm -f "$CAPTURE_FILE"
