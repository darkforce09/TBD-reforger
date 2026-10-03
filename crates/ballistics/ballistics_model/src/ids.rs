//! The typed identifiers of a ballistics catalog and of what it holds.
//!
//! **Role:** declares [`CatalogId`] (the slug naming a catalog), [`ExportGenerationId`] (the
//! Enfusion GUID of the equipment export a catalog or calibration comes from), [`WeaponId`] (a
//! launcher inside its catalog) and [`ShellId`] (a shell inside its catalog).
//! **Position:** held by [`crate::catalog::BallisticsCatalog`], [`crate::catalog::WeaponSystem`]
//! and [`crate::catalog::Shell`], and by every request, solution, calibration table and
//! agreement case of the crates built on this one.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each identifier serialises as the bare string it wraps, so every catalog,
//! fire-mission and calibration document keeps its JSON bytes; each compares, hashes and orders
//! exactly as its string.

use newtype_ids::string_id;

string_id! {
    /// The lowercase slug naming a ballistics catalog, such as `vanilla_mortars`.
    pub struct CatalogId;
}

string_id! {
    /// The Enfusion GUID of the equipment export generation a catalog's values come from.
    pub struct ExportGenerationId;
}

string_id! {
    /// The lowercase slug naming a launcher inside its catalog.
    pub struct WeaponId;
}

string_id! {
    /// The lowercase slug naming a shell inside its catalog.
    pub struct ShellId;
}
