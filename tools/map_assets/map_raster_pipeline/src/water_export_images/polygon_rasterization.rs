//! The scanline fill of the export's lake and pond polygons.
//!
//! **Role:** [`fill_water_polygons`] fills every lake or pond ring of a vector list into the
//! water raster: each covered sample takes the polygon's water class and is deepened to the
//! polygon's depth there.
//! **Position:** called by the vector rasterizer for the lakes, then the ponds, before the
//! rivers; the depth rule reads the optional [`DemHeightfield`].
//! **Signals & state:** none held; writes the caller's [`WaterRaster`].
//! **Invariants:** the ring is the first given of `polygon`, `perimeter` and `points`, and an item
//! whose ring is not an array of at least three points draws nothing and is not counted; a point
//! is `[x, y, z]` and the ring lies in plan as `(x, z)`, a missing or non-number coordinate
//! reading as NaN; the rows scanned run from the floor to the ceiling of the z extent (`bounds.min`
//! and `bounds.max` when both are given, otherwise the ring's own), and a polygon whose rows miss
//! the grid is skipped uncounted; on each row the even-odd crossings pair up and each pair fills
//! the columns it touches (see [`SampleGrid::span_columns`]); a sample's depth is `avgDepthM`,
//! else `0.6 × maxDepthM`, else 1.5 m, and with a heightfield whose terrain lies below the surface
//! `surfaceElevationYM` (default 0) it is `min(max, max(0.1, surface − terrain))`, the cap `max`
//! being `maxDepthM`, else 1.5 × the default depth; these depth fields count when they are JSON
//! numbers, 0 and negatives included.

use grid_rasterization::prelude::{SampleGrid, row_crossings};
use serde_json::Value;

use super::dem_sampling::DemHeightfield;
use super::json_field_reading::{coordinate, first_given, is_given, json_number};
use super::water_raster::{WaterRaster, nan_propagating_max, nan_propagating_min};

/// The ring fields of a lake or pond, preferred first.
const RING_FIELDS: [&str; 3] = ["polygon", "perimeter", "points"];

/// The fewest points a ring needs to be filled.
const MINIMUM_RING_POINTS: usize = 3;

/// The depth of a lake or pond that gives neither `avgDepthM` nor `maxDepthM`, in metres.
const FALLBACK_POLYGON_DEPTH_M: f64 = 1.5;

/// The share of `maxDepthM` a lake or pond without `avgDepthM` takes as its depth.
const MAXIMUM_TO_DEFAULT_DEPTH_RATIO: f64 = 0.6;

/// The cap on a heightfield depth, as a multiple of the default depth, without `maxDepthM`.
const DEFAULT_TO_MAXIMUM_DEPTH_RATIO: f64 = 1.5;

/// The shallowest depth a heightfield gives a sample below the surface, in metres.
const MINIMUM_HEIGHTFIELD_DEPTH_M: f64 = 0.1;

/// The depth rule of one lake or pond.
struct PolygonDepth {
    /// The water surface height in metres.
    surface_y: f64,
    /// The depth of a sample the heightfield does not deepen.
    default_depth_m: f64,
    /// The deepest a heightfield depth may be.
    maximum_depth_m: f64,
}

impl PolygonDepth {
    /// The depth rule `item` gives.
    fn of(item: &Value) -> Self {
        let average = json_number(item.get("avgDepthM"));
        let maximum = json_number(item.get("maxDepthM"));
        let default_depth_m = average.unwrap_or_else(|| {
            maximum.map_or(FALLBACK_POLYGON_DEPTH_M, |maximum| {
                maximum * MAXIMUM_TO_DEFAULT_DEPTH_RATIO
            })
        });
        Self {
            surface_y: json_number(item.get("surfaceElevationYM")).unwrap_or(0.0),
            default_depth_m,
            maximum_depth_m: maximum.unwrap_or(default_depth_m * DEFAULT_TO_MAXIMUM_DEPTH_RATIO),
        }
    }

    /// The depth of the sample at world `(x, z)`.
    fn at(&self, x: f64, z: f64, heightfield: Option<&DemHeightfield>) -> f64 {
        let Some(heightfield) = heightfield else {
            return self.default_depth_m;
        };
        let terrain_y = heightfield.elevation_m(x, z);
        if terrain_y < self.surface_y {
            nan_propagating_min(
                self.maximum_depth_m,
                nan_propagating_max(MINIMUM_HEIGHTFIELD_DEPTH_M, self.surface_y - terrain_y),
            )
        } else {
            self.default_depth_m
        }
    }
}

/// Fills every polygon of `items` (a JSON array; anything else draws nothing) into `raster` as
/// water class `class_code`; returns how many polygons were drawn. See the module invariants.
pub(super) fn fill_water_polygons(
    items: &Value,
    class_code: u8,
    heightfield: Option<&DemHeightfield>,
    raster: &mut WaterRaster,
) -> usize {
    let Some(items) = items.as_array() else {
        return 0;
    };
    let grid = raster.grid;
    let mut drawn = 0;
    for item in items {
        let Some(points) = first_given(item, &RING_FIELDS).and_then(Value::as_array) else {
            continue;
        };
        if points.len() < MINIMUM_RING_POINTS {
            continue;
        }
        let ring: Vec<[f64; 2]> = points
            .iter()
            .map(|point| [coordinate(Some(point), 0), coordinate(Some(point), 2)])
            .collect();
        let (min_z, max_z) = z_extent(item, &ring);
        let Some(rows) = grid.row_range(min_z, max_z) else {
            continue;
        };
        let depth = PolygonDepth::of(item);
        for row in rows {
            fill_row(&grid, &ring, row, class_code, &depth, heightfield, raster);
        }
        drawn += 1;
    }
    drawn
}

/// Fills the spans of `ring` on `row`.
fn fill_row(
    grid: &SampleGrid,
    ring: &[[f64; 2]],
    row: usize,
    class_code: u8,
    depth: &PolygonDepth,
    heightfield: Option<&DemHeightfield>,
    raster: &mut WaterRaster,
) {
    let z = grid.row_z(row);
    let crossings = row_crossings(ring, z);
    for pair in crossings.chunks_exact(2) {
        let Some(columns) = grid.span_columns(pair[0], pair[1]) else {
            continue;
        };
        for column in columns {
            let x = grid.column_x(column);
            let index = raster.index(column, row);
            raster.mask[index] = class_code;
            raster.deepen(index, depth.at(x, z, heightfield));
        }
    }
}

/// The z extent `(min_z, max_z)` of a polygon: from `bounds.min` and `bounds.max` (`[x, y, z]`)
/// when both are given, otherwise the extremes of `ring`, where a NaN never becomes an extreme (a
/// ring of NaN gives `(+∞, −∞)`).
fn z_extent(item: &Value, ring: &[[f64; 2]]) -> (f64, f64) {
    let bounds = item.get("bounds");
    let corner = |key: &str| {
        bounds
            .and_then(|bounds| bounds.get(key))
            .filter(|corner| is_given(corner))
    };
    if let (Some(min), Some(max)) = (corner("min"), corner("max")) {
        return (coordinate(Some(min), 2), coordinate(Some(max), 2));
    }
    let (mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY);
    for &[_, z] in ring {
        if z < min_z {
            min_z = z;
        }
        if z > max_z {
            max_z = z;
        }
    }
    (min_z, max_z)
}

#[cfg(test)]
#[path = "../tests/water_export_images_polygon_tests.rs"]
mod tests;
