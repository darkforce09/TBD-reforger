//! The identifiers of the equipment datasets: generations, resources, nodes, native instances
//! and fields.
//!
//! **Role:** declares one typed id per key an equipment dataset generation's files and its
//! SQLite index name things by.
//! **Position:** declared here, below every API crate; the equipment dataset service reads and
//! indexes the generations, and the equipment viewer routes pass the keys through.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner text or
//! integer (transparent serde, `#[sqlx(transparent)]`, generic over the database), so the dataset
//! files, the index rows and the viewer's JSON stay byte-equal.

newtype_ids::string_id! {
    sqlx,
    /// One imported equipment dataset generation, named by its export (`generation_id` of the
    /// manifest); the selector `latest` names the generation the service serves now.
    pub struct EquipmentGenerationId;
}

newtype_ids::string_id! {
    sqlx,
    /// One resource (a prefab or config) of an equipment dataset generation.
    pub struct EquipmentResourceId;
}

newtype_ids::string_id! {
    sqlx,
    /// One node of a resource's snapshot tree: an instance or an ancestor it inherits from.
    pub struct EquipmentNodeId;
}

newtype_ids::string_id! {
    sqlx,
    /// The identity the engine itself gives a node's instance, when it has one.
    pub struct EquipmentNativeInstanceId;
}

newtype_ids::integer_id! {
    sqlx,
    /// One field definition of a gameplay dataset: the integer key of its index's `fields` table.
    pub struct EquipmentFieldId(i64);
}
