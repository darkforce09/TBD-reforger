//! `cargo xtask map bvh-parity` + `map bvh-emit` — CLI for the 3D-occlusion lane.
//!
//! The BVH raycaster and the `.bvh` sidecar codec live in `map_engine_core::bvh`
//! (step 2 moved them there; this file kept only the xtask plumbing). `bvh-parity`
//! replays the Workbench parity oracle over either the COLL trimesh of a `.xob`
//! (`--mesh`) or an emitted sidecar (`--sidecar`) — the two lanes must print identical
//! numbers. `bvh-emit` writes the deterministic sidecar next to the blueprint JSON in
//! `packages/map-assets/everon/prefabs/buildings/`.
//!
//! Usage: `map bvh-parity (--mesh <file.xob> | --sidecar <file.bvh>) --pairs <parity.json>
//!         [--record <i>] [--t-eps <meters>] [--dump-misses <path.jsonl>]
//!         [--instances <slug>.instances.json [--exclude-kinds a,b] [--doors closed|open]]`
//! (`--instances` = the T-090.11.4 compound lane: shell + every BLAS the instances file
//! references, replayed through `CompoundBuilding::blocked_range` — glass and foliage never
//! block, doors in the requested state; `--exclude-kinds furniture` drops what the Workbench
//! oracle's world does not nest under the building.)
//!        `map bvh-emit --mesh <file.xob> --slug <slug> [--out <dir>]`

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use map_engine_core::building_compound::{
    CompoundBuilding, DoorState, InstanceKind, InstanceRecord, InstancesFile,
};
use map_engine_core::building_compound_los::Owner;
use map_engine_core::bvh::{
    Bvh, BvhSidecar, SurfaceKind, dot, emit_bytes, lift_verts, quantize_verts, sub,
};

use super::xob;
use crate::map_parity_report::ParityFile;

/// (verts, tris, bvh, per-triangle record ids — mesh lane only; sidecars carry none).
type Geometry = (Vec<[f64; 3]>, Vec<[u32; 3]>, Bvh, Option<Vec<u16>>);

