//! T-935.10 — the `TBDS` **version 2** satellite container (spec §3.5): a 32-byte
//! [`TbdsHeader`] carrying `index_len`, then an rkyv [`TbdSatIndexV2`], then the tile payload.
//!
//! It replaces v1's hand-packed JSON offset table with a *validated* archive, and it exists as a
//! module of its own because [`super::unified`] is already a SIZE-1 file: the writer, the reader
//! and the geometry every one of them derives live here, and `unified.rs` keeps only the call
//! sites (CODING_STANDARDS SIZE-1/3 — allowlisted giants grow by call sites, not by features).
//!
//! **The payload is not touched.** Both container writers consume the same encoded VP8L block
//! vector in the same order, so a v1 and a v2 bundle built from one source are byte-identical
//! from `tiles_offset` on and differ only in their index — which is what turns "a v2 container
//! renders identically to the committed v1 file at every mip" into a property of the code.

use anyhow::{Result, bail};
use map_engine_core::world::binary::{
    access_checked,
    archives::{SatLevel, SatTile, TbdSatIndexV2},
    chunk_container::{ContainerHeader, HEADER_BYTES, TbdsHeader},
    to_bytes,
};

use super::img;

/// Codec code stored in [`SatTile::format`] — the builder only ever writes VP8L WebP, so anything
/// else is a container this build must refuse rather than hand to a WebP decoder.
pub(crate) const SAT_FORMAT_WEBP: u8 = 0;

/// The container version [`super::unified::build_unified_satellite`] writes when the caller does
/// not pick one.
pub const DEFAULT_CONTAINER_VERSION: u16 = 2;

/// The mip chain from a base, by the GL halving rule, ending at 1×1. One function so the builder,
/// the v2 index and the v2 verifier cannot drift about what level *n* measures — a disagreement
/// there mislocates every tile on screen without failing any cast.
pub(crate) fn mip_dims(base_w: usize, base_h: usize) -> Vec<(usize, usize)> {
    let (mut w, mut h) = (base_w.max(1), base_h.max(1));
    let mut out = vec![(w, h)];
    while w > 1 || h > 1 {
        (w, h) = (1.max(w / 2), 1.max(h / 2));
        out.push((w, h));
    }
    out
}

/// Where tile `i` of a `w_tiles × h_tiles` grid sits inside a `lw × lh` level, row-major.
///
/// The v1 builder's crop arithmetic, kept verbatim so v2 reproduces v1's tiling exactly: everon's
/// level 0 is 12800 px at `tileThreshold` 8192, which is 2×2 tiles of **6400** px each — not
/// 8192 + 4608. The reader derives the same geometry from `w_tiles`/`h_tiles`.
pub(crate) fn tile_rect(
    lw: usize,
    lh: usize,
    w_tiles: usize,
    h_tiles: usize,
    i: usize,
) -> (usize, usize, usize, usize) {
    let (tile_w, tile_h) = (lw.div_ceil(w_tiles.max(1)), lh.div_ceil(h_tiles.max(1)));
    let (x, y) = ((i % w_tiles.max(1)) * tile_w, (i / w_tiles.max(1)) * tile_h);
    (x, y, tile_w.min(lw - x), tile_h.min(lh - y))
}

/// What either container version must tell the shared manifest cross-checks.
#[derive(Debug)]
pub(crate) struct BundleSummary {
    pub(crate) base_w: u64,
    pub(crate) base_h: u64,
    pub(crate) mip_count: u64,
    pub(crate) block_count: u64,
    /// The `tiles.satellite.unified.encoding` string this container version obliges the manifest
    /// to carry. Checking it is what stops a v2 file being served under a v1 declaration.
    pub(crate) encoding: &'static str,
}

/// One encoded VP8L block, in the order the payload writes them: level-major, then row-major
/// within a level. **Both container writers consume this same vector**, which is why a v1 and a v2
/// bundle built from one source have byte-identical payloads and differ only in their index.
pub(crate) struct TileBuf {
    pub(crate) level: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) w: usize,
    pub(crate) h: usize,
    pub(crate) buf: Vec<u8>,
}

