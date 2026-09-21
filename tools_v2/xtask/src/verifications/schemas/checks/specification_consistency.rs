use super::*;

/// Every task name reachable as `cargo xtask ci|mk|db <name>`, from the LIVE dispatch tables.
///
/// Three tables because the three T-853 Phase 3 lanes landed in parallel worktrees and each picked
/// its own clap shape (`mk_build.rs` docs, T-895). Reading all three is what lets gate 7 resolve a
/// spec's `cargo xtask …` citation instead of merely eyeballing it.
pub(super) fn live_xtask_task_names() -> HashSet<String> {
    let mut out: HashSet<String> = crate::commands::ci::task_runner::TASKS
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    out.extend(
        crate::commands::build::recipes::TARGETS
            .iter()
            .map(|t| (*t).to_string()),
    );
    out.extend(
        crate::commands::db::operations::LANE_COMMANDS
            .iter()
            .map(|t| (*t).to_string()),
    );
    out
}

pub fn specification_consistency() -> Result<u8> {
    let root = repo_root()?;
    let spec = spec_dir(&root);
    let read = |p: PathBuf| -> Result<String> {
        fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))
    };

    let mut specification_files: Vec<String> = fs::read_dir(&spec)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("t090") && n.ends_with(".md"))
        .collect();
    specification_files.sort();
    let corpus: Vec<(String, String)> = specification_files
        .iter()
        .map(|n| Ok((n.clone(), read(spec.join(n))?)))
        .collect::<Result<_>>()?;

    let mut failures: Vec<String> = Vec::new();
    let mut fail = |gate: &str, msg: String| failures.push(format!("[{gate}] {msg}"));

    let window_has = |text: &str, i: usize, radius: usize, re: &regex::Regex| -> bool {
        let lo = i.saturating_sub(radius);
        let hi = (i + radius).min(text.len());
        // Snap to char boundaries.
        let lo = (lo..=i).find(|&b| text.is_char_boundary(b)).unwrap_or(i);
        let hi = (hi..text.len())
            .find(|&b| text.is_char_boundary(b))
            .unwrap_or(text.len());
        re.is_match(&text[lo..hi])
    };

    // Gate 1.
    let g1 = regex::RegexBuilder::new(r"Pick/select world objects \(future")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        if g1.is_match(text) {
            fail(
                "1",
                format!("{name}: contains forbidden \"Pick/select world objects (future...\""),
            );
        }
    }

    // Gate 2.
    let g2a = regex::RegexBuilder::new(r"reuse\s+slotClusterIndex")
        .case_insensitive(true)
        .build()?;
    let g2b = regex::RegexBuilder::new(r"separate\s+world")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        if g2a.is_match(text) && !g2b.is_match(text) {
            fail(
                "2",
                format!(
                    "{name}: \"reuse slotClusterIndex\" without \"separate world\" clarification"
                ),
            );
        }
    }

    // Gate 3 — tile-zoom LOD tokens need deckZoom context within 800 chars.
    let lod = regex::Regex::new(r"z\s*[≤≥<>]\s*[0-5]|\bz[0-5]\s*[-–]\s*z?[0-5]\b|\bz[0-5]\+")?;
    let zoom_ctx = regex::RegexBuilder::new(r"deckZoom|Deck orthographic")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        for m in lod.find_iter(text) {
            if !window_has(text, m.start(), 800, &zoom_ctx) {
                fail(
                    "3",
                    format!(
                        "{name}: tile-zoom LOD token \"{}\" without deckZoom/Deck-orthographic context within 800 chars",
                        m.as_str().trim()
                    ),
                );
            }
        }
    }

    // Gate 4 — "Deck pick"/"onHover" need forbidden-context within 220 chars.
    let pick_ctx =
        regex::RegexBuilder::new(r"forbidden|removed|never|no\s+deck|not\s+re-?enable|do\s+not")
            .case_insensitive(true)
            .build()?;
    let deck_pick = regex::RegexBuilder::new(r"Deck\s+pick")
        .case_insensitive(true)
        .build()?;
    let on_hover = regex::Regex::new(r"onHover")?;
    for (name, text) in &corpus {
        for re in [&deck_pick, &on_hover] {
            for m in re.find_iter(text) {
                if !window_has(text, m.start(), 220, &pick_ctx) {
                    fail(
                        "4",
                        format!(
                            "{name}: \"{}\" without forbidden/removed/never context within 220 chars",
                            m.as_str()
                        ),
                    );
                }
            }
        }
    }

    // Gate 5.
    let eng: String = read(spec.join("engineering_plan.md"))?
        .chars()
        .filter(|c| *c != '`' && *c != '*')
        .collect();
    let g5 = regex::RegexBuilder::new(r"Picking via Deck's onClick/onHover")
        .case_insensitive(true)
        .build()?;
    if g5.is_match(&eng) {
        fail(
            "5",
            "engineering_plan.md: still contains \"Picking via Deck's onClick/onHover\""
                .to_string(),
        );
    }

    // Gate 6.
    let hub = read(spec.join("t090_091_map_terrain_program.md"))?;
    let gap_ids = [
        "GAP-001", "GAP-002", "GAP-003", "GAP-004", "GAP-005", "GAP-H1", "GAP-H2", "GAP-H3",
        "GAP-H4", "GAP-H5", "GAP-H6", "GAP-H7", "GAP-H8", "GAP-M1", "GAP-M2", "GAP-M3", "GAP-M4",
        "GAP-M5", "GAP-M6", "GAP-M7",
    ];
    for id in gap_ids {
        if !hub.contains(id) {
            fail(
                "6",
                format!("t090_091_map_terrain_program.md: audit closure missing {id}"),
            );
        }
    }
    for low in ["L1", "L2", "L3", "L4", "L5"] {
        let re = regex::Regex::new(&format!(r"\b{low}\b"))?;
        if !re.is_match(&hub) {
            fail(
                "6",
                format!("t090_091_map_terrain_program.md: audit closure missing {low}"),
            );
        }
    }

    // Gate 7 — every referenced task exists.
    //
    // T-897 REPOINT. This read the root `Makefile` with `?`, so it was fail-CLOSED and would have
    // gone red the moment the file died — but red for the wrong reason, and the obvious repair
    // ("drop the make half") would have retired the check instead of moving it. Both halves moved:
    //
    //   * `cargo xtask <ci|mk|db> <name>` citations are resolved against the LIVE dispatch tables,
    //     so a typo or a renamed task in a spec is caught the way a dangling `make` target was;
    //   * a bare `make <target>` citation is now a FAILURE unless it names something in
    //     [`ARCHIVAL_MAKE_TARGETS`] — the frozen set that never had a live successor. There is no
    //     Makefile, so every other `make …` in the corpus is an instruction that cannot be run.
    //
    // The net effect is that the gate bites HARDER after the deletion than before it, which is the
    // bar T-853 sets for a check whose subject is removed.
    let make_targets: HashSet<String> = ARCHIVAL_MAKE_TARGETS
        .iter()
        .map(|t| (*t).to_string())
        .collect();
    let xtask_tasks: HashSet<String> = live_xtask_task_names();
    // T-165.9: the tbd-schema npm package is deleted (the Node eradication endpoint) — any
    // npm-script citation in the spec corpus is archival by definition, so the live-scripts
    // set is empty and the allowlist below carries every historically-cited name.
    let pkg_path = schema_root(&root).join("package.json");
    let mut npm_scripts: HashSet<String> = if pkg_path.exists() {
        read_json(&pkg_path)?["scripts"]
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        HashSet::new()
    };
    for s in [
        "dev",
        "build",
        "lint",
        "preview",
        "test",
        "format",
        "format:check",
    ] {
        npm_scripts.insert(s.to_string());
    }
    // Gate scripts retired to `cargo xtask schema …` at T-165.1/.2 — historical specs may still
    // quote the npm form (archival, not executable).
    for s in [
        "validate",
        "codegen",
        "verify-map-object-golden",
        "verify-map-glyphs",
        "verify-citations",
        "verify-map-object-enums",
        "verify-type-inventory",
        "verify-n6",
        "verify-n10",
        "verify-terrain-manifest",
        // retired with the T-165.4/.9 terrain + image lanes (package deleted at .9)
        "verify-terrain-alignment",
        "verify-terrain",
    ] {
        npm_scripts.insert(s.to_string());
    }
    let make_re = regex::Regex::new(r"\bmake\s+([a-z0-9]+(?:-[a-z0-9]+)+)")?;
    let npm_re = regex::Regex::new(r"\bnpm run ([a-z0-9:_-]+)")?;
    let xtask_re = regex::Regex::new(r"\bxtask\s+(?:ci|mk|db)\s+([a-z0-9]+(?:-[a-z0-9]+)*)")?;
    for (name, text) in &corpus {
        for c in make_re.captures_iter(text) {
            if !make_targets.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `make {}` — the root Makefile was deleted at T-897; \
                         cite the `cargo xtask …` spelling instead",
                        &c[1]
                    ),
                );
            }
        }
        for c in xtask_re.captures_iter(text) {
            if !xtask_tasks.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `xtask … {}` is not a task in mk_ci::TASKS / \
                         mk_build::TARGETS / the db lane",
                        &c[1]
                    ),
                );
            }
        }
        for c in npm_re.captures_iter(text) {
            if !npm_scripts.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `npm run {}` not in the historically-cited npm-script allowlist (Node was eradicated at T-165)",
                        &c[1]
                    ),
                );
            }
        }
    }

    // Gate 8 — no doc claims T-090.1 active.
    let authority = [
        root.join("CLAUDE.md"),
        spec.join("ROADMAP.md"),
        spec.join("agent_execution.md"),
        spec.join("engineering_plan.md"),
        root.join("docs/website/frontend/ROADMAP.md"),
        root.join("docs/website/frontend/INDEX.md"),
        root.join("docs/website/frontend/pages/mission-editor.md"),
        root.join("docs/mod/CLAUDE-CODE-START.md"),
    ];
    let mut gate8: Vec<(String, String)> = corpus.clone();
    for p in authority {
        let name = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
        let text = if p.exists() { read(p)? } else { String::new() };
        gate8.push((name, text));
    }
    let t0901 = regex::Regex::new(r"T-090\.1([^\d.]|\.\D|$)")?;
    let active = regex::RegexBuilder::new(r"\bactive\b")
        .case_insensitive(true)
        .build()?;
    let ok_ctx = regex::RegexBuilder::new(r"T-090\.3\.0|\bqueued\b|active\s+basemap")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &gate8 {
        for line in text.lines() {
            if !t0901.is_match(line) || !active.is_match(line) {
                continue;
            }
            if ok_ctx.is_match(line) {
                continue;
            }
            let trimmed: String = line.trim().chars().take(90).collect();
            fail(
                "8",
                format!("{name}: claims T-090.1 active — \"{trimmed}\""),
            );
        }
    }

    // Gate 9.
    let eden = read(spec.join("t090_eden_ai_world_object_schema.md"))?;
    let g9 = regex::RegexBuilder::new(r"move/delete this object")
        .case_insensitive(true)
        .build()?;
    if g9.is_match(&eden) {
        fail("9", "t090_eden_ai_world_object_schema.md: still says \"move/delete this object\" (mutation is Workbench-only)".to_string());
    }

    // Gate 10 — hub header names the registry active slice.
    let mut active_slice = "T-090.1.2.5".to_string();
    if let Ok(reg) = ticket_engine::registry::load_registry(&root)
        && let Some(t090) = reg["tickets"]
            .as_array()
            .and_then(|a| a.iter().find(|t| t["id"] == "T-090"))
        && let Some(s) = t090["active_slice"].as_str()
    {
        active_slice = s.to_string();
    }
    let header: String = hub.chars().take(800).collect();
    if !header.contains(&active_slice) {
        fail(
            "10",
            format!(
                "t090_091_map_terrain_program.md: header does not name {active_slice} as the active slice"
            ),
        );
    }

    // Gate 11.
    let inv_spec = read(spec.join("t090_world_object_type_inventory.md"))?;
    let range_re =
        regex::RegexBuilder::new(r"800k|900k|1\.2M|2k–20k|400k–900k|order-of-magnitude \(Everon")
            .case_insensitive(true)
            .build()?;
    let ok11 = regex::RegexBuilder::new(
        r"\bnever\b|forbidden|not a substitute|PENDING|hard-coded|no hard-",
    )
    .case_insensitive(true)
    .build()?;
    for line in inv_spec.lines() {
        if range_re.is_match(line) && !ok11.is_match(line) {
            let trimmed: String = line.trim().chars().take(90).collect();
            fail(
                "11",
                format!(
                    "t090_world_object_type_inventory.md: Everon estimate range — \"{trimmed}\""
                ),
            );
        }
    }
    if !inv_spec.contains("censusStatus") || !inv_spec.contains("pending_export") {
        fail("11", "t090_world_object_type_inventory.md: must document censusStatus pending_export baseline".to_string());
    }

    // Gate 12 — phase-budget rows must cite inventory tokens, not hard-coded counts.
    let budget = regex::Regex::new(r"~?\d+(\.\d+)?\s*[kM]\b|\d{1,3},\d{3}")?;
    let inv_tok = regex::Regex::new(r"byKind|levels\.|inventory|derived")?;
    let p_row = regex::Regex::new(r"^\|\s*P\d+")?;
    for (name, text) in &corpus {
        for line in text.lines() {
            if !p_row.is_match(line) {
                continue;
            }
            if budget.is_match(line) && !inv_tok.is_match(line) {
                let trimmed: String = line.trim().chars().take(90).collect();
                fail(
                    "12",
                    format!("{name}: phase-budget row hard-codes a count — \"{trimmed}\""),
                );
            }
        }
    }

    if failures.is_empty() {
        println!(
            "specification-consistency: OK ({} spec files + authority docs, all 12 gates pass)",
            specification_files.len()
        );
        Ok(0)
    } else {
        eprintln!("specification-consistency: FAIL ({})", failures.len());
        for f in &failures {
            eprintln!("  {f}");
        }
        Ok(1)
    }
}
