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

# `$0` IS THE SHELL, NOT THIS FILE, when the script is sourced or piped — read before simplifying.
#
# MEASURED 2026-07-26: `bash -c 'source .../wave.sh status'` from a scratch directory printed
# `open: 0 / 0 tickets` and `ALL WAVES COMPLETE` about a directory that is not the repo, because
# `$0` was `bash`, `dirname` was `.`, and ROOT became `cwd/../..`.
#
# Blast radius was `status`/`wave` only and the GATE was never affected — MAIN_ROOT below comes from
# `git rev-parse --git-common-dir`, and cmd_gate refuses at rev-parse/refuse_empty_range before
# take_gate_lock when git does not resolve — so there is no path where the gate locked the wrong
# place and reported PASS on the wrong tree. It is fixed anyway, and asserted rather than assumed,
# because "a tool reporting confidently on an input it never examined" is the single defect this
# whole file exists to prevent and it does not get an exemption for being cheap.
#
# ${BASH_SOURCE[0]} is this file under both execution and sourcing. When the script is PIPED into a
# shell there is no such path at all, so the assert below is what catches that case: refuse loudly
# rather than describe a stranger's directory.
_self="${BASH_SOURCE[0]:-$0}"
ROOT="$(cd "$(dirname "$_self")/../.." 2>/dev/null && pwd)"
if [ -z "$ROOT" ] || [ ! -f "$ROOT/scripts/platform/wave.sh" ]; then
  echo "wave.sh: cannot locate the repo root from '$_self' (resolved '${ROOT:-<nothing>}')." >&2
  echo "         Every path below would describe some other directory, and 'status' would report" >&2
  echo "         ALL WAVES COMPLETE about it. Run it as a file — bash scripts/platform/wave.sh —" >&2
  echo "         rather than piping it into a shell." >&2
  # `return` when sourced (do not kill the caller's shell), `exit` when executed.
  return 2 2>/dev/null || exit 2
fi
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
#
# TEST_DATABASE_URL IS IN THE WHITELIST FOR A REASON — read before removing it.
# The whitelist used to carry CARGO_TARGET_DIR alone, and `run "test api"` runs `cargo test -p
# website-api`. Every DB-backed integration test does `let Some(x) = boot() else { eprintln!("skip:
# ..."); return; }`, and boot() returns None without TEST_DATABASE_URL — so 30 of them SKIPPED and the
# step printed PASS. Measured 2026-07-26: `TEST_DATABASE_URL=x distrobox-host-exec sh -c 'echo
# [$TEST_DATABASE_URL]'` -> [] , and the suite finishing in 0.00s for a DB-backed crate is the tell.
#
# Consequence, which is why this is a BLOCKER and not a nit: EVERY regression test this program added
# — T-343, T-346, T-347, T-348, T-349, T-366 all live in tests/{missions,events,telemetry}.rs — was
# invisible to the gate that cleared their slices. ci.yml:66 and Makefile:123 both set the var, so CI
# had real coverage; the hole was specific to the gate. Third-order instance of this run's signature
# defect: reporting success on code never examined.
GATE_DB="${TBD_GATE_DB:-postgres://tbd:tbd@localhost:5434/tbd_gate_it?sslmode=disable}"
# The gate's PRIVATE trunk working set. Named here rather than buried in the sh -c string at the
# call site, because gate_trunk_build asserts against them and the whole T-396 cure is that these
# two are never the paths `trunk serve` owns. See gate_trunk_build for the measurement.
GATE_TRUNK_TARGET="${TBD_GATE_TRUNK_TARGET:-$MAIN_ROOT/target-gate-trunk}"
GATE_TRUNK_DIST="${TBD_GATE_TRUNK_DIST:-$MAIN_ROOT/dist-gate-frontend}"
# The gate's PRIVATE dir for the ANALYSIS steps — cargo check (native + wasm32) and every clippy.
# T-421. Half of a two-part cure; the other half is touch_workspace. Neither works alone and the
# measurement for that is on touch_workspace. Read both before changing either.
#
# What this half buys: it bounds WHO CAN WRITE the artifacts these steps read. The shared dir is
# written by every slice agent's ad-hoc `cargo check`, by `make api`, by `trunk serve` and by every
# other worktree's gate. This dir is written by the gate alone, and take_gate_lock serialises those,
# so while a gate holds the lock nothing else can put an artifact here. That is what makes a single
# fingerprint invalidation at the top of the critical section hold for every step below it.
# Measured 2026-07-26: 1.4 GB resident, 23.4 s cold, 0.19 s warm. Cold exactly once — only the gate
# writes here, so there is no other process to evict it.
GATE_CHECK_TARGET="${TBD_GATE_CHECK_TARGET:-$MAIN_ROOT/target-gate-check}"
# `command -v distrobox-host-exec` IS TRUE ON THE HOST TOO — read before simplifying this back.
#
# The binary is installed on BOTH sides of the bridge: /usr/bin/distrobox-host-exec exists in the
# container AND on the host. So `command -v` alone selected the bridge even from a host shell, where
# it refuses. MEASURED 2026-07-26 on the host:
#     $ distrobox-host-exec echo hi
#     You must run  distrobox-host-exec inside a container!      (exit 126)
# run() cannot tell that from a compile error, so it reported an ordinary step FAIL — OBSERVED
# 10/10 steps red, which reads as a catastrophically broken tree and sends whoever is holding the
# pager hunting a phantom for an hour. Same family as everything else in this file: the tool was
# confident about a thing it had not actually checked.
#
# Detect which side we are on using distrobox's OWN test (distrobox-host-exec:130), copied rather
# than reinvented so the two can never disagree about what "in a container" means.
#
# On the host the bridge is not merely unavailable, it is UNNECESSARY: cargo, rustfmt and trunk are
# native there — being native on the host is the entire reason the bridge exists in the other
# direction — so run them directly. Erroring out instead would replace a phantom failure with a
# hard stop on a run that would have worked. But do NOT switch behaviour silently either: announce
# it once, by name, so the log says what happened and why.
in_container() { [ -f /run/.containerenv ] || [ -f /.dockerenv ] || [ -n "${container:-}" ]; }
if command -v distrobox-host-exec >/dev/null 2>&1 && in_container; then
  HOST_BRIDGE=1
  hostrun() { distrobox-host-exec timeout "$GATE_TIMEOUT" \
                env "CARGO_TARGET_DIR=$CARGO_TARGET_DIR" "TEST_DATABASE_URL=${TEST_DATABASE_URL:-}" \
                    "MIGRATE_TEST_DATABASE_URL=${MIGRATE_TEST_DATABASE_URL:-}" "$@"; }
else
  HOST_BRIDGE=0
  if command -v distrobox-host-exec >/dev/null 2>&1; then
    echo "wave.sh: NOTE — this is the HOST shell, not the dev container." >&2
    echo "         distrobox-host-exec is installed here too but refuses outside a container" >&2
    echo "         ('You must run  distrobox-host-exec inside a container!', rc 126). Bridging" >&2
    echo "         through it would have failed EVERY step and read as a broken tree." >&2
    echo "         Running cargo/rustfmt/trunk natively instead — correct here, and expected." >&2
  fi
  hostrun() { timeout "$GATE_TIMEOUT" "$@"; }
fi

# hostrun for the ANALYSIS steps — cargo check and clippy — into the gate's private dir (T-421).
#
# The second `env` wins over the one hostrun bakes in, which is the same idiom the test steps
# already use (`hostrun env "CARGO_TARGET_DIR=$MAIN_ROOT/target-gate-api" cargo test ...`). It is a
# named function rather than that idiom repeated seven times because the whole point is that NO
# analysis step is left on the shared dir, and one name is auditable: `grep -n 'hostrun cargo'`
# should find nothing in the gate steps.
#
# CARGO_INCREMENTAL=0 for the same reason gate_test_api sets it: incremental state is another
# mtime-keyed cache layered on top of the one that lied, and the gate has no use for it.
checkrun() { hostrun env "CARGO_TARGET_DIR=$GATE_CHECK_TARGET" "CARGO_INCREMENTAL=0" "$@"; }

# Bring up a gate-private test database, and REFUSE to call a skipped suite a pass.
#
# Its own DB, not the Makefile's `rust_it`: slice agents run `make test-it` concurrently, and that
# target DROPs and recreates rust_it, so sharing it would make the gate race them.
ensure_gate_db() {
  [ -n "${TEST_DATABASE_URL:-}" ] && return 0          # operator override wins
  local psql="podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc"
  # Same host/container test as hostrun above, and for the same reason: `command -v` alone is TRUE
  # on the host, where prefixing this with the bridge makes every psql call exit 126.
  [ "$HOST_BRIDGE" = 1 ] && psql="distrobox-host-exec $psql"
  $psql "CREATE DATABASE tbd_gate_it;" >/dev/null 2>&1 || true   # already-exists is fine
  export TEST_DATABASE_URL="$GATE_DB"
  # tests/db_migrate.rs takes a SECOND variable and its own database, because it exercises the
  # migration chain from empty — it cannot share a DB the other suites have already migrated.
  # Found only because gate_test_api refuses on any skip: with the first fix in, 28 of 30 skips
  # cleared and these two remained, naming a variable nothing had mentioned.
  #
  # THE DROP BELOW IS DESTRUCTIVE AND IS ONLY SAFE UNDER THE GATE LOCK — read before moving this
  # call, and before adding a fourth caller.
  #
  # `DROP DATABASE ... WITH (FORCE)` terminates every session on tbd_gate_migrate. Grepped
  # 2026-07-26: nothing else in the tree names that database or MIGRATE_TEST_DATABASE_URL except
  # tests/db_migrate.rs and tests/models_fromrow.rs — i.e. the only thing it can ever kill is
  # ANOTHER GATE'S test run. Gate B's startup force-drops the DB gate A's db_migrate is connected
  # to. That is a third concurrency mechanism on top of the two the lock header names.
  #
  # It is closed by the flock, not by anything here — which means it was only ever as good as the
  # lock ACTUALLY being held, and before T-406 it was not: take_gate_lock returned 0 after failing
  # to lock, so on a full disk (252 MB free, recorded in cmd_reclaim's header) this ran
  # unserialised. Assert the invariant rather than assume it. Deliberately NOT a per-run database
  # name: one mechanism that is checked beats two that are hoped for, and a per-run name leaks a
  # database every time a gate is killed.
  if [ "${GATE_LOCK_HELD:-0}" != 1 ]; then
    echo "gate: REFUSING to reset tbd_gate_migrate — the gate lock is NOT held, so a concurrent"
    echo "        gate's db_migrate run may be connected to it and WITH (FORCE) would kill it."
    echo "        ensure_gate_db must be called after take_gate_lock."
    return 2
  fi
  $psql "DROP DATABASE IF EXISTS tbd_gate_migrate WITH (FORCE);" >/dev/null 2>&1 || true
  $psql "CREATE DATABASE tbd_gate_migrate;" >/dev/null 2>&1 || true
  export MIGRATE_TEST_DATABASE_URL="${MIGRATE_TEST_DATABASE_URL:-postgres://tbd:tbd@localhost:5434/tbd_gate_migrate?sslmode=disable}"
}

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

