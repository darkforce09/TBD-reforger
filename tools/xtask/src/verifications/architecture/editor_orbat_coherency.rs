//! The permanent Class-R coherency gate for ORBAT + Eden placement, over the map engine and the
//! frontend. Fail-fast.
//!
//! ── A SEARCH THAT DID NOT RUN IS NOT A PASS ──────────────────────────────────────────────────
//!
//! A ban written as `if <search> PATTERN FILE >/dev/null; then fail; fi` reports OK for three
//! different outcomes and can tell only one of them apart:
//!
//! ```text
//!   exit 0    match found            -> ban violated        -> correctly FAILED
//!   exit 1    no match               -> ban holds           -> correctly passed
//!   exit 2    TARGET FILE MISSING    -> check never ran     -> printed OK
//!   exit 127  SEARCH TOOL ABSENT     -> check never ran     -> printed OK
//! ```
//!
//! The last two are this program's signature defect — a tool reporting success over an input it
//! never examined — living inside the check written to catch it. MEASURED 2026-07-26: a search
//! tool present in the dev container and ABSENT on the host turns every host run into three OK
//! lines for bans that executed no comparison at all; renaming a scanned file produces the same
//! false green by the other route.
//!
//! So there is no external matcher here. [`Pattern`] is the `regex` crate compiled in with
//! `multi_line(true)`, so `^`/`$` stay LINE anchors and "tool absent" is unreachable.
//! [`gate::ban`] / [`gate::require`] hand back a [`Verdict`] with no `bool` conversion, and the
//! `translate` match is exhaustive — a cause added to `NotRun` breaks this file at compile time
//! instead of quietly becoming a pass. Every check names explicit files, so no recursive search
//! and no ignore-file default is in play.
//!
//! ── `--features "scenario store"`: THE MISSION-AUTHORING TIER ────────────────────────────────
//!
//! The store rows exercise `data/store` (the Yjs document store, feature `store`), and the derive
//! and compile rows exercise `data/scenario` (the mission AST and compiler, feature `scenario`).
//! `store` enables `scenario`, and `scenario` is the crate default, so the pair builds exactly what
//! `store` alone builds; the pin names both so its failure text shows the whole tier, and every
//! row of the tier shares one lib test binary.
//!
//! ── A SELECTOR THAT MATCHES NOTHING IS NOT A PASS ────────────────────────────────────────────
//!
//! `cargo test --lib <selector>` exits 0 when the filter matches NOTHING. MEASURED 2026-07-27:
//! a selector naming no test printed `0 passed; 277 filtered out`, rc=0. A typo or a rename would
//! therefore print OK having run zero assertions — the same defect as the bans above. `classify`
//! sums every `test result: … N passed` line and fails on 0, and separately on no result line at
//! all.
//!
//! ── OUTPUT IS A CONTRACT, AND SO IS THE EXIT CODE ────────────────────────────────────────────
//!
//! Failures print one `editor-orbat-coherency FAIL: …` line on **stderr** and `ok` lines on
//! stdout — not [`verification_core::Finding`]'s two-line render — and the exit status is
//! [`Verdict::into_binary_exit_code`]'s **1** for both failure kinds, rather than the
//! four-outcome 2 that [`crate::verifications::registry::object_registry_aliases`] chose. Two
//! deviations are deliberate and reachable only when cargo itself does not run: see
//! `not_run_clause`.
//!
//! **The output is not reproducible run to run.** Two consecutive warm runs differ on wall-clock
//! readings (`Finished … in 0.07s` vs `0.06s`); cold runs add `Compiling <crate>` lines whose
//! ORDER is the build scheduler's, and `Running unittests` embeds `$CARGO_TARGET_DIR`. That is
//! cargo's own stdout passed through. The ordering that IS ours — checks, `ok` lines, pin order —
//! comes from the static tables below, never from a directory walk.

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use regex::Regex;
use verification_core::{NotRun, Pattern, Verdict, gate};

// ── Targets, relative to the repo root ───────────────────────────────────────────────────────
// A "target file missing" line prints the RELATIVE path, so these are joined onto `repo_root` to
// read and stripped back for the message, rather than mutating this process's cwd — tests run in
// parallel threads.
#[cfg(test)]
const EDITOR_OPS: &str =
    "apps/frontend/src/workspaces/editor/bridge/host_state/editor_context/mod.rs";
