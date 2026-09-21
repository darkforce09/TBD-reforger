//! The text and JSON schema gates: contract citations, specification consistency, sentence and
//! tile-budget limits, map-object enums, type inventory, terrain manifest, and ORBAT slot
//! flattening. Each gate's acceptance contract is its verdict set plus its exit code; stdout
//! formatting carries no contract.
//!
//! Two gate slots are retired and print that they are, so the missing surface stays visible
//! rather than looking like a silent pass:
//! - TS-6 front-end export tags — the front end's contract layer is Rust (its DTO modules) gated by
//!   R-api golden tests, so there is no separate export-tag surface to match.
//! - GO-7 @route match — axum wires routes through typed functions, so a route rename is a
//!   compile error rather than documentation rot.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use developer_tools::repository_layout::{
    contract_catalogs_dir, contract_definitions_dir, definition_path, registry_fixtures_dir,
};

use crate::core::repository_root::find_repo_root as repo_root;

/* ─────────────────────────── citations ─────────────────────────── */

/// Extensions scanned for `@contract` tags.
///
/// `rs` and `c` carry every citation the gate reads. `go`, `js`, `mjs`, `ts` and `tsx` match
/// nothing in this tree and are kept anyway: an extension that matches nothing cannot cause a
/// false green — only a *missing* one can, and a missing one lets a whole language's citations
/// go unread while the gate still prints "All @contract citations resolve". Their zeros in the
/// per-extension breakdown are the visible evidence that the tree holds no Go or Node sources.
const CODE_EXTS: [&str; 7] = ["c", "go", "js", "mjs", "rs", "ts", "tsx"];
/// Code roots whose contract citations must resolve: the applications and the tooling tree.
/// Markdown is excluded because prose examples are not code contract declarations, and the
/// contract and asset trees are excluded because they hold data, not code that declares a citation.
const SCAN_ROOTS: [&str; 2] = ["apps", "tools_v2"];
const IGNORE_DIRS: [&str; 6] = [
    "node_modules",
    "dist",
    ".git",
    "build",
    "coverage",
    "vendor",
];

/// One pass over the `@contract` corpus.
///
/// `problems` are dangling citations; `scope_errors` are reasons the scan itself cannot be
/// trusted (a root that was never read, an empty corpus). They are separate because "0
/// problems over 0 files" is not a pass — it is the absence of a verdict.
#[derive(Debug, Default)]
struct CitationScan {
    citations: usize,
    files_read: usize,
    per_ext: BTreeMap<&'static str, usize>,
    problems: Vec<String>,
    scope_errors: Vec<String>,
}

/// The scope contract, pinned against a fixture tree.
///
/// The failure this guards is a broad claim over a narrow scan, not a bad count. These tests
/// fail if `rs` leaves [`CODE_EXTS`], if `tools_v2/` leaves [`SCAN_ROOTS`], or if the walker
/// ever reports a clean verdict over a tree it did not read.
#[cfg(test)]
#[path = "../../tests/citation_scope_tests.rs"]
mod citation_scope_tests;

/* ─────────────────────────── n6 / n10 ─────────────────────────── */

/* ─────────────────────────── map-object enums ─────────────────────────── */

/* ─────────────────────────── type inventory (I1–I7) ─────────────────────────── */

/// The census kinds the I1 sum gate adds up — `map-object-enums.schema.json` `$defs.kind` minus
/// `$defs.regionKind`, which is exactly `byKind`'s property set.
///
/// A kind missing from this array is not cosmetic: I1 sums ONLY the kinds named here, so an
/// inventory carrying `byKind.vehicle.instances = 176` for an absent `vehicle` row comes up short
/// by exactly 176, and the gate reads as a data fault in the artifact rather than as a hole in
/// the gate itself.
///
/// The invariant is pinned two ways, deliberately:
///   * at RUNTIME inside `type_inventory()` (see `instance_kinds_lockstep_failures`) — the
///     load-bearing one, because `xtask schema type-inventory` runs in every slice gate and every
///     wave gate via `gate_schema`;
///   * by `#[cfg(test)] instance_kind_lockstep_tests` for local `cargo test -p xtask` feedback.
///
/// A `#[test]` alone is decorative: it proves the copy it can see, not the copy the gate reads.
///
/// Order is `byKind`'s emitted key order (serde_json is built with `preserve_order`): `vehicle`
/// goes after `water`, `road` stays last, matching its twin
/// `developer_tools::world_export_pipeline::INSTANCE_KINDS` and every committed inventory.
const INSTANCE_KINDS: [&str; 9] = [
    "building",
    "tree",
    "vegetation",
    "rock",
    "prop",
    "utility",
    "water",
    "vehicle",
    "road",
];

/// The developer-feedback half of the `INSTANCE_KINDS` pin. The gate-enforced half is the
/// `instance_kinds_lockstep_failures` call inside `type_inventory()`; both call the same function,
/// so neither can drift from the other's idea of the invariant.
#[cfg(test)]
#[path = "tests/checks/instance_kind_lockstep_tests.rs"]
mod instance_kind_lockstep_tests;

/* ─────────────────────────── specification consistency (12 gates) ─────────────────────────── */