# How many tickets have shipped since the last adversarial verifier ran.
#
# WHY THIS IS A COUNTER AND NOT A HABIT: the verifier was specified as "one per wave" (rule 4), and
# the run drifted from discrete waves into a continuous stream of individual agents. That did not just
# change the vocabulary — it DELETED THE EVENT the verifier fires on, so it silently stopped running
# and 27 tickets landed unverified before the operator noticed. A trigger that depends on remembering
# a boundary that no longer exists is not a trigger.
#
# `.ai/artifacts/last-verified` holds the sha the last verifier examined. Debt is the count of
# platform tickets marked shipped since. Nagging at 8, which is one wave's width.
VERIFY_DEBT_NAG="${TBD_VERIFY_DEBT_NAG:-8}"
verify_debt() {
  local marker="$ROOT/.ai/artifacts/last-verified" base
  [ -f "$marker" ] && base="$(head -1 "$marker" | tr -d '[:space:]')" || base=""
  if [ -z "$base" ]; then echo "unknown (no .ai/artifacts/last-verified)"; return; fi
  local n
  n="$(git -C "$ROOT" log --oneline "$base..HEAD" 2>/dev/null \
       | grep -ciE 'T-[0-9]+[,: ].*(shipped|ship)' || true)"
  printf '%s since %s' "${n:-0}" "$(echo "$base" | cut -c1-8)"
}

cmd_status() {
  local w; w="$(current_wave)"
  echo "═══ platform program ═══"
  echo "plan:  $PLAN"
  local vd; vd="$(verify_debt)"
  printf 'verify: %s' "$vd"
  case "$vd" in
    unknown*) printf '  <- run an adversarial verifier and record the sha\n' ;;
    *) local c; c="${vd%% *}"
       if [ "${c:-0}" -ge "$VERIFY_DEBT_NAG" ] 2>/dev/null; then
         printf '  <- OVERDUE, %s+ landings unverified (rule 4)\n' "$VERIFY_DEBT_NAG"
       else printf '\n'; fi ;;
  esac
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

# The changed-Rust-file list, and the one distinction the change-scoped steps kept getting wrong.
#
# Union of COMMITTED and WORKING-TREE changes. Diffing the base alone means an agent running the
# slice gate before committing gets "no Rust files changed" and a vacuous PASS — observed on both
# T-182 and T-185, where the same gate went red the moment the work was committed. A gate that only
# works if you already did the right thing is not a gate.
#
# THE DISTINCTION: a path being LISTED here does not mean it EXISTS. Deletions and renames appear in
# both `git diff --name-only` and `git status --porcelain`, and the file they name is gone. Every
# caller then does `[ -f "$f" ] || continue`, so a range whose Rust files were ALL deleted or renamed
# passed refuse_empty_range (the list is non-empty), skipped every entry, and returned 0.
# MEASURED 2026-07-26 on a synthetic all-deletions range: `fmt (changed)` returned 0 with rustfmt
# invoked ZERO times, and touch_changed invalidated no fingerprint — which is the only thing it
# exists to do, and the thing that makes every cargo step below trustworthy.
#
# So the callers now count LISTED against PRESENT and refuse when nothing was examined.
# "Examined nothing" is not "examined everything and it was fine" — that equation is this
# program's signature defect and it does not get a pass for being one `[ -f ]` deep.
#
# (`git status --porcelain` renders a staged rename as `R  old -> new`, so the sed leaves one
# arrow-joined pseudo-path in the list. `[ -f ]` drops it and `git diff --name-only` lists the real
# new path separately, so it costs a phantom LISTED and nothing else.)
changed_rs() {
  local base="${1:-main...HEAD}"
  { git diff --name-only "$base" 2>/dev/null
    git status --porcelain 2>/dev/null | sed 's/^...//'
  } | grep '\.rs$' | sort -u || true
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
  local base="${1:-main...HEAD}" files f ed rc=0 listed=0 checked=0
  files="$(changed_rs "$base")"
  # A range with no Rust files at all is a legitimate SKIP — that is a backend-untouched slice, and
  # refuse_empty_range has already proved the range as a whole is non-empty.
  [ -z "$files" ] && { echo "no Rust files changed"; return 0; }
  for f in $files; do
    listed=$((listed+1))
    [ -f "$f" ] || continue          # deleted or renamed away — see changed_rs
    checked=$((checked+1))
    ed="$(file_edition "$f")"
    hostrun rustfmt --edition "$ed" --check "$f" || rc=1
  done
  # Non-vacuity. Files were named and NONE of them survive in the working tree, so rustfmt was
  # never invoked and `fmt (changed) PASS` would be a verdict about nothing.
  if [ "$checked" -eq 0 ]; then
    echo "fmt: REFUSING to pass — all $listed changed Rust file(s) are gone from the working tree"
    echo "        (deleted or renamed), so rustfmt was invoked ZERO times."
    echo "        'examined nothing' is not 'examined everything and it was fine'."
    return 1
  fi
  echo "rustfmt checked $checked of $listed listed file(s)"
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
  # checkrun, not hostrun: this IS a cargo check, so it carries the T-421 exposure verbatim. The
  # ticket's fix direction names `cargo check --workspace` and the three clippy steps; this line is
  # neither, and leaving it would have left a check step on the shared dir in the one file whose
  # subject is check steps on the shared dir. Same dir as the rest — cargo namespaces by target
  # triple, so wasm32 and native coexist without either evicting the other.
  checkrun cargo check -p website-frontend --target wasm32-unknown-unknown --quiet
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
  local base="${1:-main...HEAD}" f listed=0 touched=0
  for f in $(changed_rs "$base"); do
    listed=$((listed+1))
    [ -f "$f" ] && { touch "$f"; touched=$((touched+1)); }
  done
  # Non-vacuity, and this one is load-bearing for every step after it: if nothing was touched then
  # no fingerprint was invalidated, so cargo is free to hand this gate an artifact built from
  # ANOTHER worktree's source — the exact T-193/T-235 failure the header above describes. Silence
  # here would make check/clippy/test advisory without saying so. Callers turn this into a red.
  if [ "$listed" -gt 0 ] && [ "$touched" -eq 0 ]; then
    echo "  touch_changed: REFUSING — $listed changed Rust file(s) listed, NONE present in the"
    echo "                 working tree, so no cargo fingerprint was invalidated. Every step below"
    echo "                 could run on a stale or foreign artifact and would not be able to tell."
    return 1
  fi
  return 0
}

