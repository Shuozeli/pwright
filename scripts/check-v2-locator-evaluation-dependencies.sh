#!/usr/bin/env bash
set -euo pipefail
repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-composition-chromium-locator-evaluation
actual=$(cargo tree --locked -p pwright-composition-chromium-locator-evaluation --edges normal --prefix none --format '{p}' | awk '/^pwright-/ {print $1}' | sort -u)
expected=$(printf '%s\n' \
  pwright-backend-cdp-common pwright-backend-cdp-evaluation \
  pwright-backend-cdp-handle-evaluation pwright-backend-cdp-locator-resolution \
  pwright-backend-cdp-selector-runtime pwright-browser-backend pwright-browser-model \
  pwright-cap-evaluation pwright-cap-handle-evaluation pwright-cdp \
  pwright-composition-chromium-locator-evaluation pwright-composition-chromium-locator-resolution \
  pwright-core pwright-runtime pwright-selector-model | sort -u)
if [[ "$actual" != "$expected" ]]; then
  printf 'Unexpected Locator element-evaluation dependency closure:\n%s\nExpected:\n%s\n' "$actual" "$expected" >&2
  exit 1
fi
printf 'Locator element-evaluation dependency proof passed (15 Pwright packages)\n'
