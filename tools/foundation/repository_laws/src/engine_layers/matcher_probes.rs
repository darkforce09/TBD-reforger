//! The engine-layer matchers, compiled and proved before any of them judges source.
//!
//! **Role:** compiles the nine constant matchers of rules 1 to 7 and runs each over subjects whose
//! answers are known: the shapes it must match and the legitimate spellings it must not; and
//! does the same for rule 6's matcher over the wasm-only graphics crates, which is built from
//! their names.
//! **Position:** [`probe_matchers`] is the first step of [`super::check_engine_layers`] and its
//! output feeds [`super::evaluation::evaluate`]; [`probe_wasm_only_graphics_import`] runs inside
//! that evaluation, once the walk has named the wasm-only members.
//! **Signals & state:** appends refusal lines to the report buffer it is handed; nothing else.
//! **Invariants:** a matcher that does not compile is a check that did not run (exit 2); a
//! matcher that answers a probe wrongly fails the run (exit 1) before any source is read.

use super::evaluation::BoundaryPatterns;
use super::report_text::PROBE_FAIL;
use super::rules::*;
use super::scanning::{probed, refuse, say};
use verification_core::{NotRun, Pattern, gate};

/// Compile every matcher and prove it; on refusal, the exit code and the report lines so far.
pub(super) fn probe_matchers(o: &mut Vec<String>) -> Result<BoundaryPatterns, (u8, Vec<String>)> {
    // Probe the matcher over a subject whose answer is known, BEFORE it decides anything. The
    // engine is compiled in, so "tool absent" is unreachable — but "the matcher works" is still a
    // claim, and `probe_str` returning a `Result` forces the dead arm to be written down.
    // Rule 1. The negatives are the renderer's own prose naming the map engine while describing
    // the wall, and a path through this crate.
    let map_engine_import = probed(
        o,
        "engine-layers rule 1 pattern",
        MAP_ENGINE_IMPORT_RE,
        &[
            "use map_engine::frame::EngineHandle;",
            "    let engine = map_engine::frame::boot(&device);",
            "extern crate map_engine;",
        ],
        &[
            "//! `frame/`; `map_engine` speaks it, never the reverse. It must never depend on",
            "/// `map_engine` and cannot move here: Rust's inherent-impl coherence (E0116) is",
            "use crate::frame::packet::FramePacket;",
        ],
    )?;

    let decl = match Pattern::regex(DECL_RE).and_then(Pattern::case_insensitive) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return Err(refuse(o, "engine-layers rule 2 pattern", cause));
        }
    };
    match gate::probe_str(&decl, "pub struct TerrainBlob;") {
        Ok(true) => {}
        Ok(false) => {
            say(o, PROBE_FAIL);
            return Err((1, std::mem::take(o)));
        }
        Err(cause) => return Err(refuse(o, "engine-layers self-probe", cause)),
    }

    let vocab = match Pattern::regex(FRAME_VOCAB_RE) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return Err(refuse(o, "engine-layers rule 3a pattern", cause));
        }
    };
    // Three subjects. The positive proves it fires on the import shape; `crate::frame` proves it
    // does NOT fire on the spelling every call site is supposed to use, which is the one false
    // positive that would make the rule unachievable; `::frames` proves the `\b`, because a pin
    // that can be widened by appending a letter is not a pin.
    match gate::probe_str(&vocab, "use graphics_engine::frame::DrawBatch;") {
        Ok(true) => {}
        Ok(false) => {
            say(o, PROBE_FAIL);
            return Err((1, std::mem::take(o)));
        }
        Err(cause) => return Err(refuse(o, "engine-layers rule 3a self-probe", cause)),
    }
    for subject in [
        "use crate::frame::DrawBatch;",
        "use graphics_engine::frames::x;",
    ] {
        match gate::probe_str(&vocab, subject) {
            Ok(false) => {}
            Ok(true) => {
                say(o, PROBE_FAIL);
                return Err((1, std::mem::take(o)));
            }
            Err(cause) => return Err(refuse(o, "engine-layers rule 3a self-probe", cause)),
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
            return Err(refuse(o, "engine-layers rule 3b pattern", cause));
        }
    };
    // Two subjects, not one: the positive proves the matcher fires, and `::pipelines` proves the
    // `\b` is real. A pin that can be widened by adding an `s` is not a pin.
    match gate::probe_str(&gpu, "use graphics_engine::pipeline::create_text_pipeline;") {
        Ok(true) => {}
        Ok(false) => {
            say(o, PROBE_FAIL);
            return Err((1, std::mem::take(o)));
        }
        Err(cause) => return Err(refuse(o, "engine-layers rule 3b self-probe", cause)),
    }
    match gate::probe_str(&gpu, "use graphics_engine::pipelines::x;") {
        Ok(false) => {}
        Ok(true) => {
            say(o, PROBE_FAIL);
            return Err((1, std::mem::take(o)));
        }
        Err(cause) => return Err(refuse(o, "engine-layers rule 3b self-probe", cause)),
    }

    // Rule 4. The negatives are the ones that matter: `crate::data::scenario` is the spelling
    // every legitimate line in this tree uses, `data::store_of_record` proves the `\b` on the
    // longest alternative, and `std::io` proves the matcher is anchored on `crate::` and not on
    // the module name — a rule that banned the standard library's `io` would be deleted by
    // whoever hit it first.
    let scenario_iso = probed(
        o,
        "engine-layers rule 4 pattern",
        RULE4_RE,
        &[
            "use crate::data::store::MissionDocCore;",
            "use crate::streaming::loaders::chunk::WorldChunk;",
            "    let lane = crate::overlay::lanes::lane_id(role);",
            "use graphics_engine::layout::pack::TEXT_UNIFORM_BYTES;",
            "use super::super::super::store::MissionDocCore;",
        ],
        &[
            "use crate::data::scenario::compile::compile_payload;",
            "use crate::data::store_of_record::Row;",
            "use std::io::Write;",
            "use serde_json::Value;",
            "use super::super::diagnostics::render_authored;",
        ],
    )?;

    // Rule 7, `data` side. `crate::worldgen` is the `\b` pair — a wall that can be walked through
    // by appending three letters to a module name is not a wall.
    let data_side = probed(
        o,
        "engine-layers rule 7 data pattern",
        RULE7_DATA_RE,
        &[
            "use crate::world::terrain::dem::grid::DemVectorGrid;",
            "use crate::streaming::scheduler::state::WorldResidency;",
            "    let hit = crate::spatial::indexing::picking::pick(qx, qy);",
            "use graphics_engine::draw::instances::QuadInstance;",
            "use super::super::super::streaming::loaders::chunk::WorldChunk;",
        ],
        &[
            "use crate::data::store::MissionDocCore;",
            "use crate::data::scenario::compile::terrain_bounds;",
            "use crate::worldgen::seed::X;",
            "// the world is streamed; this module only records what was authored",
            "use super::super::diagnostics::render_authored;",
        ],
    )?;

    // Rule 7, `world` side. `crate::database` and the bare word "yrs" are the two false positives
    // that would make this arm noise rather than a rule.
    let world_side = probed(
        o,
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
            "use world_file_formats::archives::codec::to_bytes;",
            "use crate::world::terrain::dem::grid::DemVectorGrid;",
        ],
    )?;

    // Rule 5. The negatives are what keep it a rule rather than noise: every legitimate line in
    // `editing/` is a `crate::` path or a `std::` one, and the two prefix subjects prove the `\b`
    // — a wall that can be walked through by appending letters to a crate name is not a wall.
    let dom = probed(
        o,
        "engine-layers rule 5 pattern",
        DOM_RE,
        &[
            "use web_sys::window;",
            "use leptos::prelude::RwSignal;",
            "use wasm_bindgen::prelude::*;",
            "    let _ = wasm_bindgen::JsValue::from_f64(1.0);",
            "// the host installs its leptos signal here",
        ],
        &[
            "use crate::data::store::MissionDocCore;",
            "use std::cell::RefCell;",
            "use web_sysfs::open;",
            "use leptosaur::prelude::*;",
        ],
    )?;

    // Rule 6. The negatives are the four mentions that exist today — prose spelling the CARGO
    // name while describing the boundary it respects — plus the map engine, whose name shares a
    // prefix with nothing here but would be caught by a sloppier alternation.
    let graphics_import = probed(
        o,
        "engine-layers rule 6 pattern",
        GRAPHICS_IMPORT_RE,
        &[
            "use graphics_engine::draw::triangulate;",
            "    let t = graphics_engine::draw::triangulate(&verts);",
            "extern crate graphics_engine;",
        ],
        &[
            "//! loop machinery moved to the renderer's one `RafPump` (`graphics_engine`)",
            "/// this app touches — through the map engine, never by depending on \
             `graphics_engine`",
            "use map_engine::frame::EngineHandle;",
            "use crate::v2::apps::editor::input::tools::ruler_tool::install_seam;",
        ],
    )?;

    Ok(BoundaryPatterns {
        map_engine_import,
        decl,
        vocab,
        gpu,
        scenario_iso,
        data_side,
        world_side,
        dom,
        graphics_import,
    })
}