# T-421. The other half of the cure, and the half that actually makes the two repros red.
#
# WHAT WAS WRONG WITH THE OLD REASONING. The comment on gate_test_api used to say `cargo
# check`/`clippy` need no private dir "because they emit no binary to run". The exposure was never
# about running a binary. Cargo's freshness test is MTIME-BASED: a unit is fresh when no source file
# is newer than its recorded output. So a check step can return a verdict about a file it never
# opened, and both of the ticket's repros are that one sentence:
#
#   A. MEASURED 2026-07-26. Append `THIS IS NOT RUST AND CANNOT COMPILE ###` to
#      crates/map-engine-core/src/slot_line.rs, then `touch -r` it back to its ORIGINAL mtime.
#      `cargo check --workspace --quiet` -> rc 0. `touch` it (identical bytes) -> rc 101,
#      "reserved multi-hash token is forbidden". The gate's own clippy line: same, 0 then 101.
#   B. MEASURED 2026-07-26. A sibling worktree added a const and built into the shared dir. From a
#      tree that does not contain that symbol, `cargo check -p map-engine-core --features
#      doc,mission,world` reported `Finished in 0.06s`, and `--message-format=json` named
#      libmap_engine_core-<hash>.rmeta as its own artifact — an rmeta that greps 1 for the foreign
#      symbol while the tree greps 0. The check stood on another tree's work and said PASS.
#
# WHY THE PRIVATE DIR IS NOT ENOUGH, which is the thing to not re-derive wrongly. MEASURED
# 2026-07-26 against a freshly built target-gate-check: repro A run in the PRIVATE dir still
# returned rc 0. Of course it does — the mechanism is mtime, and a private dir changes only whose
# artifacts are there, not how freshness is decided. A private dir alone cures neither repro; it is
# the touch that does, and the private dir is what keeps the touch sufficient (see
# GATE_CHECK_TARGET: it bounds the writers to serialised gates, so nothing can re-freshen a
# fingerprint against another tree's source between our touch and our last step).
#
# WHY THE WHOLE WORKSPACE AND NOT JUST THE DIFF. touch_changed above already covers `$base..HEAD`
# union `git status --porcelain`, and that defence is real — keep it. What it cannot cover is a
# crate this slice did not touch but some OTHER tree did: wave 5's own 12/12 run touched only
# map-engine-core, website-frontend and xtask, so website-api and every other member's verdict
# rested on artifacts of unidentified provenance. Provenance is not a property of the diff, so the
# invalidation cannot be scoped to the diff.
#
# THE COST, and why it is not the "full recheck every run" it sounds like. MEASURED 2026-07-26:
# the touch invalidates 14 of 14 workspace units and 0 of 696 dependency units — the 609-crate dep
# graph is what makes a cold build expensive and NONE of it is touched. `cargo check --workspace`
# goes 0.19 s warm -> 1.09 s touched. Nine tenths of a second buys a verdict about this tree.
touch_workspace() {
  local d dirs f n=0 missing=""
  # Members from the manifest rather than a hardcoded list: a list here rots exactly the way T-422
  # records gate_schema's rotting, and the rot is silent — a member dropped from this list is a
  # crate that goes back to being judged on someone else's artifacts.
  dirs="$(sed -n '/^\[workspace\]/,/^\[[a-z]/p' Cargo.toml \
          | sed -n '/^members *= *\[/,/\]/p' | grep -o '"[^"]*"' | tr -d '"')"
  # Non-vacuity, first layer: a manifest reformat that parses to the empty set would "succeed" here
  # and touch nothing, which is the same lie one level up.
  if [ -z "$dirs" ]; then
    echo "  touch_workspace: REFUSING — parsed ZERO workspace members out of Cargo.toml, so no"
    echo "                   fingerprint was invalidated and every cargo step below could report on"
    echo "                   another tree's artifacts. Fix the parse, or the manifest."
    return 1
  fi
  for d in $dirs; do
    [ -d "$d" ] || { missing="$missing $d"; continue; }
    # -exec ... + over one find: 289 files in a single touch, not 289 processes.
    find "$d" -name '*.rs' -type f -exec touch {} + 2>/dev/null
    n=$((n + $(find "$d" -name '*.rs' -type f 2>/dev/null | wc -l)))
  done
  # A member named by the manifest but absent from disk means the parse and the tree disagree, and
  # the crates behind the missing entries are precisely the ones that would keep a stale verdict.
  if [ -n "$missing" ]; then
    echo "  touch_workspace: REFUSING — Cargo.toml names workspace member(s) that are not on disk:"
    echo "                  $missing"
    echo "                   Their fingerprints were not invalidated, so a cargo step could still be"
    echo "                   handed an artifact built from another worktree's source."
    return 1
  fi
  # Non-vacuity, second layer. Members parsed, directories present, and still no .rs file found:
  # nothing was invalidated and "examined nothing" is not "examined everything and it was fine".
  if [ "$n" -eq 0 ]; then
    echo "  touch_workspace: REFUSING — found ZERO .rs files under the workspace members, so cargo's"
    echo "                   fingerprints are untouched and every check/clippy verdict below would be"
    echo "                   about whatever was last built into $GATE_CHECK_TARGET."
    return 1
  fi
  echo "touch_workspace: invalidated $n workspace .rs file(s) across $(printf '%s\n' "$dirs" | wc -l) member(s)"
  return 0
}

# Clippy, scoped to the crates the slice actually touched, WITH --all-targets.
#
# WHY THIS EXISTS: the slice gate ran check + wasm32 + fmt and no clippy at all, so a lint in a slice's
# own code could not surface until the wave gate ran `clippy --all-targets` on merged main — where it
# reads as somebody else's problem and blocks every other slice in the group. Hit for real on T-329,
# which added a large test file: `doc_list_item_without_indentation` and an unnecessary `to_string`,
# both in code it wrote, neither visible to the gate it was told to pass.
#
# --all-targets is the load-bearing flag: the wave gate uses it, so tests and benches are gated there.
# Without it here, a test-only lint is invisible to the agent and certain to land red. That is exactly
# the T-329 case.
#
# Scoped to changed crates rather than the workspace because `clippy --workspace -D warnings` is red on
# clean main (~45 errors in tools/tbd-tools and xtask, which CI has never gated) — a gate nothing can
# pass teaches agents that gate failures are noise. Frontend goes through wasm32 with NO -D, matching
# ci.yml:113; everything else takes -D warnings, matching the wave gate.
clippy_changed() {
  local base="${1:-main...HEAD}" files crates=() c
  files="$(changed_rs "$base")"
  [ -z "$files" ] && { echo "no rust changes"; return 0; }
  # Map each file to its owning crate by walking up to the nearest Cargo.toml with a [package] name.
  for f in $files; do
    local d; d="$(dirname "$f")"
    while [ "$d" != "." ] && [ "$d" != "/" ]; do
      if [ -f "$d/Cargo.toml" ] && grep -q '^\[package\]' "$d/Cargo.toml" 2>/dev/null; then
        c="$(sed -n '/^\[package\]/,/^\[/p' "$d/Cargo.toml" | sed -n 's/^name *= *"\([^"]*\)".*/\1/p' | head -1)"
        [ -n "$c" ] && case " ${crates[*]-} " in *" $c "*) ;; *) crates+=("$c") ;; esac
        break
      fi
      d="$(dirname "$d")"
    done
  done
  # Non-vacuity, the sibling of the fmt_changed check above. This branch used to print
  # "no crate resolved" and return 0, i.e. `clippy (changed crates) PASS` having compiled nothing.
  # Printing a reason is not the same as reporting a result: the verdict still read as clean.
  #
  # Note this is deliberately NOT keyed on the files existing. A slice that DELETES a file leaves
  # its crate's Cargo.toml in place, the crate resolves, and clippy genuinely lints the crate the
  # file was removed from — that is real examination and must stay green. The vacuous case is
  # exactly this one: nothing to lint at all.
  [ "${#crates[@]}" -eq 0 ] && {
    echo "clippy: REFUSING to pass — the changed Rust file(s) resolved to NO crate, so clippy was"
    echo "        invoked ZERO times. 'examined nothing' is not 'examined everything and it was"
    echo "        fine'. (Files listed: $(printf '%s\n' "$files" | wc -l).)"
    return 1; }
  for c in "${crates[@]}"; do
    case "$c" in
      website-frontend)
        checkrun cargo clippy -p website-frontend --target wasm32-unknown-unknown --quiet || return 1 ;;
      # Not gated by CI and red on clean main — checking them would fail every slice that touches them.
      tbd-tools|xtask) printf '(skipped %s: red on main, ungated by CI) ' "$c" ;;
      # --features doc,mission is REQUIRED, for exactly the reason it is required for `cargo test`
      # thirty lines below. lib.rs:16,23,32 gate the whole doc/mission/world modules behind features,
      # so a featureless clippy COMPILES NONE OF THEM and reports success on code it never read.
      # PROVED by perturbation 2026-07-26: a `format!("{}", "verify")` injected into flatten.rs:767 —
      # the file this script's own comment calls the most contended in the backlog — gave
      # `clippy (changed crates) PASS` / `SLICE GATE: PASS` without features, and
      # `error: useless use of format!` with them. The adversarial verifier found this; the gate did not.
      map-engine-core)
        checkrun cargo clippy -p map-engine-core --features doc,mission,world --all-targets --quiet -- -D warnings || return 1 ;;
      *)
        checkrun cargo clippy -p "$c" --all-targets --quiet -- -D warnings || return 1 ;;
    esac
  done
}

# `cargo test -p website-api`, but a run where the DB tests skipped is a FAILURE, not a pass.
# CARGO_TARGET_DIR IS PRIVATE HERE — read before removing it.
#
# `cargo test` BUILDS AND THEN RUNS a binary. With the shared dir, the binary this step runs can be one
# ANOTHER WORKTREE built: same package name and version across worktrees means the same artifact hash,
# so they clobber. T-235 measured it three ways — its test binary ran another worktree's 4-test build
# TWICE under a stable hash with changing contents, target/debug/api changed size with its own source
# unchanged, and a compile failed against a stale rlib then succeeded on retry with no edit.
#
# Consequence, which is why this is a BLOCKER: THE GATE CAN PASS ON CODE IT NEVER COMPILED. T-233
# reported 126 passed / 0 failed and its test fails on a clean database — a stale or foreign binary
# that never contained the test produces exactly that, and it was reverted.
#
# The frontend test step below has had a private dir since T-193 and T-195 each proved this
# independently. The header of that step spells it out. This step never got the same treatment.
#
# THIS PARAGRAPH USED TO END: "and `cargo check`/`clippy` do not need one because they emit no
# binary to run." THAT WAS WRONG, it was the whole of T-421, and it is corrected here rather than
# deleted because it is a reasonable-sounding inference that someone will otherwise make again.
# The exposure is not about RUNNING anything. Cargo decides freshness by MTIME, so a check step
# returns a verdict about a file it never opened whenever the file's mtime does not exceed the
# recorded output's — no execution required. Both repros are on touch_workspace, which is where the
# cure lives; the analysis steps now go through checkrun into GATE_CHECK_TARGET.
gate_test_api() {
  local out rc skips
  out="$(hostrun env "CARGO_TARGET_DIR=$MAIN_ROOT/target-gate-api" "CARGO_INCREMENTAL=0" \
           cargo test -p website-api --quiet -- --nocapture 2>&1)"; rc=$?
  skips="$(printf '%s\n' "$out" | grep -c '^skip:' || true)"
  printf '%s\n' "$out"
  if [ "$rc" -ne 0 ]; then return "$rc"; fi
  if [ "${skips:-0}" -gt 0 ]; then
    echo "REFUSING to call this a pass: ${skips} DB-backed test(s) SKIPPED."
    echo "TEST_DATABASE_URL=${TEST_DATABASE_URL:-<unset>} — is postgres up on :5434? (make db-up)"
    return 1
  fi
  return 0
}

