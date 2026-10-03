//! The documentation link check over this binary's own command tree: every command the CLI
//! declares, every build recipe and CI task, and the breaks that name an unknown recipe or task.

use crate::cli::command_vocabulary::documentation_command_vocabulary;
use clap::Command;
use documentation_checks::link_check::{
    CitedCommand, citations, command_words, walk_command, xtask_command_tree,
};

/// What the first citation of `text` walks to in `tree`.
fn walk(tree: &Command, text: &str) -> CitedCommand {
    let rest = citations(text)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("`{text}` cites no command"));
    walk_command(tree, &command_words(rest))
}

fn unknown(path: &str) -> CitedCommand {
    CitedCommand::Unknown(path.to_string())
}

#[test]
fn every_build_recipe_and_ci_task_is_a_value_the_xtask_tree_declares() {
    let tree = xtask_command_tree(&documentation_command_vocabulary());
    for recipe in ci_task_catalog::build_lane::recipes::TARGETS {
        let text = format!("cargo xtask mk {recipe}");
        assert_eq!(walk(&tree, &text), CitedCommand::Exists, "{text}");
    }
    for task in ci_task_catalog::task_runner::TASKS {
        let text = format!("cargo xtask ci {}", task.name);
        assert_eq!(walk(&tree, &text), CitedCommand::Exists, "{text}");
    }
}

#[test]
fn an_unknown_recipe_or_task_names_the_command_and_the_value() {
    let tree = xtask_command_tree(&documentation_command_vocabulary());
    assert_eq!(
        walk(&tree, "cargo xtask mk no-such-recipe"),
        unknown("cargo xtask mk no-such-recipe")
    );
    assert_eq!(
        walk(&tree, "cargo xtask mk --dry-run no-such-recipe"),
        unknown("cargo xtask mk no-such-recipe")
    );
    assert_eq!(
        walk(&tree, "cargo xtask ci no-such-task"),
        unknown("cargo xtask ci no-such-task")
    );
    for text in [
        "cargo xtask mk --dry-run leptos",
        "cargo xtask mk -n rust-api",
        "cargo xtask mk --list",
        "cargo xtask ci --help",
        "cargo xtask ci",
    ] {
        assert_eq!(walk(&tree, text), CitedCommand::Exists, "{text}");
    }
    for text in ["cargo xtask mk <target>", "cargo xtask ci <task>"] {
        assert_eq!(
            walk(&tree, text),
            CitedCommand::ReachesPlaceholder,
            "{text}"
        );
    }
}

#[test]
fn the_xtask_tree_resolves_its_own_commands() {
    let tree = xtask_command_tree(&documentation_command_vocabulary());
    for text in [
        "cargo xtask verify link-check --report",
        "cargo xtask ticket check --strict",
        "cargo xtask db up",
        "cargo xtask deploy db backup",
        "cargo xtask mk leptos",
        "cargo xtask ci ci-local",
        "cargo xtask help",
    ] {
        assert_eq!(walk(&tree, text), CitedCommand::Exists, "{text}");
    }
    assert_eq!(
        walk(&tree, "cargo xtask verify no-such-gate"),
        unknown("cargo xtask verify no-such-gate")
    );
    assert_eq!(
        walk(&tree, "cargo xtask no-such-group verb"),
        unknown("cargo xtask no-such-group")
    );
}
