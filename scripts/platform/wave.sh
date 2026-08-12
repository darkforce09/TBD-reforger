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
#      wave on merged main. `make ci-local` is deliberately NOT used: it is 15-40 minutes, not the
#      22.7 s the docs still claim. (It was ALSO red for weeks because verify-no-python failed on
#      scripts/mod/slice-collisions.py; T-620 ported both .py files to xtask and deleted them, so
#      that half is green now and `verify-no-python` is a wave-gate step in its own right below.)
#
#   bash scripts/platform/wave.sh status      # where are we? what is blocking?
#   bash scripts/platform/wave.sh prep        # create worktrees for the next disjoint set
#   bash scripts/platform/wave.sh gate        # full wave gate; base DERIVED from the last
#                                             # `wave N CLOSED` commit — pass one only to widen,
#                                             # never to narrow (T-602 refuses a narrowing base)
#   bash scripts/platform/wave.sh gate --slice T-190   # cheap per-slice gate
#   bash scripts/platform/wave.sh test --slice T-190 -p website-frontend
#                                             # ad-hoc cargo test into a PER-SLICE private
#                                             # CARGO_TARGET_DIR (T-742). Never bare cargo test
#                                             # against the shared cache — that is the
#                                             # cross-worktree false-binary class.
#   bash scripts/platform/wave.sh land        # merge every ready slice (no barrier)
#   bash scripts/platform/wave.sh reclaim     # /var/tmp agent caches + repo-root target-<SLICE>
#                                             # orphans (live slices spared) + ~/.cache/tbd-target-T-*
#   bash scripts/platform/wave.sh reclaim --gate-dirs      # opt-in: repo-root target-gate-* / dist-gate-*
#   bash scripts/platform/wave.sh reclaim --no-slice-dirs  # opt out of the target-<SLICE> sweep
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
# T-620: was scripts/platform/slice-collisions.py. Ported to xtask byte-identically (default,
# --check and --repack all diffed clean against the Python before it was deleted), because the
# factory's own tooling was the last thing keeping `make verify-no-python` red.
COLLIDE="cargo run -q -p xtask -- slice-collisions"

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
# Gate IT database (T-411 + T-490). Default is per-wave — NOT the forever-dirty shared `tbd_gate_it`.
# Residue used to ratchet forever on one DB (approvals page-1 ASC, missions NULL updated_at, …);
# a timed/periodic wipe would make that intermittent (false-red / flake shape). Per-wave names
# make a wave's verdict reproducible after the fact and shrink the concurrent-writer blast radius.
# Escape hatches (unchanged): TEST_DATABASE_URL skips ensure entirely; TBD_GATE_DB pins a full URL
# (create-if-missing that name, no wave prune).
#
# Wave number resolution (T-490): TBD_GATE_WAVE wins; else the committed packing counter at
# docs/platform/factory_pack_wave (command center bumps on promote); else current_wave() /
# plan-max-when-done. current_wave() alone is WRONG for factory packing: it is the lowest plan
# wave with any non-shipped/cancelled ticket, so a deferred Wave-3 pin keeps the default DB at
# tbd_gate_w3 forever and residue isolation never advances with packing waves.
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
  # T-575: MIGRATE_TEST_DATABASE_URL was forwarded here too and is gone with its consumer — see
  # ensure_gate_db. Forwarding an unset variable is harmless; forwarding one that looks live is how
  # a dead path survives four waves of readers.
  hostrun() { distrobox-host-exec timeout "$GATE_TIMEOUT" \
                env "CARGO_TARGET_DIR=$CARGO_TARGET_DIR" "TEST_DATABASE_URL=${TEST_DATABASE_URL:-}" "$@"; }
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
# CARGO_INCREMENTAL=0, and NOT for the reason it first looks like. An earlier draft of this comment
# justified it as "another mtime-keyed cache layered on top of the one that lied". That was wrong,
# and getting a justification wrong in this file is the same class of error as the bug — so it is
# corrected here rather than quietly dropped. Incremental state is CONTENT-keyed, not mtime-keyed,
# so it is emphatically not the mechanism this ticket is about: MEASURED 2026-07-26, repro A goes
# red with incremental left ON exactly as it does with it off. It is disabled because it is one more
# cache standing between this tree's bytes and the verdict, and the whole subject here is a verdict
# that came from a cache instead of from the source.
#
# THE PRICE IS RECORDED so the trade can be re-made knowingly rather than re-derived. With
# touch_workspace in front of it, `cargo check --workspace` costs 0.17 s untouched, 1.09 s touched
# with incremental ON, 6.05 s touched with it OFF — so this one setting is most of the difference
# between a 4.5 s slice gate and a 9.0 s one. Both are inside the ~10 s this gate is written to, and
# spending half that budget on having one less thing to trust is the right way round for the step
# whose entire job is to be believed. Turn it back on if the budget ever gets tight; not for tidiness.
checkrun() { hostrun env "CARGO_TARGET_DIR=$GATE_CHECK_TARGET" "CARGO_INCREMENTAL=0" "$@"; }

# Bring up a gate-private test database, and REFUSE to call a skipped suite a pass.
#
# Its own DB, not the Makefile's `rust_it`: slice agents run `make test-it` concurrently, and that
# target DROPs and recreates rust_it, so sharing it would make the gate race them.
#
# T-411 / T-490: the IT database is per-wave (`tbd_gate_w<N>`), create-if-missing, with DBs older
# than the last two waves dropped under the gate lock. NOT a per-run name (that leaks a DB every
# kill) and NOT a timed wipe (that turns a permanent ratchet into an intermittent flake).
#
# T-490: do NOT derive N from current_wave() when a packing counter exists. current_wave() is the
# lowest plan wave with any deferred/open ticket — a Wave-3 deferral pins tbd_gate_w3 forever while
# the factory is packing Wave 35. Prefer docs/platform/factory_pack_wave (positive integer, bumped
# on promote) so residue isolation tracks packing progress.
gate_wave_number() {
  local w pack_file pack
  if [ -n "${TBD_GATE_WAVE:-}" ]; then
    w="$TBD_GATE_WAVE"
  else
    pack_file="$ROOT/docs/platform/factory_pack_wave"
    if [ -f "$pack_file" ]; then
      # Single integer, optional trailing whitespace/newline. Reject empty, zero, non-numeric.
      pack="$(tr -d '[:space:]' < "$pack_file" 2>/dev/null || true)"
      if [[ "$pack" =~ ^[1-9][0-9]*$ ]]; then
        w="$pack"
      fi
    fi
    if [ -z "${w:-}" ]; then
      w="$(current_wave)"
      if [ "$w" = "done" ]; then
        # All plan tickets shipped — pin to the highest wave number still in the plan.
        w="$(plan_rows | awk -F'\t' '$1 ~ /^[0-9]+$/ {print $1}' | sort -n | tail -1)"
      fi
    fi
  fi
  [[ "$w" =~ ^[0-9]+$ ]] || { echo "gate: cannot derive numeric wave for gate DB (got '${w:-<empty>}')" >&2; return 2; }
  echo "$w"
}

# Drop tbd_gate_w* databases older than the last two waves (keep N and N-1). Only names matching
# ^tbd_gate_w[0-9]+$ — never tbd_gate_it, tbd_gate_migrate, or operator TBD_GATE_DB names.
#
# T-534: the wave DB is no longer the only thing to reap. `cargo test -p website-api` now gives
# each test BINARY its own database, derived as <base>_<suite>_it by
# apps/website/api/tests/common/mod.rs (per_binary_database_name) — so one gate run against
# tbd_gate_w60 also leaves tbd_gate_w60_admin_field_it, …_events_it, … 25 of them, measured.
# They are dropped and recreated on every run, so they do not grow per run — but without this
# they would accumulate 25 per WAVE forever, because the old `^tbd_gate_w[0-9]+$` pattern
# matched none of them. The wave number is now parsed out of the leading segment so a derived
# name is reaped with the wave it belongs to, on exactly the same keep-N-and-N-1 policy.
#
# The pattern is still anchored and still cannot name tbd_gate_it, tbd_gate_migrate or an
# operator TBD_GATE_DB: it requires tbd_gate_w<digits> followed by end-of-name OR by a
# _<suite>_it tail. Widening it further would put a DROP in reach of names this function was
# never meant to touch — the header above is the contract, keep it narrow.
prune_old_gate_wave_dbs() {
  local wave="$1"
  local keep_min=$((wave > 0 ? wave - 1 : 0))
  # Listing needs -Atc (tuples-only); CREATE/DROP keep -qc. Same host-bridge rule as ensure_gate_db.
  local list="podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -Atc"
  local drop="podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc"
  [ "$HOST_BRIDGE" = 1 ] && { list="distrobox-host-exec $list"; drop="distrobox-host-exec $drop"; }
  local name n
  while IFS= read -r name; do
    [ -z "$name" ] && continue
    # tbd_gate_w60 -> 60; tbd_gate_w60_admin_field_it -> 60. Anything else is skipped.
    [[ "$name" =~ ^tbd_gate_w([0-9]+)(_[a-z0-9_]+_it)?$ ]] || continue
    n="${BASH_REMATCH[1]}"
    if [ "$n" -lt "$keep_min" ]; then
      echo "gate: dropping stale wave DB $name (current wave $wave; keeping w${keep_min}+)"
      $drop "DROP DATABASE IF EXISTS ${name} WITH (FORCE);" >/dev/null 2>&1 || true
    fi
  done < <($list "SELECT datname FROM pg_database WHERE datname ~ '^tbd_gate_w[0-9]+(_[a-z0-9_]+_it)?$';" 2>/dev/null || true)
}

