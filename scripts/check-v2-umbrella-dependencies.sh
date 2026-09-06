#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
manifest="$repo_dir/examples/v2-umbrella-profiles/Cargo.toml"
temporary_files=("")

cleanup() {
  for temporary_file in "${temporary_files[@]}"; do
    if [[ -n "$temporary_file" ]]; then
      rm -f "$temporary_file"
    fi
  done
}
trap cleanup EXIT

profile_packages=(
  pwright-v2-umbrella-empty
  pwright-v2-umbrella-client-content
  pwright-v2-umbrella-client-context-lifecycle
  pwright-v2-umbrella-client-page-lifecycle
  pwright-v2-umbrella-client-page-creation
  pwright-v2-umbrella-client-page-events
  pwright-v2-umbrella-client-network-events
  pwright-v2-umbrella-client-handle-lifecycle
  pwright-v2-umbrella-client-handle-evaluation
  pwright-v2-umbrella-client-context-creation
  pwright-v2-umbrella-ordered-handle-delivery
  pwright-v2-umbrella-client-locator
  pwright-v2-umbrella-client-locator-screenshot
  pwright-v2-umbrella-client-dom-content
  pwright-v2-umbrella-client-evaluation
  pwright-v2-umbrella-client-navigation
  pwright-v2-umbrella-chromium-content
  pwright-v2-umbrella-chromium-context-lifecycle
  pwright-v2-umbrella-chromium-page-lifecycle
  pwright-v2-umbrella-chromium-page-creation
  pwright-v2-umbrella-chromium-page-events
  pwright-v2-umbrella-chromium-network-events
  pwright-v2-umbrella-chromium-handle-lifecycle
  pwright-v2-umbrella-chromium-handle-evaluation
  pwright-v2-umbrella-chromium-context-creation
  pwright-v2-umbrella-chromium-locator
  pwright-v2-umbrella-chromium-locator-screenshot
  pwright-v2-umbrella-chromium-dom-content
  pwright-v2-umbrella-chromium-evaluation
  pwright-v2-umbrella-chromium-navigation
)

for package in "${profile_packages[@]}"; do
  CARGO_INCREMENTAL=0 cargo check --release --locked \
    --manifest-path "$manifest" \
    --package "$package"
done

cargo package --package pwright --allow-dirty --list >/dev/null

check_profile() {
  local package=$1
  shift
  local tree_file
  local expected_file
  tree_file=$(mktemp)
  expected_file=$(mktemp)
  temporary_files+=("$tree_file" "$expected_file")

  cargo tree \
    --manifest-path "$manifest" \
    --package "$package" \
    --locked \
    --edges normal \
    --prefix none \
    --format '{p}' \
    | awk '$1 == "pwright" || $1 ~ /^pwright-/ { print $1 }' \
    | sort -u >"$tree_file"
  printf '%s\n' "$@" | sort -u >"$expected_file"

  if ! diff -u "$expected_file" "$tree_file"; then
    echo "umbrella dependency proof failed for $package" >&2
    exit 1
  fi
}

check_profile \
  pwright-v2-umbrella-empty \
  pwright \
  pwright-v2-umbrella-empty

check_profile \
  pwright-v2-umbrella-client-content \
  pwright \
  pwright-browser-model \
  pwright-cap-content \
  pwright-client \
  pwright-client-content \
  pwright-core \
  pwright-screenshot-model \
  pwright-v2-umbrella-client-content

check_profile \
  pwright-v2-umbrella-client-context-lifecycle \
  pwright \
  pwright-browser-model \
  pwright-cap-context-lifecycle \
  pwright-client \
  pwright-client-context-lifecycle \
  pwright-core \
  pwright-v2-umbrella-client-context-lifecycle

check_profile \
  pwright-v2-umbrella-client-page-lifecycle \
  pwright \
  pwright-browser-model \
  pwright-cap-page-lifecycle \
  pwright-client \
  pwright-client-page-lifecycle \
  pwright-core \
  pwright-v2-umbrella-client-page-lifecycle

check_profile \
  pwright-v2-umbrella-client-page-creation \
  pwright \
  pwright-browser-model \
  pwright-cap-page-creation \
  pwright-cap-page-lifecycle \
  pwright-client \
  pwright-client-page-creation \
  pwright-client-page-lifecycle \
  pwright-core \
  pwright-v2-umbrella-client-page-creation

