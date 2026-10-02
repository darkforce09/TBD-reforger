//! The engine-layer walks: the three crates the rules read, and the counts that prove it.
//!
//! **Role:** walks the graphics engine, the map engine and the frontend, splits the map engine
//! into the `data/`, `data/scenario/`, `world/` and `editing/` subsets the rules scan, and writes
//! the `scanned …` line.
//! **Position:** the second step of [`super::check_engine_layers`], after
//! [`super::matcher_probes::probe_matchers`]; its output feeds [`super::evaluation::evaluate`].
//! **Signals & state:** appends refusal lines to the report buffer it is handed; nothing else.
//! **Invariants:** a missing root or manifest is a check that did not run (exit 2), and a root or
//! subset holding no file is a failure (exit 1) — "nothing breaches the wall" and "there is no
//! wall to look at" never read the same.

use std::path::Path;

use super::evaluation::BoundarySources;
use super::report_text::NOTHING_TAIL;
use super::rules::*;
use super::scanning::{is_source, refuse, say, under};
use crate::scan;

/// Walk every root; on success, the walked files and the `scanned …` line, on refusal, the exit
/// code and the report lines so far.
pub(super) fn walk_crates(
    repo_root: &Path,
    o: &mut Vec<String>,
) -> Result<(BoundarySources, String), (u8, Vec<String>)> {
    let crate_dir = repo_root.join(CRATE_REL);
    let manifest = crate_dir.join("Cargo.toml");
    let src = crate_dir.join("src");

    let root = repo_root.to_path_buf();
    let sources = match scan::walk_files(&[&src], move |p| {
        is_source(&root, p) && p.extension().is_some_and(|e| e == "rs")
    }) {
        Ok(f) => f,
        Err(cause) => return Err(refuse(o, "engine-layers could not walk the crate", cause)),
    };
    // The manifest is walked rather than read so a missing one is the same `NotRun` as a missing
    // src/ — one refusal shape for the whole gate, not two.
    let manifest_files = match scan::walk_files(&[&manifest], |_| true) {
        Ok(f) => f,
        Err(cause) => {
            return Err(refuse(
                o,
                "engine-layers could not read the manifest",
                cause,
            ));
        }
    };

    // The map engine is the root of rules 3a, 3b, 4, 5 and 7; rules 1 and 2 scan graphics-engine
    // alone, and the check refuses to run without either crate.
    let map_src = repo_root.join(MAP_CRATE_REL).join("src");
    let root2 = repo_root.to_path_buf();
    let map_sources = match scan::walk_files(&[&map_src], move |p| {
        is_source(&root2, p) && p.extension().is_some_and(|e| e == "rs")
    }) {
        Ok(f) => f,
        Err(cause) => {
            return Err(refuse(
                o,
                "engine-layers could not walk the map engine",
                cause,
            ));
        }
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
        Err(cause) => {
            return Err(refuse(
                o,
                "engine-layers could not walk the frontend",
                cause,
            ));
        }
    };
    let front_manifest = match scan::walk_files(&[&front_dir.join("Cargo.toml")], |_| true) {
        Ok(f) => f,
        Err(cause) => {
            return Err(refuse(
                o,
                "engine-layers could not read the frontend manifest",
                cause,
            ));
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
            say(o, NOTHING_TAIL);
            o.push("ENGINE-LAYERS: FAIL (no inputs)".to_string());
            return Err((1, std::mem::take(o)));
        }
    }

    Ok((
        BoundarySources {
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
    ))
}
