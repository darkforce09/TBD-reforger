//! The documentation link check.
//!
//! **Role:** the public [`link_check`] gate and the machinery it uses: the operator's
//! [`GateRequest`], the tree of files the gate treats as tracked, the `--path` scope, the
//! repository regions, fenced-block recognition, and the run that carries the gate's verdicts to
//! [`verification_core::Report`].
//!
//! **Position:** tier 2 of `tools/checks`, over `verification_core` (verdicts and the report),
//! `process_runner` (the `git ls-files` children), `repository_layout` (the repository regions)
//! and `clap` (the command tree a citation walks). `cargo xtask verify link-check` reaches
//! [`verify_link_check`] through the xtask binary's verify dispatcher with a [`GateRequest`]. The
//! link check judges `cargo xtask` citations against the [`CommandVocabulary`] the binary hands
//! in, so this crate never reads the command line itself. Every path the gate judges comes from
//! `git ls-files` (the index, joined under `--with-untracked` by the untracked files git does not
//! ignore).
//!
//! **Signals & state:** none held; a run lists the files once, judges them, prints its verdicts
//! and returns its exit status.
//!
//! **Invariants:** a gate that could not list the files, could not read a file it judges, or
//! whose scope selects nothing reports "did not run" (exit 2), never a pass; exit 1 means at
//! least one judged item broke a rule; exit 0 means every judged item held. A run that included
//! untracked files says so on its header and its summary line, so its result never passes for a
//! check of the committed files.

pub mod link_check;
pub mod prelude;

mod gate_run;
mod gate_scope;
mod markdown_fences;
mod path_regions;
mod tracked_tree;

#[cfg(test)]
#[path = "tests/fixture_checkout.rs"]
mod fixture_checkout;

pub use gate_run::GateRequest;
pub use link_check::{BreakListing, CommandVocabulary, verify_link_check};
pub use tracked_tree::UntrackedFiles;
