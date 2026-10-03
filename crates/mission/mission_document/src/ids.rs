//! The identifiers of the rows only the mission document holds.
//!
//! **Role:** one newtype identifier per document referent the mission model does not name: a
//! faction, a squad, an Editor Layers folder, a placed entity, a vehicle, a comment, a stored
//! composition, a connection, a crew seat, and the `yrs` client a document authors as.
//! **Position:** the leaf of `mission_document`; the document's commands take them alongside the
//! mission model's slot, zone, trigger, marker and mission ids, and the map engine and the Mission
//! Creator construct them from the strings they hold.
//! **Signals & state:** none; plain data types.
//! **Invariants:** every id is serde-transparent and is written into the Yjs maps as its bare
//! string, so the document's maps, exports and every digest stay byte-identical.

newtype_ids::string_id! {
    /// A faction row's identifier (`factionsById`), such as `faction-BLUFOR`.
    pub struct FactionId;
}

newtype_ids::string_id! {
    /// A squad row's identifier (`squadsById`).
    pub struct SquadId;
}

newtype_ids::string_id! {
    /// An Editor Layers folder's identifier (`editorLayersById`).
    pub struct LayerId;
}

newtype_ids::string_id! {
    /// A placed entity's identifier: any row an Editor Layers folder files under `entityIds`
    /// (a slot, a vehicle, a placed object, a comment), and the endpoints connections and trigger
    /// owners name.
    pub struct EntityId;
}

newtype_ids::string_id! {
    /// A vehicle row's identifier (`vehiclesById`).
    pub struct VehicleId;
}

newtype_ids::string_id! {
    /// A comment row's identifier (`commentsById`).
    pub struct CommentId;
}

newtype_ids::string_id! {
    /// A stored composition's identifier (`compositionsById`).
    pub struct CompositionId;
}

newtype_ids::string_id! {
    /// A connection row's identifier (`connectionsById`).
    pub struct ConnectionId;
}

newtype_ids::string_id! {
    /// A vehicle crew seat's key within its vehicle's crew map, such as `gunner`.
    pub struct CrewSeatId;
}

newtype_ids::integer_id! {
    /// The `yrs` client id a document authors its edits as (53 bits).
    pub struct ClientId(u64);
}
