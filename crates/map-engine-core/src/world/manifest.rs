//! Terrain-manifest `objects` block + chunk-index cells — ports of the `doLoadManifest`
//! `ObjectsBlock` read (`worldObjectsCore.ts:487`) and `narrowCells` (`:361`).

use serde_json::Value;

/// Default chunk edge in meters when the manifest omits `chunkSizeM` (`DEFAULT_CHUNK_SIZE_M`).
pub const DEFAULT_CHUNK_SIZE_M: f64 = 512.0;

/// The manifest `objects` fields this parser consumes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectsManifest {
    pub prefabs_path: String,
    pub chunks_path: String,
    pub chunk_size_m: f64,
    pub roads_path: Option<String>,
    pub density_path: Option<String>,
    pub regions_path: Option<String>,
    pub instance_count: Option<f64>,
    pub prefab_count: Option<f64>,
}

/// One chunk-index cell (mirror of `WorldChunkCell`, `narrowCells`).
#[derive(Clone, Debug, PartialEq)]
pub struct ChunkCell {
    pub id: String,
    pub cx: f64,
    pub cy: f64,
    pub path: String,
    pub instance_count: Option<f64>,
}

/// Parse the manifest `objects` block. Returns `None` when the block is absent or is missing
/// `prefabsPath`/`chunksPath` (the `doLoadManifest` v2-export gate).
#[must_use]
pub fn parse_objects_manifest(raw: &Value) -> Option<ObjectsManifest> {
    let objects = raw.get("objects")?;
    let string_field = |k: &str| objects.get(k).and_then(Value::as_str).map(str::to_string);
    let prefabs_path = string_field("prefabsPath")?;
    let chunks_path = string_field("chunksPath")?;
    Some(ObjectsManifest {
        prefabs_path,
        chunks_path,
        chunk_size_m: objects
            .get("chunkSizeM")
            .and_then(Value::as_f64)
            .unwrap_or(DEFAULT_CHUNK_SIZE_M),
        roads_path: string_field("roadsPath"),
        density_path: string_field("densityPath"),
        regions_path: string_field("regionsPath"),
        instance_count: objects.get("instanceCount").and_then(Value::as_f64),
        prefab_count: objects.get("prefabCount").and_then(Value::as_f64),
    })
}

/// `narrowCells(indexRaw)` (`:361`) — the chunk-index `cells[]`. `None` when `cells` is not an
/// array (full-grid sweep mode). Keeps rows with numeric `cx`/`cy` and string `path`.
#[must_use]
pub fn narrow_cells(index_raw: &Value) -> Option<Vec<ChunkCell>> {
    let raw_cells = index_raw.get("cells")?.as_array()?;
    let mut cells = Vec::with_capacity(raw_cells.len());
    for c in raw_cells {
        let (Some(cx), Some(cy), Some(path)) = (
            c.get("cx").and_then(Value::as_f64),
            c.get("cy").and_then(Value::as_f64),
            c.get("path").and_then(Value::as_str),
        ) else {
            continue;
        };
        cells.push(ChunkCell {
            id: format!("{}_{}", cx as i64, cy as i64),
            cx,
            cy,
            path: path.to_string(),
            instance_count: c.get("instanceCount").and_then(Value::as_f64),
        });
    }
    Some(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_objects_block_with_defaults() {
        let raw = json!({ "objects": {
            "prefabsPath": "objects/prefabs.json.gz",
            "chunksPath": "objects/chunks",
            "roadsPath": "objects/roads.json.gz",
            "regionsPath": "objects/forest-regions.json.gz",
            "densityPath": "objects/density",
            "prefabCount": 391,
            "instanceCount": 508291
        }});
        let m = parse_objects_manifest(&raw).unwrap();
        assert_eq!(m.chunk_size_m, DEFAULT_CHUNK_SIZE_M); // omitted → 512
        assert_eq!(m.prefab_count, Some(391.0));
        assert_eq!(m.instance_count, Some(508291.0));
        assert_eq!(m.roads_path.as_deref(), Some("objects/roads.json.gz"));
    }

    #[test]
    fn gate_requires_prefabs_and_chunks_paths() {
        assert!(parse_objects_manifest(&json!({ "objects": { "prefabsPath": "p" } })).is_none());
        assert!(parse_objects_manifest(&json!({})).is_none());
    }

    #[test]
    fn narrow_cells_reads_index() {
        let raw = json!({ "cells": [
            { "cx": 10, "cy": 12, "path": "objects/chunks/10_12.json.gz", "instanceCount": 42 },
            { "cx": "x", "cy": 1, "path": "p" }   // non-numeric cx → dropped
        ]});
        let cells = narrow_cells(&raw).unwrap();
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].id, "10_12");
        assert_eq!(cells[0].instance_count, Some(42.0));
        assert!(narrow_cells(&json!({})).is_none()); // no cells → sweep mode
    }
}
