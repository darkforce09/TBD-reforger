#!/usr/bin/env bash
# Monorepo path resolver for mod/scripts/*.sh — source, do not execute.
_MOD_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOD_SCRIPTS="$(cd "$_MOD_LIB/.." && pwd)"
MOD_ROOT="$(cd "$MOD_SCRIPTS/.." && pwd)"

if [ -f "$MOD_ROOT/../shared/tbd-schema/schema/mission.schema.json" ]; then
  MONO_ROOT="$(cd "$MOD_ROOT/.." && pwd)"
  SCHEMA="$MONO_ROOT/shared/tbd-schema"
  WEB="$MONO_ROOT/website"
else
  MONO_ROOT="$MOD_ROOT"
  SCHEMA="$MOD_ROOT/tbd-schema"
  WEB="$MOD_ROOT/website"
fi

# Legacy: many scripts used ROOT for the mod tree.
ROOT="$MOD_ROOT"
DEPLOY_ENV="$MOD_SCRIPTS/deploy.env"
