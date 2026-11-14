#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

closure() {
  cargo tree --locked -p pwright-bridge --no-default-features "$@" \
    --edges normal --prefix none --format '{p}' \
    | awk '/^pwright-/ { print $1 }' \
    | sort -u
}

baseline=$(closure)
selected=$(closure --features test-utils)
expected_baseline=$(printf '%s\n' pwright-bridge pwright-cdp pwright-core pwright-js | sort -u)
expected_selected=$(printf '%s\n' pwright-bridge pwright-cdp pwright-core pwright-fake pwright-js | sort -u)

if [[ "$baseline" != "$expected_baseline" ]]; then
  printf 'Unexpected pwright-bridge production dependency closure:\n%s\nExpected:\n%s\n' \
    "$baseline" "$expected_baseline" >&2
  exit 1
fi

if [[ "$selected" != "$expected_selected" ]]; then
  printf 'Unexpected pwright-bridge test-utils dependency closure:\n%s\nExpected:\n%s\n' \
    "$selected" "$expected_selected" >&2
  exit 1
fi

CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-bridge --no-default-features
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-bridge \
  --no-default-features --features test-utils

echo "Bridge test-utils dependency proof passed (4 production, 5 selected Pwright packages)"
