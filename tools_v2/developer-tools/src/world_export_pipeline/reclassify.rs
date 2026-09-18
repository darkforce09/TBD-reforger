//! T-278 — regenerate the prefab catalogue's **classification lane** from committed artifacts
//! alone: no Enfusion Workbench run, no hand-copied staging, no game install.
//!
//! THE PROBLEM THIS EXISTS FOR
//! ---------------------------
//! `packages/tbd-schema/rules/prefab-classify.json` says it in its own description: *"EDITING
//! THIS FILE CHANGES NOTHING UNTIL THE CATALOGUE IS REBUILT."* The only rebuild path was
//! `world build-objects`, which hard-requires `packages/map-assets/<terrain>/staging/export/
//! raw-entities.jsonl` — a ~1.2M-row Workbench export that is **gitignored** (`.gitignore:18`)
//! and absent from every clone. So a rule edit is unverifiable and unshippable by anyone who is
//! not sitting in front of Workbench, and `cargo xtask map export-terrain` exits 2 for everybody else.
//!
//! T-244 is the measured consequence: it added a `vehicle` kind plus wreck rules, the gates went
//! green against the *rules*, and the shipped `prefabs.json.gz` never changed. Its own agent
//! disclosed the change was latent. It stayed latent.
//!
//! WHAT THIS DOES, AND WHAT IT HONESTLY CANNOT
//! -------------------------------------------
//! Classification is a pure function of `resourceName` (`classify.rs`: first rule whose
//! `match.resourceNameContains` substring hits, case-sensitive, file order = priority). Every
//! `resourceName` already lives in the committed `objects/prefabs.json.gz`, and every instance
//! count is recoverable from the committed `objects/chunks/*.json.gz`. So the whole
//! classification lane — `kind`, `class`, `ai`, `gameplay`, `render`, `tags` — plus the census
//! counts derived from it are reproducible from the repo. That is what this rebuilds.
//!
//! It does NOT invent placements. `spatial.halfExtentsM` is measured from engine halfExtents
//! sampled during the Workbench export, and those samples are not in the repo, so a measured
//! `spatial` is **preserved verbatim**. A row whose committed `spatial` is byte-equal to the
//! template of the rule that produced it was a template fallback, not a measurement, so it is
//! re-templated from the new rule (see `respatialize`). `needsReview.prefabs[]` counts raw rows
//! that were excluded from the catalogue entirely and is likewise not derivable here; it is
//! preserved and reported as staging-derived rather than silently rewritten to a subset.
//!
//! Default mode is CHECK: read-only, exit 1 on drift. That is the gate this repo did not have —
//! run on the day T-244 landed it would have gone red with the 16 rows it was about to strand.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::catalog_emit;
use serde_json::{Map, Value, json};

use super::chunk_partitioner::{gunzip, gz9};
use super::classify::{Classifier, Rules, load_rules};
use super::json_number_formatting::js_normalize;
use crate::browser_testing::server::repo_root;

/// One prefab whose classification the current rules disagree with the committed catalogue on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    pub prefab_id: u64,
    pub resource_name: String,
    pub old_kind: String,
    pub old_class: String,
    pub new_kind: String,
    pub new_class: String,
}

/// What a reclassify pass found. `kinds_*` are ordered histograms for the report.
#[derive(Debug)]
pub struct Report {
    pub rows: usize,
    pub matched: usize,
    pub drift: Vec<Drift>,
    pub kinds_before: BTreeMap<String, u64>,
    pub kinds_after: BTreeMap<String, u64>,
    pub unclassified_before: u64,
    pub unclassified_after: u64,
    /// Kinds present after the pass that no committed row carries — the new census buckets a
    /// consumer schema has to accept before the artifact can land.
    pub new_kinds: Vec<String>,
}

impl Report {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.drift.is_empty()
    }
}

/// `--write` writes the artifacts; the default reports and touches nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Check,
    Write,
}

#[cfg(test)]
#[path = "tests/reclassify/tests.rs"]
mod tests;

#[path = "reclassify/obj_get.rs"]
mod obj_get;
pub use obj_get::reclassify_rows;
pub use obj_get::reclassify_terrain;
pub use obj_get::resolve_out_base;

#[cfg(test)]
pub(crate) use obj_get::rule_for_kind_class;
