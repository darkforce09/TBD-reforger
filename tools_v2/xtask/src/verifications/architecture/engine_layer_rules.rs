//! Role: the engine-layer walls' matchers, pins and report text.
//! Position: `xtask` — the constants half of [`super`], which owns the rules themselves.
//! Signals & state: none; every item here is a `const` a reviewer can read end to end.
//! Invariants: a matcher is a constant, not a composed string. `concat!` takes literals and not
//! `const` idents and a `format!` would make these runtime `String`s built inside a gate whose
//! whole point is that its matchers are readable without running it — so each one is written out.

/// The pure crate. Rules 1 and 2 are both scoped to it and nothing else.
pub(super) const CRATE_REL: &str = "apps/website/graphics-engine";

/// Rule 1's Rust spelling — what an `use`/path reference to the map engine looks like in source.
pub(super) const MAP_ENGINE_PATH: &str = "website_map_engine";
/// Rule 1's Cargo spelling — what a dependency edge looks like in the manifest.
pub(super) const MAP_ENGINE_PKG: &str = "website-map-engine";

/// The map engine. Rule 3b is scoped to it and nothing else.
pub(super) const MAP_CRATE_REL: &str = "apps/website/map-engine";

/// Rule 3a's matcher — the packet vocabulary's path, however it is reached.
///
/// `\b` is what keeps a hypothetical `::frames` or `::frame_stats` module from counting as the
/// frame vocabulary; `::frame::X`, `::frame;` and a bare `::frame` in prose all end on a
/// boundary and all match. No `use` anchor: `website_graphics_engine::frame::CameraUniform::new`
/// written inline is the same breach as importing it, and phase 2C found three of exactly that
/// shape (`diagnostics/readback/scene.rs`, `overlay/lanes.rs`).
pub(super) const FRAME_VOCAB_RE: &str = r"website_graphics_engine::frame\b";

/// The enumerated packet boundary — file, exact count, and what the count IS.
///
/// One row, and it should stay one row. Read the module docs before adding a second: a new file
/// naming the frame vocabulary is almost never the right fix, because the thing it wants is
/// already re-exported from `frame/mod.rs` under `crate::frame::…`. The count is the size of that
/// re-export list, so a diff here is a deliberate widening of the crate's graphics interface.
pub(super) const RULE3A_PIN: &[(&str, usize, &str)] = &[(
    "apps/website/map-engine/src/frame/mod.rs",
    8,
    "the enumerated packet vocabulary (§2C.1 Kind C): damage, packet, present, CameraUniform, \
     the three ids, the batch/payload/packet/indirect group, the buffer group, the text group",
)];

/// Rule 3b's matcher — the five graphics modules that own GPU resources.
///
/// `\b` after the group is what keeps `::pipeline as pipelines` a hit and a hypothetical
/// `::pipelines` module from being one; `r#` is literal, so the raw-identifier spelling of the
/// `loop` module is matched exactly as it is written in source.
pub(super) const GPU_MODULE_RE: &str =
    r"website_graphics_engine::(device|pipeline|shaders|r#loop|text::gpu)\b";

/// The pinned residue of rule 3b — file, exact count, and why it cannot close.
///
/// Read the module docs before touching this. The short version: every entry exists because
/// `RenderEngine` did not cross to `website-graphics-engine` in Phase 1. Adding a row is claiming
/// a new GPU-resource import is permanent; almost always the right move is to relocate the
/// construction instead, which is what Phase 2B did to the other twelve.
pub(super) const RULE3B_PIN: &[(&str, usize, &str)] = &[
    (
        "apps/website/map-engine/src/frame/mod.rs",
        3,
        "device::buffers + pipeline aliases at one seam each (3 and 18 call sites), \
         and the doc line on the r#loop re-export the frontend reaches the pump through",
    ),
    (
        "apps/website/map-engine/src/frame/pump.rs",
        2,
        "impl FrameTarget for RenderEngine — E0116 pins the impl to the crate that \
         defines the type, and #[wasm_bindgen] refuses trait impls",
    ),
];

/// Rule 2's declaration matcher. See the module docs for why it is anchored on a keyword.
pub(super) const DECL_RE: &str =
    r"\b(struct|enum|trait|type|fn|const|static|mod)\s+\w*(terrain|symbology|mission|orbat|arma)";

/// The editor's decisions, inside the engine — rule 5's root and nothing else.
pub(super) const EDITING_REL: &str = "apps/website/map-engine/src/editing";

