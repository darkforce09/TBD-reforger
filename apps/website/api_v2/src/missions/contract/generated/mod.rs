//! Generated cross-boundary type projections: `typify` output for
//! `contracts_v2/definitions/*.json`.
//!
//! DO NOT EDIT the per-schema files — regenerate them with `cargo xtask ci schema-codegen`, and
//! the `verify-codegen-fresh` gate diffs this directory to prove they match their schemas. Lints
//! are suppressed because these mirror the JSON wire shapes verbatim.
//!
//! The loadout-export model is NOT here: typify's output for its versioned root `oneOf` is lossy,
//! so it is hand-maintained in [`super::loadout_projection`].

#[allow(clippy::all, dead_code)]
pub mod faction_library;
#[allow(clippy::all, dead_code)]
pub mod mission_editor;
#[allow(clippy::all, dead_code)]
pub mod registry_compat;
#[allow(clippy::all, dead_code)]
pub mod registry_items;
