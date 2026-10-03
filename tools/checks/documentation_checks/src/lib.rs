//! The documentation gates: README coverage with its Contents check, Markdown placement with its
//! size limit, and the link check.
//!
//! **Role:** one public module per gate ([`readme_coverage`], [`markdown_placement`],
//! [`link_check`]) and the machinery every gate shares: the operator's [`GateRequest`], the tree
//! of files a gate treats as tracked, the `--path` scope, the repository regions, fenced-block
//! recognition, and the run that carries a gate's verdicts to [`verification_core::Report`].
//!
//! **Position:** tier 2 of `tools/checks`, over `verification_core` (verdicts and the report),
//! `process_runner` (the `git ls-files` children), `repository_layout` (the repository regions)
//! and `clap` (the command tree a citation walks). `cargo xtask verify readme-coverage`,
//! `cargo xtask verify markdown-placement` and `cargo xtask verify link-check` reach
//! [`verify_readme_coverage`], [`verify_markdown_placement`] and [`verify_link_check`] through
//! the xtask binary's verify dispatcher, each with a [`GateRequest`]; the ci task table runs the
//! same three over the committed files. The link check judges `cargo xtask` citations against the
//! [`CommandVocabulary`] the binary hands in, so this crate never reads the command line itself.
//! Every path a gate judges comes from `git ls-files` (the index, joined under
//! `--with-untracked` by the untracked files git does not ignore).
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
pub mod markdown_placement;
pub mod prelude;
pub mod readme_coverage;

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
pub use markdown_placement::verify_markdown_placement;
pub use readme_coverage::verify_readme_coverage;
pub use tracked_tree::UntrackedFiles;
