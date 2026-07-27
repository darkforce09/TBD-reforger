#!/usr/bin/env bash
# Build (quietly, cached) + echo the mcpd broker binary path (scripts/mod/lib → ../../../).
# Honors CARGO_TARGET_DIR (default $ROOT/target) — same private-dir contract as T-328 mcp-daemon.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
(cd "$ROOT" && cargo build -q -p tbd-tools --bin mcpd)
echo "$TARGET_DIR/debug/mcpd"
