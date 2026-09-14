//! Envelopes every list endpoint wraps its rows in.
//!
//! **Role:** the three shapes that carry a page of results, a single payload, and a cursor-paged list.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** field names are the wire contract, so renaming one changes the API. A paginated response
//! always carries its own `total`, `limit` and `offset`; a cursor list carries the opaque cursor to
//! resume from, and `None` means there is no further page.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A page of rows, with the totals needed to render pagination controls.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// A single payload wrapped in the `data` key most detail endpoints use.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DataEnvelope<T> {
    pub data: Vec<T>,
}

/// A cursor-paged list. `next_cursor` is opaque and absent once the list is exhausted.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorList<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<Value>,
}
