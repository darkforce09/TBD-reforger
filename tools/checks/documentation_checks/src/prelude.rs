//! The names a caller imports with `use documentation_checks::prelude::*;`: the link check's
//! entry point, the request it takes and the vocabulary it judges citations against.

pub use crate::{BreakListing, CommandVocabulary, GateRequest, UntrackedFiles, verify_link_check};
