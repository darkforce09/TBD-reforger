//! **Role:** Regenerate the prefab catalogue's **classification lane** from committed artifacts
//! alone: no Enfusion Workbench run, no hand-copied staging, no game install.
//!
//! THE PROBLEM THIS EXISTS FOR
//! ---------------------------
//! `contracts/rules/prefab-classify.json` says it in its own description: *"EDITING
//! THIS FILE CHANGES NOTHING UNTIL THE CATALOGUE IS REBUILT."* The only rebuild path was
//! `world build-objects`, which hard-requires `assets/scratch/<terrain>/export/
//! raw-entities.jsonl` — a ~1.2M-row Workbench export that is **gitignored** (`.gitignore:18`)
//! and absent from every clone. So a rule edit is unverifiable and unshippable by anyone who is
//! not sitting in front of Workbench, and `cargo xtask map export-terrain` exits 2 for everybody else.
//!
//! The measured consequence of a rule change: adding a `vehicle` kind plus wreck rules sent the gates
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
//! run on the day such a change lands goes red with the rows it is about to strand.
//!
//! **Position:** the `reclassify` subcommand and the platform wave gate run it.
//! **Signals & state:** none at the module root.
//! **Invariants:** the default mode writes nothing; `--write` rewrites only the classification
//! lane.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt as _, refuse};

use super::catalog_emit;
use serde_json::{Map, Value, json};

use super::chunk_partitioner::{gunzip, gz9};
use super::classify::{Classifier, Rules, load_rules};
use super::json_number_formatting::js_normalize;
use ::repository_layout::find_repository_root;

/// One prefab whose classification the current rules disagree with the committed catalogue on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    pub(crate) prefab_id: u64,
    /// The prefab's resource name.
    pub resource_name: String,
    /// The kind the committed catalogue carries.
    pub old_kind: String,
    /// The class the committed catalogue carries.
    pub old_class: String,
    /// The kind the current rules give.
    pub new_kind: String,
    /// The class the current rules give.
    pub new_class: String,
}

/// What a reclassify pass found. `kinds_*` are ordered histograms for the report.
#[derive(Debug)]
pub struct Report {
    /// Catalogue rows read.
    pub rows: usize,
    /// Rows a rule matched under the current rules.
    pub matched: usize,
    /// The prefabs whose classification changed.
    pub drift: Vec<Drift>,
    /// Rows per kind in the committed catalogue.
    pub kinds_before: BTreeMap<String, u64>,
    /// Rows per kind under the current rules.
    pub kinds_after: BTreeMap<String, u64>,
    /// Rows the committed catalogue leaves unclassified.
    pub unclassified_before: u64,
    /// Rows the current rules leave unclassified.
    pub unclassified_after: u64,
    /// Kinds present after the pass that no committed row carries — the new census buckets a
    /// consumer schema has to accept before the artifact can land.
    pub new_kinds: Vec<String>,
}

impl Report {
    #[must_use]
    /// Whether the current rules change no committed classification.
    pub fn is_clean(&self) -> bool {
        self.drift.is_empty()
    }
}

/// `--write` writes the artifacts; the default reports and touches nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Report the drift and write nothing.
    Check,
    /// Write the reclassified artifacts.
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
