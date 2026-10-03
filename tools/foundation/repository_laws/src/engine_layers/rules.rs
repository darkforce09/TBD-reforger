//! The engine-layer walls' matchers, pinned residues and scanned roots.
//!
//! **Role:** every regular expression, path and pin the eight engine-layer rules judge with.
//! **Position:** the constants half of [`super`], which owns the rules; the report text lives in
//! [`super::report_text`].
//! **Signals & state:** none; every item here is a `const` a reviewer can read end to end.
//! **Invariants:** a matcher is a constant, not a composed string. `concat!` takes literals and
//! not `const` idents and a `format!` would make these runtime `String`s built inside a check
//! whose whole point is that its matchers are readable without running it — so each one is
//! written out. The one exception is rule 6's arm over the wasm-only graphics crates
//! ([`wasm_only_graphics_import_re`]): its alternatives are package names read from the
//! workspace members, which are data rather than code, and it is probed like every constant
//! before it judges.

/// The parked renderer: rules 1 and 2 scan it beside every member of [`GRAPHICS_CATEGORY`], and
/// rule 6 forbids the frontend to import it.
pub(super) const GRAPHICS_ENGINE_REL: &str = "legacy/graphics_engine";

/// The graphics category of the crate-tier law. Every workspace member declaring it is a subject
/// of rules 1 and 2; the members that declare `targets = "wasm32"` are also rule 6's.
pub(super) const GRAPHICS_CATEGORY: &str = "crates/graphics";

/// Rule 6's import matcher for the wasm-only members of [`GRAPHICS_CATEGORY`], whose crate
/// identifiers are `identifiers`: the same two import shapes as [`GRAPHICS_IMPORT_RE`], one
/// alternative per crate. A crate identifier is `[A-Za-z0-9_]` only, so no alternative needs
/// escaping.
pub(super) fn wasm_only_graphics_import_re(identifiers: &[String]) -> String {
    let crates = identifiers.join("|");
    format!(r"\b(?:{crates})\s*::|\bextern\s+crate\s+(?:{crates})\b")
}

/// Rule 1's source matcher — the two shapes that are an import of the map engine rather than a
/// mention, over the parked renderer and every graphics-category member.
///
/// The package and the library share the spelling `map_engine`, so the crate name alone would
/// match every doc comment that describes the wall. `::` after the crate name is what separates
/// `use map_engine::frame;` and an inline `map_engine::frame::boot(&d)` from that prose;
/// `extern crate` is the second spelling and costs one alternative.
pub(super) const MAP_ENGINE_IMPORT_RE: &str = r"\bmap_engine\s*::|\bextern\s+crate\s+map_engine\b";
/// Rule 1's Cargo spelling — what a dependency edge looks like in the manifest.
pub(super) const MAP_ENGINE_PKG: &str = "map_engine";

/// The map engine. Rule 3b is scoped to it and nothing else.
pub(super) const MAP_CRATE_REL: &str = "legacy/map_engine";

/// Rule 3a's matcher — the packet vocabulary's path, however it is reached.
///
/// `\b` is what keeps a hypothetical `::frames` or `::frame_stats` module from counting as the
/// frame vocabulary; `::frame::X`, `::frame;` and a bare `::frame` in prose all end on a
/// boundary and all match. No `use` anchor: `graphics_engine::frame::CameraUniform::new`
/// written inline is the same breach as importing it.
pub(super) const FRAME_VOCAB_RE: &str = r"graphics_engine::frame\b";

/// The enumerated packet boundary — file, exact count, and what the count IS.
///
/// One row, and it should stay one row. Read the module docs before adding a second: a new file
/// naming the frame vocabulary is almost never the right fix, because the thing it wants is
/// already re-exported from `frame/mod.rs` under `crate::frame::…`. The count is the size of that
/// re-export list, so a diff here is a deliberate widening of the crate's graphics interface.
///
/// The count is five `pub use graphics_engine::frame::…` lines: the `packet` and `present`
/// modules, the batch/payload/packet/indirect group, the buffer group and the text group. The
/// GPU-free half of the vocabulary (the camera uniform, damage tracking, the three ids) is
/// `render_primitives::frame`, which no matcher here counts.
pub(super) const RULE3A_PIN: &[(&str, usize, &str)] = &[(
    "legacy/map_engine/src/frame/mod.rs",
    5,
    "the enumerated packet vocabulary (§2C.1 Kind C): packet, present, the \
     batch/payload/packet/indirect group, the buffer group, the text group",
)];