/// The presentation crate — rule 6 is scoped to it and nothing else.
pub(super) const FRONTEND_REL: &str = "apps/website/frontend";

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
/// `::` after the crate name is what separates `use website_graphics_engine::draw::triangulate;`
/// and an inline `website_graphics_engine::draw::triangulate(&v)` from a doc comment describing
/// the boundary. `extern crate` is the second spelling and costs one alternative.
pub(super) const GRAPHICS_IMPORT_RE: &str =
    r"\bwebsite_graphics_engine\s*::|\bextern\s+crate\s+website_graphics_engine\b";

/// Rule 6's Cargo spelling — what a dependency edge on the renderer looks like in a manifest.
pub(super) const GRAPHICS_PKG: &str = "website-graphics-engine";

/// The authored document's tree — rules 4 and 7's `data` side.
pub(super) const DATA_REL: &str = "apps/website/map-engine/src/data";
/// The static world's tree — rule 7's `world` side.
pub(super) const WORLD_REL: &str = "apps/website/map-engine/src/world";
/// The authored mission the server links on its own — rule 4's root.
pub(super) const SCENARIO_REL: &str = "apps/website/map-engine/src/data/scenario";

// ── RULES 4 AND 7'S MATCHERS ─────────────────────────────────────────────────────────────────
//
// All three spell "outside this tree" as the nine top-level modules that are not the tree's own,
// plus the renderer crate. That is the whole of it written out, because `website-map-engine` has
// exactly ten top-level modules and an enumeration is cheaper to read — and impossible to widen
// by accident — than a negation would be. `\b` after each group is what keeps a future
// `crate::io_util` or `crate::worldgen` from being caught by its prefix rather than by its name.
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
// module name was checked against the two trees for collisions; exactly one exists —
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
pub(super) const RULE4_RE: &str = r"crate::(camera|diagnostics|doll|frame|io|overlay|spatial|streaming|world|data::store)\b|website_graphics_engine|\bsuper::(super::)*(store|camera|doll|frame|io|overlay|spatial|streaming|world)\b";

/// Rule 7's `data` side — the authored document may name `crate::data` and nothing else.
pub(super) const RULE7_DATA_RE: &str = r"crate::(camera|diagnostics|doll|frame|io|overlay|spatial|streaming|world)\b|website_graphics_engine|\bsuper::(super::)*(camera|doll|frame|io|overlay|spatial|streaming|world)\b";

/// Rule 7's `world` side — the static world may name neither the document nor the CRDT crate.
///
/// `yrs::` and not `\byrs\b`: the bare word would match prose ("3 yrs"), and a matcher that fires
/// on prose gets suppressed. Every real shape is a path — `use yrs::Doc`, `-> yrs::TransactionMut`,
/// `yrs::Transact::transact` — so the `::` is free precision, not a loophole.
pub(super) const RULE7_WORLD_RE: &str = r"crate::data\b|\byrs::|\bsuper::(super::)*data\b";

/// Rule 4's pinned residue — file, exact count, and why it does not reach the `website-api` build.
///
/// Read the module docs before adding a row. Both entries are `#[cfg(feature = "store")]` test
/// code, and `website-api` links the `scenario` feature alone, so neither is compiled by the build
/// this rule protects. An *ungated* import of the store from `data/scenario/` would satisfy this
/// pin's count and still be wrong — which is why the pin carries the reason and not just a number.
pub(super) const RULE4_PIN: &[(&str, usize, &str)] = &[
    (
        "apps/website/map-engine/src/data/scenario/compiler/flatten/tests/mod.rs",
        1,
        "cfg(feature = \"store\") — vehicles_from_writer_json_roundtrip builds a real \
         MissionDocCore and flattens it; website-api compiles neither the cfg nor the test",
    ),
    (
        "apps/website/map-engine/src/data/scenario/compiler/payload/tests/cases_1.rs",
        1,
        "cfg(feature = \"store\") — briefing_prose_round_trips_through_the_document_core, \
         the same pairing from the payload side",
    ),
];

pub(super) const RULE1_HEAD: &str =
    "==> engine-layers rule 1 — apps/website/graphics-engine must not import website_map_engine";
pub(super) const RULE2_HEAD: &str =
    "==> engine-layers rule 2 — no map noun in a declared name under apps/website/graphics-engine";
