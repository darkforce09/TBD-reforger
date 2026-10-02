use super::*;

pub fn run_voxels_from_mesh(_root: &std::path::Path, args: &[String]) -> Result<u8> {
    let mut mesh_path: Option<PathBuf> = None;
    let mut slug = String::new();
    let mut out_dir: Option<PathBuf> = None;
    let mut resource: Option<String> = None;
    let mut reference: Option<PathBuf> = None;
    let mut axes = AxesRemap::identity();
    let mut flip_winding = false;
    let mut exclude_material: Vec<String> = Vec::new();
    let mut lod: Option<u32> = None;
    let mut stats_only = false;
    let mut geometry = String::from("auto");
    let mut coll_record: Option<u16> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--geometry" if i + 1 < args.len() => {
                geometry = args[i + 1].clone();
                i += 2;
            }
            "--coll-record" if i + 1 < args.len() => {
                coll_record = Some(
                    args[i + 1]
                        .parse()
                        .context("--coll-record wants an index")?,
                );
                i += 2;
            }
            "--mesh" if i + 1 < args.len() => {
                mesh_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--slug" if i + 1 < args.len() => {
                slug = args[i + 1].clone();
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out_dir = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--resource" if i + 1 < args.len() => {
                resource = Some(args[i + 1].clone());
                i += 2;
            }
            "--reference" if i + 1 < args.len() => {
                reference = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--axes" if i + 1 < args.len() => {
                axes = AxesRemap::parse(&args[i + 1])?;
                i += 2;
            }
            "--flip-winding" => {
                flip_winding = true;
                i += 1;
            }
            "--exclude-material" if i + 1 < args.len() => {
                exclude_material.push(args[i + 1].clone());
                i += 2;
            }
            "--lod" if i + 1 < args.len() => {
                lod = Some(args[i + 1].parse().context("--lod wants a tier number")?);
                i += 2;
            }
            "--stats" => {
                stats_only = true;
                i += 1;
            }
            other => bail!("unknown arg '{other}' (see `cargo xtask map --help`)"),
        }
    }
    let mesh_path = mesh_path.context("--mesh <file.xob> is required")?;
    let bytes = fs::read(&mesh_path).with_context(|| mesh_path.display().to_string())?;
    let use_coll = match geometry.as_str() {
        "coll" => true,
        "visual" => false,
        "auto" => xob::has_coll(&bytes),
        other => bail!("--geometry '{other}' is not auto|coll|visual"),
    };
    println!(
        "geometry: {}",
        if use_coll {
            "COLL (fire-collision — the surface LOS traces)"
        } else {
            "LODS (visual mesh)"
        }
    );
    let mut parsed = if use_coll {
        xob::parse_coll(&bytes)?
    } else {
        xob::parse_xob(&bytes, lod)?
    };
    if let Some(rsel) = coll_record {
        let mut tris = Vec::new();
        let mut subs = Vec::new();
        for (i, tri) in parsed.tris.iter().enumerate() {
            if parsed.tri_submesh[i] == rsel {
                tris.push(*tri);
                subs.push(rsel);
            }
        }
        if tris.is_empty() {
            bail!("--coll-record {rsel}: no triangles in that record");
        }
        parsed.tris = tris;
        parsed.tri_submesh = subs;
    }
    print_stats(&parsed);
    if stats_only {
        return Ok(0);
    }
    if slug.is_empty() {
        bail!("--slug <name> is required");
    }

    let mesh = TriMesh::from_xob(&parsed, &axes, flip_winding, &exclude_material);
    let (min, max) = xob::aabb(&mesh.verts);

    if let Some(ref rp) = reference {
        let refd = super::super::parse::parse_dump(rp)?;
        if let Some(rm) = refd.meta {
            println!(
                "reference bbox  [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}]",
                rm.bbox_min[0],
                rm.bbox_min[1],
                rm.bbox_min[2],
                rm.bbox_max[0],
                rm.bbox_max[1],
                rm.bbox_max[2],
            );
            println!(
                "mesh bbox       [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}]",
                min[0], min[1], min[2], max[0], max[1], max[2],
            );
            let mut worst = 0.0f64;
            for a in 0..3 {
                worst = worst
                    .max((rm.bbox_min[a] - min[a]).abs())
                    .max((rm.bbox_max[a] - max[a]).abs());
            }
            if worst > 0.75 {
                println!(
                    "WARN: frame deviation {worst:.2} m — check --axes (engine bounds include \
                     child entities, so a mesh slightly INSIDE the reference box is normal)"
                );
            } else {
                println!("frame check OK (worst component deviation {worst:.2} m)");
            }
        }
    }

    let winding = {
        let origin = [min[0] - PAD, min[1] - PAD, min[2] - PAD];
        let span = [
            max[0] - min[0] + 2.0 * PAD,
            max[1] - min[1] + PAD + 1.2,
            max[2] - min[2] + 2.0 * PAD,
        ];
        let dims = [
            (span[0] / CELL).ceil() as usize,
            (span[1] / CELL).ceil() as usize,
            (span[2] / CELL).ceil() as usize,
        ];
        let bx = AxisBins::build(&mesh, 0, origin, dims);
        let bz = AxisBins::build(&mesh, 2, origin, dims);
        backface_first_fraction(&mesh, &bx, &bz)
    };
    println!("winding check: back-facing first-hit fraction {winding:.3}");
    if winding > 0.5 {
        println!("WARN: majority back-facing — likely inverted mesh; retry with --flip-winding");
    }

    let ident = DumpIdent {
        slug: slug.clone(),
        resource: resource.unwrap_or_else(|| {
            format!(
                "xob:{}",
                mesh_path.file_name().unwrap_or_default().to_string_lossy()
            )
        }),
    };
    let dump = generate(&mesh, ident);
    let out_dir = out_dir.unwrap_or_else(|| {
        crate::repository_paths::find_repo_root()
            .map(|r| r.join("target/mesh-dumps"))
            .unwrap_or_else(|_| PathBuf::from("target/mesh-dumps"))
    });
    let out_path = out_dir.join(format!("{slug}_voxels.jsonl.gz"));
    let lines = write_dump(&dump, &out_path)?;

    // Self-check: the strict dump parser is a free validator of every wire invariant.
    let reparsed = super::super::parse::parse_dump(&out_path)
        .context("self-check: generated dump failed the strict parser")?;
    let meta = reparsed.meta.as_ref().expect("meta round-trips");
    println!(
        "OK {slug}: {} tris → {} scanlines (dims {}x{}x{}) → {}",
        mesh.tris.len(),
        lines,
        meta.dims[0],
        meta.dims[1],
        meta.dims[2],
        out_path.display()
    );
    Ok(0)
}
