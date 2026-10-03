//! The voxel dump side of the blueprint compiler.
//!
//! **Role:** the in-memory model of a `tbd-voxel-dump/1` file, its strict reader, the tunables
//! every interpretation stage reads, the generator that writes a dump from a game model's
//! triangles (`voxels-from-mesh`), and the analytic buildings the tests march.
//! **Position:** read by [`crate::blueprint_from_voxels`] and every stage of
//! [`crate::architectural_analysis`]; the generator reads models through
//! [`crate::mesh_decoding`].
//! **Signals & state:** none; the reader and the generator touch one file each.
//! **Invariants:** a dump whose version is not the one the model knows is refused; the generator
//! marches the same lattice the Workbench dump action does.

pub(crate) mod analysis_parameters;
pub(crate) mod dump_parser;
pub(crate) mod mesh_voxelization;
#[cfg(test)]
pub(crate) mod synthetic_fixtures;
pub(crate) mod voxel_types;
