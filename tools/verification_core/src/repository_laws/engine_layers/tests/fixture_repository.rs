//! The fixture checkout every engine-layer test starts from.
//!
//! **Role:** writes a minimal repository in a temporary directory that passes all eight rules —
//! a clean renderer with its manifest, the map engine's pinned residue at exactly the pinned
//! counts, a clean `data/`, `data/scenario/`, `world/` and `editing/`, and a clean frontend with
//! its manifest — so each test plants one defect and asserts on the exact report.
//! **Position:** test support for the sibling test modules of [`super`].
//! **Signals & state:** each [`Repo`] owns one temporary directory and removes it on drop.
//! **Invariants:** the fixture mirrors the real shapes rather than stubbing the pins out, so the
//! pins themselves are under test: change a count in a pin and the fixture stops matching it.

use std::path::PathBuf;

/// A clean two-file renderer. Every banned word appears here as PROSE or as an asset path,
/// which is the false-positive case rule 2 has to survive to be usable.
pub(super) const CLEAN: &str = r#"
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
pub(super) const MANIFEST: &str =
    "[package]\nname = \"website-graphics-engine\"\n\n[dependencies]\nbytemuck = \"1\"\n";

/// The map engine's pinned residue, at exactly the counts [`RULE3B_PIN`] and [`RULE3A_PIN`]
/// claim — 3 GPU-module sites and 2 more in `pump.rs`, and 8 frame-vocabulary re-exports.
/// The fixture mirrors the real shape rather than stubbing the pins out, so the pins
/// themselves are under test: change a count in either table and these fixtures stop
/// matching it. The prose line below is the one that names `r#loop` and no `frame` path —
/// that asymmetry is real, and deliberate, in the file this mirrors.
pub(super) const MAP_FRAME_MOD: &str = "\
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
pub(super) const MAP_FRAME_PUMP: &str = "\
pub use website_graphics_engine::r#loop::FrameTarget;
pub use website_graphics_engine::r#loop::RafPump;
";

/// The authored document, clean: `data/` names `crate::data` and nothing else in the crate.
/// The `store -> scenario` line is deliberate — that direction is legal and rule 4 does not
/// scan `data/store/`, so a matcher that fired on it would be a false positive on the very
/// first file.
pub(super) const MAP_DATA_STORE: &str = "\
use crate::data::store::{MissionDocCore, SlotSoa};
let b = crate::data::scenario::compile::terrain_bounds(&terrain);
";
/// The authored mission, clean: `data/scenario/` names only itself.
pub(super) const MAP_SCENARIO: &str = "\
use crate::data::scenario::ast::entities;
use crate::data::scenario::validate::{Finding, Severity};
";
/// Rule 4's two pinned sites, one per file, both cfg-gated exactly as the real ones are.
pub(super) const MAP_SCENARIO_FLATTEN_TEST: &str = "\
#[cfg(feature = \"store\")]
fn vehicles_from_writer_json_roundtrip() -> serde_json::Value {
    use crate::data::store::MissionDocCore;
}
";
pub(super) const MAP_SCENARIO_PAYLOAD_TEST: &str = "\
#[cfg(feature = \"store\")]
#[test]
fn briefing_prose_round_trips_through_the_document_core() {
    use crate::data::store::MissionDocCore;
}
";
/// The static world, clean: it names the world, the format layer and the streamer, and never
/// the document. Those three are what a `world/` file legitimately imports.
pub(super) const MAP_WORLD: &str = "\
use crate::world::terrain::dem::sampling::uint16_to_meters;
use crate::io::archives::codec::to_bytes;
use crate::streaming::scheduler::state::WorldResidency;
use crate::spatial::bvh::traversal::Bvh;
";

/// Rule 5's root, green: the editor's decisions, named in `crate::` and `std::` terms only.
pub(super) const MAP_EDITING: &str = "\
//! The two-click ray capture.

use crate::data::store::MissionDocCore;
use std::cell::RefCell;

/// A host supplies its own clock; this module asks for one rather than reaching for a window.
pub fn step(now: &dyn Fn() -> f64) -> f64 {
    now()
}
";

/// Rule 6's root, green: the frontend reaching the renderer through the map engine, and saying so
/// in prose — with the CARGO spelling, which is exactly the shape the matcher must not fire on.
pub(super) const FRONT_SRC: &str = "\
//! The canvas mount. The render loop is the renderer's one `RafPump` (`website-graphics-engine`),
//! reached through the map engine and never by depending on it directly.

use website_map_engine::frame::EngineHandle;

pub fn mount(engine: EngineHandle) {
    let _ = engine;
}
";

/// Rule 6's manifest arm, green: the map engine and no renderer edge.
pub(super) const FRONT_MANIFEST: &str = "\
[package]
name = \"website-frontend\"

[dependencies]
website-map-engine = { path = \"../map-engine\" }
";

pub(super) struct Repo(pub(super) PathBuf);
impl Repo {
    pub(super) fn new(name: &str) -> Repo {
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
        // Rules 5 and 6's roots, seeded on every fixture for the same reason: an absent root is
        // a hard FAIL, and every other test would trip over it.
        r.map("editing/tools/line_of_sight/capture.rs", MAP_EDITING);
        r.front("canvas/mount.rs", FRONT_SRC);
        r.front_manifest(FRONT_MANIFEST);
        r
    }

    /// Write a file under the frontend — rule 6's root.
    pub(super) fn front(&self, rel: &str, body: &str) {
        let p = self.0.join("apps/website/frontend/src").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    /// Write the frontend manifest — rule 6's dependency-edge arm.
    pub(super) fn front_manifest(&self, body: &str) {
        let p = self.0.join("apps/website/frontend/Cargo.toml");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    /// Write a file under the map engine — rule 3b's root.
    pub(super) fn map(&self, rel: &str, body: &str) {
        let p = self.0.join("apps/website/map-engine/src").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    pub(super) fn src(&self, rel: &str, body: &str) {
        let p = self.0.join("apps/website/graphics-engine/src").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    pub(super) fn manifest(&self, body: &str) {
        let p = self.0.join("apps/website/graphics-engine/Cargo.toml");
        std::fs::write(p, body).unwrap();
    }
    /// Run; assert the exit code and every expected line; hand back the joined output.
    pub(super) fn expect(&self, code: u8, want: &[&str]) -> String {
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
