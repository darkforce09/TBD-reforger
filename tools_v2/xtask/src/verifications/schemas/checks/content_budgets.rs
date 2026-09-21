use super::*;

/// Where the building-geometry sentence starts and ends inside the prose that carries it.
///
/// The sentence itself is not written here. `map-object-prefab.schema.json` states it, and this
/// gate reads it from there, so the contract and the documents that repeat it have exactly one
/// text between them. Pinning a copy in Rust would make a sixth place to keep in step, and the
/// drift the gate exists to catch would then be able to start here.
const GEOMETRY_SENTENCE_OPENING: &str = "oriented bounding rectangle from spatial.halfExtentsM";
const GEOMETRY_SENTENCE_CLOSING: &str = "supersede OBB rectangles for render.";

/// The shortest span that can still be the whole sentence. A definition whose opening and closing
/// anchors sit closer together than this states something else, and the gate refuses rather than
/// certifying four documents against a fragment.
const GEOMETRY_SENTENCE_MIN_CHARS: usize = 120;

pub fn n6_sentence() -> Result<u8> {
    let root = repo_root()?;
    let norm = |s: &str| -> String {
        let stripped: String = s.chars().filter(|c| *c != '`' && *c != '*').collect();
        stripped.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let definition = definition_path(&root, "map-object-prefab.schema.json");
    let stated = norm(
        &fs::read_to_string(&definition)
            .with_context(|| format!("read {}", definition.display()))?,
    );
    let opening = stated.find(GEOMETRY_SENTENCE_OPENING).with_context(|| {
        format!(
            "{} states no building-geometry sentence: it does not contain \
             \"{GEOMETRY_SENTENCE_OPENING}\"",
            definition.display()
        )
    })?;
    let closing = stated[opening..]
        .find(GEOMETRY_SENTENCE_CLOSING)
        .with_context(|| {
            format!(
                "{} opens the building-geometry sentence and never closes it: no \
                 \"{GEOMETRY_SENTENCE_CLOSING}\" after the opening",
                definition.display()
            )
        })?;
    let core = &stated[opening..opening + closing + GEOMETRY_SENTENCE_CLOSING.len()];
    anyhow::ensure!(
        core.chars().count() >= GEOMETRY_SENTENCE_MIN_CHARS,
        "{} states a {}-character building-geometry sentence, below the {} the gate accepts",
        definition.display(),
        core.chars().count(),
        GEOMETRY_SENTENCE_MIN_CHARS
    );

    let spec = spec_dir(&root);
    let repeaters = [
        spec.join("t090_2_map_object_taxonomy.md"),
        spec.join("t090_5_map_object_render_layer.md"),
        spec.join("t090_6_geometry_placement_audit.md"),
        spec.join("t090_world_object_glyphs.md"),
    ];
    let mut missing = Vec::new();
    for f in &repeaters {
        let text = fs::read_to_string(f).with_context(|| format!("read {}", f.display()))?;
        if !norm(&text).contains(core) {
            missing.push(f.strip_prefix(&root).unwrap_or(f).display().to_string());
        }
    }
    if missing.is_empty() {
        println!(
            "verify-n6-sentence: OK (the contract's building-geometry sentence is repeated \
             verbatim by {} specification documents)",
            repeaters.len()
        );
        Ok(0)
    } else {
        eprintln!(
            "verify-n6-sentence: FAIL — these documents state the building-geometry sentence \
             differently from {}:",
            definition
                .strip_prefix(&root)
                .unwrap_or(&definition)
                .display()
        );
        for m in &missing {
            eprintln!("  {m}");
        }
        Ok(1)
    }
}

pub fn n10_tile_budget() -> Result<u8> {
    let root = repo_root()?;
    let spec = spec_dir(&root);
    // Dash-agnostic (figure/en/em → hyphen), mirroring the Node normalizer.
    let norm = |name: &str| -> Result<String> {
        let raw = fs::read_to_string(spec.join(name)).with_context(|| name.to_string())?;
        Ok(raw
            .chars()
            .map(|c| match c {
                '\u{2012}'..='\u{2015}' => '-',
                other => other,
            })
            .collect())
    };
    let canonical = [
        "200-400 MB",
        "400-800 MB",
        "512 tiles",
        "Max concurrent tile fetches",
        "one basemap pyramid",
    ];
    let forbidden = ["1.6 GB", "200-800 MB"];
    let mut errors = Vec::new();
    for f in [
        "t090_basemap_dual_view.md",
        "t090_terrain_export_pipeline.md",
    ] {
        let text = norm(f)?;
        for row in canonical {
            if !text.contains(row) {
                errors.push(format!("{f}: N10 row missing \"{row}\""));
            }
        }
    }
    for entry in fs::read_dir(&spec)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if !(name.starts_with("t090") && name.ends_with(".md")) {
            continue;
        }
        let text = norm(&name)?;
        for bad in forbidden {
            if text.contains(bad) {
                errors.push(format!(
                    "{name}: restates conflicting tile budget \"{bad}\" (N10 is single source)"
                ));
            }
        }
    }
    if errors.is_empty() {
        println!(
            "verify-n10-tile-budget: OK (N10 tile-budget single-source across basemap + pipeline)"
        );
        Ok(0)
    } else {
        eprintln!("verify-n10-tile-budget: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}
