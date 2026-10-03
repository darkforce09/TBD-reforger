//! Why a place-name source cannot be read or baked.
//!
//! **Role:** the crate's one error type and its `Result` alias: a label JSON file that does not
//! parse, a height label row without its coordinates, a curated road name the labels archive
//! cannot carry, and the [`BinaryError`] the labels archive reports.
//! **Position:** returned by the JSON parsers of [`crate::towns`] and [`crate::route_labels`], by
//! [`crate::route_labels::road_names_to_archive`] and by [`crate::towns::map_labels_from_bytes`].
//! **Signals & state:** none; plain data.
//! **Invariants:** each message names the file or archive lane it comes from; the archive variant
//! wraps the format error transparently.

use world_file_formats::archives::codec::BinaryError;

/// Why a place-name file or archive is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `locations.json` is not an array of location rows.
    #[error("locations json: {0}")]
    LocationsJson(#[source] serde_json::Error),

    /// `height-labels.json` is not JSON.
    #[error("height-labels json: {0}")]
    HeightLabelsJson(#[source] serde_json::Error),

    /// `height-labels.json` parses, but its root is not an array.
    #[error("height-labels json: payload is not an array")]
    HeightLabelsNotAnArray,

    /// A `height-labels.json` row lacks a numeric `x`, `y` or `value_m`.
    #[error("height-labels json: row {row} missing x/y/value_m")]
    HeightLabelRowIncomplete {
        /// The row's index in the file.
        row: usize,
    },

    /// `road-names.json` does not match its model.
    #[error("road-names json: {0}")]
    RoadNamesJson(#[source] serde_json::Error),

    /// A curated road name without its own zoom floor sits on a road class that never draws.
    #[error("road-names archive: \"{name}\" is class {road_class} which is never drawn")]
    RoadClassNeverDrawn {
        /// The road name.
        name: String,
        /// The class of the segment it is placed on.
        road_class: String,
    },

    /// A curated road name's zoom floor matches no road class's floor, so the archive's one-byte
    /// class code cannot carry it.
    #[error(
        "road-names archive: \"{name}\" needs visibility floor {floor}, which no road class \
         expresses — RoadNameLabel cannot carry it"
    )]
    UnrepresentableVisibilityFloor {
        /// The road name.
        name: String,
        /// The zoom floor the name asks for.
        floor: f64,
    },

    /// The labels archive is misaligned, malformed or of another schema version.
    #[error(transparent)]
    Binary(#[from] BinaryError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