# `trunk build --release`, ISOLATED FROM THE OPERATOR'S DEV SERVER — read before simplifying.
#
# `make leptos` is `trunk serve --release`: the same binary, running the same pipeline, over the
# same crate, continuously, for hours. Two trunks over one working tree collide, and the collision
# reads exactly like a code defect — the worst failure shape there is, because an unattended fix
# agent burns its whole retry budget on working code.
#
# MEASURED 2026-07-26 against the pinned trunk 0.21.14, two release builds started together:
#   shared dist + shared CARGO_TARGET_DIR   COLLIDED IN ALL 5 TRIALS — 4 lost by the gate, 1 by the
#                                           dev server, every one of them
#                                             "running wasm-opt / error copying (optimized) wasm
#                                              file to dist dir / No such file or directory (os
#                                              error 2)"
#                                           which is the reported symptom byte for byte
#   private --dist only                     15/15 clean. The ticket records this as a disproven
#                                           dead end; it did not reproduce here. Do not read that
#                                           as safe — $CARGO_TARGET_DIR/wasm-opt/<profile>/
#                                           website-frontend_bg.wasm is still one path two writers
#                                           share, and the adversarial verifier lost a build there.
#                                           A window that is merely narrow is the exact thing that
#                                           costs an unattended agent its retry budget.
#   private --dist AND CARGO_TARGET_DIR     15/15 clean, then 10 consecutive gate builds clean with
#                                           `trunk serve --release` live on :3000 throughout. This
#                                           one is not luck: after both flags there is no path both
#                                           writers can name.
#
# THE WHOLE TRUNK WORKING SET, enumerated from 0.21.14 rather than assumed:
#   <dist>/.stage                              staging — trunk removes and recreates this at the
#                                              START of every build and deletes it at the end,
#                                              which is what "error writing JS loader file to stage
#                                              dir / No such file or directory" was
#   <dist>/*                                   the applied distribution
#   $CARGO_TARGET_DIR/wasm32-unknown-unknown/  cargo output
#   $CARGO_TARGET_DIR/wasm-bindgen/<profile>/  bindgen staging
#   $CARGO_TARGET_DIR/wasm-opt/<profile>/      wasm-opt staging   <- the one --dist does NOT move
# There is no staging env var to set. `TRUNK_STAGING_DIR` exists in the binary only as a variable
# trunk EXPORTS to build hooks, never one it reads; staging is `<final dist>/.stage`, so `--dist` is
# what isolates it. `--dist` plus `CARGO_TARGET_DIR` is therefore the COMPLETE set — nothing else is
# written outside the read-only tool cache in ~/.cache/trunk. That completeness is the argument, not
# the trial count: a private dist alone left one shared writable path and so could only ever be
# lucky, while both flags together leave none, which is why this is a cure and not a mitigation.
# The operator's dev server stays up and serving on :3000 throughout — nothing has to be killed
# before an unattended run, and the preflight `trunk serve` warning (preflight.sh:149-157) is now
# describing a hazard the gate no longer has.
#
# CORRECTION, measured: the comment this replaces claimed the gate provokes the race itself, because
# touch_changed bumps mtime on ~18 frontend files "which is exactly what trunk serve watches".
# FALSE for 0.21.14 — touching 18 real .rs files whose CONTENT was unchanged produced no rebuild at
# all, twice over; only a content change wakes the watcher. Deleting touch_changed would therefore
# not have cured anything: the operator's own edit or a merge wakes a multi-minute rebuild, and the
# gate can arrive at any point inside it.
gate_trunk_build() {
  local fdir="$ROOT/apps/website/frontend" t0 hit
  # Refuse to build UN-ISOLATED rather than race. Once either private path is collapsed onto a
  # shared one, every trunk failure past this line is an environment race wearing a compile error's
  # clothes, and the agent reading it has no way to tell.
  #
  # TWO CORRECTIONS, both measured 2026-07-26:
  #   * The dist this guard must protect is the one `trunk serve` OWNS, which is MAIN's — but $ROOT
  #     inside a worktree is the WORKTREE, so the old compare checked
  #     .ai/artifacts/worktrees/T-nnn/apps/website/frontend/dist and never looked at the path the
  #     dev server actually writes. Check both: main's (the collision that matters) and this tree's
  #     (still not somewhere a gate should be writing).
  #   * Both compares were plain strings, so a symlink or a `./` spelling of the same directory
  #     walked straight through a guard whose entire job is "are these two the same place".
  #     Canonicalise first. readlink -f resolves symlinks and normalises lexically, and still
  #     answers for a path that does not exist yet (the gate's private dirs on a cold machine).
  # Only reachable by setting TBD_GATE_TRUNK_DIST/TARGET — the defaults never collapse — which is
  # precisely why it must be right: the one caller who ever trips it is overriding on purpose.
  local _c_gt _c_gd _c_shared _c_serve _c_wt
  _c_gt="$(readlink -f -- "$GATE_TRUNK_TARGET" 2>/dev/null || printf '%s' "$GATE_TRUNK_TARGET")"
  _c_gd="$(readlink -f -- "$GATE_TRUNK_DIST"   2>/dev/null || printf '%s' "$GATE_TRUNK_DIST")"
  _c_shared="$(readlink -f -- "$CARGO_TARGET_DIR" 2>/dev/null || printf '%s' "$CARGO_TARGET_DIR")"
  _c_serve="$(readlink -f -- "$MAIN_ROOT/apps/website/frontend/dist" 2>/dev/null || printf '%s' "$MAIN_ROOT/apps/website/frontend/dist")"
  _c_wt="$(readlink -f -- "$fdir/dist" 2>/dev/null || printf '%s' "$fdir/dist")"
  if [ "$_c_gt" = "$_c_shared" ] || [ "$_c_gd" = "$_c_serve" ] || [ "$_c_gd" = "$_c_wt" ]; then
    echo "trunk: gate build paths are not private — refusing to race the operator's dev server."
    echo "        gate target=$GATE_TRUNK_TARGET  ->  $_c_gt"
    echo "        gate dist  =$GATE_TRUNK_DIST    ->  $_c_gd"
    echo "        shared cargo target = $_c_shared"
    echo "        dev server's dist   = $_c_serve   (main — this is the one trunk serve owns)"
    echo "        this tree's dist    = $_c_wt"
    return 1
  fi
  t0="$(date +%s)"
  # `return $?`, not `return 1`: hostrun applies the timeout host-side and run() reports 124 as
  # "FAIL (TIMEOUT)" rather than a build failure. Flattening it here would relabel the single most
  # expensive step's timeout as a code error — the same category mistake this whole function is about.
  hostrun sh -c "cd '$fdir' && CARGO_TARGET_DIR='$GATE_TRUNK_TARGET' trunk build --release --dist '$GATE_TRUNK_DIST'" \
    || return $?
  # NON-VACUITY. Exit 0 only says trunk was happy; it does not say trunk HONOURED either flag. A
  # Trunk.toml key, a config-precedence change on upgrade, or one dropped quote in the sh -c above
  # would put the output back into the shared paths and the isolation would be gone SILENTLY — the
  # gate would keep printing PASS right up until the day it raced again. So prove it every run:
  # both private paths must have taken a write from THIS build.
  #
  # NO SLACK on t0, and the 5 s that used to be here is REMOVED rather than reduced. `date +%s`
  # truncates downward, so t0 <= the real start instant T0; the build takes minutes, so every file
  # it writes has mtime T_w > T0 >= t0; and `-newermt` is STRICTLY greater (verified 2026-07-26: a
  # file whose mtime equals the argument does not match). So T_w > t0 holds with certainty and the
  # slack bought nothing. It cost something, though: `@$((t0 - 5))` accepted a wasm written up to
  # five seconds BEFORE this build started — i.e. exactly the stale artifact from a just-finished
  # build that this guard exists to reject. The one assumption is sub-second mtime granularity;
  # measured on the real gate paths, both are btrfs recording nanoseconds
  # ($MAIN_ROOT/target -> .240760778). Even at whole-second granularity a multi-minute build still
  # lands many seconds past t0.
  hit="$(find "$GATE_TRUNK_DIST" -name '*_bg.wasm' -newermt "@$t0" 2>/dev/null | head -1)"
  [ -n "$hit" ] || {
    echo "trunk: reported success but $GATE_TRUNK_DIST holds no wasm from this run."
    echo "        --dist was not honoured — the gate is writing into a dist the dev server owns."
    return 1; }
  hit="$(find "$GATE_TRUNK_TARGET/wasm-opt" -name '*.wasm' -newermt "@$t0" 2>/dev/null | head -1)"
  [ -n "$hit" ] || {
    echo "trunk: reported success but $GATE_TRUNK_TARGET/wasm-opt holds no wasm from this run."
    echo "        CARGO_TARGET_DIR was not honoured — wasm-opt staging is shared with the dev server."
    return 1; }
}

# ── SCHEMA (T-420) ───────────────────────────────────────────────────────────────────────────────
#
# Until this existed the gate validated NO schema at all. MEASURED on main at 33a7aa85:
# `grep -c 'xtask schema' scripts/platform/wave.sh` -> 0, and `grep -n schema` -> zero hits in 1249
# lines. The eleven steps were cargo check / wasm32 / fmt / clippy x3 / test x3 / trunk / ticket
# registry; not one read anything under packages/tbd-schema.
#
# Realised twice in one weekend:
#   * wave 4 printed `GATE: PASS  11/11` on a wave whose HEADLINE deliverable was T-241's
#     mission.schema.json change. The only evidence that schema was valid is that T-241's own agent
#     ran the validator and said so. Agent reports are evidence, not testimony.
#   * T-244 (wave 5) added a `vehicle` kind and would have merged with `make schema-validate` RED.
#     Its slice gate passed for the worst possible reason: its diff is 0 `.rs` files, so fmt and
#     clippy are change-scoped and examined nothing whatsoever.
#
# WHY THIS IS NOT ONE LINE OF `cargo xtask schema validate`, WHICH IS THE OBVIOUS FIX AND IS VACUOUS.
# MEASURED 2026-07-26 against T-244's schema commit 25d551b6, from a detached probe worktree:
#     schema validate          rc=0   <- the obvious one-liner. GREEN.
#     schema map-object-enums  rc=1   <- "prefab-classify rule[68]: kind 'vehicle' has no
#                                        class-enum mapping" (x5)
# A `run "schema" hostrun cargo xtask schema validate` step would therefore have printed PASS over
# the exact change that motivated this function: `validate` is the golden-mission/registry suite and
# never opens prefab-classify.json. That is this program's signature defect — a tool reporting
# success over an input it never examined — reproduced BY the fix for it. The step must run the SET.
#
# The set is `make schema-validate` (Makefile:137) plus `make verify-citations` (Makefile:151),
# i.e. `make ci-local-schema`. NOT ci.yml: its `schema` job (ci.yml:133,135) is `validate` +
# `citations` only, so CI has the same hole and would not have caught T-244 either. Reported, not
# fixed — wave.sh is T-420's only file.
#
# DELIBERATELY NOT CHANGE-SCOPED. "Only run if a .json under packages/tbd-schema changed" is how fmt
# and clippy came to examine nothing on T-244's diff, and it would be wrong on the facts anyway:
# these gates read xtask/src/schema_gates.rs, packages/tbd-schema/rules/, apps/mod/tbd-framework/
# and docs/specs/**. Nine sub-gates cost ~1.4 s warm (measured, 0.12 s per hostrun call including
# cargo's up-to-date check), which is less than the cost of reasoning about whether to skip them.
#
# THE CENSUS — every `xtask schema` sub-gate, run against main at 33a7aa85 before this list was
# written, with the verdict that put it in or kept it out:
#   IN   validate           rc=0  golden missions + negative goldens + registries + kit aliases
#   IN   map-object-golden  rc=0  S2-S9 + S11-S14 semantic golden gates
#   IN   map-glyphs         rc=0  glyph coverage GL-G1..G6 (29 glyphs)
#   IN   map-object-enums   rc=0  enum single-source (GAP-M5) — THE ONE T-244 BROKE
#   IN   type-inventory     rc=0  type-inventory invariants I1-I7
#   IN   t090-specs         rc=0  T-090 spec consistency 1-12 (36 spec files)
#   IN   n6                 rc=0  N6 building-geometry sentence single-source
#   IN   n10                rc=0  N10 tile-budget single-source
#   IN   citations          rc=0  @contract citation integrity (35 citations)
#   OUT  height-labels      rc=1  RED ON MAIN, and not because of any slice. G2-G6 all PASS and it
#                                 then dies in the ASL oracle with "dem decode: PNG decode: Invalid
#                                 PNG signature": packages/map-assets/everon/dem/everon-dem-16bit.png
#                                 is a 133-byte git-lfs POINTER (declared size 71,911,548) and
#                                 git-lfs is absent from the container's PATH *and* the host's.
#                                 Adding it would red every future wave — T-409's failure mode, and
#                                 worse than the hole being closed here, because a gate everyone
#                                 routes around teaches agents that gate failures are noise.
#                                 TO INCLUDE IT: put git-lfs on PATH, `make lfs-dem`, confirm
#                                 `cargo run -p xtask -- schema height-labels` is rc=0 on a clean
#                                 main, then move it from GATE_SCHEMA_EXCLUDED to GATE_SCHEMA_GATES.
#                                 Nothing else has to change.
# Also enumerated and NOT wired, because they are not in the schema-validate contract — they belong
# to `make verify-terrain` and the label lane, and widening the gate past its stated authority is a
# separate decision from closing this hole:
#   n/a  terrain-manifest   rc=0  manifest schema + terrains cross-check   (make verify-terrain)
#   n/a  locations          rc=0  locations G2-G7
#   n/a  town-labels        rc=0  town-label gates
#   n/a  road-names         rc=0  road-name gates
#   n/a  terrain-alignment  rc=1  RED, same LFS DEM pointer as height-labels ("png read_info:
#                                 Invalid PNG signature") — anchors validate, then the decode dies.
#   n/a  codegen / validate-file / flatten-orbat-slots — generators and tools, not gates.
GATE_SCHEMA_GATES="validate map-object-golden map-glyphs map-object-enums type-inventory t090-specs n6 n10 citations"
GATE_SCHEMA_EXCLUDED="height-labels"
# A PRIVATE TARGET DIR, for the same reason as `test api` / `test map-engine` / `test frontend`, and
# it is not theoretical here — it was MEASURED while this step was being written, on this machine,
# with three sibling slices live:
#
#   21:01:54  target/debug/xtask rebuilt by ANOTHER worktree (T-244, which owns xtask/schema_gates.rs
#             this wave). `grep -ac vehicleClass target/debug/xtask` -> 2.
#   21:0x     from THIS worktree, whose xtask sources contain zero `vehicleClass`:
#               $ cargo build -p xtask        ->  Finished `dev` profile ... in 0.09s
#               $ cargo run -q -p xtask -- schema map-object-golden
#                 FAIL  S3 — prefabs-sample: no prefab example for kind 'vehicle'
#             S3 reads a HARDCODED class_enum_for_kind (golden_gate.rs:87) that has no `vehicle` in
#             this tree. The verdict came from a binary this tree cannot produce.
#
# MECHANISM, so nobody has to rediscover it: cargo's freshness test is "is any source NEWER than the
# artifact?". T-244's schema_gates.rs is mtime 21:02:39; this tree's copy is 20:57:04, older than the
# 21:01:54 artifact — so cargo calls it fresh, never rebuilds, never re-uplifts, and `cargo run`
# executes the sibling's binary. The clobber is one-directional and therefore easy to miss: whichever
# tree has the older mtimes silently inherits the other's tool.
#
# A schema gate that runs a STRANGER'S validator is this program's signature defect wearing the
# costume of the fix for it, so the step gets its own dir. ONE dir, not one per tree (a per-ticket
# dir grows without bound at ~1.7 GB each), plus a CONTENT stamp: when this tree's xtask sources
# hash differently from whatever last built here, the dir is thrown away and rebuilt. Measured:
# 14 s from cold, ~0.1 s warm, 1.7 GB resident. Content, not mtime — mtime is the thing that lied.
GATE_SCHEMA_TARGET="${TBD_GATE_SCHEMA_TARGET:-$MAIN_ROOT/target-gate-schema}"

