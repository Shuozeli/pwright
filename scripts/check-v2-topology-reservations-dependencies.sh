#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ordinary=$(mktemp)
selected=$(mktemp)
trap 'rm -f "$ordinary" "$selected"' EXIT
cd "$repo_dir"

CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-backend-cdp-topology \
  --no-default-features --features transition-reservations

# Remove only the root package line: its explicit selected feature differs.
# Every transitive normal dependency, version AND enabled feature must match.
cargo tree --locked -p pwright-backend-cdp-topology --no-default-features \
  --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d' | sort -u >"$ordinary"
cargo tree --locked -p pwright-backend-cdp-topology --no-default-features \
  --features transition-reservations --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d' | sort -u >"$selected"
diff -u "$ordinary" "$selected"
echo "topology transition reservations add no normal dependency packages or transitive features"