/// Rule 3b's matcher — the four graphics modules that own GPU resources.
///
/// `\b` after the group is what keeps `::pipeline as pipelines` a hit and a hypothetical
/// `::pipelines` module from being one; `r#` is literal, so the raw-identifier spelling of the
/// `loop` module is matched exactly as it is written in source.
pub(super) const GPU_MODULE_RE: &str = r"graphics_engine::(device|pipeline|shaders|r#loop)\b";

/// The pinned residue of rule 3b — file, exact count, and why it cannot close.
///
/// Read the module docs before touching this. The short version: every entry exists because
/// `RenderEngine` is defined in `map_engine`, not in `graphics_engine`. Adding a
/// row is claiming a new GPU-resource import is permanent; almost always the right move is to
/// relocate the construction into `graphics_engine` instead.
pub(super) const RULE3B_PIN: &[(&str, usize, &str)] = &[
    (
        "legacy/map_engine/src/frame/mod.rs",
        3,
        "device::buffers + pipeline aliases at one seam each (3 and 18 call sites), \
         and the doc line on the r#loop re-export the frontend reaches the pump through",
    ),
    (
        "legacy/map_engine/src/frame/pump.rs",
        2,
        "impl FrameTarget for RenderEngine — E0116 pins the impl to the crate that \
         defines the type, and #[wasm_bindgen] refuses trait impls",
    ),
];

/// Rule 2's declaration matcher. See the module docs for why it is anchored on a keyword.
pub(crate) const DECL_RE: &str =
    r"\b(struct|enum|trait|type|fn|const|static|mod)\s+\w*(terrain|symbology|mission|orbat|arma)";

/// The editor's decisions, inside the engine — rule 5's root and nothing else.
pub(super) const EDITING_REL: &str = "legacy/map_engine/src/editing";

/// The presentation crate — rule 6 is scoped to it and nothing else.
pub(super) const FRONTEND_REL: &str = "apps/frontend";

/// Rule 5's matcher — the three names a browser arrives under.
///
/// The bare word, on purpose: this is the same question the program's acceptance line asks
/// (`rg 'web_sys|leptos|wasm_bindgen' …/editing`), and inside this one directory a mention in
/// prose is as much a breach as an import. `\b` on both sides so a future `web_sysfs` or
/// `leptosaur` is not caught by its prefix — a rule that fires on a name it was not written for
/// is a rule that gets suppressed.
pub(super) const DOM_RE: &str = r"\b(web_sys|leptos|wasm_bindgen)\b";

/// Rule 6's source matcher — the two shapes that are an import rather than a mention.
///
/// `::` after the crate name is what separates `use graphics_engine::draw::triangulate;`
/// and an inline `graphics_engine::draw::triangulate(&v)` from a doc comment describing
/// the boundary. `extern crate` is the second spelling and costs one alternative.
pub(super) const GRAPHICS_IMPORT_RE: &str =
    r"\bgraphics_engine\s*::|\bextern\s+crate\s+graphics_engine\b";

/// Rule 6's Cargo spelling — what a dependency edge on the parked renderer looks like in a
/// manifest. A wasm-only graphics member's edge is spelled with its own package name.
pub(super) const GRAPHICS_PKG: &str = "graphics_engine";

/// The authored document's tree — rules 4 and 7's `data` side.
pub(super) const DATA_REL: &str = "legacy/map_engine/src/data";
/// The static world's tree — rule 7's `world` side.
pub(super) const WORLD_REL: &str = "legacy/map_engine/src/world";
/// The authored mission the server links on its own — rule 4's root.
pub(super) const SCENARIO_REL: &str = "legacy/map_engine/src/data/scenario";

