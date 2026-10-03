//! Occlusion sidecars and prefab placement.
//!
//! **Role:** turns game models into `.bvh` occlusion sidecars (a triangle mesh with its bounding
//! volume hierarchy), walks a building prefab's children out of the game paks into an instances
//! file, and checks the result against the engine: line-of-sight parity with the Workbench
//! oracle, socket placements against a recon dump and the Euler composition of prefab angles.
//! **Position:** reads models through [`crate::mesh_decoding`] and the paks through
//! `enfusion_pak`; its walk, sources and writer feed [`crate::archive_emission`]; the
//! `bvh-parity`, `bvh-emit`, `bvh-batch`, `instances-verify` and `rotation-pin` commands enter
//! here.
//! **Signals & state:** none; each command reads its inputs and writes its outputs once.
//! **Invariants:** a sidecar is written only when its bytes change; placements are compared in
//! the building's local frame within 2 cm and 1°.

pub(crate) mod batch_processing;
pub(crate) mod construction;
pub(crate) mod instance_pairs;
pub(crate) mod instance_verification;
pub(crate) mod prefab_catalog;
pub(crate) mod rotation_validation;
pub(crate) mod world_instances;
