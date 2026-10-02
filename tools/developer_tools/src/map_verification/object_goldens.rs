//! The semantic golden gate S2–S9 + S11–S15, run as `cargo xtask schema map-object-golden`
//! over `contracts/fixtures/map/`. Shape validation (S1) lives in
//! `schema validate`; enum drift (S10) in `schema map-object-enums`. Uses the shared
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
use map_engine::streaming::loaders::chunk::parse_chunk;
use map_engine::streaming::loaders::chunk_bin::parse_chunk_bin_for;
use map_engine::world::environment::buildings::prefab::build_prefab_maps;
use map_engine::world::environment::buildings::prefab::narrow_prefab_rows;
use world_file_formats::containers::header::CONTAINER_VERSION;
use world_file_formats::containers::header::HEADER_BYTES;
use world_file_formats::pod::instance::POD_BYTES;

mod spatial_invariants;

struct Gate {
    id: &'static str,
    label: &'static str,
    errs: Vec<String>,
}

/* ───────── S15 — the TBDC binary twin of the chunk golden (spec §2 / §3.1) ───────── */

#[path = "object_goldens/read_json.rs"]
mod read_json;
use read_json::chunk_bin_errors;
use read_json::inst_id;
use read_json::inst_prefab_id;
use read_json::read_json;

#[path = "object_goldens/map_object_golden.rs"]
mod map_object_golden;
pub use map_object_golden::map_object_golden;
