//! `cargo xtask map xob-inspect` — what the XOB decoder sees (T-090.11.2): chunk ids, the
//! string table, node records + sockets, COLL records with their layer preset and
//! per-material triangle runs, the resulting kinds histogram. The reverse-engineering
//! instrument and the acceptance check for the node / material decode; `--find <substr>`
//! lists matching in-pak paths, `--save <file>` keeps a raw copy for byte-level work
//! (scratch use only — extracted BI files are never committed).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use map_engine_core::bvh::SurfaceKind;

use super::batch::{decode_asset, open_sources};
use super::pak::{AssetSource, PakSet};
use super::surface_kind::{gamemat_stem, parse_kind_override};
use super::xob;

/// `map xob-inspect <file.xob | Assets/…/X.xob> [--paks <dir>] [--extract <dir>]
/// [--strings] [--kind <record>=<kind>]…`
pub fn run_xob_inspect(args: &[String]) -> Result<u8> {
    let mut target: Option<String> = None;
    let mut paks: Option<PathBuf> = None;
    let mut extract: Option<PathBuf> = None;
    let mut show_strings = false;
    let mut save: Option<PathBuf> = None;
    let mut find: Option<String> = None;
    let mut overrides: Vec<(u16, SurfaceKind)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--paks" if i + 1 < args.len() => {
                paks = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--extract" if i + 1 < args.len() => {
                extract = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--strings" => {
                show_strings = true;
                i += 1;
            }
            "--save" if i + 1 < args.len() => {
                save = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--find" if i + 1 < args.len() => {
                find = Some(args[i + 1].clone());
                i += 2;
            }
            "--kind" if i + 1 < args.len() => {
                overrides
                    .push(parse_kind_override(&args[i + 1]).context("--kind <record>=<kind>")?);
                i += 2;
            }
            other if !other.starts_with("--") && target.is_none() => {
                target = Some(other.to_string());
                i += 1;
            }
            other => {
                eprintln!(
                    "xob-inspect: unknown arg {other} (usage: <file.xob | in-pak path> [--paks <dir>] [--extract <dir>] [--strings] [--kind <record>=<kind>]…)"
                );
                return Ok(1);
            }
        }
    }
    if let Some(needle) = find {
        // `--find <substr>`: list matching in-pak paths with their entry facts.
        let dir = paks
            .clone()
            .or_else(PakSet::default_dir)
            .context("--find needs a pak directory (--paks <dir> or the MCP symlink farm)")?;
        let set = PakSet::from_dir(&dir)?;
        let needle_lc = needle.to_ascii_lowercase();
        let hits: Vec<String> = set
            .paths_under("")
            .into_iter()
            .filter(|p| p.to_ascii_lowercase().contains(&needle_lc))
            .collect();
        println!("xob-inspect --find {needle}: {} path(s)", hits.len());
        for p in hits.iter().take(200) {
            let e = set.find(p).expect("listed path resolves");
            println!(
                "  {p}  ({} bytes{})",
                e.decompressed_len,
                if e.compressed { ", zlib" } else { "" }
            );
        }
        return Ok(0);
    }
    let target = target.context("xob-inspect: a .xob path is required")?;
    let data = if Path::new(&target).is_file() {
        fs::read(&target)?
    } else {
        open_sources(paks.as_deref(), extract.as_deref())?.read(&target)?
    };
    if let Some(p) = &save {
        // Derived-nothing: a raw copy for byte-level inspection under the scratchpad only.
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(p, &data).with_context(|| p.display().to_string())?;
        println!("  saved {} bytes → {}", data.len(), p.display());
    }
    let asset = decode_asset(&target, &data, &overrides);
    println!(
        "xob-inspect {target}: {} bytes · {} node records · record layers {:?}",
        data.len(),
        asset.node_count,
        asset.layers
    );
    // Chunk ids present (scan like the parsers do — real files pad between chunks).
    let mut ids: Vec<String> = Vec::new();
    let mut pos = 12usize;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        if id
            .iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            let size =
                u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                    as usize;
            if size > 0 && pos + 8 + size <= data.len() {
                ids.push(format!("{}@{pos}+{size}", String::from_utf8_lossy(id)));
                pos += 8 + size;
                continue;
            }
        }
        pos += 1;
    }
    println!("  chunks: {}", ids.join(" · "));
    match xob::parse_xob(&data, None) {
        Ok(v) => println!(
            "  visual: tier {} · {} verts · {} tris · {} materials · {} descriptors",
            v.tier,
            v.verts.len(),
            v.tris.len(),
            v.materials.len(),
            v.descriptors.len()
        ),
        Err(e) => println!("  visual: (unreadable: {e:#})"),
    }
    match &asset.nodes {
        Some(n) => {
            println!(
                "  strings: {} names (table at HEAD+{}) · hierarchy: {} · node table at {}",
                n.strings.len(),
                n.name_base,
                if n.has_hierarchy() {
                    format!("Scene_Root = name #{}", n.nodes[0].name_idx)
                } else {
                    "none".to_string()
                },
                n.table_offset
            );
            if show_strings {
                for (i, s) in n.strings.iter().enumerate() {
                    println!("    #{i:4} · {s}");
                }
            }
            println!("  nodes: {}", n.nodes.len());
            for (i, rec) in n.nodes.iter().enumerate() {
                let w = n.world_of(i);
                println!(
                    "    #{i:3} {:<32} parent {:>4} pos [{:8.3} {:8.3} {:8.3}] yaw {:8.2}° quat [{:.4} {:.4} {:.4} {:.4}] next {} child {}",
                    n.node_name(i).unwrap_or("?"),
                    n.parent[i].map_or("-".to_string(), |p| p.to_string()),
                    w.t[0],
                    w.t[1],
                    w.t[2],
                    w.yaw_deg(),
                    rec.quat[0],
                    rec.quat[1],
                    rec.quat[2],
                    rec.quat[3],
                    if rec.next_sibling == 0xFFFF {
                        "-".to_string()
                    } else {
                        rec.next_sibling.to_string()
                    },
                    if rec.first_child == 0xFFFF {
                        "-".to_string()
                    } else {
                        rec.first_child.to_string()
                    },
                );
            }
            println!("  sockets: {}", n.sockets().len());
        }
        None => println!("  nodes: (no node table decoded)"),
    }
    match &asset.coll {
        Some(m) => {
            println!(
                "  coll: {} records · {} verts · {} tris",
                m.records.len(),
                m.verts.len(),
                m.tris.len()
            );
            for (r, rec) in m.records.iter().enumerate() {
                let name = |idx: u32| -> String {
                    asset
                        .nodes
                        .as_ref()
                        .and_then(|n| n.name(idx))
                        .map_or_else(|| format!("#{idx}"), |s| s.to_string())
                };
                println!(
                    "    record {r}: shape {} · layer {} · mesh {} · first material {} · tris {}..{}",
                    rec.shape,
                    name(u32::from(rec.layer_idx)),
                    name(u32::from(rec.mesh_idx)),
                    name(u32::from(rec.first_mat_idx)),
                    rec.tri_start,
                    rec.tri_start + rec.tri_count
                );
                let mut runs: Vec<(u32, usize, SurfaceKind)> = Vec::new();
                for t in rec.tri_start..rec.tri_start + rec.tri_count {
                    let mat = m.tri_material[t];
                    let kind = asset.kinds[t];
                    match runs.last_mut() {
                        Some(last) if last.0 == mat && last.2 == kind => last.1 += 1,
                        _ => runs.push((mat, 1, kind)),
                    }
                }
                for (mat, count, kind) in runs {
                    let label = if mat == u32::MAX {
                        "(no material)".to_string()
                    } else {
                        gamemat_stem(&name(mat))
                    };
                    println!("      {count:6} tris · {label:<24} → {kind:?}");
                }
            }
            let (o, g, f) = asset.kind_counts();
            println!("  kinds: opaque {o} · glass {g} · foliage {f}");
            if let Some((lo, hi)) = asset.bounds() {
                println!(
                    "  bounds: [{:.3} {:.3} {:.3}] .. [{:.3} {:.3} {:.3}]",
                    lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]
                );
            }
            if let Some(b) = &asset.sidecar_bytes {
                println!("  sidecar: {} bytes (v2)", b.len());
            }
        }
        None => println!("  coll: (no COLL chunk)"),
    }
    Ok(0)
}
