//! The engine-layer report text: rule headlines, remedy paragraphs and refusal blocks.
//!
//! **Role:** the fixed lines the engine-layer report prints around each rule's result.
//! **Position:** the text half of [`super`]; `cargo xtask verify engine-layers` prints these
//! lines unchanged, so they are part of that gate's output contract.
//! **Signals & state:** none; constants.
//! **Invariants:** a headline names its rule number and the tree it scans, and every remedy
//! paragraph cites the section of `documentation/standards/engine_boundary_rules.md` it
//! enforces. Continuation lines are indented by six spaces, as every gate's are.

pub(super) const RULE1_HEAD: &str = "==> engine-layers rule 1 — legacy/graphics_engine and the crates/graphics \
     members must not import map_engine";
pub(super) const RULE2_HEAD: &str = "==> engine-layers rule 2 — no map noun in a declared name under \
     legacy/graphics_engine or a crates/graphics member";
pub(super) const RULE3A_HEAD: &str = "==> engine-layers rule 3a — only the enumerated packet boundary may \
     name graphics_engine::frame under legacy/map_engine/src";
pub(super) const RULE3B_HEAD: &str = "==> engine-layers rule 3b — no GPU-resource module of \
     graphics_engine named under legacy/map_engine/src";
pub(super) const RULE6_HEAD: &str = "==> engine-layers rule 6 — apps/frontend must not import graphics_engine \
     or a wasm-only crates/graphics member";
pub(super) const RULE7_HEAD: &str = "==> engine-layers rule 7 — legacy/map_engine/src/world names no home \
     of the authored document";

pub(super) const RULE1_TAIL: &[&str] = &[
    "      The arrow runs map-engine -> graphics layer and only that way. Compute it in",
    "      map-engine and hand the result over as a FramePacket / DrawBatch / TextRun",
    "      (documentation/standards/engine_boundary_rules.md §1",
    "      \"Dependency direction — non-negotiable\").",
];
pub(super) const RULE2_TAIL: &[&str] = &[
    "      The graphics layer is a renderer and may name geometry and GPU handles only. A map",
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
pub(super) const RULE6_TAIL: &[&str] = &[
    "      The frontend reaches the renderer through map_engine and only through it —",
    "      that crate owns the frame vocabulary (rule 3a) and the GPU resources (rule 3b). Ask",
    "      the map engine for the answer; do not import the renderer or a wasm-only graphics",
    "      crate (the CPU-only crates/graphics members are shared building blocks)",
    "      (documentation/standards/engine_boundary_rules.md §5 rule 6).",
];
pub(super) const RULE7_TAIL: &[&str] = &[
    "      world/ is streamed, immutable and never persisted; the mission document (yrs,",
    "      mission_crdt, mission_document, mission_operations and the mission-editing crates",
    "      that host it) is authored, undoable and persisted. A document handle in world/ fuses",
    "      them back together — hand the world's answer to the editing layer instead",
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