/// The frozen set of `make <target>` names the specification corpus may still cite.
///
/// No Makefile exists, so every `make` name in a specification is an instruction nobody can run.
/// These four are archival citations inside otherwise-live specifications and are tolerated
/// rather than rewritten. The list may only SHRINK: anything not on it fails the gate.
const ARCHIVAL_MAKE_TARGETS: &[&str] = &[
    "map-assets-link",
    "verify-wgpu-gpu",
    "ci-local-frontend",
    "verify-migration",
];

/* ─────────────────────────── flatten-orbat-slots ─────────────────────────── */

/* ────────────────── kit alias ↔ spawn registry cross-reference ────────────────── */

// WHY THIS IS A GATE AND NOT A SCHEMA ENUM
// ---------------------------------------
// `mission.schema.json` types a slot kit as `^kit:[a-z0-9_]+$`. That checks the SHAPE. Whether the
// alias exists is a registry question, and the registry (`apps/mod/tbd-framework/Data/registry.json`)
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
// `unread_gate_fires_when_a_reader_appears` proves the mechanism fires rather than passing
// vacuously.
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

/// The developer-feedback half of the unread-fields gate. The load-bearing half is the
/// `unread_wire_field_failures` call inside `validate_all()` (run in every slice + wave gate via
/// `gate_schema`); this proves the mechanism actually FIRES so the assertion is not decorative —
/// the same non-vacuity discipline the INSTANCE_KINDS lockstep tests use.
#[cfg(test)]
#[path = "tests/checks/unread_wire_field_tests.rs"]
mod unread_wire_field_tests;

/* ─────────────────────────── validate (the document validation core) ─────────────────────────── */

/* ─────────────────────────── map glyphs manifest (GL-G1…G6) ─────────────────────────── */

/// The typed objective spine must actually be READ by the objectives lane.
///
/// `#/$defs/objective` (plus `#/$defs/objectiveFraming`) is the uniform attribute spine every
/// objective carries. Enfusion's `JsonLoadContext` binds JSON keys onto identically-named class
/// MEMBERS, so for this wire shape the identifier IS the contract: a property with no identifier
/// under `Scripts/Game/TBD/Gamemode/Objectives` cannot be read by any objective code, whatever the rest of
/// the mod happens to spell somewhere else.
///
/// Scoped to that ONE lane deliberately. A whole-tree count passes vacuously on the common words:
/// measured on `main` before this slice, the tree held `id` 206, `label` 111, `text` 91, `side` 59
/// — every one of them an unrelated subsystem's identifier. In the Objectives lane the same scan
/// reads `side` 0, `label` 0, `framing` 0, `lock` 0, `autoLose` 0, `variantId` 0 whenever a reader
/// goes missing: six of the eleven spine properties with no reader at all is the defect this pins.
///
/// This is the complement of `UNREAD_WIRE_FIELDS`, not a duplicate of it. That table pins fields
/// that must stay unread; this one pins a field set that must stay READ, so a later refactor that
/// deletes the reader is a red rather than a silent regression back to a dead container.
#[cfg(test)]
#[path = "tests/checks/objective_spine_tests.rs"]
mod objective_spine_tests;

/// The hand-staged 1.3 golden actually REACHES the reader.
///
/// `flatten.rs` emits no `objectives[]` on `/compiled`, so the only document that can reach
/// `TBD_ObjectiveEntityReader` is a hand-staged schemaVersion 1.3 one, and
/// `golden-missions/schema-1_3-wire-fields.json` is that document. It makes "proven" mechanical
/// for a wire field that has no live emitter.
///
/// `JsonLoadContext` binds JSON keys onto identically-named class MEMBERS, so "the golden reaches
/// the reader" is exactly the claim "every key the golden's objectives rows author is declared as a
/// member of the reader's structs". A key the structs do not declare is invisible at runtime — not
/// rejected, not logged, simply absent — so it is asserted rather than eyeballed.
///
/// What it cannot prove is stated rather than implied: the gate for the `.c` half is
/// `cargo xtask mod compile`, which cannot run a round. Whether the attacker and the defender are
/// actually shown different text with two clients connected is a human checklist item.
#[cfg(test)]
#[path = "tests/checks/staged_golden_tests.rs"]
mod staged_golden_tests;

/// Execute the actual side-validation and framing branches with a small native shim. This is
/// source simulation, not Enfusion JSON loading, engine execution or a two-client RPC proof.
/// C++ preserves these methods' control flow; only static-member punctuation is translated.
#[cfg(test)]
#[path = "tests/checks/side_fallback_tests.rs"]
mod side_fallback_tests;

mod registry_validation;

mod mission_validation;

mod contract_citations;
pub use contract_citations::citations;

mod content_budgets;
pub use content_budgets::n6_sentence;
pub use content_budgets::n10_tile_budget;

mod object_enumerations;
pub use object_enumerations::map_object_enums;

mod object_type_inventory;
pub use object_type_inventory::type_inventory;

mod specification_consistency;
pub use specification_consistency::specification_consistency;

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
use read_json::spec_dir;
use read_json::verdict;

use wire_field_readers::UNREAD_WIRE_FIELDS;

#[cfg(test)]
use object_type_inventory::instance_kinds_lockstep_failures;

#[cfg(test)]
use contract_citations::{citation_scope, scan_citations};

#[cfg(test)]
use wire_field_readers::{count_mod_readers, strip_enfusion_comments_and_strings};