/// The rkyv index for `blocks` — payload-relative offsets accumulated in the order the payload is
/// written, so `TbdsHeader::tiles_offset() + tile.offset` is the tile's byte range in the file.
pub(crate) fn tbds_v2_index(
    blocks: &[TileBuf],
    level_meta: &[(usize, usize)],
    base: (usize, usize),
    tile_threshold: usize,
) -> Result<TbdSatIndexV2> {
    let tile_px = u16::try_from(tile_threshold).unwrap_or(0);
    if tile_px == 0 {
        bail!("tileThreshold {tile_threshold} must be 1..=65535 to fit TbdSatIndexV2::tile_px");
    }
    let mut levels = Vec::with_capacity(level_meta.len());
    let mut offset = 0u64;
    for (level, &(lw, lh)) in level_meta.iter().enumerate() {
        let mut tiles = Vec::new();
        for b in blocks.iter().filter(|b| b.level == level) {
            let len = u32::try_from(b.buf.len())?;
            tiles.push(SatTile {
                offset,
                len,
                format: SAT_FORMAT_WEBP,
            });
            offset += u64::from(len);
        }
        let (w_tiles, h_tiles) = (lw.div_ceil(tile_threshold), lh.div_ceil(tile_threshold));
        if tiles.len() != w_tiles * h_tiles {
            bail!(
                "level {level}: {} encoded blocks for a {w_tiles}x{h_tiles} grid — writer and \
                 index disagree about the tiling",
                tiles.len()
            );
        }
        levels.push(SatLevel {
            w_tiles: w_tiles as u32,
            h_tiles: h_tiles as u32,
            tiles,
        });
    }
    Ok(TbdSatIndexV2 {
        base_w: base.0 as u32,
        base_h: base.1 as u32,
        tile_px,
        levels,
    })
}

/// `TbdsHeader` + rkyv index + the payload, in that order.
pub(crate) fn tbds_v2_bytes(index: &TbdSatIndexV2, blocks: &[TileBuf]) -> Result<Vec<u8>> {
    let index_bytes = to_bytes(index)?;
    let payload: usize = blocks.iter().map(|b| b.buf.len()).sum();
    let mut file = Vec::with_capacity(HEADER_BYTES + index_bytes.len() + payload);
    file.extend_from_slice(&TbdsHeader::new(u32::try_from(index_bytes.len())?).to_header_bytes());
    file.extend_from_slice(&index_bytes);
    for b in blocks {
        file.extend_from_slice(&b.buf);
    }
    Ok(file)
}

/// The v2 (rkyv index) bundle checks (T-935.10). `None` = fatal; the message is already pushed.
pub(crate) fn verify_bundle_v2(buf: &[u8], errors: &mut Vec<String>) -> Option<BundleSummary> {
    match read_bundle_v2(buf) {
        Ok(s) => Some(s),
        Err(e) => {
            errors.push(e);
            None
        }
    }
}

