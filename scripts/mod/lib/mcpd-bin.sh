#!/usr/bin/env bash
# Build (quietly, cached) + echo the mcpd broker binary path (scripts/mod/lib → ../../../).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
(cd "$ROOT" && cargo build -q -p tbd-tools --bin mcpd)
echo "$ROOT/target/debug/mcpd"
