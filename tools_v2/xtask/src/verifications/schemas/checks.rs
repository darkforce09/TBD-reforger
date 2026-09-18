//! T-165.1 — the text/JSON schema gates, ported from `contracts_v2/scripts/*.mjs`
//! (verify-contract-citations, verify-t090-spec-consistency, verify-n6-sentence,
//! verify-n10-tile-budget, verify-map-object-enums, verify-type-inventory,
//! verify-terrain-manifest, flatten-orbat-slots). Behavior parity with the Node originals:
//! same gate semantics, same OK/FAIL verdict lines, same exit codes; stdout formatting is
//! near-identical but the acceptance contract is verdict-set + exit code (T-165 plan).
//!
//! Retirements carried over from the Node era (printed, so the surface change is visible):
//! - TS-6 front-end export tags — the React contract layer was deleted at T-159.29.3; the
//!   Leptos contract layer is Rust (`dto.rs`) gated by R-api golden tests.
//! - GO-7 @route match — the Go handlers were retired at the T-145 Rust cutover; axum wires
//!   routes through typed fns, so a rename is a compile error, not doc rot.
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
/// T-611: `rs` was missing until now. The repo went **Go → Rust at T-145** and
/// **React → Leptos at T-159.29.3**, so every citation this gate could see lived in a `.c`
/// file while 18 tags in Rust were never read — and the gate still printed
/// "All @contract citations resolve". The dead pre-rewrite extensions are kept because an
/// extension that matches nothing cannot cause a false green (only a *missing* one can), and
/// the per-extension breakdown in the summary makes their zeros visible evidence that the
/// Go/Node eradication still holds.
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

/// T-611 — the scope contract, pinned against a fixture tree.
///
/// The defect this gate shipped with was not a bad count; it was a broad claim over a narrow
/// scan. These tests fail if `rs` leaves [`CODE_EXTS`], if `tools_v2/` leaves [`SCAN_ROOTS`], or
/// if the walker ever reports a clean verdict over a tree it did not read.
#[cfg(test)]
#[path = "../../tests/citation_scope_tests.rs"]
mod citation_scope_tests;

/* ─────────────────────────── n6 / n10 ─────────────────────────── */

/* ─────────────────────────── map-object enums ─────────────────────────── */

/* ─────────────────────────── type inventory (I1–I7) ─────────────────────────── */

/// The census kinds the I1 sum gate adds up — `map-object-enums.schema.json` `$defs.kind` minus
/// `$defs.regionKind`, which is exactly `byKind`'s property set.
///
/// T-594. This array was `[&str; 8]` and missing `vehicle` for a month after T-244 added that kind
/// to the enums schema and to `prefab-classify.json`. Nothing compared the two, and the shortfall
/// was not merely cosmetic: I1 sums ONLY the kinds named here, so a regenerated Everon inventory
/// carrying `byKind.vehicle.instances = 176` came up short by exactly 176 and the gate read as a
/// data fault in the artifact rather than as a hole in the gate.
///
/// Its twin `tools_v2/developer-tools/src/world/INSTANCE_KINDS` had already been corrected to nine and
/// pinned by `instance_kinds_match_enums_schema` — but that test lives in `tbd-tools`, which
/// **neither the wave gate nor CI runs** (`cargo test --workspace` is red on clean main, so the
/// gate tests `website-api` / `map-engine-*` / `website-frontend` only). So the guarded copy was
/// the one that did not feed the gate, and the copy that fed the gate was unguarded.
///
/// It is pinned two ways now, deliberately:
///   * at RUNTIME inside `type_inventory()` (see `instance_kinds_lockstep_failures`) — that is the
///     load-bearing one, because `xtask schema type-inventory` runs in every slice gate and every
///     wave gate via `gate_schema`;
///   * by `#[cfg(test)] instance_kind_lockstep_tests` for local `cargo test -p xtask` feedback.
///
/// A `#[test]` alone would have been decorative here for the same reason the tbd-tools one was.
///
/// Order is `byKind`'s emitted key order (serde_json is built with `preserve_order`): `vehicle`
/// goes after `water`, `road` stays last, matching the twin and every committed inventory.
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

/// T-594 — the developer-feedback half of the `INSTANCE_KINDS` pin. The gate-enforced half is the
/// `instance_kinds_lockstep_failures` call inside `type_inventory()`; both call the same function,
/// so neither can drift from the other's idea of the invariant.
#[cfg(test)]
#[path = "tests/checks/instance_kind_lockstep_tests.rs"]
mod instance_kind_lockstep_tests;

/* ─────────────────────────── t090 spec consistency (12 gates) ─────────────────────────── */

