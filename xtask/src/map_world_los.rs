//! `cargo xtask map world-los` — T-090.12.3: the world occluder on the committed catalogue,
//! engine-free. Loads `objects/prefabs.json.gz`, a cell and its 8 neighbours from
//! `objects/chunks/`, and every descriptor + BLAS the rows need from `prefabs/`, then:
//!
//! - `--census`            per-chunk kinds, proxy rows, pending BLAS, memory
//! - `--probe ax,ay,az bx,by,bz`   one segment (ENGINE frame `[x, y_up, z_north]`): events + verdict
//! - `--bench N`           N random eye-height segments in the cell: µs / segment
//! - `--pairs <json>`      replay a `world-parity` oracle (T-090.12.4): agree / phantom / missed
//!   per policy, bucketed by the engine's hit prefab kind; `--min-agree F` exits 1 below it
//!
//! Usage: `--cell <cx_cy> [--assets packages/map-assets/everon] [--census] [--probe a b]
//!         [--bench N] [--pairs <json>] [--glass-blocks] [--foliage-blocks] [--proxy-only]
//!         [--min-agree F] [--dump-misses <jsonl>]`

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use map_engine_core::bvh::BvhSidecar;
use map_engine_core::world::occluder::{
    BlockPolicy, PrefabDescriptor, WorldOccluder, WorldVerdict,
};
use map_engine_core::world::{TerrainSizeM, parse_chunk};
use map_engine_core::world::{build_prefab_maps, narrow_prefab_rows};
use serde_json::Value;

/// Everon: 12 800 m square, 512 m chunks.
pub const TERRAIN_M: f64 = 12_800.0;
pub const CHUNK_M: f64 = 512.0;

fn gunzip_json(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| path.display().to_string())?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut out)
        .with_context(|| format!("gunzip {}", path.display()))?;
    serde_json::from_slice(&out).with_context(|| format!("parse {}", path.display()))
}

/// The occluder over `cell` + its 8 neighbours, every needed descriptor and BLAS loaded from
/// disk. Returns the occluder and the loaded chunk ids.
pub fn load_cell(assets: &Path, cell: &str) -> Result<(WorldOccluder, Vec<String>)> {
    let prefabs_doc = gunzip_json(&assets.join("objects/prefabs.json.gz"))?;
    let rows = narrow_prefab_rows(&prefabs_doc);
    let (by_id, _) = build_prefab_maps(rows.clone());
    let mut occ = WorldOccluder::new(
        CHUNK_M,
        TerrainSizeM {
            width: TERRAIN_M,
            height: TERRAIN_M,
        },
    );
    occ.set_prefabs(rows.iter());
    let mut parts = cell.split('_');
    let cx: i64 = parts
        .next()
        .and_then(|v| v.parse().ok())
        .context("cell cx")?;
    let cy: i64 = parts
        .next()
        .and_then(|v| v.parse().ok())
        .context("cell cy")?;
    let mut loaded = Vec::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            let id = format!("{}_{}", cx + dx, cy + dy);
            let path = assets.join("objects/chunks").join(format!("{id}.json.gz"));
            if !path.is_file() {
                continue;
            }
            let raw = gunzip_json(&path)?;
            if let Some(chunk) = parse_chunk(&id, &raw, &by_id) {
                occ.insert_chunk(&id, &chunk);
                loaded.push(id);
            }
        }
    }
    // Every placed pid: descriptor, then every BLAS it names.
    let prefabs = assets.join("prefabs");
    loop {
        let want = occ.wanted(&loaded, usize::MAX);
        if want.descriptors.is_empty() && want.blas.is_empty() {
            break;
        }
        for pid in &want.descriptors {
            let path = prefabs.join("descriptors").join(format!("{pid}.json"));
            let d: PrefabDescriptor = serde_json::from_str(
                &fs::read_to_string(&path).with_context(|| path.display().to_string())?,
            )
            .with_context(|| format!("parse {}", path.display()))?;
            occ.insert_descriptor(d);
        }
        for rel in &want.blas {
            let path = prefabs.join(rel);
            let bytes = fs::read(&path).with_context(|| path.display().to_string())?;
            let sc =
                BvhSidecar::parse(&bytes).with_context(|| format!("parse {}", path.display()))?;
            occ.insert_blas(rel, Arc::new(sc));
        }
    }
    occ.refresh();
    Ok((occ, loaded))
}

fn parse_point(s: &str) -> Result<[f64; 3]> {
    let v: Vec<f64> = s
        .split(',')
        .map(|x| {
            x.trim()
                .parse::<f64>()
                .map_err(|e| anyhow::anyhow!("{x}: {e}"))
        })
        .collect::<Result<_>>()?;
    if v.len() != 3 {
        bail!("{s}: want x,y,z");
    }
    Ok([v[0], v[1], v[2]])
}

