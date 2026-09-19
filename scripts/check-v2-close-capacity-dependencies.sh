#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ordinary=$(mktemp)
selected=$(mktemp)
trap 'rm -f "$ordinary" "$selected"' EXIT
cd "$repo_dir"

cargo tree --locked -p pwright-composition-chromium --no-default-features \
  --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//' | sort -u >"$ordinary"
# Only these two explicitly selected reservation capabilities may differ.
# The topology crate already exists transitively; reject any added package or
# unrelated feature, including accidental native scheduling selection.
cargo tree --locked -p pwright-composition-chromium --no-default-features \
  --features close-capacity --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//; /^pwright-backend-cdp-topology /s/features=default,transition-reservations$/features=default/; /^pwright-runtime /s/features=default,disposal-reservations$/features=default/' \
  | sort -u >"$selected"
diff -u "$ordinary" "$selected"
echo "close-capacity selects topology/runtime reservations only, with no new normal packages"
