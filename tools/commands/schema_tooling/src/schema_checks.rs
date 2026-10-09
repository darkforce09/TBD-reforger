//! The text and JSON schema gates: map-object enums, type inventory, terrain
//! manifest, and ORBAT slot flattening. Each gate's acceptance contract is its verdict set plus
//! its exit code; stdout formatting carries no contract.
//!
//! Two gate slots are retired and print that they are, so the missing surface stays visible
//! rather than looking like a silent pass:
//! - TS-6 front-end export tags — the front end's contract layer is Rust (its DTO modules) gated by
//!   R-api golden tests, so there is no separate export-tag surface to match.
//! - GO-7 @route match — axum wires routes through typed functions, so a route rename is a
//!   compile error rather than documentation rot.
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt};
use serde_json::Value;

// The census kinds the I1 sum gate adds up: the prefab catalogue's one list,
// `map-object-enums.schema.json` `$defs.kind` minus `$defs.regionKind`, which is exactly
// `byKind`'s property set, in `byKind`'s emitted key order.
//
// A kind missing from this list is not cosmetic: I1 sums ONLY the kinds named here, so an
// inventory carrying `byKind.vehicle.instances = 176` for an absent `vehicle` row comes up short
// by exactly 176, and the gate reads as a data fault in the artifact rather than as a hole in
// the gate itself.
//
// The invariant is pinned two ways, deliberately:
//   * at RUNTIME inside `type_inventory()` (see `instance_kinds_lockstep_failures`) — the
//     load-bearing one, because `xtask schema type-inventory` runs in every slice gate and every
//     wave gate via `gate_schema`;
//   * by `#[cfg(test)] instance_kind_lockstep_tests` for local `cargo test -p schema_tooling`
//     feedback.
//
// A `#[test]` alone is decorative: it proves the list it can see, not the schema the gate reads.
use prefab_catalog::instance_kinds::INSTANCE_KINDS;

use repository_layout::{contract_catalogs_dir, definition_path, registry_fixtures_dir};

use repository_root::find_repository_root as repo_root;

/// Folder names the mod-tree scans skip: dependency, build and version-control output.
const IGNORE_DIRS: [&str; 6] = [
    "node_modules",
    "dist",
    ".git",
    "build",
    "coverage",
    "vendor",
];

/* ─────────────────────────── map-object enums ─────────────────────────── */

/* ─────────────────────────── type inventory (I1–I7) ─────────────────────────── */

/// The developer-feedback half of the `INSTANCE_KINDS` pin. The gate-enforced half is the
/// `instance_kinds_lockstep_failures` call inside `type_inventory()`; both call the same function,
/// so neither can drift from the other's idea of the invariant.
#[cfg(test)]
#[path = "tests/schema_checks/instance_kind_lockstep_tests.rs"]
mod instance_kind_lockstep_tests;

/* ─────────────────────────── flatten-orbat-slots ─────────────────────────── */

/* ────────────────── kit alias ↔ spawn registry cross-reference ────────────────── */

// WHY THIS IS A GATE AND NOT A SCHEMA ENUM
// ---------------------------------------
// `mission.schema.json` types a slot kit as `^kit:[a-z0-9_]+$`. That checks the SHAPE. Whether the
// alias exists is a registry question, and the registry (`mod/tbd-framework/Data/registry.json`)
// is generated/extensible content — a closed enum in the contract would have to be re-cut every time
// a kit is added, would go stale silently, and would reject valid missions authored against a newer
// registry. So the vocabulary check belongs HERE: same corpus, build time, reading the very file the
// game server resolves against.
//
// Cost of not having it, measured: a golden referencing `kit:us_medic`, which no registry entry
// defines, passes `cargo xtask ci schema-validate` and is then rejected by a real server boot,
// which parks the server in LOADING. TBD_MissionValidator.CheckSlotKit makes this exact
// comparison at runtime; this is the same check where it is cheap.

/// Kit aliases a committed golden references that the spawn registry provably cannot resolve.
///
/// FAIL-CLOSED with a documented escape — the same discipline `cargo xtask mod world-boot` uses
/// for vanilla script noise. Anything not listed here fails, so a NEW dangling alias is a
/// regression. Add a row
/// only with a reason saying why the registry cannot legitimately gain the entry; "it is broken
/// today" is not a reason, it is a bug to fix.
const KNOWN_UNRESOLVABLE_KITS: &[(&str, &str)] = &[
    // `last-stand-at-montfort` is a British-army scenario and Arma Reforger vanilla ships no UK
    // faction at all, so these cannot be added to the vanilla registry honestly — they need a
    // content modset this repo does not have. They are ORBAT-template-only (that document is
    // schemaVersion 1.0 with no `slots[]`), so TBD_MissionValidator never resolves them today; they
    // go live the moment the ORBAT is flattened to slots, which is why they are listed rather than
    // quietly skipped.
    ("last-stand-at-montfort.json", "kit:uk_sl"),
    ("last-stand-at-montfort.json", "kit:uk_rifleman"),
    ("last-stand-at-montfort.json", "kit:uk_gpmg"),
    ("last-stand-at-montfort.json", "kit:uk_at"),
];

/* ────────── schemaVersion 1.3 wire fields must stay UNREAD until their reader lands ────────── */

