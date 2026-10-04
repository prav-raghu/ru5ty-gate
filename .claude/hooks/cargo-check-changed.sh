#!/usr/bin/env bash
# Reads the tool input JSON from stdin, formats the edited Rust file with
# rustfmt, finds the owning crate and runs cargo clippy on it.
# Non-zero exit + stderr output surfaces the error back to Claude.

set -euo pipefail

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // .file_path // empty')

if [[ -z "$file_path" || "$file_path" != *.rs || ! -f "$file_path" ]]; then
  exit 0
fi

if [[ "$file_path" == *"/target/"* ]]; then
  exit 0
fi

dir=$(dirname "$file_path")
crate_dir=""
while [[ "$dir" != "/" && "$dir" != "." ]]; do
  if [[ -f "$dir/Cargo.toml" ]] && grep -q '^\[package\]' "$dir/Cargo.toml"; then
    crate_dir="$dir"
    break
  fi
  dir=$(dirname "$dir")
done

if [[ -z "$crate_dir" ]]; then
  exit 0
fi

package=$(sed -n 's/^name = "\(.*\)"/\1/p' "$crate_dir/Cargo.toml" | head -n1)

rustfmt --edition 2024 "$file_path" 2>/dev/null || true

if ! output=$(cargo clippy -p "$package" --all-targets --quiet -- -D warnings 2>&1); then
  echo "Clippy failed in $package:" >&2
  echo "$output" >&2
  exit 2
fi

exit 0
