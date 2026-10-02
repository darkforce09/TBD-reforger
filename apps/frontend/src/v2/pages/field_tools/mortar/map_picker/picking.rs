//! Placing the target and the guns on the map: which position a click or a drag moves, and what
//! it writes.
//!
//! **Role:** names the position being placed ([`Placement`]), lists the choices the picker offers,
//! finds the placed marker under a pointer, and writes a map position into the target or a gun as
//! a 10-figure grid reference.
//! **Position:** between the map view's pointer events (`super`) and the page's position drafts
//! (`crate::v2::pages::field_tools::mortar::inputs`); the solve reads the grids written here like
//! typed ones.
//! **Signals & state:** none; pure functions over the drafts.
//! **Invariants:** a placement writes only the grid reference of the one position it names and
//! leaves its height choice and manual height as they were; a gun that is no longer in the battery
//! is never written; the marker under a pointer is the nearest placed one within the hit radius,
//! so overlapping markers resolve deterministically.

use crate::v2::pages::field_tools::mortar::inputs::battery::GunDraft;
use crate::v2::pages::field_tools::mortar::inputs::positions::PositionDraft;
use map_engine::camera::grid_reference::{format_grid, parse_grid, GridFigures};

/// Radius around a placed marker, CSS pixels, within which a press grabs it for a drag.
pub(crate) const MARKER_HIT_RADIUS_PX: f64 = 14.0;

/// The position a map click or drag writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Placement {
    /// The target.
    Target,
    /// The gun whose [`GunDraft::key`] this is.
    Gun(u32),
}

impl Placement {
    /// The value of the placement picker's `<option>` for this placement.
    pub(crate) fn option_value(self) -> String {
        match self {
            Self::Target => "target".to_string(),
            Self::Gun(key) => format!("gun-{key}"),
        }
    }

    /// The placement an `<option>` value names; `None` for anything else.
    pub(crate) fn from_option_value(value: &str) -> Option<Self> {
        if value == "target" {
            return Some(Self::Target);
        }
        value
            .strip_prefix("gun-")
            .and_then(|key| key.parse().ok())
            .map(Self::Gun)
    }
}

/// Every placement the picker offers with its label: the target, then each gun in battery order.
pub(crate) fn placement_options(guns: &[GunDraft]) -> Vec<(Placement, String)> {
    std::iter::once((Placement::Target, "Target".to_string()))
        .chain(guns.iter().map(|gun| {
            let label = gun.label.trim();
            let label = if label.is_empty() {
                "Unnamed gun"
            } else {
                label
            };
            (Placement::Gun(gun.key), label.to_string())
        }))
        .collect()
}

/// `placing` when it still names a position of the battery, else the target.
pub(crate) fn valid_placement(placing: Placement, guns: &[GunDraft]) -> Placement {
    match placing {
        Placement::Gun(key) if guns.iter().all(|gun| gun.key != key) => Placement::Target,
        other => other,
    }
}

/// Writes map position `(x, y)` into the position `placing` names, as a 10-figure grid reference.
/// `false` when `placing` names a gun the battery no longer holds.
pub(crate) fn apply_placement(
    target: &mut PositionDraft,
    guns: &mut [GunDraft],
    placing: Placement,
    x: f64,
    y: f64,
) -> bool {
    let draft = match placing {
        Placement::Target => Some(target),
        Placement::Gun(key) => guns
            .iter_mut()
            .find(|gun| gun.key == key)
            .map(|gun| &mut gun.position),
    };
    match draft {
        Some(draft) => {
            draft.grid = format_grid(x, y, GridFigures::Ten);
            true
        }
        None => false,
    }
}

/// Every position whose grid reference parses, with its placement: the guns in battery order,
/// then the target.
pub(crate) fn placed_markers(
    target: &PositionDraft,
    guns: &[GunDraft],
) -> Vec<(Placement, [f64; 2])> {
    guns.iter()
        .filter_map(|gun| {
            parse_grid(&gun.position.grid)
                .ok()
                .map(|(x, y)| (Placement::Gun(gun.key), [x, y]))
        })
        .chain(
            parse_grid(&target.grid)
                .ok()
                .map(|(x, y)| (Placement::Target, [x, y])),
        )
        .collect()
}

/// The placed marker nearest to map position `at` within `radius_m` metres; `None` when none is
/// that close or the radius is not a positive number.
pub(crate) fn marker_near(
    markers: &[(Placement, [f64; 2])],
    at: [f64; 2],
    radius_m: f64,
) -> Option<Placement> {
    if !(radius_m.is_finite() && radius_m > 0.0) {
        return None;
    }
    markers
        .iter()
        .map(|(placement, p)| (*placement, (p[0] - at[0]).hypot(p[1] - at[1])))
        .filter(|(_, distance)| *distance <= radius_m)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(placement, _)| placement)
}
