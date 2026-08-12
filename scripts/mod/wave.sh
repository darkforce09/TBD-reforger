#!/usr/bin/env bash
# Wave lifecycle automation — the programmatic form of docs/mod/SLICE_WORKFLOW.md.
#
# WHY THIS EXISTS
# ---------------
# The wave cycle (dispatch 3 → merge → reap → verify → next 3) must not depend on any session
# remembering where it was. This script reads docs/mod/wave_plan.tsv and the live git/worktree
# state and derives the answer, so a fresh session — or one resuming after a context compaction —
# runs `wave.sh status` and knows exactly what to do next.
#
#   bash scripts/mod/wave.sh status     # where are we? what is blocking?
#   bash scripts/mod/wave.sh gate       # run every verification gate (the wave gate)
#   bash scripts/mod/wave.sh land       # merge all complete slices, reap trees, run the gate
#   bash scripts/mod/wave.sh prep N     # create worktrees for wave N
#   bash scripts/mod/wave.sh push       # push main to GitHub (refuses to skip a real LFS push)
#
# `land` is deliberately conservative: it REFUSES to merge a worktree with uncommitted changes,
# and it runs the full gate AFTER merging so a bad slice is caught on main immediately.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
PLAN="docs/mod/wave_plan.tsv"
BASE=".ai/artifacts/worktrees"

plan_rows() { grep -v '^#' "$PLAN" | grep -v '^wave\s' | sed '/^\s*$/d'; }
wave_slices() { plan_rows | awk -F'\t' -v w="$1" '$1==w {print $2}'; }
slice_title() { plan_rows | awk -F'\t' -v s="$1" '$2==s {print $3; exit}'; }

# The current wave = the lowest wave with at least one slice not yet shipped in the registry.
current_wave() {
  local shipped
  shipped="$(python3 -c "
import json
d=json.load(open('.ai/tickets/registry.json'))
t=[x for x in d['tickets'] if x['id']=='T-181'][0]
print(' '.join(k for k,v in t['slice_plan'].items() if v.get('status')=='shipped'))" 2>/dev/null)"
  local w
  for w in $(plan_rows | awk -F'\t' '{print $1}' | sort -un); do
    local s done_all=1
    for s in $(wave_slices "$w"); do
      case " $shipped " in *" $s "*) ;; *) done_all=0 ;; esac
    done
    [ "$done_all" -eq 0 ] && { echo "$w"; return; }
  done
  echo "done"
}

# A sub-slice lives in its PARENT's worktree (SLICE_WORKFLOW.md rule 1): T-181.9.2 -> T-181.9.
# wave.sh must apply the same normalisation slice-worktree.sh does, or it looks for a tree that
# was deliberately never created and reports the slice as permanently absent.
parent_slice() { echo "$1" | sed -E 's/^(T-[0-9]+\.[0-9]+).*/\1/'; }

# committed | dirty | absent
tree_state() {
  local d="$BASE/$(parent_slice "$1")"
  [ -d "$d" ] || { echo absent; return; }
  if [ -n "$(git -C "$d" status --porcelain 2>/dev/null)" ]; then echo dirty; else echo committed; fi
}

# Has this slice's branch got commits main does not have?
has_work() {
  local b="slice/$(parent_slice "$1")"
  git show-ref --verify --quiet "refs/heads/$b" || return 1
  [ -n "$(git log --oneline "main..$b" 2>/dev/null)" ]
}

cmd_status() {
  local w; w="$(current_wave)"
  echo "═══ T-181 wave status ═══"
  if [ "$w" = "done" ]; then
    echo "ALL PLANNED WAVES SHIPPED. Next: extend $PLAN or close the program."
    return 0
  fi
  echo "current wave: $w"
  local s st ready=0 total=0
  for s in $(wave_slices "$w"); do
    st="$(tree_state "$s")"
    total=$((total+1))
    local mark="…"
    if [ "$st" = committed ] && has_work "$s"; then mark="READY"; ready=$((ready+1))
    elif [ "$st" = committed ]; then mark="empty (no commits yet)"
    elif [ "$st" = dirty ]; then mark="DIRTY — agent must commit in its worktree"
    else mark="no worktree — run: wave.sh prep $w"; fi
    printf "  %-12s %-9s %s\n" "$s" "$st" "$mark"
    printf "               %s\n" "$(slice_title "$s")"
  done
  echo
  echo "ready to merge: $ready/$total"
  if [ "$ready" -eq "$total" ] && [ "$total" -gt 0 ]; then
    echo "ACTION: bash scripts/mod/wave.sh land"
  else
    echo "ACTION: wait for slice agents, then re-run status"
  fi
}

cmd_prep() {
  local w="${1:-$(current_wave)}"
  [ "$w" = "done" ] && { echo "nothing to prep"; return 0; }
  local s
  for s in $(wave_slices "$w"); do
    bash scripts/mod/slice-worktree.sh new "$s"
  done
}

