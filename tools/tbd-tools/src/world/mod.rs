//! T-165.8 — the world-export pipeline (ports of scripts/map-assets/{decode-topo, decode-edds,
//! build-world-objects, build-roads-from-topo, verify-phase, validate-export-artifacts, …}.mjs).

use anyhow::{Result, bail};

pub mod aux;
pub mod build;
pub mod classify;
pub mod edds;
pub mod gates;
pub mod jsval;
pub mod pak;
pub mod topo;

/// T-537 / T-383 — refuse structurally empty / vacuous overwrites of committed map-assets.
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
