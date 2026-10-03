//! The version 2 container's mip chain and tile rectangles.
//!
//! **Role:** derives what an archived version 2 index leaves out: the mip chain ([`mip_dims`])
//! and every tile's rectangle on its level's `tile_px` grid ([`tile_rect`]), checked against the
//! index's own grid and tile formats ([`mips_from_archive`]); and copies an index onto an 8-byte
//! boundary for validation ([`AlignedIndex`]).
//! **Position:** called by [`crate::header`] for version 2 containers; reads
//! `world_file_formats`' archived `TbdSatIndexV2`.
//! **Signals & state:** none; pure functions and an owned aligned copy.
//! **Invariants:** each level is half the one before, rounded down, at least 1, down to 1 × 1; a
//! level count, grid, tile count or tile format other than WebP that disagrees is refused.

use world_file_formats::archives::satellite::ArchivedTbdSatIndexV2;

use crate::model::{FORMAT_WEBP, TbdSatError, TbdSatMip, TbdSatTile};

/// Width and height in pixels of every level from the base (each side at least 1) down to
/// 1 × 1, each level half the one before, rounded down and never under 1.
pub(super) fn mip_dims(base_w: u32, base_h: u32) -> Vec<(u32, u32)> {
    let (mut w, mut h) = (base_w.max(1), base_h.max(1));
    let mut out = vec![(w, h)];
    while w > 1 || h > 1 {
        (w, h) = ((w / 2).max(1), (h / 2).max(1));
        out.push((w, h));
    }
    out
}

/// Pixel rectangle `(x, y, width, height)` of tile `i` (row-major) on a `lw × lh` level cut
/// into a `w_tiles × h_tiles` grid; tiles in the last column and row are clipped to the level.
pub(super) fn tile_rect(
    lw: u32,
    lh: u32,
    w_tiles: u32,
    h_tiles: u32,
    i: u32,
) -> (u32, u32, u32, u32) {
    let (tw, th) = (lw.div_ceil(w_tiles.max(1)), lh.div_ceil(h_tiles.max(1)));
    let (x, y) = ((i % w_tiles.max(1)) * tw, (i / w_tiles.max(1)) * th);
    (x, y, tw.min(lw - x), th.min(lh - y))
}

/// Rebuilds the levels of an archived version 2 index, with tile byte offsets made absolute
/// from `payload_start`; refuses a level count, tile grid, tile count or non-WebP tile that
/// disagrees with the base size and `tile_px`.
pub(super) fn mips_from_archive(
    archived: &ArchivedTbdSatIndexV2,
    payload_start: u64,
) -> Result<Vec<TbdSatMip>, TbdSatError> {
    let bad = |m: String| TbdSatError::Structure(m);
    let (base_w, base_h) = (archived.base_w.to_native(), archived.base_h.to_native());
    let tile_px = u32::from(archived.tile_px.to_native());
    if base_w < 1 || base_h < 1 || tile_px < 1 {
        return Err(bad(format!(
            "v2 base {base_w}x{base_h} / tile_px {tile_px}"
        )));
    }
    let dims = mip_dims(base_w, base_h);
    if archived.levels.len() != dims.len() {
        return Err(bad(format!(
            "v2 levels[] is {}, the chain from {base_w}x{base_h} is {}",
            archived.levels.len(),
            dims.len()
        )));
    }
    let mut mips = Vec::with_capacity(dims.len());
    for (level, &(lw, lh)) in dims.iter().enumerate() {
        let lv = &archived.levels[level];
        let (w_tiles, h_tiles) = (lv.w_tiles.to_native(), lv.h_tiles.to_native());
        if (w_tiles, h_tiles) != (lw.div_ceil(tile_px), lh.div_ceil(tile_px)) {
            return Err(bad(format!(
                "level {level}: grid {w_tiles}x{h_tiles}, tile_px {tile_px} over {lw}x{lh} means {}x{}",
                lw.div_ceil(tile_px),
                lh.div_ceil(tile_px)
            )));
        }
        if lv.tiles.len() as u64 != u64::from(w_tiles) * u64::from(h_tiles) {
            return Err(bad(format!(
                "level {level}: {} tiles for a {w_tiles}x{h_tiles} grid",
                lv.tiles.len()
            )));
        }
        let mut tiles = Vec::with_capacity(lv.tiles.len());
        for (i, t) in lv.tiles.iter().enumerate() {
            if t.format != FORMAT_WEBP {
                return Err(bad(format!("level {level} tile {i}: format {}", t.format)));
            }
            let (x, y, width, height) = tile_rect(lw, lh, w_tiles, h_tiles, i as u32);
            tiles.push(TbdSatTile {
                x,
                y,
                width,
                height,

                offset: payload_start + t.offset.to_native(),
                length: u64::from(t.len.to_native()),
            });
        }
        mips.push(TbdSatMip {
            level: level as u32,
            width: lw,
            height: lh,
            tiles,
        });
    }
    Ok(mips)
}

/// An index's bytes copied onto an 8-byte boundary: the buffer, the copy's start in it and
/// its length.
pub(super) struct AlignedIndex(pub(super) Vec<u8>, pub(super) usize, pub(super) usize);

impl AlignedIndex {
    /// Copies `src` into a fresh buffer at its first 8-byte-aligned position.
    pub(super) fn new(src: &[u8]) -> Self {
        const ALIGN: usize = 8;
        let mut buf = vec![0u8; src.len() + ALIGN];
        let at = buf.as_ptr().align_offset(ALIGN).min(ALIGN - 1);
        buf[at..at + src.len()].copy_from_slice(src);
        Self(buf, at, src.len())
    }
}

impl AlignedIndex {
    /// The aligned copy of the index bytes, exactly as long as the source.
    pub(super) fn as_slice(&self) -> &[u8] {
        &self.0[self.1..self.1 + self.2]
    }
}
