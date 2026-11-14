#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

actual=$(cargo tree --locked -p pwright-plugin-chromium-session-initialization \
  --edges normal --prefix none --format '{p}' \
  | awk '/^pwright-/ { print $1 }' \
  | sort -u)
expected=$(printf '%s\n' \
  pwright-backend-cdp-topology \
  pwright-browser-model \
  pwright-cdp \
  pwright-core \
  pwright-plugin-chromium-session-initialization \
  | sort -u)

if [[ "$actual" != "$expected" ]]; then
  printf 'Unexpected Chromium session-initialization dependency closure:\n%s\nExpected:\n%s\n' \
    "$actual" "$expected" >&2
  exit 1
fi

aggregate_files=$(rg -l 'pwright_composition_chromium' \
  crates/pwright-node-host/src --glob '*.rs' | wc -l | tr -d ' ')
if (( aggregate_files > 39 )); then
  echo "Node host aggregate coupling grew to $aggregate_files source files (maximum 39)" >&2
  exit 1
fi

for file in \
  crates/pwright-node-host/src/context_defaults.rs \
  crates/pwright-node-host/src/extra_headers/native.rs
do
  if rg -q 'pwright_composition_chromium' "$file"; then
    echo "Extracted session initializer regressed to the aggregate in $file" >&2
    exit 1
  fi
  rg -q 'pwright_plugin_chromium_session_initialization' "$file"
done

CARGO_INCREMENTAL=0 cargo check --release --locked \
  -p pwright-plugin-chromium-session-initialization

echo "Chromium session-initialization dependency proof passed (5 Pwright packages; $aggregate_files aggregate-coupled host source files)"
