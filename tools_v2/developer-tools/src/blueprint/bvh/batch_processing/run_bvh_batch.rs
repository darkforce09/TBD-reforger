use super::*;
use crate::repository_layout::{definition_path, terrain_dir};

/// `map bvh-batch --prefab <Prefabs/…/X.et> [--slug <s>] [--out <dir>] [--paks <dir>]
/// [--extract <dir>] [--scene <spec.json>] [--kind <record>=<kind>]… [--dry-run]`
pub fn run_bvh_batch(root: &std::path::Path, args: &[String]) -> Result<u8> {
    // T-090.12.2 — the whole-catalogue lane lives in `library.rs`.
    if args.iter().any(|a| a == "--all-prefabs") {
        return super::super::library_cli::run(root, args);
    }
    let mut prefab: Option<String> = None;
    let mut slug: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut paks: Option<PathBuf> = None;
    let mut extract: Option<PathBuf> = None;
    let mut scene: Option<PathBuf> = None;
    let mut overrides: Vec<(u16, SurfaceKind)> = Vec::new();
    let mut dry_run = false;
    let mut all_layers = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--prefab" if i + 1 < args.len() => {
                prefab = Some(args[i + 1].clone());
                i += 2;
            }
            "--slug" if i + 1 < args.len() => {
                slug = Some(args[i + 1].clone());
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--paks" if i + 1 < args.len() => {
                paks = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--extract" if i + 1 < args.len() => {
                extract = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--scene" if i + 1 < args.len() => {
                scene = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--kind" if i + 1 < args.len() => {
                let o = parse_kind_override(&args[i + 1]).with_context(|| {
                    format!(
                        "--kind expects <record>=<opaque|glass|foliage>, got {}",
                        args[i + 1]
                    )
                })?;
                overrides.push(o);
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--all-layers" => {
                all_layers = true;
                i += 1;
            }
            other => {
                eprintln!(
                    "bvh-batch: unknown arg {other} (usage: --prefab <Prefabs/…/X.et> [--slug <s>] [--out <dir>] \
                     [--paks <dir>] [--extract <dir>] [--scene <spec.json>] [--kind <record>=<kind>]… [--dry-run])"
                );
                return Ok(1);
            }
        }
    }
    let prefab = prefab.context("--prefab <Prefabs/…/X.et> is required")?;
    let out_dir = out.unwrap_or_else(|| terrain_dir(root, "everon").join("prefabs"));
    let schema = definition_path(root, "building-instances.schema.json");
    let slug = slug.unwrap_or_else(|| slug_of(&prefab));

    let source = open_sources(paks.as_deref(), extract.as_deref())?;
    let mut walker = Walker::new(&source);
    // The shell's --kind overrides bind to the building's own model.
    let root_prefab = walker.resolver.resolve(&prefab)?;
    let shell_xob = root_prefab
        .mesh
        .clone()
        .with_context(|| format!("{prefab}: no MeshObject in its chain — not a building"))?;
    if all_layers {
        walker.assets.set_policy(LayerPolicy::All);
    }
    if !overrides.is_empty() {
        walker.assets.set_overrides(&shell_xob, overrides);
    }
    let shell = walker
        .walk(
            &prefab,
            &slug,
            None,
            Rigid::identity(),
            PlacementSource::PrefabCoords,
            true,
            0,
        )?
        .context("shell model missing")?;
    let shell_bytes = shell.sidecar_bytes.clone().with_context(|| {
        format!("{shell_xob}: no collision chunk — cannot build the shell sidecar")
    })?;

    // Scene roots (trees etc.) go to a second document.
    let mut scene_walker_instances: Vec<InstanceRecord> = Vec::new();
    let mut scene_notes: Vec<String> = Vec::new();
    if let Some(spec_path) = &scene {
        let spec: SceneSpec = serde_json::from_str(
            &fs::read_to_string(spec_path).with_context(|| spec_path.display().to_string())?,
        )
        .with_context(|| format!("parse scene spec {}", spec_path.display()))?;
        let before = walker.instances.len();
        for e in &spec.entries {
            let t = Rigid::from_enfusion(e.pos, e.angles_deg, e.scale);
            if let Err(err) =
                walker.walk(&e.prefab, &e.id, None, t, PlacementSource::Scene, false, 1)
            {
                walker
                    .notes
                    .push(format!("scene {}: {} ({err:#})", e.id, e.prefab));
            }
        }
        scene_walker_instances = walker.instances.split_off(before);
        // Notes written while walking scene roots belong to the scene document.
        scene_notes = walker
            .notes
            .iter()
            .filter(|n| spec.entries.iter().any(|e| n.starts_with(&e.id)))
            .cloned()
            .collect();
        walker
            .notes
            .retain(|n| !spec.entries.iter().any(|e| n.starts_with(&e.id)));
    }

    let instances = InstancesFile {
        schema_version: INSTANCES_SCHEMA_VERSION.into(),
        prefab_id: slug.clone(),
        resource_name: prefab.clone(),
        shell_bvh: format!("{slug}.bvh"),
        instances: walker.instances.clone(),
        notes: walker.notes.clone(),
    };
    let value = validate_instances(&instances, &schema)?;
    let scene_doc =
        (!scene_walker_instances.is_empty() || scene.is_some()).then(|| InstancesFile {
            schema_version: INSTANCES_SCHEMA_VERSION.into(),
            prefab_id: slug.clone(),
            resource_name: prefab.clone(),
            shell_bvh: String::new(),
            instances: scene_walker_instances,
            notes: scene_notes,
        });
    let scene_value = match &scene_doc {
        Some(d) => Some(validate_instances(d, &schema)?),
        None => None,
    };

    // Report.
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for inst in &instances.instances {
        *by_kind
            .entry(format!("{:?}", inst.kind).to_ascii_lowercase())
            .or_default() += 1;
    }
    let by_source: BTreeMap<String, usize> =
        instances
            .instances
            .iter()
            .fold(BTreeMap::new(), |mut m, i| {
                *m.entry(format!("{:?}", i.source)).or_default() += 1;
                m
            });
    let used: Vec<&str> = instances.blas_paths();
    let scene_used: Vec<&str> = scene_doc
        .as_ref()
        .map(|d| d.blas_paths())
        .unwrap_or_default();
    let (so, sg, sf) = shell.kind_counts();
    println!(
        "bvh-batch {slug}: shell {} ({} tris · kinds opaque {so} / glass {sg} / foliage {sf}) · {} instances {:?} · sources {:?} · {} BLAS · {} notes{}",
        shell_xob,
        shell.kinds.len(),
        instances.instances.len(),
        by_kind,
        by_source,
        used.len() + scene_used.iter().filter(|p| !used.contains(p)).count(),
        instances.notes.len(),
        if dry_run {
            " (dry run — nothing written)"
        } else {
            ""
        }
    );
    for n in &instances.notes {
        println!("  note: {n}");
    }
    if let Some(d) = &scene_doc {
        println!("  scene: {} instances", d.instances.len());
        for n in &d.notes {
            println!("  scene note: {n}");
        }
    }
    if dry_run {
        return Ok(0);
    }

    // Write: shell, every referenced BLAS (deduplicated by stem), the two documents.
    let mut written = 0usize;
    written += usize::from(write_if_changed(
        &out_dir.join("buildings").join(format!("{slug}.bvh")),
        &shell_bytes,
    )?);
    let mut blas_written: Vec<String> = Vec::new();
    for asset in walker.assets.loaded() {
        let rel = format!("blas/{}.bvh", asset.stem);
        if !used.contains(&rel.as_str()) && !scene_used.contains(&rel.as_str()) {
            continue;
        }
        if let Some(bytes) = &asset.sidecar_bytes
            && write_if_changed(&out_dir.join(&rel), bytes)?
        {
            blas_written.push(rel);
        }
    }
    written += blas_written.len();
    let json = serde_json::to_string_pretty(&value)? + "\n";
    written += usize::from(write_if_changed(
        &out_dir
            .join("buildings")
            .join(format!("{slug}.instances.json")),
        json.as_bytes(),
    )?);
    if let Some(v) = &scene_value {
        let json = serde_json::to_string_pretty(v)? + "\n";
        written += usize::from(write_if_changed(
            &out_dir.join("buildings").join(format!("{slug}.scene.json")),
            json.as_bytes(),
        )?);
    }
    println!(
        "  wrote {written} file(s) under {} ({} BLAS changed)",
        out_dir.display(),
        blas_written.len()
    );
    Ok(0)
}