ensure_gate_db() {
  [ -n "${TEST_DATABASE_URL:-}" ] && return 0          # operator override wins
  local psql="podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc"
  # Same host/container test as hostrun above, and for the same reason: `command -v` alone is TRUE
  # on the host, where prefixing this with the bridge makes every psql call exit 126.
  [ "$HOST_BRIDGE" = 1 ] && psql="distrobox-host-exec $psql"

  local db_name url wave
  if [ -n "${TBD_GATE_DB:-}" ]; then
    # Operator-pinned full URL. Create-if-missing that database; do not prune wave DBs.
    url="$TBD_GATE_DB"
    db_name="${url##*/}"
    db_name="${db_name%%\?*}"
    if ! [[ "$db_name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
      echo "gate: TBD_GATE_DB database name '$db_name' is not a safe SQL identifier — refusing."
      return 2
    fi
    $psql "CREATE DATABASE ${db_name};" >/dev/null 2>&1 || true   # already-exists is fine
    export TEST_DATABASE_URL="$url"
  else
    wave="$(gate_wave_number)" || return 2
    db_name="tbd_gate_w${wave}"
    url="postgres://tbd:tbd@localhost:5434/${db_name}?sslmode=disable"
    $psql "CREATE DATABASE ${db_name};" >/dev/null 2>&1 || true   # already-exists is fine
    export TEST_DATABASE_URL="$url"
  fi

  # T-575 — THE SECOND VARIABLE AND ITS DATABASE ARE GONE. This block used to force-drop and
  # recreate `tbd_gate_migrate` and export `MIGRATE_TEST_DATABASE_URL` at it, because
  # tests/db_migrate.rs exercises the migration chain from empty and could not share a DB the other
  # suites had already migrated. T-558 moved db_migrate.rs AND models_fromrow.rs onto
  # `common::require_test_database_url`, so each gets its own `<base>_<suite>_it` off
  # TEST_DATABASE_URL (the T-534 shape) and NEITHER reads the variable any more.
  #
  # Verified repo-wide before deleting, not assumed: the only surviving mentions of
  # MIGRATE_TEST_DATABASE_URL are the two `//!` doc comments in those same two test files recording
  # that they no longer share it, plus ticket registry prose. `std::env::var` for it: zero hits.
  # So the export named a variable nothing read, pointed at a database nothing opened, and the
  # DROP ... WITH (FORCE) that preceded it could only ever have terminated sessions on a database
  # with no legitimate user. Deleted rather than left as harmless: a live-looking destructive
  # statement is exactly what a future reader will preserve on the assumption it matters.
  #
  # THE DROP BELOW IS DESTRUCTIVE AND IS ONLY SAFE UNDER THE GATE LOCK — read before moving this
  # call, and before adding a fourth caller. It is now the per-wave IT DB prune alone
  # (prune_old_gate_wave_dbs), which is the same destructive class the migrate reset was and keeps
  # the same assert: `DROP DATABASE ... WITH (FORCE)` terminates every session on the target, and
  # a tbd_gate_w<N> the prune considers stale can still be the DB ANOTHER GATE is testing against.
  #
  # It is closed by the flock, not by anything here — which means it was only ever as good as the
  # lock ACTUALLY being held, and before T-406 it was not: take_gate_lock returned 0 after failing
  # to lock, so on a full disk (252 MB free, recorded in cmd_reclaim's header) this ran
  # unserialised. Assert the invariant rather than assume it. IT DBs are per-wave (T-411); pruning
  # waves older than the last two sits under this assert.
  # GATE_LOCK_HELD=1 is the normal path. GATE_UNSERIALISED=1 is the deliberate escape hatch
  # (TBD_GATE_ALLOW_UNSERIALISED=1): the operator accepted a degraded verdict, and the full gate
  # must still be able to prepare its databases. T-409: the hatch used to return 0 from
  # take_gate_lock without setting GATE_LOCK_HELD, so ensure_gate_db refused and every full-gate
  # run under the hatch printed GATE: FAIL — UNSERIALISED regardless of the code.
  if [ "${GATE_LOCK_HELD:-0}" != 1 ] && [ "${GATE_UNSERIALISED:-0}" != 1 ]; then
    echo "gate: REFUSING to prune stale wave databases — the gate lock is NOT held, so a concurrent"
    echo "        gate may be connected to one of them and WITH (FORCE) would kill its test run."
    echo "        ensure_gate_db must be called after take_gate_lock."
    return 2
  fi
  if [ "${GATE_LOCK_HELD:-0}" != 1 ] && [ "${GATE_UNSERIALISED:-0}" = 1 ]; then
    echo "gate: WARNING — pruning stale wave databases WITHOUT the lock (TBD_GATE_ALLOW_UNSERIALISED)."
    echo "        A concurrent gate may be connected to one; WITH (FORCE) would kill it."
  fi
  # Prune only on the default per-wave path — never when the operator pinned TBD_GATE_DB.
  if [ -z "${TBD_GATE_DB:-}" ] && [ -n "${wave:-}" ]; then
    prune_old_gate_wave_dbs "$wave"
  fi
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

# The lowest wave AT OR ABOVE THE LIVE GENERATION FLOOR with at least one unshipped ticket.
#
# T-616. Column 1 of the plan used to carry two spellings — bare `0`-`11`/`43`-`68`/`99` and
# `w76`…`w81` — and that mix was not cosmetic, it was LOAD-BEARING. `sort -n` scores any
# non-numeric key as 0, so every `wNN` row sorted into the wave-0 block AHEAD of wave 1, and this
# loop therefore reached the live factory rows first. The answer it returned was right; the reason
# was an accident. MEASURED 2026-08-01 before the migration: `current_wave` -> `w80`, which is the
# operationally correct wave, arrived at by a sort that believed 80 < 1.
#
# So normalising the column to bare integers — which is what T-616 asks for, and what
# `slice-collisions` needs since `int('w80')` raises — is only HALF a migration. With uniform
# numbers `sort -n` finally orders honestly, and this loop then walks the LEGACY BACKLOG first and
# returns wave 3 (T-578/579/580/587, all `deferred`). `wave`, `wave --close` and `land` all key off
# this function, so that would have pointed every one of them at a four-year-old deferred backlog
# row instead of the wave in flight. A uniform format that silently re-aims `land` is a worse bug
# than the mixed format it replaced.
#
# The floor is what the `w` prefix actually MEANT, written down as data the sort can respect. The
# plan holds two generations: the legacy packing waves (0-11, 43-68, plus 99 as a parking lot,
# still carrying genuinely open `idea`/`deferred` backlog) and the live factory waves, which begin
# at 76. Only the live generation is dispatchable, so only it can be "current". MEASURED after the
# migration: waves with unshipped tickets are 0, 3, 5, 7, 8, 9, 10, 11, 80, 81, 99 — floor 76
# selects 80, identical to the pre-migration answer.
#
# Raise this when a later generation starts; it is one integer in one place, which is strictly more
# maintainable than a prefix that had to be typed onto every row and understood by every parser.
WAVE_GENERATION_FLOOR="${TBD_WAVE_GENERATION_FLOOR:-76}"
current_wave() {
  local w t
  while IFS=$'\t' read -r w t _; do
    [ "$w" = "0" ] && continue
    # Bare-integer guard: a row whose label is not numeric cannot be compared, and silently
    # skipping it is how the pre-T-616 mix hid. Say so and keep going.
    if ! [[ "$w" =~ ^[0-9]+$ ]]; then
      echo "wave.sh: non-numeric wave label '$w' in $PLAN — T-616 normalised these to integers" >&2
      continue
    fi
    [ "$w" -lt "$WAVE_GENERATION_FLOOR" ] && continue
    if ! is_shipped "$t"; then echo "$w"; return; fi
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

# Working-tree porcelain paths with LFS filters neutralised — same flags as tree_state.
#
# T-401: `changed_rs` / `wasm_changed` / `refuse_empty_range` used
# `git status --porcelain 2>/dev/null` and treated empty stdout as "no changes". When the LFS
# clean filter aborts (exit 128, empty stdout) that silently half-killed every change-scoped
# gate: committed diffs still showed, but uncommitted working-tree Rust/frontend edits vanished.
# Capture rc, never swallow a non-zero behind `2>/dev/null`, and fail loud.
git_porcelain_paths() {
  local out rc
  out="$(git -c filter.lfs.process= -c filter.lfs.clean=cat -c filter.lfs.smudge=cat \
         -c filter.lfs.required=false status --porcelain)"; rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "wave.sh: git status --porcelain failed (rc=$rc) — refusing silent empty change list" >&2
    return "$rc"
  fi
  printf '%s\n' "$out" | sed 's/^...//'
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
  echo "→ dispatch set: $COLLIDE"
}

cmd_prep() {
  echo "next disjoint dispatch set:"
  # cargo is a HOST binary inside the dev container, so this goes through the bridge — unlike the
  # `python3` it replaced, which was present on both sides. hostrun degrades to a plain exec on the
  # host, so the same line is correct from either shell.
  hostrun $COLLIDE
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
# both `git diff --name-only` and `git status --porcelain`, and the file they name is gone.
#
# Callers handle absence differently (T-409 corrected T-406's over-refuse):
#   * fmt_changed — deletion-only is a named SKIP (nothing left to format).
#   * touch_changed — touches the owning crate's Cargo.toml (or include! consumers) so cargo
#     fingerprints still invalidate; refuses only when nothing at all can be touched.
#   * clippy_changed — resolves the crate from the path (or include! consumers for orphan
#     fragments like apps/website/shared/*.rs); refuses only when zero crates resolve.
# The signature-defect refuse that remains is "listed Rust changes, examined NOTHING" — not
# "listed deletions, rustfmt had no file to open".
#
# (`git status --porcelain` renders a staged rename as `R  old -> new`, so the sed leaves one
# arrow-joined pseudo-path in the list. `[ -f ]` drops it and `git diff --name-only` lists the real
# new path separately, so it costs a phantom LISTED and nothing else.)
changed_rs() {
  local base="${1:-main...HEAD}" wt
  wt="$(git_porcelain_paths)" || return $?
  { git diff --name-only "$base" 2>/dev/null || true
    printf '%s\n' "$wt"
  } | grep '\.rs$' | sort -u || true
}

# Format-check ONLY the files this slice changed against main.
#
# Workspace-wide `cargo fmt --all --check` is the local/CI FMT-1 gate (`make rust-fmt` /
# `.github/workflows/ci.yml` website-api; T-297 cleaned the tree, T-453 aligned CI). The wave
# gate stays diff-scoped so a slice only fails on files it touched — not a substitute for CI
# `--all`. Edition is NOT fixed across this workspace: apps/website/api is edition 2024, most
# other crates are 2021, and the two style editions sort a mixed-case brace import differently.
# Hardcoding --edition 2021 made every slice touching an edition-2024 file fail a gate it did
# not cause — main's own `use axum::http::{HeaderMap, HeaderValue, StatusCode, header};` already
# fails the 2021 form. Resolve each file's edition from the nearest Cargo.toml above it.
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
  # T-492: empty→SKIP must not mask a failed changed_rs (e.g. git_porcelain_paths rc≠0).
  # wasm_changed / refuse_empty_range already check porcelain rc; these two helpers did not.
  files="$(changed_rs "$base")" || return $?
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
  # Deletion/rename-only is a legitimate SKIP for rustfmt: there is no source left to format.
  # T-406 keyed checked==0 as vacuous and refused; T-409 corrected it — the same shape already
  # stayed green in clippy_changed (crate still resolves and is linted). Silence stays banned:
  # we always name the skip. The vacuous refuse that must NOT return green is elsewhere —
  # clippy with zero resolved crates, touch that invalidated no fingerprint.
  if [ "$checked" -eq 0 ]; then
    echo "fmt: all $listed changed Rust file(s) deleted/renamed away — nothing to format"
    return 0
  fi
  echo "rustfmt checked $checked of $listed listed file(s)"
  return "$rc"
}

# Native `cargo check --workspace` does NOT compile the frontend: apps/website/frontend/src is
# `#![cfg(target_arch = "wasm32")]`, so a native check walks straight past it and reports PASS on a
# file it never looked at. T-188 hit exactly this. Any slice touching the frontend must be checked
# for wasm32 or the gate is decorative. Warm cost measured: 0.16s.
wasm_changed() {
  local base="${1:-main...HEAD}" wt
  # Same union as fmt_changed, for the same reason. LFS-safe porcelain (T-401).
  wt="$(git_porcelain_paths)" || return $?
  { git diff --name-only "$base" 2>/dev/null || true
    printf '%s\n' "$wt"
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
# Directory of the [package] Cargo.toml owning a .rs path, or empty.
# Walk-up first; orphan fragments (apps/website/shared/*.rs) have no package ancestor — those
# are handled by the include!-consumer path in clippy_changed / the touch fallback below.
owning_package_dir() {
  local f="$1" d
  d="$(dirname "$f")"
  while [ "$d" != "." ] && [ "$d" != "/" ]; do
    if [ -f "$d/Cargo.toml" ] && grep -q '^\[package\]' "$d/Cargo.toml" 2>/dev/null; then
      printf '%s\n' "$d"
      return 0
    fi
    d="$(dirname "$d")"
  done
  return 1
}

# Cargo.toml dirs of every crate that include!s an orphan .rs fragment.
# grep, not rg — rg is container-only (PLATFORM_FACTORY.md Known traps).
include_consumer_package_dirs() {
  local orphan="$1" base consumer incl cand orphan_abs d
  base="$(basename "$orphan")"
  orphan_abs="$(realpath -m "$orphan")"
  while IFS= read -r consumer; do
    [ -f "$consumer" ] || continue
    grep -qF "$base" "$consumer" || continue
    while IFS= read -r incl; do
      [ -n "$incl" ] || continue
      cand="$(cd "$(dirname "$consumer")" && realpath -m "$incl")"
      [ "$cand" = "$orphan_abs" ] || continue
      d="$(owning_package_dir "$consumer")" || continue
      printf '%s\n' "$d"
    done < <(grep -oE 'include!\(\s*"[^"]+"\s*\)' "$consumer" 2>/dev/null \
             | sed -E 's/.*include!\([[:space:]]*"([^"]+)"[[:space:]]*\).*/\1/' \
             | grep -F "$base" || true)
  done < <(grep -rl --include='*.rs' --exclude-dir=target "include!(" apps packages crates tools 2>/dev/null || true)
}

# Non-.rs files rustc embeds via include_str!/include_bytes! (T-426). grep, not rg.
#
# T-421's touch_workspace invalidated every workspace .rs mtime but not the JSON/WGSL/SQL paths
# those macros pull in — same mtime-freshness hole, narrower blast radius. MEASURED 2026-07-27:
# repro on packages/tbd-schema/schema/mission.schema.json with `touch -r` back to original mtime
# after a byte change: `cargo check -p map-engine-core --features doc,mission,world` in
# target-gate-check stayed rc 0 until the schema file itself was touched.
#
# Static paths are resolved from the including .rs file; concat!(env!("CARGO_MANIFEST_DIR"), "…")
# is resolved from the owning package dir. Macro-expanded fixture trees (dto.rs golden tests) are
# touched wholesale because their per-file paths are not statically enumerable.
compiled_include_input_paths() {
  local dirs d consumer flat manifest_dir rel suffix cand fixture_dir
  dirs="$(sed -n '/^\[workspace\]/,/^\[[a-z]/p' Cargo.toml \
          | sed -n '/^members *= *\[/,/\]/p' | grep -o '"[^"]*"' | tr -d '"')"
  for d in $dirs; do
    [ -d "$d" ] || continue
    while IFS= read -r consumer; do
      [ -f "$consumer" ] || continue
      manifest_dir="$(owning_package_dir "$consumer" || true)"
      [ -n "$manifest_dir" ] && manifest_dir="$(realpath -m "$manifest_dir")"
      flat="$(tr '\n' ' ' < "$consumer")"
      while IFS= read -r rel; do
        [ -n "$rel" ] || continue
        cand="$(cd "$(dirname "$consumer")" && realpath -m "$rel")"
        [ -f "$cand" ] && printf '%s\n' "$cand"
      done < <(printf '%s\n' "$flat" | grep -oE 'include_(str|bytes)!\(\s*"[^"]+"' 2>/dev/null \
               | sed -E 's/include_(str|bytes)!\(\s*"([^"]+)".*/\2/' || true)
      if printf '%s\n' "$flat" | grep -q 'concat!(env!("CARGO_MANIFEST_DIR")'; then
        [ -n "$manifest_dir" ] || continue
        while IFS= read -r suffix; do
          [ -n "$suffix" ] || continue
          cand="$(realpath -m "$manifest_dir/$suffix")"
          [ -f "$cand" ] && printf '%s\n' "$cand"
        done < <(printf '%s\n' "$flat" | grep -oE 'include_(str|bytes)!\(\s*concat!\(\s*env!\("CARGO_MANIFEST_DIR"\)\s*,\s*"[^"]+"' 2>/dev/null \
                 | sed -E 's/.*"([^"]+)".*/\1/' || true)
      fi
      if printf '%s\n' "$flat" | grep -qF 'concat!("../tests/fixtures/api/"'; then
        fixture_dir="$(cd "$(dirname "$consumer")" && realpath -m "../tests/fixtures/api")"
        [ -d "$fixture_dir" ] && find "$fixture_dir" -type f 2>/dev/null
      fi
    done < <(find "$d" -name '*.rs' -type f 2>/dev/null)
  done | sort -u
}

touch_changed() {
  local base="${1:-main...HEAD}" files f listed=0 touched=0 d
  # T-536: empty→listed=0→return 0 must not mask a failed changed_rs (e.g. git_porcelain_paths
  # rc≠0). Same class as T-492 for fmt_changed/clippy_changed — `for f in $(changed_rs …)`
  # discarded the rc and treated porcelain failure as an empty change list.
  files="$(changed_rs "$base")" || return $?
  for f in $files; do
    listed=$((listed+1))
    if [ -f "$f" ]; then
      touch "$f"
      touched=$((touched+1))
      continue
    fi
    # Deleted/renamed-away: the file cannot be touched, but its crate (or include! consumers)
    # still needs a fingerprint bump — otherwise cargo is free to reuse a stale artifact that
    # still contains the deleted code. T-409: deletion-only used to hard-fail here while
    # clippy_changed correctly stayed green.
    d="$(owning_package_dir "$f" || true)"
    if [ -n "$d" ] && [ -f "$d/Cargo.toml" ]; then
      touch "$d/Cargo.toml"
      touched=$((touched+1))
      continue
    fi
    while IFS= read -r d; do
      [ -n "$d" ] && [ -f "$d/Cargo.toml" ] || continue
      touch "$d/Cargo.toml"
      touched=$((touched+1))
    done < <(include_consumer_package_dirs "$f" | sort -u)
  done
  # Non-vacuity, load-bearing for every step after it: listed Rust changes but NOTHING's
  # fingerprint was invalidated → cargo may hand this gate a foreign/stale artifact.
  # Deletion-only that resolved to an owning crate (or include! consumers) is green above;
  # this refuse is the residual "wrong reason" case (orphan path, no package, no include!).
  if [ "$listed" -gt 0 ] && [ "$touched" -eq 0 ]; then
    echo "  touch_changed: REFUSING — $listed changed Rust file(s) listed, but no source and no"
    echo "                 owning crate Cargo.toml could be touched, so no cargo fingerprint was"
    echo "                 invalidated. Every step below could run on a stale or foreign artifact."
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
  local d dirs f n=0 incl_n=0 missing="" incl_paths
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
  incl_paths="$(compiled_include_input_paths)"
  if [ -n "$incl_paths" ]; then
    while IFS= read -r f; do
      [ -f "$f" ] || continue
      touch "$f"
      incl_n=$((incl_n + 1))
    done <<< "$incl_paths"
  fi
  echo "touch_workspace: invalidated $n workspace .rs file(s) and $incl_n include_str!/include_bytes! input(s) across $(printf '%s\n' "$dirs" | wc -l) member(s)"
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
# clean main — a gate nothing can pass teaches agents that gate failures are noise. T-603 re-measured
# 2026-07-31: 60 errors, ALL of them website-frontend linted natively, none in tools/tbd-tools or
# xtask (this note used to blame those two; they are clean and the wave gate now lints them by name).
# Frontend goes through wasm32 with NO -D, matching ci.yml:113; everything else takes -D warnings,
# matching the wave gate.
clippy_changed() {
  local base="${1:-main...HEAD}" files crates=() c d f pkg
  # T-492: propagate changed_rs failure — empty stdout + rc≠0 must not become SKIP.
  files="$(changed_rs "$base")" || return $?
  [ -z "$files" ] && { echo "no rust changes"; return 0; }
  # Map each file to its owning crate by walking up to the nearest Cargo.toml with a [package] name.
  # Orphan fragments (apps/website/shared/*.rs) have no package ancestor — the walk stops at '.' —
  # but they are include!'d into real crates (T-405 is_http_url_cases.rs → website-api +
  # website-frontend). T-406's empty-crates refuse false-red'd that shape; resolve via include!
  # consumers before refusing.
  for f in $files; do
    d="$(owning_package_dir "$f" || true)"
    if [ -n "$d" ]; then
      c="$(sed -n '/^\[package\]/,/^\[/p' "$d/Cargo.toml" | sed -n 's/^name *= *"\([^"]*\)".*/\1/p' | head -1)"
      [ -n "$c" ] && case " ${crates[*]-} " in *" $c "*) ;; *) crates+=("$c") ;; esac
      continue
    fi
    while IFS= read -r pkg; do
      [ -n "$pkg" ] && [ -f "$pkg/Cargo.toml" ] || continue
      c="$(sed -n '/^\[package\]/,/^\[/p' "$pkg/Cargo.toml" | sed -n 's/^name *= *"\([^"]*\)".*/\1/p' | head -1)"
      [ -n "$c" ] && case " ${crates[*]-} " in *" $c "*) ;; *) crates+=("$c") ;; esac
    done < <(include_consumer_package_dirs "$f" | sort -u)
  done
  # Non-vacuity. This branch used to print "no crate resolved" and return 0, i.e.
  # `clippy (changed crates) PASS` having compiled nothing. Printing a reason is not the same
  # as reporting a result: the verdict still read as clean.
  #
  # Deliberately NOT keyed on the files existing. A slice that DELETES a file leaves its crate's
  # Cargo.toml in place, the crate resolves, and clippy genuinely lints the crate the file was
  # removed from — that is real examination and must stay green. The vacuous case is exactly
  # this one: nothing to lint at all (no package ancestor AND no include! consumer).
  [ "${#crates[@]}" -eq 0 ] && {
    echo "clippy: REFUSING to pass — the changed Rust file(s) resolved to NO crate, so clippy was"
    echo "        invoked ZERO times. 'examined nothing' is not 'examined everything and it was"
    echo "        fine'. (Files listed: $(printf '%s\n' "$files" | wc -l).)"
    return 1; }
  for c in "${crates[@]}"; do
    case "$c" in
      website-frontend)
        # T-742: --all-targets is load-bearing (see function header) — without it, #[cfg(test)]
        # lints are invisible here and certain to land red once T-752 teaches CI/Makefile the same
        # flag. NO -D warnings: ci.yml website-frontend clippy is advisory (no -D), matching the
        # wave-gate `clippy frontend` step. Align -D with CI intent, not with the other crates.
        checkrun cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets --quiet || return 1 ;;
      # T-614 — tbd-tools AND xtask USED TO BE SKIPPED HERE, reason "red on main, ungated by CI".
      # The first half was FALSE and contradicted by the header of this same function: T-603's
      # re-measure found the 60 workspace errors are ALL website-frontend and called these two
      # clean, and the wave gate has linted them by name since then (`clippy xtask+tbd-tools`,
      # in cmd_gate). Re-verified 2026-08-01 through this very function, both directions: with
      # the arm removed, a `format!("{}", "verify")` injected into tools/tbd-tools/src/enf/
      # apidoc.rs and into xtask/src/sync.rs made clippy_changed return 1 naming each file and
      # line in turn, and with the injections removed it returned 0 having actually compiled
      # both crates. The old arm returned 0 with BOTH injections in place, printing
      # `(skipped tbd-tools: …) (skipped xtask: …)` and compiling nothing.
      # The second half is still true — the ci.yml change was comment-only — which is
      # exactly why the skip had to go: nothing else lints them before merged main, so a slice
      # editing ONLY these crates had its own gate examine none of its code and landed its lint at
      # the wave gate, where it reads as somebody else's problem and blocks the whole group. That
      # is the T-329 shape this function's header says it exists to prevent, and it was living in
      # the function. They now fall through to the default arm below, like every other crate.
      #
      # --features doc,mission,world is REQUIRED (same floor as --all-features / the gate test
      # step). lib.rs gates doc/mission/world behind features, so a featureless clippy COMPILES
      # NONE OF THEM and reports success on code it never read.
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

# T-515 — Class-R: SQL-only claim migration 0016 must keep its claim UPDATE body.
#
# `tests/db_migrate.rs` only asserts schema/object counts after sqlx migrate. A hollow
# 0016 that drops the claim UPDATE (`UPDATE match_player_stats … SET discord_id`) but
# keeps `REFRESH MATERIALIZED VIEW` still lands the same table/enum/matview census and
# stays gate-green when claimable orphans are 0. That class of defect is invisible to
# the Rust gate; pin the claim needles on disk here.
#
# Needles measured from apps/website/api/migrations/0016_backfill_pre_t326_linked_match_stats.sql
# claim step 2 (not comments — comment prose uses unqualified `discord_id IS NULL`).
#
# Path override TBD_GATE_MIGRATION_0016 is for perturbation probes only (point at a bait
# file missing the UPDATE) — never for production gating.
gate_db_migrate_claim_body() {
  local f="${TBD_GATE_MIGRATION_0016:-$ROOT/apps/website/api/migrations/0016_backfill_pre_t326_linked_match_stats.sql}"
  local needle miss=() body
  if [ ! -f "$f" ]; then
    echo "db_migrate claim body: missing migration file: $f"
    echo "        T-335 0016 is the one-shot claim for pre-T-326 linked accounts; without it"
    echo "        this Class-R cannot pin the UPDATE body. Restore the file or unset"
    echo "        TBD_GATE_MIGRATION_0016."
    return 1
  fi
  # Strip /*…*/ block comments (incl. multiline) then -- line comments before needle
  # search so comment-only bait cannot false-green (T-523 / verifier MAJOR).
  body=$(awk '
    BEGIN { inblock = 0 }
    {
      s = $0
      out = ""
      while (length(s) > 0) {
        if (inblock) {
          idx = index(s, "*/")
          if (idx == 0) { s = ""; break }
          s = substr(s, idx + 2)
          inblock = 0
          continue
        }
        i_block = index(s, "/*")
        i_line = index(s, "--")
        if (i_block == 0 && i_line == 0) {
          out = out s
          break
        }
        if (i_line > 0 && (i_block == 0 || i_line < i_block)) {
          out = out substr(s, 1, i_line - 1)
          break
        }
        out = out substr(s, 1, i_block - 1)
        s = substr(s, i_block + 2)
        inblock = 1
      }
      print out
    }
  ' "$f")
  for needle in \
    'UPDATE public.match_player_stats AS s' \
    'SET discord_id = u.discord_id' \
    'AND s.discord_id IS NULL'
  do
    printf '%s\n' "$body" | grep -qF -- "$needle" || miss+=("$needle")
  done
  if [ "${#miss[@]}" -gt 0 ]; then
    echo "db_migrate claim body: FAIL — $f is missing claim UPDATE needle(s):"
    for needle in "${miss[@]}"; do
      echo "        - $needle"
    done
    echo "        Hollow 0016 (REFRESH kept, claim UPDATE dropped) still passes schema counts."
    echo "        Restore the T-335 claim UPDATE body (do not weaken this assert)."
    return 1
  fi
  echo "db_migrate claim body: OK — 0016 retains claim UPDATE needles ($f)"
  return 0
}

# T-555 — THE POPULATED-DATABASE MIGRATION STEP. Read this header before changing anything below.
#
# ── WHAT WAS WRONG, AND WHY NO GATE COULD SEE IT ─────────────────────────────────────────────────
#
# `ensure_gate_db` force-drops `tbd_gate_migrate` at the start of EVERY run. So `db_migrate` could
# only ever run the migration chain FORWARD FROM EMPTY. Two whole classes of defect are invisible
# from there, because both need a database that already contains something:
#
#   1. EDITING AN ALREADY-APPLIED MIGRATION. sqlx checksums the WHOLE FILE (sha384) and stores it in
#      `_sqlx_migrations`. Change so much as one comment character and every database that already
#      ran that file refuses to boot: `migration N was previously applied but has been modified`.
#      From empty there is nothing to compare against, so the checksum matches BY CONSTRUCTION.
#   2. DDL THAT CANNOT SURVIVE REAL ROWS. `CREATE UNIQUE INDEX` on a column pair that already has a
#      duplicate; `SET NOT NULL` on a column that already has a NULL. From empty there are no rows,
#      so the DDL applies BY CONSTRUCTION.
#
# Both landed. a843905f (T-331) retouched an applied 0009 — comment-only, SQL byte-identical — and
# killed every existing database. 0017 (T-511) created a unique index over a duplicate seat the
# pre-T-331 seed had already inserted, and its own header asserted the row had been cleared; T-331
# had fixed the SEED FILE, which does nothing to data already seeded. EVERY WAVE GATE SINCE T-331
# WAS GREEN OVER BOTH — including, on deploy, staging and production. Not a test that examined
# nothing: a whole category, backward compatibility, that the gate architecture excluded by design.
#
# ── THE CURE: A DATABASE THAT IS NEVER DROPPED ───────────────────────────────────────────────────
#
# `tbd_gate_migrate_persist` survives across runs. There is no DROP DATABASE in this function and
# there must never be one — the persistence IS the test. Each run:
#
#   AUDIT   every migration `_sqlx_migrations` says was applied is re-hashed on disk and compared.
#           A drifted checksum is the exact failure a real boot would hit, caught before landing.
#   APPLY   only the migrations this database has not seen, against the rows it is already carrying.
#   SEED    re-applies seeds/content_golden.sql so the database stays POPULATED for the next wave.
#           An empty persist DB would make step APPLY vacuous again, which is the whole defect.
#
# ── TWO MODES, AND WHY THE SLICE GATE DOES NOT COMMIT ────────────────────────────────────────────
#
#   audit    (gate_slice) read-only audit, then each pending migration is executed inside an
#            explicit transaction that is ROLLED BACK. A unique-index violation is raised while the
#            index is being BUILT, inside that transaction, so the rollback costs nothing in
#            detection — measured against the real defect, which reproduces identically either way.
#            A slice must NOT advance the shared database: slices get abandoned, and a persist DB
#            carrying a migration that never reached main would fail every later run with
#            "applied version has no file on disk" — a self-inflicted red nobody could act on.
#   advance  (cmd_gate, on merged main) the same audit, then pending migrations are COMMITTED and
#            recorded. Only merged history advances the database, so its state is always some
#            prefix of main.
#
# ── CHECKSUM PARITY IS MEASURED, NOT ASSUMED ─────────────────────────────────────────────────────
#
# sqlx's checksum is sha384 over the raw file bytes. That is not taken on faith from the source:
# 2026-07-27, all 17 on-disk migrations were hashed with `sha384sum` and compared against the
# `_sqlx_migrations.checksum` values sqlx ITSELF wrote into the operator's dev database. 16 of 17
# matched byte-for-byte. The seventeenth was migration 9 — the defect, not a parity failure — and
# the pre-a843905f bytes hash to exactly the value sqlx had recorded. If a future sqlx changes the
# algorithm this step goes red on everything at once, which is the correct way to find that out.
#
# The applier below is psql, not sqlx. The one behavioural difference is statement framing: sqlx
# sends a migration as ONE multi-statement simple query, psql `-f` sends them individually. Both
# run inside ONE transaction per migration — the property migrations actually depend on — and the
# bookkeeping INSERT is inside that same transaction, so a migration is never recorded as applied
# unless it applied. A DDL failure is a DDL failure under either framing.
#
# ── ANTI-VACUITY ─────────────────────────────────────────────────────────────────────────────────
#
# This step exists because a check reported success over an input it never examined, so it is not
# permitted to do that itself. Every one of these is a FAIL, never a skip:
#   * sha384sum or psql missing / the database unreachable  (tool absent must fail closed)
#   * zero migration files found
#   * an applied version with no matching file on disk
#   * a migration recorded with success = false
#   * THE POPULATION FLOOR — after seeding, the tables migrations actually constrain must contain
#     rows, INCLUDING at least one CLAIMED orbat seat. An empty database passes any DDL, so a
#     persist DB that lost its data would turn this step back into the thing it replaced.
# The step also prints what it looked at — audited / pending / applied counts and the row floor —
# because a verdict you cannot attribute to an input is not evidence.
#
# ── WHAT THIS STEP DOES NOT CATCH, MEASURED ──────────────────────────────────────────────────────
#
# The checksum half is absolute: from the second run onward, ANY edit to an applied migration is
# caught, whatever the data.
#
# The DDL half is only ever as good as the rows this database happens to carry, and a VIRGIN
# persist DB carries only what today's seed inserts. Measured 2026-07-27: bootstrap a fresh persist
# DB with the current (T-331-fixed) content_golden and the PRE-T-555 0017, and it passes — because
# the fixed seed no longer produces the duplicate seat that 0017 died on. The defect only
# reproduces on a database that ran the OLD seed, which is what every real database did.
#
# So the value here compounds with age: DO NOT DROP tbd_gate_migrate_persist to "clean it up". Its
# accumulated state — rows written by older seeds at older schema versions — is the asset, and it
# is the only thing standing in for the shape of a production database. The recovery advice in the
# `missing file` branch below is a last resort and it costs exactly that history.
#
# The strictly better version of this step seeds the persist DB from the OLDEST committed seed
# rather than the current one, so a virgin bootstrap reproduces the historical shapes the current
# seed has since fixed. That is left unfilled deliberately rather than half-built.
gate_db_migrate_persist() {
  local mode="${1:-audit}"
  local db="${TBD_GATE_MIGRATE_PERSIST_DB:-tbd_gate_migrate_persist}"
  local migdir="${TBD_GATE_MIGRATION_DIR:-$ROOT/apps/website/api/migrations}"
  local seed="${TBD_GATE_MIGRATE_SEED:-$ROOT/apps/website/api/seeds/content_golden.sql}"
  local label="db_migrate persist"

  case "$mode" in audit|advance) ;; *)
    echo "$label: FAIL — unknown mode '$mode' (want audit|advance)"; return 1 ;;
  esac
  if ! [[ "$db" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
    echo "$label: FAIL — database name '$db' is not a safe SQL identifier."; return 1
  fi
  # Tool-absent fails closed. A missing hasher would otherwise make every checksum compare equal to
  # the empty string and the audit would agree with itself over nothing.
  command -v sha384sum >/dev/null 2>&1 || {
    echo "$label: FAIL — sha384sum not on PATH; the checksum audit cannot run."; return 1; }

  local px=(podman exec tbd_reforger_db psql -U tbd)
  [ "$HOST_BRIDGE" = 1 ] && px=(distrobox-host-exec "${px[@]}")
  q()     { "${px[@]}" -d "$db"    -qtA -v ON_ERROR_STOP=1 -c "$1"; }
  admin() { "${px[@]}" -d postgres -qtA -v ON_ERROR_STOP=1 -c "$1"; }

  # `advance` writes to a database every other gate on this machine shares. Same invariant
  # ensure_gate_db asserts, and for the same reason — assert it rather than assume it.
  if [ "$mode" = advance ] && [ "${GATE_LOCK_HELD:-0}" != 1 ] && [ "${GATE_UNSERIALISED:-0}" != 1 ]; then
    echo "$label: FAIL — advance mutates the shared persist DB and the gate lock is NOT held."
    return 1
  fi

  # ── the migration set on disk ──────────────────────────────────────────────────────────────────
  local files=() f
  while IFS= read -r f; do [ -n "$f" ] && files+=("$f"); done < <(ls -1 "$migdir"/*.sql 2>/dev/null | sort)
  if [ "${#files[@]}" -eq 0 ]; then
    echo "$label: FAIL — no migrations found under $migdir. Nothing would be examined."; return 1
  fi

  if ! admin "SELECT 1;" >/dev/null 2>&1; then
    echo "$label: FAIL — cannot reach Postgres (podman exec tbd_reforger_db). Is \`make db-up\` running?"
    echo "        This is a FAIL and not a skip on purpose: a migration audit that silently"
    echo "        examined no database is the defect this step was built to end."
    return 1
  fi
  admin "CREATE DATABASE ${db};" >/dev/null 2>&1 || true   # already-exists is fine; never dropped
  q "CREATE TABLE IF NOT EXISTS _sqlx_migrations (
       version bigint PRIMARY KEY, description text NOT NULL,
       installed_on timestamptz NOT NULL DEFAULT now(), success boolean NOT NULL,
       checksum bytea NOT NULL, execution_time bigint NOT NULL);" >/dev/null 2>&1 || {
    echo "$label: FAIL — could not open or initialise ${db}."; return 1; }

  local ver desc sum applied_sum ok_n=0 pending=() drift=() missing=() failed=()
  mig_ver()  { basename "$1" | sed 's/^0*\([0-9][0-9]*\)_.*/\1/'; }
  mig_desc() { basename "$1" .sql | sed 's/^[0-9][0-9]*_//; s/_/ /g'; }

  # ── BOOTSTRAP ─────────────────────────────────────────────────────────────────────────────────
  # A brand-new persist DB has nothing to audit and nothing to apply against, so bootstrapping it
  # forward-from-empty would reproduce exactly the hole this step closes. Bootstrap therefore stops
  # ONE SHORT of the newest migration and seeds there, so even the first ever run applies the newest
  # file against populated data. Every later run is the steady state and needs no such trick.
  local have_any; have_any=$(q "SELECT count(*) FROM _sqlx_migrations;" 2>/dev/null | tr -d '[:space:]')
  if [ "${have_any:-0}" = 0 ]; then
    echo "  bootstrapping ${db}: applying ${#files[@]} migration(s) minus the newest, then seeding"
    local i last=$(( ${#files[@]} - 1 ))
    for ((i = 0; i < last; i++)); do
      persist_apply_one "$db" "${files[$i]}" commit || {
        echo "$label: FAIL — bootstrap could not apply $(basename "${files[$i]}")."; return 1; }
    done
    persist_seed "$db" "$seed" || return 1
  fi

  # ── AUDIT: every applied migration re-hashed against disk ─────────────────────────────────────
  local rows row
  # `success` is spelled out rather than concatenated raw: `boolean || text` renders as
  # `true`/`false`, not the `t`/`f` psql prints for a bare boolean column, and comparing against the
  # wrong one flags EVERY migration as partially-applied. Caught by this step's own perturbation run.
  rows=$(q "SELECT version || '|' || (CASE WHEN success THEN 'ok' ELSE 'bad' END)
                   || '|' || encode(checksum,'hex') FROM _sqlx_migrations ORDER BY version;")
  while IFS='|' read -r ver row applied_sum; do
    [ -z "$ver" ] && continue
    f=""
    local cand; for cand in "${files[@]}"; do [ "$(mig_ver "$cand")" = "$ver" ] && { f="$cand"; break; }; done
    if [ -z "$f" ]; then missing+=("$ver"); continue; fi
    [ "$row" = "ok" ] || failed+=("$ver")
    sum=$(sha384sum < "$f" | cut -d' ' -f1)
    if [ "$sum" != "$applied_sum" ]; then
      drift+=("$ver|$(basename "$f")|$applied_sum|$sum")
    else
      ok_n=$((ok_n + 1))
    fi
  done <<< "$rows"

  local applied_versions; applied_versions=$(q "SELECT string_agg(version::text, ' ') FROM _sqlx_migrations;")
  for f in "${files[@]}"; do
    ver=$(mig_ver "$f")
    grep -qw -- "$ver" <<< " $applied_versions " || pending+=("$f")
  done

  local bad=0
  if [ "${#drift[@]}" -gt 0 ]; then
    bad=1
    echo "$label: FAIL — ${#drift[@]} ALREADY-APPLIED migration(s) were MODIFIED on disk."
    echo "        Every existing database — dev, staging, production — will refuse to boot with"
    echo "        \`migration N was previously applied but has been modified\` (sqlx VersionMismatch)."
    for row in "${drift[@]}"; do
      IFS='|' read -r ver f applied_sum sum <<< "$row"
      echo "        - migration $ver  $f"
      echo "            applied: $applied_sum"
      echo "            on disk: $sum"
    done
    echo "        An applied migration is IMMUTABLE — sqlx hashes the whole file, so a comment-only"
    echo "        edit is as fatal as a DDL one. Restore the original bytes and put the new prose in"
    echo "        the migration that has not shipped yet, or in a new one."
  fi
  if [ "${#missing[@]}" -gt 0 ]; then
    bad=1
    echo "$label: FAIL — applied migration(s) with NO file on disk: ${missing[*]}"
    echo "        Either a migration was deleted/renamed after shipping (real databases can never"
    echo "        reach the new chain), or this persist DB was advanced by something that never"
    echo "        merged. Recover with: DROP DATABASE ${db}; the next gate rebuilds it."
  fi
  if [ "${#failed[@]}" -gt 0 ]; then
    bad=1
    echo "$label: FAIL — migration(s) recorded with success=false: ${failed[*]} (partially applied)."
  fi
  [ "$bad" -ne 0 ] && return 1

  # ── APPLY the pending migrations against the rows this database already carries ───────────────
  local applied_n=0 finish=rollback
  [ "$mode" = advance ] && finish=commit
  for f in "${pending[@]}"; do
    persist_apply_one "$db" "$f" "$finish" || {
      echo "$label: FAIL — $(basename "$f") does not apply to a POPULATED database."
      echo "        It applies to an empty one, which is why every gate before this step was green."
      echo "        Neutralise the offending rows FIRST, in the same migration, then constrain —"
      echo "        see 0010_backfill_aar_replay_url_scheme.sql (T-405) for the established shape."
      return 1; }
    applied_n=$((applied_n + 1))
  done

  # Re-seed so the NEXT wave still meets real rows. Only in advance mode: audit rolled its pending
  # migrations back, so the schema it would seed against is not the one that will persist.
  [ "$mode" = advance ] && { persist_seed "$db" "$seed" || return 1; }

  # ── THE POPULATION FLOOR — the guard that stops this step going hollow ────────────────────────
  local floor
  floor=$(q "SELECT (SELECT count(*) FROM orbat_slots WHERE assigned_to IS NOT NULL) || ' ' ||
                    (SELECT count(*) FROM matches) || ' ' ||
                    (SELECT count(*) FROM match_player_stats);" 2>/dev/null)
  local seats rows_m rows_s; read -r seats rows_m rows_s <<< "$floor"
  if [ "${seats:-0}" -lt 1 ] || [ "${rows_m:-0}" -lt 1 ] || [ "${rows_s:-0}" -lt 1 ]; then
    echo "$label: FAIL — ${db} is not populated (claimed seats=${seats:-?} matches=${rows_m:-?} stats=${rows_s:-?})."
    echo "        Every DDL check above passed over an empty table, which proves nothing. That is"
    echo "        precisely the failure this step exists to prevent, so it is a red, not a pass."
    return 1
  fi

  echo "$label: OK [$mode] — audited ${ok_n} applied migration(s) against disk, ${applied_n} pending"
  echo "        applied to a populated ${db} (claimed seats=${seats} matches=${rows_m} stats=${rows_s})."
  return 0
}

# Stdin -> psql on <db>. Explicit rather than inherited: bash's dynamic scoping would let the
# helpers below read the caller's locals, and a helper whose database depends on who called it is
# exactly the kind of thing that quietly runs against the wrong one.
persist_feed() {
  local db="$1"
  if [ "$HOST_BRIDGE" = 1 ]; then
    distrobox-host-exec podman exec -i tbd_reforger_db psql -U tbd -d "$db" -q -v ON_ERROR_STOP=1 -f -
  else
    podman exec -i tbd_reforger_db psql -U tbd -d "$db" -q -v ON_ERROR_STOP=1 -f -
  fi
}

# One migration, one transaction — the migration body AND its `_sqlx_migrations` row together, so a
# migration can never be recorded as applied unless it applied. `rollback` runs the identical
# transaction and throws it away: used by the slice gate, which must detect without advancing.
persist_apply_one() {
  local db="$1" f="$2" finish="$3" ver desc sum
  ver=$(basename "$f" | sed 's/^0*\([0-9][0-9]*\)_.*/\1/')
  desc=$(basename "$f" .sql | sed 's/^[0-9][0-9]*_//; s/_/ /g')
  sum=$(sha384sum < "$f" | cut -d' ' -f1)
  { echo "BEGIN;"
    cat "$f"
    echo
    echo "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)"
    echo "VALUES (${ver}, '${desc}', true, decode('${sum}','hex'), 0);"
    if [ "$finish" = commit ]; then echo "COMMIT;"; else echo "ROLLBACK;"; fi
  } | persist_feed "$db"
}

persist_seed() {
  local db="$1" seed="$2" out rc
  [ -f "$seed" ] || { echo "db_migrate persist: FAIL — seed not found: $seed"; return 1; }
  out=$(persist_feed "$db" < "$seed" 2>&1); rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "db_migrate persist: FAIL — the committed seed no longer loads into the migrated schema."
    echo "        A migration that makes seeds/content_golden.sql unloadable breaks every fresh"
    echo "        environment, and leaves this persist DB unpopulated for the next wave."
    printf '%s\n' "$out" | tail -8 | sed 's/^/        /'
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
  # MEASURED 2026-07-26: Cursor/agent shells export NO_COLOR=1. trunk 0.21.14's clap binds
  # that env to `--no-color` and then rejects the value `1` (`possible values: true, false`),
  # so the wave gate printed `trunk build FAIL` over a healthy tree. Unset for this step only.
  hostrun sh -c "cd '$fdir' && unset NO_COLOR && CARGO_TARGET_DIR='$GATE_TRUNK_TARGET' trunk build --release --dist '$GATE_TRUNK_DIST'" \
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
#   CTX  height-labels      PER-CONTEXT (T-422). In make schema-validate always. GREEN on main
#                                 when the DEM is a real PNG (~71 MB, magic \x89PNG). In a
#                                 worktree that still has the 133-byte LFS pointer, the same
#                                 sub-gate is RED for an env reason, not a schema reason. T-420
#                                 forever-excluded it off a worktree measurement and told
#                                 maintainers to `make lfs-dem` for a main-only green gate that
#                                 was never red on main. Runtime inclusion follows the DEM in
#                                 THIS tree; the tripwire still demands the Makefile name.
# Also enumerated and NOT wired, because they are not in the schema-validate contract — they belong
# to `make verify-terrain` and the label lane, and widening the gate past its stated authority is a
# separate decision from closing this hole:
#   n/a  terrain-manifest   rc=0  manifest schema + terrains cross-check   (make verify-terrain)
#   n/a  locations          rc=0  locations G2-G7
#   n/a  town-labels        rc=0  town-label gates
#   n/a  road-names         rc=0  road-name gates
#   n/a  terrain-alignment  — same DEM dependency as height-labels; still outside this contract.
#   n/a  codegen / validate-file / flatten-orbat-slots — generators and tools, not gates.
#
# GATE_SCHEMA_VALIDATE_GATES must equal `make schema-validate`'s sub-gate SET (order = Makefile).
# citations comes from `make verify-citations` / ci-local-schema and is layered on after the
# tripwire. height-labels stays in VALIDATE_GATES even when a worktree skips running it.
GATE_SCHEMA_VALIDATE_GATES="validate map-object-golden map-glyphs height-labels map-object-enums type-inventory t090-specs n6 n10"
GATE_SCHEMA_EXTRA_GATES="citations"
# DEM path height-labels (and terrain-alignment) decode. Probe is PNG magic, not byte size —
# size alone would green a truncated file and red a future compressor win.
GATE_SCHEMA_DEM="packages/map-assets/everon/dem/everon-dem-16bit.png"
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
# dir grows without bound at ~1.7 GB each), plus a CONTENT stamp: when this tree's xtask *and its
# path deps* hash differently from whatever last built here, the dir is thrown away and rebuilt.
# T-420 stamped only xtask/src + xtask/Cargo.toml + Cargo.lock while xtask depends on tbd-tools
# and map-engine-core BY PATH — two slice trees could share GATE_SCHEMA_TARGET with the same stamp
# while map-engine-core differed (T-422 defect 3; T-421's touch_workspace closed the live mtime
# foreign-binary path, but the stamp itself stayed incomplete). Measured: 14 s from cold, ~0.1 s
# warm, 1.7 GB resident. Content, not mtime — mtime is the thing that lied.
GATE_SCHEMA_TARGET="${TBD_GATE_SCHEMA_TARGET:-$MAIN_ROOT/target-gate-schema}"

# True iff THIS tree's Everon DEM is a real PNG (not a git-lfs pointer, not missing).
schema_dem_materialized() {
  [ -f "$GATE_SCHEMA_DEM" ] || return 1
  local sig
  sig="$(od -An -tx1 -N8 "$GATE_SCHEMA_DEM" | tr -d ' \n')"
  [ "$sig" = "89504e470d0a1a0a" ]
}

# Parse `make schema-validate` recipe names. Survives blank lines, column-0 `#` comments, and
# backslash continuations — the three shapes that made T-420's awk silently narrow (3-of-9 /
# 8-of-9) while GNU make still ran all nine. Ends the recipe only on a real next target line.
schema_makefile_validate_gates() {
  awk '
    /^schema-validate:/ { i=1; next }
    i {
      if (/^[[:space:]]*$/) next
      if (/^#/) next
      if (!/^\t/) exit
      line = $0
      sub(/^\t+/, "", line)
      while (line ~ /\\[[:space:]]*$/) {
        sub(/\\[[:space:]]*$/, "", line)
        if ((getline nxt) <= 0) break
        sub(/^\t+/, "", nxt)
        if (nxt ~ /^#/ || nxt ~ /^[[:space:]]*$/) continue
        line = line nxt
      }
      print line
    }
  ' Makefile | sed -n 's/.*-p xtask -- schema \([a-z0-9-]*\).*/\1/p'
}

gate_schema() {
  # DRIFT TRIPWIRE. A hardcoded list is readable and greppable but it rots silently, and the way it
  # rots is precisely this ticket: `make schema-validate` grows a tenth sub-gate, nobody adds it
  # here, and the wave gate goes on printing PASS over whatever that gate checks. Diff the SET
  # against the Makefile recipe every run and refuse when they disagree — including PARTIAL
  # parses. T-420 only refused an EMPTY parse; a blank/`#`/continuation mid-recipe narrowed the
  # awk output while make still ran all nine, and the one-way ⊆ check stayed green over the hole.
  local mk_gates mk_sorted want_sorted
  mk_gates="$(schema_makefile_validate_gates)"
  if [ -z "$mk_gates" ]; then
    echo "schema: read 0 sub-gates out of the schema-validate recipe in Makefile."
    echo "        The drift check is the only thing keeping this step's list honest, so a step that"
    echo "        could not run it must not go on to report PASS. Fix the parse, or the recipe."
    return 1
  fi
  mk_sorted="$(printf '%s\n' $mk_gates | LC_ALL=C sort | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  want_sorted="$(printf '%s\n' $GATE_SCHEMA_VALIDATE_GATES | LC_ALL=C sort | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
  if [ "$mk_sorted" != "$want_sorted" ]; then
    echo "schema: Makefile schema-validate set disagrees with GATE_SCHEMA_VALIDATE_GATES."
    echo "        makefile: $mk_sorted"
    echo "        wave.sh:  $want_sorted"
    echo "        A narrowed parse or a tenth sub-gate would keep printing PASS over unchecked"
    echo "        contracts. Fail closed: sync the list, or fix the recipe parse."
    return 1
  fi

  # Runtime run-set: every VALIDATE gate, minus height-labels only when THIS tree's DEM is not a
  # materialized PNG, plus citations. Never a forever-exclusion list.
  local run_gates="" g skipped=""
  for g in $GATE_SCHEMA_VALIDATE_GATES; do
    if [ "$g" = "height-labels" ] && ! schema_dem_materialized; then
      skipped="height-labels"
      echo "schema: height-labels SKIP in this tree — $GATE_SCHEMA_DEM is not a materialized PNG"
      echo "        (LFS pointer or missing). On main with a real DEM this sub-gate RUNS; do not"
      echo "        treat a worktree skip as 'red on main' or chase make lfs-dem for that."
      continue
    fi
    run_gates="$run_gates $g"
  done
  for g in $GATE_SCHEMA_EXTRA_GATES; do
    run_gates="$run_gates $g"
  done
  run_gates="${run_gates# }"
  if [ -z "$run_gates" ]; then
    echo "schema: run-set is empty after per-context filtering — refusing vacuous PASS."
    return 1
  fi

  # ---- make sure the xtask we are about to trust is THIS tree's (see GATE_SCHEMA_TARGET) ----
  local nsrc
  nsrc=$(find xtask/src crates/map-engine-core/src tools/tbd-tools/src -name '*.rs' -type f 2>/dev/null | wc -l)
  if [ "$nsrc" -eq 0 ]; then
    echo "schema: found no stamp inputs under xtask/ + map-engine-core/ + tbd-tools/ — cannot tell whose binary would run."
    return 1
  fi
  local stamp
  # Path deps matter: xtask/Cargo.toml pulls tbd-tools + map-engine-core by path. Stamping only
  # xtask left two trees with divergent map-engine-core sharing one GATE_SCHEMA_TARGET stamp.
  stamp="$( { find xtask/src crates/map-engine-core/src tools/tbd-tools/src -name '*.rs' -type f 2>/dev/null \
                | LC_ALL=C sort | xargs cat
              cat xtask/Cargo.toml crates/map-engine-core/Cargo.toml tools/tbd-tools/Cargo.toml Cargo.lock
            } 2>/dev/null | cksum | tr -d ' ')"
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

  local want=0
  for g in $run_gates; do want=$((want+1)); done

  local rc ran=0 timedout=0 failed="" detail="" out
  for g in $run_gates; do
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

  # NON-VACUITY. An empty run-set, or a loop that exits early, reaches the verdict below
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
    if [ -n "$skipped" ]; then
      echo "schema: FAILED$failed  ($ran sub-gates run; context-skipped: $skipped — DEM not materialized here)"
    else
      echo "schema: FAILED$failed  ($ran sub-gates run)"
    fi
    [ "$timedout" -eq 1 ] && return 124
    return 1
  fi
  if [ -n "$skipped" ]; then
    echo "schema: $ran sub-gates OK ($run_gates; context-skipped: $skipped)"
  else
    echo "schema: $ran sub-gates OK ($run_gates)"
  fi
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
  # Same committed ∪ working-tree union as changed_rs. Diffing the range alone refused
  # `gate --slice` when a slice had working-tree changes but no commits yet — contradicting
  # changed_rs's stated purpose (T-409 NIT; pre-existing, not T-406).
  # Porcelain via git_porcelain_paths (T-401) — never treat LFS filter exit 128 as empty.
  local n wt
  wt="$(git_porcelain_paths)" || return $?
  n=$( { git diff --name-only "$range" 2>/dev/null || true
         printf '%s\n' "$wt"
       } | sort -u | sed '/^$/d' | wc -l)
  [ "$n" -gt 0 ] && return 0
  echo "gate: '$range' (plus working tree) contains no changed files — refusing to run."
  echo "        Every change-scoped step (wasm32, fmt, clippy, trunk) would report PASS/SKIP"
  echo "        without reading a line, and the verdict would describe nothing."
  echo "        $what"
  return 2
}

# ── WAVE GATE BASE ───────────────────────────────────────────────────────────────────────────────
#
# T-602. THE FIX EXISTED; THE DEFAULT WAS THE HAZARD.
#
# `cmd_gate` took `${1:-HEAD~1}` and every change-scoped step keys off it, so omitting the argument
# silently shrank the gate's scope to the last commit and the verdict still read PASS.
#
# OBSERVED, closing wave 75: the command center ran `wave.sh gate` with no base. After five merges
# `HEAD~1` was the LAST MERGE ONLY. GATE reported PASS 26/26 over a wave in which 4 of 5 slices
# changed the frontend and `trunk build` never ran; re-run against the real base it was 27/27 with
# the trunk build actually building. REPRODUCED here on wave 76's committed history before fixing:
#
#     base HEAD~1     ->  wasm32 (frontend)  PASS   "frontend untouched"
#                         trunk build        SKIP (frontend untouched this wave)
#                         touch_changed      0 changed .rs file(s)
#     base 1614c557   ->  apps/website/frontend/src/arsenal_rules.rs changed this wave
#                         trunk build        WOULD RUN
#                         touch_changed      1 changed .rs file(s)
#
# Four steps narrow, not the two first blamed: touch_changed, wasm32 (frontend), fmt (changed) and
# the trunk conditional. `test xtask+tbd-tools` and the other unconditional steps are unaffected.
#
# WHY DERIVE-AND-VERIFY RATHER THAN "MAKE THE BASE MANDATORY".
# Mandatory moves the computation to the operator — the same operator who got it wrong, and who has
# no cheaper way to compute it than this function does. It would also have to be threaded through
# `wave --close`, which already passes a base and already passes a WRONG one (see below). So:
#   * with no argument, DERIVE the base from the wave-close marker. Exact, not a guess.
#   * with or without an argument, VERIFY the base covers the whole wave and REFUSE if it does not.
#     Verification is what catches an explicit base, which is the half a mandatory argument cannot.
#   * never fall back to HEAD~1. There is no wave for which "the last commit" is a safe default.
#
# `origin/main` was the ticket's suggested derivation and it is measurably wrong here: main is
# pushed at every wave close, so at gate time `origin/main` == HEAD and `git merge-base origin/main
# HEAD` returns HEAD — the vacuous range this function exists to refuse. Verified 2026-07-31:
# `git rev-parse origin/main` == `git rev-parse HEAD` == efc3851c.
#
# The commit `wave --close` writes at the end of every wave. 33 in history (waves 45-77, recounted
# 2026-08-01), one format, varying only after the word CLOSED: `wave 76 CLOSED — …`,
# `wave 75 CLOSED: …`. Nothing else has ever followed `CLOSED` in any of them.
#
# T-613 — THIS IS ANCHORED, AND THE ANCHOR IS HALF THE FIX. It used to accept ANYTHING after
# `CLOSED`, so a subject that merely CONTINUES past the word became a wave base. Wave 77's verifier
# proved it in a clone with `wave 76 CLOSED? reopened — reverting T-608 pending re-gate`: derivation
# returned the fabricated commit, the gate range collapsed to ONE commit, and the entire wave sat
# outside every change-scoped step — the wave-75 incident T-602 exists to prevent, reachable
# through the front door.
#
# THE DELIMITER SET IS end-of-subject, `:`, ` —`, ` -` AND NOTHING ELSE. Reasoning, because the
# next reader will want to widen it: `CLOSED` alone plus the two forms above are everything
# `wave --close` and 33 real commits have produced, and the ASCII ` -` is admitted only because the
# em dash is a keyboard hazard, not because anything writes it. Every widening admits a class of
# English continuation — `CLOSED?`, `CLOSED,`, `CLOSED (partially)`, `CLOSEDish` — and each of
# those is a plausible thing a hurried operator writes about a wave that DID NOT close. The cost of
# being too strict is a wave-close commit that has to be reworded once; the cost of being too loose
# is a gate reporting PASS over a wave it never read.
#
# This ERE is the PREFILTER ONLY. `git rev-list --grep` runs it through GIT's regex engine, not the
# system grep, so the ugrep/GNU divergence noted inside prev_wave_close does not reach it. The
# AUTHORITY is wave_close_subject_ok below, which is pure `case` and therefore the same program
# under every shell here. Measured 2026-08-01: this pattern and the old loose one select exactly
# the same 33 commits, so anchoring the prefilter cannot lose a real marker.
WAVE_CLOSE_MARKER_RE='^wave [0-9]+ CLOSED([:]|$| —| -)'

# Is this SUBJECT a wave-close marker? Pure `case`, no regex at all — see the note inside
# prev_wave_close for why a glob and not grep, which T-613 preserves rather than replaces. The
# number is validated as digits, so `wave 7x CLOSED` cannot become a boundary either.
wave_close_subject_ok() {
  local s="$1" rest n
  case "$s" in wave\ *) ;; *) return 1 ;; esac
  rest="${s#wave }"
  n="${rest%% *}"
  case "$n" in ''|*[!0-9]*) return 1 ;; esac
  rest="${rest#"$n"}"
  case "$rest" in
    ' CLOSED')    return 0 ;;
    ' CLOSED:'*)  return 0 ;;
    ' CLOSED —'*) return 0 ;;
    ' CLOSED -'*) return 0 ;;
  esac
  return 1
}

# The wave NUMBER a marker claims. Empty + rc 1 for anything that is not an anchored marker.
wave_close_number() {
  local s n
  s="$(git log -1 --format=%s "$1" 2>/dev/null)" || return 1
  wave_close_subject_ok "$s" || return 1
  n="${s#wave }"; n="${n%% *}"
  printf '%s\n' "$n"
}

# Has this wave-close been DISAVOWED by a later revert? Prints the reverting commit; rc 0 if so.
#
# T-613 (verifier F6). A reverted close still derived as the base, so a wave the operator had
# explicitly taken back was never re-gated — narrow and silent, the same shape as everything else
# on this page. Derivation now SKIPS a disavowed marker and falls through to the one before it,
# which re-gates the disavowed wave's whole span. That is the over-broad direction, which this
# file has already established is the safe one.
#
# The evidence is git's OWN trailer, `This reverts commit <full sha>.`, written by `git revert` and
# by nothing in this repo. A hand-written revert that omits the trailer is NOT detectable here;
# that limitation is printed by the caller rather than left for someone to discover.
#
# `--fixed-strings` keeps this cheap: git prefilters to the handful of commits that quote the sha
# at all. Without it this forks `git log` once per commit in the range.
wave_close_disavowed() {
  local full c
  full="$(git rev-parse --verify --quiet "$1^{commit}" 2>/dev/null)" || return 1
  while read -r c; do
    [ -z "$c" ] && continue
    case "$(git log -1 --format=%B "$c" 2>/dev/null)" in
      *"This reverts commit ${full}."*) printf '%s\n' "$c"; return 0 ;;
    esac
  done < <(git rev-list --fixed-strings --grep="This reverts commit ${full}." "$full..HEAD" 2>/dev/null)
  return 1
}

# The previous wave's close commit = the SHA main was at when THIS wave opened.
# Empty + rc 1 when no marker is reachable. That is a real state (a tree before wave 1) and the
# caller refuses on it — this function does not invent a fallback, because inventing one is how
# `HEAD~1` got here.
#
# HEAD IS EXCLUDED DELIBERATELY. `wave --close` gates BEFORE writing its own marker, so the newest
# reachable marker is always the previous wave's.
#
# WHAT THAT NO LONGER MEANS, corrected T-618 because the sentence that used to end this paragraph
# promised behaviour the code now refuses. It said re-gating an already-closed tree "picks the
# previous close again and re-gates that whole wave, rather than gating nothing". The picking still
# happens — but T-613's ORACLE 1 then refuses the result, because the close sitting AT HEAD is
# reachable and claims a HIGHER wave than the base just derived, which is exactly the contradiction
# that oracle exists to report. Measured at b2afc99a (wave 78's own close, checked out): derives
# 2b144b5d, then refuses with "CONTRADICTED by the marker ledger", rc 2. That is fail-CLOSED and it
# is not this ticket's to change, but it is not "re-gates that whole wave" either, and a reader who
# believes the old sentence will go hunting for a bug that is really a deliberate refusal.
prev_wave_close() {
  local head sha subj rev
  head="$(git rev-parse HEAD 2>/dev/null)" || return 1
  while read -r sha; do
    [ -z "$sha" ] && continue
    [ "$sha" = "$head" ] && continue
    # git's --grep matches the WHOLE message, so a body line quoting the marker would false-match;
    # `wave --close` writes it as the SUBJECT, so confirm it there. A bash glob rather than grep on
    # purpose — see the note on this file's grep usage: `rg` does not exist under `bash -c` and the
    # two greps on this machine (ugrep interactively, GNU under `bash script.sh`) disagree on ERE
    # details. A `case` glob is the same program under both. T-613 keeps that reasoning and moves
    # the glob into wave_close_subject_ok so derivation and verification share ONE definition of
    # the format — they must not be able to disagree about what a marker is.
    subj="$(git log -1 --format=%s "$sha" 2>/dev/null)"
    wave_close_subject_ok "$subj" || continue
    # T-613 / F6: a close the operator reverted is not a boundary. Skipping it lands on the
    # PREVIOUS close, which puts the disavowed wave back inside the gate range.
    if rev="$(wave_close_disavowed "$sha")"; then
      echo "gate: skipping wave-close $(git rev-parse --short "$sha") — reverted by $(git rev-parse --short "$rev")." >&2
      echo "        $subj" >&2
      echo "        That wave was disavowed, so its span is re-gated from the close before it." >&2
      continue
    fi
    printf '%s\n' "$sha"; return 0
  done < <(git rev-list --extended-regexp --grep="$WAVE_CLOSE_MARKER_RE" HEAD 2>/dev/null)
  return 1
}

# ── T-613: DOES ANYTHING OTHER THAN THE MARKER AGREE? ────────────────────────────────────────────
#
# THE HONEST STATEMENT FIRST, because the ticket asked for an INDEPENDENT oracle and the truthful
# answer is that a FULLY independent one DOES NOT EXIST in this repository today.
#
# A wave boundary is recorded in exactly ONE place: the subject of the commit left behind when a
# wave closes. Everything else was checked, 2026-08-01, and none of it is anchored to a sha:
#   * `.ai/artifacts/last-verified` is GITIGNORED (.gitignore:55) — one line, no history.
#   * there are no wave tags: `git tag -l` is 100% `T-*` ticket tags, zero wave-shaped refs.
#   * `slice/*` branches are never deleted — 17 of them survive, spanning waves 75 to 78, so a
#     branch ref cannot say which wave is current.
#   * `docs/platform/wave_plan.tsv` names TICKETS, not commits.
# And structurally the two cases are twins: "the previous wave closed HERE" and "the previous wave
# closed at HEAD~1" both look like `landings, boundary, landings` to the graph. So a checker that
# can CONFIRM the boundary from other evidence cannot be written today, and asserting one would be
# this program's signature defect wearing a new hat.
#
# WHAT CAN BE WRITTEN is a set of checks that can REFUSE. Three of them, and exactly what each
# proves — stated together with what it CANNOT prove, because a check that overstates its own reach
# is worse than no check at all: the next reader stops looking. That is not hypothetical. The
# sentence that used to stand here said each check drew on "evidence the commit under test did not
# itself produce", and for check 2 it was FALSE. T-618 exists because it was believed.
#
#   1. MARKER LEDGER — wave_close_is_newest_wave. Evidence: the OTHER 33 markers. A derived
#      boundary must claim a wave number strictly HIGHER than every other marker reachable from
#      HEAD, and NOT MORE THAN ONE higher. Measured 2026-08-01 over all 34: 78 down to 45, strictly
#      decreasing, no repeats, NO GAPS — 33 steps of exactly 1 — so `highest other + 1` is an exact
#      upper bound rather than a guess. This is not independent of the marker FAMILY, but it is
#      independent of the commit being checked: the constraint comes from commits the forger did not
#      write, so a fake cannot self-approve. The lower bound kills the replay/re-close shape ("wave
#      76 CLOSED — reopened and re-closed…"); the upper bound kills the leap-ahead shape ("wave 99
#      CLOSED"), which a bare "strictly higher" test waves straight through.
#
#   2. TICKET LEDGER — wave_close_ledger_says. THIS CHECK WAS THE HOLE T-618 CLOSES, and the hole
#      was in this comment as much as in the code: it claimed independent evidence while reading
#      docs/platform/wave_plan.tsv and .ai/tickets/registry.json OUT OF HEAD'S CHECKOUT — where a
#      marker commit that edits both files in the same commit has already written the answer it is
#      about to be graded on. Wave 78's verifier did exactly that, and the gate returned rc 0 over a
#      range of 1 commit whose honest span was 9.
#
#      WHAT THESE TWO FILES ACTUALLY ARE, measured across the closes of waves 73-78:
#        * registry.json is a RESULT, and `wave --close` IS WHAT WRITES IT. At the PARENT of every
#          real close the wave's own tickets are still `deferred`/`ready` — wave 77's five were all
#          `deferred` one commit before 2b144b5d flipped them to `shipped`. So the registry cannot
#          be read from before the boundary: that would contradict, and hard-refuse, every
#          legitimate wave close in this repository's history.
#        * wave_plan.tsv is a SCHEDULE, and usually predates the close — w77's rows were filed by
#          2a8b41e2, a separate ticket-filing commit. But not always: the real wave-78 close
#          b2afc99a appended its own four w78 rows.
#
#      Hence the split this check now makes. MEMBERSHIP — which tickets wave N contains — is read
#      with `git show` at the boundary's PARENT, a revision the commit under test cannot have
#      written. COMPLETION — are they shipped — is read at the boundary itself, and is used ONLY to
#      CONTRADICT, never as the corroborating leg. Corroboration therefore rests entirely on the leg
#      that is independent; when the boundary filed its own plan rows, that leg is absent and this
#      reports SILENCE, which the caller escalates to demand_base_confirmation.
#
#      WHAT IT STILL CANNOT DO, stated plainly so nobody re-derives it as a surprise: a forger who
#      files the plan rows in a SEPARATE, EARLIER commit and flips the registry in the marker
#      satisfies both legs. Measured in a scratch clone — that shape passes. It is not detectable
#      here, because it is byte-for-byte the shape of a legitimate close (w77 was filed in exactly
#      that order). Nothing but demand_base_confirmation stands behind it, and an operator naming
#      the sha is the honest floor of this whole page.
#
#   3. SLICE SPAN — slice_span_check. Evidence: MERGE PARENTS. Reads no marker at all, which makes
#      it the one genuinely marker-independent check here. Two clauses: a base may not BE a slice
#      merge (a wave base is the previous close, never a landing), and no slice merge inside
#      base..HEAD may fork from before the base (that would mean the range bisects a slice, so the
#      gate reads half of somebody's work). Measured against all 33 waves of real history: 12 slice
#      merges examined, 0 violations — it has never fired on a legitimate base. It catches the
#      narrowing shapes (HEAD~1, mid-wave) on graph structure alone; it cannot catch a base placed
#      AFTER the whole wave, which is why 1 and 2 exist.
#
# WHEN NOTHING CAN SPEAK, THE GATE REFUSES AND ASKS. There is no silent pass left on this path:
# TBD_GATE_BASE_CONFIRM must name the exact sha, so confirming requires reading the sha.

# Tickets the plan assigns to a wave AS OF A REVISION, accepting both label spellings in the file
# (`77` and `w77`).
#
# THE `sub(/^w/,"",w)` BELOW IS NOT STYLE TOLERANCE AND MUST NOT BE "TIDIED UP" NOW THAT T-616 HAS
# NORMALISED THE COLUMN TO BARE INTEGERS. T-616 normalised the WORKING TREE; it cannot normalise
# HISTORY, and this function reads history exclusively — `git show "$1:$PLAN"` at the boundary's
# PARENT. Every revision at or before wave 79's close still spells those rows `w76`…`w79`, because
# that is what was committed. MEASURED 2026-08-01 against wave 79's close 6b2f4750: the parent
# 3c44b6ea holds 5 rows literally beginning `w79`, and stripping the prefix is the only reason
# oracle 2 can still say "corroborated" instead of falling silent. Delete it and every wave close
# in this repository's history becomes unverifiable in one commit.
#
# T-618: takes a rev because the checkout is not evidence. This has exactly one caller — oracle 2 —
# and that caller must not be able to read a plan row the commit it is grading just wrote, so there
# is deliberately NO checkout-reading variant of this function to reach for by mistake.
#
# The two filters are `plan_rows`' filters, repeated rather than reused, because plan_rows reads a
# FILE and this reads a BLOB. They stay BRE `^#` / `^wave[[:space:]]`, which mean the same thing
# under ugrep and GNU grep — see the engine note inside prev_wave_close.
#
# A `$PLAN` that TBD_WAVE_PLAN has pointed outside the repo is not a path `git show` can resolve;
# that yields no rows, which this check reports as silence and the caller escalates. Fail-closed.
wave_plan_tickets_at() {
  git show "$1:$PLAN" 2>/dev/null |
    grep -v '^#' | grep -v '^wave[[:space:]]' | sed '/^\s*$/d' |
    awk -F'\t' -v n="$2" '{ w=$1; sub(/^w/,"",w); if (w==n) print $2 }'
}

# Of these tickets, which does the registry AS OF A REVISION not call shipped (or cancelled)?
# Prints them space-separated; rc 3 if the registry could not be read or parsed at that revision.
#
# One python3 for the whole list rather than is_shipped's one-per-ticket: the blob has to be
# materialised anyway, and a cannot-read must be distinguishable from a clean list here. is_shipped
# answers "not shipped" for a registry it cannot read, which is the right answer for a checkout and
# the wrong one for this caller — it would turn an unreadable blob into a CONTRADICTION and
# hard-refuse the gate over a file it never actually examined.
wave_ledger_unshipped_at() {
  local rev="$1"
  shift
  git show "$rev:$REGISTRY" 2>/dev/null | python3 -c '
import json,sys
try: r=json.load(sys.stdin)
except Exception: sys.exit(3)
by={x.get("id"):x.get("status") for x in r.get("tickets",[])}
print(" ".join(t for t in sys.argv[1:] if by.get(t) not in ("shipped","cancelled")))
' "$@"
}

# ORACLE 1. rc 0 = this marker claims the highest wave number reachable, by exactly one;
# rc 2 = contradicted, from either direction.
wave_close_is_newest_wave() {
  local sha="$1" n other on high=""
  n="$(wave_close_number "$sha")" || return 2
  while read -r other; do
    [ -z "$other" ] && continue
    [ "$other" = "$sha" ] && continue
    on="$(wave_close_number "$other")" || continue
    # Highest number any OTHER marker claims, disavowed ones included. A reverted close still
    # proves its wave number was reached, so it still bounds what the next one may claim; excluding
    # it here would let a fake leap ahead through the very hole the F6 revert fix opened.
    if [ -z "$high" ] || [ "$on" -gt "$high" ]; then high="$on"; fi
    [ "$on" -lt "$n" ] && continue
    # A DISAVOWED close is not part of the ledger, so it cannot outrank anything. Without this,
    # this check and the F6 revert fix fight each other: derivation correctly steps back past a
    # reverted `wave 76 CLOSED` to wave 75's, and then this refuses wave 75 for being older than
    # the very marker that was just thrown away. Only consulted for markers that would actually
    # refuse, so the normal path pays nothing for it.
    wave_close_disavowed "$other" >/dev/null && continue
    echo "gate: the derived wave base is CONTRADICTED by the marker ledger — refusing to run."
    echo "        derived $(git rev-parse --short "$sha") claims wave $n"
    echo "          $(git log -1 --format=%s "$sha")"
    echo "        but $(git rev-parse --short "$other") also reachable from HEAD claims wave $on"
    echo "          $(git log -1 --format=%s "$other")"
    echo "        Wave numbers only ever go up: all 34 markers in history run 78 down to 45,"
    echo "        strictly decreasing, no repeats. A newer marker claiming an equal or older wave"
    echo "        means the newest one is not a wave boundary — it is a commit that looks like one,"
    echo "        and gating from it would put a whole wave outside every change-scoped step."
    echo "        If wave $n really was re-closed, revert the first close (git revert writes the"
    echo "        trailer this script reads) rather than writing a second marker for it."
    return 2
  done < <(git rev-list --extended-regexp --grep="$WAVE_CLOSE_MARKER_RE" HEAD 2>/dev/null)

  # T-618, THE OTHER DIRECTION. "Strictly higher" alone never refuses a number that is higher by a
  # MILE, so `wave 99 CLOSED` outranked all 34 real markers and sailed through. Wave numbers do not
  # merely increase, they increase by ONE: measured 2026-08-01 across every marker reachable from
  # HEAD, 78 down to 45, 33 steps, every one of them exactly 1. So the exact bound is
  # `highest other + 1`, and anything above it is a number no wave has ever reached.
  #
  # Skipped when there is no other marker at all — the first wave ever closed has nothing to be
  # one more than, and inventing a ceiling for it would refuse a legitimate tree.
  if [ -n "$high" ] && [ "$n" -gt $((high + 1)) ]; then
    echo "gate: the derived wave base claims a wave that never opened — refusing to run."
    echo "        derived $(git rev-parse --short "$sha") claims wave $n"
    echo "          $(git log -1 --format=%s "$sha")"
    echo "        but the highest wave any other marker reachable from HEAD claims is $high, so the"
    echo "        next wave to close can only be $((high + 1)). Wave numbers advance by exactly one:"
    echo "        measured over all 34 markers, 78 down to 45, 33 steps of 1, no gaps and no repeats."
    echo "        A marker $((n - high)) waves ahead of the ledger is not a boundary this history"
    echo "        ever reached — and gating from it would put every wave in between outside the"
    echo "        range, unread, while the verdict claimed to describe them."
    return 2
  fi
  return 0
}

# ORACLE 2. rc 0 = ledger corroborates; rc 1 = ledger cannot speak; rc 2 = ledger contradicts.
# Prints its own verdict either way — a check nobody sees the result of is not a check.
#
# T-618. Read the block above for what changed and why. In one line: MEMBERSHIP comes from the
# boundary's PARENT, COMPLETION from the boundary, and only the former can corroborate.
wave_close_ledger_says() {
  local sha="$1" n par tickets known open rc
  n="$(wave_close_number "$sha")" || return 1
  # No parent = no revision before this commit to ask, so there is nothing independent to ask it.
  par="$(git rev-parse --verify --quiet "${sha}^1" 2>/dev/null)" || {
    echo "        ticket ledger: $(git rev-parse --short "$sha") has no parent commit, so there is no"
    echo "                       revision preceding it to read $PLAN from — cannot corroborate."
    return 1
  }
  tickets="$(wave_plan_tickets_at "$par" "$n" | tr '\n' ' ')"
  tickets="${tickets%"${tickets##*[! ]}"}"
  known=0
  [ -n "$tickets" ] && known="$(printf '%s\n' $tickets | wc -l)"

  if [ "$known" -eq 0 ]; then
    # THE T-618 CASE, and it deserves its own message rather than a generic silence: the plan has
    # rows for wave $n at the boundary but NOT at its parent, which means this very commit filed
    # them. That is self-corroboration, and it is what the forged wave-78 marker did.
    if [ -n "$(wave_plan_tickets_at "$sha" "$n")" ]; then
      echo "        ticket ledger: $(git rev-parse --short "$sha") ADDED wave $n's own rows to $PLAN"
      echo "                       in the same commit that claims wave $n CLOSED. A commit cannot"
      echo "                       corroborate itself, so this is silence, not agreement — the rows"
      echo "                       are not there at its parent $(git rev-parse --short "$par")."
      return 1
    fi
    echo "        ticket ledger: $PLAN has NO rows for wave $n at $(git rev-parse --short "$par") —"
    echo "                       it cannot corroborate this boundary. (The plan is only maintained"
    echo "                       for some waves; this is silence, not agreement.)"
    return 1
  fi

  # COMPLETION, read at the boundary, because that is the only place it is ever true: `wave --close`
  # is what flips these tickets to shipped. Used to CONTRADICT only — see the block above.
  # shellcheck disable=SC2086
  open="$(wave_ledger_unshipped_at "$sha" $tickets)"
  rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "        ticket ledger: $REGISTRY could not be read at $(git rev-parse --short "$sha")"
    echo "                       — cannot corroborate. (Cannot-read is cannot-speak: reporting a"
    echo "                       contradiction over a file nobody parsed is the defect this whole"
    echo "                       page exists to stop.)"
    return 1
  fi
  if [ -n "$open" ]; then
    echo "gate: the derived wave base is CONTRADICTED by the ticket ledger — refusing to run."
    echo "        $(git rev-parse --short "$sha") says wave $n CLOSED, and $PLAN at its parent"
    echo "        $(git rev-parse --short "$par") assigns wave $n ticket(s) that $REGISTRY does not"
    echo "        call shipped at that same commit: $open"
    echo "        A wave with open tickets did not close, so this commit is not a wave boundary."
    return 2
  fi
  echo "        ticket ledger: wave $n has $known ticket(s) in $PLAN at $(git rev-parse --short "$par")"
  echo "                       (the boundary's parent, which it cannot have written), all shipped"
  echo "                       at $(git rev-parse --short "$sha") — corroborated."
  return 0
}

# ORACLE 3. Marker-free. rc 0 = no objection; rc 2 = the base bisects the wave's landings.
slice_span_check() {
  local base="$1" m f
  case "$(git log -1 --format=%s "$base" 2>/dev/null)" in
    "Merge branch 'slice/"*)
      echo "gate: base $(git rev-parse --short "$base") IS a slice merge — refusing to run."
      echo "          $(git log -1 --format=%s "$base")"
      echo "        A wave base is the commit the wave OPENED at, which is the previous wave's"
      echo "        close — never a landing. Starting here excludes every slice that merged"
      echo "        before it, and each of those is work this gate would report PASS over"
      echo "        without reading. (Checked from merge structure alone; no marker consulted.)"
      return 2 ;;
  esac
  while read -r m; do
    [ -z "$m" ] && continue
    case "$(git log -1 --format=%s "$m" 2>/dev/null)" in "Merge branch 'slice/"*) ;; *) continue ;; esac
    f="$(git merge-base "$m^1" "$m^2" 2>/dev/null)" || continue
    [ "$f" = "$(git rev-parse "$base")" ] && continue
    git merge-base --is-ancestor "$f" "$base" 2>/dev/null || continue
    echo "gate: base $(git rev-parse --short "$base") cuts through a slice — refusing to run."
    echo "        $(git log -1 --format=%s "$m")   ($(git rev-parse --short "$m"))"
    echo "        merged INSIDE the range but was branched at $(git rev-parse --short "$f"),"
    echo "        which is BEFORE the base. So that slice's own commits are outside $base..HEAD"
    echo "        while its merge is inside: the gate would examine part of one slice's work and"
    echo "        report on all of it. Pass a base at or before $(git rev-parse --short "$f")."
    echo "        (Checked from merge parents alone; no marker consulted.)"
    return 2
  done < <(git rev-list --merges "$base..HEAD" 2>/dev/null)
  return 0
}

# Print the derived base loudly, then demand the operator name it. Used when NOTHING could
# corroborate. Loud-and-blocked, not quiet-and-passed: the whole point of T-613.
demand_base_confirmation() {
  local bsha="$1" why="$2"
  [ "${TBD_GATE_BASE_CONFIRM:-}" = "$bsha" ] && {
    echo "        base confirmed by TBD_GATE_BASE_CONFIRM."
    return 0; }
  [ "${TBD_GATE_BASE_CONFIRM:-}" = "$(git rev-parse --short "$bsha")" ] && {
    echo "        base confirmed by TBD_GATE_BASE_CONFIRM."
    return 0; }
  echo "gate: nothing could corroborate this wave base — refusing to run unconfirmed."
  echo "        base   $bsha"
  echo "               $(git log -1 --format='%s' "$bsha")"
  echo "               $(git log -1 --format='%an, %ad' --date=short "$bsha")"
  echo "        range  $(git rev-list --count "$bsha..HEAD" 2>/dev/null) commit(s) to HEAD $(git rev-parse --short HEAD)"
  echo "        why    $why"
  echo
  echo "        This is NOT a claim that the base is wrong. It is a refusal to claim it is right."
  echo "        Read the subject above. If that is genuinely where this wave opened, re-run with:"
  echo "            TBD_GATE_BASE_CONFIRM=$bsha bash scripts/platform/wave.sh gate ..."
  echo "        The better fix is to give the ledger something to say: add this wave's rows to"
  echo "        $PLAN BEFORE the wave closes — in the commit that files its tickets, the way"
  echo "        2a8b41e2 filed wave 77's. Rows appended by the closing commit itself corroborate"
  echo "        nothing (T-618): oracle 2 reads the plan at the boundary's PARENT precisely so a"
  echo "        commit cannot vouch for itself, so rows that arrive with the marker are not there."
  return 2
}

# Refuse a base that does not cover the whole wave.
#
# One rule: THE BASE MUST BE AT OR BEFORE THE COMMIT THIS WAVE OPENED AT — i.e. it is an
# ancestor-or-equal of the previous wave's close. A base OLDER than that passes on purpose:
# over-broad gates more than it must, and over-broad has never been the failure mode here. Narrow
# is, every time.
#
# NOT "every slice MERGE is inside base..HEAD", which is how the ticket phrased it. Measured: wave
# 76 landed T-608 as a plain commit with no merge at all, and wave 74 landed three that way
# (`c7a3ff78`, `bed4f269`, `0a1a53ac`). Enumerating merges would have called such a wave covered
# while its non-merge landings sat outside the range — the same lie in a new place. The ancestor
# test is landing-shape-independent. (slice_span_check above enumerates merges for a DIFFERENT
# question — whether the range bisects one — where the shape is exactly what is being asked about.)
#
# T-613 — THE ANCESTOR TEST BELOW IS STILL ASKED OF prev_wave_close, THE FUNCTION THAT PRODUCED THE
# ANSWER, and that cannot be fixed by moving the call: there is no second record of the boundary to
# ask instead (see the block above). What changed is that the derived boundary must now survive
# three cross-checks that do NOT come from it, and that a boundary nothing can corroborate is
# refused rather than trusted.
gate_base_covers_wave() {
  local base="$1" bsha psha missed lrc
  bsha="$(git rev-parse --verify --quiet "${base}^{commit}" 2>/dev/null)" || return 2
  # A base off HEAD's history makes `base..HEAD` an unrelated set, not "this wave".
  if ! git merge-base --is-ancestor "$bsha" HEAD 2>/dev/null; then
    echo "gate: base '$base' is not an ancestor of HEAD — refusing to run."
    echo "        '$base..HEAD' would describe a set of commits nobody asked about."
    return 2
  fi
  # No marker to compare against. This used to `return 0` — an explicit base plus a silent pass,
  # which is the shape this file exists to hunt. ORACLE 3 needs no marker, so it still speaks here;
  # after that, say what could not be checked and make the operator name the sha.
  if ! psha="$(prev_wave_close)"; then
    slice_span_check "$bsha" || return 2
    demand_base_confirmation "$bsha" "no 'wave N CLOSED' commit is reachable from HEAD, so the previous wave's close is unknown" || return 2
    return 0
  fi
  # ORACLES 1 and 2, against the DERIVED boundary, before it is allowed to judge anything.
  wave_close_is_newest_wave "$psha" || return 2
  echo "gate: cross-checking derived wave base $(git rev-parse --short "$psha")"
  wave_close_ledger_says "$psha"; lrc=$?
  [ "$lrc" -eq 2 ] && return 2
  if [ "$lrc" -eq 1 ]; then
    demand_base_confirmation "$psha" \
      "the marker ledger accepts it (wave $(wave_close_number "$psha") is the newest closed wave) but the ticket ledger has no rows for that wave that the boundary did not write itself, so only one family of evidence agrees" || return 2
  fi
  # The primary rule, with the message that names the exact cost. ORACLE 3 runs after it, not
  # before, so a narrowing base is diagnosed by the check that can say how much it narrows by.
  if git merge-base --is-ancestor "$bsha" "$psha" 2>/dev/null; then
    slice_span_check "$bsha" || return 2
    return 0
  fi
  # psha..bsha, NOT bsha..psha. bsha is the NEWER of the two here (that is what makes it wrong), so
  # this counts the wave's commits that the base skips past — the ones every change-scoped step
  # would never see. Reversed, it is always 0, which is exactly the reassuring lie to avoid here.
  missed="$(git rev-list --count "$psha..$bsha" 2>/dev/null || echo '?')"
  echo "gate: base $(git rev-parse --short "$bsha") starts AFTER this wave opened — refusing to run."
  echo "        this wave opened at $(git rev-parse --short "$psha")"
  echo "          $(git log -1 --format=%s "$psha")"
  echo "        $missed commit(s) of this wave sit OUTSIDE $base..HEAD. touch_changed, wasm32,"
  echo "        fmt and the trunk build would each report PASS/SKIP without reading one of them,"
  echo "        and the verdict would describe a fraction of the wave. That is T-602 verbatim."
  echo "        Fix: run 'wave.sh gate' with NO base (it derives $(git rev-parse --short "$psha")),"
  echo "             or pass a base at or before $(git rev-parse --short "$psha")."
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
#   SHARED GATE DATABASE. Pre-T-411, ensure_gate_db handed every slice the same tbd_gate_it; T-411
#   narrowed that to per-wave tbd_gate_w<N> (last two kept). Concurrent writers inside one wave remain:
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
    # nobody reading a log can mistake the result for a clean pass. GATE_UNSERIALISED=1 is what
    # lets ensure_gate_db still reset tbd_gate_migrate under this hatch (T-409); GATE_LOCK_HELD
    # stays 0 — we do not pretend the flock is held.
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
  # T-583/T-594. The other half of the T-244 lesson above, and the half `schema` cannot reach.
  #
  # `gate_schema` validates the catalogue AS COMMITTED. It cannot tell you the committed catalogue
  # disagrees with `packages/tbd-schema/rules/prefab-classify.json`, because a rule edit changes
  # NOTHING until the catalogue is rebuilt — and until T-278 the only rebuild path needed a
  # Workbench export that is gitignored and absent from every clone. So T-244's `vehicle` rules
  # went in, every gate stayed green, and the shipped artifact was stale for four weeks. This step
  # re-derives the classification lane from committed artifacts alone and exits 1 on disagreement;
  # run on the day T-244 landed it would have gone RED immediately. ~12 s.
  #
  # `checkrun`, NOT `hostrun`: `hostrun` bakes in the SHARED CARGO_TARGET_DIR, and
  # `tools/tbd-tools/src/serve.rs` `repo_root()` is `env!("CARGO_MANIFEST_DIR")` — a COMPILE-TIME
  # constant. A shared dir can therefore hand this step a `world` binary that reads a DIFFERENT
  # WORKTREE'S rules and catalogue while reporting on yours: the signature defect, with the two
  # inputs the verdict is entirely about. `checkrun` pins $GATE_CHECK_TARGET, whose writers are
  # bounded to gates serialised by the gate lock.
  #
  # And NOT folded into `make schema-validate`: gate_schema's drift tripwire parses that recipe for
  # `xtask schema <name>` lines, and this is a `tbd-tools --bin world` call — it would either trip
  # the tripwire or be silently skipped by it.
  run "T-278 catalogue drift" checkrun make map-reclassify TERRAIN=everon
  # T-515. Class-R on 0016 claim UPDATE body — db_migrate.rs is schema-count-only;
  # a hollow claim migration stays green. Unconditional (wave.sh-only slices must hit it).
  run "db_migrate claim body" gate_db_migrate_claim_body
  # T-555. The populated-database step, in AUDIT mode: checksum-audits every already-applied
  # migration and dry-runs the pending ones against real rows, without advancing the shared DB.
  # It belongs in the CHEAP gate specifically because a843905f — the edit to an already-applied
  # migration that killed every existing database — landed through a slice gate. Unconditional and
  # not change-scoped: a slice that touches no migration can still be the one that has to notice a
  # sibling's drift, and this step is psql-only (~1 s), not a cargo step.
  run "db_migrate persist" gate_db_migrate_persist audit
  # T-462. Shell Class-R near schema: verify scripts that exist but were never
  # invoked by the cold gate (wave 24 adversarial — T-439 unwired; T-444 pin absent).
  # T-463. Same pattern for T-438 deploy-staging compose path + T-456 REST size gate
  # (wave 25 — scripts existed, cold gate never executed them).
  # T-468. Tripwire: ci.yml schema job must stay on `make ci-local-schema`.
  # T-478. verify-t440 pins both this gate_slice run and cmd_gate (comment-strip +
  # redirect recipe + dual-path); deleting either run must FAIL the verify script.
  run "T-439 objects aliases" checkrun cargo run -q -p xtask -- verify t439
  run "T-444 wiki seed"       checkrun cargo run -q -p xtask -- verify t444
  run "T-440 faction library seed" checkrun cargo run -q -p xtask -- verify t440
  run "T-438 deploy-staging"  checkrun cargo run -q -p xtask -- verify t438
  run "T-456 REST size gate"  checkrun cargo run -q -p xtask -- verify t456
  run "T-468 CI schema parity" checkrun cargo run -q -p xtask -- verify t468
  run "T-437 destroy inert"   checkrun cargo run -q -p xtask -- verify t437
  run "T-586 route tags"      checkrun cargo run -q -p xtask -- verify route-tags
  # T-556. The T-462/T-463 pattern once more, and the worst instance of it: these two
  # existed, carried the fail-open `if rg …; then fail; fi` shape, AND were invoked by
  # nothing — not this gate, not ci.yml, not the Makefile. So a reader who found them
  # would have trusted a pair of bans that had never compared anything. Wired into both
  # halves (here and cmd_gate) so neither path can drift green on its own.
  run "T-296 reporter identity" checkrun cargo run -q -p xtask -- verify t296
  run "T-452 player identity" checkrun cargo run -q -p xtask -- verify t452
  # T-620. Hot-path twin of the cmd_gate run — see the long note there for why this gate spent four
  # waves invoked by nothing. Pure bash + git ls-files, no cargo, ~0.2 s measured, so it fits the
  # slice gate's ~10 s budget. Catching a stray .py or a new python3 call at SLICE time is the
  # cheapest place to catch it; the no-node/no-shell twins stay wave-level because they need a
  # built xtask and this gate deliberately does not build one.
  run "no-python (T-620)" checkrun cargo run -q -p xtask -- verify no-python
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
# T-602: with no base argument it is DERIVED from the last wave-close commit and then VERIFIED to
# cover the whole wave; an explicit base is verified the same way. It no longer falls back to
# HEAD~1 — see the WAVE GATE BASE block above for the wave-75 incident that default caused, the
# wave-76 reproduction, and why derive-and-verify rather than a mandatory argument.
cmd_gate() {
  local base="${1:-}"
  if [ -z "$base" ]; then
    base="$(prev_wave_close)" || {
      echo "gate: no base given, and no 'wave N CLOSED' commit is reachable from HEAD."
      echo "        There is nothing to derive the wave's base from, and HEAD~1 is not a safe"
      echo "        guess — it is the exact default that reported PASS 26/26 over four unexamined"
      echo "        frontend slices in wave 75. Pass the base explicitly:"
      echo "        bash scripts/platform/wave.sh gate <sha main was at before this wave>"
      return 2
    }
    echo "gate: no base given — derived $(git rev-parse --short "$base") from the last wave-close commit"
    echo "        $(git log -1 --format=%s "$base")"
  fi
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
      echo "        wave gate:  bash scripts/platform/wave.sh gate [<base>]   (derived when omitted)"
    else
      echo "gate: base '$base' is not a resolvable commit — refusing to run."
      echo "        Every change-scoped step would diff against nothing and PASS without looking."
    fi
    return 2
  fi
  # T-602. Resolvable is not the same as CORRECT. `gate HEAD~1` resolves, is an ancestor of HEAD,
  # and contains changed files — it clears both the check above and refuse_empty_range below, and
  # it is exactly what shrank wave 75's gate to one merge. This is the check that catches it, and
  # it runs for a derived base and an explicit one alike. See gate_base_covers_wave.
  gate_base_covers_wave "$base" || return 2
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
  # Clippy is scoped per-crate, NOT --workspace.
  #
  # `cargo clippy --workspace --all-targets -- -D warnings` is still red on clean main, so a
  # workspace-wide gate would be red before a single slice merged and nothing could ever land.
  #
  # T-603 CORRECTION — THE REASON MOVED, AND THE NOTE HAD NOT. This used to read "~45 errors, almost
  # all in tools/tbd-tools and xtask, which have never been clippy-gated". MEASURED 2026-07-31, that
  # attribution is now exactly backwards: 60 errors in the bin target (61 with --all-targets), ALL
  # SIXTY in `website-frontend` linted natively, and ZERO in tools/tbd-tools or xtask — those two
  # are clean and are gated by the `clippy xtask+tbd-tools` step added below. The frontend residue
  # is the same advisory-warnings case the `clippy frontend` note names, seen through a native
  # target instead of wasm32; it is not this run's to fix.
  #
  # ci.yml gates per-crate (:59 website-api, :91 map-engine, :112 website-frontend on wasm32) and
  # the three steps here mirror it; the fourth (below) covers what ci.yml has no job for at all.
  run "clippy api"       checkrun cargo clippy -p website-api --all-targets --quiet -- -D warnings
  # --features doc,mission,world (same floor as --all-features for this crate): without them
  # clippy compiles none of those modules and passes on code it never read. Measured blind on
  # flatten.rs. Gate test step uses --all-features (T-747 / wave139 F2).
  run "clippy map-engine" checkrun cargo clippy -p map-engine-core --features doc,mission,world -p map-engine-render --all-targets --quiet -- -D warnings
  # NOTE: no `-D warnings` here, deliberately — ci.yml website-frontend clippy runs WITHOUT it, so
  # warnings are advisory upstream. Adding -D here would make the gate stricter than CI and red on
  # arrival. T-742 adds --all-targets (load-bearing for #[cfg(test)] / benches) so this step and
  # clippy_changed stay aligned with T-752's Makefile/ci-local-leptos fix; -D stays off to match CI.
  run "clippy frontend"  checkrun cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets --quiet
  # Scoped per-crate rather than `--workspace`, mirroring clippy above. ci.yml tests website-api
  # and website-frontend; map-engine is covered by its own job; xtask + tools/tbd-tools are covered
  # by the `test xtask+tbd-tools` step added below.
  #
  # T-597 CORRECTION — this note used to end: "`cargo test --workspace` pulls in tools/tbd-tools,
  # which CI never tests and which has a FAILING test on clean main
  # (density::tests::corner_partition_identity — pre-existing, filed as its own ticket). A gate that
  # is red before any slice merges is a gate nothing can ever pass."
  #
  # Every clause of that was true. The CONCLUSION was the defect: it took a red test as a fixed
  # property of the tree and scoped the gate around it, which is how a red test becomes permanent.
  # That assertion had been red since T-176 (`a5940fad9`) moved the canopy grid from 32 m to 8 m and
  # left the literal at 401; the test itself was correct and the number was four weeks stale. It is
  # fixed at tools/tbd-tools/src/density.rs, and the two crates are gated below rather than routed
  # around. Left in place rather than deleted because "scope the gate away from the red" is a
  # reasonable-sounding move that someone will otherwise make again.
  # ensure_gate_db + the skip count check are what stop this step passing vacuously. A suite that
  # reports "ok" while every DB test printed `skip:` is worse than a red one: it is a green one.
  # rc honoured: ensure_gate_db now refuses to force-drop tbd_gate_migrate without the gate lock,
  # and a gate that could not prepare its database must not go on to interpret the result.
  ensure_gate_db || fail=1
  # T-515. Adjacent to migrate DB prep: Class-R pins 0016 claim UPDATE body on disk.
  # schema-count db_migrate.rs cannot see a hollow claim (REFRESH kept, UPDATE dropped).
  run "db_migrate claim body" gate_db_migrate_claim_body
  # T-555. ADVANCE mode — the wave gate is the only caller allowed to move the persist DB forward,
  # because only merged main is history that will not be abandoned. Deliberately placed AFTER
  # ensure_gate_db (which owns the throwaway forward-from-empty DB) and BEFORE `test api`: this is
  # the step that answers "will the databases that already exist survive this wave", and the
  # forward-from-empty run that db_migrate.rs performs cannot answer it. Under the gate lock, which
  # gate_db_migrate_persist asserts for itself before writing.
  run "db_migrate persist" gate_db_migrate_persist advance
  run "test api"         gate_test_api
  # --all-features is REQUIRED (T-747 / wave139 F2). Bare `cargo test -p map-engine-core` is a
  # vacuous pass (~140 tests; tripwire REDs). `--features doc,mission` still skips the world/dem
  # suite (~133 tests). Makefile `ci-local` and this gate must match: `--all-features` (doc +
  # mission + world). Measured 2026-08-08: bare 140, doc,mission 502, --all-features 635.
  # Private target dir for the same reason as `test api` and `test frontend`: this step RUNS test
  # binaries, and a shared dir lets another worktree's build be the one that runs.
  run "test map-engine"  hostrun env "CARGO_TARGET_DIR=$MAIN_ROOT/target-gate-mapengine" "CARGO_INCREMENTAL=0" \
                                 cargo test -p map-engine-core --all-features -p map-engine-render --quiet
  # Frontend tests get a PRIVATE target dir. Two agents (T-193, T-195) independently proved that
  # with the shared CARGO_TARGET_DIR, `cargo test -p website-frontend` runs a stale
  # website_frontend-<hash> test binary built from ANOTHER worktree: T-193 saw 113 passing from a
  # binary lacking its new tests; T-195 hit it twice and had to use a private dir to get true
  # numbers. Same package name + version across worktrees = same artifact hash = clobbering.
  # A silent PASS on code that was never compiled makes every other check advisory, so this one
  # step is worth the extra disk. Builds only this crate's tree, not the 609-crate workspace.
  run "test frontend"    hostrun env "CARGO_TARGET_DIR=$MAIN_ROOT/target-gate-frontend" \
                                  cargo test -p website-frontend --quiet
  # T-597 — THE STRUCTURAL GAP. `xtask` and `tools/tbd-tools` were tested by NOTHING.
  #
  # The gate ran `test api`, `test map-engine`, `test frontend` and stopped. The assumption written
  # above the `test api` step was that ci.yml covered the rest of the workspace, so a per-crate gate
  # lost nothing. MEASURED 2026-07-31, and it does not: ci.yml's `test` step is a bare `cargo test`
  # under the website-api job, whose `defaults.run.working-directory` is `apps/website/api`. Cargo
  # with no `-p` selects the package in the CWD — `cargo pkgid` there returns `website-api@0.1.0`,
  # and `cargo test --no-run` from that directory builds no tbd-tools or xtask test target at all.
  # So both are workspace members that no gate and no workflow has ever run.
  #
  # What that cost: density::tests::corner_partition_identity sat red from T-176 (`a5940fad9`) to
  # T-597 — four weeks — and the only machine that noticed was a `cargo test --workspace` nobody
  # runs. The gate did not miss it; the gate was never pointed at it. Two crates that hold the
  # ticket CLI, the schema gates, the map-asset pipeline and the world exporter is not a rounding
  # error in coverage.
  #
  # PRIVATE TARGET DIR, same reason as `test api` and `test frontend` and not negotiable: this step
  # BUILDS AND RUNS test binaries, and on the shared dir the binary that runs can be one another
  # worktree built (same package + version = same artifact hash = clobbering). See the long headers
  # on gate_test_api and on GATE SERIALISATION for the three independent measurements of that.
  #
  # Cost: 84 tests, ~3.1 s of actual running (51 xtask + 33 tbd-tools; measured 2026-07-31 — T-597
  # wrote 81 and T-361's three serve tests landed in the same wave). The build is the whole of the
  # rest of the wall clock and it is incremental after the first wave. Cheap enough that scoping it
  # by change would buy nothing and would reintroduce exactly the "diffed against nothing, printed
  # PASS" shape this file exists to prevent — so it is unconditional, like `schema`.
  run "test xtask+tbd-tools" hostrun env "CARGO_TARGET_DIR=$MAIN_ROOT/target-gate-tools" "CARGO_INCREMENTAL=0" \
                                  cargo test -p xtask -p tbd-tools --quiet
  # T-603 — THE OTHER HALF OF T-597's GAP. T-597 established that nothing ran `cargo test` on these
  # two crates and added the step above. Nothing LINTED them either, and that half stayed open: the
  # three clippy steps above name website-api, map-engine and website-frontend, and ci.yml's own
  # clippy runs under `working-directory: apps/website/api`, so xtask and tools/tbd-tools were
  # outside every lint the program has.
  #
  # What that cost, measured 2026-07-31: 14 errors on clean main under `-D warnings` — 10 in
  # tools/tbd-tools (4 collapsible_if, 3 unnecessary_sort_by, collapsible_str_replace,
  # no_effect_replace, manual_pattern_char_comparison) and 4 in xtask (nonminimal_bool, 2 len_zero,
  # doc_lazy_continuation). All mechanical, none behavioural, and all of them older than the ticket
  # that found them. They are fixed in the same commit that adds this step, because a gate step
  # that is red the moment it lands teaches the next agent that gate failures are noise — the same
  # reasoning the `clippy frontend` note above gives for not adding `-D warnings` there.
  #
  # NOTE ON THAT 14. The ticket that filed this reported NINE errors, "all in tools/tbd-tools/src/enf".
  # That was a truncated reading, not a wrong one: `cargo clippy` aborts the remaining compilation
  # units once one fails, so the tbd-tools LIB failure hid the `enf` BIN target and every xtask
  # target behind it. The count only settles by re-running after each fix. Recorded because this
  # file's subject is tools that report on inputs they did not fully examine, and the tool that
  # measured this step's own workload was one of them.
  #
  # --all-targets, matching `clippy api` and `clippy map-engine`: without it the #[cfg(test)] code
  # is not linted, and 2 of the 14 (both len_zero) were in test modules. -D warnings, same as those
  # two steps — this is not the frontend's advisory case, there is no upstream CI job to stay in
  # step with, so the gate sets the standard rather than mirroring one.
  #
  # `checkrun`, not `hostrun`: this is a check-class step and carries the T-421 shared-target-dir
  # exposure verbatim, so it belongs on GATE_CHECK_TARGET with the other analysis steps rather than
  # on the shared dir. The test step above needs its own private dir instead because it RUNS
  # binaries; this one only reads.
  run "clippy xtask+tbd-tools" checkrun cargo clippy -p xtask -p tbd-tools --all-targets --quiet -- -D warnings
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
  # T-583/T-594 — cold-path twin of the gate_slice step. Per T-556, a step wired into only one
  # half drifts green: the half without it keeps printing PASS over the thing the other half is
  # there to catch. Same `checkrun` (compile-time `repo_root()` — see the long note in gate_slice),
  # same placement next to `schema`, deliberately NOT inside `make schema-validate`.
  run "T-278 catalogue drift" checkrun make map-reclassify TERRAIN=everon
  run "ticket registry"  checkrun cargo run -q -p xtask -- ticket check
  # T-462. Same shell Class-R as gate_slice — fail-fast actionable scripts next to
  # schema/ticket so a deleted wiki seed line or Objects guid mismatch cannot stay cargo-green.
  # T-463. Same pattern for T-438 deploy-staging compose path + T-456 REST size gate.
  # T-468. Tripwire: ci.yml schema job must stay on `make ci-local-schema`.
  # T-478. Cold-path twin of the gate_slice T-440 run — verify-t440 requires both.
  run "T-439 objects aliases" checkrun cargo run -q -p xtask -- verify t439
  run "T-444 wiki seed"       checkrun cargo run -q -p xtask -- verify t444
  run "T-440 faction library seed" checkrun cargo run -q -p xtask -- verify t440
  run "T-438 deploy-staging"  checkrun cargo run -q -p xtask -- verify t438
  run "T-456 REST size gate"  checkrun cargo run -q -p xtask -- verify t456
  run "T-468 CI schema parity" checkrun cargo run -q -p xtask -- verify t468
  run "T-437 destroy inert"   checkrun cargo run -q -p xtask -- verify t437
  run "T-586 route tags"      checkrun cargo run -q -p xtask -- verify route-tags
  # T-556. Cold-path twin of the gate_slice runs above — the two scripts that were dead
  # AND fail-open. Both halves, for the T-478 reason: one path alone can drift green.
  run "T-296 reporter identity" checkrun cargo run -q -p xtask -- verify t296
  run "T-452 player identity" checkrun cargo run -q -p xtask -- verify t452
  # T-620/T-621 — THE LANGUAGE GATES, AND WHY THEY ARE HERE RATHER THAN ONLY IN ci.yml.
  #
  # `verify-no-python` existed since T-162 and was wired into one Makefile target and `make
  # ci-local` — which this file's own header explains is deliberately NOT used by the gate. It was
  # therefore in NO path that runs: not ci.yml (measured, zero hits), not this gate. Meanwhile it
  # was RED, on scripts/{platform,mod}/slice-collisions.py, from the day the factory opened. Four
  # waves of "GATE PASS 28/28" were printed over a hard gate that was failing the whole time and
  # that nothing invoked. That is the exact shape T-556 and T-478 keep finding, at gate scope.
  #
  # `verify-no-python` is bash + git and costs nothing. The other two are xtask, and xtask is
  # already built by `test xtask+tbd-tools` above, so `cargo run -q` is a no-op relink here.
  run "no-python (T-620)" checkrun cargo run -q -p xtask -- verify no-python
  run "no-node (T-165.10)" hostrun cargo run -q -p xtask -- verify no-node
  run "no-shell (T-621)"  hostrun cargo run -q -p xtask -- verify no-shell
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
  #
  # T-602 — THE SAME BUG LIVED HERE, LATENT. This used to pass
  # `HEAD~${WAVE_GATE_DEPTH:-40}`, falling back to the root commit when HEAD had fewer than 40
  # ancestors. A COUNT is not a wave boundary: any wave longer than 40 commits silently gated only
  # its last 40 and every change-scoped step went narrow exactly as wave 75's did. Wave 75 was 10
  # commits and wave 76 was 7, so it never bit — the whole defect was one long wave away, and the
  # `WAVE_GATE_DEPTH` override made it one environment variable away. Both are gone: cmd_gate now
  # derives the base from the wave-close marker itself, which is the boundary rather than a guess
  # at where it might be, and REFUSES a base that starts after the wave opened. Passing no argument
  # is now the correct call, not the dangerous one.
  echo "gating wave $w against its own base (derived — not HEAD, which makes fmt/wasm/trunk vacuous)"
  cmd_gate || { echo "REFUSED: wave gate is red on main"; return 1; }
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
#
# T-426: gate-private dirs (target-gate-*, dist-gate-*) live at MAIN_ROOT, not /var/tmp — ~15 GB
# class, expensive to rebuild, warm is valuable (T-421 measured cold 23.4 s vs warm 9.3 s slice gate).
# Default reclaim does NOT touch them; opt in with --gate-dirs. Optional --gate-dirs-older-than-days N
# only removes gate dirs whose directory mtime is older than N days (age-based sweep without nuking
# a cache that was used today).
#
# T-589: PER-SLICE private dirs (target-<SLICE>, target-<SLICE>-api) ALSO live at MAIN_ROOT, and
# until now nothing reaped them at all. See the block inside for why they are swept BY DEFAULT
# while T-426's gate set stayed opt-in — the two look alike and are opposites.
cmd_reclaim() {
  local live="" w t freed=0 sz gate_dirs=0 gate_min_age_days=0 a slice_dirs=1
  while [ $# -gt 0 ]; do
    case "$1" in
      --gate-dirs) gate_dirs=1 ;;
      --gate-dirs-older-than-days)
        gate_dirs=1
        gate_min_age_days="${2:-0}"
        shift ;;
      --no-slice-dirs) slice_dirs=0 ;;
      *)
        echo "reclaim: refusing unknown argument '$1' (expected --gate-dirs, --gate-dirs-older-than-days N and/or --no-slice-dirs)" >&2
        return 2 ;;
    esac
    shift
  done
  # THE SPARED SET IS LOAD-BEARING, SO ITS ABSENCE MUST BE DISTINGUISHABLE FROM ITS EMPTINESS.
  # `for w in $(git worktree list | ...)` cannot tell "no other worktrees" (legitimately empty)
  # from "git did not answer" (unknown) — both leave $live empty, and the second one turns every
  # live slice's dir into an apparent orphan. For /var/tmp that has always been the standing risk;
  # for the MAIN_ROOT sweep below it would delete a running agent's build cache, so capture the
  # exit status and let that sweep refuse rather than guess.
  local wt_list live_ok=1
  wt_list="$(git worktree list 2>/dev/null)" || live_ok=0
  [ -z "$wt_list" ] && live_ok=0
  for w in $(printf '%s\n' "$wt_list" | tail -n +2 | awk '{print $1}'); do
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

  # T-589 — PER-SLICE PRIVATE TARGET DIRS AT MAIN_ROOT. Swept by DEFAULT. Here is why, since the
  # sibling set two blocks down is deliberately opt-in and the two are easy to confuse.
  #
  # T-426 made target-gate-* opt-in for one reason: it is a WARM SHARED cache. Every future slice
  # gate hits target-gate-api (24 GB today), and T-421 measured cold 23.4 s vs warm 9.3 s — so
  # deleting it bills work that has not happened yet, to everyone, invisibly. That argument does
  # not survive translation to a target-<SLICE> dir, which is the opposite on every axis: exactly
  # one slice ever hits it, that slice is gone, and nothing will ever hit it again. Its entire
  # remaining function is to occupy disk. `target-T-454` did that for weeks at 2.7 GB while this
  # very command printed "reclaimed 0 MB" standing next to it (measured 2026-07-31), and the volume
  # runs at 87%. Opt-in housekeeping that nobody opts into is not housekeeping. `--no-slice-dirs`
  # turns it off for the one operator who wants a look before a sweep.
  #
  # The leak is also SELF-INFLICTED and structural, which is what makes "just tell agents to clean
  # up" insufficient: PLATFORM_FACTORY's Known traps and the brief template now INSTRUCT every
  # slice agent to build its own runnable binary into target-<slice>-api (T-581/T-582 were served
  # each other's binaries out of the shared target/). Agents are told to delete it and mostly do —
  # T-585 reclaimed 8.0 GB itself — but "mostly" is the wrong verb for a slice that gets parked,
  # rate-limited or killed mid-run, and those are the ones that leave a dir behind.
  #
  # SELECTION IS POSITIVE IDENTIFICATION, NOT A BLOCKLIST. A dir is removed only when its own name
  # says which slice owns it: `target-<TICKET>` with the ticket FIRST, optional suffix after. A
  # blocklist here fails open — the one unlisted name is the one that gets deleted — and this
  # function's blast radius is `rm -rf` on a directory. Measured at MAIN_ROOT today, three dirs
  # that a looser rule would have eaten:
  #     target/                  67 GB  the shared CARGO_TARGET_DIR for every worktree
  #     target-dev-api          3.6 GB  the operator's live `make api` cache — no ticket in the name
  #     target-gate-schema-T422 1.7 GB  a GATE dir that CONTAINS a ticket id
  # The last one is why the ticket must be the first component after `target-`: anchoring there
  # means no target-gate-* name can be read as a slice dir even if the explicit exclusion below
  # were deleted. A name the pattern cannot parse (target-ci, target-T-068.13-api) is SPARED, not
  # guessed at — and printed with its size, because a silent skip is the same defect as the
  # "0 MB" report that produced this ticket, just wearing a quieter hat.
  if [ "$slice_dirs" -eq 0 ]; then
    echo "slice dirs at $MAIN_ROOT: not swept (--no-slice-dirs)"
  elif [ "$live_ok" -ne 1 ]; then
    echo "slice dirs at $MAIN_ROOT: REFUSED — 'git worktree list' did not answer, so the spared set"
    echo "  is unknown and every dir here would look like an orphan. Nothing swept. Run from the repo."
  else
    local unknown_mb=0 unknown_n=0 sd sbase stok skey sskip l
    echo "slice dirs at $MAIN_ROOT:"
    sz="$(du -sm "$MAIN_ROOT/target" 2>/dev/null | cut -f1)"
    printf '  spared  %-44s %s MB  (shared CARGO_TARGET_DIR — never reclaimed)\n' "$MAIN_ROOT/target" "${sz:-?}"
    for sd in "$MAIN_ROOT"/target-*; do
      [ -d "$sd" ] || continue
      sbase="$(basename "$sd")"
      # `target` cannot come out of a target-* glob and target-gate-* cannot parse as a slice dir;
      # both arms are asserted anyway rather than reasoned about, because the cost of being wrong
      # once is 67 GB or a red gate, and the cost of the arm is a string compare.
      case "$sbase" in
        target|target-gate-*) continue ;;
        # NOT a safety arm — the ticket-first rule below already spares this, and did so on its own
        # in this sweep's first real run. It is a REPORTING arm: target-dev-api is the operator's
        # live `make api` cache (Makefile:134,196), permanent by design, and the generic line below
        # filed it under "unparseable" and advised RENAMING IT so it could be reaped. Naming one
        # known-permanent dir is cheaper than printing that about the cache behind the API the
        # operator is using right now.
        target-dev-api)
          sz="$(du -sm "$sd" 2>/dev/null | cut -f1)"
          printf '  spared  %-44s %s MB  (operator dev API cache — permanent, Makefile owns it)\n' "$sd" "${sz:-?}"
          continue ;;
      esac
      if ! [[ "$sbase" =~ ^target-([Tt]-?[0-9]+)(-.*)?$ ]]; then
        sz="$(du -sm "$sd" 2>/dev/null | cut -f1)"
        unknown_mb=$((unknown_mb + ${sz:-0})); unknown_n=$((unknown_n + 1))
        printf '  spared  %-44s %s MB  (name carries no ticket id — no owner to check)\n' "$sd" "${sz:-?}"
        continue
      fi
      stok="${BASH_REMATCH[1]}"
      skey="$(printf '%s' "$stok" | tr 'A-Z' 'a-z' | tr -d '-')"
      sskip=0
      for l in $live; do [ "$skey" = "$l" ] && sskip=1; done
      [ "$sskip" -eq 1 ] && { printf '  spared  %-44s (live slice %s)\n' "$sd" "$stok"; continue; }
      sz="$(du -sm "$sd" 2>/dev/null | cut -f1)"
      rm -rf "$sd" 2>/dev/null && freed=$((freed + ${sz:-0})) && printf '  removed %-44s %s MB\n' "$sd" "${sz:-?}"
    done
    [ "$unknown_n" -gt 0 ] && echo "  ${unknown_mb} MB in ${unknown_n} unattributed dir(s) NOT reclaimed — reclaim removes only what it can attribute to a slice"
  fi


  # T-742 — orphan ad-hoc private dirs under $HOME/.cache/tbd-target-T-*. Swept by default with
  # the same live-slice spare set as MAIN_ROOT/target-T-*. The shared cache
  # ($HOME/.cache/tbd-target with no ticket suffix) is NEVER touched. Agents must still delete
  # their own dir before reporting; this is the parked/killed-agent half.
  if [ "$slice_dirs" -eq 0 ]; then
    echo "adhoc dirs at $HOME/.cache: not swept (--no-slice-dirs)"
  elif [ "$live_ok" -ne 1 ]; then
    echo "adhoc dirs at $HOME/.cache: REFUSED — live-slice set unknown; nothing swept."
  else
    local cd_path cbase ctok ckey cskip
    echo "adhoc dirs at $HOME/.cache:"
    sz="$(du -sm "$HOME/.cache/tbd-target" 2>/dev/null | cut -f1)"
    printf '  spared  %-44s %s MB  (shared agent cache — never reclaimed)\n' "$HOME/.cache/tbd-target" "${sz:-?}"
    for cd_path in "$HOME"/.cache/tbd-target-T-*; do
      [ -d "$cd_path" ] || continue
      cbase="$(basename "$cd_path")"
      if ! [[ "$cbase" =~ ^tbd-target-(T-[0-9]+)(-.*)?$ ]]; then
        sz="$(du -sm "$cd_path" 2>/dev/null | cut -f1)"
        printf '  spared  %-44s %s MB  (name carries no ticket id)\n' "$cd_path" "${sz:-?}"
        continue
      fi
      ctok="${BASH_REMATCH[1]}"
      ckey="$(printf '%s' "$ctok" | tr 'A-Z' 'a-z' | tr -d '-')"
      cskip=0
      for l in $live; do [ "$ckey" = "$l" ] && cskip=1; done
      [ "$cskip" -eq 1 ] && { printf '  spared  %-44s (live slice %s)\n' "$cd_path" "$ctok"; continue; }
      sz="$(du -sm "$cd_path" 2>/dev/null | cut -f1)"
      rm -rf "$cd_path" 2>/dev/null && freed=$((freed + ${sz:-0})) && printf '  removed %-44s %s MB\n' "$cd_path" "${sz:-?}"
    done
  fi

  if [ "$gate_dirs" -eq 1 ]; then
    echo "gate dirs (--gate-dirs${gate_min_age_days:+, min age ${gate_min_age_days}d}):"
    for d in "$MAIN_ROOT"/target-gate-* "$MAIN_ROOT"/dist-gate-*; do
      [ -e "$d" ] || continue
      if [ "$gate_min_age_days" -gt 0 ]; then
        local age_days=$(( ($(date +%s) - $(stat -c %Y "$d")) / 86400 ))
        [ "$age_days" -lt "$gate_min_age_days" ] && {
          printf '  spared (age %sd < %sd) %s\n' "$age_days" "$gate_min_age_days" "$d"
          continue
        }
      fi
      sz="$(du -sm "$d" 2>/dev/null | cut -f1)"
      rm -rf "$d" 2>/dev/null && freed=$((freed + ${sz:-0})) && printf '  removed %-44s %s MB\n' "$d" "${sz:-?}"
    done
  else
    local gate_sz=0 gd
    for gd in "$MAIN_ROOT"/target-gate-* "$MAIN_ROOT"/dist-gate-*; do
      [ -e "$gd" ] || continue
      gate_sz=$((gate_sz + $(du -sm "$gd" 2>/dev/null | cut -f1)))
    done
    [ "$gate_sz" -gt 0 ] && echo "gate dirs at $MAIN_ROOT: ${gate_sz} MB not reclaimed (pass --gate-dirs to opt in)"
  fi
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

# ── T-599 — THE PUSH GUARD ASKS GIT WHICH FILES ARE LFS. IT DOES NOT MATCH THE PATH. ────────────
#
# git-lfs is not installed in this container (any checkout needing it dies with
# `git-lfs filter-process: 1: git-lfs: not found`), so `--no-verify` is how work leaves this
# machine at all. The guard is real: pushing --no-verify over genuine LFS content publishes commits
# whose LFS objects were never uploaded, and every later clone breaks on them.
#
# WHAT THIS USED TO BE, AND WHY IT WAS WRONG:
#
#     if git diff --name-only origin/main..HEAD | grep -q '^packages/map-assets/'; then refuse
#
# It matched the DIRECTORY and assumed everything under it was LFS. `.gitattributes` has never said
# that — LFS covers exactly three globs there:
#
#     packages/map-assets/**/*.png   **/*.r16   **/*.tbd-sat
#
# Everything else beneath that tree is ordinary bytes. MEASURED 2026-07-31 while closing wave 74: a
# legitimate 19-commit push was refused, and all 30 files in the range resolved to
# `filter: unspecified` — including T-594's regenerated `everon/objects/prefabs.json.gz` and
# `everon/objects/type-inventory.json`, which are real content, not pointers. ZERO files in that
# range were LFS. The operator overrode the guard by hand, correctly.
#
# THE OVERRIDE IS THE DAMAGE, not the lost minutes. A guard that is wrong about ordinary work
# teaches whoever runs it that overriding is the normal way to push. The one time it is right, it
# gets overridden by reflex too — and that is the push that breaks the remote. Precision here is a
# safety property, not tidiness.
#
# So ask `git check-attr`, which consults the same .gitattributes git itself would, and refuse only
# on a genuine `filter: lfs`. And NAME the offending files: the old message named a directory, which
# the reader had no way to verify, so the only available responses were trust and override. A named
# path can be checked in one command, which the message prints.
#
# FAIL CLOSED. Every error path refuses. A guard that cannot answer the LFS question must not answer
# "go ahead" — that is the one direction where being wrong cannot be undone, because the remote is
# shared. This is deliberately NOT symmetric with the false-positive fix above.

# ── T-600 — EVERY COMMIT IN THE RANGE, AND EACH COMMIT'S OWN .gitattributes. ────────────────────
#
# T-599 fixed WHICH question this asks (check-attr, not path matching). It kept the wrong INPUT:
# `git diff --name-only origin/main..HEAD` is the ENDPOINT diff, so a file living only in an
# INTERMEDIATE commit — added, then deleted or renamed before HEAD — was never examined at all.
# MEASURED in a scratch repo with this function sourced verbatim: a `.tbd-sat` added in commit 2 of
# 3 and deleted in commit 3 gave `rc=0` and empty output — the guard ALLOWED the push. The commit
# publishing that pointer still reaches the remote, and every later checkout, bisect or
# `lfs fetch --all` of it breaks. A tool reporting success over an input it never read is the exact
# failure this guard exists to prevent, so it is not acceptable that it was pre-existing.
#
# So walk `git rev-list <range>` and diff EACH commit. Two flags are load-bearing:
#   -c      plain `diff-tree` prints NOTHING for a merge commit — trading one blind spot for
#           another (an evil merge that adds an LFS file in the merge itself). `-c` reports what a
#           merge introduced beyond ALL its parents, and is an ordinary diff on non-merges.
#           MEASURED: evil-merge case goes empty without it, names the file with it.
#   --root  a range containing the initial commit is otherwise silently empty.
#
# WHICH .gitattributes — HEAD'S, OR THE COMMIT'S OWN? THE COMMIT'S OWN, deliberately.
# `.gitattributes` can change inside the range. If `filter=lfs` was in force when the file landed
# and is gone by HEAD, check-attr at HEAD answers `unspecified` and the guard allows the push —
# MEASURED, the same blind spot in a second disguise. The commit's own rule is also the CORRECT
# predicate, not just the safer one: that rule is what decided whether git-lfs's clean filter ran,
# i.e. whether the blob in that commit is a pointer needing an uploaded object or ordinary bytes.
# HEAD's opinion of a historical blob is hearsay.
#
# It cuts both ways, and that is intended. A file committed as ordinary bytes BEFORE some later
# commit in the range adds an lfs rule is NOT refused: its blob is real content, nothing was ever
# cleaned, nothing needs uploading. Refusing it would be a fresh false positive of exactly the kind
# T-599 removed — and the false-positive fix is the reason the guard is believed at all.
#
# Git is 2.39 here, so `check-attr --source=<tree-ish>` (2.40+) does not exist. `--cached` does, and
# reads attributes from the index ONLY — so a throwaway GIT_INDEX_FILE filled by `read-tree <c>` is
# that answer. MEASURED: HEAD says `unspecified` for the Case-7 path, the temp index says `lfs`.
#
# Print, one per line, every path in <range> that genuinely resolves to `filter: lfs`.
# Empty output = nothing LFS in the range. rc 2 = COULD NOT TELL (never "nothing found").
lfs_paths_in_range() {
  local range="${1:-}" commits list attrs found idx c path value rc=0
  [ -n "$range" ] || return 2
  commits="$(mktemp)" || return 2
  list="$(mktemp)"  || { rm -f "$commits"; return 2; }
  attrs="$(mktemp)" || { rm -f "$commits" "$list"; return 2; }
  found="$(mktemp)" || { rm -f "$commits" "$list" "$attrs"; return 2; }
  idx="$(mktemp)"   || { rm -f "$commits" "$list" "$attrs" "$found"; return 2; }
  # A bad range dies here, before anything is examined — rc 2, not "nothing found".
  if git rev-list "$range" >"$commits" 2>/dev/null; then
    while IFS= read -r c; do
      # -z end to end: NUL-separated paths, so a filename containing a space, a quote or a newline
      # cannot split into two entries and get another file's attribute pinned on it.
      #
      # --diff-filter=d EXCLUDES deletions (lowercase excludes; uppercase D would select only
      # them). MEASURED on b5c1a8f7c: 4 files total, 3 deleted, `d` yields 1 and none of the 3.
      # Deleting an LFS file uploads nothing and so cannot leave a dangling object — counting
      # deletions would reintroduce a false refusal of exactly the kind this function exists to
      # remove. Getting this flag backwards is the one edit here that fails OPEN, which is why it
      # is measured and not assumed.
      git diff-tree -z --no-commit-id --name-only -r -c --root --diff-filter=d "$c" \
        >"$list" 2>/dev/null || { rc=2; break; }
      # Attributes AS OF $c: a fresh index holding that commit's tree, read with --cached so the
      # working tree's (i.e. HEAD's) .gitattributes cannot answer for a historical commit.
      rm -f "$idx" "$idx.lock"
      GIT_INDEX_FILE="$idx" git read-tree "$c" 2>/dev/null || { rc=2; break; }
      GIT_INDEX_FILE="$idx" git check-attr --cached -z --stdin filter \
        <"$list" >"$attrs" 2>/dev/null || { rc=2; break; }
      # `check-attr -z` emits NUL-separated triples: <path> <attr-name> <value>.
      while IFS= read -r -d '' path && IFS= read -r -d '' _ && IFS= read -r -d '' value; do
        [ "$value" = "lfs" ] && printf '%s\n' "$path" >>"$found"
      done <"$attrs"
    done <"$commits"
  else
    rc=2
  fi
  # One line per path even when several commits touched it; nothing at all if we could not tell.
  [ "$rc" -eq 0 ] && { sort -u "$found" || rc=2; }
  rm -f "$commits" "$list" "$attrs" "$found" "$idx" "$idx.lock"
  return "$rc"
}

cmd_push() {
  local range="origin/main..HEAD" lfs rc n
  lfs="$(lfs_paths_in_range "$range")"; rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "REFUSING --no-verify: could not determine LFS status for $range."
    echo "        One of \`git rev-list\` / \`diff-tree\` / \`read-tree\` / \`check-attr\` failed, so this"
    echo "        guard has no answer. It refuses rather than guessing — an unchecked --no-verify"
    echo "        push is the unrecoverable one."
    return 1
  fi
  if [ -n "$lfs" ]; then
    n="$(printf '%s\n' "$lfs" | wc -l)"
    echo "REFUSING --no-verify: $n file(s) in the commits of $range resolve to \`filter: lfs\`:"
    printf '%s\n' "$lfs" | sed 's/^/          /'
    echo "        Find the commit that publishes one:  git log --oneline $range -- <path>"
    echo "        Ask HEAD about it:                   git check-attr filter -- <path>"
    echo "        HEAD may answer \`unspecified\` and this guard still be right: it asks each commit's"
    echo "        OWN .gitattributes, because that is the rule that decided whether the blob in that"
    echo "        commit is an LFS pointer. See lfs_paths_in_range above."
    echo "        git-lfs is absent here, so --no-verify would publish commits whose LFS objects"
    echo "        are never uploaded. Install git-lfs and push normally."
    return 1
  fi
  git push --no-verify origin main
}


# T-742 — ad-hoc `cargo test` into a PER-SLICE private CARGO_TARGET_DIR.
#
# DEFECT: concurrent slice worktrees export the same shared cache
# (`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` / `$MAIN_ROOT/target`). `cargo test`
# BUILDS AND THEN RUNS a binary, so one worktree can execute another's `website_frontend-<hash>`
# (T-649 live: arsenal.rs:4733 failure that did not exist in that tree; line tracked a sibling).
# The per-slice gate and the wave-gate `test frontend` step already use private dirs; ad-hoc
# agent invocations did not, and the brief only ADVISED a private dir — nothing enforced it.
#
# THIS PATH: `wave.sh test --slice T-nnn -p <crate> …` pins
# `$HOME/.cache/tbd-target-<SLICE>` (override with TBD_ADHOC_TARGET_DIR only when it resolves to
# that same default, or to a non-`T-*` verifier path — see F2 below), refuses /tmp and any
# collapse onto the TRUE shared roots (`$HOME/.cache/tbd-target` and `$MAIN_ROOT/target` —
# NEVER against whatever CARGO_TARGET_DIR currently holds; that false-refused the sanctioned
# per-slice path when an agent had already exported it), mtime-bumps via touch_changed (same
# fingerprint cure as the gate), and runs with CARGO_INCREMENTAL=0. It does NOT take the shared
# gate lock — that lock serialises the SHARED gate dirs (target-gate-*); isolation here is the
# private directory itself (measured: frontend-only private dir ~2.7 GB, not a 57 GB
# shared-cache clone). Cargo does NOT rebuild across worktrees that share a target dir — the
# private path is the mitigator; shared-dir `cargo test` remains the foreign-binary class.
#
# Keep it lean: require an explicit `-p` / `--package` among args. Delete the private dir before
# reporting (`rm -rf "$HOME/.cache/tbd-target-T-nnn"`) — never print that for a foreign-slice or
# live-worktree path. `reclaim` also sweeps orphan `~/.cache/tbd-target-T-*` for dead slices.
cmd_test() {
  local tid="" args=() a
  while [ $# -gt 0 ]; do
    case "$1" in
      --slice)
        tid="${2:-}"
        [ -n "$tid" ] || { echo "test: REFUSING — --slice needs a ticket id (T-nnn)" >&2; return 2; }
        shift 2 ;;
      --)
        # Keep cargo's `--` separator (e.g. `… -- --list`). Dropping it made
        # `cargo test` see `--list` as its own flag and refuse.
        args+=("--")
        shift
        while [ $# -gt 0 ]; do args+=("$1"); shift; done
        break ;;
      *)
        args+=("$1")
        shift ;;
    esac
  done
  if [ -z "$tid" ]; then
    echo "test: REFUSING — --slice T-nnn is required."
    echo "        Bare \`cargo test\` against the shared CARGO_TARGET_DIR is the T-742"
    echo "        cross-worktree false-binary class. Sanctioned path:"
    echo "          bash scripts/platform/wave.sh test --slice T-742 -p website-frontend"
    return 2
  fi
  case "$tid" in
    [Tt]-[0-9]*) ;;
    *)
      echo "test: REFUSING — slice id '$tid' (expected T-nnn)" >&2
      return 2 ;;
  esac
  # Normalise t-742 → T-742 without touching digits.
  tid="T-${tid#*[Tt]-}"

  if [ "${#args[@]}" -eq 0 ]; then
    echo "test: REFUSING — pass cargo test args (at least -p <crate>)."
    echo "        An unbounded invocation would inflate the private dir toward a full workspace"
    echo "        build. Keep ad-hoc dirs lean (frontend-only measured ~2.7 GB)."
    echo "        Example: bash scripts/platform/wave.sh test --slice $tid -p website-frontend"
    return 2
  fi

  # NIT: prose said "at least -p <crate>"; enforce it — non-empty args without -p/--package
  # still accept unbounded / mis-aimed invocations that inflate the private dir.
  local has_pkg=0
  for a in "${args[@]}"; do
    case "$a" in
      -p|--package|-p?*|--package=*) has_pkg=1; break ;;
    esac
  done
  if [ "$has_pkg" -eq 0 ]; then
    echo "test: REFUSING — cargo test args must include -p / --package <crate>."
    echo "        Example: bash scripts/platform/wave.sh test --slice $tid -p website-frontend"
    return 2
  fi

  local default_priv="$HOME/.cache/tbd-target-$tid"
  local priv="${TBD_ADHOC_TARGET_DIR:-$default_priv}"
  case "$priv" in
    /tmp/*|/var/tmp/*)
      echo "test: REFUSING — private target dir must not be under /tmp ($priv)."
      echo "        Host-native rule: never /tmp for CARGO_TARGET_DIR."
      return 2 ;;
  esac
  mkdir -p "$priv" || { echo "test: cannot create $priv" >&2; return 2; }
  mkdir -p "$default_priv" 2>/dev/null || true

  local priv_r cache_r main_r default_r
  priv_r="$(readlink -f -- "$priv")"
  default_r="$(readlink -f -- "$default_priv" 2>/dev/null || printf '%s' "$default_priv")"
  # F1: compare ONLY against true shared roots — never against whatever CARGO_TARGET_DIR
  # currently holds (that false-refused when env already pointed at the per-slice private dir).
  cache_r="$(readlink -f -- "$HOME/.cache/tbd-target" 2>/dev/null || printf '%s' "$HOME/.cache/tbd-target")"
  main_r="$(readlink -f -- "$MAIN_ROOT/target" 2>/dev/null || printf '%s' "$MAIN_ROOT/target")"
  if [ "$priv_r" = "$cache_r" ] || [ "$priv_r" = "$main_r" ]; then
    echo "test: REFUSING — private dir collapsed onto the shared CARGO_TARGET_DIR ($priv_r)."
    echo "        That is exactly the T-742 defect. Unset TBD_ADHOC_TARGET_DIR or point it at"
    echo "        a per-slice path under \$HOME/.cache/tbd-target-$tid."
    return 2
  fi

  # F2: TBD_ADHOC_TARGET_DIR must resolve to this slice's default
  # (`$HOME/.cache/tbd-target-$tid`) OR a non-`T-*` verifier path (basename lacks
  # `tbd-target-T-<digits>` — e.g. `tbd-target-wave138-verify`). A foreign-slice
  # `tbd-target-T-739` under `--slice T-999` is REFUSED — never print rm -rf for it.
  local base token=""
  base="$(basename -- "$priv_r")"
  token="$(printf '%s' "$base" | sed -n 's/^tbd-target-\([Tt]-[0-9][0-9]*\).*/\1/p')"
  if [ -n "$token" ]; then
    token="T-${token#*[Tt]-}"
  fi
  if [ -n "${TBD_ADHOC_TARGET_DIR:-}" ] && [ "$priv_r" != "$default_r" ]; then
    if [ -n "$token" ]; then
      echo "test: REFUSING — TBD_ADHOC_TARGET_DIR is not the default per-slice path ($priv_r)."
      if [ "$token" != "$tid" ]; then
        echo "        Foreign-slice token '$token' != --slice '$tid'."
      fi
      echo "        Allowed overrides: \$HOME/.cache/tbd-target-$tid, or a non-T-* verifier"
      echo "        path (e.g. \$HOME/.cache/tbd-target-wave138-verify)."
      return 2
    fi
    # token empty → non-T-* verifier path — allowed (documented above).
  fi

  # Never advertise rm -rf for a path whose ticket token differs from --slice or is a
  # live worktree's foreign cache. Default per-slice for THIS tid + non-T-* verifier OK.
  local allow_rm=1 live_tokens="" wt_line t_live
  if [ -n "$token" ] && [ "$token" != "$tid" ]; then
    allow_rm=0
  fi
  while IFS= read -r wt_line; do
    t_live="$(printf '%s' "$wt_line" | sed -n 's|.*/\([Tt]-[0-9][0-9]*\)\(/*\)*$|\1|p')"
    if [ -z "$t_live" ]; then
      t_live="$(printf '%s' "$wt_line" | sed -n 's|.*/\([Tt]-[0-9][0-9]*\)/.*|\1|p')"
    fi
    if [ -n "$t_live" ]; then
      t_live="T-${t_live#*[Tt]-}"
      live_tokens="$live_tokens $t_live"
    fi
  done < <(git -C "$MAIN_ROOT" worktree list --porcelain 2>/dev/null | sed -n 's/^worktree //p')
  if [ -n "$token" ] && [ "$token" != "$tid" ]; then
    case " $live_tokens " in
      *" $token "*) allow_rm=0 ;;
    esac
    allow_rm=0
  fi
  if [ -z "$token" ] || [ "$priv_r" = "$default_r" ]; then
    allow_rm=1
  fi
  # Foreign token always blocks the banner even if somehow past the refuse (defence in depth).
  if [ -n "$token" ] && [ "$token" != "$tid" ]; then
    allow_rm=0
  fi

  echo "═══ ad-hoc test $tid ═══"
  echo "CARGO_TARGET_DIR=$priv_r  (private — not the shared cache)"
  if [ "$allow_rm" -eq 1 ]; then
    echo "delete before report: rm -rf '$priv_r'"
  else
    echo "delete before report: (omitted — path token is foreign or live; do not rm -rf)"
  fi

  # Same mtime-bump the gate uses so a WARM private dir cannot keep fingerprints from before
  # this worktree's own edits. Cross-worktree isolation is the private dir; this covers the
  # same-worktree stale-fingerprint half of the pattern.
  touch_changed || return $?

  # hostrun + explicit env: distrobox-host-exec does not forward the shell environment.
  hostrun env "CARGO_TARGET_DIR=$priv_r" "CARGO_INCREMENTAL=0" cargo test "${args[@]}"
}

