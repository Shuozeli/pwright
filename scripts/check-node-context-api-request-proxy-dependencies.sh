#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

dependency_closure() {
  local features=$1
  cargo tree --locked -p pwright-node-addon \
    --no-default-features \
    --features "$features" \
    --edges normal --prefix none --format '{p}' \
    | awk '{ print $1 }' \
    | sort -u
}

base_file=$(mktemp)
selected_file=$(mktemp)
trap 'rm -f "$base_file" "$selected_file"' EXIT

dependency_closure api-request >"$base_file"
dependency_closure context-api-request,context-proxy,api-request-http-proxy,api-request-socks5-proxy,api-request-proxy-bypass >"$selected_file"

added=$(comm -13 "$base_file" "$selected_file")
removed=$(comm -23 "$base_file" "$selected_file")
expected=$(cat <<'EOF'
aho-corasick
pwright-api-request-connector-http-proxy
pwright-api-request-connector-proxy-bypass
pwright-api-request-connector-socks5
pwright-plugin-chromium-frame-lifecycle
regex
regex-automata
regex-syntax
EOF
)

if [[ -n "$removed" ]]; then
  printf 'Context API-request proxy profile removed base dependencies:\n%s\n' "$removed" >&2
  exit 1
fi

if [[ "$added" != "$expected" ]]; then
  printf 'Unexpected Context API-request proxy dependency delta:\n%s\nExpected:\n%s\n' \
    "$added" "$expected" >&2
  exit 1
fi

CARGO_INCREMENTAL=0 cargo check --release --locked \
  -p pwright-node-addon --no-default-features \
  --features context-api-request,context-proxy,api-request-http-proxy,api-request-socks5-proxy,api-request-proxy-bypass
echo 'Node Context API-request proxy dependency proof passed (exact explicit union)'