check_profile \
  pwright-v2-umbrella-client-page-events \
  pwright \
  pwright-browser-model \
  pwright-cap-page-events \
  pwright-client \
  pwright-client-events \
  pwright-client-page-events \
  pwright-client-wire \
  pwright-codec-page-events \
  pwright-connection-control \
  pwright-core \
  pwright-transport \
  pwright-v2-umbrella-client-page-events

check_profile \
  pwright-v2-umbrella-client-network-events \
  pwright \
  pwright-browser-model \
  pwright-cap-network-events \
  pwright-client \
  pwright-client-events \
  pwright-client-network-events \
  pwright-client-wire \
  pwright-codec-binary \
  pwright-codec-network-events \
  pwright-connection-control \
  pwright-core \
  pwright-network-model \
  pwright-transport \
  pwright-v2-umbrella-client-network-events

check_profile \
  pwright-v2-umbrella-client-handle-lifecycle \
  pwright \
  pwright-browser-model \
  pwright-cap-handle-lifecycle \
  pwright-client \
  pwright-client-handle-lifecycle \
  pwright-core \
  pwright-v2-umbrella-client-handle-lifecycle

check_profile \
  pwright-v2-umbrella-client-handle-evaluation \
  pwright \
  pwright-browser-model \
  pwright-cap-evaluation \
  pwright-cap-handle-evaluation \
  pwright-cap-handle-lifecycle \
  pwright-client \
  pwright-client-handle-evaluation \
  pwright-client-handle-lifecycle \
  pwright-core \
  pwright-v2-umbrella-client-handle-evaluation

check_profile \
  pwright-v2-umbrella-client-context-creation \
  pwright \
  pwright-browser-model \
  pwright-cap-context-creation \
  pwright-cap-context-lifecycle \
  pwright-client \
  pwright-client-context-creation \
  pwright-client-context-lifecycle \
  pwright-core \
  pwright-v2-umbrella-client-context-creation

check_profile \
  pwright-v2-umbrella-client-locator \
  pwright \
  pwright-browser-model \
  pwright-cap-locator \
  pwright-client \
  pwright-client-locator \
  pwright-core \
  pwright-input-model \
  pwright-selector-model \
  pwright-v2-umbrella-client-locator

check_profile \
  pwright-v2-umbrella-client-locator-screenshot \
  pwright \
  pwright-browser-model \
  pwright-cap-locator-screenshot \
  pwright-client \
  pwright-client-locator-screenshot \
  pwright-core \
  pwright-screenshot-model \
  pwright-selector-model \
  pwright-v2-umbrella-client-locator-screenshot

check_profile \
  pwright-v2-umbrella-client-dom-content \
  pwright \
  pwright-browser-model \
  pwright-cap-dom-content \
  pwright-client \
  pwright-client-dom-content \
  pwright-core \
  pwright-v2-umbrella-client-dom-content

check_profile \
  pwright-v2-umbrella-client-evaluation \
  pwright \
  pwright-browser-model \
  pwright-cap-evaluation \
  pwright-client \
  pwright-client-evaluation \
  pwright-core \
  pwright-v2-umbrella-client-evaluation

check_profile \
  pwright-v2-umbrella-client-navigation \
  pwright \
  pwright-browser-model \
  pwright-cap-navigation \
  pwright-client \
  pwright-client-navigation \
  pwright-core \
  pwright-v2-umbrella-client-navigation

check_profile \
  pwright-v2-umbrella-chromium-dom-content \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-dom-content \
  pwright-browser-backend \
  pwright-cap-dom-content \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-dom-content \
  pwright-composition-chromium-dom-content \
  pwright-connection-control \
  pwright-core \
  pwright-server-dom-content \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-dom-content

check_profile \
  pwright-v2-umbrella-chromium-content \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-content \
  pwright-browser-backend \
  pwright-cap-content \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-content \
  pwright-codec-screenshot \
  pwright-composition-chromium-content \
  pwright-connection-control \
  pwright-core \
  pwright-screenshot-model \
  pwright-server-content \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-content

