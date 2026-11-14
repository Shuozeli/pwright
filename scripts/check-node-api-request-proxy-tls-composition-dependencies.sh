#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"

proof_dir=$(mktemp -d "${TMPDIR:-/tmp}/pwright-proxy-tls-proof.XXXXXX")
trap 'rm -rf "$proof_dir"' EXIT

dependency_closure() {
  local features=$1
  local output=$2
  cargo tree --locked -p pwright-node-addon \
    --no-default-features --features "$features" \
    --edges normal --prefix none --format '{p}' \
    | awk '{ print $1 }' \
    | sort -u >"$output"
}

prove_exact_union() {
  local proxy_feature=$1
  local label=$2
  local certificate_closure="$proof_dir/certificate"
  local proxy_closure="$proof_dir/proxy"
  local combined_closure="$proof_dir/combined"
  local expected_closure="$proof_dir/expected"

  dependency_closure api-request-client-certificates "$certificate_closure"
  dependency_closure "$proxy_feature" "$proxy_closure"
  dependency_closure \
    "api-request-client-certificates,$proxy_feature" \
    "$combined_closure"
  sort -u "$certificate_closure" "$proxy_closure" >"$expected_closure"

  if ! diff -u "$expected_closure" "$combined_closure"; then
    printf '%s composition dependency closure is not the exact feature union\n' \
      "$label" >&2
    exit 1
  fi
}

prove_exact_union api-request-http-proxy 'Client-certificate plus HTTP-proxy'
prove_exact_union api-request-socks5-proxy 'Client-certificate plus SOCKS5-proxy'

CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --no-default-features \
  --features api-request-client-certificates,api-request-http-proxy
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-node-addon \
  --no-default-features \
  --features api-request-client-certificates,api-request-socks5-proxy

echo 'Node proxy TLS composition dependency proof passed (both exact feature unions)'