/// `make <target>` names the T-090 spec corpus may still cite, FROZEN at T-897.
///
/// Every one of these named a target that had already stopped existing when the Makefile was
/// deleted — the React-era `web`/`wasm`/`ci-local-frontend` lane and two one-off verifies. They
/// are archival citations inside otherwise-live specs, so they are tolerated rather than rewritten
/// into a command nobody can run. The list may only SHRINK: anything not on it is a dangling
/// instruction, because there is no Makefile for `make` to read.
const ARCHIVAL_MAKE_TARGETS: &[&str] = &[
    "map-assets-link",
    "verify-wgpu-gpu",
    "ci-local-frontend",
    "verify-migration",
];

/* ─────────────────────────── flatten-orbat-slots ─────────────────────────── */

/* ────────────────── kit alias ↔ spawn registry cross-reference (T-181.34/.36) ────────────────── */

// WHY THIS IS A GATE AND NOT A SCHEMA ENUM
// ---------------------------------------
// `mission.schema.json` types a slot kit as `^kit:[a-z0-9_]+$`. That checks the SHAPE. Whether the
// alias exists is a registry question, and the registry (`apps/mod/tbd-framework/Data/registry.json`)
// is generated/extensible content — a closed enum in the contract would have to be re-cut every time
// a kit is added, would go stale silently, and would reject valid missions authored against a newer
// registry. So the vocabulary check belongs HERE: same corpus, build time, reading the very file the
// game server resolves against.
//
// Cost of not having it, measured: `slot-loadout-coverage.json` referenced `kit:us_medic`, which no
// registry entry defined. `cargo xtask ci schema-validate` passed it. A real server boot rejected the mission
// and parked the server in LOADING (T-181.36). TBD_MissionValidator.CheckSlotKit already does this
// exact comparison at runtime; this is the same check moved to where it is cheap.

/// Kit aliases a committed golden references that the spawn registry provably cannot resolve.
///
/// FAIL-CLOSED with a documented escape — the same discipline `world-boot.sh` uses for vanilla
/// script noise. Anything not listed here fails, so a NEW dangling alias is a regression. Add a row
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

/* ────────── T-706 — schemaVersion 1.3 wire fields must stay UNREAD until their reader lands ────────── */

// WHY THIS GATE EXISTS (the ticket's own non-negotiable acceptance)
// ----------------------------------------------------------------
// T-706 widened `mission.schema.json` ONCE for the whole editor program: sixteen mod-side tickets
// each add a `$def`/property, landed in one pass so the sixteen Enfusion-runtime halves can pack
// freely afterwards. Every one of those fields is on the WIRE today and READ BY NOTHING — the
// readers are mod-side and land under the named ticket. A wire field ahead of its consumer is a
// legitimate contract (the schema is the shared definition the mod/API/editor each build against),
// but it is a definition, not a capability, and the schema descriptions say so per field.
//
// The failure mode this gate prevents is the description going stale: a reader lands under (say)
// T-678, and the schema still says "on the wire only — no reader on any shipped build". So this
// asserts, PER FIELD, that the mod tree has exactly its baseline number of readers. The day a real
// reader lands, the count for that field rises above its baseline, THIS GATE FAILS, and whoever
// landed the reader is forced to come here, drop the field's "no reader" wording, and move its row
// out of the table. That is the mechanism the ticket mandates: "ship a test asserting EACH NEW
// FIELD IS CURRENTLY UNREAD — so the day a reader lands, the test fails and forces its comment to
// be removed." Fired once during authoring (a synthetic reader flips a field baseline→FAIL — see
// `unread_gate_fires_when_a_reader_appears`), then removed.
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
// STATED LIMIT (wave-120 m-3): because string bodies are stripped, a reader that fetches a wire
// key BY STRING — `ctx.ReadValue("combatMode", …)` or a runtime-built key — is invisible to this
// gate by construction. That is accepted, not overlooked: current mod practice is exclusively
// member-name binding (`ReadValue("", struct)`), which IS what the identifier count sees; a
// string-keyed reader landing would be a house-style break its own review should catch.
//
// Most fields strip to 0 — no identifier of that spelling exists in the mod. SEVEN rows collide with
// a GENUINELY UNRELATED identifier already in the tree, and those seven — and ONLY those seven — are
// the pinned non-zero baselines in the table below: `objectives`=13 (`TBD_ObjectivesComponent`'s own
// win-condition objective list); `callsign`=16 (the EXISTING group callsign, a different wire key
// from the new slot.callsign); `seats`=8 (briefing/lobby seat-count UI); `shape`=32 (the existing
// zone-shape reader `TBD_MissionShapeStruct`, circle/polygon geometry — a different field from the
// new marker.shape glyph selector); `area`=13 (loadout `LoadoutAreaType`, worn-garment); `gadgets`=6
// (the radio/gadget subsystem's own vocabulary); `tag`=42 (DOMINATED by UI list-row `int tag`
// numbering — LobbyScreen/ListBox/AdminScreen/ListBoxRow ≈ 32 of the 42 — NOT loadout/spectator as
// once annotated). For those seven the baseline is the MEASURED pre-existing count and the row says
// WHY it is not a reader of the NEW field plus which ticket lands the real one.
//
// A NEW FIELD WHOSE INTERIOR IS WRAPPER-COVERED GETS NO ROW, and this is deliberate — do NOT read
// the seven collision rows as "every colliding word". `slot.gadgets.map`/`.radio`, `marker.area`'s
// `circle`/`polygon`/`rectangle`/`ellipse` extents, `activation`'s interior, etc. are reached only by
// a reader that FIRST binds the parent member (`gadgets`/`area`/`activation`, each of which HAS a
// row), so binding the wrapper covers the interior — the same JsonLoadContext transitivity the T-706
// commit relied on for `map`/`radio`. `map` and `radio` therefore have NO row of their own; they are
// not pinned collisions, they are interiors, and earlier prose that listed them alongside the pinned
// seven conflated the two.
//
// The assertion is `== baseline`: an unrelated refactor that changes a collision count is a
// deliberate, visible re-pin here (rare), while the event this gate is FOR — a new reader of the new
// field — is always a +1 that trips it. Fail-closed with a documented table, the same discipline
// `KNOWN_UNRESOLVABLE_KITS` uses above.

