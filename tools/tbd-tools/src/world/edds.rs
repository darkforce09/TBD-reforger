//! T-165.8 — Enfusion `_supertexture.edds` decoder (port of `scripts/map-assets/decode-edds.mjs`;
//! BC7 via the pure-Rust `bcdec_rs` — replaces the vendored bcdec.wasm + JS glue).
//!
//! See the .mjs header for the cracked pipeline: EDDS header (dxgiFormat u32LE @ 0x48, 99 =
//! BC7_UNORM_SRGB), chunk table @ 0x5c ([4B tag][u32LE len], tag ∈ {COPY, "LZ4 "}), mip
//! record i side = 1<<i (mip0 = last/largest), COPY = raw BC7, LZ4 = [u32LE size][u32 _][block].

use anyhow::{Result, bail};

use super::pak::PakVfs;

pub const EDEN_DATA_DIR: &str = "worlds/Eden/Eden/.Data";
pub const GRID: u32 = 50;
pub const CELL_COUNT: u32 = GRID * GRID;
pub const CELL_PX: u32 = 256;
pub const CELL_M: u32 = 256;
pub const WORLD_M: u32 = GRID * CELL_M;
pub const DXGI_BC7_UNORM_SRGB: u32 = 99;
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

pub struct MipRec {
    pub tag: [u8; 4],
    pub len: u32,
    pub off: usize,
}

pub struct EddsInfo {
    pub dxgi: u32,
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

/// LZ4 raw-block decompressor (byte-identical port of the proven .mjs implementation).
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
        bail!("LZ4 size mismatch: got {d}, expected {dst_size}");
    }
    Ok(out)
}

/// Raw BC7 bytes for a mip record (COPY = stored, LZ4 = [u32 size][u32 _][block]).
pub fn mip_bc7(buf: &[u8], rec: &MipRec, side: usize) -> Result<Vec<u8>> {
    let expected = side * side; // BC7 = 1 byte/px
    if &rec.tag == b"COPY" {
        let bc7 = &buf[rec.off..rec.off + rec.len as usize];
        if bc7.len() < expected {
            bail!("COPY mip short: {} < {expected}", bc7.len());
        }
        return Ok(bc7.to_vec());
    }
    let body = &buf[rec.off..rec.off + rec.len as usize];
    let decomp_size = u32le(body, 0) as usize;
    if decomp_size != expected {
        bail!("LZ4 decompSize {decomp_size} != expected {expected} (side {side})");
    }
    lz4_block(&body[8..], decomp_size)
}

/// mip0 side from mip count (smallest mip = 1px ⇒ largest = 2^(n-1)).
pub fn mip0_side(mip_count: usize) -> usize {
    1 << (mip_count - 1)
}

/// Decode a full BC7 surface (w×h, /4 dims) to RGBA8 — the vendor/bc7.mjs contract, on
/// pure-Rust `bcdec_rs` instead of the wasm build.
pub fn decode_bc7(bc7: &[u8], w: usize, h: usize) -> Result<Vec<u8>> {
    if !w.is_multiple_of(4) || !h.is_multiple_of(4) {
        bail!("BC7 dims must be /4, got {w}x{h}");
    }
    let src_len = w * h; // 1 byte/px = 16 B per 4×4 block
    if bc7.len() < src_len {
        bail!("BC7 src too short: {} < {src_len}", bc7.len());
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

pub struct CellRgba {
    pub rgba: Vec<u8>,
    pub side: usize,
    pub dxgi: u32,
    pub mip_count: usize,
}

/// Decode cell N's mip0 to RGBA8. Throws on missing/corrupt cells (no grey fill).
pub fn decode_cell_rgba(vfs: &PakVfs, n: u32) -> Result<CellRgba> {
    let path = cell_path(n);
    if !vfs.exists(&path) {
        bail!("cell missing in pak: {path}");
    }
    let buf = vfs.read_file(&path)?;
    let info = parse_edds(&buf);
    let mip_count = info.recs.len();
    if mip_count < 1 {
        bail!("no mip chunks in {path}");
    }
    if info.dxgi != DXGI_BC7_UNORM_SRGB && info.dxgi != DXGI_BC7_UNORM {
        bail!(
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
mod tests {
    use super::*;

    /// The vendored bc7.test.mjs golden, ported verbatim: a varied 4×4 BC7 block from
    /// Eden_1174 mip0; EXPECT_RGB produced by an INDEPENDENT decoder (Pillow) — guards the
    /// decoder against wrong layout, not a circular self-check.
    #[test]
    fn bc7_decodes_known_block() {
        let block: Vec<u8> = (0..16)
            .map(|i| {
                u8::from_str_radix(&"c05ae575293dfeff0d726b56cf79ef7b"[i * 2..i * 2 + 2], 16)
                    .unwrap()
            })
            .collect();
        #[rustfmt::skip]
        let expect_rgb: [u8; 48] = [
            81, 76, 57, 107, 95, 75, 98, 88, 69, 77, 73, 54,
            60, 60, 43, 81, 76, 57, 81, 76, 57, 86, 79, 61,
            43, 47, 31, 56, 57, 40, 69, 67, 49, 77, 73, 54,
            43, 47, 31, 47, 50, 34, 60, 60, 43, 77, 73, 54,
        ];
        let rgba = decode_bc7(&block, 4, 4).unwrap();
        assert_eq!(rgba.len(), 64, "4x4 RGBA = 64 bytes");
        let rgb: Vec<u8> = (0..16)
            .flat_map(|i| rgba[i * 4..i * 4 + 3].to_vec())
            .collect();
        assert_eq!(rgb, expect_rgb);
    }

    #[test]
    fn bc7_rejects_bad_dims() {
        assert!(decode_bc7(&[0u8; 16], 3, 4).is_err());
    }

    #[test]
    fn lz4_roundtrip_simple() {
        // literal-only block: token 0x50 (5 literals, no match at end)
        let src = [0x50, b'h', b'e', b'l', b'l', b'o'];
        assert_eq!(lz4_block(&src, 5).unwrap(), b"hello");
    }
}
