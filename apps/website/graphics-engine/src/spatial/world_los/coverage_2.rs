//! Role: coverage 2.
//! Position: `spatial/world_los` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::compound::instances::InstanceKind;
use crate::spatial::world_los::state::WorldOccluder;

impl WorldOccluder {
    /// The instance kinds a pid's root record carries, once expanded.
    #[must_use]
    pub fn root_kind_of(&self, pid: u16) -> Option<InstanceKind> {
        self.expanded
            .get(&pid)
            .and_then(|po| po.instances.first())
            .map(|i| i.record.kind)
    }
}
