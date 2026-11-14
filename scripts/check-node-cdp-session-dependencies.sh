#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

base_file=$(mktemp)
selected_file=$(mktemp)
browser_file=$(mktemp)
trap 'rm -f "$base_file" "$selected_file" "$browser_file"' EXIT

cargo tree --offline --locked -p pwright-node-addon --no-default-features \
  --edges normal --prefix none --format '{p}' | sort -u >"$base_file"
cargo tree --offline --locked -p pwright-node-addon --no-default-features \
  --features cdp-session --edges normal --prefix none --format '{p}' \
  | sort -u >"$selected_file"

if ! diff -u "$base_file" "$selected_file"; then
  printf 'Node cdp-session changed the base package dependency closure.\n' >&2
  exit 1
fi

cargo tree --offline --locked -p pwright-node-addon --no-default-features \
  --features browser-cdp-session --edges normal --prefix none --format '{p}' \
  | sort -u >"$browser_file"
if ! diff -u "$base_file" "$browser_file"; then
  printf 'Node browser-cdp-session changed the base package dependency closure.\n' >&2
  exit 1
fi

CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo check --offline --release --locked \
  -p pwright-node-addon --no-default-features --features cdp-session
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo check --offline --release --locked \
  -p pwright-node-addon --no-default-features --features browser-cdp-session
printf 'Node cdp-session and browser-cdp-session dependency proofs passed (zero added packages).\n'