/// **Layout is not meaning.** `access_checked` proves rkyv can walk the index; it says nothing
/// about whether this build and the writer agree what the fields *mean*. So on top of it this
/// re-derives the whole geometry — the mip chain from `base_w`/`base_h`, each level's grid from
/// `tile_px`, each tile's rect from `w_tiles`/`h_tiles` — and holds the VP8L headers in the
/// payload against it. A `tile_px` whose meaning drifted (nominal threshold vs actual tile edge)
/// validates perfectly and paints everon's four 6400 px quadrants in the wrong places.
///
/// Fail-fast rather than v1's collect-every-error: a v2 index that disagrees with the payload is
/// not an audit finding, it is a container this build must not read.
fn read_bundle_v2(buf: &[u8]) -> Result<BundleSummary, String> {
    let (header, payload) = TbdsHeader::read(buf).map_err(|e| format!("TBDS v2 header: {e}"))?;
    let index_bytes = header
        .index(payload)
        .map_err(|e| format!("index_len {} overruns file: {e}", header.index_len))?;
    let aligned = AlignedArchive::new(index_bytes);
    let index = access_checked::<TbdSatIndexV2>(aligned.as_slice())
        .map_err(|e| format!("rkyv index does not validate: {e}"))?;

    let tiles_offset = header.tiles_offset();
    let (base_w, base_h) = (index.base_w.to_native(), index.base_h.to_native());
    let tile_px = index.tile_px.to_native() as usize;
    if base_w < 1 || base_h < 1 || tile_px < 1 {
        return Err(format!(
            "index base {base_w}x{base_h} / tile_px {tile_px}: all must be >= 1"
        ));
    }
    let dims = mip_dims(base_w as usize, base_h as usize);
    if index.levels.len() != dims.len() {
        return Err(format!(
            "levels[] length {} !== floor(log2(base))+1 = {}",
            index.levels.len(),
            dims.len()
        ));
    }
    let (mut block_count, mut payload_bytes) = (0u64, 0u64);
    for (level, (lw, lh)) in dims.iter().copied().enumerate() {
        let lv = &index.levels[level];
        let (w_tiles, h_tiles) = (
            lv.w_tiles.to_native() as usize,
            lv.h_tiles.to_native() as usize,
        );
        if (w_tiles, h_tiles) != (lw.div_ceil(tile_px), lh.div_ceil(tile_px)) {
            return Err(format!(
                "level {level}: grid {w_tiles}x{h_tiles}, tile_px {tile_px} over {lw}x{lh} expects {}x{}",
                lw.div_ceil(tile_px),
                lh.div_ceil(tile_px)
            ));
        }
        if lv.tiles.len() != w_tiles * h_tiles {
            return Err(format!(
                "level {level}: {} tiles for a {w_tiles}x{h_tiles} grid",
                lv.tiles.len()
            ));
        }
        for (i, t) in lv.tiles.iter().enumerate() {
            block_count += 1;
            let (off, len) = (t.offset.to_native() as usize, t.len.to_native() as usize);
            payload_bytes += len as u64;
            let (x, y, tw, th) = tile_rect(lw, lh, w_tiles, h_tiles, i);
            let at = format!("level {level} tile @({x},{y})");
            if t.format != SAT_FORMAT_WEBP {
                return Err(format!(
                    "{at}: format {} is not webp ({SAT_FORMAT_WEBP})",
                    t.format
                ));
            }
            let end = tiles_offset
                .checked_add(off)
                .and_then(|s| s.checked_add(len));
            if end.is_none_or(|e| e > buf.len()) {
                return Err(format!(
                    "{at}: offset {tiles_offset}+{off}+{len} out of range"
                ));
            }
            let start = tiles_offset + off;
            let d = img::webp_dims(&buf[start..start + len.min(64)])
                .ok_or_else(|| format!("{at}: not a RIFF/WEBP block"))?;
            if &d.fourcc != b"VP8L" {
                let got = String::from_utf8_lossy(&d.fourcc).into_owned();
                return Err(format!("{at}: {got}, expected VP8L (lossless)"));
            }
            if d.w as usize != tw || d.h as usize != th {
                return Err(format!(
                    "{at}: VP8L says {}x{}, the grid derives {tw}x{th}",
                    d.w, d.h
                ));
            }
        }
    }
    if tiles_offset as u64 + payload_bytes != buf.len() as u64 {
        return Err(format!(
            "payload bytes {payload_bytes} + header/index {tiles_offset} !== file size {}",
            buf.len()
        ));
    }
    Ok(BundleSummary {
        base_w: u64::from(base_w),
        base_h: u64::from(base_h),
        mip_count: index.levels.len() as u64,
        block_count,
        encoding: "tbd-sat-v2",
    })
}

/// rkyv index bytes copied once into an 8-aligned window: `(buffer, start)`.
///
/// `access_checked` **validates** alignment rather than assuming it, and index bytes lifted out of
/// a file buffer (or an HTTP Range body) start at byte 32 of whatever the allocator handed back —
/// so reading them in place is a coin flip that loses as "the archive is corrupt". Over-allocating
/// by one alignment needs neither `unsafe` nor a new dependency.
struct AlignedArchive(Vec<u8>, usize, usize);

impl AlignedArchive {
    fn new(src: &[u8]) -> Self {
        const ALIGN: usize = 8;
        let mut buf = vec![0u8; src.len() + ALIGN];
        let at = buf.as_ptr().align_offset(ALIGN).min(ALIGN - 1);
        buf[at..at + src.len()].copy_from_slice(src);
        Self(buf, at, src.len())
    }

    fn as_slice(&self) -> &[u8] {
        &self.0[self.1..self.1 + self.2]
    }
}