check_profile \
  pwright-v2-umbrella-chromium-context-lifecycle \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-context-lifecycle \
  pwright-backend-cdp-topology \
  pwright-browser-model \
  pwright-cap-context-lifecycle \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-context-lifecycle \
  pwright-composition-chromium-context-lifecycle \
  pwright-connection-control \
  pwright-core \
  pwright-runtime \
  pwright-server-context-lifecycle \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-context-lifecycle

check_profile \
  pwright-v2-umbrella-chromium-page-lifecycle \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-page-lifecycle \
  pwright-backend-cdp-topology \
  pwright-browser-model \
  pwright-cap-page-lifecycle \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-page-lifecycle \
  pwright-composition-chromium-page-lifecycle \
  pwright-connection-control \
  pwright-core \
  pwright-runtime \
  pwright-server-page-lifecycle \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-page-lifecycle

check_profile \
  pwright-v2-umbrella-chromium-page-creation \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-page-creation \
  pwright-backend-cdp-page-lifecycle \
  pwright-backend-cdp-topology \
  pwright-browser-model \
  pwright-cap-page-creation \
  pwright-cap-page-lifecycle \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-page-creation \
  pwright-codec-page-lifecycle \
  pwright-composition-chromium-page-creation \
  pwright-composition-chromium-page-lifecycle \
  pwright-connection-control \
  pwright-core \
  pwright-runtime \
  pwright-server-page-creation \
  pwright-server-page-lifecycle \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-page-creation

check_profile \
  pwright-v2-umbrella-chromium-page-events \
  pwright \
  pwright-backend-cdp-page-events \
  pwright-backend-cdp-topology \
  pwright-browser-model \
  pwright-cap-page-events \
  pwright-cdp \
  pwright-codec-page-events \
  pwright-composition-chromium-page-events \
  pwright-connection-control \
  pwright-core \
  pwright-runtime \
  pwright-runtime-control \
  pwright-server-events \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-page-events

check_profile \
  pwright-v2-umbrella-chromium-network-events \
  pwright \
  pwright-backend-cdp-network-events \
  pwright-browser-model \
  pwright-cap-network-events \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-network-events \
  pwright-composition-chromium-network-events \
  pwright-connection-control \
  pwright-core \
  pwright-network-model \
  pwright-runtime \
  pwright-runtime-control \
  pwright-server-events \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-network-events

check_profile \
  pwright-v2-umbrella-chromium-handle-lifecycle \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-handle-lifecycle \
  pwright-cap-handle-lifecycle \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-handle-lifecycle \
  pwright-composition-chromium-handle-lifecycle \
  pwright-connection-control \
  pwright-core \
  pwright-runtime \
  pwright-server-handle-lifecycle \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-handle-lifecycle

check_profile \
  pwright-v2-umbrella-chromium-handle-evaluation \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-evaluation \
  pwright-backend-cdp-handle-evaluation \
  pwright-backend-cdp-handle-lifecycle \
  pwright-browser-backend \
  pwright-browser-model \
  pwright-cap-evaluation \
  pwright-cap-handle-evaluation \
  pwright-cap-handle-lifecycle \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-handle-evaluation \
  pwright-codec-handle-lifecycle \
  pwright-composition-chromium-handle-evaluation \
  pwright-composition-chromium-handle-lifecycle \
  pwright-connection-control \
  pwright-core \
  pwright-runtime \
  pwright-server-handle-evaluation \
  pwright-server-handle-lifecycle \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-handle-evaluation

check_profile \
  pwright-v2-umbrella-chromium-context-creation \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-context-creation \
  pwright-backend-cdp-context-lifecycle \
  pwright-backend-cdp-topology \
  pwright-browser-model \
  pwright-cap-context-creation \
  pwright-cap-context-lifecycle \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-context-creation \
  pwright-codec-context-lifecycle \
  pwright-composition-chromium-context-creation \
  pwright-composition-chromium-context-lifecycle \
  pwright-connection-control \
  pwright-core \
  pwright-runtime \
  pwright-server-context-creation \
  pwright-server-context-lifecycle \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-context-creation

