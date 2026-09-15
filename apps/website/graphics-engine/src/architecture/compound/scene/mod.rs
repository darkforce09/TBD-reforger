//! Role: Module boundary for architecture/compound/scene.
//! Position: `architecture/compound/scene` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::architecture::compound::assembly::CompoundBuilding`.
pub use crate::architecture::compound::assembly::CompoundBuilding;

/// Re-export `crate::architecture::compound::assembly::CompoundError`.
pub use crate::architecture::compound::assembly::CompoundError;

/// Re-export `crate::architecture::compound::assembly::CoverTier`.
pub use crate::architecture::compound::assembly::CoverTier;

/// Re-export `crate::architecture::compound::assembly::FlatMesh`.
pub use crate::architecture::compound::assembly::FlatMesh;

/// Re-export `crate::architecture::compound::assembly::INSTANCES_SCHEMA_VERSION`.
pub use crate::architecture::compound::assembly::INSTANCES_SCHEMA_VERSION;

/// Re-export `crate::architecture::compound::assembly::PlacementSource`.
pub use crate::architecture::compound::assembly::PlacementSource;

/// Re-export `crate::architecture::compound::instances::Instance`.
pub use crate::architecture::compound::instances::Instance;

/// Re-export `crate::architecture::compound::instances::InstanceKind`.
pub use crate::architecture::compound::instances::InstanceKind;

/// Re-export `crate::architecture::compound::instances::InstanceRecord`.
pub use crate::architecture::compound::instances::InstanceRecord;

/// Re-export `crate::architecture::compound::instances::InstancesFile`.
pub use crate::architecture::compound::instances::InstancesFile;

/// Re-export `crate::architecture::compound::instances::LocalTransform`.
pub use crate::architecture::compound::instances::LocalTransform;

/// Re-export `crate::architecture::compound::instances::instances_from_records`.
pub use crate::architecture::compound::instances::instances_from_records;

/// Re-export `crate::architecture::compound::doors::DoorRecord`.
pub use crate::architecture::compound::doors::DoorRecord;

/// Re-export `crate::architecture::compound::doors::DoorState`.
pub use crate::architecture::compound::doors::DoorState;
#[cfg(test)]
mod tests;
