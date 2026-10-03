//! The fixture checkout every engine-layer test starts from.
//!
//! **Role:** writes a minimal repository in a temporary directory that passes all seven rules —
//! a root manifest whose `crates/*/*` glob makes one CPU-only `crates/graphics` member, a clean
//! parked renderer with its manifest, the map engine's pinned residue at exactly the pinned
//! counts, a clean `world/` and `editing/`, and a clean frontend with its manifest — so each test
//! plants one defect and asserts on the exact report.
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
    "[package]\nname = \"graphics_engine\"\n\n[dependencies]\nbytemuck = \"1\"\n";

/// The root manifest: every `crates/<category>/<name>` folder holding a `Cargo.toml` is a member,
/// as in the real workspace.
pub(super) const WORKSPACE_MANIFEST: &str = "[workspace]\nmembers = [\"crates/*/*\"]\n";

/// The graphics-category member every fixture carries: CPU-only, so rules 1 and 2 scan it and
/// rule 6 lets the frontend link it.
pub(super) const GRAPHICS_MEMBER: &str = "render_primitives";
/// [`GRAPHICS_MEMBER`]'s manifest.
pub(super) const GRAPHICS_MEMBER_MANIFEST: &str = "\
[package]
name = \"render_primitives\"

[package.metadata.layout]
category = \"crates/graphics\"
tier = 0
targets = \"any\"

[dependencies]
bytemuck = \"1\"
";
/// [`GRAPHICS_MEMBER`]'s source, clean: geometry and byte layouts, with a map noun in prose only.
pub(super) const GRAPHICS_MEMBER_SOURCE: &str = "\
//! Per-instance byte layouts. The map engine decides which terrain cell an instance draws.
pub struct QuadInstance {
    pub min: [f32; 2],
}
";

/// The map engine's pinned residue, at exactly the counts [`RULE3B_PIN`] and [`RULE3A_PIN`]
/// claim — 3 GPU-module sites and 2 more in `pump.rs`, and 5 frame-vocabulary re-exports.
/// The fixture mirrors the real shape rather than stubbing the pins out, so the pins
/// themselves are under test: change a count in either table and these fixtures stop
/// matching it. The prose line below is the one that names `r#loop` and no `frame` path —
/// that asymmetry is real, and deliberate, in the file this mirrors.
pub(super) const MAP_FRAME_MOD: &str = "\
pub use graphics_engine::device::buffers;
pub use graphics_engine::pipeline as pipelines;
/// Re-export `graphics_engine::r#loop::{FrameTarget, RafPump}`.
pub use pump::{FrameTarget, RafPump};
pub use graphics_engine::frame::packet;
pub use graphics_engine::frame::present;
pub use graphics_engine::frame::{DrawBatch, DrawPayload, FramePacket, IndirectDraw};
pub use graphics_engine::frame::{IndexedMesh, InstanceBuffer, VertexStream};
pub use graphics_engine::frame::{
    GlyphAtlasGpu, TextAtlasGpu, TextRun, create_glyph_atlas, create_text_atlas,
};
";
pub(super) const MAP_FRAME_PUMP: &str = "\
pub use graphics_engine::r#loop::FrameTarget;
pub use graphics_engine::r#loop::RafPump;
";

/// The static world, clean: it names the world, the format layer and the streamer, and never
/// the document. Those three are what a `world/` file legitimately imports; the mission model
/// line is the domain crate that is not the document, which rule 7 must not fire on.
pub(super) const MAP_WORLD: &str = "\
use terrain_elevation::sampling::uint16_to_meters;
use world_file_formats::archives::codec::to_bytes;
use crate::streaming::scheduler::state::WorldResidency;
use spatial_indexes::bounding_volume_hierarchy::triangle_tree::Bvh;
use mission_model::orbat::OrbatSlot;
";

/// Rule 5's root, green: the editor's decisions, named in `crate::` and `std::` terms only.
pub(super) const MAP_EDITING: &str = "\
//! The two-click ray capture.

