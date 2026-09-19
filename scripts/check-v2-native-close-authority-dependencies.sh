#!/usr/bin/env bash
set -euo pipefail

selected_feature=${1:-native-close-authority}
case "$selected_feature" in
  native-close-authority|native-close-scheduler) ;;
  *) echo "unsupported native-close selection" >&2; exit 2 ;;
esac

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ordinary=$(mktemp)
selected=$(mktemp)
trap 'rm -f "$ordinary" "$selected"' EXIT
cd "$repo_dir"

cargo tree --locked -p pwright-composition-chromium --no-default-features \
  --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//' | sort -u >"$ordinary"
# Only the root authority feature and its explicitly selected native Page
# confirmation prerequisite may differ. Preserve all other package/features.
cargo tree --locked -p pwright-composition-chromium --no-default-features \
  --features "$selected_feature" --edges normal --prefix none --format '{p} features={f}' \
  | sed '1d; s/ (\*)$//; /^pwright-backend-cdp-page-lifecycle /s/features=default,native-confirmation$/features=default/' \
  | sort -u >"$selected"
diff -u "$ordinary" "$selected"
echo "$selected_feature selects native-confirmation only, with no new normal packages or other transitive features"
