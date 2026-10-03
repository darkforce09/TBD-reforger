//! Tests for [`super`] — the engine-layer walls, §5 rules 1, 2, 3a, 3b, 6 and 7.
//!
//! Every rule is pinned red and green against a fixture checkout, every matcher line by line,
//! and every refusal path by its exit code. The module reaches the private matchers and `run()`
//! through `use super::*`, and the fixture through [`super::fixture_repository`].

use super::fixture_repository::*;
use super::*;

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
            RULE7_HEAD,
            "  OK (none)",
            "  OK — 5 pinned site(s) in 1 file(s), 0 unpinned.",
            "  OK — 5 pinned site(s), 0 unpinned.",
            "  OK — 0 site(s) across 1 .rs file(s) under world/: the static world names \
                 neither yrs, nor a mission document crate, nor a mission-editing crate.",
            "  OK — 0 import(s) across 1 .rs file(s) and the manifest:",
            "  scanned 1 .rs file(s) + legacy/graphics_engine/Cargo.toml, \
                 1 .rs file(s) + Cargo.toml across 1 crates/graphics member(s) (0 wasm-only), \
                 3 .rs file(s) under legacy/map_engine/src — of those 1 under world/ — \
                 plus 1 .rs file(s) + Cargo.toml under apps/frontend",
            "ENGINE-LAYERS: PASS",
        ],
    );
    assert!(!all.contains("FAIL"), "{all}");
}

/// RULE 1, RED — an import of the map engine, reported with its exact line.
#[test]
fn importing_the_map_engine_fails() {
    let r = Repo::new("rule1");
    r.src("draw/bad.rs", "use map_engine::x;\n");
    r.expect(
        1,
        &[
            "FAIL: the graphics layer reaches back into the map engine:",
            "  legacy/graphics_engine/src/draw/bad.rs:1:use map_engine::x;",
            RULE1_TAIL[0],
            "ENGINE-LAYERS: FAIL — 1 wall breach(es), 0 map-noun declaration(s), \
                 0 frame-vocab finding(s), 0 GPU-module finding(s), \
                 0 direct-renderer import(s), 0 world-document finding(s)",
        ],
    );
}

/// RULE 1's false positives: prose naming the map engine while describing the wall is not an
/// import, whichever comment form carries it.
#[test]
fn rule_1_matches_imports_and_not_the_prose_that_describes_the_wall() {
    let r = Repo::new("rule1-prose");
    r.src(
        "lib.rs",
        "//! `map_engine` speaks the frame vocabulary, never the reverse.\n\
         /// `map_engine`'s `RenderEngine` owns the atlas slots.\n\
         pub mod frame;\n",
    );
    r.expect(0, &["ENGINE-LAYERS: PASS"]);
}

