#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

actual=$(cargo tree --locked -p pwright-plugin-chromium-frame-lifecycle \
  --edges normal --prefix none --format '{p}' \
  | awk '/^pwright-/ { print $1 }' \
  | sort -u)
expected=$(printf '%s\n' \
  pwright-backend-cdp-common \
  pwright-backend-cdp-navigation \
  pwright-backend-cdp-topology \
  pwright-browser-backend \
  pwright-browser-model \
  pwright-cap-navigation \
  pwright-cdp \
  pwright-core \
  pwright-plugin-chromium-frame-lifecycle \
  pwright-plugin-chromium-frame-observer \
  | sort -u)

if [[ "$actual" != "$expected" ]]; then
  printf 'Unexpected Chromium frame-lifecycle dependency closure:\n%s\nExpected:\n%s\n' \
    "$actual" "$expected" >&2
  exit 1
fi

aggregate_files=$(rg -l 'pwright_composition_chromium' \
  crates/pwright-node-host/src --glob '*.rs' | wc -l | tr -d ' ')
if (( aggregate_files > 39 )); then
  echo "Node host aggregate coupling grew to $aggregate_files source files (maximum 39)" >&2
  exit 1
fi

host_lifecycle=crates/pwright-node-host/src/frame_lifecycle.rs
if rg -q 'pwright_composition_chromium' "$host_lifecycle"; then
  echo "Extracted Frame lifecycle regressed to the aggregate in $host_lifecycle" >&2
  exit 1
fi
rg -q 'pwright_plugin_chromium_frame_lifecycle' "$host_lifecycle"
rg -q 'pub use pwright_plugin_chromium_frame_lifecycle::\*' \
  crates/pwright-composition-chromium/src/lib.rs

CARGO_INCREMENTAL=0 cargo check --release --locked \
  -p pwright-plugin-chromium-frame-lifecycle

echo "Chromium frame-lifecycle dependency proof passed (10 Pwright packages; $aggregate_files aggregate-coupled host source files)"
