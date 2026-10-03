//! The identifier of one curated road name.
//!
//! **Role:** declares [`RoadNameId`], the `id` of one entry of a terrain's `road-names.json`.
//! **Position:** held by [`crate::route_labels::RoadNameEntry`]; the developer tools and the map
//! engine's label loader read it from the file.
//! **Signals & state:** none; a plain data type.
//! **Invariants:** the id serialises exactly as the string it wraps, so an entry's JSON is
//! byte-for-byte the JSON of a bare string `id`.

newtype_ids::string_id! {
    /// The identifier of one curated road name, such as `main-highway`.
    pub struct RoadNameId;
}