check_profile \
  pwright-v2-umbrella-chromium-locator \
  pwright \
  pwright-backend-cdp-input \
  pwright-backend-cdp-locator \
  pwright-backend-cdp-selector-runtime \
  pwright-cap-locator \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-locator \
  pwright-composition-chromium-locator \
  pwright-connection-control \
  pwright-core \
  pwright-input-model \
  pwright-selector-model \
  pwright-server-locator \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-locator

check_profile \
  pwright-v2-umbrella-chromium-locator-screenshot \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-content \
  pwright-backend-cdp-locator-screenshot \
  pwright-backend-cdp-selector-geometry \
  pwright-backend-cdp-selector-runtime \
  pwright-browser-backend \
  pwright-cap-content \
  pwright-cap-locator-screenshot \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-locator-screenshot \
  pwright-codec-screenshot \
  pwright-codec-selector \
  pwright-composition-chromium-locator-screenshot \
  pwright-connection-control \
  pwright-core \
  pwright-screenshot-model \
  pwright-selector-model \
  pwright-server-locator-screenshot \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-locator-screenshot

check_profile \
  pwright-v2-umbrella-chromium-evaluation \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-evaluation \
  pwright-browser-backend \
  pwright-cap-evaluation \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-evaluation \
  pwright-composition-chromium-evaluation \
  pwright-connection-control \
  pwright-core \
  pwright-server-evaluation \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-evaluation

check_profile \
  pwright-v2-umbrella-ordered-handle-delivery \
  pwright \
  pwright-cap-evaluation \
  pwright-cap-handle-evaluation \
  pwright-cap-handle-lifecycle \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-handle-evaluation \
  pwright-connection-control \
  pwright-core \
  pwright-plugin-ordered-handle-delivery \
  pwright-runtime \
  pwright-runtime-control \
  pwright-server-handle-evaluation \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-ordered-handle-delivery

check_profile \
  pwright-v2-umbrella-chromium-navigation \
  pwright \
  pwright-backend-cdp-common \
  pwright-backend-cdp-navigation \
  pwright-browser-backend \
  pwright-cap-navigation \
  pwright-cdp \
  pwright-codec-binary \
  pwright-codec-common \
  pwright-codec-navigation \
  pwright-composition-chromium-navigation \
  pwright-connection-control \
  pwright-core \
  pwright-server-navigation \
  pwright-server-wire \
  pwright-transport \
  pwright-v2-umbrella-chromium-navigation

empty_features=$(mktemp)
client_content_features=$(mktemp)
client_context_lifecycle_features=$(mktemp)
client_page_lifecycle_features=$(mktemp)
client_page_creation_features=$(mktemp)
client_page_events_features=$(mktemp)
client_network_events_features=$(mktemp)
client_handle_lifecycle_features=$(mktemp)
client_handle_evaluation_features=$(mktemp)
client_context_creation_features=$(mktemp)
ordered_handle_delivery_features=$(mktemp)
client_locator_features=$(mktemp)
client_locator_screenshot_features=$(mktemp)
client_dom_content_features=$(mktemp)
client_evaluation_features=$(mktemp)
client_features=$(mktemp)
chromium_content_features=$(mktemp)
chromium_context_lifecycle_features=$(mktemp)
chromium_page_lifecycle_features=$(mktemp)
chromium_page_creation_features=$(mktemp)
chromium_page_events_features=$(mktemp)
chromium_network_events_features=$(mktemp)
chromium_handle_lifecycle_features=$(mktemp)
chromium_handle_evaluation_features=$(mktemp)
chromium_context_creation_features=$(mktemp)
chromium_locator_features=$(mktemp)
chromium_locator_screenshot_features=$(mktemp)
chromium_dom_content_features=$(mktemp)
chromium_evaluation_features=$(mktemp)
chromium_features=$(mktemp)
temporary_files+=(
  "$empty_features"
  "$client_content_features"
  "$client_context_lifecycle_features"
  "$client_page_lifecycle_features"
  "$client_page_creation_features"
  "$client_page_events_features"
  "$client_network_events_features"
  "$client_handle_lifecycle_features"
  "$client_handle_evaluation_features"
  "$client_context_creation_features"
  "$ordered_handle_delivery_features"
  "$client_locator_features"
  "$client_locator_screenshot_features"
  "$client_dom_content_features"
  "$client_evaluation_features"
  "$client_features"
  "$chromium_content_features"
  "$chromium_context_lifecycle_features"
  "$chromium_page_lifecycle_features"
  "$chromium_page_creation_features"
  "$chromium_page_events_features"
  "$chromium_network_events_features"
  "$chromium_handle_lifecycle_features"
  "$chromium_handle_evaluation_features"
  "$chromium_context_creation_features"
  "$chromium_locator_features"
  "$chromium_locator_screenshot_features"
  "$chromium_dom_content_features"
  "$chromium_evaluation_features"
  "$chromium_features"
)

cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-empty \
  --locked --edges features --prefix none >"$empty_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-content \
  --locked --edges features --prefix none >"$client_content_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-context-lifecycle \
  --locked --edges features --prefix none >"$client_context_lifecycle_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-page-lifecycle \
  --locked --edges features --prefix none >"$client_page_lifecycle_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-page-creation \
  --locked --edges features --prefix none >"$client_page_creation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-page-events \
  --locked --edges features --prefix none >"$client_page_events_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-network-events \
  --locked --edges features --prefix none >"$client_network_events_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-handle-lifecycle \
  --locked --edges features --prefix none >"$client_handle_lifecycle_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-handle-evaluation \
  --locked --edges features --prefix none >"$client_handle_evaluation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-context-creation \
  --locked --edges features --prefix none >"$client_context_creation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-ordered-handle-delivery \
  --locked --edges features --prefix none >"$ordered_handle_delivery_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-locator \
  --locked --edges features --prefix none >"$client_locator_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-locator-screenshot \
  --locked --edges features --prefix none >"$client_locator_screenshot_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-dom-content \
  --locked --edges features --prefix none >"$client_dom_content_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-evaluation \
  --locked --edges features --prefix none >"$client_evaluation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-client-navigation \
  --locked --edges features --prefix none >"$client_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-content \
  --locked --edges features --prefix none >"$chromium_content_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-context-lifecycle \
  --locked --edges features --prefix none >"$chromium_context_lifecycle_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-page-lifecycle \
  --locked --edges features --prefix none >"$chromium_page_lifecycle_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-page-creation \
  --locked --edges features --prefix none >"$chromium_page_creation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-page-events \
  --locked --edges features --prefix none >"$chromium_page_events_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-network-events \
  --locked --edges features --prefix none >"$chromium_network_events_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-handle-lifecycle \
  --locked --edges features --prefix none >"$chromium_handle_lifecycle_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-handle-evaluation \
  --locked --edges features --prefix none >"$chromium_handle_evaluation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-context-creation \
  --locked --edges features --prefix none >"$chromium_context_creation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-locator \
  --locked --edges features --prefix none >"$chromium_locator_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-locator-screenshot \
  --locked --edges features --prefix none >"$chromium_locator_screenshot_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-dom-content \
  --locked --edges features --prefix none >"$chromium_dom_content_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-evaluation \
  --locked --edges features --prefix none >"$chromium_evaluation_features"
cargo tree --manifest-path "$manifest" --package pwright-v2-umbrella-chromium-navigation \
  --locked --edges features --prefix none >"$chromium_features"

