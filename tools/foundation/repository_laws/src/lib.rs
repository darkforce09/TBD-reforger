//! The repository's structural laws as pure checks over a checkout.
//!
//! **Role:** answers, for a repository root, whether the tree keeps the structural laws: the size
//! advice for production files ([`file_length`]) and the workspace laws over the members the
//! root manifest names ([`workspace_members`], [`workspace_laws`]): crate tiers (the application
//! boundary and the external-crate firewalls), crate anatomy, test-file reachability, frontend
//! layering and Tailwind sources.
//!
//! **Position:** tier 1 of `tools/foundation`, over `verification_core` (the outcome
//! vocabulary, the scans and the patterns) and `regex`. `cargo xtask verify file-length` renders
//! these results as its gate output, as do the five workspace-law gates
//! (`cargo xtask verify crate-tiers` and its siblings). It reads files and nothing else: no
//! process, no network, no environment variable.
//!
//! **Signals & state:** none. Every public function takes the repository root and returns
//! findings, or [`verification_core::NotRun`] when an input the rule needs is missing or
//! unreadable.
//!
//! **Invariants:** a rule that could not read its input never reports a pass — a missing root
//! or an unreadable file is [`verification_core::NotRun`], and a walk that found nothing is
//! reported as a count of zero for the caller to refuse. The size advice reads every workspace
//! member folder plus [`source_roots::PINNED_SCRIPT_ROOTS`].

pub mod cargo_manifest;
mod error;
pub mod file_length;
pub mod prelude;
pub mod source_roots;
pub mod workspace_laws;
pub mod workspace_members;

pub use error::{Error, Result};

#[cfg(test)]
#[path = "tests/temporary_checkout.rs"]
mod temporary_checkout;
