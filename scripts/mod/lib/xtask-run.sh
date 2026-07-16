#!/usr/bin/env bash
# Run cargo xtask from monorepo root (scripts/mod/lib → ../../../).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
exec cargo run -q -p xtask -- "$@"