gate_schema() {
  # DRIFT TRIPWIRE. A hardcoded list is readable and greppable but it rots silently, and the way it
  # rots is precisely this ticket: `make schema-validate` grows a tenth sub-gate, nobody adds it
  # here, and the wave gate goes on printing PASS over whatever that gate checks. So diff the list
  # against the Makefile recipe every run and refuse when they disagree.
  local mk_gates
  mk_gates="$(awk '/^schema-validate:/{i=1;next} i&&/^\t/{print;next} i{exit}' Makefile \
              | sed -n 's/.*-p xtask -- schema \([a-z0-9-]*\).*/\1/p')"
  # The tripwire has to be non-vacuous too, or it is one more tool reporting on an input it never
  # read: a reformatted recipe that parses to the empty set would "agree" with any list at all.
  if [ -z "$mk_gates" ]; then
    echo "schema: read 0 sub-gates out of the schema-validate recipe in Makefile."
    echo "        The drift check is the only thing keeping this step's list honest, so a step that"
    echo "        could not run it must not go on to report PASS. Fix the parse, or the recipe."
    return 1
  fi
  local m unknown=""
  for m in $mk_gates; do
    case " $GATE_SCHEMA_GATES $GATE_SCHEMA_EXCLUDED " in
      *" $m "*) ;;
      *) unknown="$unknown $m" ;;
    esac
  done
  if [ -n "$unknown" ]; then
    echo "schema: 'make schema-validate' runs sub-gate(s) this step does not:$unknown"
    echo "        The gate would keep printing PASS over everything they check. Add each one to"
    echo "        GATE_SCHEMA_GATES above — or to GATE_SCHEMA_EXCLUDED, with the verdict that"
    echo "        justifies it and what would have to be true to include it later."
    return 1
  fi

  # ---- make sure the xtask we are about to trust is THIS tree's (see GATE_SCHEMA_TARGET) ----
  local nsrc; nsrc=$(find xtask/src -name '*.rs' -type f 2>/dev/null | wc -l)
  if [ "$nsrc" -eq 0 ]; then
    echo "schema: found no xtask/src/*.rs to stamp — cannot tell whose binary would run."
    return 1
  fi
  local stamp
  stamp="$( { find xtask/src -name '*.rs' -type f | LC_ALL=C sort | xargs cat
              cat xtask/Cargo.toml Cargo.lock; } 2>/dev/null | cksum | tr -d ' ')"
  local stampfile="$GATE_SCHEMA_TARGET/.tbd-xtask-src"
  if [ "$(cat "$stampfile" 2>/dev/null)" != "$stamp" ]; then
    rm -rf "$GATE_SCHEMA_TARGET"
    mkdir -p "$GATE_SCHEMA_TARGET" || { echo "schema: cannot create $GATE_SCHEMA_TARGET"; return 1; }
  fi
  # Build once and separately, so a compile error reads as a compile error rather than as nine
  # identical schema failures. `run()` shows the tail, and a broken xtask fails all nine otherwise.
  local build_out
  build_out="$(hostrun env "CARGO_TARGET_DIR=$GATE_SCHEMA_TARGET" cargo build -q -p xtask 2>&1)"
  local build_rc=$?
  if [ "$build_rc" -ne 0 ]; then
    printf '%s\n' "$build_out" | tail -12
    echo "schema: xtask failed to BUILD (rc $build_rc) — no sub-gate was run."
    [ "$build_rc" -eq 124 ] && return 124
    return 1
  fi
  printf '%s\n' "$stamp" > "$stampfile"

  local want=0 g
  for g in $GATE_SCHEMA_GATES; do want=$((want+1)); done

  local rc ran=0 timedout=0 failed="" detail="" out
  for g in $GATE_SCHEMA_GATES; do
    out="$(hostrun env "CARGO_TARGET_DIR=$GATE_SCHEMA_TARGET" \
             cargo run -q -p xtask -- schema "$g" 2>&1)"; rc=$?
    ran=$((ran+1))
    [ "$rc" -eq 0 ] && continue
    # 124 is hostrun's timeout, not a broken schema. Propagated below so run() can say so.
    [ "$rc" -eq 124 ] && timedout=1
    failed="$failed $g"
    detail="$detail
── schema $g (rc $rc) ──
$(printf '%s\n' "$out" | tail -6)"
  done

  # NON-VACUITY. An empty GATE_SCHEMA_GATES, or a loop that exits early, reaches the verdict below
  # having validated nothing — and would print PASS. That is the defect this function was added to
  # fix, one layer in. Count what actually executed and refuse to interpret a set that did not run.
  if [ "$ran" -eq 0 ] || [ "$ran" -ne "$want" ]; then
    echo "schema: executed $ran of $want sub-gate(s) — refusing to report on a set it did not run."
    return 1
  fi

  # Summary LAST, on purpose: both run() implementations print `tail -15` of a failed step, so a
  # verdict printed first is the line that gets cut when several sub-gates fail at once.
  if [ -n "$failed" ]; then
    printf '%s\n' "$detail"
    echo "schema: FAILED$failed  ($ran sub-gates run; excluded: $GATE_SCHEMA_EXCLUDED — see gate_schema)"
    [ "$timedout" -eq 1 ] && return 124
    return 1
  fi
  echo "schema: $ran sub-gates OK ($GATE_SCHEMA_GATES)"
}

# Refuse a gate whose change set is EMPTY.
#
# Resolvability is not non-vacuity, and the first version of this guard only checked the former.
# Found by wave 1's adversarial verifier, which got `GATE: PASS` out of both surviving holes:
#   `gate HEAD`          -> `HEAD^{commit}` resolves, `HEAD..HEAD` is empty, every change-scoped
#                           step PASSes without invoking hostrun even once.
#   `gate --slice T-393` -> gate_slice never passed a base at all, so the helpers defaulted to
#                           `main...HEAD` — correct inside a worktree, EMPTY when run on main,
#                           and the ticket id argument is decorative so it cannot self-correct.
# Both printed PASS having compiled nothing. Same signature defect, two more doorways.
#
# A slice legitimately has an empty *frontend* change set — that is what the per-step SKIPs are
# for. What is never legitimate is the WHOLE range being empty, because then no change-scoped
# step examined anything and the verdict describes nothing.
refuse_empty_range() {
  local range="$1" what="$2"
  local n; n=$(git diff --name-only "$range" 2>/dev/null | wc -l)
  [ "$n" -gt 0 ] && return 0
  echo "gate: '$range' contains no changed files — refusing to run."
  echo "        Every change-scoped step (wasm32, fmt, clippy, trunk) would report PASS/SKIP"
  echo "        without reading a line, and the verdict would describe nothing."
  echo "        $what"
  return 2
}

