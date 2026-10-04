//! The keys the catalog and outliner trees name their nodes by.
//!
//! **Role:** one newtype key per tree the crate builds: the right dock's catalog palette trees
//! and the Editor Layers outliner (with the ORBAT tree that shares its node model).
//! **Position:** built by [`crate::asset_catalog`] and [`crate::outliner_model`]; read by the
//! palette, the outliner, the ORBAT manager and the favourites store, which route a node by its
//! key and its kind.
//! **Signals & state:** none; plain data types.
//! **Invariants:** every key is serde-transparent and spelled exactly as the tree builder spells
//! it, so persisted favourites and collapsed sets keep their bytes.

newtype_ids::string_id! {
    /// A catalog tree node's key, unique within its palette tree: a folder's `/`-joined path of
    /// folder captions, or a leaf's asset id (the prefab it places).
    pub struct CatalogNodeId;
}

newtype_ids::string_id! {
    /// An outliner node's key. What it names follows the node's
    /// [`crate::outliner_model::NodeKind`]: an Editor Layers folder id, a slot, faction or squad
    /// document id, a comment key, or [`crate::outliner_model::UNFILED_ID`] for the virtual
    /// Unfiled root.
    pub struct OutlinerNodeId;
}
