#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
manifest="$repo_dir/examples/v2-page-lifecycle-composition-only/Cargo.toml"
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
  pwright-v2-page-lifecycle-composition-only-example
  pwright-backend-cdp-common
  pwright-backend-cdp-page-lifecycle
  pwright-backend-cdp-topology
  pwright-browser-model
  pwright-cap-page-lifecycle
  pwright-cdp
  pwright-codec-binary
  pwright-codec-common
  pwright-codec-page-lifecycle
  pwright-composition-chromium-page-lifecycle
  pwright-connection-control
  pwright-core
  pwright-runtime
  pwright-server-page-lifecycle
  pwright-server-wire
  pwright-transport
)

for package in "${required_packages[@]}"; do
  if ! grep -Fxq "$package" "$tree_file"; then
    echo "Page-lifecycle composition proof is missing required package: $package" >&2
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
    echo "Page-lifecycle composition proof found forbidden Pwright package: $package" >&2
    exit 1
  fi
done <"$tree_file"

echo "Page-lifecycle composition dependency proof passed (${#required_packages[@]} Pwright packages)"
