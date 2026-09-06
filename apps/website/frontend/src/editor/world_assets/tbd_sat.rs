//! T-166 — TBDS container parse. Port of React `satelliteUnified.ts` (tag T-159.29.2).
//! The engine never parses TBDS (T-151.1 L2); the Leptos host owns fetch + structural validate.
//!
//! T-935.10 — **two container versions behind one magic.** v1 is the hand-packed JSON offset table
//! `everon-sat.tbd-sat` is committed as; v2 is a 32-byte [`TbdsHeader`] plus a validated rkyv
//! [`TbdSatIndexV2`] (spec §3.5). [`parse_header`] dispatches on the version field and both paths
//! produce the same [`TbdSatIndex`], so every caller downstream — level picking, the tile Range
//! plan, `commit_mip` — is version-blind and the tile Range math is shared by construction rather
//! than by discipline.
//!
//! v2's index does **not** store per-tile rects: it stores a per-level tile grid, and the reader
//! derives each rect the way the writer cropped it. That derivation is the whole risk in the
//! format, so it is checked against the writer's rule rather than assumed — see
//! [`mips_from_archive`].

use map_engine_core::world::binary::{
    access_checked,
    archives::{ArchivedTbdSatIndexV2, TbdSatIndexV2},
    chunk_container::{ContainerHeader, TbdsHeader},
};
use serde::Deserialize;

const MAGIC: u32 = 0x5344_4254; // "TBDS" LE

/// The hand-packed JSON table (T-166) and the rkyv index (T-935.10). Both are `TBDS`.
const V1: u16 = 1;
const V2: u16 = 2;

/// `SatTile::format` code this build can decode. Anything else is a codec the WebP decoder
/// downstream would be handed bytes it cannot read, so the parse refuses it here.
const FORMAT_WEBP: u8 = 0;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatTile {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatMip {
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TbdSatTile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatIndex {
    /// Which TBDS container this index came out of: 1 or 2. **Not a wire field on either side** —
    /// `serde(skip)` keeps it out of the v1 JSON and the parser stamps it, so `validate_index` can
    /// hold the index's own declared `format_version` against the container it was actually read
    /// from instead of against a hardcoded 1.
    #[serde(skip)]
    pub container_version: u32,
    pub format_version: u32,
    // v1-only: present in the `.tbd-sat` v1 JSON header and retained to document that schema, but
    // the host keys off world size / mips. v2 does not carry them (they live in `manifest.json`,
    // spec §5), so they are `None` there rather than a fabricated default.
    #[allow(dead_code)]
    pub terrain_id: Option<String>,
    #[allow(dead_code)]
    pub world_bounds: Option<[f64; 4]>,
    pub base_width_px: u32,
    pub base_height_px: u32,
    pub mip_count: u32,
    pub mips: Vec<TbdSatMip>,
}

#[derive(Debug)]
pub enum TbdSatError {
    TooSmall,
    BadMagic,
    UnsupportedVersion(u32),
    JsonOverrun,
    Json(String),
    Structure(String),
    /// T-935.10 — the v2 header or its rkyv index: truncated, misaligned, or failing bytecheck.
    Archive(String),
}

impl std::fmt::Display for TbdSatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall => write!(f, "tbd-sat: file too small for header"),
            Self::BadMagic => write!(f, "tbd-sat: bad magic (expected TBDS)"),
            Self::UnsupportedVersion(v) => write!(f, "tbd-sat: unsupported formatVersion {v}"),
            Self::JsonOverrun => write!(f, "tbd-sat: JSON index overruns file"),
            Self::Json(e) => write!(f, "tbd-sat: JSON index unparseable: {e}"),
            Self::Structure(e) => write!(f, "tbd-sat: {e}"),
            Self::Archive(e) => write!(f, "tbd-sat: v2 index: {e}"),
        }
    }
}

