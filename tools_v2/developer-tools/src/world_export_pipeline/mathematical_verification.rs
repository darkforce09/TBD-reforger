//! T-165.8 — the mathematical phase gate (port of `scripts/map-assets/verify-phase.mjs`):
//! G1-G12 global invariants + P1-*/PH-P2-* phase gates + D/F density-forest gates + E6/G4/I6
//! determinism on the STAGED raw export and the COMMITTED objects/ artifacts. Phases are
//! CUMULATIVE — phase-scoped gates filter committed rows to the requested phase's kinds while
//! catalog-scope gates run on the whole committed set; E6 rebuilds at the COMMITTED
//! importPhaseMax (in-process double scratch build — the Node script spawned itself).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use super::chunk_partitioner::{
    CHUNK_SIZE_M, build_roads_from_topo_opt, build_world_objects_opt, gunzip, phase_kinds,
};
use super::classify::{Classifier, load_rules, stream_raw_entities};
use super::json_number_formatting::round2;
use crate::browser_testing::server::repo_root;
use crate::repository_layout::contract_definitions_dir;
use crate::world_export_pipeline::forest_contours::{self as forest, Tree, derive_forest_regions};
use crate::world_export_pipeline::polygon_geometry as geometry;
use crate::world_export_pipeline::polygon_geometry::{cell_of, check_anchors, chunk_key};
use crate::world_export_pipeline::vegetation_density as density;

const MAX_CHUNK_AGGREGATE_BYTES: u64 = 40 * 1024 * 1024;

pub struct SchemaSet {
    registry: jsonschema::Registry<'static>,
    schemas: HashMap<&'static str, Value>,
}

pub const MAP_OBJECT_SCHEMAS: [&str; 9] = [
    "map-object-enums",
    "map-object-prefab",
    "map-object-instance",
    "map-object-region",
    "map-object-roads",
    "map-object-catalog",
    "map-object-resolved",
    "map-object-type-inventory",
    "terrain-registry",
];

impl SchemaSet {
    pub fn load() -> Result<SchemaSet> {
        let dir = contract_definitions_dir(&repo_root());
        let mut registered: Vec<(String, Value)> = Vec::new();
        let mut schemas = HashMap::new();
        for name in MAP_OBJECT_SCHEMAS {
            let doc: Value = serde_json::from_str(
                &std::fs::read_to_string(dir.join(format!("{name}.schema.json")))
                    .with_context(|| name.to_string())?,
            )?;
            let id = doc["$id"].as_str().unwrap_or_default().to_string();
            registered.push((id, doc.clone()));
            schemas.insert(name, doc);
        }
        let registry = jsonschema::Registry::new()
            .extend(
                registered
                    .into_iter()
                    .map(|(id, doc)| (id, jsonschema::Resource::from_contents(doc))),
            )
            .map_err(|e| anyhow::anyhow!("registry: {e}"))?
            .prepare()
            .map_err(|e| anyhow::anyhow!("registry prepare: {e}"))?;
        Ok(SchemaSet { registry, schemas })
    }

    pub fn validator(&self, name: &str) -> Result<jsonschema::Validator> {
        let doc = self
            .schemas
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("unknown schema {name}"))?;
        jsonschema::options()
            .with_registry(&self.registry)
            .build(doc)
            .map_err(|e| anyhow::anyhow!("compile {name}: {e}"))
    }
}

struct Gate {
    id: String,
    label: String,
    errs: Vec<String>,
    err_count: usize,
}

#[derive(Default)]
struct Gates(Vec<Gate>);

impl Gates {
    fn gate(&mut self, id: &str, label: &str, errs: Vec<String>) {
        self.0.push(Gate {
            id: id.to_string(),
            label: label.to_string(),
            err_count: errs.len(),
            errs: errs.into_iter().take(8).collect(),
        });
    }
}

mod artifact_integrity;

#[path = "mathematical_verification/gunzip_json.rs"]
mod gunzip_json;
pub use gunzip_json::gunzip_json;

#[path = "mathematical_verification/verify_phase.rs"]
mod verify_phase;
pub use verify_phase::verify_phase;

#[path = "mathematical_verification/tempdir.rs"]
mod tempdir;
use tempdir::list_files;
use tempdir::run_p1_gates;
use tempdir::tempdir;

#[path = "mathematical_verification/run_p2_gates.rs"]
mod run_p2_gates;
use run_p2_gates::run_p2_gates;

mod phase_validation;
pub use phase_validation::phase_gate;
