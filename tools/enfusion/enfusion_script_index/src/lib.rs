//! The Enfusion script oracle: queryable symbol indexes over the mod's reference scripts.
//!
//! **Role:** turns two unreadable code piles — the upstream framework lane in
//! `apps/mod/References/crf_framework` and the vanilla game's shipped scripts — into TSV indexes
//! of symbol names and `file:line` coordinates ([`index`], [`symbols`]), answers lookups against
//! them, checks the `@idx` citations in `documentation/` ([`citations`]) and the framework
//! capability verdicts ([`capability`]), extracts, carves and reconstructs vanilla sources
//! ([`carve`], [`source`], [`apidoc`]), and mirrors the vanilla reference pages
//! ([`vanilla_page_fetch`]). [`run_command_line`] is the `enf` binary.
//! **Position:** tier 2 of `tools/enfusion`, over `enfusion_pak`, `repository_layout`,
//! `content_digest`, `process_runner` and `verification_core`. The `enf` binary of
//! `developer_tools` calls [`run_command_line`]; xtask's `fetch` command calls
//! [`vanilla_page_fetch`]; the mod wave gate runs this crate's unit tests.
//! **Signals & state:** none held; each command reads its inputs, writes its outputs and returns.
//! **Invariants:** an index carries names and coordinates only, never code bodies, so it can be
//! committed while the sources stay gitignored; a committed index is never overwritten with an
//! empty one; a reference lane is written only inside the references folder; every failure is an
//! [`Error`] and only the binary decides the exit code.

pub mod apidoc;
pub mod capability;
pub mod carve;
pub mod citations;
mod command_line;
mod empty_write_guard;
mod error;
pub mod index;
pub mod prelude;
pub mod reference_output;
pub mod script_index_layout;
pub mod source;
pub mod symbols;
pub mod vanilla_page_fetch;

pub use command_line::run_command_line;
pub use error::{Error, Result};
