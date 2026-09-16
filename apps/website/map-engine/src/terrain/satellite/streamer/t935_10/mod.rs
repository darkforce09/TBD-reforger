//! Role: Module boundary for terrain/satellite/streamer/t935_10.
//! Position: `terrain/satellite/streamer/t935_10` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::formats::containers::header::ContainerHeader;

use crate::terrain::satellite::streamer::archive::mip_dims;

use crate::terrain::satellite::streamer::archive::tile_rect;

use super::*;

use crate::formats::archives::satellite::SatLevel;

use crate::formats::archives::satellite::SatTile;

use crate::formats::archives::satellite::TbdSatIndexV2;

use crate::formats::containers::tbds::TbdsHeader;

use crate::formats::containers::header::HEADER_BYTES;

use crate::formats::archives::codec::to_bytes;

const BASE: u32 = 5;

const TILE_PX: u16 = 2;

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

mod cases_1;
