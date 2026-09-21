use super::*;

pub(super) fn evaluate(
    repo_root: &Path,
    patterns: BoundaryPatterns,
    sources: BoundarySources,
    scanned: String,
    mut o: Vec<String>,
) -> (u8, Vec<String>) {
    let BoundaryPatterns {
        decl,
        vocab,
        gpu,
        scenario_iso,
        data_side,
        world_side,
        dom,
        graphics_import,
    } = patterns;
    let BoundarySources {
        sources,
        manifest_files,
        map_sources,
        scenario_files,
        editing_files,
        front_sources,
        front_manifest,
        data_files,
        world_files,
    } = sources;

    o.push(RULE1_HEAD.to_string());
    let path_use = Pattern::literal(MAP_ENGINE_PATH);
    let mut breaches: Vec<String> = Vec::new();
    match scan::matching_lines(&path_use, &sources) {
        Ok(hits) => breaches.extend(hits.iter().map(|h| rel(repo_root, h))),
        Err(cause) => return refuse(&mut o, "engine-layers rule 1 scan", cause),
    }
    // The manifest arm closes the one hole the source arm has: `[dependencies] r = { package =
    // "website-map-engine" }` renames the crate, and every `use r::…` then spells something this
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
        o.push("FAIL: graphics-engine reaches back into the map engine:".to_string());
        o.extend(breaches.iter().map(|b| format!("  {b}")));
        say(&mut o, RULE1_TAIL);
    }

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

    // ── rule 4 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE4_HEAD.to_string());
    let iso_hits = match scan::matching_lines(&scenario_iso, &scenario_files) {
        Ok(hits) => hits,
        Err(cause) => return refuse(&mut o, "engine-layers rule 4 scan", cause),
    };
    let (iso_findings, iso_bad) = against_pin(repo_root, &iso_hits, RULE4_PIN);
    let iso_total: usize = RULE4_PIN.iter().map(|(_, n, _)| *n).sum();
    if iso_bad.is_empty() {
        o.push(format!(
            "  OK — {iso_total} pinned site(s) in {} file(s), 0 unpinned. Production code reaches \
             outside data/scenario nowhere; the pinned residue is cfg-gated test code:",
            RULE4_PIN.len()
        ));
        for (file, n, why) in RULE4_PIN {
            o.push(format!("    {file} ({n}) — {why}"));
        }
    } else {
        o.push("FAIL: the authored mission reaches outside its own tree:".to_string());
        o.extend(iso_bad.iter().cloned());
        say(&mut o, RULE4_TAIL);
    }

    // ── rule 5 ───────────────────────────────────────────────────────────────────────────────
    //
    // Hard zero, no allowlist. See the module docs for why the subject is one directory and not
    // the crate, and why the matcher is the bare word inside it.
    o.push(RULE5_HEAD.to_string());
    let dom_hits: Vec<String> = match scan::matching_lines(&dom, &editing_files) {
        Ok(hits) => hits.iter().map(|h| rel(repo_root, h)).collect(),
        Err(cause) => return refuse(&mut o, "engine-layers rule 5 scan", cause),
    };
    if dom_hits.is_empty() {
        o.push(format!(
            "  OK — 0 site(s) across {} .rs file(s) under editing/: the editor's decisions name \
             no browser, so `cargo test` can answer every one of them.",
            editing_files.len()
        ));
    } else {
        o.push("FAIL: the browser reached into the engine's editing tree:".to_string());
        o.extend(dom_hits.iter().map(|h| format!("  {h}")));
        say(&mut o, RULE5_TAIL);
    }

    // ── rule 6 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE6_HEAD.to_string());
    let mut direct: Vec<String> = Vec::new();
    match scan::matching_lines(&graphics_import, &front_sources) {
        Ok(hits) => direct.extend(hits.iter().map(|h| rel(repo_root, h))),
        Err(cause) => return refuse(&mut o, "engine-layers rule 6 scan", cause),
    }
    // The manifest arm closes rule 1's hole from the other side: a renamed dependency
    // (`g = { package = "website-graphics-engine" }`) makes every `use g::…` invisible to the
    // source arm. A `#` line is a comment — naming the renderer in prose is not an edge.
    match scan::matching_lines(&Pattern::literal(GRAPHICS_PKG), &front_manifest) {
        Ok(hits) => direct.extend(
            hits.iter()
                .filter(|h| !h.line.trim_start().starts_with('#'))
                .map(|h| rel(repo_root, h)),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 6 manifest scan", cause),
    }
    if direct.is_empty() {
        o.push(format!(
            "  OK — 0 import(s) across {} .rs file(s) and the manifest: the frontend reaches the \
             renderer through website-map-engine and only through it.",
            front_sources.len()
        ));
    } else {
        o.push("FAIL: the frontend imports the renderer directly:".to_string());
        o.extend(direct.iter().map(|d| format!("  {d}")));
        say(&mut o, RULE6_TAIL);
    }

    // ── rule 7 ───────────────────────────────────────────────────────────────────────────────
    //
    // One rule, two directions, one findings list — a breach in either direction is the same
    // wall coming down, and reporting it as two rules would let half of it read green.
    o.push(RULE7_HEAD.to_string());
    let mut wall: Vec<String> = Vec::new();
    match scan::matching_lines(&data_side, &data_files) {
        Ok(hits) => wall.extend(
            hits.iter()
                .map(|h| format!("  data/ names the world — {}", rel(repo_root, h))),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 7 data scan", cause),
    }
    match scan::matching_lines(&world_side, &world_files) {
        Ok(hits) => wall.extend(
            hits.iter()
                .map(|h| format!("  world/ names the document — {}", rel(repo_root, h))),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 7 world scan", cause),
    }
    if wall.is_empty() {
        o.push(format!(
            "  OK — 0 site(s) in both directions: {} .rs file(s) under data/ name no world \
             module, {} under world/ name neither crate::data nor yrs.",
            data_files.len(),
            world_files.len()
        ));
    } else {
        o.push("FAIL: the world/data wall is breached:".to_string());
        o.extend(wall.iter().cloned());
        say(&mut o, RULE7_TAIL);
    }

    o.push(scanned);
    if breaches.is_empty()
        && nouns.is_empty()
        && vocab_bad.is_empty()
        && gpu_bad.is_empty()
        && iso_bad.is_empty()
        && dom_hits.is_empty()
        && direct.is_empty()
        && wall.is_empty()
    {
        o.push("ENGINE-LAYERS: PASS".to_string());
        return (0, o);
    }
    o.push(format!(
        "ENGINE-LAYERS: FAIL — {} wall breach(es), {} map-noun declaration(s), \
         {vocab_findings} frame-vocab finding(s), {gpu_findings} GPU-module finding(s), \
         {iso_findings} scenario-isolation finding(s), {} browser-in-editing site(s), \
         {} direct-renderer import(s), {} world/data finding(s)",
        breaches.len(),
        nouns.len(),
        dom_hits.len(),
        direct.len(),
        wall.len(),
    ));
    (1, o)
}

pub(super) struct BoundaryPatterns {
    pub(super) decl: Pattern,
    pub(super) vocab: Pattern,
    pub(super) gpu: Pattern,
    pub(super) scenario_iso: Pattern,
    pub(super) data_side: Pattern,
    pub(super) world_side: Pattern,
    pub(super) dom: Pattern,
    pub(super) graphics_import: Pattern,
}

pub(super) struct BoundarySources {
    pub(super) sources: Vec<std::path::PathBuf>,
    pub(super) manifest_files: Vec<std::path::PathBuf>,
    pub(super) map_sources: Vec<std::path::PathBuf>,
    pub(super) scenario_files: Vec<std::path::PathBuf>,
    pub(super) editing_files: Vec<std::path::PathBuf>,
    pub(super) front_sources: Vec<std::path::PathBuf>,
    pub(super) front_manifest: Vec<std::path::PathBuf>,
    pub(super) data_files: Vec<std::path::PathBuf>,
    pub(super) world_files: Vec<std::path::PathBuf>,
}
