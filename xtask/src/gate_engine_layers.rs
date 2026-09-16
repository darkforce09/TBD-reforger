//! Engine-layer walls — [`ENGINE_SPLIT_PROGRAM.md`] §5 rules **1, 2, 3a, 3b, 4 and 7**.
//!
//! ── WHAT THIS DEFENDS ────────────────────────────────────────────────────────────────────────
//!
//! Phase 1 of the engine split took `website-graphics-engine` apart and put the ~50k lines of map
//! domain (terrain, symbology, spatial, streaming) in `website-map-engine`, leaving behind a crate
//! that is *only* a renderer: device, pipelines, shaders, draw batching, text packing, the rAF
//! pump. The dependency arrow is one-way and non-negotiable:
//!
//! ```text
//! frontend ──► website-map-engine ──► website-graphics-engine
//! ```
//!
//! Nothing in the compiler enforces that. `website-graphics-engine` could add a dependency edge
//! back to `website-map-engine` tomorrow and everything would still build — the wall exists only
//! because someone drew it in a document. §5 of the program says so outright: *"One 99k crate rots
//! without enforcement."* Rules 1 and 2 are the two halves of that wall:
//!
//! | # | rule | what rots without it |
//! |---|------|----------------------|
//! | 1 | `apps/website/graphics-engine/**` may not import `website_map_engine` | the arrow turns into a cycle and the split is undone by accident |
//! | 2 | no `terrain` / `symbology` / `mission` / `orbat` / `arma` in a declared name there | "pure renderer" becomes a claim in a README rather than a property of the code |
//! | 3a | only one enumerated file of `apps/website/map-engine/src` names `website_graphics_engine::frame` | the packet boundary stops being a boundary and becomes 39 scattered imports again |
//! | 3b | no module of `apps/website/map-engine/src` names `website_graphics_engine::{device, pipeline, shaders, text::gpu, r#loop}` | GPU resource creation drifts back to the caller one convenient import at a time |
//! | 4 | `apps/website/map-engine/src/data/scenario/**` imports nothing outside itself | the `website-api` build stops being thin |
//! | 7 | `data/**` names no world module and `world/**` names no document module | the static world and the authored document fuse back into one soup |
//!
//! Rules 5 and 6 (no DOM in map-engine, frontend may not import graphics) police a tree that
//! phase 3 has not built yet. Rule 5 in particular **cannot** hold while `map-engine` is a wasm
//! crate with `editing/` still in the browser; it lands with phase 3, not before.
//!
//! ── RULE 4 AND WHY ITS PIN HAS TWO ROWS ──────────────────────────────────────────────────────
//!
//! §5 writes rule 4 as *"`map-engine/data/scenario/**` imports nothing outside itself"*, and what
//! it guards is the `website-api` build: the server links `website-map-engine` at the `scenario`
//! feature alone, which is why `cargo tree -p website-api | rg -i 'wgpu|png|rkyv|flate2'` comes
//! back empty. One import of `crate::streaming` inside `data/scenario/` would drag the whole
//! streaming tier — and with it `rkyv`, `flate2`, `png` and `wgpu` — into an HTTP server.
//!
//! "Outside itself" is enumerable because the crate has exactly ten top-level modules. Nine of
//! them are outside `data`, `data::store` is the tenth's other half, and `website_graphics_engine`
//! is outside the crate entirely. That list *is* the matcher; there is no wildcard in it.
//!
//! The production tree satisfies the rule at zero sites. Two test sites remain and they are
//! pinned rather than moved, because both are `#[cfg(feature = "store")]` — `briefing_prose_
//! round_trips_through_the_document_core` and `vehicles_from_writer_json_roundtrip` build a real
//! `MissionDocCore` and push it through the scenario compiler, which is the only honest way to
//! test that pairing. `website-api` compiles neither: the `scenario` tier does not turn `store`
//! on, so the two sites are invisible to the build the rule exists to protect. Pinning records
//! that, and ratchets it — a third one, or an ungated one, fails.
//!
//! ── RULE 7 — THE WALL §2D ASKED FOR, AND EXACTLY WHAT IT CAN SEE ─────────────────────────────
//!
//! §2D: *"`world/` is immutable, streamed from `packages/map-assets`, cacheable, never persisted.
//! `data/` is mutable, undoable, CRDT-synced, persisted. They share the spatial index and nothing
//! else. Do not let a `world/` type gain a `dirty` flag or a `data/` type gain a chunk id."*
//!
//! The literal sentence cannot be gated. A `dirty` flag is a struct **field** — a bare identifier
//! inside a block with no keyword in front of it — and rule 2's module note already explains why
//! a line matcher cannot see those. A matcher for the *word* `dirty` under `world/` would fire on
//! prose and get suppressed, and a rule nobody is stopped by is not a rule.
//!
//! What can be gated exactly is the **import wall**, and it is not a weaker statement than it
//! looks. A `world/` type cannot acquire undo, persistence or CRDT state without naming either
//! `crate::data` or `yrs` — those are the only two places in this crate where a value becomes
//! authored content that survives a reload. A `data/` type cannot acquire a chunk id, a tile
//! coordinate, an LOD level or a residency handle without naming one of the nine sibling modules,
//! because that is where every one of those types is declared. So:
//!
//! * `data/**` may name `crate::data` and nothing else in the crate, and may not name
//!   `website_graphics_engine` at all;
//! * `world/**` may not name `crate::data` and may not name `yrs::`.
//!
//! Both sides read **zero** today, and zero is what makes the anti-vacuity guard load-bearing
//! here rather than decorative: "no file under `data/` names a world module" and "there is no
//! `data/` any more" are the same sentence to a matcher. So both roots are counted, both counts
//! are printed, and an empty `data/` or an empty `world/` is a hard FAIL, not a clean wall.
//!
//! Two things rule 7 deliberately does **not** claim:
//!
//! * It does not catch a hand-rolled `pub dirty: bool` on a world struct that imports nothing.
//!   It catches that flag the moment anything tries to persist it, which is the only point at
//!   which it stops being a local scratch bool and starts being authored content.
//! * The compiler already enforces half of this under a narrow feature build —
//!   `cargo build -p website-map-engine --no-default-features --features store` compiles `data/`
//!   with `world`, `io`, `streaming`, `spatial`, `overlay` and `frame` all absent from the crate.
//!   It does **not** enforce it under `--all-features`, which is the build everything else in CI
//!   runs, and `world/ -> data/` it never enforces at all. That gap is the gate's actual subject.
//!
//! ── RULE 3a AND WHY IT IS A FILE, NOT A DIRECTORY ────────────────────────────────────────────
//!
//! §5 writes rule 3a as *"only `map-engine/src/frame/**` may name `website_graphics_engine::
//! frame`"*. That directory is the ceiling; phase 2C put the tree well under it, and the gate
//! pins where the tree actually is. Before 2C the vocabulary was spelled at **39 sites across 22
//! files** — 16 of those sites, in 11 files, nowhere near `frame/`: `overlay/lanes.rs`,
//! `world/environment/{buildings,vegetation}/buffers.rs`, `diagnostics/readback/scene.rs` and the
//! rest. After 2C it is spelled on **eight `pub use` lines in `frame/mod.rs` and nowhere else** —
//! `frame/`'s own submodules read `use crate::frame::DrawBatch;` like every other module.
//!
//! That is the whole point of the rule and it is worth being exact about, because the cheap
//! version of this gate — "allow anything under `frame/`" — would pass a tree in which thirteen
//! files under `frame/` each imported whatever they felt like. The value of a chokepoint is not
//! the indirection; re-exports compile away. The value is that the crate's entire graphics
//! interface is a list a reviewer can read in one screen, and that widening it is a diff to that
//! screen. A directory rule cannot express that. A per-file pin can.
//!
//! So 3a shares 3b's shape exactly — an enumerated pin with a count per file, ratcheting in both
//! directions. An unpinned file naming the vocabulary fails. A pinned file that gains a site
//! fails, because the list grew without the list being reviewed. A pinned file that loses one
//! fails too, because a pin that no longer describes the tree has quietly stopped meaning what it
//! says. And a pinned file that has vanished entirely fails — which is also 3a's anti-vacuity
//! guard, the one that matters most here: "nothing names the frame vocabulary anywhere" is what a
//! deleted or renamed `frame/` looks like, and it must never read as a clean boundary.
//!
//! The matcher is a line matcher and sees prose as well as code — deliberately, and it cuts both
//! ways. Outside the pinned file that is exactly right: a comment naming
//! `website_graphics_engine::frame` is a comment telling the next reader to import the wrong
//! thing, and phase 2C rewrote the two that existed (`overlay/mod.rs`, `overlay/lanes.rs`) to say
//! `crate::frame::LaneId`, which is what those files actually use. Inside the pinned file it
//! would mean a typo fix in a doc comment could turn the build red, which is how a gate gets
//! suppressed — so `frame/mod.rs`'s own prose is written not to spell the path, and the pinned
//! count is therefore exactly the size of the interface list. If that count changes, the
//! interface changed. That is the property worth ratcheting.
//!
//! ── RULE 3b AND WHY IT IS A PIN, NOT A ZERO ──────────────────────────────────────────────────
//!
//! §5 says rule 3b "must read **zero**". Phase 2B closed 12 of the 17 sites it inherited — the
//! cell-atlas handles moved inside graphics-engine from `text::gpu` to `frame::atlas`, the
//! `TextUniforms` packing moved to `text::pack` and is reached through `layout`, shader-module
//! compilation moved to `pipeline::create_map_shader`, and three shader-scrub tests moved to
//! graphics-engine outright. **Five cannot close, and they all have one cause:** `RenderEngine` is
//! defined in `website-map-engine` (`frame/engine.rs`) and holds every GPU resource the renderer
//! owns — 127 `impl RenderEngine` blocks against graphics-engine's zero. While that is true:
//!
//! * `impl FrameTarget for RenderEngine` must live in the crate that defines the type (E0116), and
//!   `#[wasm_bindgen]` refuses trait impls, so `frame/pump.rs` names `r#loop` and `frame/mod.rs`
//!   documents the re-export it publishes for the frontend — 3 sites;
//! * `RenderEngine` holds a `LanePool` and the eighteen pipeline-construction call sites build
//!   against its own shader module and layouts, so `frame/mod.rs` aliases `device::buffers` and
//!   `pipeline` at one named seam each rather than spelling them 3 and 18 times — 2 sites.
//!
//! So the rule is a **pin**, not a threshold: every allowed site is enumerated below with the file
//! it sits in and the exact count. A new site anywhere fails. An extra site in a pinned file fails.
//! And a pinned file that drops below its count fails too — the pin is a ratchet, and a stale pin
//! is a rule that has quietly stopped meaning what it says. Moving `RenderEngine` across is the
//! only thing that empties this list, and that is an operator decision, not a gate's.
//!
//! ── WHY EXIT 2 EXISTS ────────────────────────────────────────────────────────────────────────
//!
//! A layer wall is exactly the kind of gate that gets renamed out from under itself: phase 2 moves
//! directories on purpose, and the first thing a move breaks is the path this gate scans. If a
//! missing root read as "no violations found", the wall would evaporate on the very commit that
//! most needs it, silently, with a green tick. So a root that is absent or unreadable is
//! [`NotRun`] and exits **2** — "I never looked" is a different operator action from "I looked and
//! it is dirty". Same reasoning, and the same [`tbd_gate::Verdict`] machinery, as
//! `sql_gates::report_did_not_run`.
//!
//! The second vacuity hole is subtler and is guarded separately: a root that *exists* and is
//! *empty*. `walk_files` would return `Ok(vec![])`, `grep_lines` would find nothing, and the gate
//! would report a clean wall over zero bytes of source. So the scanned file count is printed, and
//! zero Rust files is a hard FAIL rather than a pass.
//!
//! ── RULE 2 AND THE WORD "SUBMISSION" ─────────────────────────────────────────────────────────
//!
//! The naive form of rule 2 — grep the five nouns case-insensitively — cannot be used as a gate.
//! The words are ordinary English and ordinary graphics vocabulary: a doc comment may discuss what
//! the *map engine* hands over, a `.wgsl` path may be named after the feature it draws, and
//! "submission" contains "mission". A gate that fires on prose gets suppressed, and a suppressed
//! gate is not a gate.
//!
//! So rule 2 matches **declarations, not mentions**: a declaration keyword, whitespace, then an
//! identifier that contains the noun.
//!
//! ```text
//! \b(struct|enum|trait|type|fn|const|static|mod)\s+\w*(terrain|symbology|mission|orbat|arma)
//! ```
//!
//! case-insensitive, so `struct TerrainBlob`, `fn pack_mission`, `fn submit_mission` and
//! `mod symbology` all fail while `/// a submission arrives here`, `include_str!("terrain.wgsl")`
//! and any sentence naming the map engine all pass. `\w*` is what makes `submit_mission` a
//! violation and `submission` not one: the noun has to sit inside a name that a keyword just
//! introduced.
//!
//! What it deliberately does not see: enum **variants** and struct **fields**, which are bare
//! identifiers inside a block with no keyword in front of them. §5's prose names variants too, but
//! catching them needs block tracking rather than a line matcher, and phase 1D.1 already emptied
//! the one real population (`LaneRole`'s 48 named variants moved to `map-engine/overlay/lanes.rs`
//! and graphics keys on an opaque `LaneId`). The line-level rule is what phase 1F was scoped to.
//!
//! ── WHY `verify engine-layers` AND NOT `verify-engine-layers` ────────────────────────────────
//!
//! §5 writes the command as bare `cargo xtask verify-engine-layers`, which is not the shape this
//! CLI has: every sibling gate is `cargo xtask verify <name>` ([`crate::VerifyCmd`]), and a
//! top-level `verify-engine-layers` would be the only hyphenated verb on the root command. The
//! spec was describing the gate, not the argv. Both spellings work anyway — the `TASKS` alias row
//! is named `verify-engine-layers`, so `cargo xtask ci verify-engine-layers` resolves, the same
//! way `verify-no-node` aliases `verify no-node`.

