#!/usr/bin/env bash
# Platform wave lifecycle — the programmatic form of docs/platform/PLATFORM_FACTORY.md.
#
# WHY THIS EXISTS SEPARATELY FROM scripts/mod/wave.sh
# ---------------------------------------------------
# Same shape, different physics. The mod program gates on the Enfusion compiler and a real
# headless game boot. This program gates on cargo and trunk. Three things had to change, and
# each is a measured correction to how T-181 ran — not a preference:
#
#   1. SHARED CARGO TARGET DIR.  The mod slices were Enfusion `.c`, so worktrees cost nothing.
#      These slices are Rust. Without CARGO_TARGET_DIR every worktree starts a COLD build of a
#      609-crate workspace; the repo's own target/ is 52 GB. Eight cold worktrees is not a slow
#      wave, it is a dead afternoon. Pointing every tree at one target dir means cargo's lock
#      serialises builds instead — and a warm `cargo check --workspace` is 6.8 s measured, so
#      the wait is cheap and the cache is hot for everyone.
#
#   2. PER-SLICE LANDING, NO WAVE BARRIER.  T-181's rule "merge only when all three complete"
#      cost 89% of its wall clock: mean 64 minutes between lands, on merges that take zero
#      seconds. Finished slices sat blocked behind unfinished ones. Here `land` merges ANY slice
#      that is committed, clean and gate-green, the moment it is ready. `land --wave` keeps the
#      old barrier behaviour if you ever actually want it.
#
#   3. TIERED GATES.  A slice pays only the cheap gate (~10 s). The expensive suite runs once per
#      wave on merged main. `make ci-local` is deliberately NOT used: it has been red for weeks
#      (verify-no-python fails on scripts/mod/slice-collisions.py) and it is 15-40 minutes, not
#      the 22.7 s the docs still claim.
#
#   bash scripts/platform/wave.sh status      # where are we? what is blocking?
#   bash scripts/platform/wave.sh prep        # create worktrees for the next disjoint set
#   bash scripts/platform/wave.sh gate        # full wave gate on the current tree
#   bash scripts/platform/wave.sh gate --slice T-190   # cheap per-slice gate
#   bash scripts/platform/wave.sh land        # merge every ready slice (no barrier)
#   bash scripts/platform/wave.sh push        # push main
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
PLAN="${TBD_WAVE_PLAN:-docs/platform/wave_plan.tsv}"
REGISTRY=".ai/tickets/registry.json"
WORKTREES=".ai/artifacts/worktrees"
COLLIDE="scripts/platform/slice-collisions.py"

# See note 1. Every worktree build lands in the root target dir.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

# The container's glibc (2.36) is older than the host's (2.39), so binaries built on the host —
# including target/debug/xtask — refuse to run in here. Route those through the host when we can.
if command -v distrobox-host-exec >/dev/null 2>&1; then hostrun() { distrobox-host-exec "$@"; }
else hostrun() { "$@"; }; fi

plan_rows() { grep -v '^#' "$PLAN" 2>/dev/null | grep -v '^wave[[:space:]]' | sed '/^\s*$/d'; }
ticket_title() { plan_rows | awk -F'\t' -v s="$1" '$2==s {print $3; exit}'; }
ticket_owns()  { plan_rows | awk -F'\t' -v s="$1" '$2==s {print $4; exit}'; }
ticket_wave()  { plan_rows | awk -F'\t' -v s="$1" '$2==s {print $1; exit}'; }

is_shipped() {
  python3 - "$1" <<'EOF' 2>/dev/null
import json,sys
r=json.load(open('.ai/tickets/registry.json'))
t=[x for x in r['tickets'] if x['id']==sys.argv[1]]
sys.exit(0 if (t and t[0]['status'] in ('shipped','cancelled')) else 1)
EOF
}

