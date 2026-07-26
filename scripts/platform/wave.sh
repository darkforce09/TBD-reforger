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

# See note 1. Every worktree build must land in the MAIN repo's target dir.
#
# `$ROOT` is this script's own repo — which inside a worktree IS the worktree, so defaulting to
# "$ROOT/target" pointed each slice at its own target and defeated the entire mitigation. Resolve
# the main checkout instead: --git-common-dir is shared by every worktree and points at the main
# repo's .git, so its parent is the main working tree.
_git_common="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null || echo "$ROOT/.git")"
MAIN_ROOT="$(dirname "$_git_common")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$MAIN_ROOT/target}"

# The container's glibc (2.36) is older than the host's (2.43), so binaries built on the host —
# including target/debug/xtask — refuse to run in here. Route those through the host when we can.
#
# MEASURED 2026-07-26: `distrobox-host-exec` does NOT forward the environment.
#   $ FOO=bar distrobox-host-exec sh -c 'echo [$FOO]'  ->  []
# So the `export CARGO_TARGET_DIR` above is invisible to cargo on the host, and every worktree
# silently builds its own target/ — 1.4 GB within 25 s of a single `cargo check`, ~44 GB for a full
# build. Eight worktrees would exhaust 129 GB of free disk around the third slice, and every gate
# after that fails with a No-space error that reads exactly like a compile error.
# It must be passed explicitly through `env`.
# The timeout lives HERE, not in run(). Two reasons: `command -v` matches shell functions, so a
# run()-level wrapper tried to `timeout hostrun` and failed outright; and wrapping on this side
# kills the actual host process rather than just severing the bridge and orphaning a cargo build.
GATE_TIMEOUT="${TBD_GATE_TIMEOUT:-1200}"
if command -v distrobox-host-exec >/dev/null 2>&1; then
  hostrun() { distrobox-host-exec timeout "$GATE_TIMEOUT" env "CARGO_TARGET_DIR=$CARGO_TARGET_DIR" "$@"; }
else
  hostrun() { timeout "$GATE_TIMEOUT" "$@"; }
fi

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

# committed | dirty | absent | unknown
#
# This is the guard that stops `land` merging a slice an agent is still writing into, so a silent
# failure here is a correctness bug, not an inconvenience: swallowing the error with 2>/dev/null and
# testing for empty output makes a FAILED status indistinguishable from a CLEAN one, and the
# half-finished slice merges. Verified 2026-07-26 that bare `status --porcelain` is unaffected by the
# missing git-lfs (only `add`/`stash` run the clean filters), but check the exit status anyway —
# `land` treats anything that is not `committed` as not-ready.
tree_state() {
  local d="$WORKTREES/$1" out rc
  [ -d "$d" ] || { echo absent; return; }
  # git-lfs is installed neither in the container nor on the host, and `status` runs the clean
  # filter to re-hash modified files. In a worktree that has touched anything LFS-adjacent this
  # aborts with `git-lfs filter-process: not found` / `fatal: the remote end hung up unexpectedly`
  # and exit 128 — OBSERVED on slice/T-192 mid-run. Neutralise the filters for this read-only check,
  # exactly as slice-worktree.sh:19-31 already does for the same reason.
  out="$(git -C "$d" -c filter.lfs.process= -c filter.lfs.clean=cat -c filter.lfs.smudge=cat \
         -c filter.lfs.required=false status --porcelain 2>/dev/null)"; rc=$?
  if [ "$rc" -ne 0 ]; then echo unknown; return; fi
  if [ -n "$out" ]; then echo dirty; else echo committed; fi
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
    elif [ "$st" = unknown ]; then
      printf "  %-9s ⚠ STATUS UNREADABLE — will not land\n" "$t"
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
# The edition is NOT fixed across this workspace: apps/website/api is edition 2024, most other
# crates are 2021, and the two style editions sort a mixed-case brace import differently. Hardcoding
# --edition 2021 made every slice touching an edition-2024 file fail a gate it did not cause — main's
# own `use axum::http::{HeaderMap, HeaderValue, StatusCode, header};` already fails the 2021 form.
# Resolve each file's edition from the nearest Cargo.toml above it.
file_edition() {
  local d; d="$(dirname "$1")"
  while [ "$d" != "." ] && [ "$d" != "/" ]; do
    if [ -f "$d/Cargo.toml" ]; then
      local e; e="$(grep -m1 '^edition' "$d/Cargo.toml" | tr -dc '0-9')"
      [ -n "$e" ] && { echo "$e"; return; }
    fi
    d="$(dirname "$d")"
  done
  echo 2021
}

# Takes the diff base. Defaults to main...HEAD, which is correct inside a WORKTREE (the slice gate)
# and EMPTY on merged main (the wave gate) — so without an explicit base this silently checked
# nothing exactly where it mattered most. It hid a real rustfmt violation in mission_compile.rs
# through five consecutive green wave gates.
fmt_changed() {
  local base="${1:-main...HEAD}" files f ed rc=0
  # Union of COMMITTED and WORKING-TREE changes. Diffing the base alone means an agent running the
  # slice gate before committing gets "no Rust files changed" and a vacuous PASS — observed on both
  # T-182 and T-185, where the same gate went red the moment the work was committed. A gate that
  # only works if you already did the right thing is not a gate.
  files="$( { git diff --name-only "$base" 2>/dev/null
              git status --porcelain 2>/dev/null | sed 's/^...//'
            } | grep '\.rs$' | sort -u || true)"
  [ -z "$files" ] && { echo "no Rust files changed"; return 0; }
  for f in $files; do
    [ -f "$f" ] || continue
    ed="$(file_edition "$f")"
    hostrun rustfmt --edition "$ed" --check "$f" || rc=1
  done
  return "$rc"
}

