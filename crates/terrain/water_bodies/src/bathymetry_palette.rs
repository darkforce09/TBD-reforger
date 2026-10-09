//! The colours of water depth on the Workbench water export images.
//!
//! **Role:** the water classes a rasterized water mask holds ([`WaterClass`]), the depth colour
//! ramps of the sea, of lakes and ponds and of rivers ([`OCEAN_DEPTH_STOPS`],
//! [`LAKE_DEPTH_STOPS`], [`RIVER_DEPTH_STOPS`]), the dark land colour ([`DARK_LAND_RGB`]), the
//! colour at a depth ([`interpolate_depth_stops`], [`bathymetry_rgb`]) and the darkening of the sea's
//! depth contour lines ([`contour_multiplier`]).
//! **Position:** read by the map raster pipeline's water export image lane, which rasterizes the
//! Workbench water export into a mask and a depth grid and colours them with this palette.
//! **Signals & state:** none; constant tables and pure functions.
//! **Invariants:** each ramp starts at 0 m and its depths rise strictly; a depth at or below the
//! first stop takes the first colour and at or above the last stop (or NaN) the last colour;
//! between stops each channel is interpolated linearly and rounded to the nearest integer, a tie
//! rounding up; contour lines darken the sea only, never a lake, pond or river.

/// What a water mask sample holds; the discriminant is the mask byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WaterClass {
    /// Dry ground.
    Land = 0,
    /// The sea.
    Ocean = 1,
    /// A lake or a pond.
    LakeOrPond = 2,
    /// A river.
    River = 3,
}

impl WaterClass {
    /// The mask byte of this class.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The class of mask byte `code`, or `None` for a byte no class has.
    #[must_use]
    pub fn from_code(code: u8) -> Option<WaterClass> {
        match code {
            0 => Some(WaterClass::Land),
            1 => Some(WaterClass::Ocean),
            2 => Some(WaterClass::LakeOrPond),
            3 => Some(WaterClass::River),
            _ => None,
        }
    }
}

/// The class whose colours paint a sample of mask byte `code`, a sample known to be water: 2 is a
/// lake or pond, 3 a river, and every other byte the sea.
#[must_use]
pub fn palette_class_for_mask_code(code: u8) -> WaterClass {
    match code {
        2 => WaterClass::LakeOrPond,
        3 => WaterClass::River,
        _ => WaterClass::Ocean,
    }
}

/// One stop of a depth colour ramp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthStop {
    /// The depth below the surface in metres.
    pub depth_m: f64,
    /// The colour at that depth, `[r, g, b]`.
    pub rgb: [u8; 3],
}

impl DepthStop {
    /// The stop of colour `rgb` at `depth_m` metres.
    #[must_use]
    pub const fn new(depth_m: f64, rgb: [u8; 3]) -> Self {
        Self { depth_m, rgb }
    }
}

/// The sea's ramp, from the pale shore to the dark abyss.
pub const OCEAN_DEPTH_STOPS: [DepthStop; 9] = [
    DepthStop::new(0.0, [160, 232, 242]),
    DepthStop::new(2.0, [110, 215, 235]),
    DepthStop::new(6.0, [65, 185, 222]),
    DepthStop::new(15.0, [45, 155, 210]),
    DepthStop::new(30.0, [35, 130, 195]),
    DepthStop::new(60.0, [24, 98, 175]),
    DepthStop::new(100.0, [16, 68, 145]),
    DepthStop::new(150.0, [10, 42, 110]),
    DepthStop::new(210.0, [4, 16, 65]),
];

/// The ramp of lakes and ponds, shallow teal to deep teal.
pub const LAKE_DEPTH_STOPS: [DepthStop; 4] = [
    DepthStop::new(0.0, [95, 225, 205]),
    DepthStop::new(2.0, [50, 195, 175]),
    DepthStop::new(6.0, [30, 155, 140]),
    DepthStop::new(18.0, [16, 110, 100]),
];

