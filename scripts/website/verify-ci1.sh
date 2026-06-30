#!/usr/bin/env bash
set -euo pipefail
# CI-1 (CODING_STANDARDS.md §0.3): contracts.yml must not use only-new-issues.
if grep -q 'only-new-issues: true' .github/workflows/contracts.yml; then
  echo "CI-1: remove only-new-issues: true from .github/workflows/contracts.yml" >&2
  exit 1
fi