# Native `cargo check --workspace` does NOT compile the frontend: apps/website/frontend/src is
# `#![cfg(target_arch = "wasm32")]`, so a native check walks straight past it and reports PASS on a
# file it never looked at. T-188 hit exactly this. Any slice touching the frontend must be checked
# for wasm32 or the gate is decorative. Warm cost measured: 0.16s.
wasm_changed() {
  local base="${1:-main...HEAD}"
  # Same union as fmt_changed, for the same reason.
  { git diff --name-only "$base" 2>/dev/null
    git status --porcelain 2>/dev/null | sed 's/^...//'
  } | grep -q '^apps/website/frontend/' || { echo "frontend untouched"; return 0; }
  hostrun cargo check -p website-frontend --target wasm32-unknown-unknown --quiet
}

# Force cargo to actually recompile what this slice changed.
#
# The shared CARGO_TARGET_DIR is necessary (a per-worktree target is ~44 GB) but it lets cargo hand
# one worktree an artifact built from ANOTHER worktree's source. OBSERVED by T-193: `cargo test`
# reported 113 passing from a binary that did not contain its new tests, and `--list` showed main's
# 15 eden_chrome tests rather than its own 18. Touching the source forced a real rebuild and the
# true numbers appeared.
#
# That means a slice gate can print PASS on source it never compiled — which makes every other
# check in this file advisory. Bumping mtime on the changed files invalidates the fingerprint.
touch_changed() {
  local base="${1:-main...HEAD}" f
  for f in $( { git diff --name-only "$base" 2>/dev/null
                git status --porcelain 2>/dev/null | sed 's/^...//'
              } | grep '\.rs$' | sort -u ); do
    [ -f "$f" ] && touch "$f"
  done
}

# Cheap gate — what a slice agent runs before reporting done. Target: ~10 s warm.
gate_slice() {
  local tid="${1:-}"
  echo "═══ slice gate ${tid} ═══"
  touch_changed
  local fail=0
  run() { local l="$1"; shift; printf "  %-24s " "$l"
    if out="$("$@" 2>&1)"; then echo PASS; else echo FAIL; printf '%s\n' "$out" | tail -15 | sed 's/^/      /'; fail=1; fi; }
  run "cargo check"  hostrun cargo check --workspace --quiet
  run "wasm32 (frontend)" wasm_changed
  run "fmt (changed)" fmt_changed
  echo
  [ "$fail" -ne 0 ] && { echo "SLICE GATE: FAIL"; return 1; }
  echo "SLICE GATE: PASS"
}

