//! Turning a stored fire mission back into the calculator's drafts.
//!
//! **Role:** unpacks a [`SavedFire`] row — a catalog-model row with its guns, or a legacy row with
//! one fire position — into the target, the battery, the armament, the wind and the burst height
//! the page's inputs hold.
//! **Position:** read by the saved list's click and by the load-time hydration (`super`); the page
//! applies the drafts.
//! **Signals & state:** none; pure functions over plain values.
//! **Invariants:** the numeric columns are authoritative and read first; a legacy row's only record
//! of a coordinate pair may be its `x, y` grid text, and [`parse_legacy_grid`] reads it; the two
//! pairs fall back independently; a row with no usable coordinates restores as nothing, never as
//! the origin; a manual height comes back manual with its value, an elevation-model height comes
//! back as a terrain height that is re-sampled.

use crate::v2::core::api::dto::{HeightSource, SavedFire};
use crate::v2::pages::field_tools::mortar::inputs::battery::GunDraft;
use crate::v2::pages::field_tools::mortar::inputs::positions::{HeightChoice, PositionDraft};
use crate::v2::pages::field_tools::mortar::inputs::weapon_and_shell::{
    ArmamentSelection, ChargeChoice,
};
use crate::v2::pages::field_tools::mortar::inputs::wind::WindDraft;
use map_coordinates::grid_reference::{format_grid, parse_grid, GridFigures};

/// One restored position: map metres and, when the row recorded one, its height and source.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RestoredPosition {
    /// Easting, metres.
    pub(crate) x: f64,
    /// Northing, metres.
    pub(crate) y: f64,
    /// The recorded height and where it came from.
    pub(crate) height: Option<(f64, HeightSource)>,
}

/// A stored row unpacked into what the inputs hold.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Restored {
    /// The target.
    pub(crate) target: RestoredPosition,
    /// The guns in battery order, with their labels.
    pub(crate) guns: Vec<(String, RestoredPosition)>,
    /// Weapon, shell and charge; `None` for a legacy row.
    pub(crate) selection: Option<ArmamentSelection>,
    /// The wind; `None` when the row recorded none.
    pub(crate) wind: Option<WindDraft>,
    /// Burst height text; `None` when the row recorded none.
    pub(crate) burst_height: Option<String>,
    /// When the row was stored.
    pub(crate) saved_at: String,
}

/// Parses the legacy `x, y` grid text a fire mission stored before grid references: two finite
/// numbers of metres around one comma.
///
/// # The backfill that filled the numeric columns is narrower
///
/// Migration 0020 backfilled `fp_x`/`fp_y`/`tgt_x`/`tgt_y` from this encoding with
/// `^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$`. This function parses with
/// `str::parse::<f64>`, which accepts strictly more syntax:
///
/// ```text
///   input            regex   parse_legacy_grid
///   '1000, 2000'       t         t        agree: what the legacy writer (`{x}, {y}`) wrote
///   '+1000, 2000'      f         t        `-?` has no `+`
///   '.5, 2'            f         t        `\d+` requires a digit before the point
///   '5., 2'            f         t        `(\.\d+)?` requires digits after it
///   '1e3, 500'         f         t        no exponent form
/// ```
///
/// **The divergence is under-permissive, which is the safe direction.** A backfill that accepted
/// more than this function would invent coordinates for rows the calculator shows as
/// unrestorable; one that accepts less only leaves a row where it is, because [`restore`] falls
/// back to this function whenever the numeric columns are null. The legacy writer formatted
/// `f64`'s `Display`, which never emits a `+`, a bare leading or trailing point, or an exponent,
/// so no stored row takes the divergent forms.
///
/// The accept sets are not nested: a grid whose integer part exceeds `f64::MAX` matches the regex
/// and overflows `double precision`, while this function returns `None` for it, because
/// `parse::<f64>` yields infinity and the finiteness guard refuses it.
pub(crate) fn parse_legacy_grid(text: &str) -> Option<(f64, f64)> {
    let (a, b) = text.split_once(',')?;
    let x: f64 = a.trim().parse().ok()?;
    let y: f64 = b.trim().parse().ok()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// A stored grid text: a 6-, 8- or 10-figure grid reference, else the legacy `x, y` text.
fn parse_stored_grid(text: &str) -> Option<(f64, f64)> {
    parse_grid(text).ok().or_else(|| parse_legacy_grid(text))
}

/// Unpacks `row`; `None` when it has no usable target or gun coordinates.
pub(crate) fn restore(row: &SavedFire) -> Option<Restored> {
    let target_height = row.target_height_m.zip(row.target_height_source);
    let (tx, ty) = match (row.tgt_x, row.tgt_y) {
        (Some(x), Some(y)) => (x, y),
        _ => parse_stored_grid(&row.target_grid)?,
    };
    let guns: Vec<(String, RestoredPosition)> = if row.guns.is_empty() {
        let (x, y) = match (row.fp_x, row.fp_y) {
            (Some(x), Some(y)) => (x, y),
            _ => parse_stored_grid(&row.fp_grid)?,
        };
        vec![("Gun 1".to_string(), RestoredPosition { x, y, height: None })]
    } else {
        row.guns
            .iter()
            .map(|gun| {
                let position = RestoredPosition {
                    x: gun.x,
                    y: gun.y,
                    height: Some((gun.height_m, gun.height_source)),
                };
                (gun.label.clone(), position)
            })
            .collect()
    };
    let selection = row
        .weapon_id
        .clone()
        .zip(row.shell_id.clone())
        .map(|(weapon_id, shell_id)| ArmamentSelection {
            weapon_id,
            shell_id,
            charge: row
                .charge_rings
                .map_or(ChargeChoice::Recommended, ChargeChoice::Rings),
        });
    let wind = row
        .wind_speed_m_s
        .zip(row.wind_from_deg)
        .map(|(speed, from)| WindDraft {
            speed_m_s: format!("{speed}"),
            from_deg: format!("{from}"),
        });
    Some(Restored {
        target: RestoredPosition {
            x: tx,
            y: ty,
            height: target_height,
        },
        guns,
        selection,
        wind,
        burst_height: row.burst_height_m.map(|h| format!("{h}")),
        saved_at: row.created_at.clone(),
    })
}

/// The draft of `position`: a 10-figure grid reference, and the recorded height, else
/// `fallback`'s height choice and text.
pub(crate) fn position_draft(
    position: &RestoredPosition,
    fallback: &PositionDraft,
) -> PositionDraft {
    let grid = format_grid(position.x, position.y, GridFigures::Ten);
    match position.height {
        Some((height, HeightSource::Manual)) => PositionDraft {
            grid,
            height_choice: HeightChoice::Manual,
            manual_height: format!("{height}"),
        },
        Some((_, HeightSource::Dem)) => PositionDraft {
            grid,
            height_choice: HeightChoice::Terrain,
            manual_height: fallback.manual_height.clone(),
        },
        None => PositionDraft {
            grid,
            ..fallback.clone()
        },
    }
}

/// The battery of `restored`, keyed from zero; each gun falls back on the current gun at its
/// place, else the first current gun, for a height the row did not record.
pub(crate) fn gun_drafts(restored: &Restored, current: &[GunDraft]) -> Vec<GunDraft> {
    let default = PositionDraft::default();
    restored
        .guns
        .iter()
        .enumerate()
        .map(|(index, (label, position))| {
            let fallback = current
                .get(index)
                .or_else(|| current.first())
                .map_or(&default, |gun| &gun.position);
            GunDraft {
                key: index as u32,
                label: label.clone(),
                position: position_draft(position, fallback),
            }
        })
        .collect()
}
