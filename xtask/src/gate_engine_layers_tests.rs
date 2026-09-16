//! Tests for [`super`] — the engine-layer walls, §5 rules 1, 2, 3a, 3b, 4 and 7.
//!
//! Split out of `gate_engine_layers.rs` at 2D purely for SIZE-3: rules 4 and 7 brought six more
//! cases and the single file passed 1000 lines. `#[path]` keeps them one module — `use super::*`
//! still reaches the private matchers and `run()` — so nothing about what is tested moved.

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
