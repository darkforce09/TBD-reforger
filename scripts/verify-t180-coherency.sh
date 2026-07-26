#!/usr/bin/env bash
# T-180.10 — permanent Class-R coherency gate for ORBAT + Eden placement.
# Fail-fast. Packages: map-engine-core, website-frontend, map-engine-render.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="${HOME}/.cargo/bin:${PATH}"

fail() { echo "verify-t180 FAIL: $*" >&2; exit 1; }
ok() { echo "verify-t180 OK: $*"; }

# ── Static bans ──────────────────────────────────────────────────────────────
if rg -n 'ensure_default_squad' apps/website/frontend/src/editor_ops.rs >/dev/null 2>&1; then
  fail "ensure_default_squad still present in editor_ops.rs"
fi
ok "no ensure_default_squad on place path"

if rg -n 'loadout: String::new\(\)' crates/map-engine-core/src/mission/orbat.rs >/dev/null 2>&1; then
  fail "orbat.rs still hardcodes loadout: String::new()"
fi
ok "no loadout String::new() hardcode in derive"

if rg -ni 'standardization|IFAK|Grenade Complement' \
  apps/website/frontend/src/orbat_manager.rs \
  apps/website/frontend/src/eden_chrome.rs >/dev/null 2>&1; then
  fail "Standardization / IFAK / Grenade Complement UI strings found (L8 omit)"
fi
ok "no Standardization UI strings"

rg -n 'SIDE_BLUFOR_RGBA: \[u8; 4\] = \[173, 198, 255, 255\]' \
  crates/map-engine-core/src/slots_gpu.rs >/dev/null \
  || fail "SIDE_BLUFOR_RGBA pin missing"
rg -n 'SIDE_OPFOR_RGBA: \[u8; 4\] = \[248, 113, 113, 255\]' \
  crates/map-engine-core/src/slots_gpu.rs >/dev/null \
  || fail "SIDE_OPFOR_RGBA pin missing"
rg -n 'SIDE_INDFOR_RGBA: \[u8; 4\] = \[34, 197, 94, 255\]' \
  crates/map-engine-core/src/slots_gpu.rs >/dev/null \
  || fail "SIDE_INDFOR_RGBA pin missing"
ok "RGBA side pins present"

# ── A / B / H — doc feature ──────────────────────────────────────────────────
cargo test -p map-engine-core --features doc --lib place_ -- --quiet
cargo test -p map-engine-core --features doc --lib set_leader_exclusive -- --quiet
cargo test -p map-engine-core --features doc --lib empty_squad_garbage_collected -- --quiet
cargo test -p map-engine-core --features doc --lib move_slot_bidirectional -- --quiet
cargo test -p map-engine-core --features doc --lib leader_invariant_holds -- --quiet
cargo test -p map-engine-core --features doc --lib attach_vehicle_roundtrip -- --quiet
cargo test -p map-engine-core --features doc --lib apply_faction_ -- --quiet
ok "doc-feature place/mutator/apply gates"

# ── C / D / G / vehicle pack ─────────────────────────────────────────────────
cargo test -p map-engine-core --lib side_tint_three_distinct -- --quiet
cargo test -p map-engine-core --lib squad_link_ -- --quiet
cargo test -p map-engine-core --lib format_slot_line -- --quiet
cargo test -p map-engine-core --lib pack_vehicle_instances -- --quiet
cargo test -p map-engine-render --lib mission_vehicles -- --quiet
ok "tint / links / slot_line / vehicles lane"

# ── I — mission feature derive / compile ─────────────────────────────────────
cargo test -p map-engine-core --features mission --lib derive_fills_loadout -- --quiet
cargo test -p map-engine-core --features mission --lib derive_empty_loadout -- --quiet
cargo test -p map-engine-core --features mission --lib derives_from_editor_sorted -- --quiet
cargo test -p map-engine-core --features mission --lib compile_export_orbat_loadout -- --quiet
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
cargo test -p map-engine-core --features mission --lib \
  the_compile_boundary_ledger_is_checked_against_the_contract -- --quiet
cargo test -p map-engine-core --features mission --lib \
  a_compiled_slot_carries_exactly_these_keys -- --quiet
cargo test -p map-engine-core --features mission --lib \
  the_vehicle_row_still_has_the_shape_this_module_reads -- --quiet
ok "compile-boundary ledger + compiled-slot key set + vehicle contract floor"

# ── E / F / G / H / I — FE (bin crate) ────────────────────────────────────────
cargo test -p website-frontend eden_side -- --quiet
cargo test -p website-frontend apply_eden -- --quiet
cargo test -p website-frontend objects_chip -- --quiet
cargo test -p website-frontend open_arsenal -- --quiet
cargo test -p website-frontend g1_dialog -- --quiet
cargo test -p website-frontend orbat_ -- --quiet
ok "website-frontend Eden/ORBAT gates"

echo "verify-t180: ALL PASS"
