//! The road-class codec: the closed table of road classes and its one-byte wire code.
//!
//! **Role:** maps a road class name to the `road_class` byte the road network archive and the
//! labels archive store, and back.
//! **Position:** read by the road network decoder ([`crate::network`]), the map engine's road
//! name labels and store tests, and the developer tools' road export.
//! **Signals & state:** none; a constant table and pure functions.
//! **Invariants:** a class's code is its index in [`ROAD_CLASSES`] plus one; `0` is the unknown
//! class, and every code outside the table decodes to the empty name.

/// The closed road-class table, in `roads::road_style_width` order. The index **+1** is the `road_class` byte on the wire; `0` is "unknown class", which no gate admits.
pub const ROAD_CLASSES: [&str; 6] = [
    "highway_paved",
    "road_paved",
    "road_dirt",
    "track",
    "path",
    "runway",
];

/// Wire code for a road class (`0` when the class is not in [`ROAD_CLASSES`]).
#[must_use]
pub fn road_class_code(road_class: &str) -> u8 {
    ROAD_CLASSES
        .iter()
        .position(|c| *c == road_class)
        .map_or(0, |i| (i + 1) as u8)
}

/// Class name for a wire code (`""` for `0` / out of range).
#[must_use]
pub fn road_class_name(code: u8) -> &'static str {
    match usize::from(code)
        .checked_sub(1)
        .and_then(|i| ROAD_CLASSES.get(i))
    {
        Some(c) => c,
        None => "",
    }
}