#[cfg(test)]
mod t935_10 {
    use serde_json::Value;

    use super::*;
    use crate::map::unified::build_tbds_v1_bytes;

    /// 5 px base at `tile_px` 2: the chain is 5 → 2 → 1 (three mips), and level 0 is a ragged 3×3
    /// grid whose last column and row are 1 px wide. That raggedness is the point — a reader that
    /// treats `tile_px` as the *actual* tile edge rather than the threshold derives 2 px there and
    /// lands every tile of the level in the wrong place while every cast still succeeds.
    fn synthetic() -> (Vec<TileBuf>, Vec<(usize, usize)>) {
        let meta = mip_dims(5, 5);
        let mut blocks = Vec::new();
        for (level, &(lw, lh)) in meta.iter().enumerate() {
            let (wt, ht) = (lw.div_ceil(2), lh.div_ceil(2));
            for i in 0..wt * ht {
                let (x, y, w, h) = tile_rect(lw, lh, wt, ht, i);
                // A distinct fill per tile, so identical payload bytes mean identical tiles.
                let data = vec![(level * 37 + i * 11 + 1) as u8; w * h * 3];
                let buf = img::encode_webp_lossless_rgb(&img::Rgb8 { w, h, data }).expect("vp8l");
                blocks.push(TileBuf {
                    level,
                    x,
                    y,
                    w,
                    h,
                    buf,
                });
            }
        }
        (blocks, meta)
    }

    fn v2_of(blocks: &[TileBuf], meta: &[(usize, usize)]) -> Vec<u8> {
        let index = tbds_v2_index(blocks, meta, (5, 5), 2).expect("index");
        tbds_v2_bytes(&index, blocks).expect("frame")
    }

    /// The acceptance, at container level: one source, two containers, and the tiles a reader
    /// finds must be the same bytes at the same rects. The payload comparison is what makes
    /// "renders identically at every mip" checkable without a GPU.
    #[test]
    fn v1_and_v2_carry_the_same_tiles_at_the_same_rects() {
        let (blocks, meta) = synthetic();
        let v1 = build_tbds_v1_bytes(
            &blocks,
            &meta,
            (5, 5),
            "arland",
            [0, 0, 4096, 4096],
            &Value::Null,
            "0",
        )
        .expect("v1");
        let v2 = v2_of(&blocks, &meta);

        let json_len = u32::from_le_bytes(v1[8..12].try_into().unwrap()) as usize;
        let idx_len = u32::from_le_bytes(v2[8..12].try_into().unwrap()) as usize;
        assert_eq!(
            &v1[12 + json_len..],
            &v2[HEADER_BYTES + idx_len..],
            "the two containers must share a byte-identical payload"
        );

        let summary = read_bundle_v2(&v2).expect("the v2 bundle must verify");
        assert_eq!(
            (
                summary.base_w,
                summary.base_h,
                summary.mip_count,
                summary.block_count,
                summary.encoding
            ),
            (5, 5, 3, 11, "tbd-sat-v2")
        );

        // Every rect the v1 table states is the rect the v2 grid derives, and at the same absolute
        // byte range — the two indexes describe the same picture.
        let table: Value = serde_json::from_slice(&v1[12..12 + json_len]).expect("v1 json");
        let index_bytes = AlignedArchive::new(&v2[HEADER_BYTES..HEADER_BYTES + idx_len]);
        let index = access_checked::<TbdSatIndexV2>(index_bytes.as_slice()).expect("v2 index");
        let mut seen = 0;
        for (level, &(lw, lh)) in meta.iter().enumerate() {
            let (wt, ht) = (lw.div_ceil(2), lh.div_ceil(2));
            let lv = &index.levels[level];
            let mips = table["mips"][level]["tiles"].as_array().expect("v1 tiles");
            assert_eq!(mips.len(), lv.tiles.len(), "level {level} tile count");
            for (i, t) in mips.iter().enumerate() {
                let (x, y, w, h) = tile_rect(lw, lh, wt, ht, i);
                assert_eq!(
                    (
                        t["x"].as_u64(),
                        t["y"].as_u64(),
                        t["width"].as_u64(),
                        t["height"].as_u64()
                    ),
                    (
                        Some(x as u64),
                        Some(y as u64),
                        Some(w as u64),
                        Some(h as u64)
                    ),
                    "level {level} tile {i} rect"
                );
                let abs = HEADER_BYTES as u64 + idx_len as u64 + lv.tiles[i].offset.to_native();
                assert_eq!(
                    t["offset"].as_u64(),
                    Some(12 + json_len as u64 + lv.tiles[i].offset.to_native()),
                    "level {level} tile {i} v1 offset"
                );
                assert_eq!(
                    &v1[t["offset"].as_u64().unwrap() as usize
                        ..t["offset"].as_u64().unwrap() as usize
                            + t["length"].as_u64().unwrap() as usize],
                    &v2[abs as usize..abs as usize + lv.tiles[i].len.to_native() as usize],
                    "level {level} tile {i} bytes"
                );
                seen += 1;
            }
        }
        assert_eq!(seen, 11, "9 + 1 + 1 tiles over three mips");
    }

