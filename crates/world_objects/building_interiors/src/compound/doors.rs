//! A compound's doors: their records, their states and how a leaf moves.
//!
//! **Role:** the door record a leaf carries ([`DoorRecord`]), its state ([`DoorState`]), the hinge or
//! slide motion that places the leaf, and the door lookups and setters on [`CompoundBuilding`].
//! **Position:** read by [`crate::compound::instances`]; the Mission Creator's building viewer and
//! the map engine's interior line of sight set and read door states.
//! **Signals & state:** a compound's door states, set by its owner.
//! **Invariants:** an open fraction is clamped to `0..=1` on use; setting a door on an instance that
//! is not a door leaf changes nothing.

use std::borrow::Borrow;

use crate::building_ids::CompoundInstanceId;
use crate::compound::assembly::CompoundBuilding;
use crate::compound::instances::Instance;
use crate::compound::instances::InstanceKind;
use geometry_primitives::rigid_transform::Rigid;
use serde::Deserialize;
use serde::Serialize;

/// Door mechanics from the prefab's `DoorComponent` / `SlidingDoorComponent`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorRecord {
    /// Signed sweep about the leaf's local Y; the sign is the swing side.
    pub angle_range_deg: f64,

    /// Leaf yaw about local Y when closed, degrees (default 0); a hinge turns to this plus
    /// `fraction * angle_range_deg`.
    pub closed_angle_deg: f64,

    /// Starting position of the leaf (yaw in degrees for a hinge, travel in metres for a
    /// slider); its offset from `closed_angle_deg` sets the initial open fraction.
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
    /// The leaf at its closed pose (fraction 0).
    Closed,

    /// Open by part of the full sweep.
    Open {
        /// Fraction of the full sweep (`angle_range_deg` for a hinge, `opened_distance` for a
        /// slider), clamped to `0..=1` wherever it is applied.
        fraction: f64,
    },
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
    /// Whether the applied fraction is above 0; a non-finite fraction reads as closed.
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
    /// `id` is a [`CompoundInstanceId`] or any form it borrows as, such as its `&str` spelling.
    pub fn set_door<Q>(&mut self, id: &Q, state: DoorState) -> bool
    where
        CompoundInstanceId: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
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
    /// A leaf's state (`None` when `id` is not a door). `id` is a [`CompoundInstanceId`] or any
    /// form it borrows as, such as its `&str` spelling.
    #[must_use]
    pub fn door_state<Q>(&self, id: &Q) -> Option<DoorState>
    where
        CompoundInstanceId: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        self.instance_index(id)
            .filter(|&i| self.instances[i].is_door())
            .map(|i| self.instances[i].state)
    }
}
