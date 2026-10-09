//! The command vocabulary the documentation link check judges `cargo xtask` citations against.
//!
//! **Role:** [`documentation_command_vocabulary`] hands the link check this binary's own clap
//! command tree, the build recipe names `mk` looks up and the CI task names `ci` looks up.
//! **Position:** the binary's side of the injection: `verify link-check` passes the vocabulary to
//! the gate, so the documentation checks never read the command line themselves.
//! **Signals & state:** none; it builds a value per call.
//! **Invariants:** the tree is [`Cli`]'s as clap derives it, and the names are the live recipe and
//! task tables, so a cited command is judged against exactly what this binary accepts.

use clap::CommandFactory;

use super::Cli;
use ci_task_catalog::build_lane::recipes;
use ci_task_catalog::task_runner;
use documentation_checks::link_check::CommandVocabulary;

/// xtask's clap command tree, as clap derives it from [`Cli`] before parsing.
pub(crate) fn xtask_command_tree() -> clap::Command {
    Cli::command()
}

/// The vocabulary the documentation link check judges command citations against: this binary's
/// command tree, the `mk` recipe names and the `ci` task names.
pub(crate) fn documentation_command_vocabulary() -> CommandVocabulary {
    CommandVocabulary {
        command_tree: xtask_command_tree,
        recipe_targets: recipes::TARGETS.to_vec(),
        task_names: task_runner::TASKS.iter().map(|task| task.name).collect(),
    }
}
