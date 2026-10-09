//! Manifest-driven relocation of tracked paths and every reference to them.
//!
//! **Role:** moves tracked files and folders and rewrites every reference to them from a
//! relocation manifest, so no path is ever rewritten by hand, and proves that no live file still
//! spells what a manifest retired. [`dry_run`], [`apply`] and [`verify`] are the three modes of
//! `cargo xtask refactor relocate`.
//!
//! **Position:** tier 3 of `tools/commands`, over `verification_core` (verdicts and the run
//! report), `process_runner` (git), `repository_layout` (the frozen areas and the manifests
//! folder) and `ticket_model` (the closed ticket statuses). The xtask binary parses the command
//! line, finds the checkout root and calls the modes; every move of a tracked file runs through
//! it, and its manifests stay in `documentation/relocation_manifests/` as the registry it verifies.
//!
//! **Signals & state:** none held; each mode reads one checkout, and only [`apply`] changes it.
//!
//! **Invariants:** nothing is written before the whole plan is computed, unresolved-free and its
//! planned tree clean; a failed apply leaves index and working tree byte-identical; each mode
//! returns the exit code (0 clean, 1 findings, 2 no verdict, never a pass on an unread input).

mod file_treatment;
mod manifest;
mod manifest_chronology;
mod move_placement;
mod path_mapping;
mod path_references;
mod plan_application;
mod plan_summary;
mod planned_tree;
pub mod prelude;
mod relocation_modes;
mod relocation_plan;
mod repository_files;
mod retired_spellings;
mod rust_lexer;
mod rust_paths;
mod scope_history;
mod text_edits;
mod text_tokens;

pub use relocation_modes::{apply, dry_run, verify};

#[cfg(test)]
#[path = "tests/fixture_repository.rs"]
mod fixture_repository;

#[cfg(test)]
#[path = "tests/building_blocks.rs"]
mod building_blocks;

#[cfg(test)]
#[path = "tests/relocation_scenarios.rs"]
mod relocation_scenarios;

#[cfg(test)]
#[path = "tests/rust_path_scenarios.rs"]
mod rust_path_scenarios;

#[cfg(test)]
#[path = "tests/unusual_spelling_scenarios.rs"]
mod unusual_spelling_scenarios;

#[cfg(test)]
#[path = "tests/move_placement_scenarios.rs"]
mod move_placement_scenarios;

#[cfg(test)]
#[path = "tests/ambiguous_literal_scenarios.rs"]
mod ambiguous_literal_scenarios;

#[cfg(test)]
#[path = "tests/manifest_chronology_scenarios.rs"]
mod manifest_chronology_scenarios;

#[cfg(test)]
#[path = "tests/crate_anchor_scenarios.rs"]
mod crate_anchor_scenarios;

#[cfg(test)]
#[path = "tests/single_pass_verification_scenarios.rs"]
mod single_pass_verification_scenarios;
