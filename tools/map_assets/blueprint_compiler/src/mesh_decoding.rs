//! The reader of Enfusion `.xob` game models.
//!
//! **Role:** decodes a model's triangles, its fire-collision (COLL) colliders with each
//! triangle's game material and its node table of sockets, and holds the `xob-inspect` and
//! `pak-cat` commands that print what the reader sees and peek into the game paks.
//! **Position:** fed model bytes by `enfusion_pak` and [`crate::occlusion_sidecars`]'s sources; read by
//! [`crate::occlusion_sidecars`], [`crate::archive_emission`] and [`crate::voxel_processing`].
//! **Signals & state:** none; pure parsers over byte slices, plus the two print commands.
//! **Invariants:** a parser never reads past its chunk; a malformed model is an error, never a
//! partial mesh.

pub(crate) mod archive_inspection;
pub(crate) mod mesh_format;
pub(crate) mod node_records;
