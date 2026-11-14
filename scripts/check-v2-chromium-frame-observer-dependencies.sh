#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

actual=$(cargo tree --locked -p pwright-plugin-chromium-frame-observer \
  --edges normal --prefix none --format '{p}' \
  | awk '/^pwright-/ { print $1 }' \
  | sort -u)
expected=$(printf '%s\n' \
  pwright-backend-cdp-topology \
  pwright-browser-model \
  pwright-cdp \
  pwright-core \
  pwright-plugin-chromium-frame-observer \
  | sort -u)

if [[ "$actual" != "$expected" ]]; then
  printf 'Unexpected Chromium frame-observer dependency closure:\n%s\nExpected:\n%s\n' \
    "$actual" "$expected" >&2
  exit 1
fi

aggregate_files=$(rg -l 'pwright_composition_chromium' \
  crates/pwright-node-host/src --glob '*.rs' | wc -l | tr -d ' ')
if (( aggregate_files > 39 )); then
  echo "Node host aggregate coupling grew to $aggregate_files source files (maximum 39)" >&2
  exit 1
fi

for host_observer in \
  crates/pwright-node-host/src/extra_headers/session_retirement.rs \
  crates/pwright-node-host/src/frame_metadata.rs \
  crates/pwright-node-host/src/frame_trees.rs
do
  if rg -q 'pwright_composition_chromium' "$host_observer"; then
    echo "Extracted frame observer regressed to the aggregate in $host_observer" >&2
    exit 1
  fi
  rg -q 'pwright_plugin_chromium_frame_observer' "$host_observer"
done
rg -q 'pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver' \
  crates/pwright-node-host/src/lib.rs
rg -q 'ChromiumFrameEventObserver, FrameObservationResult' \
  crates/pwright-composition-chromium/src/frame_event_observer.rs
rg -q 'ChromiumFrameUrlObserver' \
  crates/pwright-composition-chromium/src/topology_events.rs

CARGO_INCREMENTAL=0 cargo check --release --locked \
  -p pwright-plugin-chromium-frame-observer

echo "Chromium frame-observer dependency proof passed (5 Pwright packages; $aggregate_files aggregate-coupled host source files)"