    /// The perturbation the ticket names: `index_len` one byte short.
    #[test]
    fn an_index_len_one_byte_short_is_rejected() {
        let (blocks, meta) = synthetic();
        let mut f = v2_of(&blocks, &meta);
        let short = u32::from_le_bytes(f[8..12].try_into().unwrap()) - 1;
        f[8..12].copy_from_slice(&short.to_le_bytes());
        let e = read_bundle_v2(&f).expect_err("a short index_len must not validate");
        assert!(e.contains("rkyv index does not validate"), "{e}");
    }

    /// Meaning, not layout — isolated so that **only** the grid rule can catch the fault. Moving
    /// `tile_px` alone leaves the rects, the offsets, the VP8L dimensions and bytecheck all intact:
    /// a verifier without the rule reports OK over a container it reads as a different picture.
    #[test]
    fn a_grid_that_disagrees_with_tile_px_is_rejected() {
        let (blocks, meta) = synthetic();
        let mut index = tbds_v2_index(&blocks, &meta, (5, 5), 2).expect("index");
        index.tile_px = 4;
        let f = tbds_v2_bytes(&index, &blocks).expect("frame");
        let e = read_bundle_v2(&f).expect_err("a grid that contradicts tile_px must not pass");
        assert!(
            e.contains("level 0: grid 3x3, tile_px 4 over 5x5 expects 2x2"),
            "{e}"
        );
    }

    #[test]
    fn an_unknown_tile_format_is_rejected() {
        let (blocks, meta) = synthetic();
        let mut index = tbds_v2_index(&blocks, &meta, (5, 5), 2).expect("index");
        index.levels[2].tiles[0].format = 1;
        let f = tbds_v2_bytes(&index, &blocks).expect("frame");
        let e = read_bundle_v2(&f).expect_err("an unknown codec must not be handed to a decoder");
        assert!(e.contains("format 1 is not webp"), "{e}");
    }

    /// The committed `everon-sat.tbd-sat` index, read off the bundle: 14 levels from 12800², and
    /// level 0 is four 6400 px quadrants — **not** 8192 + 4608. If the v2 derivation ever stops
    /// reproducing that, a regenerated everon paints its quadrants in the wrong places.
    #[test]
    fn the_v2_derivation_reproduces_the_committed_everon_tiling() {
        let dims = mip_dims(12_800, 12_800);
        assert_eq!(dims.len(), 14);
        assert_eq!(dims[1], (6_400, 6_400));
        assert_eq!(*dims.last().expect("chain"), (1, 1));
        assert_eq!(12_800_usize.div_ceil(8_192), 2, "level 0 is a 2x2 grid");
        let rects: Vec<_> = (0..4).map(|i| tile_rect(12_800, 12_800, 2, 2, i)).collect();
        assert_eq!(
            rects,
            vec![
                (0, 0, 6_400, 6_400),
                (6_400, 0, 6_400, 6_400),
                (0, 6_400, 6_400, 6_400),
                (6_400, 6_400, 6_400, 6_400),
            ]
        );
        assert_eq!(6_400_usize.div_ceil(8_192), 1, "level 1 is one whole tile");
        assert_eq!(tile_rect(6_400, 6_400, 1, 1, 0), (0, 0, 6_400, 6_400));
    }
}