/// One oracle pair: `[ox, oy, oz, tx, ty, tz, clearEnts, clearWorld, hitPrefabSlug]` (engine frame).
pub type WorldPair = (f64, f64, f64, f64, f64, f64, bool, bool, String);

/// One oracle file of the `world-parity` action.
#[derive(serde::Deserialize)]
pub struct WorldParityFile {
    pub cell: [i64; 2],
    #[serde(default)]
    pub seed: i64,
    pub pairs: Vec<WorldPair>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplayReport {
    pub n: usize,
    pub agree: usize,
    /// Model blocked, engine clear.
    pub phantom: usize,
    /// Model clear, engine blocked.
    pub missed: usize,
    pub provisional: usize,
    /// Disagreements bucketed by the engine's hit prefab slug (empty = engine clear).
    pub by_hit: BTreeMap<String, (usize, usize)>,
}

impl ReplayReport {
    #[must_use]
    pub fn agreement(&self) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.agree as f64 / self.n as f64
        }
    }
}

/// Replay every pair through `blocked` under `policy` against the `ENTS` (objects-only) column.
pub fn replay(
    occ: &WorldOccluder,
    file: &WorldParityFile,
    policy: BlockPolicy,
    misses: &mut Vec<String>,
) -> ReplayReport {
    let mut r = ReplayReport {
        n: file.pairs.len(),
        ..ReplayReport::default()
    };
    for (ox, oy, oz, tx, ty, tz, clear_ents, _clear_world, hit) in &file.pairs {
        let obs = [*ox, *oy, *oz];
        let tgt = [*tx, *ty, *tz];
        let blocked = occ.blocked(obs, tgt, policy);
        let verdict = occ.evaluate_los(obs, tgt).verdict;
        if verdict == WorldVerdict::Provisional {
            r.provisional += 1;
        }
        let model_clear = !blocked;
        if model_clear == *clear_ents {
            r.agree += 1;
        } else {
            let e = r.by_hit.entry(hit.clone()).or_insert((0, 0));
            if model_clear {
                r.missed += 1;
                e.1 += 1;
            } else {
                r.phantom += 1;
                e.0 += 1;
            }
            if misses.len() < 4000 {
                misses.push(format!(
                    "{{\"obs\":[{ox},{oy},{oz}],\"tgt\":[{tx},{ty},{tz}],\"engineClear\":{clear_ents},\"modelClear\":{model_clear},\"hit\":\"{hit}\"}}"
                ));
            }
        }
    }
    r
}

