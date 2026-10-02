//! The `jsonb` passthrough column type.

/// `jsonb` passthrough: sqlx decodes the column into a `RawValue` and serde re-emits
/// it verbatim (Postgres-normalized bytes, no re-serialization).
pub type RawJson = sqlx::types::Json<Box<serde_json::value::RawValue>>;
