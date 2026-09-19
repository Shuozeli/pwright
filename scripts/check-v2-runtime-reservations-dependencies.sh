#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ordinary=$(mktemp)
selected=$(mktemp)
trap 'rm -f "$ordinary" "$selected"' EXIT
cd "$repo_dir"

cargo tree --locked -p pwright-runtime --no-default-features \
  --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d' | sort -u >"$ordinary"
for features in disposal-reservations disposal-reservations,lifetime-handoff; do
  CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-runtime \
    --no-default-features --features "$features"
  # Only the root feature selection may change, not any dependency or feature.
  cargo tree --locked -p pwright-runtime --no-default-features --features "$features" \
    --edges normal --prefix none --format '{p} features={f}' \
    | sed '1d' | sort -u >"$selected"
  diff -u "$ordinary" "$selected"
done
echo "runtime disposal reservations add no normal dependency packages or transitive features"