# ── GATE SERIALISATION ───────────────────────────────────────────────────────────────────────────
#
# TWO GATES AT ONCE REPORT ON EACH OTHER'S CODE. Two independent mechanisms, one root cause: every
# gate in every worktree writes to the same shared paths.
#
#   ARTIFACT CLOBBERING. The per-step private target dirs used below (target-gate-api, -frontend,
#   -mapengine, -trunk, -schema, and T-421's -check) are private per STEP but SHARED ACROSS
#   WORKTREES — same package + same version = same artifact hash = clobbering. T-334's agent watched
#   target-gate-api/debug/deps/events-* be overwritten mid-session by a sibling worktree's build
#   and found main's literals inside a binary its own gate had just produced, with ps confirming a
#   concurrent gate_test_api from another tree. So "N passed" was not its own code. The header on
#   gate_test_api says this hazard is fixed; it is fixed against the SHARED target dir, not against
#   another worktree using the same PRIVATE one.
#
#   THAT RESIDUE IS WHY T-421 DID NOT STOP AT A PRIVATE DIR. Because these dirs are shared across
#   worktrees, a private dir narrows WHO writes an artifact (to serialised gates) but never makes
#   the artifact this tree's — gate-to-gate clobbering survives it, and MEASURED 2026-07-26 the
#   mtime repro still returned rc 0 inside a private dir. The analysis steps therefore pair
#   GATE_CHECK_TARGET with touch_workspace, and it is the pairing that is load-bearing: the lock
#   bounds the writers, the touch makes every workspace unit recompile from THIS tree, and neither
#   is sufficient alone. The test steps above still carry the residue; that is not this ticket.
#
#   SHARED GATE DATABASE. ensure_gate_db hands every slice the same tbd_gate_it, while
#   tests/registry_compat.rs:38-60 DELETEs and re-imports two FIXED modpack UUIDs. Re-measured here
#   2026-07-26, two copies of one binary against tbd_gate_it: one panicked at registry_compat.rs:511
#   with left (0, 5) / right (16, 7) while the other passed. Run alone it always passes.
#
# Both are FALSE-RED, which is the most expensive failure shape this program has: the honest
# response to a red gate is to go hunting for a bug in your own diff, and an unattended fix agent
# will spend its whole retry budget doing exactly that to working code.
#
# THE LOCK COVERS THE WHOLE GATE, not only the steps that touch shared state. Three reasons:
#   1. A verdict is a claim about ONE tree at ONE moment. Per-step locking still lets a sibling's
#      build land between our steps, so "GATE: PASS" would describe a tree that changed underneath
#      it — a smaller version of the same lie.
#   2. touch_changed runs ONCE, at the top, and its entire job is to invalidate cargo's fingerprints
#      so the following steps compile THIS slice. A sibling gate building the same package between
#      our touch and our test re-freshens that fingerprint against ITS source, and then our step
#      runs the resulting binary. Only holding across steps closes that window.
#   3. Every step added later is inside it by default. A per-step lock is a rule the next author has
#      to remember; this is one they would have to deliberately remove.
# The cost is wall clock and nothing else, and that trade is not close.
#
# NOT per-worktree CARGO_TARGET_DIR: that fights correction 1 at the top of this file (a cold
# per-tree target is ~44 GB, the repo's own is 52 GB) and exhausts the disk by the third slice.
#
# The lock lives under the MAIN repo's target/ — the one directory every worktree already agrees on
# (correction 1), gitignored at /target/.
#
# LOCK RELEASE — the previous claim here ("a killed or timed-out gate cannot wedge the queue: there
# is no stale lock to clear by hand") was FALSE IN GENERAL and is corrected rather than deleted,
# because the thing that makes it true today is not obvious and someone will otherwise re-derive it
# wrongly.
#
# flock releases when the LAST fd on the description closes. `exec 9>>` does not set close-on-exec,
# so every child inherits fd 9 and a descendant that outlives the gate keeps the lock. MEASURED
# 2026-07-26, bash 5.2.15, 3/3 trials: a `setsid sleep` backgrounded from a gate that was then
# SIGKILLed held the lock afterwards. Bash offers no clean fix — the `exec {var}>>` form, which is
# the usual suggestion, leaks identically (also 3/3); bash has no builtin that sets FD_CLOEXEC on a
# redirection.
#
# WHY IT IS SAFE ANYWAY, TODAY: every expensive step goes through hostrun, and distrobox-host-exec
# does NOT propagate fd 9 across the bridge, so no cargo/trunk process on the host can hold it.
# Container-side descendants could, and none of the current steps background anything container-side.
# THE COST IF THAT EVER CHANGES: every subsequent gate waits GATE_LOCK_MAX (3600 s) and then refuses.
# So: do not add a step that backgrounds a process container-side without closing fd 9 in it
# (`cmd 9>&-`).
GATE_LOCK="${TBD_GATE_LOCK:-$MAIN_ROOT/target/.tbd-gate.lock}"
GATE_LOCK_POLL="${TBD_GATE_LOCK_POLL:-30}"     # heartbeat interval while blocked
GATE_LOCK_MAX="${TBD_GATE_LOCK_MAX:-3600}"     # give up (REFUSE, never run unserialised) after this

# Set by take_gate_lock on success. ensure_gate_db refuses its destructive DROP without it, and the
# verdict printer refuses to render a plain PASS without it.
GATE_LOCK_HELD=0
GATE_UNSERIALISED=0
GATE_UNSERIALISED_WHY=""

# The verdict, and the reason it is a function rather than an `echo`.
#
# A gate that could not serialise must not be able to print a string that looks like a clean pass.
# Labelling it in the VERDICT ITSELF — not in a warning fifteen lines earlier that scrolls off, and
# not only in an exit code — is the point: whatever a human or a log scraper reads last has to carry
# the caveat. FAIL is labelled too, because an unserialised red is just as likely to be a sibling's
# artifacts as it is to be a real defect, and sending someone to debug working code is this
# program's most expensive failure shape.
gate_verdict() {                      # $1 = PASS|FAIL   $2 = label
  if [ "$GATE_UNSERIALISED" = 1 ]; then
    printf '%s: %s — UNSERIALISED, NOT A CLEAN %s\n' "$2" "$1" "$1"
    printf '        %s, so another worktree may have been building into the same paths\n' "$GATE_UNSERIALISED_WHY"
    printf '        while this ran. The verdict describes an unknown tree. Fix the lock and re-run\n'
    printf '        before acting on it.\n'
  else
    printf '%s: %s\n' "$2" "$1"
  fi
}

# A gate that blocks silently for minutes is indistinguishable from a hung one, and this program
# runs unattended — so the wait announces itself, names the holder, and heartbeats until it clears.
take_gate_lock() {
  local what="$1" waited=0
  mkdir -p "$(dirname "$GATE_LOCK")" 2>/dev/null || true
  # Probe in a SUBSHELL. `exec 9>>file` with a trailing `2>/dev/null` would redirect this shell's
  # stderr permanently, and a failed bare-redirection `exec` can take the shell down with it.
  if ! command -v flock >/dev/null 2>&1 || ! ( : >>"$GATE_LOCK" ) 2>/dev/null; then
    # REFUSE, do not degrade. This used to print a WARNING and `return 0`, so the gate ran on and
    # printed `GATE: PASS` with the serialisation guarantee silently void. MEASURED 2026-07-26 by
    # extracting this function: unwritable lock path -> rc 0; flock off PATH -> rc 0; held by
    # another gate -> rc 2. Two of three failure branches degraded, and only the third matched the
    # policy the branch below states in its own comment ("Refusing beats proceeding").
    #
    # WHY REFUSE RATHER THAN WARN-AND-PASS, given the wait branch already refuses:
    #   * The unwritable branch is reachable on a FULL DISK — cmd_reclaim's header records that
    #     actually happening at 252 MB free mid-wave. A disk that full is exactly when steps start
    #     failing with "No space left on device" that reads like a build error, i.e. the worst
    #     possible moment to also hand out a verdict nobody can trust.
    #   * What the lock buys is not a nicety. T-334 watched target-gate-api/debug/deps/events-* be
    #     overwritten mid-session by a sibling worktree and found MAIN's literals inside a binary
    #     its own gate had just produced. Unserialised, "N passed" is not a claim about this slice.
    #   * Both callers already do `|| return $?`, and cmd_land treats rc 2 as red, so refusing
    #     fails safe end to end with no call-site change.
    #   * The asymmetry settles it: refusing wrongly costs one human command; degrading wrongly
    #     lands a slice on an unreliable green, which is the failure this entire file is about.
    local why
    if command -v flock >/dev/null 2>&1; then why="the lock file ($GATE_LOCK) is not writable"
    else why="flock is not on PATH"; fi
    echo "gate: REFUSING — $why, so this gate CANNOT be serialised."
    echo "        Two gates at once report on each other's artifacts (shared gate target dirs and"
    echo "        one gate database), and an unserialised verdict is the thing this lock exists to"
    echo "        prevent. A full disk reaches this branch — check \`df\` and \`wave.sh reclaim\`."
    # Escape hatch, for a machine where flock genuinely is not available. It does NOT restore the
    # old behaviour: it proceeds with the verdict itself relabelled, so nothing downstream and
    # nobody reading a log can mistake the result for a clean pass.
    if [ "${TBD_GATE_ALLOW_UNSERIALISED:-0}" = 1 ]; then
      GATE_UNSERIALISED=1
      GATE_UNSERIALISED_WHY="$why"
      echo "        TBD_GATE_ALLOW_UNSERIALISED=1 — proceeding DEGRADED at your instruction."
      echo "        The verdict will be labelled UNSERIALISED and must not be read as a pass."
      return 0
    fi
    echo "        Override deliberately with TBD_GATE_ALLOW_UNSERIALISED=1 (verdict gets labelled)."
    return 2
  fi
  exec 9>>"$GATE_LOCK"
  if ! flock -n 9; then
    # The holder writes its note just AFTER taking the lock, so losing the race by microseconds
    # reads it empty. Give it one second rather than printing "unknown" at the reader.
    local holder; holder="$(cat "$GATE_LOCK.holder" 2>/dev/null)"
    [ -n "$holder" ] || { sleep 1; holder="$(cat "$GATE_LOCK.holder" 2>/dev/null)"; }
    echo "gate: WAITING for the gate lock — this is serialisation, NOT a hang."
    echo "        holder: ${holder:-not recorded yet}"
    echo "        why:    the gate target dirs and the gate database are shared across worktrees,"
    echo "                so two gates at once report on each other's artifacts."
    while ! flock -w "$GATE_LOCK_POLL" 9; do
      waited=$((waited + GATE_LOCK_POLL))
      if [ "$waited" -ge "$GATE_LOCK_MAX" ]; then
        # Refusing beats proceeding. An unserialised verdict is the thing this lock exists to
        # prevent, so waiting out the clock must not degrade into producing one.
        echo "gate: REFUSING — no lock after ${waited}s. Another gate is stuck; do not run two."
        echo "        holder: $(cat "$GATE_LOCK.holder" 2>/dev/null || echo unknown)"
        return 2
      fi
      printf '        …still waiting %dm%02ds — holder: %s\n' \
        $((waited / 60)) $((waited % 60)) "$(cat "$GATE_LOCK.holder" 2>/dev/null || echo unknown)"
    done
    echo "gate: lock acquired after ~${waited}s."
  fi
  # The lock is genuinely ours from here. ensure_gate_db's destructive DROP asserts on this.
  GATE_LOCK_HELD=1
  printf '%s  pid %s  %s  since %s\n' "$what" "$$" "$ROOT" "$(date -u +%FT%TZ)" \
    > "$GATE_LOCK.holder" 2>/dev/null || true
  # Clear the note on the way out, but only if it is still OURS — otherwise a finishing gate would
  # wipe the note the gate that just took the lock behind it wrote, and the next waiter would be
  # told "unknown". The lock itself is the fd; this file is only ever the human-readable half.
  trap 'grep -q "pid $$ " "$GATE_LOCK.holder" 2>/dev/null && rm -f "$GATE_LOCK.holder"' EXIT
}

