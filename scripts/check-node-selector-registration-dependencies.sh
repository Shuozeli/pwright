#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

base=$(cargo tree --locked -p pwright-node-addon --no-default-features \
  --edges normal --prefix none --format '{p}' \
  | awk '/^pwright-/ { print $1 }' \
  | sort -u)
selected=$(cargo tree --locked -p pwright-node-addon --no-default-features \
  --features selector-registration \
  --edges normal --prefix none --format '{p}' \
  | awk '/^pwright-/ { print $1 }' \
  | sort -u)

if [[ "$selected" != "$base" ]]; then
  printf 'Node selector-registration profile changed the base Pwright package closure:\n%s\nBase:\n%s\n' \
    "$selected" "$base" >&2
  exit 1
fi

CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --no-default-features
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --no-default-features --features selector-registration
echo "Node selector-registration dependency proof passed (no additional Pwright package)"