fn read_u32_le(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

/// Parse header + index; `file_size` is the full-file size (a Range head may be shorter).
///
/// The version lives at the same offset in both containers — v1 writes a `u32` 1 there, v2's
/// `TbdsHeader` a `u16` 2 followed by a zero `flags` — so the dispatch reads two bytes and does
/// not have to know the header's shape before it has chosen one. v1 re-reads the full `u32` and
/// still rejects anything but 1, so a hypothetical `65537` cannot be mistaken for it.
pub fn parse_header(buf: &[u8], file_size: u64) -> Result<(TbdSatIndex, u64), TbdSatError> {
    if buf.len() < 12 {
        return Err(TbdSatError::TooSmall);
    }
    let magic = read_u32_le(buf, 0).ok_or(TbdSatError::TooSmall)?;
    if magic != MAGIC {
        return Err(TbdSatError::BadMagic);
    }
    match u16::from_le_bytes([buf[4], buf[5]]) {
        V1 => parse_header_v1(buf, file_size),
        V2 => parse_header_v2(buf, file_size),
        v => Err(TbdSatError::UnsupportedVersion(u32::from(v))),
    }
}

/// v1: `"TBDS"`, `u32` version, `u32` jsonLength, then the serde table. Unchanged since T-166 —
/// the committed `everon-sat.tbd-sat` is read by exactly this path.
fn parse_header_v1(buf: &[u8], file_size: u64) -> Result<(TbdSatIndex, u64), TbdSatError> {
    let version = read_u32_le(buf, 4).ok_or(TbdSatError::TooSmall)?;
    if version != u32::from(V1) {
        return Err(TbdSatError::UnsupportedVersion(version));
    }
    let json_len = read_u32_le(buf, 8).ok_or(TbdSatError::TooSmall)? as u64;
    if 12 + json_len > buf.len() as u64 {
        return Err(TbdSatError::JsonOverrun);
    }
    let mut index: TbdSatIndex = serde_json::from_slice(&buf[12..12 + json_len as usize])
        .map_err(|e| TbdSatError::Json(e.to_string()))?;
    index.container_version = u32::from(V1);
    let payload_start = 12 + json_len;
    if payload_start > file_size {
        return Err(TbdSatError::JsonOverrun);
    }
    Ok((index, payload_start))
}

/// v2: a 32-byte [`TbdsHeader`] whose `version` field this build pins to 2, then `index_len` bytes
/// of rkyv [`TbdSatIndexV2`], then the tiles.
///
/// **Two versions are checked, not one.** `access_checked` proves rkyv can walk the buffer; the
/// header's `version` — validated by [`TbdsHeader::read`] — is what says this build and the writer
/// mean the same thing by the fields inside it. `TbdSatIndexV2` is the one T-935.1 archive with no
/// `schema_version` of its own, and the container header carries it instead.
fn parse_header_v2(buf: &[u8], file_size: u64) -> Result<(TbdSatIndex, u64), TbdSatError> {
    let (header, payload) =
        TbdsHeader::read(buf).map_err(|e| TbdSatError::Archive(e.to_string()))?;
    let index_bytes = header
        .index(payload)
        .map_err(|e| TbdSatError::Archive(e.to_string()))?;
    let aligned = AlignedIndex::new(index_bytes);
    let archived = access_checked::<TbdSatIndexV2>(aligned.as_slice())
        .map_err(|e| TbdSatError::Archive(e.to_string()))?;
    let payload_start = header.tiles_offset() as u64;
    if payload_start > file_size {
        return Err(TbdSatError::JsonOverrun);
    }
    let (base_w, base_h) = (archived.base_w.to_native(), archived.base_h.to_native());
    let mips = mips_from_archive(archived, payload_start)?;
    Ok((
        TbdSatIndex {
            container_version: u32::from(V2),
            format_version: u32::from(V2),
            terrain_id: None,
            world_bounds: None,
            base_width_px: base_w,
            base_height_px: base_h,
            mip_count: mips.len() as u32,
            mips,
        },
        payload_start,
    ))
}

/// The mip chain from a base, GL halving rule, ending at 1×1 — the writer's own chain.
fn mip_dims(base_w: u32, base_h: u32) -> Vec<(u32, u32)> {
    let (mut w, mut h) = (base_w.max(1), base_h.max(1));
    let mut out = vec![(w, h)];
    while w > 1 || h > 1 {
        (w, h) = ((w / 2).max(1), (h / 2).max(1));
        out.push((w, h));
    }
    out
}

/// Where tile `i` of a `w_tiles × h_tiles` grid sits in a `lw × lh` level, row-major — the crop
/// arithmetic `tbd-tools`' builder uses, mirrored here because v2 stores the grid, not the rects.
fn tile_rect(lw: u32, lh: u32, w_tiles: u32, h_tiles: u32, i: u32) -> (u32, u32, u32, u32) {
    let (tw, th) = (lw.div_ceil(w_tiles.max(1)), lh.div_ceil(h_tiles.max(1)));
    let (x, y) = ((i % w_tiles.max(1)) * tw, (i / w_tiles.max(1)) * th);
    (x, y, tw.min(lw - x), th.min(lh - y))
}

/// Turn the archived grid into the per-tile rects every caller downstream already understands.
///
/// **This is where meaning is checked, and it is not the coverage rule.** A v2 index cannot
/// express a gap — the rects are derived to tile the level exactly, so `validate_index`'s coverage
/// arithmetic is satisfied by construction and proves nothing here. The invariant that *can* be
/// violated is what `tile_px` means: the writer stores the tiling **threshold**, so a level's grid
/// must be `ceil(level / tile_px)`. Everon's level 0 is 12800 px at threshold 8192 — a 2×2 grid of
/// **6400 px** tiles, not 8192 + 4608 — and a build that read `tile_px` as the actual tile edge
/// would pass `access_checked`, pass coverage, and hang all four quadrants in the wrong places.
fn mips_from_archive(
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
                // Payload-relative on the wire, absolute here: every caller downstream (the tile
                // Range plan, `split_range`, the `content-range` cross-check) reads whole-file
                // offsets, which is how v1 and v2 come to share that arithmetic untouched.
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

/// rkyv index bytes copied once into an 8-aligned window.
///
/// `access_checked` **validates** alignment rather than assuming it, and the index begins at byte
/// 32 of a `Vec<u8>` straight off a Range fetch — so reading it in place is a coin flip that loses
/// as "the archive is corrupt". Over-allocating by one alignment and copying to the first aligned
/// byte inside costs one memcpy of a few KB and needs neither `unsafe` nor a new SPA dependency.
struct AlignedIndex(Vec<u8>, usize, usize);

impl AlignedIndex {
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

fn validate_mip_tiles(
    mip: &TbdSatMip,
    payload_start: u64,
    file_size: u64,
) -> Result<(), TbdSatError> {
    let mut covered: u64 = 0;
    for t in &mip.tiles {
        if t.offset < payload_start || t.offset + t.length > file_size {
            return Err(TbdSatError::Structure(format!(
                "level {} block out of range",
                mip.level
            )));
        }
        if t.x + t.width > mip.width || t.y + t.height > mip.height {
            return Err(TbdSatError::Structure(format!(
                "level {} tile exceeds level bounds",
                mip.level
            )));
        }
        covered += u64::from(t.width) * u64::from(t.height);
    }
    let expect = u64::from(mip.width) * u64::from(mip.height);
    if covered != expect {
        return Err(TbdSatError::Structure(format!(
            "level {} tiles do not cover the level",
            mip.level
        )));
    }
    Ok(())
}

/// Index-only parse for Range preview (React `parseTbdSatIndexOnly`).
pub fn parse_tbd_sat_index_only(buf: &[u8], file_size: u64) -> Result<TbdSatIndex, TbdSatError> {
    let (index, payload_start) = parse_header(buf, file_size)?;
    validate_index(&index, payload_start, file_size, false)?;
    Ok(index)
}

/// T-627 — header + index out of a **partial** buffer, validated with the full mip-chain rules.
///
/// Replaces the whole-buffer `parse_tbd_sat` (React `parseTbdSat`), which was the only strict
/// parse and needed all 152 MB in hand to run. Dropping the whole-file GET in favour of per-tile
/// Range requests would otherwise have quietly traded those checks (level numbering, the halving
/// rule, tiles covering their level, the 1×1 terminator) for the loose block-range one. It does not
/// have to: every one of those rules reads `offset` / `length` / `width` / `height` against
/// `file_size`, and not one of them reads a payload byte. Same validation, ~2.6 KB of index instead
/// of the whole texture.
pub fn parse_tbd_sat_index_strict(buf: &[u8], file_size: u64) -> Result<TbdSatIndex, TbdSatError> {
    let (index, payload_start) = parse_header(buf, file_size)?;
    validate_index(&index, payload_start, file_size, true)?;
    Ok(index)
}

fn validate_index(
    index: &TbdSatIndex,
    payload_start: u64,
    file_size: u64,
    full_coverage: bool,
) -> Result<(), TbdSatError> {
    // Against the container it was actually read from, not a hardcoded 1: a v1 table declaring
    // formatVersion 2 (or the reverse) is a file whose two halves disagree about their own format.
    if index.format_version != index.container_version {
        return Err(TbdSatError::Structure(format!(
            "index formatVersion {} !== container version {}",
            index.format_version, index.container_version
        )));
    }
    if index.base_width_px < 1 || index.base_height_px < 1 {
        return Err(TbdSatError::Structure(
            "bad baseWidthPx/baseHeightPx".into(),
        ));
    }
    if index.mips.len() as u32 != index.mip_count || index.mip_count < 1 {
        return Err(TbdSatError::Structure(
            "mips[] does not match mipCount".into(),
        ));
    }
    if full_coverage {
        let mut w = index.base_width_px;
        let mut h = index.base_height_px;
        for (i, mip) in index.mips.iter().enumerate() {
            if mip.level != i as u32 {
                return Err(TbdSatError::Structure(format!(
                    "mips[{i}].level = {}",
                    mip.level
                )));
            }
            if mip.width != w || mip.height != h {
                return Err(TbdSatError::Structure(format!(
                    "level {i} is {}x{}, GL rule expects {w}x{h}",
                    mip.width, mip.height
                )));
            }
            validate_mip_tiles(mip, payload_start, file_size)?;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        let last = index.mips.last().unwrap();
        if last.width != 1 || last.height != 1 {
            return Err(TbdSatError::Structure("mip chain must end at 1x1".into()));
        }
    } else {
        for mip in &index.mips {
            for t in &mip.tiles {
                if t.offset < payload_start || t.offset + t.length > file_size {
                    return Err(TbdSatError::Structure(format!(
                        "level {} block out of file range",
                        mip.level
                    )));
                }
            }
        }
    }
    Ok(())
}

/// First mip whose long edge fits `max_texture_dimension_2d`.
pub fn pick_base_level(index: &TbdSatIndex, max_texture_dimension_2d: u32) -> u32 {
    for mip in &index.mips {
        if mip.width.max(mip.height) <= max_texture_dimension_2d {
            return mip.level;
        }
    }
    index.mip_count.saturating_sub(1)
}

/// T-629 — [`pick_base_level`] over a limit that **may not be known**.
///
/// The displayed resolution of the whole basemap is this one number's output, and the caller used
/// to reach it through `engine.max_texture_dimension_2d()`…`.unwrap_or(8192)`. That default is not
/// a conservative choice, it is a *silent* one: on everon, 8192 forbids the 12800 px level 0 and
/// commits level 1 — exactly half resolution — with nothing on screen or in the console to say a
/// limit had been assumed rather than read. An assumed limit and a measured one are not
/// interchangeable inputs, so they do not share a type here: no limit means **no level**, and the
/// caller has to decide out loud what to do about it.
#[must_use]
pub fn pick_base_level_for_limit(
    index: &TbdSatIndex,
    max_texture_dimension_2d: Option<u32>,
) -> Option<u32> {
    max_texture_dimension_2d.map(|max| pick_base_level(index, max))
}

/// Coarsest-usable preview mip (long edge ≤ `max_edge_px`).
pub fn pick_preview_level(index: &TbdSatIndex, max_edge_px: u32) -> &TbdSatMip {
    for mip in &index.mips {
        if mip.width.max(mip.height) <= max_edge_px {
            return mip;
        }
    }
    index.mips.last().unwrap()
}

/// T-935.10 — the two containers, read by the one parser.
///
/// These run on the HOST, through `mission_editor::tbd_sat_pure` (mission_editor.rs:3115): the
/// module is `cfg(target_arch = "wasm32")` in the bundle, and mounting it a second time under
/// `cfg(all(test, not(wasm32)))` is what lets the rkyv path be *executed* rather than only
/// compiled for wasm32. The v2 containers below are framed with the same `TbdsHeader` and
/// `to_bytes` the `tbd-tools` writer uses, so a change to either side of the format shows up here.
#[cfg(test)]
mod t935_10 {
    use map_engine_core::world::binary::{
        archives::{SatLevel, SatTile, TbdSatIndexV2},
        chunk_container::{TbdsHeader, HEADER_BYTES},
        to_bytes,
    };

    use super::*;

    /// 5 px base at `tile_px` 2 — three mips (5 → 2 → 1) and a **ragged** 3×3 level-0 grid whose
    /// last column and row are 1 px. The raggedness is the point: a reader that took `tile_px` for
    /// the actual tile edge would derive 2 px there and misplace the level while every cast, every
    /// bytecheck and the coverage rule all still passed.
    const BASE: u32 = 5;
    const TILE_PX: u16 = 2;
    /// One length for every tile, so a tile's payload bytes identify it by offset alone.
    const LEN: u32 = 16;

    fn dims() -> Vec<(u32, u32)> {
        mip_dims(BASE, BASE)
    }

    fn index() -> TbdSatIndexV2 {
        let mut levels = Vec::new();
        let mut offset = 0u64;
        for &(lw, lh) in &dims() {
            let (w_tiles, h_tiles) = (
                lw.div_ceil(u32::from(TILE_PX)),
                lh.div_ceil(u32::from(TILE_PX)),
            );
            let tiles = (0..w_tiles * h_tiles)
                .map(|_| {
                    let t = SatTile {
                        offset,
                        len: LEN,
                        format: FORMAT_WEBP,
                    };
                    offset += u64::from(LEN);
                    t
                })
                .collect();
            levels.push(SatLevel {
                w_tiles,
                h_tiles,
                tiles,
            });
        }
        TbdSatIndexV2 {
            base_w: BASE,
            base_h: BASE,
            tile_px: TILE_PX,
            levels,
        }
    }

    /// `TbdsHeader` + rkyv index + one distinct byte per tile.
    fn frame(index: &TbdSatIndexV2) -> Vec<u8> {
        let bytes = to_bytes(index).expect("serialise");
        let tiles: usize = index.levels.iter().map(|l| l.tiles.len()).sum();
        let mut f = TbdsHeader::new(bytes.len() as u32)
            .to_header_bytes()
            .to_vec();
        f.extend_from_slice(&bytes);
        for i in 0..tiles {
            f.extend(std::iter::repeat_n(i as u8, LEN as usize));
        }
        f
    }

    /// The same pyramid as a v1 hand-packed table, offsets patched until the JSON length settles —
    /// exactly what `tbd-tools` does.
    fn v1() -> Vec<u8> {
        let mut json_len = 0usize;
        loop {
            let mut offset = 12 + json_len as u64;
            let mips: Vec<serde_json::Value> = dims()
                .iter()
                .enumerate()
                .map(|(level, &(lw, lh))| {
                    let (w_tiles, h_tiles) = (
                        lw.div_ceil(u32::from(TILE_PX)),
                        lh.div_ceil(u32::from(TILE_PX)),
                    );
                    let tiles: Vec<serde_json::Value> = (0..w_tiles * h_tiles)
                        .map(|i| {
                            let (x, y, w, h) = tile_rect(lw, lh, w_tiles, h_tiles, i);
                            let t = serde_json::json!({
                                "x": x, "y": y, "width": w, "height": h,
                                "offset": offset, "length": LEN,
                            });
                            offset += u64::from(LEN);
                            t
                        })
                        .collect();
                    serde_json::json!({"level": level, "width": lw, "height": lh, "tiles": tiles})
                })
                .collect();
            let json = serde_json::to_string(&serde_json::json!({
                "formatVersion": 1, "terrainId": "synthetic", "worldBounds": [0, 0, 5, 5],
                "baseWidthPx": BASE, "baseHeightPx": BASE, "mipCount": mips.len(), "mips": mips,
            }))
            .expect("json");
            if json.len() == json_len {
                let mut f = MAGIC.to_le_bytes().to_vec();
                f.extend_from_slice(&1u32.to_le_bytes());
                f.extend_from_slice(&(json_len as u32).to_le_bytes());
                f.extend_from_slice(json.as_bytes());
                let tiles: usize = dims()
                    .iter()
                    .map(|&(lw, lh)| {
                        (lw.div_ceil(u32::from(TILE_PX)) * lh.div_ceil(u32::from(TILE_PX))) as usize
                    })
                    .sum();
                for i in 0..tiles {
                    f.extend(std::iter::repeat_n(i as u8, LEN as usize));
                }
                return f;
            }
            json_len = json.len();
        }
    }

    fn rects(idx: &TbdSatIndex) -> Vec<(u32, u32, u32, u32, u32, u64)> {
        idx.mips
            .iter()
            .flat_map(|m| {
                m.tiles
                    .iter()
                    .map(|t| (m.level, t.x, t.y, t.width, t.height, t.length))
            })
            .collect()
    }

    /// The acceptance: one pyramid, two containers, the same tiles in the same places, and the
    /// same bytes behind every Range the caller will issue.
    #[test]
    fn v1_and_v2_of_one_pyramid_read_back_identical() {
        let (v1b, v2b) = (v1(), frame(&index()));
        let a = parse_tbd_sat_index_strict(&v1b, v1b.len() as u64).expect("v1 strict");
        let b = parse_tbd_sat_index_strict(&v2b, v2b.len() as u64).expect("v2 strict");
        assert_eq!((a.container_version, b.container_version), (1, 2));
        assert_eq!(
            (a.base_width_px, a.base_height_px, a.mip_count),
            (b.base_width_px, b.base_height_px, b.mip_count)
        );
        assert_eq!(rects(&a), rects(&b), "the two indexes describe one pyramid");
        assert_eq!(rects(&a).len(), 11, "9 + 1 + 1 tiles over three mips");
        for (ta, tb) in a
            .mips
            .iter()
            .flat_map(|m| &m.tiles)
            .zip(b.mips.iter().flat_map(|m| &m.tiles))
        {
            let (sa, sb) = (ta.offset as usize, tb.offset as usize);
            assert_eq!(
                &v1b[sa..sa + ta.length as usize],
                &v2b[sb..sb + tb.length as usize],
                "the Range each index points at must hold the same tile"
            );
        }
    }

    /// The committed everon ladder, which the loose and strict paths both have to keep reading.
    #[test]
    fn the_v1_path_is_untouched() {
        let v1b = v1();
        let loose = parse_tbd_sat_index_only(&v1b, v1b.len() as u64).expect("v1 loose");
        assert_eq!(loose.container_version, 1);
        assert_eq!(loose.terrain_id.as_deref(), Some("synthetic"));
        assert_eq!(loose.world_bounds, Some([0.0, 0.0, 5.0, 5.0]));
        assert_eq!(
            pick_base_level(&loose, 2),
            1,
            "5 px does not fit a 2 px limit"
        );
        assert_eq!(pick_preview_level(&loose, 1).width, 1);
    }

    /// A v2 index carries no `terrainId` / `worldBounds` (they live in `manifest.json`, spec §5) —
    /// `None`, never a fabricated default a later reader could mistake for real bounds.
    #[test]
    fn v2_reports_the_fields_it_does_not_carry_as_absent() {
        let f = frame(&index());
        let idx = parse_tbd_sat_index_strict(&f, f.len() as u64).expect("v2");
        assert_eq!((idx.terrain_id, idx.world_bounds), (None, None));
    }

    #[test]
    fn an_index_len_one_byte_short_is_an_error() {
        let mut f = frame(&index());
        let short = u32::from_le_bytes(f[8..12].try_into().unwrap()) - 1;
        f[8..12].copy_from_slice(&short.to_le_bytes());
        let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
        assert!(matches!(e, TbdSatError::Archive(_)), "{e}");
    }

    #[test]
    fn a_corrupt_index_is_an_error() {
        let mut f = frame(&index());
        let mid = HEADER_BYTES + 8;
        f[mid] ^= 0xff;
        assert!(parse_tbd_sat_index_strict(&f, f.len() as u64).is_err());
    }

    /// The check `access_checked` cannot make: the archive is perfectly readable and still means
    /// something this build must refuse.
    #[test]
    fn a_grid_that_contradicts_tile_px_is_rejected() {
        let mut i = index();
        i.levels[0].w_tiles = 1;
        i.levels[0].h_tiles = 1;
        i.levels[0].tiles.truncate(1);
        let f = frame(&i);
        let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
        assert!(format!("{e}").contains("level 0: grid 1x1"), "{e}");
    }

    #[test]
    fn an_unknown_tile_format_is_rejected() {
        let mut i = index();
        i.levels[2].tiles[0].format = 1;
        let f = frame(&i);
        let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
        assert!(format!("{e}").contains("format 1"), "{e}");
    }

    /// A Range body is a `Vec<u8>` at whatever alignment the allocator chose, and the index starts
    /// 32 bytes into it. Reading it in place would be a coin flip; the aligned copy is what makes
    /// this deterministic rather than machine-dependent.
    #[test]
    fn an_unaligned_buffer_still_validates() {
        let f = frame(&index());
        for pad in 1..8usize {
            let mut shifted = vec![0u8; pad];
            shifted.extend_from_slice(&f);
            let view = &shifted[pad..];
            parse_tbd_sat_index_strict(view, view.len() as u64)
                .unwrap_or_else(|e| panic!("pad {pad}: {e}"));
        }
    }

    #[test]
    fn a_third_container_version_is_refused_by_name() {
        let mut f = frame(&index());
        f[4..6].copy_from_slice(&3u16.to_le_bytes());
        let e = parse_tbd_sat_index_strict(&f, f.len() as u64).expect_err("must not validate");
        assert!(matches!(e, TbdSatError::UnsupportedVersion(3)), "{e}");
    }

    /// Committed-everon geometry, derived rather than stored: 14 levels from 12800², level 0 four
    /// 6400 px quadrants (**not** 8192 + 4608), level 1 one whole tile.
    #[test]
    fn the_derivation_reproduces_the_committed_everon_tiling() {
        let d = mip_dims(12_800, 12_800);
        assert_eq!(
            (d.len(), d[1], *d.last().expect("chain")),
            (14, (6_400, 6_400), (1, 1))
        );
        assert_eq!(12_800u32.div_ceil(8_192), 2);
        assert_eq!(
            (0..4)
                .map(|i| tile_rect(12_800, 12_800, 2, 2, i))
                .collect::<Vec<_>>(),
            vec![
                (0, 0, 6_400, 6_400),
                (6_400, 0, 6_400, 6_400),
                (0, 6_400, 6_400, 6_400),
                (6_400, 6_400, 6_400, 6_400),
            ]
        );
        assert_eq!(tile_rect(6_400, 6_400, 1, 1, 0), (0, 0, 6_400, 6_400));
    }
}