cmd_gate() {
  echo "═══ wave gate ═══"
  local fail=0
  run() { # label, command...
    local label="$1"; shift
    printf "  %-26s " "$label"
    if out="$("$@" 2>&1)"; then
      echo "PASS"
    else
      echo "FAIL"
      printf '%s\n' "$out" | tail -12 | sed 's/^/      /'
      fail=1
    fi
  }
  run "compile"            bash scripts/mod/compile.sh
  run "compile-selftest"   distrobox-host-exec make mod-compile-selftest
  # The compile gate is blind to prefab wiring: a component listed on TBD_GameMode.et whose
  # class fails to resolve is dropped SILENTLY and everything still compiles clean.
  # world-boot (cargo xtask mod world-boot) boots the real scenario and asserts the game mode's component roll-call.
  run "world boot"          cargo run -q -p xtask -- mod world-boot
  run "world-boot selftest" cargo run -q -p xtask -- mod world-boot --selftest
  # A bare world-boot proves only that the game mode WIRES UP — the loader refuses with no
  # missionId, so the validator, zone registry, slot materialisation and marker service never
  # run. The wave-5 verifier found two live MAJOR bugs sitting in exactly that blind spot.
  # Seeding the reference mission makes the whole document path a gated one.
  # The other goldens are a manual sweep: cargo xtask mod world-boot --mission=<name>.
  run "world boot +mission" cargo run -q -p xtask -- mod world-boot --mission=bridgehead-at-levie
  # T-181.20 shipped golden mission fixtures whose whole purpose is to make a schema regression
  # fail. Platform CI validates them; the wave gate did not, so a broken fixture landed green
  # here and was only caught later. A slice's own deliverable must be gated by its own wave.
  # A layout only loads when a menu opens, which needs a client — so nothing else in this gate
  # can see a UI regression. This checks the structural invariants that actually broke: an empty
  # slot block (a child with no HorizontalAlign keeps its DESIRED size, which for a Frame is ZERO),
  # slot classes used on the wrong parent, geometry that disagrees with itself, and the
  # FindAnyWidget name contract between layout and script.
  run "ui layouts"         cargo run -q -p xtask -- verify ui-layouts
  run "schema validate"    distrobox-host-exec make schema-validate
  run "capability"         distrobox-host-exec make verify-capability
  run "oracle citations"   distrobox-host-exec make verify-oracle
  run "no-crf-leak"        distrobox-host-exec make verify-no-crf-leak
  run "ticket registry"    distrobox-host-exec cargo run -q -p xtask -- ticket check
  run "enf unit tests"     distrobox-host-exec cargo test -q -p tbd-tools --lib enf::
  echo
  if [ "$fail" -ne 0 ]; then echo "GATE: FAIL"; return 1; fi
  echo "GATE: PASS"
}

cmd_land() {
  local w; w="$(current_wave)"
  [ "$w" = "done" ] && { echo "nothing to land"; return 0; }
  local s st merged=0 skipped=0

  # 1. Refuse to proceed if any tree is dirty — uncommitted slice work would be lost.
  for s in $(wave_slices "$w"); do
    if [ "$(tree_state "$s")" = dirty ]; then
      echo "REFUSING: $s worktree has uncommitted changes." >&2
      git -C "$BASE/$s" status --short >&2
      return 1
    fi
  done

  # 2. Merge every slice that actually has commits.
  for s in $(wave_slices "$w"); do
    if has_work "$s"; then
      echo "── merging $s"
      if bash scripts/mod/slice-worktree.sh merge "$s"; then
        merged=$((merged+1))
      else
        echo "MERGE FAILED for $s — resolve manually, then re-run land" >&2
        return 1
      fi
    else
      echo "── skipping $s (no commits)"
      skipped=$((skipped+1))
    fi
  done
  echo "merged $merged, skipped $skipped"

  # 3. Gate the merged result BEFORE reaping, so a failure is still easy to inspect.
  if ! cmd_gate; then
    echo
    echo "Gate FAILED after merge. Worktrees kept for inspection. Fix on main, re-run: wave.sh gate" >&2
    return 1
  fi

  # 4. Reap only once the gate is green — disk is the constraint (operator instruction).
  echo
  bash scripts/mod/slice-worktree.sh reap

  # 5. Push. Operator instruction: every completed wave goes to GitHub, so the work is not
  #    trapped on one machine.
  echo
  cmd_push
}

# Push main to origin.
#
# --no-verify is deliberate and verified, not lazy: git-lfs is installed on NEITHER the
# container nor the host, and .git/hooks/pre-push exits 2 unconditionally when it is missing,
# which would block every push forever. LFS tracks ONLY packages/map-assets/** (see
# .gitattributes), so we refuse to bypass when a commit actually touches those paths — in that
# case real LFS objects would need uploading and skipping the hook could leave the remote
# referencing objects that were never sent.
cmd_push() {
  echo "═══ push ═══"
  local n; n="$(git log --oneline @{u}..HEAD 2>/dev/null | wc -l | tr -d " ")"
  if [ "${n:-0}" = "0" ]; then echo "  nothing to push"; return 0; fi

  local lfs; lfs="$(git diff --name-only @{u}..HEAD 2>/dev/null | grep -cE "^packages/map-assets/" || true)"
  if [ "${lfs:-0}" != "0" ]; then
    echo "  REFUSING to bypass the LFS hook: $lfs file(s) under packages/map-assets/ are in these" >&2
    echo "  commits and need real LFS objects uploaded. Install git-lfs, then: git push origin main" >&2
    return 1
  fi

  echo "  pushing $n commit(s) (no LFS content — hook bypass is safe)"
  git push --no-verify origin main 2>&1 | tail -4
}

case "${1:-status}" in
  status) cmd_status ;;
  push)   cmd_push ;;
  gate)   cmd_gate ;;
  land)   cmd_land ;;
  prep)   cmd_prep "${2:-}" ;;
  *) sed -n '2,20p' "$0"; exit 2 ;;
esac
