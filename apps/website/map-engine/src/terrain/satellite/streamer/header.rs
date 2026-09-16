//! Role: header.
//! Position: `terrain/satellite/streamer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::AlignedIndex;
use super::MAGIC;
use super::TbdSatError;
use super::TbdSatIndex;
use super::TbdSatIndexV2;
use super::TbdsHeader;
use super::V1;
use super::V2;
use super::access_checked;
use super::mips_from_archive;
use crate::formats::containers::header::ContainerHeader;

/// Read u32 le.
pub(super) fn read_u32_le(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

/// Inclusive Range end of header+index, sized from a 12-byte prefix.
pub fn index_range_end(prefix: &[u8]) -> Result<u64, TbdSatError> {
    if prefix.len() < 12 {
        return Err(TbdSatError::TooSmall);
    }
    let magic = read_u32_le(prefix, 0).ok_or(TbdSatError::TooSmall)?;
    if magic != MAGIC {
        return Err(TbdSatError::BadMagic);
    }
    match u16::from_le_bytes([prefix[4], prefix[5]]) {
        V1 => {
            let json_len = read_u32_le(prefix, 8).ok_or(TbdSatError::TooSmall)? as u64;
            if json_len == 0 || json_len > 16 * 1024 * 1024 {
                return Err(TbdSatError::JsonOverrun);
            }
            Ok(11 + json_len)
        }
        V2 => {
            let index_len = read_u32_le(prefix, 8).ok_or(TbdSatError::TooSmall)? as u64;
            if index_len == 0 || index_len > 16 * 1024 * 1024 {
                return Err(TbdSatError::JsonOverrun);
            }
            Ok(31 + index_len)
        }
        v => Err(TbdSatError::UnsupportedVersion(u32::from(v))),
    }
}

/// Parse header + index; `file_size` is the full-file size (a Range head may be shorter).
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

/// Parse header v1.
pub(super) fn parse_header_v1(
    buf: &[u8],
    file_size: u64,
) -> Result<(TbdSatIndex, u64), TbdSatError> {
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

/// Parse header v2.
pub(super) fn parse_header_v2(
    buf: &[u8],
    file_size: u64,
) -> Result<(TbdSatIndex, u64), TbdSatError> {
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
