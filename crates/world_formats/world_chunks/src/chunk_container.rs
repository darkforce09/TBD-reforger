//! The `TBDC` binary chunk container: decode, identity check and file path.
//!
//! **Role:** decodes a `TBDC` container into the same [`WorldChunk`] columns the JSON lane builds
//! ([`parse_chunk_bin`]), checks that the bytes are the chunk that was requested
//! ([`parse_chunk_bin_for`]), and fills the manifest's chunk path template ([`chunk_bin_path`]).
//! **Position:** under `world_chunks`, over `world_file_formats`' container header and object rows;
//! read by the map engine's chunk ingest and world loader and by the developer tools'
//! verification.
//! **Signals & state:** none; pure functions over byte slices.
//! **Invariants:** a payload whose length disagrees with its header is refused; a misaligned
//! payload is copied into aligned words, never read unaligned; the chunk identifier is the
//! header's `cx_cy`.

use std::collections::HashMap;

use crate::chunk_id::ChunkId;
use crate::world_chunk::WorldChunk;
use prefab_catalog::render_classes::NO_CLASS;
use world_file_formats::archives::codec::BinaryError;
use world_file_formats::containers::header::ContainerHeader;
use world_file_formats::containers::tbdc::TbdcHeader;
use world_file_formats::pod::instance::ObjectInstancePod;
use world_file_formats::pod::instance::instances_from_bytes;

const CX_PLACEHOLDER: &str = "{cx}";

const CY_PLACEHOLDER: &str = "{cy}";

/// Everything that can be wrong with a chunk `.bin` *as a chunk*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkBinError {
    /// The buffer is not a well-formed `TBDC` container.
    Format(BinaryError),

    /// Well-formed, but it is a different tile than the one requested.
    IdMismatch {
        /// The chunk the caller asked for.
        requested: ChunkId,
        /// The chunk the container header names.
        header: ChunkId,
    },
}

impl std::fmt::Display for ChunkBinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Format(e) => write!(f, "{e}"),
            Self::IdMismatch { requested, header } => write!(
                f,
                "TBDC: chunk {requested} was fetched but the header declares {header}"
            ),
        }
    }
}

impl core::error::Error for ChunkBinError {}

impl From<BinaryError> for ChunkBinError {
    fn from(e: BinaryError) -> Self {
        Self::Format(e)
    }
}

/// Parse one `TBDC` chunk container into the SoA [`WorldChunk`] the residency stores.
pub fn parse_chunk_bin(bytes: &[u8]) -> Result<WorldChunk, BinaryError> {
    let (header, payload) = TbdcHeader::read(bytes)?;
    let Some(expected) = header.payload_bytes() else {
        return Err(BinaryError::LengthMismatch {
            what: TbdcHeader::NAME,
            expected: usize::MAX,
            actual: payload.len(),
        });
    };
    if payload.len() != expected {
        return Err(BinaryError::LengthMismatch {
            what: TbdcHeader::NAME,
            expected,
            actual: payload.len(),
        });
    }
    match instances_from_bytes(payload) {
        Ok(rows) => Ok(columns(&header, rows)),
        Err(BinaryError::Misaligned { .. }) => {
            let words = align4(payload);
            let rows = instances_from_bytes(bytemuck::cast_slice(&words))?;
            Ok(columns(&header, rows))
        }
        Err(other) => Err(other),
    }
}

/// [`parse_chunk_bin`] plus the check that the bytes are the chunk that was actually requested.
pub fn parse_chunk_bin_for(id: &ChunkId, bytes: &[u8]) -> Result<WorldChunk, ChunkBinError> {
    let chunk = parse_chunk_bin(bytes)?;
    if chunk.id != *id {
        return Err(ChunkBinError::IdMismatch {
            requested: id.clone(),
            header: chunk.id,
        });
    }
    Ok(chunk)
}

/// Fill the manifest's `objects.binary.chunks` template (`objects/chunks/{cx}_{cy}.bin`) for one chunk id, relative to the terrain asset base.
#[must_use]
pub fn chunk_bin_path(template: &str, id: &ChunkId) -> Option<String> {
    if !template.contains(CX_PLACEHOLDER) || !template.contains(CY_PLACEHOLDER) {
        return None;
    }
    let (cx, cy) = id.as_str().split_once('_')?;

    cx.parse::<i16>().ok()?;
    cy.parse::<i16>().ok()?;
    Some(
        template
            .replace(CX_PLACEHOLDER, cx)
            .replace(CY_PLACEHOLDER, cy),
    )
}

fn align4(payload: &[u8]) -> Vec<u32> {
    let mut words = vec![0_u32; payload.len() / 4];
    let dst: &mut [u8] = bytemuck::cast_slice_mut(&mut words);
    dst.copy_from_slice(payload);
    words
}

fn columns(header: &TbdcHeader, rows: &[ObjectInstancePod]) -> WorldChunk {
    let n = rows.len();
    let mut positions: Vec<f32> = Vec::with_capacity(2 * n);
    let mut prefab_idx: Vec<u16> = Vec::with_capacity(n);
    let mut rotations: Vec<f32> = Vec::with_capacity(n);
    let mut z: Vec<f32> = Vec::with_capacity(n);
    let mut pitch: Vec<f32> = Vec::with_capacity(n);
    let mut roll: Vec<f32> = Vec::with_capacity(n);
    let mut scale: Vec<f32> = Vec::with_capacity(n);
    let mut cls_codes: Vec<u8> = Vec::with_capacity(n);
    let mut rows_by_class: HashMap<u8, Vec<u32>> = HashMap::new();

    for (i, r) in rows.iter().enumerate() {
        positions.push(r.x);
        positions.push(r.y);
        prefab_idx.push(r.prefab_id.get());
        rotations.push(r.yaw);
        z.push(r.z);
        pitch.push(r.pitch);
        roll.push(r.roll);
        scale.push(r.scale);
        cls_codes.push(r.class_code);
        if r.class_code != NO_CLASS {
            rows_by_class
                .entry(r.class_code)
                .or_default()
                .push(i as u32);
        }
    }

    WorldChunk {
        id: ChunkId::of_cell(i64::from(header.cx), i64::from(header.cy)),
        cx: f64::from(header.cx),
        cy: f64::from(header.cy),

        count: header.count,
        positions,
        prefab_idx,
        rotations,
        z,
        pitch,
        roll,
        scale,
        cls_codes,
        rows_by_class,
    }
}

#[cfg(test)]
#[path = "tests/chunk_container_tests.rs"]
mod tests;
