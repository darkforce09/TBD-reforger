//! The identifiers of the community content domain: announcements, modpacks, vehicle databases
//! and wiki pages.
//!
//! **Role:** declares one typed id per community content table key, plus the Workshop item id
//! and addon GUID a modpack's mod line names.
//! **Position:** declared here, below every API crate; community content owns the tables, and
//! the operations, missions and server code that names a modpack takes the same type.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner UUID, text or
//! integer (transparent serde, `#[sqlx(transparent)]`), so the wire shapes and the SQL stay those
//! of the bare value.

newtype_ids::uuid_id! {
    sqlx,
    /// A published announcement: the key of `announcements`.
    pub struct AnnouncementId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A modpack manifest: the key of `modpacks`.
    pub struct ModpackId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One mod of a modpack: the key of `modpack_mods`.
    pub struct ModpackModId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One vehicle of the vehicle database: the key of `vehicle_databases`.
    pub struct VehicleDatabaseId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A wiki article: the key of `wiki_pages`.
    pub struct WikiPageId;
}

newtype_ids::string_id! {
    sqlx,
    /// A mod's Arma Reforger Workshop item id, as text; empty when the mod line names none.
    #[derive(Default)]
    pub struct WorkshopItemId;
}

newtype_ids::string_id! {
    sqlx,
    /// The GUID a mod's addon declares, as text; empty when the mod line names none.
    #[derive(Default)]
    pub struct ModGuid;
}

impl WorkshopItemId {
    /// True when the mod line names no Workshop item (the text is empty).
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

impl ModGuid {
    /// True when the mod line names no addon GUID (the text is empty).
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}