use std::path::{Path, PathBuf};

use anyhow::Result;
use tbd_gate::scan::{self, Hit};
use tbd_gate::{Kind, NotRun, Pattern, Verdict, gate};

/// The pure crate. Rules 1 and 2 are both scoped to it and nothing else.
const CRATE_REL: &str = "apps/website/graphics-engine";

/// Rule 1's Rust spelling — what an `use`/path reference to the map engine looks like in source.
const MAP_ENGINE_PATH: &str = "website_map_engine";
/// Rule 1's Cargo spelling — what a dependency edge looks like in the manifest.
const MAP_ENGINE_PKG: &str = "website-map-engine";

/// The map engine. Rule 3b is scoped to it and nothing else.
const MAP_CRATE_REL: &str = "apps/website/map-engine";

/// Rule 3a's matcher — the packet vocabulary's path, however it is reached.
///
/// `\b` is what keeps a hypothetical `::frames` or `::frame_stats` module from counting as the
/// frame vocabulary; `::frame::X`, `::frame;` and a bare `::frame` in prose all end on a
/// boundary and all match. No `use` anchor: `website_graphics_engine::frame::CameraUniform::new`
/// written inline is the same breach as importing it, and phase 2C found three of exactly that
/// shape (`diagnostics/readback/scene.rs`, `overlay/lanes.rs`).
const FRAME_VOCAB_RE: &str = r"website_graphics_engine::frame\b";

