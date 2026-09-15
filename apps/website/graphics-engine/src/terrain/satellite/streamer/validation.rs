//! Role: validation.
//! Position: `terrain/satellite/streamer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::TbdSatError;
use super::TbdSatIndex;
use super::TbdSatMip;
use super::parse_header;

/// Validate mip tiles.
pub(super) fn validate_mip_tiles(
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

/// Parse tbd sat index only.
pub fn parse_tbd_sat_index_only(buf: &[u8], file_size: u64) -> Result<TbdSatIndex, TbdSatError> {
    let (index, payload_start) = parse_header(buf, file_size)?;
    validate_index(&index, payload_start, file_size, false)?;
    Ok(index)
}

/// Parse tbd sat index strict.
pub fn parse_tbd_sat_index_strict(buf: &[u8], file_size: u64) -> Result<TbdSatIndex, TbdSatError> {
    let (index, payload_start) = parse_header(buf, file_size)?;
    validate_index(&index, payload_start, file_size, true)?;
    Ok(index)
}

/// Validate index.
pub(super) fn validate_index(
    index: &TbdSatIndex,
    payload_start: u64,
    file_size: u64,
    full_coverage: bool,
) -> Result<(), TbdSatError> {
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
