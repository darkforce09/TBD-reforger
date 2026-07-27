#!/usr/bin/env bash
# T-180.10 — permanent Class-R coherency gate for ORBAT + Eden placement.
# Fail-fast. Packages: map-engine-core, website-frontend, map-engine-render.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="${HOME}/.cargo/bin:${PATH}"

fail() { echo "verify-t180 FAIL: $*" >&2; exit 1; }
ok() { echo "verify-t180 OK: $*"; }

# ── How a static check reports, and why it is not a bare `if rg` (T-216) ──────
#
# The three bans below used to read `if rg -n PAT FILE >/dev/null 2>&1; then fail; fi`.
# That form reports OK for three different outcomes and can only tell one of them apart:
#
#   exit 0    match found            -> ban violated        -> correctly FAILED
#   exit 1    no match               -> ban holds           -> correctly passed
#   exit 2    TARGET FILE MISSING    -> check never ran     -> printed OK
#   exit 127  SEARCH TOOL ABSENT     -> check never ran     -> printed OK
#
# The last two are this program's signature defect — a tool reporting success over an
# input it never examined — living inside the script written to catch it.
#
# MEASURED 2026-07-26: `rg` is present in the dev container and ABSENT on the host, and
# cargo only runs on the host (glibc 2.36 vs 2.39, E0463). So every host run of this gate
# printed three OK lines for bans that had not executed a single comparison. Renaming
# `slots_gpu.rs` would have produced the same false green by the other route.
#
# Two changes, and both are needed — one alone still leaves a hole.
#
# 1. `grep -E` replaces `rg`, because grep exists on BOTH sides of that bridge. This is a
#    dependency REMOVED rather than a dependency asserted: a hard `command -v rg || fail`
#    preflight would have been honest, but it would also have made this gate unrunnable in
#    the only environment where its cargo half works. The patterns are unchanged and mean
#    the same thing in ERE — `\(`, `\[` and `|` are identical in both engines — and every
#    call names explicit files, so ripgrep's recursion and gitignore defaults were never
#    in play.
# 2. The exit status is READ instead of collapsed to a boolean, and the target files are
#    checked for existence first. Anything the helpers cannot examine is a FAILURE. A
#    check that did not run must never be reported as a check that passed.

# `ban <message> [-i] <pattern> <file>...` — the pattern must NOT appear in any file.
ban() {
  local msg="$1"; shift
  local flags=(-E)
  if [ "$1" = "-i" ]; then flags+=(-i); shift; fi
  local pat="$1"; shift
  local f
  for f in "$@"; do
    [ -f "$f" ] || fail "$msg — target file missing: $f. The ban could not run, and a \
moved or deleted file must not read as a clean result."
  done
  local status=0
  grep "${flags[@]}" -- "$pat" "$@" >/dev/null 2>&1 || status=$?
  case "$status" in
    0) fail "$msg" ;;
    1) : ;; # no match — the ban holds, and we know it holds because the search ran
    *) fail "$msg — grep exited $status (tool absent or bad pattern). Refusing to report \
OK on a check that did not execute." ;;
  esac
}

# `require <message> <pattern> <file>` — the pattern MUST appear.
#
# The `... || fail` form this replaces was already correct on the fail-closed question:
# `||` fires on EVERY non-zero status, so an absent tool or a moved file failed loudly
# rather than passing. It is kept as a helper only to separate the two causes — "the pin
# is genuinely gone" and "the check could not run" send a reader to different places, and
# the old single message sent everyone to the first one.
require() {
  local msg="$1" pat="$2" file="$3"
  [ -f "$file" ] || fail "$msg — target file missing: $file. The pin could not be checked."
  local status=0
  grep -E -- "$pat" "$file" >/dev/null 2>&1 || status=$?
  case "$status" in
    0) : ;;
    1) fail "$msg" ;;
    *) fail "$msg — grep exited $status (tool absent or bad pattern). Refusing to report \
OK on a check that did not execute." ;;
  esac
}

# ── Static bans ──────────────────────────────────────────────────────────────
ban "ensure_default_squad still present in editor_ops.rs" \
  'ensure_default_squad' apps/website/frontend/src/editor_ops.rs
ok "no ensure_default_squad on place path"

