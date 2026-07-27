//! T-181 — the Enfusion oracle toolchain (`enf` bin).
//!
//! Turns two unreadable code piles into queryable indexes:
//!   * `apps/mod/crf_framework` — 266 `.c` / ~71k LOC of a working Reforger event framework
//!     (Arma Public License, reference only, gitignored).
//!   * the vanilla game's shipped scripts, carved out of `addons/data/*.pak` (T-181.3).
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

/// T-537 / T-383 — refuse structurally empty writes before they overwrite committed indexes.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}

#[cfg(test)]
mod refuse_empty_tests {
    use super::refuse_empty_write;

    #[test]
    fn refuse_empty_write_reds_on_empty() {
        let err = refuse_empty_write("probe", true, "structurally empty").expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(msg.contains("refusing empty write (probe)"), "{msg}");
        assert!(msg.contains("structurally empty"), "{msg}");
    }

    #[test]
    fn refuse_empty_write_ok_when_nonempty() {
        refuse_empty_write("probe", false, "unused").expect("non-empty must pass");
    }
}
