#!/usr/bin/env bash
# Audit all resources loaded by a page (sizes, types, timing).
# Uses the retroactive Performance API query — no listener needed.
#
# Usage:
#   ./page-resource-audit.sh https://example.com
#   ./page-resource-audit.sh https://example.com --filter ".js"
#
# Output: Table of all resources with type, status, size, and duration.

set -euo pipefail

URL="${1:?Usage: page-resource-audit.sh <url> [--filter pattern]}"
FILTER="${2:-}"
CDP="${PWRIGHT_CDP:-http://localhost:9222}"

# Open and wait for full page load
pwright --cdp "$CDP" open "$URL"
sleep 2

echo "Resources loaded by ${URL}:"
echo ""

if [ -n "$FILTER" ] && [ "$FILTER" = "--filter" ]; then
  PATTERN="${3:-}"
  pwright --cdp "$CDP" network-list --filter "$PATTERN"
else
  pwright --cdp "$CDP" network-list
fi
