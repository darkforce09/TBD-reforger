use super::*;

pub fn verify_engine_layers(repo_root: &Path) -> Result<u8> {
    let (code, out) = run(repo_root);
    for line in out {
        println!("{line}");
    }
    Ok(code)
}

/// The gate proper, writing into a sink so the tests assert on exact bytes instead of scraping
/// stdout — the [`crate::verifications::architecture::route_tags`] shape.
pub(super) fn run(repo_root: &Path) -> (u8, Vec<String>) {
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

    // Rule 5. The negatives are what keep it a rule rather than noise: every legitimate line in
    // `editing/` is a `crate::` path or a `std::` one, and the two prefix subjects prove the `\b`
    // — a wall that can be walked through by appending letters to a crate name is not a wall.
    let dom = match probed(
        &mut o,
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
    ) {
        Ok(p) => p,
        Err(r) => return r,
    };

    // Rule 6. The negatives are the four mentions that exist today — prose spelling the CARGO
    // name while describing the boundary it respects — plus the map engine, whose name shares a
    // prefix with nothing here but would be caught by a sloppier alternation.
    let graphics_import = match probed(
        &mut o,
        "engine-layers rule 6 pattern",
        GRAPHICS_IMPORT_RE,
        &[
            "use website_graphics_engine::draw::triangulate;",
            "    let t = website_graphics_engine::draw::triangulate(&verts);",
            "extern crate website_graphics_engine;",
        ],
        &[
            "//! loop machinery moved to the renderer's one `RafPump` (`website-graphics-engine`)",
            "/// this app touches — through the map engine, never by depending on \
             `website-graphics-engine`",
            "use website_map_engine::frame::EngineHandle;",
            "use crate::v2::apps::editor::input::tools::ruler_tool::install_seam;",
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
    let editing_files = under(repo_root, &map_sources, EDITING_REL);

    // Rule 6's root is a third crate, walked here because no earlier rule reads it. Its manifest
    // rides the same walk as its sources, exactly as rule 1's does for graphics-engine.
    let front_dir = repo_root.join(FRONTEND_REL);
    let root3 = repo_root.to_path_buf();
    let front_sources = match scan::walk_files(&[&front_dir.join("src")], move |p| {
        is_source(&root3, p) && p.extension().is_some_and(|e| e == "rs")
    }) {
        Ok(f) => f,
        Err(cause) => return refuse(&mut o, "engine-layers could not walk the frontend", cause),
    };
    let front_manifest = match scan::walk_files(&[&front_dir.join("Cargo.toml")], |_| true) {
        Ok(f) => f,
        Err(cause) => {
            return refuse(
                &mut o,
                "engine-layers could not read the frontend manifest",
                cause,
            );
        }
    };

    let scanned = format!(
        "  scanned {} .rs file(s) + {CRATE_REL}/Cargo.toml, {} .rs file(s) under \
         {MAP_CRATE_REL}/src — of those {} under data/ ({} under data/scenario), {} under world/ \
         and {} under editing/ — plus {} .rs file(s) + Cargo.toml under {FRONTEND_REL}",
        sources.len(),
        map_sources.len(),
        data_files.len(),
        scenario_files.len(),
        world_files.len(),
        editing_files.len(),
        front_sources.len()
    );
    for (n, root) in [
        (sources.len(), format!("{CRATE_REL}/src")),
        (map_sources.len(), format!("{MAP_CRATE_REL}/src")),
        (data_files.len(), DATA_REL.to_string()),
        (world_files.len(), WORLD_REL.to_string()),
        (scenario_files.len(), SCENARIO_REL.to_string()),
        (editing_files.len(), EDITING_REL.to_string()),
        (front_sources.len(), format!("{FRONTEND_REL}/src")),
        (front_manifest.len(), format!("{FRONTEND_REL}/Cargo.toml")),
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
    boundary_evaluation::evaluate(
        repo_root,
        boundary_evaluation::BoundaryPatterns {
            decl,
            vocab,
            gpu,
            scenario_iso,
            data_side,
            world_side,
            dom,
            graphics_import,
        },
        boundary_evaluation::BoundarySources {
            sources,
            manifest_files,
            map_sources,
            scenario_files,
            editing_files,
            front_sources,
            front_manifest,
            data_files,
            world_files,
        },
        scanned,
        o,
    )
}