/// RULE 1, the manifest arm: a dependency edge is a breach even with no `use` anywhere, and a
/// `#` comment naming the other crate is not.
#[test]
fn a_dependency_edge_is_a_breach_and_a_comment_is_not() {
    let r = Repo::new("rule1-manifest");
    r.manifest(&format!("{MANIFEST}me = {{ package = \"map_engine\" }}\n"));
    r.expect(
        1,
        &[
            "  legacy/graphics_engine/Cargo.toml:6:me = { package = \"map_engine\" }",
            "ENGINE-LAYERS: FAIL — 1 wall breach(es), 0 map-noun declaration(s), \
                 0 frame-vocab finding(s), 0 GPU-module finding(s), \
                 0 direct-renderer import(s), 0 world-document finding(s)",
        ],
    );
    r.manifest(&format!(
        "{MANIFEST}# map_engine depends on us, never the reverse\n"
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
            "  legacy/graphics_engine/src/draw/bad.rs:1:pub struct TerrainBlob;",
            "  legacy/graphics_engine/src/draw/bad.rs:2:pub fn pack_mission() {}",
            RULE2_TAIL[0],
            "ENGINE-LAYERS: FAIL — 0 wall breach(es), 2 map-noun declaration(s), \
                 0 frame-vocab finding(s), 0 GPU-module finding(s), \
                 0 direct-renderer import(s), 0 world-document finding(s)",
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
    assert!(
        all.contains(GRAPHICS_ENGINE_REL) && !all.contains("PASS"),
        "{all}"
    );
    assert!(all.contains("ENGINE-LAYERS: FAIL (did not run)"), "{all}");

    // src/ present but empty of Rust: `walk_files` returns Ok(vec![]) and every grep finds
    // nothing, which is the shape that would report a clean wall over zero bytes of source.
    let r = Repo::new("empty");
    std::fs::remove_file(r.0.join("legacy/graphics_engine/src/draw/mod.rs")).unwrap();
    r.expect(
        1,
        &[
            "FAIL: engine-layers walked 0 .rs file(s) under legacy/graphics_engine/src",
            NOTHING_TAIL[0],
            "ENGINE-LAYERS: FAIL (no inputs)",
        ],
    );

    // Rule 3b's root has the same hole, and it is the one phase 2B reshaped on purpose.
    let r = Repo::new("empty-map");
    std::fs::remove_dir_all(r.0.join("legacy/map_engine/src")).unwrap();
    std::fs::create_dir_all(r.0.join("legacy/map_engine/src")).unwrap();
    r.expect(
        1,
        &[
            "FAIL: engine-layers walked 0 .rs file(s) under legacy/map_engine/src",
            "ENGINE-LAYERS: FAIL (no inputs)",
        ],
    );
}

/// RULE 7'S ANTI-VACUITY CASE, and the reason the rule prints its count.
///
/// Rule 7's green state is **zero sites**, which makes it able to go vacuously green: a matcher
/// that finds nothing over a tree that is not there reports exactly what a clean wall reports.
/// Code keeps leaving `world/` for crates of its own, so "the directory is gone" is a live
/// outcome, not a hypothetical.
#[test]
fn an_absent_world_tree_is_not_a_clean_rule_7() {
    let r = Repo::new("no-world");
    std::fs::remove_dir_all(r.0.join(WORLD_REL)).unwrap();
    let all = r.expect(
        1,
        &[
            "FAIL: engine-layers walked 0 .rs file(s) under legacy/map_engine/src/world",
            "ENGINE-LAYERS: FAIL (no inputs)",
        ],
    );
    assert!(
        !all.contains("ENGINE-LAYERS: PASS"),
        "a missing world tree must never read as a clean wall:\n{all}"
    );
}

/// RULE 7, RED — the CRDT crate, a mission document crate and a mission-editing crate reaching
/// `world/`, each reported with its exact line.
#[test]
fn the_world_naming_the_document_breaches_the_wall() {
    let r = Repo::new("rule7");
    r.map(
        "world/terrain/dem/edited.rs",
        "use yrs::Doc;\nlet t = mission_document::MissionDocCore::new();\n\
         use mission_editing_session::history::UndoStack;\n",
    );
    r.expect(
        1,
        &[
            "FAIL: the static world names the authored document:",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/\
                 edited.rs:1:use yrs::Doc;",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/\
                 edited.rs:2:let t = mission_document::MissionDocCore::new();",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/\
                 edited.rs:3:use mission_editing_session::history::UndoStack;",
            RULE7_TAIL[0],
            "3 world-document finding(s)",
        ],
    );
}

/// RULE 7's matcher, one line at a time.
///
/// The `ok` list carries the false positives that would each, on their own, make the rule
/// unusable: "3 yrs" is the English word the CRDT crate is spelled as, `my_mission_document` and
/// `mission_document_notes` are one affix away from a hit, and `mission_model` is the mission
/// domain crate that is not the document.
#[test]
fn rule_7_matches_the_document_and_not_the_legal_traffic() {
    let world = Pattern::regex(RULE7_WORLD_RE).unwrap();
    for bad in [
        "use mission_document::MissionDocCore;",
        "use mission_crdt::slot_columns::SlotSoa;",
        "    let ids = mission_operations::entity::paste_at_cursor(&core, &buffer);",
        "use yrs::{Doc, Transact};",
        "fn tx(d: &yrs::Doc) -> yrs::TransactionMut<'_> { d.transact_mut() }",
        "use mission_editing_session::history::UndoStack;",
        "    let s = mission_editing_commands::hosted_commands::summarise(&doc);",
    ] {
        assert!(world.is_match(bad), "should fail the gate: {bad}");
    }
    for ok in [
        "use crate::editing_notes::Note;",
        "use my_mission_document::Row;",
        "use mission_document_notes::Note;",
        "use mission_model::orbat::OrbatSlot;",
        "// resurveyed 3 yrs after the original DEM pass",
        "// the editing tools read this module; it never reads them",
        "use world_file_formats::archives::codec::to_bytes;",
        "use crate::world::terrain::dem::grid::DemVectorGrid;",
        "use crate::streaming::loaders::fetch::fetch_bytes;",
        "use super::super::dem::grid::DemVectorGrid;",
        "pub struct DemVectorGrid { pub cells: Vec<u16> }",
    ] {
        assert!(!world.is_match(ok), "should pass the gate: {ok}");
    }
}

/// RULE 7, RED — the static world naming each mission-editing crate that hosts the live document:
/// the session with its undo drive, the hosted commands, the draft decisions and the map tools.
#[test]
fn the_world_naming_a_mission_editing_crate_breaches_the_wall() {
    let r = Repo::new("rule7-mission-editing");
    r.map(
        "world/terrain/dem/undo.rs",
        "use mission_editing_session::history::UndoStack;\n\
         let s = mission_editing_commands::hosted_commands::summarise(&doc);\n\
         use mission_persistence::draft::DraftDecision;\n\
         use map_editing_tools::ruler::RulerTool;\n",
    );
    r.expect(
        1,
        &[
            "FAIL: the static world names the authored document:",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/undo.rs:1:\
             use mission_editing_session::history::UndoStack;",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/undo.rs:2:\
             let s = mission_editing_commands::hosted_commands::summarise(&doc);",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/undo.rs:3:\
             use mission_persistence::draft::DraftDecision;",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/undo.rs:4:\
             use map_editing_tools::ruler::RulerTool;",
            RULE7_TAIL[0],
            "4 world-document finding(s)",
        ],
    );
}

/// Rule 7's matcher over the mission-editing crates, one line at a time: each crate's path
/// matches, and a crate whose name only ends or starts with one, a crate-local module whose name
/// starts with `editing`, or prose naming the editing layer does not.
#[test]
fn rule_7_matches_the_mission_editing_crates_and_nothing_adjacent() {
    let p = Pattern::regex(RULE7_WORLD_RE).unwrap();
    for bad in [
        "use mission_editing_session::history::UndoStack;",
        "    let s = mission_editing_commands::hosted_commands::summarise(&doc);",
        "use mission_persistence::draft::DraftDecision;",
        "use map_editing_tools::ruler::RulerTool;",
    ] {
        assert!(p.is_match(bad), "rule 7 must match {bad:?}");
    }
    for ok in [
        "use crate::editing_notes::Note;",
        "use my_mission_editing_session::Row;",
        "use map_editing_tools_settings::Panel;",
        "// the mission editing tools read this module; it never reads them",
    ] {
        assert!(!p.is_match(ok), "rule 7 must not match {ok:?}");
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
        "use graphics_engine::frame::DrawPayload;\n",
    );
    r.map(
        "diagnostics/readback/scene.rs",
        "let p = graphics_engine::frame::FramePacket { camera };\n",
    );
    r.expect(
        1,
        &[
            "FAIL: the frame vocabulary is named outside the packet boundary:",
            "  unpinned file — 1 site(s):",
            "legacy/map_engine/src/world/environment/vegetation/buffers.rs:1:\
                 use graphics_engine::frame::DrawPayload;",
            "legacy/map_engine/src/diagnostics/readback/scene.rs:1:\
                 let p = graphics_engine::frame::FramePacket { camera };",
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
        &format!("{MAP_FRAME_MOD}pub use graphics_engine::frame::Extra;\n"),
    );
    r.expect(
        1,
        &[
            "legacy/map_engine/src/frame/mod.rs: pinned at 5 site(s), found 6",
            "1 frame-vocab finding(s)",
        ],
    );

    let r = Repo::new("rule3a-shrink");
    // Drop one re-export. Everything else about the file — including its three rule-3b
    // sites — stays, so this isolates the 3a count and nothing else moves.
    r.map(
        "frame/mod.rs",
        &MAP_FRAME_MOD.replace("pub use graphics_engine::frame::present;\n", ""),
    );
    r.expect(
        1,
        &[
            "legacy/map_engine/src/frame/mod.rs: pinned at 5 site(s), found 4",
            "1 frame-vocab finding(s)",
        ],
    );

    let r = Repo::new("rule3a-vanished");
    r.map("frame/mod.rs", "// the boundary moved somewhere else\n");
    let all = r.expect(
        1,
        &[
            "legacy/map_engine/src/frame/mod.rs: pinned at 5 site(s), found 0 — \
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
        "use graphics_engine::frame::DrawBatch;",
        "pub use graphics_engine::frame::{BindGroupId, LaneId, PipelineId};",
        "pub use graphics_engine::frame::damage;",
        "    camera: graphics_engine::frame::CameraUniform::new(mvp),",
        "pub fn lane_id(r: R) -> graphics_engine::frame::LaneId {",
        "//! the opaque `graphics_engine::frame::LaneId`",
    ] {
        assert!(p.is_match(bad), "should fail the gate: {bad}");
    }
    for ok in [
        "use crate::frame::DrawBatch;",
        "use crate::frame::{DrawBatch, DrawPayload, InstanceBuffer};",
        "pub(crate) use crate::frame::TextAtlasGpu;",
        "use graphics_engine::frames::x;",
        "use graphics_engine::frame_stats::x;",
        "use graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;",
        "use graphics_engine::draw::instances::QuadInstance;",
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
        "use graphics_engine::pipeline::create_glyph_pipeline;\n",
    );
    r.expect(
        1,
        &[
            "FAIL: GPU-resource modules named inside the map engine:",
            "  unpinned file — 1 site(s):",
            "legacy/map_engine/src/overlay/symbology/atlas.rs:1:\
                 use graphics_engine::pipeline::create_glyph_pipeline;",
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
        &format!("{MAP_FRAME_PUMP}use graphics_engine::device::x;\n"),
    );
    r.expect(
        1,
        &[
            "legacy/map_engine/src/frame/pump.rs: pinned at 2 site(s), found 3",
            "1 GPU-module finding(s)",
        ],
    );

    let r = Repo::new("rule3b-shrink");
    r.map("frame/pump.rs", "// the impl crossed; nothing to import\n");
    r.expect(
        1,
        &[
            "legacy/map_engine/src/frame/pump.rs: pinned at 2 site(s), found 0 — \
                 the pin is stale, delete the row.",
            "1 GPU-module finding(s)",
        ],
    );
}

/// RULE 3b's matcher, one line at a time. `::pipelines` is the pair that proves the `\b`:
/// the pin must not be wideable by appending a letter.
#[test]
fn rule_3b_matches_the_four_gpu_modules_and_nothing_adjacent() {
    let p = Pattern::regex(GPU_MODULE_RE).unwrap();
    for bad in [
        "pub use graphics_engine::device::buffers;",
        "pub use graphics_engine::pipeline as pipelines;",
        "graphics_engine::shaders::SHADER_WGSL",
        "pub use graphics_engine::r#loop::RafPump;",
    ] {
        assert!(p.is_match(bad), "should fail the gate: {bad}");
    }
    for ok in [
        "use graphics_engine::pipelines::x;",
        "use graphics_engine::frame::{DrawBatch, TextRun};",
        "use graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;",
        "use graphics_engine::draw::instances::QuadInstance;",
        "use graphics_engine::text::metrics::TextGlyphInstance;",
        "// the device lives in graphics_engine, one crate over",
    ] {
        assert!(!p.is_match(ok), "should pass the gate: {ok}");
    }
}

/// A missing manifest is `did not run`, not "no dependency edge found".
#[test]
fn a_missing_manifest_does_not_read_as_clean() {
    let r = Repo::new("no-manifest");
    std::fs::remove_file(r.0.join("legacy/graphics_engine/Cargo.toml")).unwrap();
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
        is_source(root, &root.join("legacy/graphics_engine/src/draw/mod.rs")),
        "the prune must not see the root's own path components"
    );
    assert!(!is_source(root, &root.join("src/target-container/x.rs")));
    assert!(!is_source(root, &root.join("target/debug/x.rs")));
}

/// RULE 6, RED — both import shapes and the dependency edge, each reported with its line.
#[test]
fn the_frontend_importing_the_renderer_fails() {
    let r = Repo::new("rule6-red");
    r.front(
        "canvas/bad.rs",
        "use graphics_engine::draw::triangulate;\n\
         pub fn t(v: &[f32]) -> Vec<u32> { graphics_engine::draw::triangulate(v) }\n",
    );
    let all = r.expect(
        1,
        &[
            "FAIL: the frontend imports the renderer directly:",
            "  apps/frontend/src/canvas/bad.rs:1:use graphics_engine::draw::\
             triangulate;",
            RULE6_TAIL[0],
        ],
    );
    assert!(all.contains("2 direct-renderer import(s)"), "{all}");

    // The manifest arm, which is the one a rename would otherwise walk straight through:
    // `g = { package = "graphics_engine" }` makes every `use g::…` invisible to the
    // source arm.
    let r = Repo::new("rule6-manifest");
    r.front_manifest(
        "[dependencies]\ng = { path = \"../../legacy/graphics_engine\", package = \"graphics_engine\" }\n",
    );
    r.expect(
        1,
        &[
            "FAIL: the frontend imports the renderer directly:",
            "apps/frontend/Cargo.toml:2:g = { path",
        ],
    );
}

/// RULE 6's FALSE POSITIVES — the four mentions that exist in the tree today, every one of them
/// prose describing the boundary it respects, and every one spelling the CARGO name. A bare-word
/// matcher would turn all four red, which is how a gate teaches people to delete the comment
/// rather than keep the wall. A `#` line in the manifest is a comment, not an edge.
#[test]
fn rule_6_matches_imports_and_not_the_prose_that_describes_the_wall() {
    let r = Repo::new("rule6-prose");
    r.front(
        "canvas/viewport.rs",
        "//! loop machinery moved to the renderer's one `RafPump` (`graphics_engine`)\n\
         /// through the map engine, never by depending on `graphics_engine`\n\
         use map_engine::frame::EngineHandle;\n",
    );
    r.front_manifest(
        "[dependencies]\n# graphics_engine is reached through the map engine\n\
         map_engine = { path = \"../../legacy/map_engine\" }\n",
    );
    r.expect(0, &["ENGINE-LAYERS: PASS"]);
}

/// RULE 6's ANTI-VACUITY CASE, on both halves of its root: an unwalked frontend and an unreadable
/// manifest are each "nothing to look at", and neither may read as a clean wall.
#[test]
fn an_absent_frontend_is_not_a_clean_rule_6() {
    let r = Repo::new("rule6-empty");
    std::fs::remove_file(r.0.join("apps/frontend/src/canvas/mount.rs")).unwrap();
    let all = r.expect(
        1,
        &[
            "FAIL: engine-layers walked 0 .rs file(s) under apps/frontend/src",
            "ENGINE-LAYERS: FAIL (no inputs)",
        ],
    );
    assert!(!all.contains("ENGINE-LAYERS: PASS"), "{all}");

    let r = Repo::new("rule6-no-manifest");
    std::fs::remove_file(r.0.join("apps/frontend/Cargo.toml")).unwrap();
    let (code, out) = super::run(&r.0);
    let all = out.join("\n");
    assert_eq!(code, 2, "{all}");
    assert!(all.contains("frontend manifest"), "{all}");
}
