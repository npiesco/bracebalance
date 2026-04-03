#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

mkdir -p \
  "$ROOT_DIR/demo/fixtures/session1/mixed"

cp "$ROOT_DIR/test_artifacts/broken_nested_1.ts" \
  "$ROOT_DIR/demo/fixtures/session1/broken_review.ts"
cp "$ROOT_DIR/test_artifacts/broken_nested_1.rs" \
  "$ROOT_DIR/demo/fixtures/session1/broken_review.rs"
cp "$ROOT_DIR/test_artifacts/valid_nested_1.ts" \
  "$ROOT_DIR/demo/fixtures/session1/mixed/valid_sample.ts"
cp "$ROOT_DIR/test_artifacts/broken_nested_2.ts" \
  "$ROOT_DIR/demo/fixtures/session1/mixed/broken_sample.ts"

echo "Reset demo fixtures in $ROOT_DIR/demo/fixtures/session1"