case "${1:-status}" in
  status) cmd_status ;;
  prep)   cmd_prep ;;
  test)   shift; cmd_test "$@"; exit $? ;;
  # `--migrate-persist [audit|advance]` runs the T-555 populated-database step alone. It is how the
  # step was proven to go red on the real defects and green on the fix, and it is what to reach for
  # when a gate reports migration drift and you want the detail without a full gate run.
  gate)   case "${2:-}" in
            --slice)           gate_slice "${3:-}" ;;
            # `advance` writes the shared persist DB, so it takes the same lock the wave gate holds
            # when it calls this. GATE_LOCK_HELD is deliberately not settable from the environment
            # (it is reset at load, below), so there is no way to skip this by exporting a variable.
            --migrate-persist)
              if [ "${3:-audit}" = advance ]; then
                take_gate_lock "migrate-persist advance" || exit $?
              fi
              gate_db_migrate_persist "${3:-audit}" ;;
            *)                 cmd_gate "${2:-}" ;;
          esac ;;
  wave)   if [ "${2:-}" = "--close" ]; then cmd_wave_close; else cmd_wave; fi ;;
  verified) cmd_verified "${2:-}" ;;
  reclaim) shift; cmd_reclaim "$@" ;;
  land)   shift; cmd_land "$@" ;;
  revert) cmd_revert "${2:-}" ;;
  push)   cmd_push ;;
  *) sed -n '2,40p' "$0"; exit 1 ;;
esac
