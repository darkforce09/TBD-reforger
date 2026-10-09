//! The blueprint compiler's writers.
//!
//! **Role:** assembles one building's interpretation into a schema-checked blueprint JSON, reads
//! the whole-catalogue prefab occluder library of `bvh-batch --all-prefabs`, and folds the
//! library and every blueprint into `prefabs/building_blueprints.rkyv`.
//! **Position:** fed by [`crate::architectural_analysis`] (walls, roof, plates) and
//! [`crate::occlusion_sidecars`] (the prefab walk, sources and writer); run by
//! [`crate::blueprint_from_voxels`] and its `archive` subcommand.
//! **Signals & state:** none; each command reads its inputs and writes its outputs once.
//! **Invariants:** every emitted document passes its schema in `contracts/definitions/` before it
//! is written, and the archive refuses inputs that disagree.

pub(crate) mod archive_command;
pub(crate) mod archive_writer;
pub(crate) mod blueprint_assembly;
pub(crate) mod prefab_library;
