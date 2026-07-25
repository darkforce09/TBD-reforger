#!/usr/bin/env bash
# T-181.4 — guard against ORACLE code or assets leaking into the production mod.
# T-181.52 — extended from CRF-only to every read-only oracle lane (now CRF + PlayableSelector).
#
# We keep several third-party frameworks on disk as READ-ONLY oracles. Hundreds of files of
# working code next to a thinner implementation makes copy-paste the path of least resistance,
# so this makes the leak a build failure. Per lane:
#
#   * no <PREFIX> identifier may appear in apps/mod/tbd-framework/** (outside comments)
#   * no GUID that appears in that oracle's own .layout/.et files may appear in ours
#     (vanilla component GUIDs are fine — those are engine facts, not the oracle's)
#
# ── THE LANES, AND WHY THE LICENCE DIFFERENCE MATTERS ───────────────────────────────────
#   crf_framework      Arma Public License. Attribution-bearing, but still READ-NEVER-COPY
#                      for us: we design-mirror and cite, we do not vendor. Lives in-repo at
#                      apps/mod/crf_framework (gitignored).
#   playable_selector  NO LICENCE AT ALL. That is strictly WORSE than APL, not better: with no
#                      licence grant, default copyright applies and we have NO permission to
#                      copy, adapt or redistribute ANY of it. It is a DESIGN-MIRROR ONLY
#                      oracle — read it to understand how a lobby/slot-picker is shaped, then
#                      write our own. Never a line. Lives OUTSIDE the repo (see PS_SRC below).
#
# The oracle index is deliberately symbols-only (no bodies), so it cannot carry code; this
# covers the human/agent shortcut instead.
#
# NOTE ON THE TARGET NAME: the make target stays `verify-no-crf-leak` because wave.sh, the
# Makefile, SLICE_WORKFLOW.md and t181_event_mod_program.md all invoke it by that name and a
# rename would silently drop the gate out of the wave runner. The name is now narrower than
# what it checks; the banner below says so.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MOD="$ROOT/apps/mod/tbd-framework"
CRF="$ROOT/apps/mod/crf_framework"
# PlayableSelector lives outside the repo and is not provisioned by any repo script. Prefer the
# lane symlink a slice worktree gets from slice-worktree.sh; fall back to the operator's checkout.
# Override with TBD_PS_ORACLE on a machine that keeps it elsewhere.
PS_SRC="${TBD_PS_ORACLE:-$HOME/Projects/Archive/Reforger_Lobby/PlayableSelector-main}"
[ -d "$ROOT/apps/mod/playable_selector" ] && PS_SRC="$ROOT/apps/mod/playable_selector"
FAIL=0

#---------------------------------------------------------------------------------------
# <prefix> identifiers in our own code.
#
# The MCP bridge dir is injected dev-only tooling and is gitignored; skip it.
#
# Comments are ALLOWED to name an oracle: citing the source you design-mirrored is exactly the
# practice we want (e.g. "//! CRF_PlayerCharacter.DisableAI port: …" in TBD_SpawnManager.c).
# What must never appear is the prefix in actual code. So drop comment-only lines before judging.
#
# The pattern is anchored on a non-identifier character so a SHORT prefix cannot false-positive
# on a longer word. This matters for PS_: a bare grep would also hit MAPS_, GROUPS_, OPS_, TIPS_.
check_identifier_leak() {
  local label="$1" prefix="$2"
  echo "==> $prefix identifiers in tbd-framework code ($label)"
  local hits
  hits=$(grep -rnE --binary-files=without-match "(^|[^A-Za-z0-9_])${prefix}" "$MOD" \
          --exclude-dir=EnfusionMCP 2>/dev/null \
        | grep -vE '^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*|#)' || true)
  if [ -n "$hits" ]; then
    echo "FAIL: $prefix symbols found in the production mod:"
    printf '%s\n' "$hits" | head -20
    FAIL=1
  else
    echo "  OK (none)"
  fi
}

