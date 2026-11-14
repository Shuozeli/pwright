#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

dependency_closure() {
  local features=$1
  cargo tree --locked -p pwright-node-addon \
    --features "$features" \
    --edges normal --prefix none --format '{p}' \
    | awk '{ print $1 }' \
    | sort -u
}

base_file=$(mktemp)
selected_file=$(mktemp)
trap 'rm -f "$base_file" "$selected_file"' EXIT

dependency_closure api-request >"$base_file"
dependency_closure api-request-proxy-bypass >"$selected_file"

added=$(comm -13 "$base_file" "$selected_file")
removed=$(comm -23 "$base_file" "$selected_file")
expected='pwright-api-request-connector-proxy-bypass'

if [[ -n "$removed" ]]; then
  printf 'Proxy-bypass profile removed base dependencies:\n%s\n' "$removed" >&2
  exit 1
fi

if [[ "$added" != "$expected" ]]; then
  printf 'Unexpected proxy-bypass dependency delta:\n%s\nExpected:\n%s\n' \
    "$added" "$expected" >&2
  exit 1
fi

CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --features api-request-proxy-bypass
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --features api-request-http-proxy,api-request-proxy-bypass
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --features api-request-socks5-proxy,api-request-proxy-bypass
echo 'Node proxy-bypass dependency proof passed (exactly one Pwright leaf)'