// ── RULES 4 AND 7'S MATCHERS ─────────────────────────────────────────────────────────────────
//
// Rule 4 and rule 7's `data` side spell "outside this tree" as the top-level modules that are
// not the tree's own, plus the renderer crate. That is the whole of it written out, because
// `map_engine` declares exactly ten top-level modules (`lib.rs`: camera, data, diagnostics, doll,
// editing, frame, overlay, spatial, streaming, world) and an enumeration is cheaper to read — and
// impossible to widen by accident — than a negation would be. Code that left the map engine for a
// crate of its own (the on-disk formats in `world_file_formats`, the camera projections in
// `camera_math`, the grid references in `map_coordinates`) is no module of the crate, so no arm
// names it: the crate-tier law and the map engine's feature gates judge those edges. `camera`
// stays in the list because the module still holds the viewport. `\b` after each group is what
// keeps a future `crate::worldgen` from being caught by its prefix rather than by its name.
//
// A raw string processes no escapes, so these stay one line each: a `\` continuation inside
// `r"…"` would put a literal backslash and the next line's indentation into the pattern.
//
// ── THE `super::` ARM, AND THE ONE NAME IT CANNOT COVER ──────────────────────────────────────
//
// `crate::streaming::x` is not the only way to spell an escape. `use super::super::super::
// streaming::x;` reaches the same module and a `crate::`-anchored matcher never sees it, which
// would leave every rule below with a documented one-line bypass. How many `super`s it takes to
// escape depends on the file's depth, and file depth is not module depth in this repo —
// `#[path = "../../tests/cases_1.rs"] mod tests;` is used throughout — so a depth calculation would be
// unsound, and an unsound gate rule is worse than none.
//
// What IS sound is the destination. A `super::` chain of any length that lands on a name which
// does not exist inside the scanned tree has escaped it, whatever the depth. Every top-level
// module name, `editing` included, collides with nothing in the two trees but one —
// `data/scenario/compiler/flatten/diagnostics.rs` — so `diagnostics` is the single name left out
// of the `super::` arm, and `super::diagnostics` from inside `flatten/` stays legal because it
// is. The `crate::` arm still covers `crate::diagnostics`.
//
// The three patterns are written out rather than composed from shared fragments: `concat!` takes
// literals and not `const` idents, and a `format!` would make them runtime `String`s built in a
// gate whose whole point is that its matchers are constants a reviewer can read.

/// Rule 4's matcher — everything outside `data/scenario`: the nine sibling modules, the renderer,
/// the document store, and the `super::` spelling of each. `\b` after `data::store` is what keeps
/// a hypothetical `data::stored_rows` from matching on the prefix.
pub(super) const RULE4_RE: &str = r"crate::(camera|diagnostics|doll|editing|frame|overlay|spatial|streaming|world|data::store)\b|graphics_engine|\bsuper::(super::)*(store|camera|doll|editing|frame|overlay|spatial|streaming|world)\b";

/// Rule 7's `data` side — the authored document may name `crate::data` and nothing else.
pub(super) const RULE7_DATA_RE: &str = r"crate::(camera|diagnostics|doll|editing|frame|overlay|spatial|streaming|world)\b|graphics_engine|\bsuper::(super::)*(camera|doll|editing|frame|overlay|spatial|streaming|world)\b";

/// Rule 7's `world` side — the static world may name neither the document nor the CRDT crate.
///
/// `yrs::` and not `\byrs\b`: the bare word would match prose ("3 yrs"), and a matcher that fires
/// on prose gets suppressed. Every real shape is a path — `use yrs::Doc`, `-> yrs::TransactionMut`,
/// `yrs::Transact::transact` — so the `::` is free precision, not a loophole.
pub(super) const RULE7_WORLD_RE: &str = r"crate::data\b|\byrs::|\bsuper::(super::)*data\b";

/// Rule 4's pinned residue — file, exact count, and why it does not reach the `api` build.
///
/// Read the module docs before adding a row. Both entries are `#[cfg(feature = "store")]` test
/// code, and `api` links the `scenario` feature alone, so neither is compiled by the build
/// this rule protects. An *ungated* import of the store from `data/scenario/` would satisfy this
/// pin's count and still be wrong — which is why the pin carries the reason and not just a number.
pub(super) const RULE4_PIN: &[(&str, usize, &str)] = &[
    (
        "legacy/map_engine/src/data/scenario/compiler/flatten/tests/mod.rs",
        1,
        "cfg(feature = \"store\") — vehicles_from_writer_json_roundtrip builds a real \
         MissionDocCore and flattens it; api compiles neither the cfg nor the test",
    ),
    (
        "legacy/map_engine/src/data/scenario/compiler/payload/tests/cases_1.rs",
        1,
        "cfg(feature = \"store\") — briefing_prose_round_trips_through_the_document_core, \
         the same pairing from the payload side",
    ),
];
