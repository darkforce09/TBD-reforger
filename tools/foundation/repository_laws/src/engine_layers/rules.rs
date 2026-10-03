//! The engine-layer walls' matchers, pinned residues and scanned roots.
//!
//! **Role:** every regular expression, path and pin the six engine-layer rules judge with.
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

/// The presentation crate — rule 6 is scoped to it and nothing else.
pub(super) const FRONTEND_REL: &str = "apps/frontend";

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

/// The static world's tree — rule 7's subject.
pub(super) const WORLD_REL: &str = "legacy/map_engine/src/world";

// ── RULE 7'S MATCHER ─────────────────────────────────────────────────────────────────────────
//
// The static world may name no home of the authored document: the CRDT crate (`yrs`), the mission
// crates that hold the document (`mission_crdt`, `mission_document`, `mission_operations`), and
// the mission-editing crates that host the live document and drive its undo, commands, drafts and
// tools (`mission_editing_session`, `mission_editing_commands`, `mission_persistence`,
// `map_editing_tools`). That is the whole list written out, because an enumeration is cheaper to
// read — and impossible to widen by accident — than a negation would be. The mission and
// mission-editing crates' own edges are the crate-tier law's (neither category may depend on the
// map engine or on a streaming or graphics crate), so this matcher judges the one direction that
// law cannot see: a module of this crate reaching the document.
//
// `yrs::` and not `\byrs\b`: the bare word would match prose ("3 yrs"), and a matcher that fires
// on prose gets suppressed. Every real shape is a path — `use yrs::Doc`, `-> yrs::TransactionMut`,
// `yrs::Transact::transact` — so the `::` is free precision, not a loophole. `\b` before each crate
// name keeps `my_mission_document::` from matching on a suffix, and the `::` after it keeps a
// `mission_document_notes::` from matching on a prefix.
//
// A raw string processes no escapes, so the matcher stays one line: a `\` continuation inside
// `r"…"` would put a literal backslash and the next line's indentation into the pattern.

/// Rule 7's matcher — the static world may name neither the CRDT crate, nor a mission document
/// crate, nor a mission-editing crate that hosts the live document.
pub(super) const RULE7_WORLD_RE: &str = r"\byrs::|\b(mission_crdt|mission_document|mission_operations|mission_editing_session|mission_editing_commands|mission_persistence|map_editing_tools)\s*::";
