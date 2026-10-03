//! The names a call site imports with `use api_failpoints::prelude::*;`: the macro, which expands
//! to nothing in a build without the `failpoints` feature.

pub use crate::fail_point;
