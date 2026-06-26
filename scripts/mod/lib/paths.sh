#!/usr/bin/env bash
# Monorepo path resolver for scripts/mod/*.sh — source, do not execute.
_MOD_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # scripts/mod/lib
MOD_SCRIPTS="$(cd "$_MOD_LIB/.." && pwd)"                  # scripts/mod
MONO_ROOT="$(cd "$MOD_SCRIPTS/../.." && pwd)"              # repo root

MOD_ROOT="$MONO_ROOT/apps/mod"
SCHEMA="$MONO_ROOT/packages/tbd-schema"
WEB="$MONO_ROOT/apps/website"

# Legacy: many scripts used ROOT for the mod tree.
ROOT="$MOD_ROOT"
DEPLOY_ENV="$MONO_ROOT/scripts/deploy/deploy.env"
