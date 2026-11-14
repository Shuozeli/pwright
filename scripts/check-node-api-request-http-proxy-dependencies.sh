#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

base=$(cargo tree --locked -p pwright-node-addon \
  --features api-request \
  --edges normal --prefix none --format '{p}' \
  | awk '{ print $1 }' \
  | sort -u)
selected=$(cargo tree --locked -p pwright-node-addon \
  --features api-request-http-proxy \
  --edges normal --prefix none --format '{p}' \
  | awk '{ print $1 }' \
  | sort -u)
added=$(comm -13 <(printf '%s\n' "$base") <(printf '%s\n' "$selected"))
removed=$(comm -23 <(printf '%s\n' "$base") <(printf '%s\n' "$selected"))
expected='pwright-api-request-connector-http-proxy'

if [[ -n "$removed" ]]; then
  printf 'HTTP-proxy profile removed base dependencies:\n%s\n' "$removed" >&2
  exit 1
fi

if [[ "$added" != "$expected" ]]; then
  printf 'Unexpected HTTP-proxy dependency delta:\n%s\nExpected:\n%s\n' \
    "$added" "$expected" >&2
  exit 1
fi

CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --features api-request
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --features api-request-http-proxy
echo 'Node HTTP-proxy dependency proof passed (exactly one Pwright leaf)'