/// The enumerated packet boundary — file, exact count, and what the count IS.
///
/// One row, and it should stay one row. Read the module docs before adding a second: a new file
/// naming the frame vocabulary is almost never the right fix, because the thing it wants is
/// already re-exported from `frame/mod.rs` under `crate::frame::…`. The count is the size of that
/// re-export list, so a diff here is a deliberate widening of the crate's graphics interface.
const RULE3A_PIN: &[(&str, usize, &str)] = &[(
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
const GPU_MODULE_RE: &str =
    r"website_graphics_engine::(device|pipeline|shaders|r#loop|text::gpu)\b";

/// The pinned residue of rule 3b — file, exact count, and why it cannot close.
///
/// Read the module docs before touching this. The short version: every entry exists because
/// `RenderEngine` did not cross to `website-graphics-engine` in Phase 1. Adding a row is claiming
/// a new GPU-resource import is permanent; almost always the right move is to relocate the
/// construction instead, which is what Phase 2B did to the other twelve.
const RULE3B_PIN: &[(&str, usize, &str)] = &[
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
const DECL_RE: &str =
    r"\b(struct|enum|trait|type|fn|const|static|mod)\s+\w*(terrain|symbology|mission|orbat|arma)";

/// The authored document's tree — rules 4 and 7's `data` side.
const DATA_REL: &str = "apps/website/map-engine/src/data";
/// The static world's tree — rule 7's `world` side.
const WORLD_REL: &str = "apps/website/map-engine/src/world";
/// The authored mission the server links on its own — rule 4's root.
const SCENARIO_REL: &str = "apps/website/map-engine/src/data/scenario";

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
// `#[path = "tests/cases_1.rs"] mod tests;` is used throughout — so a depth calculation would be
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
const RULE4_RE: &str = r"crate::(camera|diagnostics|doll|frame|io|overlay|spatial|streaming|world|data::store)\b|website_graphics_engine|\bsuper::(super::)*(store|camera|doll|frame|io|overlay|spatial|streaming|world)\b";

/// Rule 7's `data` side — the authored document may name `crate::data` and nothing else.
const RULE7_DATA_RE: &str = r"crate::(camera|diagnostics|doll|frame|io|overlay|spatial|streaming|world)\b|website_graphics_engine|\bsuper::(super::)*(camera|doll|frame|io|overlay|spatial|streaming|world)\b";

/// Rule 7's `world` side — the static world may name neither the document nor the CRDT crate.
///
/// `yrs::` and not `\byrs\b`: the bare word would match prose ("3 yrs"), and a matcher that fires
/// on prose gets suppressed. Every real shape is a path — `use yrs::Doc`, `-> yrs::TransactionMut`,
/// `yrs::Transact::transact` — so the `::` is free precision, not a loophole.
const RULE7_WORLD_RE: &str = r"crate::data\b|\byrs::|\bsuper::(super::)*data\b";

/// Rule 4's pinned residue — file, exact count, and why it does not reach the `website-api` build.
///
/// Read the module docs before adding a row. Both entries are `#[cfg(feature = "store")]` test
/// code, and `website-api` links the `scenario` feature alone, so neither is compiled by the build
/// this rule protects. An *ungated* import of the store from `data/scenario/` would satisfy this
/// pin's count and still be wrong — which is why the pin carries the reason and not just a number.
const RULE4_PIN: &[(&str, usize, &str)] = &[
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

const RULE1_HEAD: &str =
    "==> engine-layers rule 1 — apps/website/graphics-engine must not import website_map_engine";
const RULE2_HEAD: &str =
    "==> engine-layers rule 2 — no map noun in a declared name under apps/website/graphics-engine";
const RULE3A_HEAD: &str = "==> engine-layers rule 3a — only the enumerated packet boundary may \
     name website_graphics_engine::frame under apps/website/map-engine/src";
const RULE3B_HEAD: &str = "==> engine-layers rule 3b — no GPU-resource module of \
     website-graphics-engine named under apps/website/map-engine/src";
const RULE4_HEAD: &str = "==> engine-layers rule 4 — apps/website/map-engine/src/data/scenario \
     imports nothing outside itself";
const RULE7_HEAD: &str = "==> engine-layers rule 7 — the static world and the authored document \
     share nothing under apps/website/map-engine/src";

const RULE1_TAIL: &[&str] = &[
    "      The arrow runs map-engine -> graphics-engine and only that way. Compute it in",
    "      map-engine and hand the result over as a FramePacket / DrawBatch / TextRun",
    "      (ENGINE_SPLIT_PROGRAM.md §1 \"Dependency direction — non-negotiable\").",
];
const RULE2_TAIL: &[&str] = &[
    "      graphics-engine is a renderer and may name geometry and GPU handles only. A map",
    "      noun in a declared name means domain logic came back across the wall — move the",
    "      decision to map-engine and leave the packing here (ENGINE_SPLIT_PROGRAM.md §5 rule 2).",
];
const RULE3A_TAIL: &[&str] = &[
    "      The frame vocabulary is re-exported from map-engine/src/frame/mod.rs, enumerated.",
    "      Write `use crate::frame::DrawBatch;` — the point of the boundary is that the whole",
    "      graphics interface reads as one list (ENGINE_SPLIT_PROGRAM.md §5 rule 3a, §2C.1 Kind C).",
];
const RULE3B_TAIL: &[&str] = &[
    "      device / pipeline / shaders / text::gpu / r#loop create and own GPU resources, and",
    "      that is graphics-engine's job — map-engine receives already-built handles. Relocate",
    "      the construction; do not add a row to RULE3B_PIN (ENGINE_SPLIT_PROGRAM.md §5 rule 3b).",
];
const RULE4_TAIL: &[&str] = &[
    "      website-api links this crate at the `scenario` feature alone — that is why its tree",
    "      carries no wgpu, png, rkyv or flate2. One import here drags a whole tier into an HTTP",
    "      server. Pass the value in as an argument (ENGINE_SPLIT_PROGRAM.md §5 rule 4).",
];
const RULE7_TAIL: &[&str] = &[
    "      world/ is streamed, immutable and never persisted; data/ is authored, undoable and",
    "      persisted. They share the spatial index and nothing else — a chunk id in data/ or a",
    "      document handle in world/ fuses them back together (ENGINE_SPLIT_PROGRAM.md §2D).",
];
const PROBE_FAIL: &[&str] = &[
    "FAIL: matcher self-probe returned no match over a subject it must match.",
    "      The search engine is broken. A check that cannot run is not a pass.",
];
const NOTHING_TAIL: &[&str] = &[
    "      An engine-layer check with no inputs is not a pass: either the crate moved and this",
    "      gate is scanning a husk, or the walk is broken. Both are red.",
];

pub fn verify_engine_layers(repo_root: &Path) -> Result<u8> {
    let (code, out) = run(repo_root);
    for line in out {
        println!("{line}");
    }
    Ok(code)
}

/// Push a fixed block. `""` is a deliberate blank line, which `str::lines` would swallow.
fn say(o: &mut Vec<String>, lines: &[&str]) {
    o.extend(lines.iter().map(|s| (*s).to_string()));
}

/// `path:line:text` — [`Hit::rendered`]'s shape, but repo-relative.
///
/// `walk_files` is handed absolute roots, so `rendered()` would print this machine's checkout
/// prefix. Every path in this gate's output names a file a reader has to go and edit, and the
/// relative form is the one that pastes into an editor and into a test assertion unchanged.
fn rel(repo_root: &Path, hit: &Hit) -> String {
    let p = hit.path.strip_prefix(repo_root).unwrap_or(&hit.path);
    format!("{}:{}:{}", p.display(), hit.line_no, hit.line)
}

/// Build output is not source.
///
/// A stray `apps/website/graphics-engine/src/target-container/` (536 MB of wasm artifacts, one
/// generated `thiserror` `private.rs` among them) exists in this checkout today, and a gate that
/// reported on a dependency's generated code would be reporting on code nobody in this repo wrote.
/// `.gitignore` covers `/target/` and `target-*/` anywhere in the tree, so a pruned directory can
/// never hold a tracked file — the prune removes noise, not coverage. Only components *below*
/// `repo_root` are considered, or a checkout that happened to live under `~/target-x` would prune
/// the entire repository and the gate would go vacuously green.
fn is_source(repo_root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(repo_root).unwrap_or(path);
    !rel.components().any(|c| {
        let n = c.as_os_str().to_string_lossy();
        n == "target" || n.starts_with("target-")
    })
}

/// Group `hits` by repo-relative file and judge them against an enumerated pin.
///
/// Returns `(findings, lines)` — findings is what a reader has to go and fix, lines is how tall
/// the report of it is, and those are different numbers because one finding prints many lines.
///
/// Rules 3a and 3b are the same judgement over two different matchers, so they are the same
/// code. Three ways to fail and all three are load-bearing:
///
/// * an **unpinned** file matched at all — the rule's actual subject;
/// * a **pinned** file whose count moved in either direction — a pin that no longer describes
///   the tree has quietly stopped meaning what it says, whether it grew or shrank;
/// * a pinned file that **no longer matches at all**, which never reaches the first loop because
///   it is not in `per_file` — so the pin is also checked from its own side. This is the arm that
///   catches a renamed or deleted directory, i.e. the case where "no violations" and "nothing
///   left to look at" would otherwise be indistinguishable.
fn against_pin(
    repo_root: &Path,
    hits: &[Hit],
    pin: &[(&str, usize, &str)],
) -> (usize, Vec<String>) {
    let mut per_file: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for h in hits {
        let p = h.path.strip_prefix(repo_root).unwrap_or(&h.path);
        per_file
            .entry(p.display().to_string())
            .or_default()
            .push(rel(repo_root, h));
    }
    let mut bad: Vec<String> = Vec::new();
    let mut findings = 0usize;
    for (file, lines) in &per_file {
        match pin.iter().find(|(f, _, _)| f == file) {
            None => {
                findings += 1;
                bad.push(format!("  unpinned file — {} site(s):", lines.len()));
                bad.extend(lines.iter().map(|l| format!("    {l}")));
            }
            Some((_, want, _)) if *want != lines.len() => {
                findings += 1;
                bad.push(format!(
                    "  {file}: pinned at {want} site(s), found {} — update the pin.",
                    lines.len()
                ));
                bad.extend(lines.iter().map(|l| format!("    {l}")));
            }
            Some(_) => {}
        }
    }
    for (file, want, _) in pin {
        if !per_file.contains_key(*file) {
            findings += 1;
            bad.push(format!(
                "  {file}: pinned at {want} site(s), found 0 — the pin is stale, delete the row."
            ));
        }
    }
    (findings, bad)
}

/// The subset of `files` sitting under one repo-relative directory.
///
/// [`Path::starts_with`] compares whole components, so `src/data` does not capture a sibling
/// `src/database` — the same property the `\b` gives the matchers, applied to the walk.
fn under(repo_root: &Path, files: &[PathBuf], rel: &str) -> Vec<PathBuf> {
    let root = repo_root.join(rel);
    files
        .iter()
        .filter(|p| p.starts_with(&root))
        .cloned()
        .collect()
}

/// Compile a matcher and prove it over subjects whose answers are known, before it judges
/// anything.
///
/// Both lists are load-bearing and the second one more than the first. A matcher that fails to
/// fire is a gate that passes vacuously; a matcher that fires on the spelling every call site is
/// *supposed* to use is a rule nobody can satisfy, and an unsatisfiable rule gets deleted. Rules
/// 1, 2, 3a and 3b spell their probes out inline because they predate this helper and their
/// refusal strings differ; the three added with rules 4 and 7 share one shape, so they share one
/// function rather than three more copies of the same nine-line match.
fn probed(
    o: &mut Vec<String>,
    what: &str,
    re: &str,
    must: &[&str],
    must_not: &[&str],
) -> Result<Pattern, (u8, Vec<String>)> {
    let p = match Pattern::regex(re) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return Err(refuse(o, what, cause));
        }
    };
    for (subject, want) in must
        .iter()
        .map(|s| (s, true))
        .chain(must_not.iter().map(|s| (s, false)))
    {
        match gate::probe_str(&p, subject) {
            Ok(got) if got == want => {}
            Ok(_) => {
                say(o, PROBE_FAIL);
                return Err((1, std::mem::take(o)));
            }
            Err(cause) => return Err(refuse(o, what, cause)),
        }
    }
    Ok(p)
}

fn refuse(o: &mut Vec<String>, what: &str, cause: NotRun) -> (u8, Vec<String>) {
    o.push(Verdict::did_not_run(what, Kind::Ban, cause).to_string());
    say(o, &["", "ENGINE-LAYERS: FAIL (did not run)"]);
    // 2, not 1: "the wall is breached" and "I never read the crate" are different operator
    // actions, and phase 2 will move these paths on purpose.
    (2, std::mem::take(o))
}

/// The gate proper, writing into a sink so the tests assert on exact bytes instead of scraping
/// stdout — the [`crate::gate_route_tags`] shape.
fn run(repo_root: &Path) -> (u8, Vec<String>) {
    let mut o: Vec<String> = Vec::new();

    // Probe the matcher over a subject whose answer is known, BEFORE it decides anything. The
    // engine is compiled in, so "tool absent" is unreachable — but "the matcher works" is still a
    // claim, and `probe_str` returning a `Result` forces the dead arm to be written down.
    let decl = match Pattern::regex(DECL_RE).and_then(Pattern::case_insensitive) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return refuse(&mut o, "engine-layers rule 2 pattern", cause);
        }
    };
    match gate::probe_str(&decl, "pub struct TerrainBlob;") {
        Ok(true) => {}
        Ok(false) => {
            say(&mut o, PROBE_FAIL);
            return (1, o);
        }
        Err(cause) => return refuse(&mut o, "engine-layers self-probe", cause),
    }

    let vocab = match Pattern::regex(FRAME_VOCAB_RE) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return refuse(&mut o, "engine-layers rule 3a pattern", cause);
        }
    };
    // Three subjects. The positive proves it fires on the import shape; `crate::frame` proves it
    // does NOT fire on the spelling every call site is supposed to use, which is the one false
    // positive that would make the rule unachievable; `::frames` proves the `\b`, because a pin
    // that can be widened by appending a letter is not a pin.
    match gate::probe_str(&vocab, "use website_graphics_engine::frame::DrawBatch;") {
        Ok(true) => {}
        Ok(false) => {
            say(&mut o, PROBE_FAIL);
            return (1, o);
        }
        Err(cause) => return refuse(&mut o, "engine-layers rule 3a self-probe", cause),
    }
    for subject in [
        "use crate::frame::DrawBatch;",
        "use website_graphics_engine::frames::x;",
    ] {
        match gate::probe_str(&vocab, subject) {
            Ok(false) => {}
            Ok(true) => {
                say(&mut o, PROBE_FAIL);
                return (1, o);
            }
            Err(cause) => return refuse(&mut o, "engine-layers rule 3a self-probe", cause),
        }
    }

    let gpu = match Pattern::regex(GPU_MODULE_RE) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return refuse(&mut o, "engine-layers rule 3b pattern", cause);
        }
    };
    // Two subjects, not one: the positive proves the matcher fires, and `::pipelines` proves the
    // `\b` is real. A pin that can be widened by adding an `s` is not a pin.
    match gate::probe_str(
        &gpu,
        "use website_graphics_engine::text::gpu::create_text_atlas;",
    ) {
        Ok(true) => {}
        Ok(false) => {
            say(&mut o, PROBE_FAIL);
            return (1, o);
        }
        Err(cause) => return refuse(&mut o, "engine-layers rule 3b self-probe", cause),
    }
    match gate::probe_str(&gpu, "use website_graphics_engine::pipelines::x;") {
        Ok(false) => {}
        Ok(true) => {
            say(&mut o, PROBE_FAIL);
            return (1, o);
        }
        Err(cause) => return refuse(&mut o, "engine-layers rule 3b self-probe", cause),
    }

    // Rule 4. The negatives are the ones that matter: `crate::data::scenario` is the spelling
    // every legitimate line in this tree uses, `data::store_of_record` proves the `\b` on the
    // longest alternative, and `std::io` proves the matcher is anchored on `crate::` and not on
    // the module name — a rule that banned the standard library's `io` would be deleted by
    // whoever hit it first.
    let scenario_iso = match probed(
        &mut o,
        "engine-layers rule 4 pattern",
        RULE4_RE,
        &[
            "use crate::data::store::MissionDocCore;",
            "use crate::streaming::loaders::chunk::WorldChunk;",
            "    let b = crate::io::archives::codec::to_bytes(&v);",
            "use website_graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;",
            "use super::super::super::store::MissionDocCore;",
        ],
        &[
            "use crate::data::scenario::compile::compile_payload;",
            "use crate::data::store_of_record::Row;",
            "use std::io::Write;",
            "use serde_json::Value;",
            "use super::super::diagnostics::render_authored;",
        ],
    ) {
        Ok(p) => p,
        Err(r) => return r,
    };

    // Rule 7, `data` side. `crate::worldgen` is the `\b` pair — a wall that can be walked through
    // by appending three letters to a module name is not a wall.
    let data_side = match probed(
        &mut o,
        "engine-layers rule 7 data pattern",
        RULE7_DATA_RE,
        &[
            "use crate::world::terrain::dem::grid::DemVectorGrid;",
            "use crate::streaming::scheduler::state::WorldResidency;",
            "    let hit = crate::spatial::indexing::picking::pick(qx, qy);",
            "use website_graphics_engine::draw::instances::QuadInstance;",
            "use super::super::super::streaming::loaders::chunk::WorldChunk;",
        ],
        &[
            "use crate::data::store::MissionDocCore;",
            "use crate::data::scenario::compile::terrain_bounds;",
            "use crate::worldgen::seed::X;",
            "// the world is streamed; this module only records what was authored",
            "use super::super::diagnostics::render_authored;",
        ],
    ) {
        Ok(p) => p,
        Err(r) => return r,
    };

    // Rule 7, `world` side. `crate::database` and the bare word "yrs" are the two false positives
    // that would make this arm noise rather than a rule.
    let world_side = match probed(
        &mut o,
        "engine-layers rule 7 world pattern",
        RULE7_WORLD_RE,
        &[
            "use crate::data::store::MissionDocCore;",
            "    let b = crate::data::scenario::compile::terrain_bounds(&t);",
            "use yrs::{Doc, Transact};",
            "fn tx(d: &yrs::Doc) -> yrs::TransactionMut<'_> { d.transact_mut() }",
            "use super::super::super::data::store::MissionDocCore;",
        ],
        &[
            "use crate::database::pool::Pool;",
            "// resurveyed 3 yrs after the original DEM pass",
            "use crate::io::archives::codec::to_bytes;",
            "use crate::world::terrain::dem::grid::DemVectorGrid;",
        ],
    ) {
        Ok(p) => p,
        Err(r) => return r,
    };

    let crate_dir = repo_root.join(CRATE_REL);
    let manifest = crate_dir.join("Cargo.toml");
    let src = crate_dir.join("src");

    let root = repo_root.to_path_buf();
    let sources = match scan::walk_files(&[&src], move |p| {
        is_source(&root, p) && p.extension().is_some_and(|e| e == "rs")
    }) {
        Ok(f) => f,
        Err(cause) => return refuse(&mut o, "engine-layers could not walk the crate", cause),
    };
    // The manifest is walked rather than read so a missing one is the same `NotRun` as a missing
    // src/ — one refusal shape for the whole gate, not two.
    let manifest_files = match scan::walk_files(&[&manifest], |_| true) {
        Ok(f) => f,
        Err(cause) => return refuse(&mut o, "engine-layers could not read the manifest", cause),
    };

    // Rule 3b's root, added beside the first rather than replacing it — rules 1 and 2 still scan
    // only graphics-engine, and this gate now has two crates it refuses to run without.
    let map_src = repo_root.join(MAP_CRATE_REL).join("src");
    let root2 = repo_root.to_path_buf();
    let map_sources = match scan::walk_files(&[&map_src], move |p| {
        is_source(&root2, p) && p.extension().is_some_and(|e| e == "rs")
    }) {
        Ok(f) => f,
        Err(cause) => return refuse(&mut o, "engine-layers could not walk the map engine", cause),
    };

    // Rules 4 and 7 scan three subtrees of the walk rules 3a/3b already do, rather than walking
    // the disk three more times. Each is its own anti-vacuity subject below: "no file under
    // `data/` names a world module" and "there is no `data/` any more" are the same sentence to a
    // matcher, and only the count tells them apart.
    let data_files = under(repo_root, &map_sources, DATA_REL);
    let world_files = under(repo_root, &map_sources, WORLD_REL);
    let scenario_files = under(repo_root, &map_sources, SCENARIO_REL);

    let scanned = format!(
        "  scanned {} .rs file(s) + {CRATE_REL}/Cargo.toml, {} .rs file(s) under \
         {MAP_CRATE_REL}/src — of those {} under data/ ({} under data/scenario) and {} under world/",
        sources.len(),
        map_sources.len(),
        data_files.len(),
        scenario_files.len(),
        world_files.len()
    );
    for (n, root) in [
        (sources.len(), format!("{CRATE_REL}/src")),
        (map_sources.len(), format!("{MAP_CRATE_REL}/src")),
        (data_files.len(), DATA_REL.to_string()),
        (world_files.len(), WORLD_REL.to_string()),
        (scenario_files.len(), SCENARIO_REL.to_string()),
    ] {
        if n == 0 {
            o.push(format!(
                "FAIL: engine-layers walked 0 .rs file(s) under {root} — refusing a vacuous pass."
            ));
            say(&mut o, NOTHING_TAIL);
            o.push("ENGINE-LAYERS: FAIL (no inputs)".to_string());
            return (1, o);
        }
    }

    // ── rule 1 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE1_HEAD.to_string());
    let path_use = Pattern::literal(MAP_ENGINE_PATH);
    let mut breaches: Vec<String> = Vec::new();
    match scan::grep_lines(&path_use, &sources) {
        Ok(hits) => breaches.extend(hits.iter().map(|h| rel(repo_root, h))),
        Err(cause) => return refuse(&mut o, "engine-layers rule 1 scan", cause),
    }
    // The manifest arm closes the one hole the source arm has: `[dependencies] r = { package =
    // "website-map-engine" }` renames the crate, and every `use r::…` then spells something this
    // gate has never heard of. A `#` line is a comment — naming the other crate in prose is not a
    // dependency edge.
    match scan::grep_lines(&Pattern::literal(MAP_ENGINE_PKG), &manifest_files) {
        Ok(hits) => breaches.extend(
            hits.iter()
                .filter(|h| !h.line.trim_start().starts_with('#'))
                .map(|h| rel(repo_root, h)),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 1 manifest scan", cause),
    }
    if breaches.is_empty() {
        o.push("  OK (none)".to_string());
    } else {
        o.push("FAIL: graphics-engine reaches back into the map engine:".to_string());
        o.extend(breaches.iter().map(|b| format!("  {b}")));
        say(&mut o, RULE1_TAIL);
    }

    // ── rule 2 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE2_HEAD.to_string());
    let nouns: Vec<String> = match scan::grep_lines(&decl, &sources) {
        Ok(hits) => hits.iter().map(|h| rel(repo_root, h)).collect(),
        Err(cause) => return refuse(&mut o, "engine-layers rule 2 scan", cause),
    };
    if nouns.is_empty() {
        o.push("  OK (none)".to_string());
    } else {
        o.push("FAIL: map nouns declared inside the pure renderer:".to_string());
        o.extend(nouns.iter().map(|n| format!("  {n}")));
        say(&mut o, RULE2_TAIL);
    }

    // ── rule 3a ──────────────────────────────────────────────────────────────────────────────
    o.push(RULE3A_HEAD.to_string());
    let vocab_hits = match scan::grep_lines(&vocab, &map_sources) {
        Ok(hits) => hits,
        Err(cause) => return refuse(&mut o, "engine-layers rule 3a scan", cause),
    };
    let (vocab_findings, vocab_bad) = against_pin(repo_root, &vocab_hits, RULE3A_PIN);
    let vocab_total: usize = RULE3A_PIN.iter().map(|(_, n, _)| *n).sum();
    if vocab_bad.is_empty() {
        o.push(format!(
            "  OK — {vocab_total} pinned site(s) in {} file(s), 0 unpinned. The crate's whole \
             graphics interface, enumerated:",
            RULE3A_PIN.len()
        ));
        for (file, n, why) in RULE3A_PIN {
            o.push(format!("    {file} ({n}) — {why}"));
        }
    } else {
        o.push("FAIL: the frame vocabulary is named outside the packet boundary:".to_string());
        o.extend(vocab_bad.iter().cloned());
        say(&mut o, RULE3A_TAIL);
    }

    // ── rule 3b ──────────────────────────────────────────────────────────────────────────────
    o.push(RULE3B_HEAD.to_string());
    let gpu_hits = match scan::grep_lines(&gpu, &map_sources) {
        Ok(hits) => hits,
        Err(cause) => return refuse(&mut o, "engine-layers rule 3b scan", cause),
    };
    let (gpu_findings, gpu_bad) = against_pin(repo_root, &gpu_hits, RULE3B_PIN);
    let pinned_total: usize = RULE3B_PIN.iter().map(|(_, n, _)| *n).sum();
    if gpu_bad.is_empty() {
        o.push(format!(
            "  OK — {pinned_total} pinned site(s), 0 unpinned. Every one is `RenderEngine` not \
             having crossed:"
        ));
        for (file, n, why) in RULE3B_PIN {
            o.push(format!("    {file} ({n}) — {why}"));
        }
    } else {
        o.push("FAIL: GPU-resource modules named inside the map engine:".to_string());
        o.extend(gpu_bad.iter().cloned());
        say(&mut o, RULE3B_TAIL);
    }

    // ── rule 4 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE4_HEAD.to_string());
    let iso_hits = match scan::grep_lines(&scenario_iso, &scenario_files) {
        Ok(hits) => hits,
        Err(cause) => return refuse(&mut o, "engine-layers rule 4 scan", cause),
    };
    let (iso_findings, iso_bad) = against_pin(repo_root, &iso_hits, RULE4_PIN);
    let iso_total: usize = RULE4_PIN.iter().map(|(_, n, _)| *n).sum();
    if iso_bad.is_empty() {
        o.push(format!(
            "  OK — {iso_total} pinned site(s) in {} file(s), 0 unpinned. Production code reaches \
             outside data/scenario nowhere; the pinned residue is cfg-gated test code:",
            RULE4_PIN.len()
        ));
        for (file, n, why) in RULE4_PIN {
            o.push(format!("    {file} ({n}) — {why}"));
        }
    } else {
        o.push("FAIL: the authored mission reaches outside its own tree:".to_string());
        o.extend(iso_bad.iter().cloned());
        say(&mut o, RULE4_TAIL);
    }

    // ── rule 7 ───────────────────────────────────────────────────────────────────────────────
    //
    // One rule, two directions, one findings list — a breach in either direction is the same
    // wall coming down, and reporting it as two rules would let half of it read green.
    o.push(RULE7_HEAD.to_string());
    let mut wall: Vec<String> = Vec::new();
    match scan::grep_lines(&data_side, &data_files) {
        Ok(hits) => wall.extend(
            hits.iter()
                .map(|h| format!("  data/ names the world — {}", rel(repo_root, h))),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 7 data scan", cause),
    }
    match scan::grep_lines(&world_side, &world_files) {
        Ok(hits) => wall.extend(
            hits.iter()
                .map(|h| format!("  world/ names the document — {}", rel(repo_root, h))),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 7 world scan", cause),
    }
    if wall.is_empty() {
        o.push(format!(
            "  OK — 0 site(s) in both directions: {} .rs file(s) under data/ name no world \
             module, {} under world/ name neither crate::data nor yrs.",
            data_files.len(),
            world_files.len()
        ));
    } else {
        o.push("FAIL: the world/data wall is breached:".to_string());
        o.extend(wall.iter().cloned());
        say(&mut o, RULE7_TAIL);
    }

    o.push(scanned);
    if breaches.is_empty()
        && nouns.is_empty()
        && vocab_bad.is_empty()
        && gpu_bad.is_empty()
        && iso_bad.is_empty()
        && wall.is_empty()
    {
        o.push("ENGINE-LAYERS: PASS".to_string());
        return (0, o);
    }
    o.push(format!(
        "ENGINE-LAYERS: FAIL — {} wall breach(es), {} map-noun declaration(s), \
         {vocab_findings} frame-vocab finding(s), {gpu_findings} GPU-module finding(s), \
         {iso_findings} scenario-isolation finding(s), {} world/data finding(s)",
        breaches.len(),
        nouns.len(),
        wall.len(),
    ));
    (1, o)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A clean two-file renderer. Every banned word appears here as PROSE or as an asset path,
    /// which is the false-positive case rule 2 has to survive to be usable.
    const CLEAN: &str = r#"
//! The draw belt. A submission from the map engine arrives as a DrawBatch; this file never
//! decides which symbol to draw, only how to pack it.
const SHADER: &str = include_str!("shaders/terrain.wgsl");
pub struct DrawBatch {
    pub lane: u32,
}
pub fn pack_batch(b: &DrawBatch) -> u32 {
    b.lane
}
"#;
    const MANIFEST: &str =
        "[package]\nname = \"website-graphics-engine\"\n\n[dependencies]\nbytemuck = \"1\"\n";

    /// The map engine's pinned residue, at exactly the counts [`RULE3B_PIN`] and [`RULE3A_PIN`]
    /// claim — 3 GPU-module sites and 2 more in `pump.rs`, and 8 frame-vocabulary re-exports.
    /// The fixture mirrors the real shape rather than stubbing the pins out, so the pins
    /// themselves are under test: change a count in either table and these fixtures stop
    /// matching it. The prose line below is the one that names `r#loop` and no `frame` path —
    /// that asymmetry is real, and deliberate, in the file this mirrors.
    const MAP_FRAME_MOD: &str = "\
pub use website_graphics_engine::device::buffers;
pub use website_graphics_engine::pipeline as pipelines;
/// Re-export `website_graphics_engine::r#loop::{FrameTarget, RafPump}`.
pub use pump::{FrameTarget, RafPump};
pub use website_graphics_engine::frame::damage;
pub use website_graphics_engine::frame::packet;
pub use website_graphics_engine::frame::present;
pub use website_graphics_engine::frame::CameraUniform;
pub use website_graphics_engine::frame::{BindGroupId, LaneId, PipelineId};
pub use website_graphics_engine::frame::{DrawBatch, DrawPayload, FramePacket, IndirectDraw};
pub use website_graphics_engine::frame::{IndexedMesh, InstanceBuffer, VertexStream};
pub use website_graphics_engine::frame::{
    GlyphAtlasGpu, TextAtlasGpu, TextRun, create_glyph_atlas, create_text_atlas,
};
";
    const MAP_FRAME_PUMP: &str = "\
pub use website_graphics_engine::r#loop::FrameTarget;
pub use website_graphics_engine::r#loop::RafPump;
";

    /// The authored document, clean: `data/` names `crate::data` and nothing else in the crate.
    /// The `store -> scenario` line is deliberate — that direction is legal and rule 4 does not
    /// scan `data/store/`, so a matcher that fired on it would be a false positive on the very
    /// first file.
    const MAP_DATA_STORE: &str = "\
use crate::data::store::{MissionDocCore, SlotSoa};
let b = crate::data::scenario::compile::terrain_bounds(&terrain);
";
    /// The authored mission, clean: `data/scenario/` names only itself.
    const MAP_SCENARIO: &str = "\
use crate::data::scenario::ast::entities;
use crate::data::scenario::validate::{Finding, Severity};
";
    /// Rule 4's two pinned sites, one per file, both cfg-gated exactly as the real ones are.
    const MAP_SCENARIO_FLATTEN_TEST: &str = "\
#[cfg(feature = \"store\")]
fn vehicles_from_writer_json_roundtrip() -> serde_json::Value {
    use crate::data::store::MissionDocCore;
}
";
    const MAP_SCENARIO_PAYLOAD_TEST: &str = "\
#[cfg(feature = \"store\")]
#[test]
fn briefing_prose_round_trips_through_the_document_core() {
    use crate::data::store::MissionDocCore;
}
";
    /// The static world, clean: it names the world, the format layer and the streamer, and never
    /// the document. Those three are what a `world/` file legitimately imports.
    const MAP_WORLD: &str = "\
use crate::world::terrain::dem::sampling::uint16_to_meters;
use crate::io::archives::codec::to_bytes;
use crate::streaming::scheduler::state::WorldResidency;
use crate::spatial::bvh::traversal::Bvh;
";

    struct Repo(PathBuf);
    impl Repo {
        fn new(name: &str) -> Repo {
            let mut p = std::env::temp_dir();
            p.push(format!("tbd-el-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(p.join("apps/website/graphics-engine/src/draw")).unwrap();
            let r = Repo(p);
            r.src("draw/mod.rs", CLEAN);
            r.manifest(MANIFEST);
            r.map("frame/mod.rs", MAP_FRAME_MOD);
            r.map("frame/pump.rs", MAP_FRAME_PUMP);
            // Rules 4 and 7's roots. They are seeded on every fixture, not only the tests that
            // exercise them, because an absent root is a hard FAIL — which is the point.
            r.map("data/store/rows/merge.rs", MAP_DATA_STORE);
            r.map("data/scenario/compiler/flatten/mod.rs", MAP_SCENARIO);
            r.map(
                "data/scenario/compiler/flatten/tests/mod.rs",
                MAP_SCENARIO_FLATTEN_TEST,
            );
            r.map(
                "data/scenario/compiler/payload/tests/cases_1.rs",
                MAP_SCENARIO_PAYLOAD_TEST,
            );
            r.map("world/terrain/dem/grid.rs", MAP_WORLD);
            r
        }
        /// Write a file under the map engine — rule 3b's root.
        fn map(&self, rel: &str, body: &str) {
            let p = self.0.join("apps/website/map-engine/src").join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        fn src(&self, rel: &str, body: &str) {
            let p = self.0.join("apps/website/graphics-engine/src").join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        fn manifest(&self, body: &str) {
            let p = self.0.join("apps/website/graphics-engine/Cargo.toml");
            std::fs::write(p, body).unwrap();
        }
        /// Run; assert the exit code and every expected line; hand back the joined output.
        fn expect(&self, code: u8, want: &[&str]) -> String {
            let (got, out) = super::run(&self.0);
            let all = out.join("\n");
            assert_eq!(got, code, "{all}");
            for w in want {
                assert!(all.contains(w), "missing {w:?} in:\n{all}");
            }
            all
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// THE GREEN ARM — and with it the two false positives that would make this gate unusable:
    /// the word "submission" in a doc comment, and a `.wgsl` asset path named after a map feature.
    #[test]
    fn a_pure_renderer_passes_and_prose_does_not_trip_it() {
        let all = Repo::new("clean").expect(
            0,
            &[
                RULE1_HEAD,
                RULE2_HEAD,
                RULE3A_HEAD,
                RULE3B_HEAD,
                RULE4_HEAD,
                RULE7_HEAD,
                "  OK (none)",
                "  OK — 8 pinned site(s) in 1 file(s), 0 unpinned.",
                "  OK — 5 pinned site(s), 0 unpinned.",
                "  OK — 2 pinned site(s) in 2 file(s), 0 unpinned.",
                "  OK — 0 site(s) in both directions: 4 .rs file(s) under data/ name no world \
                 module, 1 under world/ name neither crate::data nor yrs.",
                "  scanned 1 .rs file(s) + apps/website/graphics-engine/Cargo.toml, \
                 7 .rs file(s) under apps/website/map-engine/src — of those 4 under data/ \
                 (3 under data/scenario) and 1 under world/",
                "ENGINE-LAYERS: PASS",
            ],
        );
        assert!(!all.contains("FAIL"), "{all}");
    }

    /// RULE 1, RED — an import of the map engine, reported with its exact line.
    #[test]
    fn importing_the_map_engine_fails() {
        let r = Repo::new("rule1");
        r.src("draw/bad.rs", "use website_map_engine::x;\n");
        r.expect(
            1,
            &[
                "FAIL: graphics-engine reaches back into the map engine:",
                "  apps/website/graphics-engine/src/draw/bad.rs:1:use website_map_engine::x;",
                RULE1_TAIL[0],
                "ENGINE-LAYERS: FAIL — 1 wall breach(es), 0 map-noun declaration(s), \
                 0 frame-vocab finding(s), 0 GPU-module finding(s), \
                 0 scenario-isolation finding(s), 0 world/data finding(s)",
            ],
        );
    }

    /// RULE 1, the manifest arm: a dependency edge is a breach even with no `use` anywhere, and a
    /// `#` comment naming the other crate is not.
    #[test]
    fn a_dependency_edge_is_a_breach_and_a_comment_is_not() {
        let r = Repo::new("rule1-manifest");
        r.manifest(&format!(
            "{MANIFEST}me = {{ package = \"website-map-engine\" }}\n"
        ));
        r.expect(
            1,
            &[
                "  apps/website/graphics-engine/Cargo.toml:6:me = { package = \"website-map-engine\" }",
                "ENGINE-LAYERS: FAIL — 1 wall breach(es), 0 map-noun declaration(s), \
                 0 frame-vocab finding(s), 0 GPU-module finding(s), \
                 0 scenario-isolation finding(s), 0 world/data finding(s)",
            ],
        );
        r.manifest(&format!(
            "{MANIFEST}# website-map-engine depends on us, never the reverse\n"
        ));
        r.expect(0, &["ENGINE-LAYERS: PASS"]);
    }

    /// RULE 2, RED — a type name and a fn name, each reported with its exact line.
    #[test]
    fn map_nouns_in_declared_names_fail() {
        let r = Repo::new("rule2");
        r.src(
            "draw/bad.rs",
            "pub struct TerrainBlob;\npub fn pack_mission() {}\n",
        );
        r.expect(
            1,
            &[
                "FAIL: map nouns declared inside the pure renderer:",
                "  apps/website/graphics-engine/src/draw/bad.rs:1:pub struct TerrainBlob;",
                "  apps/website/graphics-engine/src/draw/bad.rs:2:pub fn pack_mission() {}",
                RULE2_TAIL[0],
                "ENGINE-LAYERS: FAIL — 0 wall breach(es), 2 map-noun declaration(s), \
                 0 frame-vocab finding(s), 0 GPU-module finding(s), \
                 0 scenario-isolation finding(s), 0 world/data finding(s)",
            ],
        );
    }

    /// The whole shape of rule 2, one line at a time. `submit_mission` is the pair that proves
    /// `\w*` is doing the work: the same eight letters pass as a word and fail as a name.
    #[test]
    fn rule_two_matches_declarations_and_not_mentions() {
        let p = Pattern::regex(DECL_RE)
            .and_then(Pattern::case_insensitive)
            .unwrap();
        for bad in [
            "pub struct TerrainBlob;",
            "enum SymbologyKind { A }",
            "trait MissionSink {}",
            "type ArmaId = u32;",
            "pub fn submit_mission() {}",
            "const ORBAT_SLOTS: u8 = 4;",
            "static TERRAIN_LOD: u8 = 2;",
            "mod symbology;",
            "    pub(crate) fn pack_mission_x(v: u32) -> u32 { v }",
        ] {
            assert!(p.is_match(bad), "should fail the gate: {bad}");
        }
        for ok in [
            "/// a submission from the map engine arrives here",
            "const SHADER: &str = include_str!(\"shaders/terrain.wgsl\");",
            "// symbology decides which symbol; we only pack it",
            "let mission = 1;",
            "pub struct DrawBatch;",
            "fn pack_icon_instance(v: u32) -> u32 { v }",
            "// the type for terrain lives one crate over",
        ] {
            assert!(!p.is_match(ok), "should pass the gate: {ok}");
        }
    }

    /// THE ANTI-VACUITY CASES. Neither "the crate is gone" nor "the crate is empty" may read as a
    /// clean wall — a moved directory is exactly what phase 2 does on purpose.
    #[test]
    fn inputs_that_were_never_read_do_not_pass() {
        let (code, out) = super::run(Path::new("/nonexistent/tbd-engine-layers/repo"));
        let all = out.join("\n");
        assert_eq!(code, 2, "a check that never ran must not exit 0:\n{all}");
        assert!(all.contains(CRATE_REL) && !all.contains("PASS"), "{all}");
        assert!(all.contains("ENGINE-LAYERS: FAIL (did not run)"), "{all}");

        // src/ present but empty of Rust: `walk_files` returns Ok(vec![]) and every grep finds
        // nothing, which is the shape that would report a clean wall over zero bytes of source.
        let r = Repo::new("empty");
        std::fs::remove_file(r.0.join("apps/website/graphics-engine/src/draw/mod.rs")).unwrap();
        r.expect(
            1,
            &[
                "FAIL: engine-layers walked 0 .rs file(s) under apps/website/graphics-engine/src",
                NOTHING_TAIL[0],
                "ENGINE-LAYERS: FAIL (no inputs)",
            ],
        );

        // Rule 3b's root has the same hole, and it is the one phase 2B reshaped on purpose.
        let r = Repo::new("empty-map");
        std::fs::remove_dir_all(r.0.join("apps/website/map-engine/src")).unwrap();
        std::fs::create_dir_all(r.0.join("apps/website/map-engine/src")).unwrap();
        r.expect(
            1,
            &[
                "FAIL: engine-layers walked 0 .rs file(s) under apps/website/map-engine/src",
                "ENGINE-LAYERS: FAIL (no inputs)",
            ],
        );
    }

    /// RULE 7'S ANTI-VACUITY CASE, and the reason the rule prints two counts.
    ///
    /// Rule 7 is the one rule in this gate whose green state is **zero sites**, which makes it the
    /// one most able to go vacuously green: a matcher that finds nothing over a tree that is not
    /// there reports exactly what a clean wall reports. Phase 2 moved `data/` and `world/` into
    /// existence and phase 3 moves more code into them, so "the directory is gone" is a live
    /// outcome, not a hypothetical. Each half is removed separately — a combined check would pass
    /// if either guard existed.
    #[test]
    fn an_absent_half_of_the_wall_is_not_a_clean_wall() {
        for (name, dir) in [
            ("no-data", "apps/website/map-engine/src/data"),
            ("no-world", "apps/website/map-engine/src/world"),
        ] {
            let r = Repo::new(name);
            std::fs::remove_dir_all(r.0.join(dir)).unwrap();
            let want = format!("FAIL: engine-layers walked 0 .rs file(s) under {dir}");
            let all = r.expect(1, &[want.as_str(), "ENGINE-LAYERS: FAIL (no inputs)"]);
            assert!(
                !all.contains("ENGINE-LAYERS: PASS"),
                "a missing {dir} must never read as a clean wall:\n{all}"
            );
        }

        // And the scenario root on its own: rule 4's pin catches a vanished *file*, but a
        // vanished *tree* has to be caught by the count, because an empty walk finds no hits to
        // group and the pin's own arm would then report two stale rows instead of "no inputs".
        let r = Repo::new("no-scenario");
        std::fs::remove_dir_all(r.0.join("apps/website/map-engine/src/data/scenario")).unwrap();
        r.expect(
            1,
            &[
                "FAIL: engine-layers walked 0 .rs file(s) under \
                 apps/website/map-engine/src/data/scenario",
                "ENGINE-LAYERS: FAIL (no inputs)",
            ],
        );
    }

    /// RULE 4, RED — the authored mission importing the document store outside a `cfg`, and
    /// importing the streaming tier, which is the import that would actually cost the API build.
    #[test]
    fn the_scenario_tree_reaching_outside_itself_fails() {
        let r = Repo::new("rule4-new");
        r.map(
            "data/scenario/compiler/flatten/terrain.rs",
            "use crate::streaming::loaders::chunk::WorldChunk;\n",
        );
        r.expect(
            1,
            &[
                "FAIL: the authored mission reaches outside its own tree:",
                "  unpinned file — 1 site(s):",
                "apps/website/map-engine/src/data/scenario/compiler/flatten/terrain.rs:1:\
                 use crate::streaming::loaders::chunk::WorldChunk;",
                RULE4_TAIL[0],
                "1 scenario-isolation finding(s)",
            ],
        );
    }

    /// RULE 4, THE RATCHET — the cfg-gated residue may not grow, shrink, or move house unseen.
    #[test]
    fn the_rule_4_pin_is_a_ratchet_in_both_directions() {
        let r = Repo::new("rule4-grow");
        r.map(
            "data/scenario/compiler/flatten/tests/mod.rs",
            &format!("{MAP_SCENARIO_FLATTEN_TEST}use crate::data::store::SlotSoa;\n"),
        );
        r.expect(
            1,
            &[
                "apps/website/map-engine/src/data/scenario/compiler/flatten/tests/mod.rs: \
                 pinned at 1 site(s), found 2",
                "1 scenario-isolation finding(s)",
            ],
        );

        let r = Repo::new("rule4-shrink");
        r.map(
            "data/scenario/compiler/payload/tests/cases_1.rs",
            "// the pairing is tested from the store side now\n",
        );
        r.expect(
            1,
            &[
                "apps/website/map-engine/src/data/scenario/compiler/payload/tests/cases_1.rs: \
                 pinned at 1 site(s), found 0 — the pin is stale, delete the row.",
                "1 scenario-isolation finding(s)",
            ],
        );
    }

    /// RULE 4's matcher, one line at a time. The `ok` list is the whole point: `data/scenario`
    /// names itself on nearly every line it has, and `std::io` is one keystroke from the banned
    /// `crate::io`.
    #[test]
    fn rule_4_matches_only_what_is_outside_the_scenario_tree() {
        let p = Pattern::regex(RULE4_RE).unwrap();
        for bad in [
            "use crate::data::store::MissionDocCore;",
            "use crate::streaming::loaders::chunk::WorldChunk;",
            "use crate::world::terrain::dem::grid::DemVectorGrid;",
            "use crate::io::archives::codec::to_bytes;",
            "use crate::spatial::bvh::traversal::Bvh;",
            "use crate::overlay::lanes::LaneRole;",
            "use crate::frame::EngineHandle;",
            "use crate::camera::orbit::Orbit;",
            "use crate::diagnostics::bench::Sample;",
            "use crate::doll::pose::Pose;",
            "use website_graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;",
            // The `super::` chain, at three depths — the bypass a `crate::`-anchored matcher
            // would leave open. Depth is irrelevant to the match; the destination is what tells.
            "use super::store::MissionDocCore;",
            "use super::super::super::store::MissionDocCore;",
            "use super::super::super::super::streaming::loaders::chunk::WorldChunk;",
            "    let m = super::super::io::archives::codec::to_bytes(&v);",
        ] {
            assert!(p.is_match(bad), "should fail the gate: {bad}");
        }
        for ok in [
            "use crate::data::scenario::ast::entities;",
            "use crate::data::scenario::compile::terrain_bounds;",
            "use crate::data::store_of_record::Row;",
            "use std::io::Write;",
            "use serde_json::Value;",
            "use super::*;",
            "use super::ast::entities;",
            // The one collision: `data/scenario/compiler/flatten/diagnostics.rs` exists, so a
            // `super::` chain landing on that name is in-tree traffic and the arm leaves it out.
            "use super::super::diagnostics::render_authored;",
            "pub(super) fn merge_resident_index() {}",
            "use super::super::io_helpers::read;",
            "// the streaming tier lives above this one and must stay there",
        ] {
            assert!(!p.is_match(ok), "should pass the gate: {ok}");
        }
    }

    /// RULE 7, RED, BOTH WAYS — a chunk id reaching `data/` and a document handle reaching
    /// `world/`, in one fixture, because the rule is one wall and half of it standing is not a
    /// pass.
    #[test]
    fn the_world_data_wall_fails_in_either_direction() {
        let r = Repo::new("rule7");
        r.map(
            "data/store/rows/chunked.rs",
            "use crate::streaming::scheduler::chunk_math::Bbox;\n",
        );
        r.map(
            "world/terrain/dem/edited.rs",
            "use yrs::Doc;\nlet t = crate::data::store::MissionDocCore::new();\n",
        );
        r.expect(
            1,
            &[
                "FAIL: the world/data wall is breached:",
                "  data/ names the world — apps/website/map-engine/src/data/store/rows/\
                 chunked.rs:1:use crate::streaming::scheduler::chunk_math::Bbox;",
                "  world/ names the document — apps/website/map-engine/src/world/terrain/dem/\
                 edited.rs:1:use yrs::Doc;",
                "  world/ names the document — apps/website/map-engine/src/world/terrain/dem/\
                 edited.rs:2:let t = crate::data::store::MissionDocCore::new();",
                RULE7_TAIL[0],
                "3 world/data finding(s)",
            ],
        );
    }

    /// RULE 7's two matchers, one line at a time.
    ///
    /// The `ok` lists carry the three false positives that would each, on their own, make the
    /// rule unusable: `crate::worldgen` and `crate::database` are one suffix away from a hit, and
    /// "3 yrs" is the English word the CRDT crate is unfortunately spelled as.
    #[test]
    fn rule_7_matches_the_wall_and_not_the_legal_traffic() {
        let data = Pattern::regex(RULE7_DATA_RE).unwrap();
        for bad in [
            "use crate::world::terrain::dem::grid::DemVectorGrid;",
            "use crate::streaming::scheduler::state::WorldResidency;",
            "use crate::spatial::indexing::picking::pick;",
            "use crate::io::containers::header::ContainerHeader;",
            "use crate::overlay::lod::class_visible;",
            "use crate::frame::DrawPayload;",
            "    let c = crate::camera::ortho::Ortho::default();",
            "use website_graphics_engine::draw::instances::QuadInstance;",
            "use super::super::super::streaming::loaders::chunk::WorldChunk;",
            "use super::world::terrain::dem::grid::DemVectorGrid;",
        ] {
            assert!(data.is_match(bad), "should fail the gate: {bad}");
        }
        for ok in [
            "use crate::data::store::MissionDocCore;",
            "use crate::data::scenario::compile::terrain_bounds;",
            "use crate::worldgen::seed::X;",
            "use std::io::Write;",
            "use super::super::store::MissionDocCore;",
            "use super::super::diagnostics::render_authored;",
            "pub struct SlotSoa { pub x: Vec<f64>, pub y: Vec<f64> }",
            "// authored positions are world-space metres; that is not a chunk id",
        ] {
            assert!(!data.is_match(ok), "should pass the gate: {ok}");
        }

        let world = Pattern::regex(RULE7_WORLD_RE).unwrap();
        for bad in [
            "use crate::data::store::MissionDocCore;",
            "use crate::data::scenario::flatten::MissionMeta;",
            "    let b = crate::data::store::operations::attrs::slot_z(&d);",
            "use yrs::{Doc, Transact};",
            "fn tx(d: &yrs::Doc) -> yrs::TransactionMut<'_> { d.transact_mut() }",
            "use super::super::super::super::data::store::MissionDocCore;",
        ] {
            assert!(world.is_match(bad), "should fail the gate: {bad}");
        }
        for ok in [
            "use crate::database::pool::Pool;",
            "// resurveyed 3 yrs after the original DEM pass",
            "use crate::io::archives::codec::to_bytes;",
            "use crate::world::terrain::dem::grid::DemVectorGrid;",
            "use crate::streaming::loaders::fetch::fetch_bytes;",
            "use super::super::dem::grid::DemVectorGrid;",
            "use super::super::database_of_record::Row;",
            "pub struct DemVectorGrid { pub cells: Vec<u16> }",
        ] {
            assert!(!world.is_match(ok), "should pass the gate: {ok}");
        }
    }

    /// RULE 3a, RED — the frame vocabulary imported outside the packet boundary.
    ///
    /// Two subjects in one fixture because they are the two shapes phase 2C actually found: an
    /// import at the top of an upload belt, and the path written inline mid-expression. A rule
    /// that only saw `use` lines would have missed three of the sixteen sites it closed.
    #[test]
    fn naming_the_frame_vocabulary_outside_the_boundary_fails() {
        let r = Repo::new("rule3a-new");
        r.map(
            "world/environment/vegetation/buffers.rs",
            "use website_graphics_engine::frame::DrawPayload;\n",
        );
        r.map(
            "diagnostics/readback/scene.rs",
            "let p = website_graphics_engine::frame::FramePacket { camera };\n",
        );
        r.expect(
            1,
            &[
                "FAIL: the frame vocabulary is named outside the packet boundary:",
                "  unpinned file — 1 site(s):",
                "apps/website/map-engine/src/world/environment/vegetation/buffers.rs:1:\
                 use website_graphics_engine::frame::DrawPayload;",
                "apps/website/map-engine/src/diagnostics/readback/scene.rs:1:\
                 let p = website_graphics_engine::frame::FramePacket { camera };",
                RULE3A_TAIL[0],
                "2 frame-vocab finding(s)",
            ],
        );
    }

    /// RULE 3a, THE RATCHET — the interface list may not grow or shrink unreviewed, and the
    /// file holding it may not vanish.
    ///
    /// The third arm is 3a's real anti-vacuity guard. "Nothing in the crate names the frame
    /// vocabulary" is what a deleted or renamed `frame/` looks like from the matcher's side, and
    /// it is indistinguishable from a perfectly clean boundary unless the pin is also checked
    /// from its own direction. Phase 2 moves directories on purpose; this is the arm that means
    /// the gate says so instead of going green over a husk.
    #[test]
    fn the_rule_3a_pin_is_a_ratchet_in_both_directions() {
        let r = Repo::new("rule3a-grow");
        r.map(
            "frame/mod.rs",
            &format!("{MAP_FRAME_MOD}pub use website_graphics_engine::frame::Extra;\n"),
        );
        r.expect(
            1,
            &[
                "apps/website/map-engine/src/frame/mod.rs: pinned at 8 site(s), found 9",
                "1 frame-vocab finding(s)",
            ],
        );

        let r = Repo::new("rule3a-shrink");
        // Drop one re-export. Everything else about the file — including its three rule-3b
        // sites — stays, so this isolates the 3a count and nothing else moves.
        r.map(
            "frame/mod.rs",
            &MAP_FRAME_MOD.replace("pub use website_graphics_engine::frame::present;\n", ""),
        );
        r.expect(
            1,
            &[
                "apps/website/map-engine/src/frame/mod.rs: pinned at 8 site(s), found 7",
                "1 frame-vocab finding(s)",
            ],
        );

        let r = Repo::new("rule3a-vanished");
        r.map("frame/mod.rs", "// the boundary moved somewhere else\n");
        let all = r.expect(
            1,
            &[
                "apps/website/map-engine/src/frame/mod.rs: pinned at 8 site(s), found 0 — \
                 the pin is stale, delete the row.",
                "1 frame-vocab finding(s)",
            ],
        );
        assert!(
            !all.contains("ENGINE-LAYERS: PASS"),
            "a crate with no packet boundary at all must never read as a clean one:\n{all}"
        );
    }

    /// RULE 3a's matcher, one line at a time. `crate::frame` is the pair that matters most —
    /// it is the spelling every one of the 38 converted call sites now uses, and a matcher that
    /// fired on it would make the rule impossible to satisfy rather than merely noisy.
    #[test]
    fn rule_3a_matches_the_frame_path_and_not_the_crate_local_one() {
        let p = Pattern::regex(FRAME_VOCAB_RE).unwrap();
        for bad in [
            "use website_graphics_engine::frame::DrawBatch;",
            "pub use website_graphics_engine::frame::{BindGroupId, LaneId, PipelineId};",
            "pub use website_graphics_engine::frame::damage;",
            "    camera: website_graphics_engine::frame::CameraUniform::new(mvp),",
            "pub fn lane_id(r: R) -> website_graphics_engine::frame::LaneId {",
            "//! the opaque `website_graphics_engine::frame::LaneId`",
        ] {
            assert!(p.is_match(bad), "should fail the gate: {bad}");
        }
        for ok in [
            "use crate::frame::DrawBatch;",
            "use crate::frame::{DrawBatch, DrawPayload, InstanceBuffer};",
            "pub(crate) use crate::frame::TextAtlasGpu;",
            "use website_graphics_engine::frames::x;",
            "use website_graphics_engine::frame_stats::x;",
            "use website_graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;",
            "use website_graphics_engine::draw::instances::QuadInstance;",
            "// the frame vocabulary lives one crate over",
        ] {
            assert!(!p.is_match(ok), "should pass the gate: {ok}");
        }
    }

    /// RULE 3b, RED — a GPU-resource import in a file the pin has never heard of.
    #[test]
    fn a_new_gpu_module_import_in_the_map_engine_fails() {
        let r = Repo::new("rule3b-new");
        r.map(
            "overlay/symbology/atlas.rs",
            "use website_graphics_engine::text::gpu::create_glyph_atlas;\n",
        );
        r.expect(
            1,
            &[
                "FAIL: GPU-resource modules named inside the map engine:",
                "  unpinned file — 1 site(s):",
                "apps/website/map-engine/src/overlay/symbology/atlas.rs:1:\
                 use website_graphics_engine::text::gpu::create_glyph_atlas;",
                RULE3B_TAIL[0],
                "1 GPU-module finding(s)",
            ],
        );
    }

    /// RULE 3b, THE RATCHET — a pinned file may not grow, and may not shrink either. A pin that
    /// no longer describes the tree is a rule that has stopped meaning what it says.
    #[test]
    fn the_rule_3b_pin_is_a_ratchet_in_both_directions() {
        let r = Repo::new("rule3b-grow");
        r.map(
            "frame/pump.rs",
            &format!("{MAP_FRAME_PUMP}use website_graphics_engine::device::x;\n"),
        );
        r.expect(
            1,
            &[
                "apps/website/map-engine/src/frame/pump.rs: pinned at 2 site(s), found 3",
                "1 GPU-module finding(s)",
            ],
        );

        let r = Repo::new("rule3b-shrink");
        r.map("frame/pump.rs", "// the impl crossed; nothing to import\n");
        r.expect(
            1,
            &[
                "apps/website/map-engine/src/frame/pump.rs: pinned at 2 site(s), found 0 — \
                 the pin is stale, delete the row.",
                "1 GPU-module finding(s)",
            ],
        );
    }

    /// RULE 3b's matcher, one line at a time. `::pipelines` is the pair that proves the `\b`:
    /// the pin must not be wideable by appending a letter.
    #[test]
    fn rule_3b_matches_the_five_gpu_modules_and_nothing_adjacent() {
        let p = Pattern::regex(GPU_MODULE_RE).unwrap();
        for bad in [
            "pub use website_graphics_engine::device::buffers;",
            "pub use website_graphics_engine::pipeline as pipelines;",
            "website_graphics_engine::shaders::SHADER_WGSL",
            "pub use website_graphics_engine::r#loop::RafPump;",
            "website_graphics_engine::text::gpu::create_text_atlas(",
        ] {
            assert!(p.is_match(bad), "should fail the gate: {bad}");
        }
        for ok in [
            "use website_graphics_engine::pipelines::x;",
            "use website_graphics_engine::frame::{DrawBatch, TextRun};",
            "use website_graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;",
            "use website_graphics_engine::draw::instances::QuadInstance;",
            "use website_graphics_engine::text::metrics::TextGlyphInstance;",
            "// the device lives in website_graphics_engine, one crate over",
        ] {
            assert!(!p.is_match(ok), "should pass the gate: {ok}");
        }
    }

    /// A missing manifest is `did not run`, not "no dependency edge found".
    #[test]
    fn a_missing_manifest_does_not_read_as_clean() {
        let r = Repo::new("no-manifest");
        std::fs::remove_file(r.0.join("apps/website/graphics-engine/Cargo.toml")).unwrap();
        let (code, out) = super::run(&r.0);
        let all = out.join("\n");
        assert_eq!(code, 2, "{all}");
        assert!(all.contains("Cargo.toml"), "{all}");
    }

    /// Build output is pruned, and the prune is scoped below the repo root — a checkout living
    /// under a `target-*` path must not prune itself into a vacuous pass.
    #[test]
    fn build_output_is_pruned_but_only_below_the_root() {
        let r = Repo::new("prune");
        r.src(
            "target-container/debug/build/dep/out/private.rs",
            "pub struct TerrainBlob;\n",
        );
        r.expect(0, &["ENGINE-LAYERS: PASS", "  scanned 1 .rs file(s)"]);

        let root = Path::new("/home/x/target-container/checkout");
        assert!(
            is_source(
                root,
                &root.join("apps/website/graphics-engine/src/draw/mod.rs")
            ),
            "the prune must not see the root's own path components"
        );
        assert!(!is_source(
            root,
            &root.join("src/target-gate-frontend/x.rs")
        ));
        assert!(!is_source(root, &root.join("target/debug/x.rs")));
    }
}
