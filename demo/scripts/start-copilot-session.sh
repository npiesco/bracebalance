#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PROMPT='Inspect demo/fixtures/session1/broken_review.ts in this repo. Use the bracebalance MCP server tools first to identify the structural delimiter errors, then use the CLI to verify the diagnosis, apply the smallest correct fix, and confirm the file is balanced.'

cd "$ROOT_DIR"

exec copilot -i "$PROMPT"
