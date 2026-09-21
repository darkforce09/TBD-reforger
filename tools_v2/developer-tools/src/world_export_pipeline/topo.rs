//! Enfusion `.topo` decoder — the terrain's road and airfield polylines, read from the pak VFS.
//!
//! The format, all little-endian:
//!
//! ```text
//! 0x00  16 bytes   unused here
//! 0x10  u32        section count (≥ 1; section 1 is the full-detail set this decoder keeps)
//! 0x14  u32        record count of section 1
//! 0x18  records of section 1, then for each further section: u32 record count, records
//!
//! record: u8 type · u32 n · n × (f32 x, f32 y) · u32 k · k × u32 attribute
//! ```
//!
//! `x` is world metres east; `y` is north-up IMAGE metres. A record is rejected when its vertex
//! count is zero or absurd, when its first or last vertex lies outside the world by more than
//! 2 km, or when it runs past the buffer — the parser stops at the first such record rather than
//! guess where the next one starts.

use anyhow::{Result, anyhow, bail};

use crate::enfusion_pak::PakVfs;

pub struct TopoCfg {
    pub topo_path: &'static str,
    pub world_size_m: f64,
}

/// Per-terrain config — add a row per terrain; zero Eden hardcodes below this table.
pub fn topo_terrain(terrain: &str) -> Option<TopoCfg> {
    match terrain {
        "everon" => Some(TopoCfg {
            topo_path: "worlds/Eden/Eden.topo",
            world_size_m: 12800.0,
        }),
        "arland" => Some(TopoCfg {
            topo_path: "worlds/Arland/Arland.topo",
            world_size_m: 4096.0,
        }),
        _ => None,
    }
}

pub const TOPO_AIRFIELD: u8 = 0;
pub const TOPO_RIVER: u8 = 1; // legacy name — 12 m main asphalt highway (see build-roads header)
pub const TOPO_STREAM: u8 = 2; // 8 m secondary asphalt
pub const TOPO_ROAD_A: u8 = 3; // 4.5 m gravel/country
pub const TOPO_ROAD_B: u8 = 5; // 1.75 m farm tracks / trails

const HEADER_LEN: usize = 0x18;

#[derive(Debug)]
pub struct TopoRecord {
    pub rec_type: u8,
    /// Interleaved [x0, y0, x1, y1, …] — x world metres east, y north-up IMAGE metres.
    pub verts: Vec<f32>,
    pub attrs: Vec<u32>,
}

#[derive(Debug)]
pub struct Topo {
    pub world_size_m: f64,
    pub section_count: u32,
    pub per_section: u32,
    /// Section 1 (full detail).
    pub records: Vec<TopoRecord>,
    pub bytes: usize,
    pub consumed: usize,
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn f32le(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn parse_record(buf: &[u8], pos: usize, world_size_m: f64) -> Option<(TopoRecord, usize)> {
    if pos + 5 > buf.len() {
        return None;
    }
    let rec_type = buf[pos];
    let count = u32le(buf, pos + 1) as usize;
    let v_start = pos + 5;
    let v_end = v_start + count * 8;
    if count == 0 || count > 2_000_000 || v_end + 4 > buf.len() {
        return None;
    }
    let in_range = |v: f32| f64::from(v) > -2000.0 && f64::from(v) < world_size_m + 2000.0;
    if !in_range(f32le(buf, v_start)) || !in_range(f32le(buf, v_end - 8)) {
        return None;
    }
    let k = u32le(buf, v_end) as usize;
    if k > 64 || v_end + 4 + k * 4 > buf.len() {
        return None;
    }
    let mut verts = Vec::with_capacity(count * 2);
    for i in 0..count * 2 {
        verts.push(f32le(buf, v_start + i * 4));
    }
    let mut attrs = Vec::with_capacity(k);
    for i in 0..k {
        attrs.push(u32le(buf, v_end + 4 + i * 4));
    }
    Some((
        TopoRecord {
            rec_type,
            verts,
            attrs,
        },
        v_end + 4 + k * 4,
    ))
}

/// Decode a terrain's `.topo` from the pak VFS (section 1 = full detail in `records`).
pub fn decode_topo(vfs: &PakVfs, terrain: &str) -> Result<Topo> {
    let cfg = topo_terrain(terrain).ok_or_else(|| {
        anyhow!("no topo config for terrain \"{terrain}\" (add it to topo_terrain)")
    })?;
    let buf = vfs.read_file(cfg.topo_path)?;
    parse_topo(&buf, &cfg)
}

/// Decode `.topo` bytes. Separate from the VFS read so the format can be exercised on a buffer.
pub fn parse_topo(buf: &[u8], cfg: &TopoCfg) -> Result<Topo> {
    if buf.len() < HEADER_LEN {
        bail!(
            "topo parse broke: {} is {} bytes, shorter than the {HEADER_LEN}-byte header",
            cfg.topo_path,
            buf.len()
        );
    }
    let section_count = u32le(buf, 0x10);
    let per_section = u32le(buf, 0x14);
    // Section 1 is the payload; a file that declares none of it is not a topo this decoder
    // knows, and reporting that beats indexing into an empty section list.
    if section_count == 0 {
        bail!(
            "topo parse broke: {} declares 0 sections in its header ({} bytes) — not a .topo this \
             decoder understands; check the pak build behind ENFUSION_GAME_PATH",
            cfg.topo_path,
            buf.len()
        );
    }

    let mut sections: Vec<Vec<TopoRecord>> = Vec::with_capacity(section_count as usize);
    let mut pos = HEADER_LEN;
    for s in 0..section_count {
        let expect = if s > 0 {
            let e = u32le(buf, pos);
            pos += 4;
            e
        } else {
            per_section
        };
        let mut records = Vec::with_capacity(expect as usize);
        for r in 0..expect {
            let Some((rec, next)) = parse_record(buf, pos, cfg.world_size_m) else {
                bail!("topo parse broke: section {s} record {r} @0x{pos:x}");
            };
            records.push(rec);
            pos = next;
        }
        sections.push(records);
    }
    let records = sections.remove(0);
    Ok(Topo {
        world_size_m: cfg.world_size_m,
        section_count,
        per_section,
        records,
        bytes: buf.len(),
        consumed: pos,
    })
}

#[cfg(test)]
#[path = "tests/topo/tests.rs"]
mod tests;