/// The ramp of rivers, sky blue to the deep channel.
pub const RIVER_DEPTH_STOPS: [DepthStop; 4] = [
    DepthStop::new(0.0, [110, 205, 255]),
    DepthStop::new(1.5, [60, 165, 250]),
    DepthStop::new(4.0, [30, 125, 225]),
    DepthStop::new(25.0, [15, 85, 180]),
];

/// The colour of dry ground on the dark water image, `[r, g, b]`.
pub const DARK_LAND_RGB: [u8; 3] = [22, 26, 31];

/// The sea depths, in metres, whose contour lines are darkened.
const CONTOUR_INTERVALS_M: [f64; 7] = [5.0, 10.0, 20.0, 50.0, 100.0, 150.0, 200.0];

/// How close to a contour depth, in metres, a sample lies on its line.
const CONTOUR_HALF_WIDTH_M: f64 = 0.18;

/// The brightness factor of a sample on a contour line.
const CONTOUR_SHADE: f64 = 0.78;

/// The colour of `depth_m` on the ramp `stops` (depths rising): the first colour at or below the
/// first stop, the last colour at or above the last stop or for NaN, and between two stops each
/// channel interpolated linearly and rounded to the nearest integer. An empty ramp is black.
///
/// The interpolated channel lies between two `u8` values, so it is never negative and
/// [`f64::round`] rounds a tie up exactly as ties toward +∞ would.
#[must_use]
pub fn interpolate_depth_stops(stops: &[DepthStop], depth_m: f64) -> [u8; 3] {
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
        return [0, 0, 0];
    };
    if depth_m <= first.depth_m {
        return first.rgb;
    }
    if depth_m >= last.depth_m {
        return last.rgb;
    }
    for pair in stops.windows(2) {
        let (lower, upper) = (pair[0], pair[1]);
        if depth_m >= lower.depth_m && depth_m <= upper.depth_m {
            let t = (depth_m - lower.depth_m) / (upper.depth_m - lower.depth_m);
            let channel = |index: usize| {
                let from = f64::from(lower.rgb[index]);
                let to = f64::from(upper.rgb[index]);
                // Between two bytes, so the cast is exact.
                (from + t * (to - from)).round() as u8
            };
            return [channel(0), channel(1), channel(2)];
        }
    }
    last.rgb
}

/// The colour of water `depth_m` deep of `class`: the lake ramp for [`WaterClass::LakeOrPond`],
/// the river ramp for [`WaterClass::River`] and the sea ramp otherwise.
#[must_use]
pub fn bathymetry_rgb(depth_m: f64, class: WaterClass) -> [u8; 3] {
    match class {
        WaterClass::LakeOrPond => interpolate_depth_stops(&LAKE_DEPTH_STOPS, depth_m),
        WaterClass::River => interpolate_depth_stops(&RIVER_DEPTH_STOPS, depth_m),
        WaterClass::Land | WaterClass::Ocean => {
            interpolate_depth_stops(&OCEAN_DEPTH_STOPS, depth_m)
        }
    }
}

/// The brightness factor of a sample `depth_m` deep of `class`: 0.78 when a sea sample lies
/// within 0.18 m of a multiple of 5, 10, 20, 50, 100, 150 or 200 m, otherwise 1.0; lakes, ponds
/// and rivers are always 1.0.
///
/// The nearest multiple comes from [`f64::round`]. Only an exact tie (`depth_m / interval` a half
/// integer) can round differently from ties toward +∞, and a tie lies half an interval (at least
/// 2.5 m) from both neighbouring multiples, so the factor is the same under either rule.
#[must_use]
pub fn contour_multiplier(depth_m: f64, class: WaterClass) -> f64 {
    if matches!(class, WaterClass::LakeOrPond | WaterClass::River) {
        return 1.0;
    }
    let on_contour = CONTOUR_INTERVALS_M.iter().any(|&interval| {
        (depth_m - (depth_m / interval).round() * interval).abs() < CONTOUR_HALF_WIDTH_M
    });
    if on_contour { CONTOUR_SHADE } else { 1.0 }
}

#[cfg(test)]
#[path = "tests/bathymetry_palette_tests.rs"]
mod tests;
