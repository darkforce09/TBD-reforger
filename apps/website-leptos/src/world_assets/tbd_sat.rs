//! T-166 — TBDS (tbd-sat v1) container parse. Port of React `satelliteUnified.ts` (tag T-159.29.2).
//! The engine never parses TBDS (T-151.1 L2); the Leptos host owns fetch + structural validate.

use serde::Deserialize;

const MAGIC: u32 = 0x5344_4254; // "TBDS" LE

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
    pub format_version: u32,
    // Present in the `.tbd-sat` index header; retained to document the on-disk schema even though
    // the host keys off world size / mips, not these fields.
    #[allow(dead_code)]
    pub terrain_id: String,
    #[allow(dead_code)]
    pub world_bounds: [f64; 4],
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
        }
    }
}

fn read_u32_le(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

/// Parse header + JSON index; `file_size` is the full-file size (Range head may be shorter).
pub fn parse_header(buf: &[u8], file_size: u64) -> Result<(TbdSatIndex, u64), TbdSatError> {
    if buf.len() < 12 {
        return Err(TbdSatError::TooSmall);
    }
    let magic = read_u32_le(buf, 0).ok_or(TbdSatError::TooSmall)?;
    if magic != MAGIC {
        return Err(TbdSatError::BadMagic);
    }
    let version = read_u32_le(buf, 4).ok_or(TbdSatError::TooSmall)?;
    if version != 1 {
        return Err(TbdSatError::UnsupportedVersion(version));
    }
    let json_len = read_u32_le(buf, 8).ok_or(TbdSatError::TooSmall)? as u64;
    if 12 + json_len > buf.len() as u64 {
        return Err(TbdSatError::JsonOverrun);
    }
    let index: TbdSatIndex = serde_json::from_slice(&buf[12..12 + json_len as usize])
        .map_err(|e| TbdSatError::Json(e.to_string()))?;
    let payload_start = 12 + json_len;
    if payload_start > file_size {
        return Err(TbdSatError::JsonOverrun);
    }
    Ok((index, payload_start))
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

/// Full-buffer structural parse (React `parseTbdSat`).
pub fn parse_tbd_sat(buf: &[u8]) -> Result<TbdSatIndex, TbdSatError> {
    let (index, payload_start) = parse_header(buf, buf.len() as u64)?;
    validate_index(&index, payload_start, buf.len() as u64, true)?;
    Ok(index)
}

/// Index-only parse for Range preview (React `parseTbdSatIndexOnly`).
pub fn parse_tbd_sat_index_only(buf: &[u8], file_size: u64) -> Result<TbdSatIndex, TbdSatError> {
    let (index, payload_start) = parse_header(buf, file_size)?;
    validate_index(&index, payload_start, file_size, false)?;
    Ok(index)
}

fn validate_index(
    index: &TbdSatIndex,
    payload_start: u64,
    file_size: u64,
    full_coverage: bool,
) -> Result<(), TbdSatError> {
    if index.format_version != 1 {
        return Err(TbdSatError::Structure(format!(
            "index formatVersion {} !== 1",
            index.format_version
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

/// Coarsest-usable preview mip (long edge ≤ `max_edge_px`).
pub fn pick_preview_level(index: &TbdSatIndex, max_edge_px: u32) -> &TbdSatMip {
    for mip in &index.mips {
        if mip.width.max(mip.height) <= max_edge_px {
            return mip;
        }
    }
    index.mips.last().unwrap()
}
