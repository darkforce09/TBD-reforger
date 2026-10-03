//! The engineering rules of `CLAUDE.md` law 6 and law 7 as pure checks over a checkout.
//!
//! **Role:** answers, for a repository root, whether the tree keeps the structural laws: every
//! production file at or under 500 lines and every test file at or under 1000
//! ([`file_length`]); unit tests only in sibling files, never in an inline test-module body
//! ([`sibling_test_placement`]); no exemption mechanism for either rule
//! ([`exemption_mechanisms`]); the engine layer walls ([`engine_layers`]); the dependency
//! direction between the website crates ([`crate_dependencies`]); and the workspace laws over the
//! members the root manifest names ([`workspace_members`], [`workspace_laws`]): crate tiers,
//! crate anatomy, the strangler rule, frontend layering and Tailwind sources.
//!
//! **Position:** tier 1 of `tools/foundation`, over `verification_core` (the outcome
//! vocabulary, the scans and the patterns) and `regex`. `cargo xtask verify file-length` and
//! `cargo xtask verify engine-layers` render these results as their gate output, as do the five
//! workspace-law gates (`cargo xtask verify crate-tiers` and its siblings), and the
//! `engineering_laws` test binary of `api` asserts on them directly. It reads files and
//! nothing else: no process, no network, no environment variable.
//!
//! **Signals & state:** none. Every public function takes the repository root and returns
//! findings, or [`verification_core::NotRun`] when an input the rule needs is missing or
//! unreadable.
//!
//! **Invariants:** a rule that could not read its input never reports a pass — a missing root
//! or an unreadable file is [`verification_core::NotRun`], and a walk that found nothing is
//! reported as a count of zero for the caller to refuse. Every law reads the same roots
//! (every workspace member folder plus [`source_roots::PINNED_SCRIPT_ROOTS`]), so the
//! gates and the test binary can never disagree about what the tree is.

pub mod cargo_manifest;
pub mod crate_dependencies;
pub mod engine_layers;
mod error;
pub mod exemption_mechanisms;
pub mod file_length;
pub mod prelude;
pub mod sibling_test_placement;
pub mod source_roots;
pub mod workspace_laws;
pub mod workspace_members;

pub use error::{Error, Result};

#[cfg(test)]
#[path = "tests/temporary_checkout.rs"]
mod temporary_checkout;
