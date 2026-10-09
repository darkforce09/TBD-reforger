//! The crate's scenario and building-block tests: one fixture repository, the building blocks, and
//! one scenario file per relocation concern.
//!
//! **Role:** gathers the test modules under one `tests` module and brings the crate's private
//! modules and modes into scope, so each test file names them as `super::<module>`.
//! **Position:** compiled only under `cfg(test)`, declared once by `lib.rs`.
//! **Signals & state:** none; module declarations.
//! **Invariants:** every file in this folder is declared here exactly once.

use super::{
    apply, dry_run, file_treatment, manifest, manifest_chronology, path_mapping, path_references,
    plan_application, planned_tree, relocation_modes, relocation_plan, repository_files,
    retired_spellings, rust_lexer, rust_paths, scope_history, verify,
};

mod ambiguous_literal_scenarios;
mod building_blocks;
mod crate_anchor_scenarios;
mod fixture_repository;
mod manifest_chronology_scenarios;
mod move_placement_scenarios;
mod relocation_scenarios;
mod rust_path_scenarios;
mod single_pass_verification_scenarios;
mod spelling_revival_scenarios;
mod unusual_spelling_scenarios;
