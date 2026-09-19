#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ordinary=$(mktemp)
selected=$(mktemp)
trap 'rm -f "$ordinary" "$selected"' EXIT
cd "$repo_dir"

cargo tree --locked -p pwright-composition-chromium-handle-evaluation \
  --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d' | sort -u >"$ordinary"
cargo tree --locked -p pwright-composition-chromium-handle-evaluation \
  --features native-creation --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d' | sort -u >"$selected"
diff -u "$ordinary" "$selected"
echo "native Handle creation adds no dependency package or transitive feature"