if grep -Eq '^pwright feature ' "$empty_features"; then
  echo "empty umbrella profile unexpectedly selected a pwright feature" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-content"' "$client_content_features"; then
  echo "client umbrella profile did not select client-content" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-content|chromium-dom-content|chromium-evaluation|chromium-navigation|dom-content|evaluation|navigation)"' "$client_content_features"; then
  echo "client content profile leaked a CDP, server, DOM, evaluation, or navigation feature" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-context-lifecycle"' "$client_context_lifecycle_features"; then
  echo "client umbrella profile did not select client-context-lifecycle" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|dom-content|evaluation|locator|navigation|runtime)"' "$client_context_lifecycle_features"; then
  echo "client Context-lifecycle profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-page-lifecycle"' "$client_page_lifecycle_features"; then
  echo "client umbrella profile did not select client-page-lifecycle" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|context-|dom-content|evaluation|locator|navigation|runtime)"' "$client_page_lifecycle_features"; then
  echo "client Page-lifecycle profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-page-creation"' "$client_page_creation_features"; then
  echo "client umbrella profile did not select client-page-creation" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-page-lifecycle"' "$client_page_creation_features"; then
  echo "client Page-creation profile did not select its required lifecycle client" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|context-|dom-content|evaluation|locator|navigation|runtime)"' "$client_page_creation_features"; then
  echo "client Page-creation profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-page-events"' "$client_page_events_features"; then
  echo "client umbrella profile did not select client-page-events" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|context-|dom-content|evaluation|locator|navigation|runtime)"' "$client_page_events_features"; then
  echo "client Page-events profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-network-events"' "$client_network_events_features"; then
  echo "client umbrella profile did not select client-network-events" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|context-|dom-content|evaluation|handle-|locator|navigation|page-|runtime)"' "$client_network_events_features"; then
  echo "client Network-events profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-handle-lifecycle"' "$client_handle_lifecycle_features"; then
  echo "client umbrella profile did not select client-handle-lifecycle" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|context-|dom-content|evaluation|locator|navigation|page-|runtime)"' "$client_handle_lifecycle_features"; then
  echo "client Handle-lifecycle profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-handle-evaluation"' "$client_handle_evaluation_features"; then
  echo "client umbrella profile did not select client-handle-evaluation" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-handle-lifecycle"' "$client_handle_evaluation_features"; then
  echo "client Handle-evaluation profile did not select its required lifecycle client" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|context-|dom-content|locator|navigation|page-|runtime)"' "$client_handle_evaluation_features"; then
  echo "client Handle-evaluation profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-context-creation"' "$client_context_creation_features"; then
  echo "client umbrella profile did not select client-context-creation" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-context-lifecycle"' "$client_context_creation_features"; then
  echo "client Context-creation profile did not select its required lifecycle client" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|dom-content|evaluation|locator|navigation|runtime)"' "$client_context_creation_features"; then
  echo "client Context-creation profile leaked CDP, server, runtime, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "ordered-handle-delivery"' "$ordered_handle_delivery_features"; then
  echo "Ordered Handle delivery profile did not select its plugin feature" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "handle-evaluation"' "$ordered_handle_delivery_features"; then
  echo "Ordered Handle delivery profile did not select Handle evaluation" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "runtime"' "$ordered_handle_delivery_features"; then
  echo "Ordered Handle delivery profile did not select runtime ownership" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "transport"' "$ordered_handle_delivery_features"; then
  echo "Ordered Handle delivery profile did not select transport ownership" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|client|client-|content|context-|dom-content|locator|navigation|page-)"' "$ordered_handle_delivery_features"; then
  echo "Ordered Handle delivery profile leaked a browser backend, client, or unrelated capability feature" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-locator"' "$client_locator_features"; then
  echo "client umbrella profile did not select client-locator" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|dom-content|evaluation|locator-screenshot|navigation)"' "$client_locator_features"; then
  echo "client Locator profile leaked CDP, server, or unrelated capability features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-locator-screenshot"' "$client_locator_screenshot_features"; then
  echo "client umbrella profile did not select client-locator-screenshot" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-|content|dom-content|evaluation|navigation)"' "$client_locator_screenshot_features"; then
  echo "client Locator screenshot profile leaked CDP, server, Page screenshot, DOM, evaluation, or navigation features" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-dom-content"' "$client_dom_content_features"; then
  echo "client umbrella profile did not select client-dom-content" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-content|chromium-dom-content|chromium-evaluation|chromium-navigation|content|evaluation|navigation)"' "$client_dom_content_features"; then
  echo "client DOM-content profile leaked a CDP, server, screenshot, evaluation, or navigation feature" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-evaluation"' "$client_evaluation_features"; then
  echo "client umbrella profile did not select client-evaluation" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-evaluation|chromium-navigation|navigation)"' "$client_evaluation_features"; then
  echo "client evaluation umbrella profile leaked a CDP, server, or navigation feature" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "client-navigation"' "$client_features"; then
  echo "client umbrella profile did not select client-navigation" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(cdp|chromium-navigation)"' "$client_features"; then
  echo "client umbrella profile leaked a CDP/server composition feature" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-content"' "$chromium_content_features"; then
  echo "Chromium umbrella profile did not select chromium-content" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-content|client-dom-content|client-evaluation|client-navigation|dom-content|evaluation|navigation|chromium-dom-content|chromium-evaluation|chromium-navigation)"' "$chromium_content_features"; then
  echo "Chromium content profile leaked a client, DOM, evaluation, or navigation feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_content_features"; then
  echo "Chromium content profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-context-lifecycle"' "$chromium_context_lifecycle_features"; then
  echo "Chromium umbrella profile did not select chromium-context-lifecycle" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "runtime"' "$chromium_context_lifecycle_features"; then
  echo "Chromium Context-lifecycle profile did not expose its required runtime boundary" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-dom-content|chromium-evaluation|chromium-locator|chromium-navigation|content|dom-content|evaluation|locator|navigation)"' "$chromium_context_lifecycle_features"; then
  echo "Chromium Context-lifecycle profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_context_lifecycle_features"; then
  echo "Chromium Context-lifecycle profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-page-lifecycle"' "$chromium_page_lifecycle_features"; then
  echo "Chromium umbrella profile did not select chromium-page-lifecycle" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "runtime"' "$chromium_page_lifecycle_features"; then
  echo "Chromium Page-lifecycle profile did not expose its required runtime boundary" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-context|chromium-dom-content|chromium-evaluation|chromium-locator|chromium-navigation|content|context-|dom-content|evaluation|locator|navigation)"' "$chromium_page_lifecycle_features"; then
  echo "Chromium Page-lifecycle profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_page_lifecycle_features"; then
  echo "Chromium Page-lifecycle profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-page-creation"' "$chromium_page_creation_features"; then
  echo "Chromium umbrella profile did not select chromium-page-creation" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-page-lifecycle"' "$chromium_page_creation_features"; then
  echo "Chromium Page-creation profile did not select its required lifecycle composition" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-context|chromium-dom-content|chromium-evaluation|chromium-locator|chromium-navigation|content|context-|dom-content|evaluation|locator|navigation)"' "$chromium_page_creation_features"; then
  echo "Chromium Page-creation profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_page_creation_features"; then
  echo "Chromium Page-creation profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-page-events"' "$chromium_page_events_features"; then
  echo "Chromium umbrella profile did not select chromium-page-events" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "transport"' "$chromium_page_events_features"; then
  echo "Chromium Page-events profile did not expose its transport boundary" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-context|chromium-dom-content|chromium-evaluation|chromium-handle|chromium-locator|chromium-navigation|content|context-|dom-content|evaluation|handle-|locator|navigation)"' "$chromium_page_events_features"; then
  echo "Chromium Page-events profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_page_events_features"; then
  echo "Chromium Page-events profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-network-events"' "$chromium_network_events_features"; then
  echo "Chromium umbrella profile did not select chromium-network-events" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "transport"' "$chromium_network_events_features"; then
  echo "Chromium Network-events profile did not expose its transport boundary" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "cdp"' "$chromium_network_events_features"; then
  echo "Chromium Network-events profile did not expose its CDP batch boundary" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "runtime"' "$chromium_network_events_features"; then
  echo "Chromium Network-events profile did not expose its lifecycle runtime boundary" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-context|chromium-dom-content|chromium-evaluation|chromium-handle|chromium-locator|chromium-navigation|chromium-page|content|context-|dom-content|evaluation|handle-|locator|navigation|page-)"' "$chromium_network_events_features"; then
  echo "Chromium Network-events profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_network_events_features"; then
  echo "Chromium Network-events profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-handle-lifecycle"' "$chromium_handle_lifecycle_features"; then
  echo "Chromium umbrella profile did not select chromium-handle-lifecycle" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "runtime"' "$chromium_handle_lifecycle_features"; then
  echo "Chromium Handle-lifecycle profile did not expose its required runtime boundary" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-context|chromium-dom-content|chromium-evaluation|chromium-locator|chromium-navigation|chromium-page|content|context-|dom-content|evaluation|locator|navigation|page-)"' "$chromium_handle_lifecycle_features"; then
  echo "Chromium Handle-lifecycle profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_handle_lifecycle_features"; then
  echo "Chromium Handle-lifecycle profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-handle-evaluation"' "$chromium_handle_evaluation_features"; then
  echo "Chromium umbrella profile did not select chromium-handle-evaluation" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-handle-lifecycle"' "$chromium_handle_evaluation_features"; then
  echo "Chromium Handle-evaluation profile did not select its required lifecycle composition" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "runtime"' "$chromium_handle_evaluation_features"; then
  echo "Chromium Handle-evaluation profile did not expose its required runtime boundary" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-context|chromium-dom-content|chromium-evaluation|chromium-locator|chromium-navigation|chromium-page|content|context-|dom-content|locator|navigation|page-)"' "$chromium_handle_evaluation_features"; then
  echo "Chromium Handle-evaluation profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_handle_evaluation_features"; then
  echo "Chromium Handle-evaluation profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-context-creation"' "$chromium_context_creation_features"; then
  echo "Chromium umbrella profile did not select chromium-context-creation" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-context-lifecycle"' "$chromium_context_creation_features"; then
  echo "Chromium Context-creation profile did not select its required lifecycle composition" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-dom-content|chromium-evaluation|chromium-locator|chromium-navigation|content|dom-content|evaluation|locator|navigation)"' "$chromium_context_creation_features"; then
  echo "Chromium Context-creation profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_context_creation_features"; then
  echo "Chromium Context-creation profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-locator"' "$chromium_locator_features"; then
  echo "Chromium umbrella profile did not select chromium-locator" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-dom-content|chromium-evaluation|chromium-locator-screenshot|chromium-navigation|content|dom-content|evaluation|locator-screenshot|navigation)"' "$chromium_locator_features"; then
  echo "Chromium Locator profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_locator_features"; then
  echo "Chromium Locator profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-locator-screenshot"' "$chromium_locator_screenshot_features"; then
  echo "Chromium umbrella profile did not select chromium-locator-screenshot" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-selector-geometry"' "$chromium_locator_screenshot_features"; then
  echo "Chromium Locator screenshot profile did not select the first-party selector-geometry plugin" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-|chromium-content|chromium-dom-content|chromium-evaluation|chromium-navigation|content|dom-content|evaluation|navigation)"' "$chromium_locator_screenshot_features"; then
  echo "Chromium Locator screenshot profile leaked a client or unrelated capability feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_locator_screenshot_features"; then
  echo "Chromium Locator screenshot profile selected the aggregate CDP backend" >&2
  exit 1
