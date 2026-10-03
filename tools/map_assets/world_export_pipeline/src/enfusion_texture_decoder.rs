//! Enfusion `_supertexture.edds` decoder, BC7 through the pure-Rust `bcdec_rs`.
//!
//! The container: an EDDS header whose `dxgiFormat` is a u32LE at 0x48 (99 = BC7_UNORM_SRGB), a
//! chunk table at 0x5c of `[4B tag][u32LE len]` records with tag ∈ {`COPY`, `LZ4 `}, and mip
//! record `i` at side `1 << i` so mip 0 is the last and largest. A `COPY` chunk holds raw BC7;
//! an `LZ4 ` chunk holds `[u32LE size][u32 _][block]`.
//!
//! **Role:** one supertexture cell decoded from the game's archives to RGBA8.
//! **Position:** the `edds-cell` subcommand and the map raster pipeline's orthophoto builder read
//! cells through it.
//! **Signals & state:** none; pure decoding.
//! **Invariants:** a missing or corrupt cell is an error, never a grey fill.

use crate::error::{Result, refuse};

use enfusion_pak::PakVfs;

/// The pak folder that holds Everon's supertexture cells.
pub const EDEN_DATA_DIR: &str = "worlds/Eden/Eden/.Data";
/// Cells per side of the supertexture grid.
pub const GRID: u32 = 50;
/// Cells in the supertexture grid.
pub const CELL_COUNT: u32 = GRID * GRID;
/// Pixels per side of one decoded cell (mip 0).
pub const CELL_PX: u32 = 256;
/// Metres per side of one cell.
pub const CELL_M: u32 = 256;
/// Metres per side of the whole supertexture.
pub const WORLD_M: u32 = GRID * CELL_M;
/// The DXGI format code of sRGB BC7.
pub const DXGI_BC7_UNORM_SRGB: u32 = 99;
/// The DXGI format code of linear BC7.
pub const DXGI_BC7_UNORM: u32 = 98;

const CHUNK_TABLE_OFFSET: usize = 0x5c;
const DXGI_OFFSET: usize = 0x48;

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Virtual path for Eden cell N.
pub fn cell_path(n: u32) -> String {
    format!("{EDEN_DATA_DIR}/Eden_{n}_supertexture.edds")
}

/// Row-major grid coords for linear index N (x east, y=0 north/top).
pub fn cell_grid(n: u32) -> (u32, u32) {
    (n % GRID, n / GRID)
}

/// One record of the EDDS chunk table: one mip level's stored chunk.
pub struct MipRec {
    /// The chunk tag, `COPY` or `LZ4 `.
    pub tag: [u8; 4],
    /// The stored length in bytes.
    pub len: u32,
    /// The byte offset of the chunk's payload in the file.
    pub off: usize,
}

/// An EDDS header as parsed: the pixel format and the chunk table.
pub struct EddsInfo {
    /// The DXGI format code.
    pub dxgi: u32,
    /// The chunk records, smallest mip first.
    pub recs: Vec<MipRec>,
}

/// Parse the Enfusion EDDS header + chunk table.
pub fn parse_edds(buf: &[u8]) -> EddsInfo {
    let dxgi = u32le(buf, DXGI_OFFSET);
    let mut o = CHUNK_TABLE_OFFSET;
    let mut recs = Vec::new();
    while o + 8 <= buf.len() {
        let tag: [u8; 4] = [buf[o], buf[o + 1], buf[o + 2], buf[o + 3]];
        if &tag != b"COPY" && &tag != b"LZ4 " {
            break;
        }
        recs.push(MipRec {
            tag,
            len: u32le(buf, o + 4),
            off: 0,
        });
        o += 8;
    }
    let mut cur = o;
    for r in &mut recs {
        r.off = cur;
        cur += r.len as usize;
    }
    EddsInfo { dxgi, recs }
}

/// LZ4 raw-block decompressor for the `LZ4 ` chunk payload.
pub fn lz4_block(src: &[u8], dst_size: usize) -> Result<Vec<u8>> {
    let mut out = vec![0u8; dst_size];
    let mut s = 0usize;
    let mut d = 0usize;
    while s < src.len() {
        let tok = src[s];
        s += 1;
        let mut ll = (tok >> 4) as usize;
        if ll == 15 {
            loop {
                let x = src[s];
                s += 1;
                ll += x as usize;
                if x != 255 {
                    break;
                }
            }
        }
        out[d..d + ll].copy_from_slice(&src[s..s + ll]);
        s += ll;
        d += ll;
        if s >= src.len() {
            break;
        }
        let off = src[s] as usize | ((src[s + 1] as usize) << 8);
        s += 2;
        let mut ml = (tok & 15) as usize + 4;
        if (tok & 15) == 15 {
            loop {
                let x = src[s];
                s += 1;
                ml += x as usize;
                if x != 255 {
                    break;
                }
            }
        }
        // Overlap-forward copy: ascending index order reproduces LZ4 window semantics
        // (off >= 1 ⇒ source trails destination; freshly written bytes are re-readable).
        let m = d - off;
        for i in 0..ml {
            out[d + i] = out[m + i];
        }
        d += ml;
    }
    if d != dst_size {
        refuse!("LZ4 size mismatch: got {d}, expected {dst_size}");
    }
    Ok(out)
}