/// T-706 — the developer-feedback half of the unread-fields gate. The load-bearing half is the
/// `unread_wire_field_failures` call inside `validate_all()` (run in every slice + wave gate via
/// `gate_schema`); this proves the mechanism actually FIRES so the assertion is not decorative —
/// the same non-vacuity discipline the INSTANCE_KINDS lockstep tests use.
#[cfg(test)]
#[path = "tests/checks/unread_wire_field_tests.rs"]
mod unread_wire_field_tests;

/* ─────────────────────────── validate (T-165.2 — the validate.mjs core) ─────────────────────────── */

/* ─────────────────────────── map glyphs manifest (GL-G1…G6) ─────────────────────────── */

/// T-212 — the typed objective spine must actually be READ by the objectives lane.
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
/// read `side` 0, `label` 0, `framing` 0, `lock` 0, `autoLose` 0, `variantId` 0: six of the eleven
/// spine properties had no reader at all, which is the defect T-212 closes.
///
/// This is the complement of `UNREAD_WIRE_FIELDS`, not a duplicate of it. That table pins fields
/// that must stay unread; this one pins a field set that must stay READ, so a later refactor that
/// deletes the reader is a red rather than a silent regression back to a dead container.
#[cfg(test)]
#[path = "tests/checks/t212_objective_spine_tests.rs"]
mod t212_objective_spine_tests;

/// T-212 — the hand-staged 1.3 golden actually REACHES the reader.
///
/// `flatten.rs` emits no `objectives[]` on `/compiled` (T-946.36), so the only document that can
/// reach `TBD_ObjectiveEntityReader` today is a hand-staged schemaVersion 1.3 one, and
/// `golden-missions/schema-1_3-wire-fields.json` is that document. This is the T-685 precedent for
/// what "proven" means for a wire field with no live emitter, made mechanical.
///
/// `JsonLoadContext` binds JSON keys onto identically-named class MEMBERS, so "the golden reaches
/// the reader" is exactly the claim "every key the golden's objectives rows author is declared as a
/// member of the reader's structs". A key the structs do not declare is invisible at runtime — not
/// rejected, not logged, simply absent — which is the failure mode this whole ticket exists to end,
/// so it is asserted rather than eyeballed.
///
/// What it cannot prove is stated rather than implied: the gate for the `.c` half is
/// `cargo xtask mod compile`, which cannot run a round. Whether the attacker and the defender are
/// actually shown different text with two clients connected is a human checklist item.
#[cfg(test)]
#[path = "tests/checks/t212_staged_golden_tests.rs"]
mod t212_staged_golden_tests;

/// Execute the actual side-validation and framing branches with a small native shim. This is
/// source simulation, not Enfusion JSON loading, engine execution or a two-client RPC proof.
/// C++ preserves these methods' control flow; only static-member punctuation is translated.
#[cfg(test)]
#[path = "tests/checks/t212_side_fallback_tests.rs"]
mod t212_side_fallback_tests;

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
pub use specification_consistency::t090_specs;

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