pub(super) const RULE3A_HEAD: &str = "==> engine-layers rule 3a — only the enumerated packet boundary may \
     name website_graphics_engine::frame under apps/website/map-engine/src";
pub(super) const RULE3B_HEAD: &str = "==> engine-layers rule 3b — no GPU-resource module of \
     website-graphics-engine named under apps/website/map-engine/src";
pub(super) const RULE4_HEAD: &str = "==> engine-layers rule 4 — apps/website/map-engine/src/data/scenario \
     imports nothing outside itself";
pub(super) const RULE5_HEAD: &str = "==> engine-layers rule 5 — no web_sys / leptos / wasm_bindgen under \
     apps/website/map-engine/src/editing";
pub(super) const RULE6_HEAD: &str =
    "==> engine-layers rule 6 — apps/website/frontend must not import website_graphics_engine";
pub(super) const RULE7_HEAD: &str = "==> engine-layers rule 7 — the static world and the authored document \
     share nothing under apps/website/map-engine/src";

pub(super) const RULE1_TAIL: &[&str] = &[
    "      The arrow runs map-engine -> graphics-engine and only that way. Compute it in",
    "      map-engine and hand the result over as a FramePacket / DrawBatch / TextRun",
    "      (ENGINE_SPLIT_PROGRAM.md §1 \"Dependency direction — non-negotiable\").",
];
pub(super) const RULE2_TAIL: &[&str] = &[
    "      graphics-engine is a renderer and may name geometry and GPU handles only. A map",
    "      noun in a declared name means domain logic came back across the wall — move the",
    "      decision to map-engine and leave the packing here (ENGINE_SPLIT_PROGRAM.md §5 rule 2).",
];
pub(super) const RULE3A_TAIL: &[&str] = &[
    "      The frame vocabulary is re-exported from map-engine/src/frame/mod.rs, enumerated.",
    "      Write `use crate::frame::DrawBatch;` — the point of the boundary is that the whole",
    "      graphics interface reads as one list (ENGINE_SPLIT_PROGRAM.md §5 rule 3a, §2C.1 Kind C).",
];
pub(super) const RULE3B_TAIL: &[&str] = &[
    "      device / pipeline / shaders / text::gpu / r#loop create and own GPU resources, and",
    "      that is graphics-engine's job — map-engine receives already-built handles. Relocate",
    "      the construction; do not add a row to RULE3B_PIN (ENGINE_SPLIT_PROGRAM.md §5 rule 3b).",
];
pub(super) const RULE4_TAIL: &[&str] = &[
    "      website-api links this crate at the `scenario` feature alone — that is why its tree",
    "      carries no wgpu, png, rkyv or flate2. One import here drags a whole tier into an HTTP",
    "      server. Pass the value in as an argument (ENGINE_SPLIT_PROGRAM.md §5 rule 4).",
];
pub(super) const RULE5_TAIL: &[&str] = &[
    "      editing/ holds the editor's DECISIONS — tool state machines, the undo drive, the",
    "      command formatting — and every one of them must be answerable by `cargo test` with no",
    "      browser. Take what only a host can supply as an injected closure or fn pointer, and",
    "      leave the window, the signal and the element on the frontend side of the wall",
    "      (ENGINE_SPLIT_PROGRAM.md §5 rule 5).",
];
pub(super) const RULE6_TAIL: &[&str] = &[
    "      The frontend reaches the renderer through website-map-engine and only through it —",
    "      that crate owns the frame vocabulary (rule 3a) and the GPU resources (rule 3b). Ask",
    "      the map engine for the answer; do not import the renderer",
    "      (ENGINE_SPLIT_PROGRAM.md §5 rule 6).",
];
pub(super) const RULE7_TAIL: &[&str] = &[
    "      world/ is streamed, immutable and never persisted; data/ is authored, undoable and",
    "      persisted. They share the spatial index and nothing else — a chunk id in data/ or a",
    "      document handle in world/ fuses them back together (ENGINE_SPLIT_PROGRAM.md §2D).",
];
pub(super) const PROBE_FAIL: &[&str] = &[
    "FAIL: matcher self-probe returned no match over a subject it must match.",
    "      The search engine is broken. A check that cannot run is not a pass.",
];
pub(super) const NOTHING_TAIL: &[&str] = &[
    "      An engine-layer check with no inputs is not a pass: either the crate moved and this",
    "      gate is scanning a husk, or the walk is broken. Both are red.",
];
