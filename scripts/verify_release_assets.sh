#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <release-assets-dir>" >&2
  exit 2
fi

ASSET_DIR="$1"
if [[ ! -d "$ASSET_DIR" ]]; then
  echo "error: directory not found: $ASSET_DIR" >&2
  exit 2
fi

shopt -s nullglob
checksum_files=("$ASSET_DIR"/*.sha256)
shopt -u nullglob

if [[ ${#checksum_files[@]} -eq 0 ]]; then
  echo "error: no .sha256 files found in $ASSET_DIR" >&2
  exit 1
fi

failures=0
for checksum_file in "${checksum_files[@]}"; do
  line="$(tr -d '\r' < "$checksum_file" | head -n 1 || true)"
  expected_hash="$(awk '{print $1}' <<<"$line")"
  artifact_name="$(awk '{print $2}' <<<"$line")"

  if [[ -z "$expected_hash" || -z "$artifact_name" ]]; then
    echo "FAIL: invalid checksum format in $(basename "$checksum_file")" >&2
    failures=$((failures + 1))
    continue
  fi

  artifact_path="$ASSET_DIR/$artifact_name"
  if [[ ! -f "$artifact_path" ]]; then
    echo "FAIL: missing artifact for $(basename "$checksum_file"): $artifact_name" >&2
    failures=$((failures + 1))
    continue
  fi

  actual_hash="$(shasum -a 256 "$artifact_path" | awk '{print $1}')"
  if [[ "$actual_hash" != "$expected_hash" ]]; then
    echo "FAIL: checksum mismatch for $artifact_name" >&2
    echo "  expected: $expected_hash" >&2
    echo "  actual:   $actual_hash" >&2
    failures=$((failures + 1))
    continue
  fi

  echo "OK: $artifact_name"
done

if [[ $failures -gt 0 ]]; then
  echo "verification failed: $failures artifact(s) did not validate" >&2
  exit 1
fi

echo "all release artifacts validated"