/// Compile and prove rule 6's matcher over the wasm-only graphics crates named `identifiers`
/// ([`wasm_only_graphics_import_re`]); on refusal, the exit code and the report lines so far.
///
/// The subjects are built from the same names: each crate's two import shapes must match, and
/// prose naming the crate while describing the boundary, a crate-local path and another crate
/// whose name extends it must not. No subject names a fixed crate, because any fixed name could
/// be one of the wasm-only crates and turn a correct matcher's probe red.
pub(super) fn probe_wasm_only_graphics_import(
    o: &mut Vec<String>,
    identifiers: &[String],
) -> Result<Pattern, (u8, Vec<String>)> {
    let mut must: Vec<String> = Vec::new();
    let mut must_not: Vec<String> = Vec::new();
    for identifier in identifiers {
        must.push(format!("use {identifier}::device::GpuDevice;"));
        must.push(format!("    let d = {identifier}::device::open(&canvas);"));
        must.push(format!("extern crate {identifier};"));
        must_not.push(format!(
            "//! the map engine owns `{identifier}` and the frontend never depends on it"
        ));
        must_not.push(format!("use crate::{identifier}_settings::Panel;"));
        must_not.push(format!("use {identifier}_test_support::FakeDevice;"));
    }
    let must: Vec<&str> = must.iter().map(String::as_str).collect();
    let must_not: Vec<&str> = must_not.iter().map(String::as_str).collect();
    probed(
        o,
        "engine-layers rule 6 wasm-only graphics pattern",
        &wasm_only_graphics_import_re(identifiers),
        &must,
        &must_not,
    )
}