pub fn run(args: &[String]) -> Result<u8> {
    let mut cell: Option<String> = None;
    let mut assets: Option<PathBuf> = None;
    let mut census = false;
    let mut probe: Option<([f64; 3], [f64; 3])> = None;
    let mut bench: Option<usize> = None;
    let mut pairs: Option<PathBuf> = None;
    let mut policy = BlockPolicy::VISION;
    let mut min_agree: Option<f64> = None;
    let mut dump: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--cell" if i + 1 < args.len() => {
                cell = Some(args[i + 1].clone());
                i += 2;
            }
            "--assets" if i + 1 < args.len() => {
                assets = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--census" => {
                census = true;
                i += 1;
            }
            "--probe" if i + 2 < args.len() => {
                probe = Some((parse_point(&args[i + 1])?, parse_point(&args[i + 2])?));
                i += 3;
            }
            "--bench" if i + 1 < args.len() => {
                bench = Some(args[i + 1].parse().context("--bench <N>")?);
                i += 2;
            }
            "--pairs" if i + 1 < args.len() => {
                pairs = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--glass-blocks" => {
                policy.glass_blocks = true;
                i += 1;
            }
            "--foliage-blocks" => {
                policy.foliage_blocks = true;
                i += 1;
            }
            "--proxy-only" => {
                policy.proxy_blocks = true;
                i += 1;
            }
            "--min-agree" if i + 1 < args.len() => {
                min_agree = Some(args[i + 1].parse().context("--min-agree <F>")?);
                i += 2;
            }
            "--dump-misses" if i + 1 < args.len() => {
                dump = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                eprintln!(
                    "world-los: unknown arg {other} (usage: --cell <cx_cy> [--assets <dir>] [--census] [--probe a b] [--bench N] [--pairs <json>] [--glass-blocks] [--foliage-blocks] [--proxy-only] [--min-agree F] [--dump-misses <jsonl>])"
                );
                return Ok(1);
            }
        }
    }
    let cell = cell.context("--cell <cx_cy> is required")?;
    let root = crate::root::find_repo_root()?;
    let assets = assets.unwrap_or_else(|| root.join("packages/map-assets/everon"));
    let t0 = std::time::Instant::now();
    let (occ, loaded) = load_cell(&assets, &cell)?;
    println!(
        "world-los {cell}: {} chunks loaded {:?} · {} descriptors expanded · {} BLAS · {:.1} MB · {:.2} s",
        loaded.len(),
        loaded,
        occ.expanded_count(),
        occ.blas_count(),
        occ.memory_bytes() as f64 / 1_048_576.0,
        t0.elapsed().as_secs_f64()
    );
    let mut code = 0u8;
    if census {
        for id in &loaded {
            let Some(chunk) =
                gunzip_json(&assets.join("objects/chunks").join(format!("{id}.json.gz"))).ok()
            else {
                continue;
            };
            let rows = chunk["instances"].as_array().map_or(0, Vec::len);
            let mut kinds: HashMap<&str, usize> = HashMap::new();
            for r in chunk["instances"].as_array().into_iter().flatten() {
                if let Some(pid) = r[0].as_u64().and_then(|p| u16::try_from(p).ok()) {
                    *kinds.entry(occ.kind_of(pid).unwrap_or("?")).or_insert(0) += 1;
                }
            }
            let mut kv: Vec<(&str, usize)> = kinds.into_iter().collect();
            kv.sort();
            println!(
                "  {id}: {rows} rows {kv:?} · proxy rows {}",
                occ.proxy_rows(id).unwrap_or(0)
            );
        }
    }
    if let Some((a, b)) = probe {
        let (events, cov) = occ.trace(a, b);
        let los = occ.evaluate_los(a, b);
        println!(
            "  probe {a:?} → {b:?}: {:?} · concealment {:.3} · {} events · coverage {cov:?}",
            los.verdict,
            los.concealment,
            events.len()
        );
        for h in &los.hits {
            println!(
                "    t {:.4} {:?} {} c {:.3}",
                h.t, h.kind, h.id, h.concealment
            );
        }
        if let Some(bl) = &los.blocker {
            println!(
                "    blocker pid {} ({}) chunk {} row {} {:?} at {:?}",
                bl.pid,
                occ.label_of(bl.pid).unwrap_or("?"),
                bl.chunk,
                bl.row,
                bl.fidelity,
                bl.pos
            );
        }
    }
    if let Some(n) = bench {
        let mut parts = cell.split('_');
        let cx: f64 = parts.next().unwrap().parse().unwrap_or(0.0);
        let cy: f64 = parts.next().unwrap().parse().unwrap_or(0.0);
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rnd = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let mut blocked = 0usize;
        let t = std::time::Instant::now();
        for _ in 0..n {
            let a = [
                cx * CHUNK_M + rnd() * CHUNK_M,
                20.0 + rnd() * 40.0,
                cy * CHUNK_M + rnd() * CHUNK_M,
            ];
            let ang = rnd() * std::f64::consts::TAU;
            let len = 20.0 + rnd() * 480.0;
            let b = [
                a[0] + len * ang.cos(),
                a[1] + (rnd() - 0.5) * 4.0,
                a[2] + len * ang.sin(),
            ];
            if occ.blocked(a, b, policy) {
                blocked += 1;
            }
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / n.max(1) as f64;
        println!(
            "  bench {n} segments (20–500 m, eye height): {blocked} blocked · {us:.1} µs / segment"
        );
    }
    if let Some(path) = pairs {
        let file: WorldParityFile = serde_json::from_str(
            &fs::read_to_string(&path).with_context(|| path.display().to_string())?,
        )?;
        let mut misses = Vec::new();
        let r = replay(&occ, &file, policy, &mut misses);
        println!(
            "  parity cell {:?} seed {}: {}/{} agree ({:.2} %) · phantom (model-blocked/engine-clear) {} · missed (model-clear/engine-blocked) {} · provisional {} · policy {policy:?}",
            file.cell,
            file.seed,
            r.agree,
            r.n,
            r.agreement() * 100.0,
            r.phantom,
            r.missed,
            r.provisional
        );
        let mut by: Vec<(&String, &(usize, usize))> = r.by_hit.iter().collect();
        by.sort_by(|a, b| (b.1.0 + b.1.1).cmp(&(a.1.0 + a.1.1)).then(a.0.cmp(b.0)));
        for (hit, (ph, mi)) in by.iter().take(24) {
            println!(
                "    {:<48} phantom {ph:>4}  missed {mi:>4}",
                if hit.is_empty() {
                    "(engine clear)"
                } else {
                    hit.as_str()
                }
            );
        }
        if let Some(d) = dump {
            fs::write(&d, misses.join("\n") + "\n")?;
            println!("  {} disagreements → {}", misses.len(), d.display());
        }
        if let Some(m) = min_agree {
            if r.agreement() < m {
                println!("  FAIL agreement {:.4} < {m}", r.agreement());
                code = 1;
            }
        }
    }
    Ok(code)
}

#[cfg(test)]
#[path = "map_world_los_tests.rs"]
mod tests;