pub fn run_bvh_parity(args: &[String]) -> Result<u8> {
    let mut mesh_path: Option<PathBuf> = None;
    let mut sidecar_path: Option<PathBuf> = None;
    let mut pairs_path: Option<PathBuf> = None;
    let mut record: Option<u16> = None;
    let mut t_eps = 0.0f64;
    let mut dump_misses: Option<PathBuf> = None;
    let mut instances_path: Option<PathBuf> = None;
    let mut exclude_kinds: Vec<String> = Vec::new();
    let mut doors_open = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--instances" if i + 1 < args.len() => {
                instances_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--exclude-kinds" if i + 1 < args.len() => {
                exclude_kinds = args[i + 1]
                    .split(',')
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 2;
            }
            "--doors" if i + 1 < args.len() => {
                doors_open = match args[i + 1].as_str() {
                    "open" => true,
                    "closed" => false,
                    other => bail!("--doors expects closed|open, got {other}"),
                };
                i += 2;
            }
            "--mesh" if i + 1 < args.len() => {
                mesh_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--sidecar" if i + 1 < args.len() => {
                sidecar_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--pairs" if i + 1 < args.len() => {
                pairs_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--record" if i + 1 < args.len() => {
                record = Some(args[i + 1].parse().context("--record expects a u16")?);
                i += 2;
            }
            "--t-eps" if i + 1 < args.len() => {
                t_eps = args[i + 1]
                    .parse()
                    .context("--t-eps expects meters (f64)")?;
                i += 2;
            }
            "--dump-misses" if i + 1 < args.len() => {
                dump_misses = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                eprintln!(
                    "bvh-parity: unknown arg {other} (usage: (--mesh <file.xob> | --sidecar <file.bvh>) \
                     --pairs <parity.json> [--record <i>] [--t-eps <meters>] [--dump-misses <path.jsonl>] \
                     [--instances <slug>.instances.json [--exclude-kinds a,b] [--doors closed|open]])"
                );
                return Ok(1);
            }
        }
    }
    let pairs_path = pairs_path.context("--pairs <parity.json> is required")?;

    // Geometry source: the raw COLL trimesh (f64, straight from the xob — the step-1
    // lane) or an emitted sidecar (f32-quantized). Records exist only on the mesh lane.
    let (verts, tris, bvh, records): Geometry = match (mesh_path, sidecar_path) {
        (Some(_), Some(_)) | (None, None) => {
            bail!("exactly one of --mesh <file.xob> / --sidecar <file.bvh> is required")
        }
        (Some(mesh_path), None) => {
            let bytes = fs::read(&mesh_path).with_context(|| mesh_path.display().to_string())?;
            let mut parsed = xob::parse_coll(&bytes)?;
            if let Some(rsel) = record {
                let mut tris = Vec::new();
                let mut subs = Vec::new();
                for (i, tri) in parsed.tris.iter().enumerate() {
                    if parsed.tri_submesh[i] == rsel {
                        tris.push(*tri);
                        subs.push(rsel);
                    }
                }
                if tris.is_empty() {
                    bail!("--record {rsel}: no triangles in that record");
                }
                parsed.tris = tris;
                parsed.tri_submesh = subs;
            }
            let mut per_record: std::collections::BTreeMap<u16, usize> =
                std::collections::BTreeMap::new();
            for &r in &parsed.tri_submesh {
                *per_record.entry(r).or_default() += 1;
            }
            let recs: Vec<String> = per_record
                .iter()
                .map(|(r, n)| format!("{r}: {n}"))
                .collect();
            let (lo, hi) = xob::aabb(&parsed.verts);
            let bvh = Bvh::build(&parsed.verts, &parsed.tris);
            println!(
                "coll: {} verts · {} tris · records [{}] · aabb [{:.2},{:.2},{:.2}]..[{:.2},{:.2},{:.2}] · {} bvh nodes",
                parsed.verts.len(),
                parsed.tris.len(),
                recs.join(", "),
                lo[0],
                lo[1],
                lo[2],
                hi[0],
                hi[1],
                hi[2],
                bvh.node_count(),
            );
            (parsed.verts, parsed.tris, bvh, Some(parsed.tri_submesh))
        }
        (None, Some(sidecar_path)) => {
            if record.is_some() {
                bail!("--record needs --mesh: sidecars carry no record ids (v1)");
            }
            let bytes =
                fs::read(&sidecar_path).with_context(|| sidecar_path.display().to_string())?;
            let sc = BvhSidecar::parse(&bytes)
                .with_context(|| format!("parse sidecar {}", sidecar_path.display()))?;
            let (opaque, glass, foliage) = sc.kind_counts();
            println!(
                "sidecar: {} verts · {} tris · {} bvh nodes · {} bytes · kinds opaque {opaque} / glass {glass} / foliage {foliage}",
                sc.verts.len(),
                sc.tris.len(),
                sc.bvh.node_count(),
                bytes.len(),
            );
            (sc.verts, sc.tris, sc.bvh, None)
        }
    };

    let parity: ParityFile = serde_json::from_str(
        &fs::read_to_string(&pairs_path)
            .with_context(|| format!("read {}", pairs_path.display()))?,
    )
    .context("parse parity JSON")?;

    // T-090.11.4: the compound lane — the shell (the geometry loaded above) plus every
    // instance the file references, in the requested door state.
    let compound: Option<CompoundBuilding> = match &instances_path {
        Some(path) => {
            let shell = Arc::new(BvhSidecar {
                verts: verts.clone(),
                tris: tris.clone(),
                bvh: Bvh::build(&verts, &tris),
                kinds: vec![SurfaceKind::Opaque; tris.len()],
            });
            let (c, kept, dropped) = load_compound(path, shell, &exclude_kinds, doors_open)?;
            println!(
                "compound: {kept} instances ({dropped} excluded by kind [{}]) · doors {}",
                exclude_kinds.join(", "),
                if doors_open { "OPEN" } else { "closed" }
            );
            Some(c)
        }
        None => None,
    };

    let mut agree = 0usize;
    let mut model_clear_engine_blocked = 0usize;
    let mut model_blocked_engine_clear = 0usize;
    let mut misses: Vec<String> = Vec::new();
    for (idx, &(ox, oy, oz, tx, ty, tz, engine_clear)) in parity.pairs.iter().enumerate() {
        let p = [ox, oy, oz];
        let q = [tx, ty, tz];
        let seg_len = dot(sub(q, p), sub(q, p)).sqrt();
        // Endpoint policy: strict [0,1] by default; --t-eps shrinks both ends by a metric
        // margin (an oracle endpoint on a 0.01-quantized surface must not self-block).
        let (t_lo, t_hi) = if t_eps > 0.0 && seg_len >= 2.0 * t_eps {
            (t_eps / seg_len, 1.0 - t_eps / seg_len)
        } else {
            (0.0, 1.0)
        };
        let hit = if seg_len < 1e-9 {
            None // degenerate pair: zero-length segment occludes nothing
        } else if let Some(c) = &compound {
            // Compound: the first opaque crossing (shell or instance) is the block.
            c.blocked_range(p, q, t_lo, t_hi).then(|| {
                c.trace_range(p, q, t_lo, t_hi)
                    .into_iter()
                    .find(|e| e.kind == SurfaceKind::Opaque)
                    .map_or(
                        map_engine_core::bvh::Hit {
                            t: f64::NAN,
                            tri: u32::MAX,
                        },
                        |e| map_engine_core::bvh::Hit {
                            t: e.t,
                            tri: match e.owner {
                                Owner::Shell => e.tri,
                                Owner::Instance(i) => 1_000_000 + i as u32,
                            },
                        },
                    )
            })
        } else {
            bvh.any_hit(&verts, &tris, p, q, t_lo, t_hi)
        };
        let model_clear = hit.is_none();
        if model_clear == engine_clear {
            agree += 1;
        } else {
            if model_clear {
                model_clear_engine_blocked += 1;
            } else {
                model_blocked_engine_clear += 1;
            }
            if dump_misses.is_some() {
                let row = match &hit {
                    Some(h) => serde_json::json!({
                        "pair": idx,
                        "engine_clear": engine_clear,
                        "model_clear": model_clear,
                        "t": h.t,
                        "tri": h.tri,
                        "record": records.as_ref().map(|r| r[h.tri as usize]),
                        "hit": [ox + h.t * (tx - ox), oy + h.t * (ty - oy), oz + h.t * (tz - oz)],
                    }),
                    None => serde_json::json!({
                        "pair": idx,
                        "engine_clear": engine_clear,
                        "model_clear": model_clear,
                        "t": null,
                        "tri": null,
                        "record": null,
                        "hit": null,
                    }),
                };
                misses.push(row.to_string());
            }
        }
    }

    if let Some(path) = &dump_misses {
        let mut f = fs::File::create(path).with_context(|| path.display().to_string())?;
        for m in &misses {
            writeln!(f, "{m}")?;
        }
        println!("wrote {} misses → {}", misses.len(), path.display());
    }

    let total = parity.pairs.len().max(1);
    println!(
        "bvh-parity {}: {agree}/{} agree ({:.1}%) · model-clear/engine-blocked {} · model-blocked/engine-clear {}",
        parity.slug,
        parity.pairs.len(),
        agree as f64 * 100.0 / total as f64,
        model_clear_engine_blocked,
        model_blocked_engine_clear,
    );
    Ok(0)
}

