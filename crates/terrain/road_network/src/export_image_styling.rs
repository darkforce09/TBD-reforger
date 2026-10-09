//! How the road export images draw each road class.
//!
//! **Role:** the stroke colour, world width and minimum pixel width of each class
//! ([`road_export_image_style`]), the per-class layer files ([`ROAD_EXPORT_LAYER_FILES`]), the
//! order classes are drawn in ([`ROAD_EXPORT_DRAW_ORDER`]), the dark background, and the junction
//! marker's colour, size and degree threshold.
//! **Position:** read by the map raster pipeline's road export image lane, which strokes the
//! Workbench road export onto a transparent canvas, a dark canvas and one layer canvas per class.
//! It sits beside [`crate::styling`], the map's own road styling, and shares only the class names
//! of [`crate::road_class::ROAD_CLASSES`] with it.
//! **Signals & state:** none; constant tables and pure lookups.
//! **Invariants:** every class of [`crate::road_class::ROAD_CLASSES`] has exactly one style, one
//! layer file and one place in the draw order; highways draw last so they sit on top, runways
//! first so every road crosses over them.

/// How the road export images stroke one road class.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadExportImageStyle {
    /// The stroke colour, `[r, g, b, a]`.
    pub rgba: [u8; 4],
    /// The road width in world metres, used when a segment carries no width of its own.
    pub width_m: f64,
    /// The narrowest stroke in pixels, however far the image is zoomed out.
    pub min_width_px: f64,
}

/// The export image style of `road_class`, or `None` for a class outside
/// [`crate::road_class::ROAD_CLASSES`].
#[must_use]
pub fn road_export_image_style(road_class: &str) -> Option<RoadExportImageStyle> {
    let (rgba, width_m, min_width_px) = match road_class {
        "highway_paved" => ([255, 175, 45, 255], 8.0, 3.5),
        "road_paved" => ([240, 242, 248, 255], 6.0, 2.5),
        "road_dirt" => ([215, 140, 75, 255], 4.5, 2.0),
        "track" => ([145, 205, 60, 255], 3.5, 1.5),
        "path" => ([50, 220, 240, 255], 2.0, 1.2),
        "runway" => ([255, 80, 120, 255], 25.0, 6.0),
        _ => return None,
    };
    Some(RoadExportImageStyle {
        rgba,
        width_m,
        min_width_px,
    })
}

/// Each road export layer as `(file stem, road class)`: the export reads `<file stem>.json` and
/// writes `layer-<file stem>.png`, in the order of [`crate::road_class::ROAD_CLASSES`].
pub const ROAD_EXPORT_LAYER_FILES: [(&str, &str); 6] = [
    ("highways", "highway_paved"),
    ("roads_paved", "road_paved"),
    ("roads_dirt", "road_dirt"),
    ("tracks", "track"),
    ("paths", "path"),
    ("runways", "runway"),
];

/// The file stem of `road_class`'s export layer (see [`ROAD_EXPORT_LAYER_FILES`]), or `None` for
/// a class outside [`crate::road_class::ROAD_CLASSES`].
#[must_use]
pub fn road_export_layer_file_stem(road_class: &str) -> Option<&'static str> {
    ROAD_EXPORT_LAYER_FILES
        .iter()
        .find(|(_, class)| *class == road_class)
        .map(|(file_stem, _)| *file_stem)
}

/// The road classes in the order the export draws them, later classes over earlier ones: runways
/// at the bottom, highways on top.
pub const ROAD_EXPORT_DRAW_ORDER: [&str; 6] = [
    "runway",
    "road_paved",
    "road_dirt",
    "track",
    "path",
    "highway_paved",
];

/// The dark image's background, `[r, g, b]`, which every pixel's alpha blends over.
pub const ROAD_EXPORT_DARK_BACKGROUND_RGB: [u8; 3] = [18, 22, 28];

/// The junction marker's colour, `[r, g, b]`.
pub const ROAD_JUNCTION_RGB: [u8; 3] = [255, 225, 100];

/// The junction marker's alpha.
pub const ROAD_JUNCTION_ALPHA: u8 = 220;

/// The junction marker's radius in pixels.
pub const ROAD_JUNCTION_RADIUS_PX: f64 = 2.0;

/// The fewest connected segments a junction needs to be marked.
pub const ROAD_JUNCTION_MIN_DEGREE: u32 = 3;

#[cfg(test)]
#[path = "tests/export_image_styling_tests.rs"]
mod tests;