# Full gate — runs once per wave on merged main.
#
# Takes the wave's BASE commit (the SHA main was at before this wave's merges). Two things depend
# on it, and getting it wrong is silent:
#   * the frontend check. It used to diff HEAD~1..HEAD, which after landing N slices sees only the
#     LAST merge — so a frontend-touching slice merged first, followed by a backend slice, skipped
#     the trunk build entirely and a frontend regression landed green.
#   * anything else that needs to reason about "what this wave changed".
# With no base argument it falls back to HEAD~1, which is correct only for a single-slice wave.
cmd_gate() {
  local base="${1:-HEAD~1}"
  echo "═══ platform wave gate (base ${base:0:12}) ═══"
  touch_changed "$base..HEAD"
  local fail=0
  # hostrun applies the timeout host-side; run() only has to report 124 distinctly from a real fail.
  run() {
    local l="$1"; shift
    printf "  %-24s " "$l"
    out="$("$@" 2>&1)"; local rc=$?
    if [ "$rc" -eq 0 ]; then echo PASS
    elif [ "$rc" -eq 124 ]; then echo "FAIL (TIMEOUT after ${GATE_TIMEOUT}s)"; fail=1
    else echo FAIL; printf '%s\n' "$out" | tail -15 | sed 's/^/      /'; fail=1; fi
  }
  run "cargo check"      hostrun cargo check --workspace --quiet
  run "wasm32 (frontend)" wasm_changed "$base..HEAD"
  run "fmt (changed)"    fmt_changed "$base..HEAD"
  # Clippy is scoped to the crates CI actually gates, NOT --workspace.
  #
  # `cargo clippy --workspace --all-targets -- -D warnings` is red on clean main — ~45 errors, almost
  # all in tools/tbd-tools and xtask, which have never been clippy-gated (exactly as they were never
  # fmt-gated; that drift is T-297). A workspace-wide gate would therefore be red before a single
  # slice merged, and nothing could ever land. ci.yml gates per-crate (:59 website-api, :91
  # map-engine, :112 website-frontend on wasm32) and this mirrors it.
  run "clippy api"       hostrun cargo clippy -p website-api --all-targets --quiet -- -D warnings
  run "clippy map-engine" hostrun cargo clippy -p map-engine-core -p map-engine-render --all-targets --quiet -- -D warnings
  # NOTE: no `-D warnings` here, deliberately — ci.yml:113 runs frontend clippy WITHOUT it, so
  # warnings are advisory upstream and there are 25 of them on clean main. Adding -D here would make
  # the gate stricter than CI and red on arrival. The weakness is real but it is not this run's to
  # fix; filed separately.
  run "clippy frontend"  hostrun cargo clippy -p website-frontend --target wasm32-unknown-unknown --quiet
  # Scoped to CI's crates for the same reason clippy is: `cargo test --workspace` pulls in
  # tools/tbd-tools, which CI never tests and which has a FAILING test on clean main
  # (density::tests::corner_partition_identity — pre-existing, filed as its own ticket). A gate that
  # is red before any slice merges is a gate nothing can ever pass. ci.yml:68 tests website-api,
  # :115 tests website-frontend; map-engine is covered by its own job.
  run "test api"         hostrun cargo test -p website-api --quiet
  # --features mission is REQUIRED. The mission module is feature-gated, so a bare
  # `cargo test -p map-engine-core` runs 116 tests and silently skips 26 — every test in flatten.rs,
  # which is the most contended file in the backlog and the one T-182 inverted a pinning assertion
  # in last wave. Measured 2026-07-26: bare 116, --features mission 142. Found by T-183's agent.
  run "test map-engine"  hostrun cargo test -p map-engine-core --features mission -p map-engine-render --quiet
  # Frontend tests get a PRIVATE target dir. Two agents (T-193, T-195) independently proved that
  # with the shared CARGO_TARGET_DIR, `cargo test -p website-frontend` runs a stale
  # website_frontend-<hash> test binary built from ANOTHER worktree: T-193 saw 113 passing from a
  # binary lacking its new tests; T-195 hit it twice and had to use a private dir to get true
  # numbers. Same package name + version across worktrees = same artifact hash = clobbering.
  # A silent PASS on code that was never compiled makes every other check advisory, so this one
  # step is worth the extra disk. Builds only this crate's tree, not the 609-crate workspace.
  run "test frontend"    hostrun env "CARGO_TARGET_DIR=$MAIN_ROOT/target-gate-frontend" \
                                  cargo test -p website-frontend --quiet
  # The Leptos build is the single most expensive gate (2-6 min warm). Wave-level only, and only
  # when the wave actually touched the frontend — measured across the WHOLE wave, not the last merge.
  if git diff --name-only "$base..HEAD" 2>/dev/null | grep -q '^apps/website/frontend/'; then
    run "trunk build"    hostrun make ci-local-leptos
  else
    printf "  %-24s SKIP (frontend untouched this wave)\n" "trunk build"
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

  # The base is the last known-GREEN main. It is the gate's diff anchor and the revert target.
  local base; base="$(git rev-parse HEAD)"
  echo "wave base: $base"

  local landed=()
  for t in "${ready[@]}"; do
    echo "── landing $t: $(ticket_title "$t")"
    if git merge --no-ff "slice/$t" -m "$t: $(ticket_title "$t")"; then
      landed+=("$t")
    else
      echo "  MERGE FAILED — resolve by hand, then re-run land"
      echo "  (nothing dropped; every worktree is intact)"
      return 1
    fi
  done

  echo
  echo "landed ${#landed[@]} slice(s). Running the wave gate on merged main:"
  if ! cmd_gate "$base"; then
    # DO NOT DROP. slice-worktree.sh drop is `worktree remove --force` + `branch -D`, so dropping
    # here would destroy the tree and branch of every slice in the wave BEFORE anyone can see which
    # one broke it — the exact failure the T-181 reap incident (643c5233) was fixed to prevent, and
    # which this script originally reproduced by dropping inside the merge loop.
    echo "GATE RED AFTER MERGE — all ${#landed[@]} worktree(s) KEPT for inspection: ${landed[*]}"
    echo "  fix on main and re-run:  bash scripts/platform/wave.sh gate $base"
    echo "  or roll back the wave :  bash scripts/platform/wave.sh revert $base"
    return 1
  fi

  # Green. Only now is it safe to destroy the evidence.
  local t2
  for t2 in "${landed[@]}"; do
    bash scripts/mod/slice-worktree.sh drop "$t2" || echo "  (drop failed for $t2 — remove by hand)"
  done

  # Rule 5: work must not be trapped on one machine. This was missing entirely.
  cmd_push || echo "PUSH FAILED — work is landed on local main but not on origin"

  [ "${#blocked[@]}" -gt 0 ] && echo "still in flight: ${blocked[*]}"
  return 0
}