ban "orbat.rs still hardcodes loadout: String::new()" \
  'loadout: String::new\(\)' crates/map-engine-core/src/mission/orbat.rs
ok "no loadout String::new() hardcode in derive"

ban "Standardization / IFAK / Grenade Complement UI strings found (L8 omit)" \
  -i 'standardization|IFAK|Grenade Complement' \
  apps/website/frontend/src/orbat_manager.rs \
  apps/website/frontend/src/eden_chrome.rs
ok "no Standardization UI strings"

require "SIDE_BLUFOR_RGBA pin missing" \
  'SIDE_BLUFOR_RGBA: \[u8; 4\] = \[173, 198, 255, 255\]' \
  crates/map-engine-core/src/slots_gpu.rs
require "SIDE_OPFOR_RGBA pin missing" \
  'SIDE_OPFOR_RGBA: \[u8; 4\] = \[248, 113, 113, 255\]' \
  crates/map-engine-core/src/slots_gpu.rs
require "SIDE_INDFOR_RGBA pin missing" \
  'SIDE_INDFOR_RGBA: \[u8; 4\] = \[34, 197, 94, 255\]' \
  crates/map-engine-core/src/slots_gpu.rs
ok "RGBA side pins present"

# ── cargo_test_pin — selector must match ≥1 test (T-424) ─────────────────────
#
# `cargo test --lib <selector>` exits 0 when the filter matches NOTHING:
#
#   test result: ok. 0 passed; 0 failed; … N filtered out     rc=0
#
# Measured 2026-07-27:
#   cargo test -p map-engine-core --lib --features doc,mission -- \
#     zzz_no_such_test_exists_anywhere
#   # → 0 passed; 277 filtered out; rc=0
#
# Every selector below used to be a bare `cargo test`. A typo or rename of a pinned
# test name therefore printed verify-t180 OK having run zero assertions — the same
# signature defect as the T-216 `if rg` bans: success reported over an input never
# examined. This wrapper parses every `test result: … N passed` line cargo prints
# for the invocation, sums N, and FAILS when the sum is 0 (or when no result line
# appears at all). cargo's own non-zero exit (compile error / failed test) still
# fails first.
cargo_test_pin() {
  local out status=0 passed
  out="$(cargo test "$@" 2>&1)" || status=$?
  printf '%s\n' "$out"
  if [ "$status" -ne 0 ]; then
    fail "cargo test $* exited $status"
  fi
  # sed+awk (not grep) so pipefail cannot abort before we classify: no result line
  # and "0 passed" are different failures and both must be loud.
  local nlines
  nlines="$(printf '%s\n' "$out" | sed -n 's/.*test result:.* \([0-9][0-9]*\) passed.*/\1/p' | wc -l)"
  nlines="${nlines// /}"
  if [ "$nlines" -lt 1 ]; then
    fail "cargo test $* — no 'test result: N passed' line. Refusing to report OK on a check that did not execute."
  fi
  passed="$(printf '%s\n' "$out" | sed -n 's/.*test result:.* \([0-9][0-9]*\) passed.*/\1/p' | awk '{s += $1} END {print s+0}')"
  if [ "$passed" -lt 1 ]; then
    fail "cargo test $* — 0 tests passed (selector matched nothing). A renamed/typo'd pin must not silently empty."
  fi
}

# ── A / B / H — doc feature ──────────────────────────────────────────────────
#
# `doc mission`, not `doc` alone (T-216). `doc/store.rs`'s own tests call
# `crate::mission::compile::compile_payload` (store.rs:2589, 2601, 2909, 2932 — the
# hydrate/compile round-trips T-344 added), and `mission` is a separate feature gate at
# `lib.rs:23`. So `--features doc` cannot COMPILE the lib test target:
#
#   error[E0433]: cannot find `mission` in `crate`   (x4)
#   error: could not compile `map-engine-core` (lib test) due to 4 previous errors
#
# `set -euo pipefail` means that killed the script on the FIRST of these seven lines, so
# every section below — the tint/links lane, the derive gates, the compile boundary and
# the entire website-frontend block — had not run since T-344. Reproduced on main at
# 33a7aa85, so it predates this slice and is not caused by it.
#
# Fixed HERE rather than by gating those four tests in `doc/store.rs`, which is another
# slice's file in this wave. Adding `mission` cannot weaken the gate: it is strictly more
# code compiled, the selectors are unchanged, and the seven lines now share one test
# binary with the `mission` section below instead of building a second feature set.
cargo_test_pin -p map-engine-core --features "doc mission" --lib place_ -- --quiet
cargo_test_pin -p map-engine-core --features "doc mission" --lib set_leader_exclusive -- --quiet
cargo_test_pin -p map-engine-core --features "doc mission" --lib empty_squad_garbage_collected -- --quiet
cargo_test_pin -p map-engine-core --features "doc mission" --lib move_slot_bidirectional -- --quiet
cargo_test_pin -p map-engine-core --features "doc mission" --lib leader_invariant_holds -- --quiet
cargo_test_pin -p map-engine-core --features "doc mission" --lib attach_vehicle_roundtrip -- --quiet
cargo_test_pin -p map-engine-core --features "doc mission" --lib apply_faction_ -- --quiet
ok "doc-feature place/mutator/apply gates"