// The place path spans two crates: the document mutations in the map engine
// (`data/store/operations` and the hosted commands that drive them) and the host half in the
// frontend that arms a placement and commits it. Both sides are scanned together, and the scratch
// fixtures perturb one of each, so moving a mutation across the crate boundary cannot bypass the
// ban.
const EDITOR_OPS_SPLIT: &[&str] = &[
    "apps/frontend/src/workspaces/editor/arsenal/loadout_commands.rs",
    "apps/frontend/src/workspaces/editor/bridge/tactical_graphics_authoring.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/armed_placement/map_release.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/armed_placement/mod.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/armed_placement/palette_arming.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/armed_placement/zone_draw.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/editor_context/attributes_modal.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/editor_context/dock_mirrors.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/editor_context/document_fields.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/editor_context/installation.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/editor_context/mod.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/entity_selection.rs",
    "apps/frontend/src/workspaces/editor/bridge/host_state/undo_grouped_gestures.rs",
    "legacy/map_engine/src/data/store/operations/apply_faction/apply.rs",
    "legacy/map_engine/src/data/store/operations/apply_faction/authorship.rs",
    "legacy/map_engine/src/data/store/operations/apply_faction/library.rs",
    "legacy/map_engine/src/data/store/operations/apply_faction/mod.rs",
    "legacy/map_engine/src/data/store/operations/assets.rs",
    "legacy/map_engine/src/data/store/operations/attrs.rs",
    "legacy/map_engine/src/data/store/operations/cargo.rs",
    "legacy/map_engine/src/data/store/operations/cargo_rules.rs",
    "legacy/map_engine/src/data/store/operations/compositions.rs",
    "legacy/map_engine/src/data/store/operations/document_index.rs",
    "legacy/map_engine/src/data/store/operations/entity/clipboard.rs",
    "legacy/map_engine/src/data/store/operations/entity/comments.rs",
    "legacy/map_engine/src/data/store/operations/entity/connections.rs",
    "legacy/map_engine/src/data/store/operations/entity/factions.rs",
    "legacy/map_engine/src/data/store/operations/entity/identity.rs",
    "legacy/map_engine/src/data/store/operations/entity/markers.rs",
    "legacy/map_engine/src/data/store/operations/entity/mod.rs",
    "legacy/map_engine/src/data/store/operations/entity/placement.rs",
    "legacy/map_engine/src/data/store/operations/entity/roster.rs",
    "legacy/map_engine/src/data/store/operations/entity/selection.rs",
    "legacy/map_engine/src/data/store/operations/entity/vehicles.rs",
    "legacy/map_engine/src/data/store/operations/entity/zones.rs",
    "legacy/map_engine/src/data/store/operations/environment.rs",
    "legacy/map_engine/src/data/store/operations/faction_library.rs",
    "legacy/map_engine/src/data/store/operations/mod.rs",
    "legacy/map_engine/src/data/store/operations/place_orbat/mod.rs",
    "legacy/map_engine/src/data/store/operations/place_orbat/placement.rs",
    "legacy/map_engine/src/data/store/operations/placement/alignment.rs",
    "legacy/map_engine/src/data/store/operations/placement/garrison.rs",
    "legacy/map_engine/src/data/store/operations/placement/geometry.rs",
    "legacy/map_engine/src/data/store/operations/placement/mod.rs",
    "legacy/map_engine/src/data/store/operations/placement/patterns.rs",
    "legacy/map_engine/src/data/store/operations/projections.rs",
    "legacy/map_engine/src/data/store/operations/reassign.rs",
    "legacy/map_engine/src/data/store/operations/rotation.rs",
    "legacy/map_engine/src/data/store/operations/rows.rs",
    "legacy/map_engine/src/data/store/operations/slot_ids/duplicates.rs",
    "legacy/map_engine/src/data/store/operations/slot_ids/mod.rs",
    "legacy/map_engine/src/data/store/operations/tactical_graphics.rs",
    "legacy/map_engine/src/data/store/operations/transform.rs",
    "legacy/map_engine/src/data/store/operations/zones.rs",
    "legacy/map_engine/src/editing/hosted_commands/composition_library.rs",
    "legacy/map_engine/src/editing/hosted_commands/document_edit.rs",
    "legacy/map_engine/src/editing/hosted_commands/document_search.rs",
    "legacy/map_engine/src/editing/hosted_commands/editor_layers.rs",
    "legacy/map_engine/src/editing/hosted_commands/entity_clipboard.rs",
    "legacy/map_engine/src/editing/hosted_commands/entity_connections.rs",
    "legacy/map_engine/src/editing/hosted_commands/map_comments.rs",
    "legacy/map_engine/src/editing/hosted_commands/map_markers.rs",
    "legacy/map_engine/src/editing/hosted_commands/map_triggers.rs",
    "legacy/map_engine/src/editing/hosted_commands/mod.rs",
    "legacy/map_engine/src/editing/hosted_commands/orbat_roster.rs",
    "legacy/map_engine/src/editing/hosted_commands/placed_vehicles.rs",
    "legacy/map_engine/src/editing/hosted_commands/selection_transform.rs",
    "legacy/map_engine/src/editing/hosted_commands/slot_attributes.rs",
    "legacy/map_engine/src/editing/hosted_commands/slot_loadouts.rs",
    "legacy/map_engine/src/editing/hosted_commands/squad_reassignment.rs",
    "legacy/map_engine/src/editing/hosted_commands/zone_authoring.rs",
];
const ORBAT_RS: &str = "legacy/map_engine/src/data/scenario/ast/factions/orbat_slot_template.rs";
const ORBAT_MGR: &str = "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager.rs";
const EDEN_CHROME: &str = "apps/frontend/src/workspaces/editor/session/eden_chrome.rs";
const SLOTS_GPU: &str = "crates/map_overlay/unit_symbology/src/classification.rs";

