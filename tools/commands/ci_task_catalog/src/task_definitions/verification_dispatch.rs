//! Zero-argument adapters for the verifications a [`super::TASKS`] row runs in-process.
//!
//! **Role:** one adapter per in-process verification step of the task table.
//! **Position:** pulled into `task_definitions.rs` by `#[path]`; over the check, schema, database,
//! deployment and map asset verification crates and this crate's own workflow checks.
//! **Signals & state:** none; each call reads the checkout afresh.
//! **Invariants:** an adapter returns its verification's exit status unchanged.
//!
//! A `Step::Xtask` holds a plain `fn() -> Result<u8>` pointer, which cannot capture; each adapter
//! here supplies the arguments its verification needs — the repository root, the terrain the CI
//! lane always checks, or, for a documentation gate, a request for the whole repository's
//! committed files (the link check also takes the command tree the binary handed the lane).

use super::*;
use documentation_checks::GateRequest;

/// The terrain every map asset step of the task table checks.
const CHECKED_TERRAIN: &str = "everon";

pub(super) fn run_map_object_golden() -> crate::Result<u8> {
    Ok(map_asset_verification::object_goldens::map_object_golden(
        &find_repository_root()?,
    )?)
}

pub(super) fn run_height_labels() -> crate::Result<u8> {
    Ok(map_asset_verification::labels::height_labels(
        &find_repository_root()?,
        CHECKED_TERRAIN,
    )?)
}

pub(super) fn run_terrain_manifest() -> crate::Result<u8> {
    Ok(map_asset_verification::terrain_manifest::terrain_manifest(
        &find_repository_root()?,
        CHECKED_TERRAIN,
    )?)
}

pub(super) fn run_terrain_alignment() -> crate::Result<u8> {
    Ok(map_asset_verification::labels::terrain_alignment(
        &find_repository_root()?,
        CHECKED_TERRAIN,
        false,
    )?)
}

pub(super) fn run_terrain_alignment_strict() -> crate::Result<u8> {
    Ok(map_asset_verification::labels::terrain_alignment(
        &find_repository_root()?,
        CHECKED_TERRAIN,
        true,
    )?)
}

pub(super) fn run_no_select_star() -> crate::Result<u8> {
    Ok(
        database_operations::database_checks::sql_deserialization::verify_no_select_star(
            &find_repository_root()?,
        )?,
    )
}

/// The database lane's complete integration suite in process: a fresh database per run and the
/// cleanup after every outcome (`cargo xtask db test-it`, which `mk rust-ci` also runs).
pub(crate) fn run_database_test_suite() -> crate::Result<u8> {
    Ok(database_operations::local_database::test_it::run_complete_suite()?)
}

pub(super) fn run_route_tags() -> crate::Result<u8> {
    Ok(repository_checks::architecture::route_tags::verify_route_tags(&find_repository_root()?)?)
}

/// Judges the pinned Enfusion script roots against the comment card (no `--path` narrowing).
pub(super) fn run_enfusion_comments() -> crate::Result<u8> {
    Ok(
        mod_script_checks::enfusion_comments::verify_enfusion_comments(
            &find_repository_root()?,
            &[],
        ),
    )
}

pub(super) fn run_engine_layers() -> crate::Result<u8> {
    Ok(
        repository_checks::architecture::engine_layer_boundaries::verify_engine_layers(
            &find_repository_root()?,
        )?,
    )
}

pub(super) fn run_staging_compose_paths() -> crate::Result<u8> {
    Ok(
        deployment::deployment_checks::staging_compose_paths::verify_staging_compose_paths(
            &find_repository_root()?,
        )?,
    )
}

pub(super) fn run_mission_rest_size_limits() -> crate::Result<u8> {
    Ok(
        mod_script_checks::mission_rest_size_limits::verify_mission_rest_size_limits(
            &find_repository_root()?,
        )?,
    )
}

pub(super) fn run_ci_schema_parity() -> crate::Result<u8> {
    crate::workflow_checks::schema_parity::verify_ci_schema_parity(&find_repository_root()?)
}

/// `cargo xtask verify readme-coverage` over the whole repository's committed files, the view
/// CI judges.
pub(super) fn run_readme_coverage() -> crate::Result<u8> {
    Ok(
        documentation_checks::readme_coverage::verify_readme_coverage(
            &find_repository_root()?,
            &GateRequest::default(),
        ),
    )
}

/// `cargo xtask verify link-check` over the whole repository's committed files, printing the
/// first breaks in full as the bare verb does, with citations judged against the command tree the
/// binary handed the lane and the recipe and task tables.
pub(super) fn run_link_check() -> crate::Result<u8> {
    use documentation_checks::link_check::{BreakListing, CommandVocabulary, verify_link_check};
    let command_tree = crate::task_runner::installed_command_tree().ok_or_else(|| {
        crate::Error::msg(
            "link-check runs in the ci lane only with the command tree xtask hands it",
        )
    })?;
    let vocabulary = CommandVocabulary {
        command_tree,
        recipe_targets: crate::build_lane::recipes::TARGETS.to_vec(),
        task_names: super::TASKS.iter().map(|task| task.name).collect(),
    };
    Ok(verify_link_check(
        &find_repository_root()?,
        &GateRequest::default(),
        BreakListing::First,
        &vocabulary,
    ))
}

/// `cargo xtask verify markdown-placement` over the whole repository's committed files.
pub(super) fn run_markdown_placement() -> crate::Result<u8> {
    Ok(
        documentation_checks::markdown_placement::verify_markdown_placement(
            &find_repository_root()?,
            &GateRequest::default(),
        ),
    )
}
