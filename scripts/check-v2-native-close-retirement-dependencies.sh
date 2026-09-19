#!/usr/bin/env bash
set -euo pipefail
repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
explicit=$(mktemp)
selected=$(mktemp)
trap 'rm -f "$explicit" "$selected"' EXIT
cd "$repo_dir"
cargo tree --locked -p pwright-composition-chromium --no-default-features \
  --features close-capacity,close-route-retirement,native-close-scheduler \
  --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//' | sort -u >"$explicit"
cargo tree --locked -p pwright-composition-chromium --no-default-features \
  --features native-close-retirement --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//' | sort -u >"$selected"
diff -u "$explicit" "$selected"
echo "native-close-retirement adds no packages or transitive features beyond its three explicit prerequisites"
