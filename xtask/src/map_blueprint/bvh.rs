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
//!         [--record <i>] [--t-eps <meters>] [--dump-misses <path.jsonl>]`
//!        `map bvh-emit --mesh <file.xob> --slug <slug> [--out <dir>]`

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use map_engine_core::bvh::{Bvh, BvhSidecar, dot, emit_bytes, lift_verts, quantize_verts, sub};

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
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
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
                     --pairs <parity.json> [--record <i>] [--t-eps <meters>] [--dump-misses <path.jsonl>])"
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
            println!(
                "sidecar: {} verts · {} tris · {} bvh nodes · {} bytes",
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
    let out_bytes = emit_bytes(&verts_f32, &parsed.tris, &bvh);
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
            (3170, 4012, 1521),
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
