//! The mortar calculator's inputs: what fires, from where, onto what, through which wind.
//!
//! **Role:** declares the five input groups — weapon and shell, positions, battery, wind and
//! illumination — and the control class they share.
//! **Position:** under the `/tools/mortar` page; each group owns a draft type, its pure parse or
//! resolution, and the fields that edit it. The page owns the signals; the solve bridge reads
//! the resolved values.
//! **Signals & state:** none at this level.
//! **Invariants:** every parse and resolution is a pure function over the draft, so the rules
//! run in native unit tests; a draft that does not parse yields a typed error with a sentence
//! for the page, never a substituted value.

pub(crate) mod battery;
pub(crate) mod illumination;
pub(crate) mod positions;
pub(crate) mod weapon_and_shell;
pub(crate) mod wind;

/// The class every text, number and select control of the inputs wears.
#[cfg(target_arch = "wasm32")]
pub(crate) const INPUT_CLASS: &str =
    "mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm";

#[cfg(test)]
#[path = "../tests/inputs.rs"]
mod tests;