/// Every UI source that can render the banned ORBAT copy, plus the editor shell.
const ORBAT_UI_BAN_TARGETS: &[&str] = &[
    ORBAT_MGR,
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/dialog.rs",
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/dialog_lifecycle.rs",
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/faction_templates.rs",
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/slot_inspector.rs",
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/snapshot.rs",
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/stats.rs",
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/tree_panel.rs",
    "apps/frontend/src/workspaces/editor/ui/modals/orbat_manager/tree_rows.rs",
    EDEN_CHROME,
];

/// One `ban`: message, ERE pattern, `-i`?, targets, and the `ok` line printed when it holds.
#[rustfmt::skip]
type BanRow = (&'static str, &'static str, bool, &'static [&'static str], &'static str);

#[rustfmt::skip]
const BANS: &[BanRow] = &[
    ("ensure_default_squad still present on the place path",
     "ensure_default_squad", false,
     EDITOR_OPS_SPLIT,
     "no ensure_default_squad on place path"),
    ("orbat.rs still hardcodes loadout: String::new()",
     r"loadout: String::new\(\)", false, &[ORBAT_RS],
     "no loadout String::new() hardcode in derive"),
    ("Standardization / IFAK / Grenade Complement UI strings found (L8 omit)",
     "standardization|IFAK|Grenade Complement", true, ORBAT_UI_BAN_TARGETS,
     "no Standardization UI strings"),
];

/// The three side-tint pins, all in [`SLOTS_GPU`], all sharing one `ok` line. The RGBA triples
/// are the Class-R lock — BLUFOR/OPFOR/INDFOR must stay three visually distinct colours — and the
/// literal spacing is part of the pin, so reformatting the array is a change the gate should see.
#[rustfmt::skip]
const PINS: &[(&str, &str)] = &[
    ("SIDE_BLUFOR_RGBA pin missing", r"SIDE_BLUFOR_RGBA: \[u8; 4\] = \[173, 198, 255, 255\]"),
    ("SIDE_OPFOR_RGBA pin missing",  r"SIDE_OPFOR_RGBA: \[u8; 4\] = \[248, 113, 113, 255\]"),
    ("SIDE_INDFOR_RGBA pin missing", r"SIDE_INDFOR_RGBA: \[u8; 4\] = \[34, 197, 94, 255\]"),
];

const FE: &str = "frontend";
/// The map engine, pinned at its mission-authoring feature tier.
///
/// One argv element, not two, so the failure text reads `--features scenario store`. Rendered by
/// `shown`.
const MC: &str = "map_engine";
/// The mission-authoring feature tier.
const MSN: Option<&str> = Some("scenario store");
/// The map overlay crates that draw the ORBAT: the lane order (`map_draw_lanes`), the side tints
/// and squad links (`unit_symbology`) and the slot and vehicle instances (`overlay_instances`).
/// They have no features, so their pins pass `NOF`.
const LANES: &str = "map_draw_lanes";
const SYMBOLOGY: &str = "unit_symbology";
const INSTANCES: &str = "overlay_instances";
const NOF: Option<&str> = None;

