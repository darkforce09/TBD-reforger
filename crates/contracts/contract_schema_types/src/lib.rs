//! The serde types generated from the contract JSON Schemas.
//!
//! **Role:** holds the Rust form of every schema in `contracts/definitions/` that `typify`
//! converts, one module per API domain and one module per schema below it, such as
//! [`operations::event_hub`].
//! **Position:** contracts tier, depending on no workspace crate. `cargo xtask schema codegen`
//! writes the `generated` module tree; the API's registry import and equipment data viewer
//! decode and re-read through it, and the API's contract tests decode live answers into it.
//! **Signals & state:** none; plain data types with their serde and conversion impls.
//! **Invariants:** nothing under `src/generated/` is edited by hand, and
//! `cargo xtask ci verify-codegen-fresh` proves the tree equals a fresh render of the schemas;
//! every string pattern a schema declares is checked on deserialisation with `regress`.

// The `generated` tree is written by `cargo xtask schema codegen` (typify) and never edited by
// hand, so the lints its output trips are allowed on the tree instead of fixed in it:
// `missing_docs`, because a field or variant carries documentation only when its schema
// describes it; `clippy::unwrap_used`, because each schema `pattern` compiles once into a
// `LazyLock<regress::Regex>` through `unwrap()` on a pattern that is a literal of the schema;
// `clippy::module_inception`, because a schema module holds a definition of the schema's own name
// (`event_hub::event_hub`) and a domain a schema of the domain's name;
// `clippy::derivable_impls`, because typify writes `Default` by hand for a struct whose every
// field takes its type's default.
#[allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::module_inception,
    clippy::derivable_impls
)]
mod generated;
pub mod prelude;

pub use generated::*;
