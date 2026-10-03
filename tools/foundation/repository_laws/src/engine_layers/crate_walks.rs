//! The engine-layer walks: the crates the rules read, and the counts that prove it.
//!
//! **Role:** walks the graphics layer (the parked graphics engine and every workspace member of
//! the graphics category, each with its manifest), the map engine and the frontend, splits the
//! map engine into the `world/` and `editing/` subsets the rules scan,
//! names the wasm-only graphics crates rule 6 guards, and writes the `scanned …` line.
//! **Position:** the second step of [`super::check_engine_layers`], after
//! [`super::matcher_probes::probe_matchers`]; its output feeds [`super::evaluation::evaluate`].
//! **Signals & state:** appends refusal lines to the report buffer it is handed; nothing else.
//! **Invariants:** a missing root or manifest, or a workspace member list that cannot be read, is
//! a check that did not run (exit 2), and a root or subset holding no file — a graphics category
//! with no member included — is a failure (exit 1): "nothing breaches the wall" and "there is no
//! wall to look at" never read the same.

use std::path::{Path, PathBuf};

use super::evaluation::{BoundarySources, WasmOnlyGraphicsCrate};
use super::report_text::NOTHING_TAIL;
use super::rules::*;
use super::scanning::{is_source, refuse, say, under};
use crate::workspace_laws::crate_layout::{TargetPlatforms, declared_targets, effective_category};
use crate::workspace_members::{WorkspaceMember, read_workspace_members};
use verification_core::{NotRun, scan};

/// Walk every root; on success, the walked files and the `scanned …` line, on refusal, the exit
/// code and the report lines so far.
pub(super) fn walk_crates(
    repo_root: &Path,
    o: &mut Vec<String>,
) -> Result<(BoundarySources, String), (u8, Vec<String>)> {
    let engine_dir = repo_root.join(GRAPHICS_ENGINE_REL);
    let mut sources = rust_files(repo_root, &engine_dir.join("src"))
        .map_err(|cause| refuse(o, "engine-layers could not walk the crate", cause))?;
    // The manifest is walked rather than read so a missing one is the same `NotRun` as a missing
    // src/ — one refusal shape for the whole gate, not two.
    let mut manifest_files = scan::walk_files(&[&engine_dir.join("Cargo.toml")], |_| true)
        .map_err(|cause| refuse(o, "engine-layers could not read the manifest", cause))?;
    let engine_source_count = sources.len();

    // Rules 1 and 2 scan the whole graphics layer: the parked renderer above and every member of
    // the graphics category here, read from the workspace members so a crate born into the
    // category is judged from its first commit. Rule 6 guards the wasm-only members among them.
    let members = graphics_category_members(repo_root).map_err(|cause| {
        refuse(
            o,
            "engine-layers could not read the workspace members",
            cause,
        )
    })?;
    if members.is_empty() {
        o.push(format!(
            "FAIL: engine-layers found no workspace member in {GRAPHICS_CATEGORY} — refusing a \
             vacuous pass."
        ));
        say(o, NOTHING_TAIL);
        o.push("ENGINE-LAYERS: FAIL (no inputs)".to_string());
        return Err((1, std::mem::take(o)));
    }
    let mut member_counts: Vec<(usize, String)> = Vec::new();
    let mut wasm_only_graphics: Vec<WasmOnlyGraphicsCrate> = Vec::new();
    for member in &members {
        let member_dir = repo_root.join(&member.path);
        let files = rust_files(repo_root, &member_dir.join("src"))
            .map_err(|cause| refuse(o, "engine-layers could not walk a graphics crate", cause))?;
        let manifest =
            scan::walk_files(&[&member_dir.join("Cargo.toml")], |_| true).map_err(|cause| {
                refuse(
                    o,
                    "engine-layers could not read a graphics crate manifest",
                    cause,
                )
            })?;
        member_counts.push((files.len(), format!("{}/src", member.path)));
        sources.extend(files);
        manifest_files.extend(manifest);
        if declared_targets(member) == Some(TargetPlatforms::Wasm32) {
            wasm_only_graphics.push(WasmOnlyGraphicsCrate {
                package_name: member.package_name.clone(),
                identifier: member.crate_identifier(),
            });
        }
    }

    // The map engine is the root of rules 3a, 3b, 5 and 7, and the check refuses to run
    // without it.
    let map_sources = rust_files(repo_root, &repo_root.join(MAP_CRATE_REL).join("src"))
        .map_err(|cause| refuse(o, "engine-layers could not walk the map engine", cause))?;

    // Rules 5 and 7 scan two subtrees of the walk rules 3a/3b already do, rather than walking
    // the disk twice more. Each is its own anti-vacuity subject below: "no file under `world/`
    // names the document" and "there is no `world/` any more" are the same sentence to a matcher,
    // and only the count tells them apart.
    let world_files = under(repo_root, &map_sources, WORLD_REL);
    let editing_files = under(repo_root, &map_sources, EDITING_REL);

    // Rule 6's root is the frontend, walked here because no earlier rule reads it. Its manifest
    // rides the same walk as its sources, exactly as rule 1's does for the graphics layer.
    let front_dir = repo_root.join(FRONTEND_REL);
    let front_sources = rust_files(repo_root, &front_dir.join("src"))
        .map_err(|cause| refuse(o, "engine-layers could not walk the frontend", cause))?;
    let front_manifest =
        scan::walk_files(&[&front_dir.join("Cargo.toml")], |_| true).map_err(|cause| {
            refuse(
                o,
                "engine-layers could not read the frontend manifest",
                cause,
            )
        })?;

    let member_source_count = sources.len() - engine_source_count;
    let scanned = format!(
        "  scanned {engine_source_count} .rs file(s) + {GRAPHICS_ENGINE_REL}/Cargo.toml, \
         {member_source_count} .rs file(s) + Cargo.toml across {} {GRAPHICS_CATEGORY} member(s) \
         ({} wasm-only), {} .rs file(s) under {MAP_CRATE_REL}/src — of those {} under world/ and \
         {} under editing/ — plus {} .rs file(s) + Cargo.toml under {FRONTEND_REL}",
        members.len(),
        wasm_only_graphics.len(),
        map_sources.len(),
        world_files.len(),
        editing_files.len(),
        front_sources.len()
    );
    let mut roots = vec![(engine_source_count, format!("{GRAPHICS_ENGINE_REL}/src"))];
    roots.extend(member_counts);
    roots.extend([
        (map_sources.len(), format!("{MAP_CRATE_REL}/src")),
        (world_files.len(), WORLD_REL.to_string()),
        (editing_files.len(), EDITING_REL.to_string()),
        (front_sources.len(), format!("{FRONTEND_REL}/src")),
        (front_manifest.len(), format!("{FRONTEND_REL}/Cargo.toml")),
    ]);
    for (n, root) in roots {
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
            wasm_only_graphics,
            map_sources,
            editing_files,
            front_sources,
            front_manifest,
            world_files,
        },
        scanned,
    ))
}

/// Every `.rs` file under `root` that is source rather than build output.
fn rust_files(repo_root: &Path, root: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let checkout = repo_root.to_path_buf();
    scan::walk_files(&[root], move |p| {
        is_source(&checkout, p) && p.extension().is_some_and(|e| e == "rs")
    })
}

/// Every workspace member whose category is [`GRAPHICS_CATEGORY`], sorted by path.
fn graphics_category_members(repo_root: &Path) -> Result<Vec<WorkspaceMember>, NotRun> {
    Ok(read_workspace_members(repo_root)?
        .into_iter()
        .filter(|member| effective_category(member) == GRAPHICS_CATEGORY)
        .collect())
}
