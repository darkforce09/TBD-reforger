//! The engine-layer judgement: rules 1, 2, 3a, 3b, 6 and 7 over the walked files, in report
//! order.
//!
//! **Role:** runs each rule's matcher over its subset of the walked files, writes the rule's
//! headline, its `OK` line or its findings and remedy, records the rule's finding count, and
//! closes the report with `ENGINE-LAYERS: PASS` or the `FAIL` tally.
//! **Position:** the last step of [`super::check_engine_layers`], after
//! [`super::matcher_probes::probe_matchers`] and [`super::crate_walks::walk_crates`].
//! **Signals & state:** appends to the report buffer and the rule results it is handed.
//! **Invariants:** every rule reports, in order, whether or not an earlier one failed; a file
//! that cannot be read stops the run as "did not run" (exit 2) with the rules judged so far
//! recorded.

use std::path::Path;

use super::matcher_probes::probe_wasm_only_graphics_import;
use super::report_text::*;
use super::rules::*;
use super::scanning::{against_pin, refuse, rel, say};
use super::{EngineLayerRule, EngineLayerRuleResult};
use verification_core::{Pattern, scan};

/// Judge every rule; the exit code and the complete report lines.
pub(super) fn evaluate(
    repo_root: &Path,
    patterns: BoundaryPatterns,
    sources: BoundarySources,
    scanned: String,
    mut o: Vec<String>,
    results: &mut Vec<EngineLayerRuleResult>,
) -> (u8, Vec<String>) {
    let BoundaryPatterns {
        map_engine_import,
        decl,
        vocab,
        gpu,
        world_side,
        graphics_import,
    } = patterns;
    let BoundarySources {
        sources,
        manifest_files,
        wasm_only_graphics,
        map_sources,
        front_sources,
        front_manifest,
        world_files,
    } = sources;

    o.push(RULE1_HEAD.to_string());
    let mut breaches: Vec<String> = Vec::new();
    match scan::matching_lines(&map_engine_import, &sources) {
        Ok(hits) => breaches.extend(hits.iter().map(|h| rel(repo_root, h))),
        Err(cause) => return refuse(&mut o, "engine-layers rule 1 scan", cause),
    }
    // The manifest arm closes the one hole the source arm has: `[dependencies] r = { package =
    // "map_engine" }` renames the crate, and every `use r::…` then spells something this
    // gate has never heard of. A `#` line is a comment — naming the other crate in prose is not a
    // dependency edge.
    match scan::matching_lines(&Pattern::literal(MAP_ENGINE_PKG), &manifest_files) {
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
        o.push("FAIL: the graphics layer reaches back into the map engine:".to_string());
        o.extend(breaches.iter().map(|b| format!("  {b}")));
        say(&mut o, RULE1_TAIL);
    }

    results.push(EngineLayerRuleResult::new(
        EngineLayerRule::GraphicsEngineImportsNoMapEngine,
        breaches.len(),
        &breaches,
    ));

    // ── rule 2 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE2_HEAD.to_string());
    let nouns: Vec<String> = match scan::matching_lines(&decl, &sources) {
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

    results.push(EngineLayerRuleResult::new(
        EngineLayerRule::GraphicsEngineDeclaresNoMapNoun,
        nouns.len(),
        &nouns,
    ));

    // ── rule 3a ──────────────────────────────────────────────────────────────────────────────
    o.push(RULE3A_HEAD.to_string());
    let vocab_hits = match scan::matching_lines(&vocab, &map_sources) {
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

    results.push(EngineLayerRuleResult::new(
        EngineLayerRule::FrameVocabularyStaysAtThePacketBoundary,
        vocab_findings,
        &vocab_bad,
    ));

    // ── rule 3b ──────────────────────────────────────────────────────────────────────────────
    o.push(RULE3B_HEAD.to_string());
    let gpu_hits = match scan::matching_lines(&gpu, &map_sources) {
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

    results.push(EngineLayerRuleResult::new(
        EngineLayerRule::GpuResourceModulesStayPinned,
        gpu_findings,
        &gpu_bad,
    ));

    // ── rule 6 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE6_HEAD.to_string());
    let mut direct: Vec<String> = Vec::new();
    match scan::matching_lines(&graphics_import, &front_sources) {
        Ok(hits) => direct.extend(hits.iter().map(|h| rel(repo_root, h))),
        Err(cause) => return refuse(&mut o, "engine-layers rule 6 scan", cause),
    }
    // The manifest arm closes rule 1's hole from the other side: a renamed dependency
    // (`g = { package = "graphics_engine" }`) makes every `use g::…` invisible to the
    // source arm. A `#` line is a comment — naming the renderer in prose is not an edge.
    match scan::matching_lines(&Pattern::literal(GRAPHICS_PKG), &front_manifest) {
        Ok(hits) => direct.extend(
            hits.iter()
                .filter(|h| !h.line.trim_start().starts_with('#'))
                .map(|h| rel(repo_root, h)),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 6 manifest scan", cause),
    }
    // The wasm-only members of the graphics category are GPU-side code the frontend may not reach
    // either; the CPU-only members (`targets = "any"`) are shared building blocks it may link.
    // Same two arms, spelled with each member's own crate and package names.
    if !wasm_only_graphics.is_empty() {
        let identifiers: Vec<String> = wasm_only_graphics
            .iter()
            .map(|c| c.identifier.clone())
            .collect();
        let wasm_only_import = match probe_wasm_only_graphics_import(&mut o, &identifiers) {
            Ok(pattern) => pattern,
            Err(refusal) => return refusal,
        };
        match scan::matching_lines(&wasm_only_import, &front_sources) {
            Ok(hits) => direct.extend(hits.iter().map(|h| rel(repo_root, h))),
            Err(cause) => return refuse(&mut o, "engine-layers rule 6 wasm-only scan", cause),
        }
        for graphics_crate in &wasm_only_graphics {
            let package = Pattern::literal(&graphics_crate.package_name);
            match scan::matching_lines(&package, &front_manifest) {
                Ok(hits) => direct.extend(
                    hits.iter()
                        .filter(|h| !h.line.trim_start().starts_with('#'))
                        .map(|h| rel(repo_root, h)),
                ),
                Err(cause) => {
                    return refuse(
                        &mut o,
                        "engine-layers rule 6 wasm-only manifest scan",
                        cause,
                    );
                }
            }
        }
    }
    if direct.is_empty() {
        o.push(format!(
            "  OK — 0 import(s) across {} .rs file(s) and the manifest: the frontend reaches the \
             renderer through map_engine and only through it.",
            front_sources.len()
        ));
    } else {
        o.push("FAIL: the frontend imports the renderer directly:".to_string());
        o.extend(direct.iter().map(|d| format!("  {d}")));
        say(&mut o, RULE6_TAIL);
    }

    results.push(EngineLayerRuleResult::new(
        EngineLayerRule::FrontendImportsNoRenderer,
        direct.len(),
        &direct,
    ));

    // ── rule 7 ───────────────────────────────────────────────────────────────────────────────
    //
    // The document's side of the wall is the mission crates', and the crate-tier law judges their
    // edges; this rule judges the world's side, inside the one crate that holds both.
    o.push(RULE7_HEAD.to_string());
    let wall: Vec<String> = match scan::matching_lines(&world_side, &world_files) {
        Ok(hits) => hits
            .iter()
            .map(|h| format!("  world/ names the document — {}", rel(repo_root, h)))
            .collect(),
        Err(cause) => return refuse(&mut o, "engine-layers rule 7 world scan", cause),
    };
    if wall.is_empty() {
        o.push(format!(
            "  OK — 0 site(s) across {} .rs file(s) under world/: the static world names neither \
             yrs, nor a mission document crate, nor a mission-editing crate.",
            world_files.len()
        ));
    } else {
        o.push("FAIL: the static world names the authored document:".to_string());
        o.extend(wall.iter().cloned());
        say(&mut o, RULE7_TAIL);
    }

    results.push(EngineLayerRuleResult::new(
        EngineLayerRule::WorldAndDocumentShareNothing,
        wall.len(),
        &wall,
    ));

    o.push(scanned);
    if breaches.is_empty()
        && nouns.is_empty()
        && vocab_bad.is_empty()
        && gpu_bad.is_empty()
        && direct.is_empty()
        && wall.is_empty()
    {
        o.push("ENGINE-LAYERS: PASS".to_string());
        return (0, o);
    }
    o.push(format!(
        "ENGINE-LAYERS: FAIL — {} wall breach(es), {} map-noun declaration(s), \
         {vocab_findings} frame-vocab finding(s), {gpu_findings} GPU-module finding(s), \
         {} direct-renderer import(s), {} world-document finding(s)",
        breaches.len(),
        nouns.len(),
        direct.len(),
        wall.len(),
    ));
    (1, o)
}

/// The six proved matchers, one per rule scan (rule 1's manifest arm matches literals).
pub(super) struct BoundaryPatterns {
    pub(super) map_engine_import: Pattern,
    pub(super) decl: Pattern,
    pub(super) vocab: Pattern,
    pub(super) gpu: Pattern,
    pub(super) world_side: Pattern,
    pub(super) graphics_import: Pattern,
}

/// The walked files each rule scans: the graphics layer (the parked graphics engine and every
/// graphics-category member) with its manifests, the map engine and its `world/` subset, and the
/// frontend with its manifest.
pub(super) struct BoundarySources {
    pub(super) sources: Vec<std::path::PathBuf>,
    pub(super) manifest_files: Vec<std::path::PathBuf>,
    pub(super) wasm_only_graphics: Vec<WasmOnlyGraphicsCrate>,
    pub(super) map_sources: Vec<std::path::PathBuf>,
    pub(super) front_sources: Vec<std::path::PathBuf>,
    pub(super) front_manifest: Vec<std::path::PathBuf>,
    pub(super) world_files: Vec<std::path::PathBuf>,
}

/// A graphics-category member declaring `targets = "wasm32"`, which rule 6 forbids the frontend
/// to import.
pub(super) struct WasmOnlyGraphicsCrate {
    /// `[package] name`, the spelling of a dependency edge in a manifest.
    pub(super) package_name: String,
    /// The package name with `-` as `_`, the spelling of a path in source.
    pub(super) identifier: String,
}
