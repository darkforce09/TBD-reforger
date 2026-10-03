//! The names a caller imports with `use documentation_checks::prelude::*;`: each gate's entry
//! point, the request every gate takes and the vocabulary the link check judges citations against.

pub use crate::{
    BreakListing, CommandVocabulary, GateRequest, UntrackedFiles, verify_link_check,
    verify_markdown_placement, verify_readme_coverage,
};