#---------------------------------------------------------------------------------------
# Asset GUIDs an oracle declares in its own UI/prefab files, reused verbatim in ours.
#
# UI/ and Prefabs/ are found rather than hardcoded because the lanes nest differently:
# crf_framework/UI vs PlayableSelector-main/PlayableSelector/UI.
#
# `find -L` is LOAD-BEARING, not tidiness. Inside a slice worktree EVERY oracle lane is a
# SYMLINK (slice-worktree.sh links them from the main tree / the out-of-repo checkout), and a
# bare `find <symlink>` does not descend into it — it reports the link itself, which is not
# -type d, so the search returns nothing. Measured: without -L this printed a cheerful
# "nothing to compare" for CRF in a worktree while the real comparison never ran. A gate that
# reports OK because it looked at nothing is worse than no gate.
check_guid_leak() {
  local label="$1" oracle="$2"
  echo "==> $label layout/prefab GUIDs reused in tbd-framework"
  if [ ! -d "$oracle" ]; then
    echo "  SKIP — $label not present locally (gitignored / out-of-repo); GUID check is advisory here"
    return 0
  fi

  local asset_dirs
  mapfile -t asset_dirs < <(find -L "$oracle" -maxdepth 2 -type d \( -name UI -o -name Prefabs \) 2>/dev/null)
  if [ "${#asset_dirs[@]}" -eq 0 ]; then
    # Deliberately NOT worded as OK: reaching here means we compared nothing, which is how the
    # symlink bug above hid itself. Say plainly that no comparison happened.
    echo "  SKIP — no UI/ or Prefabs/ dirs under $oracle; NO GUID comparison was made"
    return 0
  fi

  local oracle_guids ours shared
  oracle_guids=$(grep -rhoE '\{[0-9A-F]{16}\}' "${asset_dirs[@]}" 2>/dev/null | sort -u || true)
  ours=$(grep -rhoE '\{[0-9A-F]{16}\}' "$MOD" 2>/dev/null | sort -u || true)
  if [ -z "$oracle_guids" ] || [ -z "$ours" ]; then
    echo "  OK (nothing to compare)"
    return 0
  fi

  shared=$(comm -12 <(printf '%s\n' "$oracle_guids") <(printf '%s\n' "$ours") || true)
  # A GUID we share with an oracle is only a leak if it is NOT a vanilla engine GUID. Every mod
  # legitimately references the same vanilla components — the handoff calls these out explicitly
  # as "vanilla facts and fine to reuse". Measured: all 4 initial CRF hits were vanilla component
  # GUIDs in TBD_PlayerController.et / default.layer, i.e. false positives.
  local game leaks="" g bare
  game="$HOME/.local/share/Steam/steamapps/common/Arma Reforger/addons/data"
  for g in $shared; do
    bare="${g//[\{\}]/}"
    if [ -d "$game" ] && grep -qla "$bare" "$game"/*.pak 2>/dev/null; then
      continue   # present in vanilla -> engine fact, not an oracle leak
    fi
    leaks="$leaks $g"
  done
  if [ -n "${leaks// /}" ]; then
    echo "FAIL: $label-only asset GUIDs reused (not present in vanilla):"
    for g in $leaks; do echo "  $g"; done
    FAIL=1
  else
    echo "  OK (shared GUIDs are all vanilla engine facts)"
  fi
}

check_identifier_leak "CRF, Arma Public License"     "CRF_"
check_identifier_leak "PlayableSelector, NO LICENCE" "PS_"
check_guid_leak       "CRF"                          "$CRF"
check_guid_leak       "PlayableSelector"             "$PS_SRC"

if [ "$FAIL" -ne 0 ]; then
  echo
  echo "Oracles are reference-only. Design-mirror them; never copy them."
  echo "  CRF              — Arma Public License; read, cite, do not vendor."
  echo "  PlayableSelector — NO LICENCE; default copyright, so no permission to copy at all."
  echo "See docs/mod/TBD_MOD_DESIGN.md §2 and docs/mod/SLICE_WORKFLOW.md §Oracle lanes."
  exit 1
fi
echo "no-oracle-leak: PASS (CRF + PlayableSelector)"