// WHY THIS GATE EXISTS
// --------------------
// `mission.schema.json` carries a set of schemaVersion 1.3 fields that are on the WIRE and READ BY
// NOTHING: their readers are mod-side and land one at a time. A wire field ahead of its consumer is
// a legitimate contract — the schema is the shared definition the mod, the API and the editor each
// build against — but it is a definition, not a capability, and the schema descriptions say so per
// field.
//
// The failure mode this gate prevents is that description going stale: a reader lands while the
// schema still says "on the wire only — no reader on any shipped build". So this asserts, PER
// FIELD, that the mod tree holds exactly its baseline number of readers. The day a real reader
// lands, that field's count rises above its baseline, THIS GATE FAILS, and whoever landed the
// reader must come here, drop the field's "no reader" wording, and move its row out of the table.
//
// WHY A COMMENT/STRING-STRIPPED WHOLE-WORD COUNT, AND WHY A PER-FIELD BASELINE
// ---------------------------------------------------------------------------
// Enfusion's `JsonLoadContext` binds a JSON key onto a class MEMBER OF THE SAME NAME (the mod's own
// structs say so: "Field names must equal the JSON keys — JsonLoadContext maps by name"). So a
// reader of wire key `combatMode` manifests as the identifier `combatMode` in a `.c` file. A raw
// grep would false-fire on the word inside a `//` doc-comment or a `""` string literal, so both are
// stripped before counting (MEASURED: without stripping, `wind`/`size`/`behaviour` already "read"
// via prose). Whole-word (`\b…\b`) so `map` does not match `heatmap`.
//
// STATED LIMIT: because string bodies are stripped, a reader that fetches a wire key BY STRING —
// `ctx.ReadValue("combatMode", …)` or a runtime-built key — is invisible to this gate by
// construction. That is accepted, not overlooked: mod practice is exclusively member-name binding
// (`ReadValue("", struct)`), which IS what the identifier count sees; a string-keyed reader would
// be a house-style break its own review catches.
//
// Most fields strip to 0 — no identifier of that spelling exists in the mod. SEVEN rows collide with
// a GENUINELY UNRELATED identifier already in the tree, and those seven — and ONLY those seven — are
// the pinned non-zero baselines in the table below: `objectives`=13 (`TBD_ObjectivesComponent`'s own
// win-condition objective list); `callsign`=16 (the EXISTING group callsign, a different wire key
// from the new slot.callsign); `seats`=8 (briefing/lobby seat-count UI); `shape`=32 (the existing
// zone-shape reader `TBD_MissionShapeStruct`, circle/polygon geometry — a different field from the
// new marker.shape glyph selector); `area`=13 (loadout `LoadoutAreaType`, worn-garment); `gadgets`=6
// (the radio/gadget subsystem's own vocabulary); `tag`=42 (DOMINATED by UI list-row `int tag`
// numbering — LobbyScreen/ListBox/AdminScreen/ListBoxRow ≈ 32 of the 42). For those seven the
// baseline is the MEASURED unrelated count and the row says WHY it is not a reader of the NEW
// field.
//
// A NEW FIELD WHOSE INTERIOR IS WRAPPER-COVERED GETS NO ROW, and this is deliberate — do NOT read
// the seven collision rows as "every colliding word". `slot.gadgets.map`/`.radio`, `marker.area`'s
// `circle`/`polygon`/`rectangle`/`ellipse` extents, `activation`'s interior, etc. are reached only by
// a reader that FIRST binds the parent member (`gadgets`/`area`/`activation`, each of which HAS a
// row), so binding the wrapper covers the interior — JsonLoadContext transitivity. `map` and
// `radio` therefore have NO row of their own: they are interiors, not pinned collisions.
//
// The assertion is `== baseline`: an unrelated refactor that changes a collision count is a
// deliberate, visible re-pin here (rare), while the event this gate is FOR — a new reader of the new
// field — is always a +1 that trips it. Fail-closed with a documented table, the same discipline
// `KNOWN_UNRESOLVABLE_KITS` uses above.

/* ─────────────────────────── validate (the document validation core) ─────────────────────────── */

/* ─────────────────────────── map glyphs manifest (GL-G1…G6) ─────────────────────────── */

/// Execute the actual side-validation and framing branches with a small native shim. This is
/// source simulation, not Enfusion JSON loading, engine execution or a two-client RPC proof.
/// C++ preserves these methods' control flow; only static-member punctuation is translated.
#[cfg(test)]
#[path = "tests/schema_checks/side_fallback_tests.rs"]
mod side_fallback_tests;

mod registry_validation;

mod mission_validation;

mod ballistics_validation;

mod object_enumerations;
pub use object_enumerations::map_object_enums;

mod object_type_inventory;
pub use object_type_inventory::type_inventory;

mod kit_registry_references;
use kit_registry_references::dangling_kits;
use kit_registry_references::mission_preset_refs;
use kit_registry_references::spawn_registry_aliases;

mod wire_field_readers;
use wire_field_readers::unread_wire_field_failures;

mod contract_validation;
pub use contract_validation::validate_all;
pub use contract_validation::validate_file;

mod map_glyphs;
pub use map_glyphs::map_glyphs;

mod read_json;
use read_json::read_json;
use read_json::schema_root;
use read_json::verdict;

use wire_field_readers::UNREAD_WIRE_FIELDS;

#[cfg(test)]
use object_type_inventory::instance_kinds_lockstep_failures;

#[cfg(test)]
use wire_field_readers::strip_enfusion_comments_and_strings;
