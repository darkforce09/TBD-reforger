//! The binary twin of every `objects/chunks/{cx}_{cy}.json.gz`.
//!
//! `build-objects` writes each chunk twice: the gzip-9 JSON the shipped loader still reads, and a
//! `TBDC`-framed `{cx}_{cy}.bin` of raw `ObjectInstancePod` rows. Dual emission is deliberate and
//! stays until the terrain manifest names only the binary path — deleting either write before
//! then blinds one reader.
//!
//! # Why the emitter narrows the JSON rows instead of the builder's own `f64`s
//!
//! The obvious emitter is `pod.x = chunk_row.x as f32`, straight off the in-memory row. It is
//! wrong, and the parity test is what proves it: the JSON round trip is not the identity on the
//! values this pipeline holds.
//!
//! * **Negative zero.** `json_number_formatting::js_num(-0.0)` takes the integral branch and prints `0`, so the
//!   loader reads `+0.0` (f32 bits `0x0000_0000`) where the builder held `-0.0` (bits
//!   `0x8000_0000`). Bit-equality would fail on a value both sides call "zero".
//! * **Five-wide rows.** `chunk_row_values` drops
//!   `pitch`/`roll`/`scale` whenever `trailers_trivial` holds, and
//!   the loader then *defaults* them (`0`, `0`, `1`). A `-0.0` roll is trivial by `==` but is not
//!   the `+0.0` the loader will produce.
//! * **Non-positive scale.** `narrow_instance_row_v2` clamps a `<= 0.0` scale to `1.0`; the raw
//!   row does not.
//!
//! So the emitter reads the *same* `Value` rows that are about to be gzipped, through the *same*
//! `narrow_instance_row_v2` the loader uses. The `.bin` is then equal to the `.json.gz` decode by
//! construction rather than by coincidence, including the skip rule: a row the narrow rejects is
//! dropped with no gap in either file, so row `i` means the same instance in both.
//!
//! # Class codes
//!
//! `class_code` is the one column the JSON chunk does not carry — the loader derives it from
//! `prefabs.json.gz`. `class_code_table` derives it from the very document that is about to be
//! written, through `narrow_prefab_rows` + `build_prefab_maps`, i.e. the loader's own chain. The
//! parity test therefore pins the *indexing* (row `i` gets prefab `i`'s code), not the taxonomy —
//! `classify.rs`'s own tests own that.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use website_map_engine::io::containers::header::ContainerHeader;
use website_map_engine::io::containers::tbdc::TbdcHeader;
use website_map_engine::io::pod::instance::ObjectInstancePod;
use website_map_engine::io::pod::instance::instances_to_bytes;
use website_map_engine::world::environment::buildings::prefab::build_prefab_maps;
use website_map_engine::world::environment::buildings::prefab::narrow_prefab_rows;
use website_map_engine::world::environment::classify::NO_CLASS;
use website_map_engine::world::environment::classify::narrow_instance_row_v2;

/// Prefab id (`pid.to_bits()`, the loader's key) → render-class code, for the prefab catalogue
/// document `build-objects` is about to write.
///
/// Keyed by the f64 bit pattern rather than an index so it matches
/// `build_prefab_maps` exactly: a chunk row's `pid`
/// is an f64 and a `Vec` index would silently truncate a non-integral or out-of-range one.
#[must_use]
pub fn class_code_table(prefabs_doc: &Value) -> HashMap<u64, u8> {
    let (by_id, _has_oversized) = build_prefab_maps(narrow_prefab_rows(prefabs_doc));
    by_id.into_iter().map(|(k, e)| (k, e.code)).collect()
}

/// Narrow the chunk's JSON rows to wire rows, in order, skipping exactly what the loader skips.
///
/// `class_by_pid` comes from `class_code_table`; an unknown prefab takes
/// `NO_CLASS`, which is what `parse_chunk` stores.
#[must_use]
pub fn pods_from_rows(rows: &[Value], class_by_pid: &HashMap<u64, u8>) -> Vec<ObjectInstancePod> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(r) = narrow_instance_row_v2(row) else {
            continue;
        };
        out.push(ObjectInstancePod {
            x: r.x as f32,
            y: r.y as f32,
            z: r.z as f32,
            yaw: r.rot as f32,
            pitch: r.pitch as f32,
            roll: r.roll as f32,
            scale: r.scale as f32,
            // `pid as u16` is the loader's `prefab_idx` store verbatim (JS `Uint16Array`); everon
            // pids top out at 1623, three orders of magnitude below the wrap.
            prefab_id: r.pid as u16,
            class_code: class_by_pid
                .get(&r.pid.to_bits())
                .copied()
                .unwrap_or(NO_CLASS),
            _pad: 0,
        });
    }
    out
}

/// Write `objects/chunks/{cx}_{cy}.bin`: a 32-byte `TBDC` header then `rows.len()` 32-byte
/// `ObjectInstancePod`s, little-endian and uncompressed. File length is exactly
/// `32 + 32 * rows.len()`.
///
/// # Errors
/// When `cx`/`cy` do not fit the header's `i16`, when the row count does not fit its `u32`, or when
/// the write fails. The two range checks are `try_from`, not `as`: a lossy cast would stamp a
/// wrapped chunk index into a file whose *name* still carried the right one, and every later reader
/// would trust the header.
pub fn write_chunk_bin(path: &Path, cx: i64, cy: i64, rows: &[ObjectInstancePod]) -> Result<()> {
    let cx16 = i16::try_from(cx).with_context(|| format!("chunk cx {cx} does not fit TBDC i16"))?;
    let cy16 = i16::try_from(cy).with_context(|| format!("chunk cy {cy} does not fit TBDC i16"))?;
    let count = u32::try_from(rows.len())
        .with_context(|| format!("chunk {cx}_{cy} has {} rows, over u32", rows.len()))?;
    let header = TbdcHeader::new(cx16, cy16, count);
    let mut buf = Vec::with_capacity(header.file_bytes().unwrap_or(0));
    buf.extend_from_slice(&header.to_header_bytes());
    buf.extend_from_slice(instances_to_bytes(rows));
    std::fs::write(path, &buf).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/binary_emit/tests.rs"]
mod tests;