# Roll main back to a known-green commit, keeping the slice branches alive.
#
# The bounded-rollback half of self-healing: when a wave cannot be fixed within its retry budget,
# main returns to green and the offending slices are quarantined rather than left broken. Uses
# `revert`, never `reset --hard` — main is pushed, so history must not be rewritten.
cmd_revert() {
  local base="${1:-}"
  [ -z "$base" ] && { echo "usage: wave.sh revert <known-green-sha>"; return 1; }
  git rev-parse --verify "$base^{commit}" >/dev/null 2>&1 || { echo "no such commit: $base"; return 1; }
  local n; n="$(git rev-list --count "$base..HEAD")"
  [ "$n" -eq 0 ] && { echo "already at $base"; return 0; }
  echo "reverting $n commit(s) back to $base"
  local c
  for c in $(git rev-list "$base..HEAD"); do
    if [ "$(git rev-list --parents -n1 "$c" | wc -w)" -gt 2 ]; then
      git revert --no-edit -m 1 "$c" || { echo "revert of merge $c failed — resolve by hand"; return 1; }
    else
      git revert --no-edit "$c" || { echo "revert of $c failed — resolve by hand"; return 1; }
    fi
  done
  echo "main is back at the $base tree. Slice branches were NOT deleted."
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
  gate)   if [ "${2:-}" = "--slice" ]; then gate_slice "${3:-}"; else cmd_gate "${2:-}"; fi ;;
  land)   cmd_land "${2:-}" ;;
  revert) cmd_revert "${2:-}" ;;
  push)   cmd_push ;;
  *) sed -n '2,40p' "$0"; exit 1 ;;
esac