/// Load `<slug>.instances.json` + every BLAS it references (paths relative to the file's
/// directory) onto `shell`; `exclude_kinds` are lower-camel kind names (`furniture`, `glass`,
/// …). Returns `(compound, kept, dropped)`.
pub fn load_compound(
    instances_path: &Path,
    shell: Arc<BvhSidecar>,
    exclude_kinds: &[String],
    doors_open: bool,
) -> Result<(CompoundBuilding, usize, usize)> {
    let file: InstancesFile = serde_json::from_str(
        &fs::read_to_string(instances_path)
            .with_context(|| instances_path.display().to_string())?,
    )
    .context("parse instances JSON")?;
    let dir = instances_path.parent().unwrap_or_else(|| Path::new("."));
    let kind_name = |k: InstanceKind| {
        serde_json::to_value(k)
            .ok()
            .and_then(|v| v.as_str().map(str::to_ascii_lowercase))
            .unwrap_or_default()
    };
    let kept: Vec<InstanceRecord> = file
        .instances
        .iter()
        .filter(|r| !exclude_kinds.contains(&kind_name(r.kind)))
        .cloned()
        .collect();
    let dropped = file.instances.len() - kept.len();
    let mut blas_by_path: HashMap<String, Arc<BvhSidecar>> = HashMap::new();
    for r in &kept {
        if blas_by_path.contains_key(&r.blas) {
            continue;
        }
        // `blas/<stem>.bvh` is relative to the prefabs root (the parent of `buildings/`); a
        // flat layout beside the instances file is accepted too.
        let mut path = dir.join(&r.blas);
        if !path.is_file() {
            if let Some(parent) = dir.parent() {
                let up = parent.join(&r.blas);
                if up.is_file() {
                    path = up;
                }
            }
        }
        let bytes = fs::read(&path).with_context(|| path.display().to_string())?;
        let sc =
            BvhSidecar::parse(&bytes).with_context(|| format!("parse BLAS {}", path.display()))?;
        blas_by_path.insert(r.blas.clone(), Arc::new(sc));
    }
    let mut c = CompoundBuilding::assemble(shell, &kept, &blas_by_path)
        .map_err(|e| anyhow::anyhow!("assemble compound: {e}"))?;
    if doors_open {
        let ids: Vec<String> = c.doors().map(|d| d.record.id.clone()).collect();
        for id in ids {
            c.set_door(&id, DoorState::OPEN);
        }
    }
    Ok((c, kept.len(), dropped))
}

