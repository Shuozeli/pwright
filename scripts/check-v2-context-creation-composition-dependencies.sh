#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
manifest="$repo_dir/examples/v2-context-creation-composition-only/Cargo.toml"
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
  pwright-v2-context-creation-composition-only-example
  pwright-backend-cdp-common
  pwright-backend-cdp-context-creation
  pwright-backend-cdp-context-lifecycle
  pwright-backend-cdp-topology
  pwright-browser-model
  pwright-cap-context-creation
  pwright-cap-context-lifecycle
  pwright-cdp
  pwright-codec-binary
  pwright-codec-common
  pwright-codec-context-creation
  pwright-codec-context-lifecycle
  pwright-composition-chromium-context-creation
  pwright-composition-chromium-context-lifecycle
  pwright-connection-control
  pwright-core
  pwright-runtime
  pwright-server-context-creation
  pwright-server-context-lifecycle
  pwright-server-wire
  pwright-transport
)

for package in "${required_packages[@]}"; do
  if ! grep -Fxq "$package" "$tree_file"; then
    echo "Context-creation composition proof is missing required package: $package" >&2
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
    echo "Context-creation composition proof found forbidden Pwright package: $package" >&2
    exit 1
  fi
done <"$tree_file"

echo "Context-creation composition dependency proof passed (${#required_packages[@]} Pwright packages)"
