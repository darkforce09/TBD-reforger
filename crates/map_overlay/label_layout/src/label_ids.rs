//! The identifiers of a map label and of a named map location.
//!
//! **Role:** declares [`LabelId`], the caller's stable id of one [`crate::declutter::LabelSpec`],
//! and [`LocationId`], the id of one named location in a terrain's `locations.json`.
//! **Position:** `label_layout`; held by the label specs the map engine's place-name packers
//! build and by the location rows its loader parses.
//! **Signals & state:** none; plain data types.
//! **Invariants:** both ids serialise exactly as the value they wrap, so a location row's JSON
//! is byte-for-byte the JSON of a bare string `id`.

newtype_ids::integer_id! {
    /// The caller's stable id of one label spec: the tie-break of the declutter order, carried
    /// through to the renderer's glyph spec id.
    pub struct LabelId(u32);
}

newtype_ids::string_id! {
    /// The id of one named map location, such as `everon-airport`.
    pub struct LocationId;
}

#[cfg(test)]
#[path = "tests/label_ids_tests.rs"]
mod tests;
