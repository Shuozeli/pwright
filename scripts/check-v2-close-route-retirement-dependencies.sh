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
cargo tree --locked -p pwright-composition-chromium --no-default-features \
  --features close-route-retirement --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//' | sort -u >"$selected"
diff -u "$ordinary" "$selected"
echo "close route retirement adds no normal packages or transitive features"