# Cheap gate — what a slice agent runs before reporting done. Target: ~10 s warm.
gate_slice() {
  local tid="${1:-}"
  echo "═══ slice gate ${tid} ═══"
  # gate_slice's helpers all default to `main...HEAD`, which is the slice's own diff when run
  # from its worktree and empty anywhere else. Check the range they will actually use.
  refuse_empty_range "main...HEAD" "Run this from the slice's WORKTREE, not from main." || return 2
  # Even the cheap gate builds into the SHARED CARGO_TARGET_DIR (cargo check, clippy), which is
  # exactly the dir T-193 and T-235 measured one worktree's artifacts appearing in another's.
  take_gate_lock "slice ${tid:-?}" || return $?
  local fail=0
  # touch_changed's rc was previously DISCARDED, which mattered: its whole job is to invalidate the
  # cargo fingerprints the steps below depend on, so "it invalidated nothing" has to be a red, not
  # a line of output nobody is looking at. See touch_changed.
  touch_changed || fail=1
  # T-421. Inside the lock and before every cargo step, for the same reason touch_changed is: it
  # invalidates the fingerprints those steps depend on. rc honoured — a run that invalidated nothing
  # cannot go on to interpret what the steps below report. See touch_workspace.
  touch_workspace || fail=1
  run() { local l="$1"; shift; printf "  %-24s " "$l"
    if out="$("$@" 2>&1)"; then echo PASS; else echo FAIL; printf '%s\n' "$out" | tail -15 | sed 's/^/      /'; fail=1; fi; }
  run "cargo check"  checkrun cargo check --workspace --quiet
  run "wasm32 (frontend)" wasm_changed
  run "fmt (changed)" fmt_changed
  run "clippy (changed crates)" clippy_changed
  # T-420. NOT change-scoped, and it is in the CHEAP gate on purpose: this is the step that would
  # have stopped T-244, whose diff is 0 .rs files — so every other step above it is change-scoped
  # down to nothing and its slice gate was green over a red `make schema-validate`. ~1.4 s warm.
  run "schema"       gate_schema
  echo
  [ "$fail" -ne 0 ] && { gate_verdict FAIL "SLICE GATE"; return 1; }
  gate_verdict PASS "SLICE GATE"
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
  # A base git cannot resolve makes EVERY change-scoped step below diff against nothing:
  # touch_changed, wasm_changed, fmt_changed and the trunk build each see an empty file list and
  # print PASS/SKIP without examining a single line. That is this program's signature defect —
  # a tool reporting success over an input it never looked at — living inside the gate runner.
  #
  # OBSERVED 2026-07-26 (found by T-394's slice agent, fixed here): the command center's own slice
  # briefs said `wave.sh gate T-394`, putting a ticket id where a rev belongs. `git rev-parse
  # T-394` fails, so `T-394..HEAD` resolved to nothing and the gate reported `wasm32 (frontend)
  # PASS` plus `trunk build SKIP (frontend untouched)` on a slice that changed ONLY frontend Rust.
  # Verdict: GATE: PASS. Three slices in that wave ran this way.
  #
  # Refuse instead. An unresolvable base is never a thing you meant.
  if ! git rev-parse --verify --quiet "${base}^{commit}" >/dev/null 2>&1; then
    if [[ "$base" =~ ^T-[0-9] ]]; then
      echo "gate: '$base' is a ticket id, not a git base — the per-slice gate is a different command."
      echo "        per-slice:  bash scripts/platform/wave.sh gate --slice $base"
      echo "        wave gate:  bash scripts/platform/wave.sh gate [<base>]   (default HEAD~1)"
    else
      echo "gate: base '$base' is not a resolvable commit — refusing to run."
      echo "        Every change-scoped step would diff against nothing and PASS without looking."
    fi
    return 2
  fi
  # Resolving is not the same as containing anything — `gate HEAD` cleared the check above and
  # still gated an empty range. See refuse_empty_range.
  refuse_empty_range "$base..HEAD" \
    "Pick a base that actually precedes the work — e.g. the commit before this wave opened." || return 2
  # Serialise against every other gate on this machine. The wave gate is the one that runs the
  # test steps and the trunk build, so it is the one with the most shared mutable state to lose:
  # three private-per-step target dirs that are shared per WORKTREE, one gate database, and one
  # gate dist. Taken BEFORE touch_changed — the fingerprint invalidation and the steps that depend
  # on it have to be inside the same critical section or the invalidation means nothing.
  take_gate_lock "wave gate ${base:0:12}" || return $?
  echo "═══ platform wave gate (base ${base:0:12}) ═══"
  local fail=0
  # rc honoured, not discarded — see the same call in gate_slice.
  touch_changed "$base..HEAD" || fail=1
  # T-421, same placement and same reason as in gate_slice — inside the lock, ahead of every cargo
  # step. This is the one that mattered most here: wave 5's range touched three crates, so every
  # OTHER workspace member's `cargo check` and `clippy` verdict rested on artifacts nothing in this
  # file could attribute to a tree. See touch_workspace.
  touch_workspace || fail=1
  # hostrun applies the timeout host-side; run() only has to report 124 distinctly from a real fail.
  run() {
    local l="$1"; shift
    printf "  %-24s " "$l"
    out="$("$@" 2>&1)"; local rc=$?
    if [ "$rc" -eq 0 ]; then echo PASS
    elif [ "$rc" -eq 124 ]; then echo "FAIL (TIMEOUT after ${GATE_TIMEOUT}s)"; fail=1
    else echo FAIL; printf '%s\n' "$out" | tail -15 | sed 's/^/      /'; fail=1; fi
  }
  run "cargo check"      checkrun cargo check --workspace --quiet
  run "wasm32 (frontend)" wasm_changed "$base..HEAD"
  run "fmt (changed)"    fmt_changed "$base..HEAD"
  # Clippy is scoped to the crates CI actually gates, NOT --workspace.
  #
  # `cargo clippy --workspace --all-targets -- -D warnings` is red on clean main — ~45 errors, almost
  # all in tools/tbd-tools and xtask, which have never been clippy-gated (exactly as they were never
  # fmt-gated; that drift is T-297). A workspace-wide gate would therefore be red before a single
  # slice merged, and nothing could ever land. ci.yml gates per-crate (:59 website-api, :91
  # map-engine, :112 website-frontend on wasm32) and this mirrors it.
  run "clippy api"       checkrun cargo clippy -p website-api --all-targets --quiet -- -D warnings
  # --features doc,mission for the same reason as the test step below: without them clippy compiles
  # neither doc nor mission and passes on code it never read. Measured blind on flatten.rs.
  run "clippy map-engine" checkrun cargo clippy -p map-engine-core --features doc,mission,world -p map-engine-render --all-targets --quiet -- -D warnings
  # NOTE: no `-D warnings` here, deliberately — ci.yml:113 runs frontend clippy WITHOUT it, so
  # warnings are advisory upstream and there are 25 of them on clean main. Adding -D here would make
  # the gate stricter than CI and red on arrival. The weakness is real but it is not this run's to
  # fix; filed separately.
  run "clippy frontend"  checkrun cargo clippy -p website-frontend --target wasm32-unknown-unknown --quiet
  # Scoped to CI's crates for the same reason clippy is: `cargo test --workspace` pulls in
  # tools/tbd-tools, which CI never tests and which has a FAILING test on clean main
  # (density::tests::corner_partition_identity — pre-existing, filed as its own ticket). A gate that
  # is red before any slice merges is a gate nothing can ever pass. ci.yml:68 tests website-api,
  # :115 tests website-frontend; map-engine is covered by its own job.
  # ensure_gate_db + the skip count check are what stop this step passing vacuously. A suite that
  # reports "ok" while every DB test printed `skip:` is worse than a red one: it is a green one.
  # rc honoured: ensure_gate_db now refuses to force-drop tbd_gate_migrate without the gate lock,
  # and a gate that could not prepare its database must not go on to interpret the result.
  ensure_gate_db || fail=1
  run "test api"         gate_test_api
  # --features mission is REQUIRED. The mission module is feature-gated, so a bare
  # `cargo test -p map-engine-core` runs 116 tests and silently skips 26 — every test in flatten.rs,
  # which is the most contended file in the backlog and the one T-182 inverted a pinning assertion
  # in last wave. Measured 2026-07-26: bare 116, --features mission 142.
  # AND `doc` compiles out too — T-217 measured mission-only skipping all 155 doc tests
  # (apply_faction, store, undo). doc,mission gives 183. Both features are required.
  # Private target dir for the same reason as `test api` and `test frontend`: this step RUNS test
  # binaries, and a shared dir lets another worktree's build be the one that runs.
  run "test map-engine"  hostrun env "CARGO_TARGET_DIR=$MAIN_ROOT/target-gate-mapengine" "CARGO_INCREMENTAL=0" \
                                 cargo test -p map-engine-core --features doc,mission -p map-engine-render --quiet
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
    # Isolated from `make leptos`, and it proves the isolation held. See gate_trunk_build — the
    # measurement, the enumerated trunk working set, and why the operator's dev server no longer
    # has to be killed before an unattended run, all live on that function.
    run "trunk build"    gate_trunk_build
  else
    printf "  %-24s SKIP (frontend untouched this wave)\n" "trunk build"
  fi
  # T-420. Placed next to `ticket registry` rather than up with the compile steps because the two
  # are the gate's repo-artifact validators — neither depends on the Rust build, and keeping the
  # code half and the data half legible matters more than ordering by cost here (the gate is not
  # fail-fast; every step runs and `fail` accumulates). Unconditional, never behind the frontend
  # `if`: wave 4's schema change was backend-only and would have skipped a conditional step.
  run "schema"           gate_schema
  run "ticket registry"  hostrun ./scripts/ticket check
  echo
  [ "$fail" -ne 0 ] && { gate_verdict FAIL "GATE"; return 1; }
  gate_verdict PASS "GATE"
}

