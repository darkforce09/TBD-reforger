//! T-165.4 — the semantic golden gate S2–S9 + S11–S15 (port of
//! `packages/tbd-schema/scripts/verify-map-object-golden.mjs`). Shape validation (S1) lives in
//! `schema validate`; enum drift (S10) in `schema map-object-enums`. Uses the shared tbd-tools
//! compute libs (geometry/density/forest) — the same code the world builder + phase gates run.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::repository_layout::contracts_dir;
use serde_json::{Value, json};

use crate::world_export_pipeline::binary_emit::{
    class_code_table, pods_from_rows, write_chunk_bin,
};
use website_map_engine::io::containers::header::CONTAINER_VERSION;
use website_map_engine::io::containers::header::HEADER_BYTES;
use website_map_engine::io::pod::instance::POD_BYTES;
use website_map_engine::streaming::loaders::chunk::parse_chunk;
use website_map_engine::streaming::loaders::chunk_bin::parse_chunk_bin_for;
use website_map_engine::world::environment::buildings::prefab::build_prefab_maps;
use website_map_engine::world::environment::buildings::prefab::narrow_prefab_rows;

mod spatial_invariants;

struct Gate {
    id: &'static str,
    label: &'static str,
    errs: Vec<String>,
}

/* ───────── S15 — the TBDC binary twin of the chunk golden (T-935.12, spec §2 / §3.1) ───────── */

#[path = "object_goldens/read_json.rs"]
mod read_json;
use read_json::chunk_bin_errors;
use read_json::inst_id;
use read_json::inst_prefab_id;
use read_json::read_json;

#[path = "object_goldens/map_object_golden.rs"]
mod map_object_golden;
pub use map_object_golden::map_object_golden;
