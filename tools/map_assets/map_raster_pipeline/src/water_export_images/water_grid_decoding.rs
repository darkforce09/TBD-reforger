//! The digit-run reader of the Workbench water export's ASCII grids.
//!
//! **Role:** [`for_each_digit_run`] streams a grid file's bytes and hands every run of decimal
//! digits to the caller as `(sample index, value)`; [`decode_mask_grid`] and
//! [`decode_depth_grid`] store those values into the water class mask and the depth grid.
//! **Position:** called by the lane's `run` when the image grid is the export's own grid and both
//! `bathymetry_mask.txt` and `bathymetry_depth.txt` (or their `TBD_WaterExport_` names) are
//! present; the files run to hundreds of megabytes, so they are read through one large buffer,
//! never whole.
//! **Signals & state:** none held; each call owns its reader.
//! **Invariants:** a digit extends the pending number (`value × 10 + digit`); any other byte ends
//! a pending number, which is stored at the next index while the index is below the sample count
//! and dropped once it is not, so a file with too many numbers fills the grid and stops storing
//! and a file with too few leaves the rest at zero; a number pending at the end of the file is
//! stored the same way; nothing else is checked, so `-5` reads as 5, `1.5` as 1 then 5, and any
//! separator or line ending (`\n`, `\r\n`) works; the value accumulates with wrapping arithmetic
//! and is stored modulo 256 in the mask and modulo 65536 in the depth grid (both divide 2⁶⁴, so
//! the wrap never changes the stored value).

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use crate::error::{Result, ResultExt};

/// The read buffer of a grid file: large enough that a 500 MB grid takes a few hundred reads.
const GRID_READ_BUFFER_BYTES: usize = 4 << 20;

/// Calls `store(index, value)` for each run of decimal digits `reader` yields, in order, while
/// `index` is below `sample_count`; returns how many runs were stored.
pub(super) fn for_each_digit_run(
    reader: impl Read,
    sample_count: usize,
    mut store: impl FnMut(usize, u64),
) -> std::io::Result<usize> {
    let mut reader = BufReader::with_capacity(GRID_READ_BUFFER_BYTES, reader);
    let mut stored = 0_usize;
    let mut value = 0_u64;
    let mut in_number = false;
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            break;
        }
        for &byte in chunk {
            if byte.is_ascii_digit() {
                value = value.wrapping_mul(10).wrapping_add(u64::from(byte - b'0'));
                in_number = true;
            } else if in_number {
                if stored < sample_count {
                    store(stored, value);
                    stored += 1;
                }
                value = 0;
                in_number = false;
            }
        }
        let consumed = chunk.len();
        reader.consume(consumed);
    }
    if in_number && stored < sample_count {
        store(stored, value);
        stored += 1;
    }
    Ok(stored)
}

/// Reads the water class mask grid at `path` into `mask`, each value modulo 256; returns how many
/// samples were stored.
pub(super) fn decode_mask_grid(path: &Path, mask: &mut [u8]) -> Result<usize> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let sample_count = mask.len();
    for_each_digit_run(file, sample_count, |index, value| {
        // The low byte: the value modulo 256.
        mask[index] = value as u8;
    })
    .with_context(|| format!("read {}", path.display()))
}

/// Reads the depth grid (decimetres) at `path` into `depth_decimetres`, each value modulo 65536;
/// returns how many samples were stored.
pub(super) fn decode_depth_grid(path: &Path, depth_decimetres: &mut [u16]) -> Result<usize> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let sample_count = depth_decimetres.len();
    for_each_digit_run(file, sample_count, |index, value| {
        // The low two bytes: the value modulo 65536.
        depth_decimetres[index] = value as u16;
    })
    .with_context(|| format!("read {}", path.display()))
}

#[cfg(test)]
#[path = "../tests/water_export_images_grid_decoding_tests.rs"]
mod tests;
