#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
manifest="$repo_dir/examples/v2-locator-screenshot-only/Cargo.toml"
tree_file=$(mktemp)
trap 'rm -f "$tree_file"' EXIT

CARGO_INCREMENTAL=0 cargo check --release --locked --manifest-path "$manifest"

cargo tree \
  --manifest-path "$manifest" \
  --locked \
  --edges normal \
  --prefix none \
  --format '{p}' \
  | awk '/^pwright-/ { print $1 }' \
  | sort -u >"$tree_file"

required_packages=(
  pwright-v2-locator-screenshot-only-example
  pwright-composition-chromium-locator-screenshot
  pwright-backend-cdp-common
  pwright-backend-cdp-content
  pwright-backend-cdp-locator-screenshot
  pwright-backend-cdp-selector-geometry
  pwright-backend-cdp-selector-runtime
  pwright-browser-backend
  pwright-cap-content
  pwright-cap-locator-screenshot
  pwright-cdp
  pwright-codec-binary
  pwright-codec-common
  pwright-codec-locator-screenshot
  pwright-codec-screenshot
  pwright-codec-selector
  pwright-connection-control
  pwright-core
  pwright-screenshot-model
  pwright-selector-model
  pwright-server-locator-screenshot
  pwright-server-wire
  pwright-transport
)

for package in "${required_packages[@]}"; do
  if ! grep -Fxq "$package" "$tree_file"; then
    echo "Locator-screenshot dependency proof is missing required package: $package" >&2
    exit 1
  fi
done

while IFS= read -r package; do
  allowed=false
  for required in "${required_packages[@]}"; do
    if [[ "$package" == "$required" ]]; then
      allowed=true
      break
    fi
  done
  if [[ "$allowed" != true ]]; then
    echo "Locator-screenshot dependency proof found forbidden Pwright package: $package" >&2
    exit 1
  fi
done <"$tree_file"

echo "Locator-screenshot dependency proof passed (${#required_packages[@]} Pwright packages)"
