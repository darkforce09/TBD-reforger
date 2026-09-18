//! Engine-layer walls — `ENGINE_SPLIT_PROGRAM.md` §5 rules **1, 2, 3a, 3b, 4, 5, 6 and 7**.
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
//! | 5 | no `web_sys` / `leptos` / `wasm_bindgen` under `apps/website/map-engine/src/editing` | editing logic re-grows a browser and stops being testable without one |
//! | 6 | `apps/website/frontend/**` may not import `website_graphics_engine` | the frontend starts driving the GPU directly and the middle crate becomes optional |
//! | 7 | `data/**` names no world module and `world/**` names no document module | the static world and the authored document fuse back into one soup |
//!
//! ── RULE 5 AND WHY IT IS SCOPED TO ONE DIRECTORY ─────────────────────────────────────────────
//!
//! §5 writes rule 5 as "no DOM in map-engine". Taken crate-wide it can never be green and never
//! could be: `map-engine` is a wasm crate whose `streaming/host`, `diagnostics/readback`,
//! `doll/renderer` and `frame/*` reach the browser on purpose — 266 sites across 57 files — and
//! a rule nobody can satisfy gets deleted by whoever hits it first. The program's own acceptance
//! line is narrower than its prose and is the honest statement of the rule:
//! `rg 'web_sys|leptos|wasm_bindgen' apps/website/map-engine/src/editing` comes back empty.
//!
//! That is the subject here, and it is a **hard zero with no allowlist**. `editing/` is the tree
//! the editor's decisions live in — tool state machines, the undo drive, the command formatting —
//! and every one of them must be answerable by `cargo test` with no browser in the room. A host
//! injects what it alone can supply (a clock, a frame pump, a transport) as a plain function
//! pointer or closure, which is why the tree needs none of these three names.
//!
//! The matcher is the bare word, deliberately, and it is the same question the acceptance line
//! asks. Outside `editing/` a bare-word matcher would be noise; inside it, a comment that tells
//! the next reader to reach for `leptos` here is exactly the thing the rule exists to stop.
//!
//! ── RULE 6 AND WHY IT MATCHES SYNTAX RATHER THAN THE WORD ────────────────────────────────────
//!
//! The frontend reaches the renderer through `website-map-engine` and only through it: the middle
//! crate owns the frame vocabulary (rule 3a) and the GPU resources (rule 3b), and a frontend that
//! imported the renderer directly would make both of those walls optional.
//!
//! The subject is already zero and the four existing mentions are all prose — three doc comments
//! and a README, every one spelling the CARGO name (`website-graphics-engine`) while describing
//! the boundary they respect. A bare-word matcher would turn four correct comments red, which is
//! how a gate teaches people to delete the comment rather than keep the wall. So the source arm
//! matches the two shapes that are actually an import — a `website_graphics_engine::` path and an
//! `extern crate` — and the manifest arm matches the dependency edge, closing the rename hole
//! exactly as rule 1's does.
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
//! §2D: *"`world/` is immutable, streamed from `assets_v2/terrains`, cacheable, never persisted.
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
//! `NotRun` and exits **2** — "I never looked" is a different operator action from "I looked and
//! it is dirty". Same reasoning, and the same [`verification_core::Verdict`] machinery, as
//! `crate::verifications::database::sql_deserialization::report_did_not_run`.
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
//! `mod symbology` all fail while `/// a submission arrives here`, `include_str!("../../terrain.wgsl")`
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
//! CLI has: every sibling gate is `cargo xtask verify <name>` (`crate::VerifyCmd`), and a
//! top-level `verify-engine-layers` would be the only hyphenated verb on the root command. The
//! spec was describing the gate, not the argv. Both spellings work anyway — the `TASKS` alias row
//! is named `verify-engine-layers`, so `cargo xtask ci verify-engine-layers` resolves, the same
//! way `verify-no-node` aliases `verify no-node`.

use std::path::Path;

use anyhow::Result;
use verification_core::scan;
use verification_core::{NotRun, Pattern, gate};

#[path = "engine_layer_rules.rs"]
mod rules;
#[path = "engine_layer_scan.rs"]
mod scanning;

use rules::*;
use scanning::{against_pin, is_source, probed, refuse, rel, say, under};

#[cfg(test)]
#[path = "tests/engine_layer_boundaries.rs"]
mod tests;

mod boundary_evaluation;

mod verify_engine_layers;
pub use verify_engine_layers::verify_engine_layers;

#[cfg(test)]
use verify_engine_layers::run;
