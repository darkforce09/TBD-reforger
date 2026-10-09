//! The repository locations only the Enfusion script oracle names.
//!
//! **Role:** the symbol index folder `enf index` writes, the upstream-framework table inside it, and
//! the hand-authored capability verdict table `enf capability` joins against.
//! **Position:** the `enf` command line takes these as its argument defaults; the locations every
//! tool shares (the reference lanes and their folders) come from the `repository_layout` crate.
//! **Signals & state:** none; constants.
//! **Invariants:** every path is relative to a checkout root; the index folder lies inside the
//! agent artifact tree, so it is pipeline output and never a committed input.

/// The symbol index built over Enfusion sources: one table per lane, read by every citation,
/// lookup and capability query. Pipeline output, not a committed input.
pub const ENF_INDEX_DIR: &str = ".ai/artifacts/enf-index";

/// The upstream-framework symbol table inside [`ENF_INDEX_DIR`], which the lookup, directory
/// census and capability join all read by default.
pub const CRF_SYMBOL_TABLE: &str = ".ai/artifacts/enf-index/crf_symbols.tsv";

/// The hand-authored verdict table `enf capability` joins the upstream symbol index against, so a
/// framework file nobody has triaged is a build error rather than an oversight.
pub const CAPABILITY_VERDICTS: &str = "documentation/mod/tbd-framework/capability_verdicts.tsv";

#[cfg(test)]
#[path = "tests/script_index_layout_tests.rs"]
mod tests;
