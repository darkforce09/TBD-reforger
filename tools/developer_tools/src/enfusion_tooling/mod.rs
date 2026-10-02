//! The Enfusion oracle toolchain (the `enf` binary).
//!
//! Turns two unreadable code piles into queryable indexes:
//!   * `apps/mod/crf_framework` — 266 `.c` / ~71k LOC of a working Reforger event framework
//!     (Arma Public License, reference only, gitignored).
//!   * the vanilla game's shipped scripts, carved out of `addons/data/*.pak`.
//!
//! Indexes are TSV so an agent greps them with `rg` at zero parse cost, and they carry only
//! symbol names and coordinates — never code bodies — so they can be committed without
//! vendoring anything.

use anyhow::{Result, bail};

pub mod apidoc;
pub mod capability;
pub mod carve;
pub mod citations;
pub mod index;
pub mod source;
pub mod symbols;

/// Refuse a structurally empty write before it overwrites a committed index.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/module/refuse_empty_tests.rs"]
mod refuse_empty_tests;

pub mod cli;

pub mod enfusion_mcp_entrypoint;
pub mod mcp_broker;
