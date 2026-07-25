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

# committed | dirty | absent
tree_state() {
  local d="$BASE/$1"
  [ -d "$d" ] || { echo absent; return; }
  if [ -n "$(git -C "$d" status --porcelain 2>/dev/null)" ]; then echo dirty; else echo committed; fi
}

# Has this slice's branch got commits main does not have?
has_work() {
  local b="slice/$1"
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
  run "capability"         distrobox-host-exec make verify-capability
  run "oracle citations"   distrobox-host-exec make verify-oracle
  run "no-crf-leak"        distrobox-host-exec make verify-no-crf-leak
  run "ticket registry"    distrobox-host-exec ./scripts/ticket check
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
}

case "${1:-status}" in
  status) cmd_status ;;
  gate)   cmd_gate ;;
  land)   cmd_land ;;
  prep)   cmd_prep "${2:-}" ;;
  *) sed -n '2,20p' "$0"; exit 2 ;;
esac
