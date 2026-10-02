//! Role: coverage 2.
//! Position: `spatial/los/world` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::los::world::state::WorldOccluder;
use crate::world::architecture::compound::instances::InstanceKind;

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
