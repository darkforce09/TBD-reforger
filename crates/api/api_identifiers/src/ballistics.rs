//! The stored identifiers of the ballistics catalogs: catalogs, their weapons and shells.
//!
//! **Role:** declares the database-bindable forms of the ballistics catalog keys, for the optional
//! catalog pins of the `fire_missions` rows the API reads.
//! **Position:** declared here, below every API crate; the operations domain stores the catalogs
//! and the fire missions pinned to them. `ballistics_model` declares the same keys for the
//! solver without a database dependency; the API converts between the two through the text.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as the slug or GUID text
//! it wraps (transparent serde, `#[sqlx(transparent)]`), so the catalog documents, the fire
//! mission rows and their JSON stay byte-equal.

newtype_ids::string_id! {
    sqlx,
    /// The lowercase slug naming a ballistics catalog across its versions, such as
    /// `vanilla_mortars`.
    pub struct BallisticsCatalogId;
}

newtype_ids::string_id! {
    sqlx,
    /// The lowercase slug naming a launcher inside its ballistics catalog.
    pub struct BallisticsWeaponId;
}

newtype_ids::string_id! {
    sqlx,
    /// The lowercase slug naming a shell inside its ballistics catalog.
    pub struct BallisticsShellId;
}