# The lowest wave with at least one unshipped ticket.
current_wave() {
  local w t last=""
  while IFS=$'\t' read -r w t _; do
    [ "$w" = "0" ] && continue
    if ! is_shipped "$t"; then echo "$w"; return; fi
    last="$w"
  done < <(plan_rows | sort -n -k1,1)
  echo "done"
}

wave_tickets() { plan_rows | awk -F'\t' -v w="$1" '$1==w {print $2}'; }

# committed | dirty | absent
tree_state() {
  local d="$WORKTREES/$1"
  [ -d "$d" ] || { echo absent; return; }
  if [ -n "$(git -C "$d" status --porcelain 2>/dev/null)" ]; then echo dirty; else echo committed; fi
}
has_work() { [ "$(git rev-list --count "main..slice/$1" 2>/dev/null || echo 0)" -gt 0 ]; }

cmd_status() {
  local w; w="$(current_wave)"
  echo "═══ platform program ═══"
  echo "plan:  $PLAN"
  local total open
  total="$(plan_rows | awk -F'\t' '$1!="0"' | wc -l)"
  open=0
  while IFS=$'\t' read -r _ t _; do is_shipped "$t" || open=$((open+1)); done < <(plan_rows | awk -F'\t' '$1!="0"')
  echo "open:  $open / $total tickets"
  [ "$w" = "done" ] && { echo "ALL WAVES COMPLETE"; return 0; }
  echo "wave:  $w"
  echo
  local t st ready=0
  for t in $(wave_tickets "$w"); do
    is_shipped "$t" && { printf "  %-9s SHIPPED\n" "$t"; continue; }
    st="$(tree_state "$t")"
    if [ "$st" = committed ] && has_work "$t"; then
      printf "  %-9s READY TO LAND  %s\n" "$t" "$(ticket_title "$t")"; ready=$((ready+1))
    elif [ "$st" = committed ]; then
      printf "  %-9s tree clean, no commits yet\n" "$t"
    elif [ "$st" = dirty ]; then
      printf "  %-9s IN PROGRESS (uncommitted)\n" "$t"
    else
      printf "  %-9s not started\n" "$t"
    fi
  done
  echo
  [ "$ready" -gt 0 ] && echo "→ $ready slice(s) ready: bash scripts/platform/wave.sh land"
  echo "→ dispatch set: python3 $COLLIDE"
}

cmd_prep() {
  echo "next disjoint dispatch set:"
  python3 "$COLLIDE"
  echo
  echo "create trees with:  bash scripts/mod/slice-worktree.sh new <TICKET>"
  echo "(slice-worktree.sh is program-agnostic; it keys off the branch name only)"
}

# Format ONLY the files this slice changed against main.
#
# `cargo fmt --all --check` is deliberately NOT used: 32 files are already unformatted on main
# (mostly tools/tbd-tools/src/bin/enf.rs, written during T-181 and never formatted), so a
# workspace-wide check would be red on day one for every agent — the precise anti-pattern that
# made verify-no-python worthless. Scope it to the diff and the gate stays honest.
fmt_changed() {
  local files
  files="$(git diff --name-only main...HEAD 2>/dev/null | grep '\.rs$' || true)"
  [ -z "$files" ] && { echo "no Rust files changed"; return 0; }
  # shellcheck disable=SC2086
  hostrun rustfmt --edition 2021 --check $files
}

# Cheap gate — what a slice agent runs before reporting done. Target: ~10 s warm.
gate_slice() {
  local tid="${1:-}"
  echo "═══ slice gate ${tid} ═══"
  local fail=0
  run() { local l="$1"; shift; printf "  %-24s " "$l"
    if out="$("$@" 2>&1)"; then echo PASS; else echo FAIL; printf '%s\n' "$out" | tail -15 | sed 's/^/      /'; fail=1; fi; }
  run "cargo check"  hostrun cargo check --workspace --quiet
  run "fmt (changed)" fmt_changed
  echo
  [ "$fail" -ne 0 ] && { echo "SLICE GATE: FAIL"; return 1; }
  echo "SLICE GATE: PASS"
}

