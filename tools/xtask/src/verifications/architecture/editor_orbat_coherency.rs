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
//! ── THE MISSION-AUTHORING ROWS ───────────────────────────────────────────────────────────────
//!
//! The authoring rows run in the mission crates, which have no features: the character placement
//! and the faction library apply in `mission_operations`, the roster mutators, the leader
//! invariant and the vehicle contract floor in `mission_document`. The ORBAT derive, the slot line,
//! the export payload's ORBAT and the compile boundary run in the `mission_model`,
//! `mission_payload` and `mission_compiler` crates.
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
// The place path spans three crates: the document mutations in `mission_operations`, the map
// engine's hosted commands that drive them, and the host half in the frontend that arms a
// placement and commits it. Both sides are scanned together, and the scratch
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
    "crates/mission/mission_operations/src/apply_faction/apply.rs",
    "crates/mission/mission_operations/src/apply_faction/authorship.rs",
    "crates/mission/mission_operations/src/apply_faction/library.rs",
    "crates/mission/mission_operations/src/apply_faction/mod.rs",
    "crates/mission/mission_operations/src/assets.rs",
    "crates/mission/mission_operations/src/attrs.rs",
    "crates/mission/mission_operations/src/cargo.rs",
    "crates/mission/mission_operations/src/cargo_rules.rs",
    "crates/mission/mission_operations/src/compositions.rs",
    "crates/mission/mission_operations/src/document_index.rs",
    "crates/mission/mission_operations/src/entity/clipboard.rs",
    "crates/mission/mission_operations/src/entity/comments.rs",
    "crates/mission/mission_operations/src/entity/connections.rs",
    "crates/mission/mission_operations/src/entity/factions.rs",
    "crates/mission/mission_operations/src/entity/identity.rs",
    "crates/mission/mission_operations/src/entity/markers.rs",
    "crates/mission/mission_operations/src/entity/mod.rs",
    "crates/mission/mission_operations/src/entity/placement.rs",
    "crates/mission/mission_operations/src/entity/roster.rs",
    "crates/mission/mission_operations/src/entity/selection.rs",
    "crates/mission/mission_operations/src/entity/vehicles.rs",
    "crates/mission/mission_operations/src/entity/zones.rs",
    "crates/mission/mission_operations/src/environment.rs",
    "crates/mission/mission_operations/src/faction_library.rs",
    "crates/mission/mission_operations/src/lib.rs",
    "crates/mission/mission_operations/src/place_orbat/mod.rs",
    "crates/mission/mission_operations/src/place_orbat/placement.rs",
    "crates/mission/formation_geometry/src/alignment.rs",
    "crates/mission/formation_geometry/src/garrison.rs",
    "crates/mission/formation_geometry/src/geometry.rs",
    "crates/mission/formation_geometry/src/lib.rs",
    "crates/mission/formation_geometry/src/patterns.rs",
    "crates/mission/mission_operations/src/projections.rs",
    "crates/mission/mission_operations/src/reassign.rs",
    "crates/mission/mission_operations/src/rotation.rs",
    "crates/mission/mission_operations/src/rows.rs",
    "crates/mission/mission_operations/src/slot_ids/duplicates.rs",
    "crates/mission/mission_operations/src/slot_ids/mod.rs",
    "crates/mission/mission_operations/src/tactical_graphics.rs",
    "crates/mission/mission_operations/src/transform.rs",
    "crates/mission/mission_operations/src/zones.rs",
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
const ORBAT_RS: &str = "crates/mission/mission_model/src/orbat/orbat_slot_template.rs";
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
/// The mission model crate: the ORBAT derive and the slot line, no features.
const MM: &str = "mission_model";
/// The mission payload crate: the export payload's ORBAT, no features.
const MP: &str = "mission_payload";
/// The game-document compiler crate: the compile boundary ledger and the compiled slot's keys, no
/// features.
const MCP: &str = "mission_compiler";
/// The mission document crate: the roster mutators, the leader invariant and the vehicle writer
/// round trip, no features.
const MD: &str = "mission_document";
/// The mission operations crate: character placement under a side and the faction library apply,
/// no features.
const MO: &str = "mission_operations";
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
    // A / B / H — the mission-authoring rows; module docs §2.
    (MO, NOF, true, "place_", None),
    (MD, NOF, true, "place_", None),
    (MD, NOF, true, "set_leader_exclusive", None),
    (MD, NOF, true, "empty_squad_garbage_collected", None),
    (MD, NOF, true, "move_slot_bidirectional", None),
    (MD, NOF, true, "leader_invariant_holds", None),
    (MD, NOF, true, "attach_vehicle_roundtrip", None),
    (MO, NOF, true, "apply_faction_", Some("mission authoring place/mutator/apply gates")),
    // C / D / G / vehicle pack.
    (INSTANCES, NOF, true, "side_tint_three_distinct", None),
    (SYMBOLOGY, NOF, true, "squad_link_", None),
    (MM, NOF, true, "format_slot_line", None),
    (INSTANCES, NOF, true, "pack_vehicle_instances", None),
    (LANES, NOF, true, "mission_vehicles", Some("tint / links / slot_line / vehicles lane")),
    // I — the mission model's ORBAT derive and the payload compiler's export.
    (MM, NOF, true, "derive_fills_loadout", None),
    (MM, NOF, true, "derive_empty_loadout", None),
    (MM, NOF, true, "derives_from_editor_sorted", None),
    (MP, NOF, true, "compile_export_orbat_loadout", Some("derive/compile loadout gates")),
    // ── THE COMPILE BOUNDARY. Read this before trimming the list above. ─────────────────────
    // Every selector up to here proves the editor can AUTHOR an ORBAT value
    // (mission_operations::place_orbat, mission_document), that the map can DRAW it
    // (the map overlay crates), or that the ORBAT derive keeps it (mission_model::orbat,
    // mission_payload). None of them crosses the edge where the document is handed to
    // the game server: without these two rows, a payload authoring a squad's leaderSlotId, a
    // slot's tag / callsign / rank / stance and the whole vehicle roster could compile to a
    // document carrying none of them while this gate prints ALL PASS. A gate is worth nothing
    // until you know what it looked at. These two rows are that edge — the
    // ledger walks each value from the saved payload to the serialized wire against
    // mission.schema.json, so a widened contract turns the newly-legal key's row red and the dead
    // feature becomes visible work; the second pins the compiled slot's key set, so nothing is
    // added to or removed from the website<->mod interface in silence.
    (MCP, NOF, true, "the_compile_boundary_ledger_is_checked_against_the_contract", None),
    (MCP, NOF, true, "a_compiled_slot_carries_exactly_these_keys", None),
    // The vehicle-floor test is a mission document test (the MissionDocCore writer round trip in
    // crates/mission/mission_document/src/tests/vehicle_row_round_trips.rs). Aligned with the
    // place_/attach_vehicle pins above.
    (MD, NOF, true, "the_vehicle_row_still_has_the_shape_this_module_reads",
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