pub fn run_bvh_emit(args: &[String]) -> Result<u8> {
    let mut mesh_path: Option<PathBuf> = None;
    let mut slug = String::new();
    let mut out_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mesh" if i + 1 < args.len() => {
                mesh_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--slug" if i + 1 < args.len() => {
                slug = args[i + 1].clone();
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out_override = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                eprintln!(
                    "bvh-emit: unknown arg {other} (usage: --mesh <file.xob> --slug <slug> [--out <dir>])"
                );
                return Ok(1);
            }
        }
    }
    let mesh_path = mesh_path.context("--mesh <file.xob> is required")?;
    if slug.is_empty() {
        bail!("--slug <name> is required");
    }
    let out_dir = match out_override {
        Some(d) => d,
        None => crate::root::find_repo_root()?.join("packages/map-assets/everon/prefabs/buildings"),
    };

    let bytes = fs::read(&mesh_path).with_context(|| mesh_path.display().to_string())?;
    let parsed = xob::parse_coll(&bytes)?;
    // Determinism authority (see map_engine_core::bvh): quantize to the stored f32s FIRST
    // and build over their lifted values, so loader-side raycasts are bit-identical.
    let verts_f32 = quantize_verts(&parsed.verts);
    if verts_f32.iter().flatten().any(|c| !c.is_finite()) {
        bail!("COLL vertex overflows f32 — sidecar v1 cannot carry this mesh");
    }
    let lifted = lift_verts(&verts_f32);
    let bvh = Bvh::build(&lifted, &parsed.tris);
    // Every COLL triangle is Opaque on this lane (the shell); T-090.11.2's batch emitter is
    // the one that reads game materials and tags glass / foliage.
    let kinds = vec![SurfaceKind::Opaque; parsed.tris.len()];
    let out_bytes = emit_bytes(&verts_f32, &parsed.tris, &kinds, &bvh);
    BvhSidecar::parse(&out_bytes).context("emitted sidecar failed its own parse self-check")?;

    fs::create_dir_all(&out_dir).with_context(|| out_dir.display().to_string())?;
    let out_path = out_dir.join(format!("{slug}.bvh"));
    fs::write(&out_path, &out_bytes).with_context(|| out_path.display().to_string())?;
    println!(
        "bvh-emit {slug}: {} verts · {} tris · {} bvh nodes · {} bytes → {}",
        verts_f32.len(),
        parsed.tris.len(),
        bvh.node_count(),
        out_bytes.len(),
        out_path.display(),
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_blueprint::tests::fixture;

    /// The engine-free parity pin: the committed sidecar replayed against the committed
    /// 400-pair Workbench oracle — CI re-proves the 3D lane without the (unshippable)
    /// .xob. Numbers measured live 2026-09-01; re-bless deliberately, old→new in the
    /// commit message.
    #[test]
    fn farmhouse_bvh_sidecar_parity_is_pinned() {
        let golden =
            fs::read(fixture("FarmHouse_E_1L01_Wood.bvh.golden")).expect("golden sidecar fixture");
        // The shipping sidecar and the test golden are the same bytes, forever.
        let shipping = crate::root::find_repo_root()
            .expect("repo root")
            .join("packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.bvh");
        assert_eq!(
            golden,
            fs::read(&shipping).expect("shipping sidecar"),
            "map-assets sidecar diverged from the golden fixture — re-emit and re-bless both"
        );
        let sc = BvhSidecar::parse(&golden).expect("golden sidecar parses");
        assert_eq!(
            (sc.verts.len(), sc.tris.len(), sc.bvh.node_count()),
            (3170, 2883, 1125),
            "sidecar shape drifted"
        );

        let oracle: ParityFile = serde_json::from_str(
            &fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_parity.json")).expect("oracle"),
        )
        .expect("parse oracle");
        assert_eq!(oracle.pairs.len(), 400);
        let mut agree = 0usize;
        let mut phantom = 0usize;
        for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
            let clear = sc
                .bvh
                .any_hit(&sc.verts, &sc.tris, [ox, oy, oz], [tx, ty, tz], 0.0, 1.0)
                .is_none();
            if clear == engine_clear {
                agree += 1;
            } else if !clear {
                phantom += 1;
            }
        }
        assert_eq!(agree, 400, "sidecar parity drifted (was 100.0%)");
        assert_eq!(phantom, 0, "phantom geometry blocks rays");
    }
}

