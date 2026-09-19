#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ordinary=$(mktemp)
selected=$(mktemp)
trap 'rm -f "$ordinary" "$selected"' EXIT
cd "$repo_dir"

cargo tree --locked -p pwright-backend-cdp-page-lifecycle --no-default-features \
  --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//' | sort -u >"$ordinary"
cargo tree --locked -p pwright-backend-cdp-page-lifecycle --no-default-features \
  --features native-confirmation --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//' | sort -u >"$selected"
diff -u "$ordinary" "$selected"
echo "native close confirmation/phases add no normal dependency packages or transitive features"
