#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

base_file=$(mktemp)
selected_file=$(mktemp)
trap 'rm -f "$base_file" "$selected_file"' EXIT

cargo tree --offline --locked -p pwright-node-addon --no-default-features \
  --edges normal --prefix none --format '{p}' | sort -u >"$base_file"
cargo tree --offline --locked -p pwright-node-addon --no-default-features \
  --features cdp-handshake-headers --edges normal --prefix none --format '{p}' \
  | sort -u >"$selected_file"

if ! diff -u "$base_file" "$selected_file"; then
  printf 'Node cdp-handshake-headers changed the base package dependency closure.\n' >&2
  exit 1
fi

CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo check --offline --release --locked \
  -p pwright-node-addon --no-default-features
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo check --offline --release --locked \
  -p pwright-node-addon --no-default-features --features cdp-handshake-headers
printf 'Node cdp-handshake-headers dependency proof passed (zero added packages).\n'
