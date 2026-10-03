//! The vanilla reference page mirrors behind `cargo xtask fetch`.
//!
//! **Role:** mirrors the two public references of the vanilla Arma Reforger scripts into the
//! vanilla reference lane: the Script API pages ([`vanilla_api`]) and the per-file source pages
//! ([`vanilla_source`]), each into its own cache folder.
//! **Position:** called by xtask's `fetch` command line, which picks the checkout root and passes
//! the raw arguments; `enf apidoc` and `enf source` parse what these cache.
//! **Signals & state:** reads `TBD_FETCH_DELAY` and `TBD_FETCH_VANILLA_API_CURL`; writes only
//! inside the vanilla lane.
//! **Invariants:** the references folder is never created here; a cached page that is non-empty
//! is never fetched again; every fetch is a `curl` run through `process_runner`.

mod reference_cache;
pub mod vanilla_api;
pub mod vanilla_source;
