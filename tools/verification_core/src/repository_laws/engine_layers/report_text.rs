//! The engine-layer report text: rule headlines, remedy paragraphs and refusal blocks.
//!
//! **Role:** the fixed lines the engine-layer report prints around each rule's result.
//! **Position:** the text half of [`super`]; `cargo xtask verify engine-layers` prints these
//! lines unchanged, so they are part of that gate's output contract.
//! **Signals & state:** none; constants.
//! **Invariants:** a headline names its rule number and the tree it scans, and every remedy
//! paragraph cites the section of `documentation/standards/engine_boundary_rules.md` it
//! enforces. Continuation lines are indented by six spaces, as every gate's are.

pub(super) const RULE1_HEAD: &str =
    "==> engine-layers rule 1 — legacy/graphics_engine must not import map_engine";
pub(super) const RULE2_HEAD: &str =
    "==> engine-layers rule 2 — no map noun in a declared name under legacy/graphics_engine";
pub(super) const RULE3A_HEAD: &str = "==> engine-layers rule 3a — only the enumerated packet boundary may \
     name graphics_engine::frame under legacy/map_engine/src";
pub(super) const RULE3B_HEAD: &str = "==> engine-layers rule 3b — no GPU-resource module of \
     graphics_engine named under legacy/map_engine/src";
pub(super) const RULE4_HEAD: &str = "==> engine-layers rule 4 — legacy/map_engine/src/data/scenario \
     imports nothing outside itself";
pub(super) const RULE5_HEAD: &str = "==> engine-layers rule 5 — no web_sys / leptos / wasm_bindgen under \
     legacy/map_engine/src/editing";
pub(super) const RULE6_HEAD: &str =
    "==> engine-layers rule 6 — apps/frontend must not import graphics_engine";
pub(super) const RULE7_HEAD: &str = "==> engine-layers rule 7 — the static world and the authored document \
     share nothing under legacy/map_engine/src";

pub(super) const RULE1_TAIL: &[&str] = &[
    "      The arrow runs map-engine -> graphics-engine and only that way. Compute it in",
    "      map-engine and hand the result over as a FramePacket / DrawBatch / TextRun",
    "      (documentation/standards/engine_boundary_rules.md §1",
    "      \"Dependency direction — non-negotiable\").",
];
pub(super) const RULE2_TAIL: &[&str] = &[
    "      graphics-engine is a renderer and may name geometry and GPU handles only. A map",
    "      noun in a declared name means domain logic came back across the wall — move the",
    "      decision to map-engine and leave the packing here",
    "      (documentation/standards/engine_boundary_rules.md §5 rule 2).",
];
pub(super) const RULE3A_TAIL: &[&str] = &[
    "      The frame vocabulary is re-exported from map-engine/src/frame/mod.rs, enumerated.",
    "      Write `use crate::frame::DrawBatch;` — the point of the boundary is that the whole",
    "      graphics interface reads as one list",
    "      (documentation/standards/engine_boundary_rules.md §5 rule 3a, §2C.1 Kind C).",
];
pub(super) const RULE3B_TAIL: &[&str] = &[
    "      device / pipeline / shaders / r#loop create and own GPU resources, and that is",
    "      graphics-engine's job — map-engine receives already-built handles. Relocate",
    "      the construction; do not add a row to RULE3B_PIN",
    "      (documentation/standards/engine_boundary_rules.md §5 rule 3b).",
];
pub(super) const RULE4_TAIL: &[&str] = &[
    "      api links this crate at the `scenario` feature alone — that is why its tree",
    "      carries no wgpu, png, rkyv or flate2. One import here drags a whole tier into an HTTP",
    "      server. Pass the value in as an argument",
    "      (documentation/standards/engine_boundary_rules.md §5 rule 4).",
];
pub(super) const RULE5_TAIL: &[&str] = &[
    "      editing/ holds the editor's DECISIONS — tool state machines, the undo drive, the",
    "      command formatting — and every one of them must be answerable by `cargo test` with no",
    "      browser. Take what only a host can supply as an injected closure or fn pointer, and",
    "      leave the window, the signal and the element on the frontend side of the wall",
    "      (documentation/standards/engine_boundary_rules.md §5 rule 5).",
];
pub(super) const RULE6_TAIL: &[&str] = &[
    "      The frontend reaches the renderer through map_engine and only through it —",
    "      that crate owns the frame vocabulary (rule 3a) and the GPU resources (rule 3b). Ask",
    "      the map engine for the answer; do not import the renderer",
    "      (documentation/standards/engine_boundary_rules.md §5 rule 6).",
];
pub(super) const RULE7_TAIL: &[&str] = &[
    "      world/ is streamed, immutable and never persisted; data/ is authored, undoable and",
    "      persisted. They share the spatial index and nothing else — a chunk id in data/ or a",
    "      document handle in world/ fuses them back together",
    "      (documentation/standards/engine_boundary_rules.md §2D).",
];
pub(super) const PROBE_FAIL: &[&str] = &[
    "FAIL: matcher self-probe returned no match over a subject it must match.",
    "      The search engine is broken. A check that cannot run is not a pass.",
];
pub(super) const NOTHING_TAIL: &[&str] = &[
    "      An engine-layer check with no inputs is not a pass: either the crate moved and this",
    "      gate is scanning a husk, or the walk is broken. Both are red.",
];