/// Raw BC7 bytes for a mip record (COPY = stored, LZ4 = `[u32 size][u32 _][block]`).
pub fn mip_bc7(buf: &[u8], rec: &MipRec, side: usize) -> Result<Vec<u8>> {
    let expected = side * side; // BC7 = 1 byte/px
    if &rec.tag == b"COPY" {
        let bc7 = &buf[rec.off..rec.off + rec.len as usize];
        if bc7.len() < expected {
            refuse!("COPY mip short: {} < {expected}", bc7.len());
        }
        return Ok(bc7.to_vec());
    }
    let body = &buf[rec.off..rec.off + rec.len as usize];
    let decomp_size = u32le(body, 0) as usize;
    if decomp_size != expected {
        refuse!("LZ4 decompSize {decomp_size} != expected {expected} (side {side})");
    }
    lz4_block(&body[8..], decomp_size)
}

/// mip0 side from mip count (smallest mip = 1px ⇒ largest = 2^(n-1)).
pub fn mip0_side(mip_count: usize) -> usize {
    1 << (mip_count - 1)
}

/// Decode a full BC7 surface (w×h, both divisible by 4) to RGBA8 through `bcdec_rs`.
pub fn decode_bc7(bc7: &[u8], w: usize, h: usize) -> Result<Vec<u8>> {
    if !w.is_multiple_of(4) || !h.is_multiple_of(4) {
        refuse!("BC7 dims must be /4, got {w}x{h}");
    }
    let src_len = w * h; // 1 byte/px = 16 B per 4×4 block
    if bc7.len() < src_len {
        refuse!("BC7 src too short: {} < {src_len}", bc7.len());
    }
    let mut rgba = vec![0u8; w * h * 4];
    let pitch = w * 4;
    let mut src = 0usize;
    for by in (0..h).step_by(4) {
        for bx in (0..w).step_by(4) {
            let dst_off = by * pitch + bx * 4;
            bcdec_rs::bc7(&bc7[src..src + 16], &mut rgba[dst_off..], pitch);
            src += 16;
        }
    }
    Ok(rgba)
}

/// One supertexture cell decoded to RGBA8.
pub struct CellRgba {
    /// The pixels, row-major, four bytes each.
    pub rgba: Vec<u8>,
    /// The side of the decoded square in pixels.
    pub side: usize,
    /// The cell's DXGI format code.
    pub dxgi: u32,
    /// How many mip records the cell's chunk table holds.
    pub mip_count: usize,
}

/// Decode cell N's mip0 to RGBA8. Throws on missing/corrupt cells (no grey fill).
pub fn decode_cell_rgba(vfs: &PakVfs, n: u32) -> Result<CellRgba> {
    let path = cell_path(n);
    if !vfs.exists(&path) {
        refuse!("cell missing in pak: {path}");
    }
    let buf = vfs.read_file(&path)?;
    let info = parse_edds(&buf);
    let mip_count = info.recs.len();
    if mip_count < 1 {
        refuse!("no mip chunks in {path}");
    }
    if info.dxgi != DXGI_BC7_UNORM_SRGB && info.dxgi != DXGI_BC7_UNORM {
        refuse!(
            "unexpected dxgiFormat {} in {path} (expected BC7 98/99)",
            info.dxgi
        );
    }
    let side = mip0_side(mip_count);
    let bc7 = mip_bc7(&buf, &info.recs[mip_count - 1], side)?;
    let rgba = decode_bc7(&bc7, side, side)?;
    Ok(CellRgba {
        rgba,
        side,
        dxgi: info.dxgi,
        mip_count,
    })
}

/// List the Eden cells present in the pak, sorted by index.
pub fn list_eden_cells(vfs: &PakVfs) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for p in vfs.all_file_paths() {
        if let Some(rest) = p.strip_prefix(EDEN_DATA_DIR)
            && let Some(name) = rest.strip_prefix("/Eden_")
            && let Some(num) = name.strip_suffix("_supertexture.edds")
            && let Ok(n) = num.parse::<u32>()
        {
            out.push((n, p.to_string()));
        }
    }
    out.sort_by_key(|(n, _)| *n);
    out
}

#[cfg(test)]
#[path = "tests/enfusion_texture_decoder/tests.rs"]
mod tests;
