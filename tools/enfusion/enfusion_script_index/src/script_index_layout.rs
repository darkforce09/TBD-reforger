//! The repository locations only the Enfusion script oracle names.
//!
//! **Role:** the symbol index folder `enf index` writes, the upstream-framework table inside it, and
//! the hand-authored capability verdict table `enf capability` joins against.
//! **Position:** the `enf` command line takes these as its argument defaults; the locations every
//! tool shares (the reference lanes and their folders) come from the `repository_layout` crate.
//! **Signals & state:** none; constants.
//! **Invariants:** every path is relative to a checkout root; the index folder is committed beside
//! the mod, because the reference lanes it indexes are licensed and never committed.

/// The symbol index built over the Enfusion reference lanes: one table per lane (names and
/// coordinates, no bodies), read by every citation, lookup and capability query. Committed, so a
/// checkout without the licensed lanes still answers lookups; `enf index` rewrites it.
pub const ENF_INDEX_DIR: &str = "mod/reference_symbol_index";

/// The upstream-framework symbol table inside [`ENF_INDEX_DIR`], which the lookup, directory
/// census and capability join all read by default.
pub const CRF_SYMBOL_TABLE: &str = "mod/reference_symbol_index/crf_symbols.tsv";

/// The hand-authored verdict table `enf capability` joins the upstream symbol index against, so a
/// framework file nobody has triaged is a build error rather than an oversight.
pub const CAPABILITY_VERDICTS: &str = "documentation/mod/tbd-framework/capability_verdicts.tsv";