# ── C / D / G / vehicle pack ─────────────────────────────────────────────────
cargo_test_pin -p map-engine-core --lib side_tint_three_distinct -- --quiet
cargo_test_pin -p map-engine-core --lib squad_link_ -- --quiet
cargo_test_pin -p map-engine-core --lib format_slot_line -- --quiet
cargo_test_pin -p map-engine-core --lib pack_vehicle_instances -- --quiet
cargo_test_pin -p map-engine-render --lib mission_vehicles -- --quiet
ok "tint / links / slot_line / vehicles lane"

# ── I — mission feature derive / compile ─────────────────────────────────────
cargo_test_pin -p map-engine-core --features mission --lib derive_fills_loadout -- --quiet
cargo_test_pin -p map-engine-core --features mission --lib derive_empty_loadout -- --quiet
cargo_test_pin -p map-engine-core --features mission --lib derives_from_editor_sorted -- --quiet
cargo_test_pin -p map-engine-core --features mission --lib compile_export_orbat_loadout -- --quiet
ok "derive/compile loadout gates"

# ── T-216 — THE COMPILE BOUNDARY. Read this before trimming the list above. ───
#
# Every selector in this file up to here proves the editor can AUTHOR a T-180 value
# (doc::place_orbat, doc::store), that the map can DRAW it (slots_gpu, map-engine-render)
# or that the ORBAT derive keeps it (mission::orbat, mission::compile). Not one of them
# named a test in `mission::flatten` — so the gate never crossed the edge where the
# document is handed to the game server, and six values crossed nothing: a squad's
# leaderSlotId, a slot's tag / callsign / rank / stance, and the whole vehicle roster.
# Measured 2026-07-26: a payload authoring all six compiles to a document carrying none,
# with this script printing ALL PASS. A gate is worth nothing until you know what it looked at.
#
# These two are that missing edge. The ledger walks each value from the saved payload to
# the serialized wire and asserts against mission.schema.json — so when the contract widens
# (T-242), the row for the newly-legal key turns red and the dead feature becomes visible
# work instead of staying quietly dead. The second pins the compiled slot's key set, so
# nothing can be added to or removed from the website<->mod interface in silence.
cargo_test_pin -p map-engine-core --features mission --lib \
  the_compile_boundary_ledger_is_checked_against_the_contract -- --quiet
cargo_test_pin -p map-engine-core --features mission --lib \
  a_compiled_slot_carries_exactly_these_keys -- --quiet
cargo_test_pin -p map-engine-core --features mission --lib \
  the_vehicle_row_still_has_the_shape_this_module_reads -- --quiet
ok "compile-boundary ledger + compiled-slot key set + vehicle contract floor"

# ── E / F / G / H / I — FE (bin crate) ────────────────────────────────────────
cargo_test_pin -p website-frontend eden_side -- --quiet
cargo_test_pin -p website-frontend apply_eden -- --quiet
cargo_test_pin -p website-frontend objects_chip -- --quiet
cargo_test_pin -p website-frontend open_arsenal -- --quiet
cargo_test_pin -p website-frontend g1_dialog -- --quiet
cargo_test_pin -p website-frontend orbat_ -- --quiet
ok "website-frontend Eden/ORBAT gates"

echo "verify-t180: ALL PASS"
