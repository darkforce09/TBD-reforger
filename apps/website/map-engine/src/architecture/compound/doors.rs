//! Role: doors.
//! Position: `architecture/compound` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::compound::assembly::CompoundBuilding;
use crate::architecture::compound::instances::Instance;
use crate::architecture::compound::instances::InstanceKind;
use crate::architecture::compound::transform::Rigid;
use serde::Deserialize;
use serde::Serialize;

/// Door mechanics from the prefab's `DoorComponent` / `SlidingDoorComponent`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorRecord {
    /// Signed sweep about the leaf's local Y; the sign is the swing side.
    pub angle_range_deg: f64,

    /// Closed angle deg.
    pub closed_angle_deg: f64,

    /// Initial angle deg.
    pub initial_angle_deg: f64,

    /// `AngleRange` was set somewhere in the prefab chain (else the 90° default).
    #[serde(default)]
    pub angle_range_explicit: bool,

    /// Sliding door: travel along local X when fully open (m). `None` for a rotating door.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened_distance: Option<f64>,
}

/// A door leaf's state: closed, or open by a fraction of its full sweep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorState {
    /// Closed.
    Closed,

    /// Fraction of the full sweep (`angle_range_deg` for a hinge, `opened_distance` for a slider), clamped to `0..=1` wherever it is applied.
    Open { fraction: f64 },
}

impl DoorState {
    /// Fully open.
    pub const OPEN: DoorState = DoorState::Open { fraction: 1.0 };
}

impl DoorState {
    /// The applied fraction (`0` closed … `1` fully open).
    #[must_use]
    pub fn fraction(self) -> f64 {
        match self {
            DoorState::Closed => 0.0,
            DoorState::Open { fraction } => {
                if fraction.is_finite() {
                    fraction.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
        }
    }
}

impl DoorState {
    /// Is open.
    #[must_use]
    pub fn is_open(self) -> bool {
        self.fraction() > 0.0
    }
}

impl DoorState {
    /// Closed ↔ fully open (the viewer's click).
    #[must_use]
    pub fn toggled(self) -> DoorState {
        if self.is_open() {
            DoorState::Closed
        } else {
            DoorState::OPEN
        }
    }
}

impl Instance {
    /// Is this a door leaf with hinge / slide parameters?.
    #[must_use]
    pub fn is_door(&self) -> bool {
        self.record.kind == InstanceKind::DoorLeaf && self.record.door.is_some()
    }
}

impl Instance {
    /// The leaf's motion for its current state, in leaf space: a hinge turns about the leaf origin's local Y (`DoorComponent` — the collider hangs off the hinge along local +X, so `rot_y(θ)` swings the free edge; the yaw sense is the pinned Enfusion yaw), a slider translates along local X by `fraction · opened_distance`. Identity for everything else.
    #[must_use]
    pub fn hinge(&self) -> Rigid {
        let Some(door) = self
            .record
            .door
            .filter(|_| self.record.kind == InstanceKind::DoorLeaf)
        else {
            return Rigid::identity();
        };
        let f = self.state.fraction();
        match door.opened_distance {
            Some(dist) => Rigid::translation([f * dist, 0.0, 0.0]),
            None => Rigid::rot_y(door.closed_angle_deg + f * door.angle_range_deg),
        }
    }
}

impl CompoundBuilding {
    /// Every door leaf, in instance order.
    pub fn doors(&self) -> impl Iterator<Item = &Instance> {
        self.instances.iter().filter(|i| i.is_door())
    }
}

impl CompoundBuilding {
    /// Set a leaf's state (the fraction is clamped on use). `false` when `id` is not a door.
    pub fn set_door(&mut self, id: &str, state: DoorState) -> bool {
        match self.instance_index(id) {
            Some(i) if self.instances[i].is_door() => {
                self.instances[i].state = state;
                true
            }
            _ => false,
        }
    }
}

impl CompoundBuilding {
    /// A leaf's state (`None` when `id` is not a door).
    #[must_use]
    pub fn door_state(&self, id: &str) -> Option<DoorState> {
        self.instance_index(id)
            .filter(|&i| self.instances[i].is_door())
            .map(|i| self.instances[i].state)
    }
}