fi
if grep -Fq 'pwright-cap-locator ' "$chromium_locator_screenshot_features"; then
  echo "Chromium Locator screenshot profile selected public Locator command authority" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-dom-content"' "$chromium_dom_content_features"; then
  echo "Chromium umbrella profile did not select chromium-dom-content" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-dom-content|client-evaluation|client-navigation|evaluation|navigation|chromium-evaluation|chromium-navigation)"' "$chromium_dom_content_features"; then
  echo "Chromium DOM-content profile leaked a client, evaluation, or navigation feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_dom_content_features"; then
  echo "Chromium DOM-content profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-evaluation"' "$chromium_evaluation_features"; then
  echo "Chromium umbrella profile did not select chromium-evaluation" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-evaluation|client-navigation|navigation|chromium-navigation)"' "$chromium_evaluation_features"; then
  echo "Chromium evaluation umbrella profile leaked a client or navigation feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_evaluation_features"; then
  echo "Chromium evaluation umbrella profile selected the aggregate CDP backend" >&2
  exit 1
fi
if ! grep -Fq 'pwright feature "chromium-navigation"' "$chromium_features"; then
  echo "Chromium umbrella profile did not select chromium-navigation" >&2
  exit 1
fi
if grep -Eq 'pwright feature "(client|client-navigation)"' "$chromium_features"; then
  echo "Chromium umbrella profile leaked a client feature" >&2
  exit 1
fi
if grep -Fq 'pwright-backend-cdp ' "$chromium_features"; then
  echo "Chromium navigation profile selected the aggregate CDP backend" >&2
  exit 1
fi

echo "v2 umbrella dependency proofs passed (30 explicit empty, client, Chromium, and ordered Handle-delivery profiles)"