# ── WAVE DISCIPLINE ──────────────────────────────────────────────────────────────────────────────
#
# Restored on operator instruction 2026-07-26 after the run drifted into a continuous stream of
# individual agents. The drift was not merely cosmetic: the wave boundary is the EVENT that fires the
# adversarial verifier (rule 4), so dissolving waves silently deleted the verifier and 27 tickets
# landed unreviewed. The operator noticed; the tooling did not.
#
# Note this does NOT reintroduce the T-181 land barrier that cost 89% of that program's wall clock.
# Slices still land the moment they are green (note 2). What a wave gates is DISPATCH: you may not
# open wave N+1 until wave N is shipped, gated and VERIFIED. Landing stays eager; starting is paced.
#
#   wave.sh wave            # what is in the current wave, and what is blocking it
#   wave.sh wave --close    # verify wave N is complete + verified, then permit N+1
#   wave.sh verified <sha>  # record that an adversarial verifier examined <sha>
cmd_wave() {
  local w; w="$(current_wave)"
  if [ "$w" = "done" ]; then echo "all waves shipped"; return 0; fi
  local total=0 shipped=0 open=""
  while IFS= read -r t; do
    total=$((total+1))
    if is_shipped "$t"; then shipped=$((shipped+1)); else open="$open $t"; fi
  done < <(wave_tickets "$w")
  echo "═══ wave $w — $shipped/$total shipped ═══"
  [ -n "$open" ] && { echo "open:"; for t in $open; do printf "  %-8s %s\n" "$t" "$(ticket_title "$t")"; done; }
  echo
  local vd; vd="$(verify_debt)"
  echo "verify debt: $vd"
  if [ -n "$open" ]; then
    echo "STATUS: wave $w is OPEN — finish it before dispatching wave $((w+1))."
  else
    echo "STATUS: wave $w tickets are all shipped. Run 'wave.sh wave --close' to gate and advance."
  fi
}

# Refuse to advance until the wave is genuinely finished: every ticket shipped, the full gate green on
# merged main, and an adversarial verifier recorded against a sha at or after the last landing. That
# third condition is the one that was being skipped, so it is checked here rather than trusted.
cmd_wave_close() {
  local w; w="$(current_wave)"
  [ "$w" = "done" ] && { echo "all waves shipped — nothing to close"; return 0; }
  local bad=0 open=""
  while IFS= read -r t; do is_shipped "$t" || open="$open $t"; done < <(wave_tickets "$w")
  if [ -n "$open" ]; then echo "REFUSED: wave $w still open:$open"; return 1; fi
  echo "wave $w: all tickets shipped ✓"

  local marker="$ROOT/.ai/artifacts/last-verified" vsha=""
  [ -f "$marker" ] && vsha="$(head -1 "$marker" | tr -d '[:space:]')"
  if [ -z "$vsha" ]; then
    echo "REFUSED: no adversarial verifier recorded. Run one against main, then:"
    echo "         bash scripts/platform/wave.sh verified \$(git rev-parse HEAD)"
    return 1
  fi
  # The verifier must have looked at a tree that CONTAINS this wave's work, not an older one.
  if ! git merge-base --is-ancestor "$vsha" HEAD 2>/dev/null; then
    echo "REFUSED: recorded verify sha $vsha is not an ancestor of HEAD — stale or wrong marker."; return 1
  fi
  local behind; behind="$(git rev-list --count "$vsha..HEAD" 2>/dev/null || echo '?')"
  if [ "${behind:-0}" -gt 0 ]; then
    echo "REFUSED: $behind commit(s) have landed since the last verifier saw $(echo "$vsha" | cut -c1-8)."
    echo "         Rule 4: the verifier examines MERGED MAIN, so it must run after the last landing."
    return 1
  fi
  echo "wave $w: verifier examined this exact tree ✓"
  # Gate against the wave's OWN BASE, not $vsha. The ancestor + behind checks above force
  # vsha == HEAD, so `cmd_gate "$vsha"` was `cmd_gate HEAD` — and fmt_changed/wasm_changed/trunk all
  # key off `$base..HEAD`, so they saw "nothing changed" and skipped. Measured: 0 files to fmt, trunk
  # build SKIP. That silently omitted the single most expensive step, and the one MAJOR-1's private
  # CARGO_TARGET_DIR fix exists to protect. It also reproduced verbatim the failure documented at the
  # top of fmt_changed — "EMPTY on merged main, so without an explicit base this checked nothing
  # exactly where it mattered most".
  local wbase
  wbase="$(git rev-parse "HEAD~${WAVE_GATE_DEPTH:-40}" 2>/dev/null || git rev-list --max-parents=0 HEAD | tail -1)"
  echo "gating wave $w against $(git rev-parse --short "$wbase") (not HEAD — that makes fmt/wasm/trunk vacuous)"
  cmd_gate "$wbase" || { echo "REFUSED: wave gate is red on main"; return 1; }
  echo
  echo "WAVE $w CLOSED. Wave $((w+1)) may be dispatched."
}

# Reclaim orphan build caches. THIS IS NOT OPTIONAL HOUSEKEEPING — it is the failure that stopped this
# program dead once.
#
# OBSERVED 2026-07-26: the disk hit **252 MB free of 952 GB** mid-wave. Two gate steps failed with
# "No space left on device", which reads exactly like a build error. `/var/tmp` held ~116 GB of agent
# target dirs from slices that had already SHIPPED — every agent is told to remove its own and many
# either forgot or were killed by a session limit before they could.
#
# Skips any dir belonging to a slice whose worktree still exists, so a live agent's cache survives.
cmd_reclaim() {
  local live="" w t freed=0 sz
  for w in $(git worktree list | tail -n +2 | awk '{print $1}'); do
    live="$live $(basename "$w" | tr 'A-Z' 'a-z' | tr -d '-')"
  done
  echo "live slices (spared):${live:- none}"
  for d in /var/tmp/*target* /var/tmp/v2-* /var/tmp/t[0-9]*-probe /var/tmp/t[0-9]*-dist; do
    [ -e "$d" ] || continue
    local key; key="$(basename "$d" | tr 'A-Z' 'a-z' | tr -d '-')"
    local skip=0 l
    for l in $live; do case "$key" in "$l"*) skip=1 ;; esac; done
    [ "$skip" -eq 1 ] && { printf '  spared  %s\n' "$d"; continue; }
    sz="$(du -sm "$d" 2>/dev/null | cut -f1)"
    rm -rf "$d" 2>/dev/null && freed=$((freed + ${sz:-0})) && printf '  removed %-44s %s MB\n' "$d" "${sz:-?}"
  done
  echo "reclaimed ${freed} MB — $(df -h "$ROOT" | tail -1 | awk '{print $4}') free"
}

cmd_verified() {
  local sha="${1:-}"
  [ -z "$sha" ] && { echo "usage: wave.sh verified <sha>"; return 1; }
  git rev-parse --verify "$sha" >/dev/null 2>&1 || { echo "not a sha: $sha"; return 1; }
  mkdir -p "$ROOT/.ai/artifacts"
  git rev-parse "$sha" > "$ROOT/.ai/artifacts/last-verified"
  echo "recorded: adversarial verifier examined $(git rev-parse --short "$sha")"
}

# Land every slice that is ready. No barrier — see note 2.
cmd_land() {
  # ARGUMENTS ARE AN ALLOWLIST, and unknown ones are REFUSED.
  #
  # This used to be `[ "${1:-}" = "--wave" ] && barrier=1` and nothing else, so any other argument
  # was silently discarded: `land T-204` was byte-for-byte `land`, and landed every committed slice
  # in the wave. OBSERVED 2026-07-26 — it merged T-389 and T-229 whose agents had not yet REPORTED,
  # defeating rule 11 from inside the tool that rule depends on, and dropped their worktrees out
  # from under two live agents. Nothing was lost only because the gate happened to pass.
  #
  # That is this run's signature defect one more time: an interface that reads narrow and acts wide.
  # A filter-shaped argument MUST filter or MUST refuse — silently ignoring it is the one option
  # that cannot be discovered before it does damage.
  local barrier=0 only=() a
  for a in "$@"; do
    case "$a" in
      --wave) barrier=1 ;;
      '')     ;;
      T-[0-9]*) only+=("$a") ;;
      *) echo "land: refusing unknown argument '$a' (expected --wave and/or T-nnn ticket ids)" >&2
         return 2 ;;
    esac
  done
  local w; w="$(current_wave)"
  [ "$w" = "done" ] && { echo "nothing to land"; return 0; }

  local t st ready=() blocked=() skipped=()
  for t in $(wave_tickets "$w"); do
    is_shipped "$t" && continue
    if [ "${#only[@]}" -gt 0 ] && ! printf '%s\n' "${only[@]}" | grep -qx "$t"; then
      skipped+=("$t"); continue
    fi
    st="$(tree_state "$t")"
    if [ "$st" = committed ] && has_work "$t"; then ready+=("$t")
    else blocked+=("$t"); fi
  done

  # A named ticket that is not in the current wave would otherwise land NOTHING and say
  # "no slice is ready" — indistinguishable from "your slice is not finished".
  if [ "${#only[@]}" -gt 0 ]; then
    local want miss=()
    for want in "${only[@]}"; do
      printf '%s\n' $(wave_tickets "$w") | grep -qx "$want" || miss+=("$want")
    done
    [ "${#miss[@]}" -gt 0 ] && {
      echo "land: ${miss[*]} not in wave $w — nothing named was landed" >&2; return 2; }
    # "other unshipped", NOT "other ready" — these were filtered out before tree_state ran, so
    # their readiness is unknown and claiming it would be the same overclaim this script exists to catch.
    echo "landing ONLY: ${only[*]}${skipped[0]:+  (holding ${#skipped[@]} other unshipped slice(s))}"
  fi

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
  wave)   if [ "${2:-}" = "--close" ]; then cmd_wave_close; else cmd_wave; fi ;;
  verified) cmd_verified "${2:-}" ;;
  reclaim) cmd_reclaim ;;
  land)   shift; cmd_land "$@" ;;
  revert) cmd_revert "${2:-}" ;;
  push)   cmd_push ;;
  *) sed -n '2,40p' "$0"; exit 1 ;;
esac
