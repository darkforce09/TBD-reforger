//! The map-object semantic golden gate, S2 to S9 and S11 to S15.
//!
//! **Role:** `cargo xtask schema map-object-golden` over `contracts/fixtures/map/`: the
//! enum coverage, prefab, instance, region, road and resolved samples (S2 to S9), the chunk
//! placement, anchor, density and forest-region invariants (S11 to S14) and the binary twin of the
//! chunk golden (S15). Shape validation (S1) is `cargo xtask schema validate`; enum drift (S10) is
//! `cargo xtask schema map-object-enums`.
//! **Position:** called through the CI task catalogue's map asset checks; runs the world export
//! pipeline's emitters and geometry, the code that writes the served assets.
//! **Signals & state:** none; the S15 re-emit writes one temporary file and removes it.
//! **Invariants:** every gate prints `PASS` or `FAIL` with up to eight errors; a missing or
//! unreadable S15 golden is a failure, never a skip.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result, ResultExt, refusal};

use ::repository_layout::contracts_dir;
use serde_json::{Value, json};

use prefab_catalog::prefab_rows::build_prefab_maps;
use prefab_catalog::prefab_rows::narrow_prefab_rows;
use world_chunks::chunk_container::parse_chunk_bin_for;
use world_chunks::world_chunk::parse_chunk;
use world_export_pipeline::binary_emit::{class_code_table, pods_from_rows, write_chunk_bin};
use world_file_formats::containers::header::CONTAINER_VERSION;
use world_file_formats::containers::header::HEADER_BYTES;
use world_file_formats::pod::instance::POD_BYTES;

mod spatial_invariants;

/// One numbered gate's verdict: its id, its label and the errors it found.
struct Gate {
    id: &'static str,
    label: &'static str,
    errs: Vec<String>,
}

mod golden_row_readers;
use golden_row_readers::chunk_bin_errors;
use golden_row_readers::instance_id;
use golden_row_readers::instance_prefab_id;
use golden_row_readers::read_json_file;

mod map_object_golden;
pub use map_object_golden::map_object_golden;
