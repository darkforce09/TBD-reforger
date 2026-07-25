#!/usr/bin/env bash
# T-181.4 — guard against CRF code or assets leaking into the production mod.
#
# CRF is Arma Public License and sits on disk at apps/mod/crf_framework (gitignored) purely as
# a read-only oracle. 266 files of working code next to an empty implementation makes
# copy-paste the path of least resistance, so this makes the leak a build failure:
#
#   * no `CRF_` identifier may appear in apps/mod/tbd-framework/**
#   * no GUID that appears in CRF's own .layout/.et files may appear in ours
#     (vanilla component GUIDs are fine — those are engine facts, not CRF's)
#
# The oracle index is deliberately symbols-only (no bodies), so it cannot carry code; this
# covers the human/agent shortcut instead.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MOD="$ROOT/apps/mod/tbd-framework"
CRF="$ROOT/apps/mod/crf_framework"
FAIL=0

echo "==> CRF_ identifiers in tbd-framework code"
# The MCP bridge dir is injected dev-only tooling and is gitignored; skip it.
#
# Comments are ALLOWED to name CRF: citing the oracle you design-mirrored is exactly the
# practice we want (e.g. "//! CRF_PlayerCharacter.DisableAI port: …" in TBD_SpawnManager.c).
# What must never appear is CRF_ in actual code. So drop comment-only lines before judging.
HITS=$(grep -rn --binary-files=without-match "CRF_" "$MOD" \
        --exclude-dir=EnfusionMCP 2>/dev/null \
      | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*|#)' || true)
if [ -n "$HITS" ]; then
  echo "FAIL: CRF_ symbols found in the production mod:"
  printf '%s\n' "$HITS" | head -20
  FAIL=1
else
  echo "  OK (none)"
fi

echo "==> CRF layout/prefab GUIDs reused in tbd-framework"
if [ -d "$CRF" ]; then
  # Collect {16-hex} GUIDs declared in CRF's own UI/prefab assets.
  CRF_GUIDS=$(grep -rhoE '\{[0-9A-F]{16}\}' "$CRF/UI" "$CRF/Prefabs" 2>/dev/null | sort -u || true)
  OURS=$(grep -rhoE '\{[0-9A-F]{16}\}' "$MOD" 2>/dev/null | sort -u || true)
  if [ -n "$CRF_GUIDS" ] && [ -n "$OURS" ]; then
    SHARED=$(comm -12 <(printf '%s\n' "$CRF_GUIDS") <(printf '%s\n' "$OURS") || true)
    # A GUID we share with CRF is only a leak if it is NOT a vanilla engine GUID. Both mods
    # legitimately reference the same vanilla components — the handoff calls these out
    # explicitly as "vanilla facts and fine to reuse". Measured: all 4 initial hits were
    # vanilla component GUIDs in TBD_PlayerController.et / default.layer, i.e. false positives.
    GAME="$HOME/.local/share/Steam/steamapps/common/Arma Reforger/addons/data"
    LEAKS=""
    for g in $SHARED; do
      bare="${g//[\{\}]/}"
      if [ -d "$GAME" ] && grep -qla "$bare" "$GAME"/*.pak 2>/dev/null; then
        continue   # present in vanilla -> engine fact, not a CRF leak
      fi
      LEAKS="$LEAKS $g"
    done
    if [ -n "${LEAKS// /}" ]; then
      echo "FAIL: CRF-only asset GUIDs reused (not present in vanilla):"
      for g in $LEAKS; do echo "  $g"; done
      FAIL=1
    else
      echo "  OK (shared GUIDs are all vanilla engine facts)"
    fi
  else
    echo "  OK (nothing to compare)"
  fi
else
  echo "  SKIP — crf_framework not present locally (gitignored); gate is advisory here"
fi

if [ "$FAIL" -ne 0 ]; then
  echo
  echo "CRF is reference-only (Arma Public License). Design-mirror it; never copy it."
  echo "See docs/mod/TBD_MOD_DESIGN.md §2."
  exit 1
fi
echo "no-crf-leak: PASS"
