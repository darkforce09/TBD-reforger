//! Envelopes every list endpoint wraps its rows in, and the wire form every partial-change body
//! shares.
//!
//! **Role:** the three shapes that carry a page of results, a single payload, and a cursor-paged
//! list; and `absent_null_or_value`, the field form a `PATCH` body uses to tell an absent key,
//! `null` and a value apart.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests. The patch bodies of the other DTO
//! modules name `absent_null_or_value` in their `serde` attributes.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** field names are the wire contract, so renaming one changes the API. A paginated response
//! always carries its own `total`, `limit` and `offset`; a cursor list carries the opaque cursor to
//! resume from, and `None` means there is no further page.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A page of rows, with the totals needed to render pagination controls.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Paginated<T> {
    /// The rows of this page.
    pub data: Vec<T>,
    /// Total number of the server's deployments.
    pub total: i64,
    /// The page size applied.
    pub limit: i64,
    /// The number of deployments skipped before this page.
    pub offset: i64,
}

/// A single payload wrapped in the `data` key most detail endpoints use.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DataEnvelope<T> {
    /// The rows.
    pub data: Vec<T>,
}

/// A cursor-paged list. `next_cursor` is opaque and absent once the list is exhausted.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorList<T> {
    /// The rows of this page.
    pub data: Vec<T>,
    /// The cursor that fetches the next page; absent on the last page.
    pub next_cursor: Option<Value>,
}

/// The wire form of a patch field that tells an absent key, `null` and a value apart.
///
/// `serde` reads a present `null` into an `Option<Option<T>>` as the outer `None`, the same as an
/// absent key. Reading a present key as `Some(inner)`, and leaving the absent key to
/// `#[serde(default)]`, keeps `null` as `Some(None)`; writing skips the outer `None` through
/// `skip_serializing_if` and writes the inner option as `null` or the value.
pub(crate) mod absent_null_or_value {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Writes a present key: `null` for `Some(None)`, the value for `Some(Some(value))`. The outer
    /// `None` is skipped before this runs, and would write `null` if it were not.
    pub(crate) fn serialize<T, S>(
        field: &Option<Option<T>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        match field {
            Some(inner) => inner.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Reads a present key: `null` becomes `Some(None)`, a value `Some(Some(value))`.
    pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}