#[cfg(test)]
mod compound_tests {
    use super::*;
    use crate::map_blueprint::tests::fixture;

    /// The T-090.11.4 door-parity pin: the committed shell + every architectural instance
    /// (doors closed — the editor's `InitialAngle 0`; furniture excluded because the Workbench
    /// world places the furniture composition beside the building, not under it, so the oracle
    /// never traced it) replayed against the 4000-pair door-inclusive oracle of T-090.11.3.
    /// Measured 2026-09-04: 3983/4000 agree (re-blessed to 3998/4000 the same day under the
    /// T-090.12.4 projectile layer policy, which drops the shell's `Building` physics mesh in
    /// favour of its `FireView` fire geometry — 15 of the 17 phantoms were that mesh); the
    /// shell alone scored 3965 with 20
    /// model-clear/engine-blocked pairs — every one of them a closed door leaf, all recovered
    /// here (0 left); the 17 model-blocked/engine-clear pairs are 15 the shell already had on
    /// this larger oracle (roof ridge / eave skims at y ≈ 8.3 m and rays starting inside a
    /// collider, which `TraceMove` ignores) plus 2 instance-owned ones, both window frames
    /// (`Socket_Win_110x142_005` with the observer inside its collider at t = 0, and the
    /// interior `socket_win_130x142_003` at t = 0.065). Re-bless deliberately, old → new in
    /// the commit message.
    #[test]
    fn farmhouse_compound_door_parity_is_pinned() {
        let root = crate::root::find_repo_root().expect("repo root");
        let buildings = root.join("packages/map-assets/everon/prefabs/buildings");
        let shell_bytes = fs::read(buildings.join("FarmHouse_E_1L01_Wood.bvh")).expect("shell");
        let sc = BvhSidecar::parse(&shell_bytes).expect("shell parses");
        let shell = Arc::new(BvhSidecar {
            verts: sc.verts.clone(),
            tris: sc.tris.clone(),
            bvh: Bvh::build(&sc.verts, &sc.tris),
            kinds: sc.kinds.clone(),
        });
        let instances = buildings.join("FarmHouse_E_1L01_Wood.instances.json");
        assert_eq!(
            fs::read(&instances).expect("instances"),
            fs::read(fixture("FarmHouse_E_1L01_Wood.instances.golden.json")).expect("golden"),
            "map-assets instances diverged from the golden fixture — re-emit and re-bless both"
        );
        let (c, kept, dropped) =
            load_compound(&instances, shell, &["furniture".to_string()], false).expect("compound");
        // T-090.12.4 — 132 → 120: the projectile layer policy left the twelve `LightSwitch_02`
        // props (Prop preset, no fire geometry) out of the instance set.
        assert_eq!((kept, dropped), (120, 49));
        assert_eq!(c.doors().count(), 7);
        assert!(c.doors().all(|d| d.state == DoorState::Closed));

        let replay = |name: &str| -> (usize, usize, usize, usize) {
            let oracle: ParityFile =
                serde_json::from_str(&fs::read_to_string(fixture(name)).expect("oracle"))
                    .expect("parse oracle");
            let (mut agree, mut missed, mut phantom) = (0usize, 0usize, 0usize);
            for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
                let clear = !c.blocked([ox, oy, oz], [tx, ty, tz]);
                if clear == engine_clear {
                    agree += 1;
                } else if clear {
                    missed += 1;
                } else {
                    phantom += 1;
                }
            }
            (oracle.pairs.len(), agree, missed, phantom)
        };
        assert_eq!(
            replay("FarmHouse_E_1L01_Wood_parity_doors.json"),
            (4000, 3998, 0, 2),
            "door-inclusive parity drifted (T-090.12.4: 3998/4000, 0 missed blocks, 2 phantoms — \
             3983/4000 with the Building physics mesh in the shell before the layer policy)"
        );
        // The T-090.6 oracle (doors and glass excluded) is untouched by the instances.
        assert_eq!(
            replay("FarmHouse_E_1L01_Wood_parity.json"),
            (400, 400, 0, 0),
            "shell oracle drifted under the compound"
        );
    }
}