/// One `cargo_test_pin`: package, `--features` value, `--lib`?, selector, and the `ok` line to
/// print after it — `Some` only on the row that closes a section.
#[rustfmt::skip]
type PinRow = (&'static str, Option<&'static str>, bool, &'static str, Option<&'static str>);

#[rustfmt::skip]
const CARGO_PINS: &[PinRow] = &[
    // A / B / H — the mission-authoring tier `scenario store`; module docs §2.
    (MC, MSN, true, "place_", None),
    (MC, MSN, true, "set_leader_exclusive", None),
    (MC, MSN, true, "empty_squad_garbage_collected", None),
    (MC, MSN, true, "move_slot_bidirectional", None),
    (MC, MSN, true, "leader_invariant_holds", None),
    (MC, MSN, true, "attach_vehicle_roundtrip", None),
    (MC, MSN, true, "apply_faction_", Some("store-feature place/mutator/apply gates")),
    // C / D / G / vehicle pack.
    (INSTANCES, NOF, true, "side_tint_three_distinct", None),
    (SYMBOLOGY, NOF, true, "squad_link_", None),
    (MC, MSN, true, "format_slot_line", None),
    (INSTANCES, NOF, true, "pack_vehicle_instances", None),
    (LANES, NOF, true, "mission_vehicles", Some("tint / links / slot_line / vehicles lane")),
    // I — scenario feature derive / compile.
    (MC, MSN, true, "derive_fills_loadout", None),
    (MC, MSN, true, "derive_empty_loadout", None),
    (MC, MSN, true, "derives_from_editor_sorted", None),
    (MC, MSN, true, "compile_export_orbat_loadout", Some("derive/compile loadout gates")),
    // ── THE COMPILE BOUNDARY. Read this before trimming the list above. ─────────────────────
    // Every selector up to here proves the editor can AUTHOR an ORBAT value
    // (data::store::operations::place_orbat, data::store::rows), that the map can DRAW it
    // (the map overlay crates), or that the ORBAT derive keeps it (data::scenario::ast::factions,
    // data::scenario::compiler). None of them crosses the edge where the document is handed to
    // the game server: without these two rows, a payload authoring a squad's leaderSlotId, a
    // slot's tag / callsign / rank / stance and the whole vehicle roster could compile to a
    // document carrying none of them while this gate prints ALL PASS. A gate is worth nothing
    // until you know what it looked at. These two rows are that edge — the
    // ledger walks each value from the saved payload to the serialized wire against
    // mission.schema.json, so a widened contract turns the newly-legal key's row red and the dead
    // feature becomes visible work; the second pins the compiled slot's key set, so nothing is
    // added to or removed from the website<->mod interface in silence.
    (MC, MSN, true, "the_compile_boundary_ledger_is_checked_against_the_contract", None),
    (MC, MSN, true, "a_compiled_slot_carries_exactly_these_keys", None),
    // The vehicle-floor test lives behind #[cfg(feature = "store")] (the MissionDocCore writer
    // round-trip in flatten.rs), so a scenario-only feature set matches 0 tests and this pin
    // FAILs. Aligned with the place_/attach_vehicle pins above rather than weakened.
    (MC, MSN, true, "the_vehicle_row_still_has_the_shape_this_module_reads",
        Some("compile-boundary ledger + compiled-slot key set + vehicle contract floor")),
    // E / F / G / H / I — FE. A bin crate, so no `--lib`: its tests live in src/main.rs.
    (FE, NOF, false, "eden_side", None),
    (FE, NOF, false, "apply_eden", None),
    (FE, NOF, false, "objects_chip", None),
    (FE, NOF, false, "open_arsenal", None),
    (FE, NOF, false, "g1_dialog", None),
    (FE, NOF, false, "orbat_", Some("frontend Eden/ORBAT gates")),
];

// ── Static bans and pins ─────────────────────────────────────────────────────────────────────

/// Which "target file missing" sentence a check prints. `ban` and `require` word it differently
/// and both wordings are scraped by operators, so the difference is preserved.
enum Kind {
    Ban,
    Pin,
}

/// The `ban` continuation, on one line: the printed message never wraps.
const BAN_MISSING: &str =
    "The ban could not run, and a moved or deleted file must not read as a clean result.";

// ── cargo_test_pin ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/editor_orbat_coherency/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_editor_orbat_coherency;

#[cfg(test)]
use source_audit::{classify, not_run_clause, passed_counts, shown, static_checks};