use mission_document::MissionDocCore;
use std::cell::RefCell;

/// A host supplies its own clock; this module asks for one rather than reaching for a window.
pub fn step(now: &dyn Fn() -> f64) -> f64 {
    now()
}
";

/// Rule 6's root, green: the frontend reaching the renderer through the map engine, and saying so
/// in prose — with the CARGO spelling, which is exactly the shape the matcher must not fire on.
pub(super) const FRONT_SRC: &str = "\
//! The canvas mount. The render loop is the renderer's one `RafPump` (`graphics_engine`),
//! reached through the map engine and never by depending on it directly.

use map_engine::frame::EngineHandle;

pub fn mount(engine: EngineHandle) {
    let _ = engine;
}
";

/// Rule 6's manifest arm, green: the map engine and no renderer edge.
pub(super) const FRONT_MANIFEST: &str = "\
[package]
name = \"frontend\"

[dependencies]
map_engine = { path = \"../../legacy/map_engine\" }
";

pub(super) struct Repo(pub(super) PathBuf);
impl Repo {
    pub(super) fn new(name: &str) -> Repo {
        let mut p = std::env::temp_dir();
        p.push(format!("tbd-el-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(p.join("legacy/graphics_engine/src/draw")).unwrap();
        let r = Repo(p);
        std::fs::write(r.0.join("Cargo.toml"), WORKSPACE_MANIFEST).unwrap();
        r.member_manifest(GRAPHICS_MEMBER, GRAPHICS_MEMBER_MANIFEST);
        r.member(GRAPHICS_MEMBER, "draw/instances.rs", GRAPHICS_MEMBER_SOURCE);
        r.src("draw/mod.rs", CLEAN);
        r.manifest(MANIFEST);
        r.map("frame/mod.rs", MAP_FRAME_MOD);
        r.map("frame/pump.rs", MAP_FRAME_PUMP);
        // Rule 7's root. It is seeded on every fixture, not only the tests that exercise it,
        // because an absent root is a hard FAIL — which is the point.
        r.map("world/terrain/dem/loader.rs", MAP_WORLD);
        // Rules 5 and 6's roots, seeded on every fixture for the same reason: an absent root is
        // a hard FAIL, and every other test would trip over it.
        r.map("editing/tools/line_of_sight/capture.rs", MAP_EDITING);
        r.front("canvas/mount.rs", FRONT_SRC);
        r.front_manifest(FRONT_MANIFEST);
        r
    }

    /// Write a file under the `src` folder of the graphics-category member `name` — rules 1 and
    /// 2's roots beside the parked renderer.
    pub(super) fn member(&self, name: &str, rel: &str, body: &str) {
        let p = self
            .0
            .join("crates/graphics")
            .join(name)
            .join("src")
            .join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    /// Write the manifest of the graphics-category member `name`, which makes it a workspace
    /// member through the root glob.
    pub(super) fn member_manifest(&self, name: &str, body: &str) {
        let p = self.0.join("crates/graphics").join(name).join("Cargo.toml");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    /// Write a file under the frontend — rule 6's root.
    pub(super) fn front(&self, rel: &str, body: &str) {
        let p = self.0.join("apps/frontend/src").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    /// Write the frontend manifest — rule 6's dependency-edge arm.
    pub(super) fn front_manifest(&self, body: &str) {
        let p = self.0.join("apps/frontend/Cargo.toml");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    /// Write a file under the map engine — rule 3b's root.
    pub(super) fn map(&self, rel: &str, body: &str) {
        let p = self.0.join("legacy/map_engine/src").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    pub(super) fn src(&self, rel: &str, body: &str) {
        let p = self.0.join("legacy/graphics_engine/src").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    pub(super) fn manifest(&self, body: &str) {
        let p = self.0.join("legacy/graphics_engine/Cargo.toml");
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