# Full gate — runs once per wave on merged main.
cmd_gate() {
  echo "═══ platform wave gate ═══"
  local fail=0
  run() { local l="$1"; shift; printf "  %-24s " "$l"
    if out="$("$@" 2>&1)"; then echo PASS; else echo FAIL; printf '%s\n' "$out" | tail -15 | sed 's/^/      /'; fail=1; fi; }
  run "cargo check"      hostrun cargo check --workspace --quiet
  run "fmt (changed)"    fmt_changed
  run "cargo clippy"     hostrun cargo clippy --workspace --all-targets --quiet -- -D warnings
  run "cargo test"       hostrun cargo test --workspace --quiet
  # The Leptos build is the single most expensive gate (2-6 min warm). Wave-level only, and only
  # when the wave actually touched the frontend.
  if git diff --name-only HEAD~1..HEAD 2>/dev/null | grep -q '^apps/website/frontend/'; then
    run "trunk build"    hostrun make ci-local-leptos
  else
    printf "  %-24s SKIP (frontend untouched)\n" "trunk build"
  fi
  run "ticket registry"  hostrun ./scripts/ticket check
  echo
  [ "$fail" -ne 0 ] && { echo "GATE: FAIL"; return 1; }
  echo "GATE: PASS"
}

# Land every slice that is ready. No barrier — see note 2.
cmd_land() {
  local barrier=0
  [ "${1:-}" = "--wave" ] && barrier=1
  local w; w="$(current_wave)"
  [ "$w" = "done" ] && { echo "nothing to land"; return 0; }

  local t st ready=() blocked=()
  for t in $(wave_tickets "$w"); do
    is_shipped "$t" && continue
    st="$(tree_state "$t")"
    if [ "$st" = committed ] && has_work "$t"; then ready+=("$t")
    else blocked+=("$t"); fi
  done

  if [ "${#ready[@]}" -eq 0 ]; then echo "no slice is ready to land"; return 0; fi
  if [ "$barrier" -eq 1 ] && [ "${#blocked[@]}" -gt 0 ]; then
    echo "--wave: holding ${#ready[@]} ready slice(s) for ${#blocked[@]} unfinished: ${blocked[*]}"
    echo "(this is the T-181 barrier that cost 89% of wall clock — omit --wave to land now)"
    return 0
  fi

  local merged=0
  for t in "${ready[@]}"; do
    echo "── landing $t: $(ticket_title "$t")"
    if git merge --no-ff "slice/$t" -m "$t: $(ticket_title "$t")"; then
      merged=$((merged+1))
      bash scripts/mod/slice-worktree.sh drop "$t" || echo "  (worktree drop failed — remove by hand)"
    else
      echo "  MERGE FAILED — resolve by hand, then re-run land"
      return 1
    fi
  done
  echo
  echo "landed $merged slice(s). Running the wave gate on merged main:"
  cmd_gate || { echo "GATE RED AFTER MERGE — fix on main before dispatching more"; return 1; }
  [ "${#blocked[@]}" -gt 0 ] && echo "still in flight: ${blocked[*]}"
  return 0
}

cmd_push() {
  if git diff --name-only origin/main..HEAD | grep -q '^packages/map-assets/'; then
    echo "REFUSING --no-verify: this range touches packages/map-assets/ (the only LFS path)."
    echo "Install git-lfs and push normally, or the remote will reference objects never uploaded."
    return 1
  fi
  git push --no-verify origin main
}

case "${1:-status}" in
  status) cmd_status ;;
  prep)   cmd_prep ;;
  gate)   if [ "${2:-}" = "--slice" ]; then gate_slice "${3:-}"; else cmd_gate; fi ;;
  land)   cmd_land "${2:-}" ;;
  push)   cmd_push ;;
  *) sed -n '2,40p' "$0"; exit 1 ;;
esac
