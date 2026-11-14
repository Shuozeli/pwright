#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

dependency_closure() {
  local features=$1
  local -a arguments=(--locked -p pwright-node-addon --no-default-features
    --edges normal --prefix none --format '{p}')
  if [[ -n "$features" ]]; then
    arguments+=(--features "$features")
  fi
  cargo tree "${arguments[@]}" | awk '{ print $1 }' | sort -u
}

base_file=$(mktemp)
selected_file=$(mktemp)
trap 'rm -f "$base_file" "$selected_file"' EXIT

dependency_closure '' >"$base_file"
dependency_closure context-cookies >"$selected_file"

added=$(comm -13 "$base_file" "$selected_file")
removed=$(comm -23 "$base_file" "$selected_file")
expected=$(printf '%s\n' aho-corasick regex regex-automata regex-syntax)

if [[ -n "$removed" || "$added" != "$expected" ]]; then
  printf 'Context cookies changed the exact normal dependency closure.\nAdded:\n%s\nRemoved:\n%s\nExpected additions:\n%s\n' \
    "$added" "$removed" "$expected" >&2
  exit 1
fi

CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --no-default-features --features context-cookies
echo 'Node Context cookies dependency proof passed (four regex packages, no Pwright leaf)'
