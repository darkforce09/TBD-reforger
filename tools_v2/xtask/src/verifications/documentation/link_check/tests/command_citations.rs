use std::path::Path;

use clap::{Arg, ArgAction};

use super::super::super::fixture_checkout::{FixtureCheckout, failures, outcome_counts};
use super::super::super::tracked_tree::TrackedTree;
use super::super::markdown_scan::scan;
use super::super::{Break, BreakListing, judge as judge_gate};
use super::*;
use crate::core::repository_layout::documentation::ARCHIVE_DIR;

/// A small command tree shaped like xtask's: groups with verbs, an alias, a group option that
/// takes a value, a leaf with arguments, and a leaf group root.
fn fixture_tree() -> Command {
    let mut tree = Command::new("xtask")
        .disable_help_subcommand(true)
        .subcommand(
            Command::new("ticket")
                .subcommand(
                    Command::new("check")
                        .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue)),
                )
                .subcommand(Command::new("show").arg(Arg::new("id")))
                .subcommand(Command::new("list").alias("ls")),
        )
        .subcommand(
            Command::new("verify")
                .arg(Arg::new("format").long("format").short('f'))
                .arg(
                    Arg::new("quiet")
                        .long("quiet")
                        .short('q')
                        .action(ArgAction::SetTrue),
                )
                .subcommand(Command::new("link-check")),
        )
        .subcommand(Command::new("ci").arg(Arg::new("target")));
    tree.build();
    tree
}

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

/// What the rule makes of one document judged alone over the fixture tree: every break as it
/// renders, and the totals line. The rule reads neither the checkout nor its tracked tree.
fn judge_document(text: &str) -> (Vec<String>, String) {
    let tree = fixture_tree();
    let tracked = TrackedTree::default();
    let context = RuleContext {
        repo_root: Path::new("/nonexistent/command-citations"),
        tree: &tracked,
    };
    let scanned = scan(text);
    let mut rule = CommandCitations::new(&tree);
    let mut findings = RuleFindings::default();
    rule.judge(
        &JudgedDocument {
            path: "apps/tool/README.md",
            scan: &scanned,
        },
        &context,
        &mut findings,
    );
    let breaks = findings.breaks.iter().map(Break::render).collect();
    (breaks, rule.totals().join("\n"))
}

#[test]
fn a_group_and_its_verb_exist() {
    let tree = fixture_tree();
    assert_eq!(
        walk(&tree, "cargo xtask ticket check"),
        CitedCommand::Exists
    );
    assert_eq!(
        walk(&tree, "cargo xtask verify link-check"),
        CitedCommand::Exists
    );
    assert_eq!(walk(&tree, "cargo xtask ticket"), CitedCommand::Exists);
    assert_eq!(walk(&tree, "cargo xtask"), CitedCommand::Exists);
    assert_eq!(walk(&tree, "cargo xtask --help"), CitedCommand::Exists);
}

#[test]
fn an_unknown_group_or_verb_names_the_path_so_far() {
    let tree = fixture_tree();
    assert_eq!(
        walk(&tree, "cargo xtask tickets check"),
        unknown("cargo xtask tickets")
    );
    assert_eq!(
        walk(&tree, "cargo xtask ticket milestone --all"),
        unknown("cargo xtask ticket milestone")
    );
    assert_eq!(
        walk(&tree, "cargo xtask ticket help"),
        unknown("cargo xtask ticket help"),
        "a group whose tree has no help subcommand refuses one"
    );
}

#[test]
fn the_words_after_a_leaf_are_arguments() {
    let tree = fixture_tree();
    assert_eq!(
        walk(&tree, "cargo xtask ticket show T-042 extra words"),
        CitedCommand::Exists
    );
    assert_eq!(walk(&tree, "cargo xtask ci ci-local"), CitedCommand::Exists);
    assert_eq!(
        walk(&tree, "cargo xtask verify link-check anything at all"),
        CitedCommand::Exists
    );
}

#[test]
fn flags_are_skipped_and_a_flag_that_takes_a_value_takes_it_along() {
    let tree = fixture_tree();
    for text in [
        "cargo xtask --verbose ticket check",
        "cargo xtask verify --format json link-check",
        "cargo xtask verify -f json link-check",
        "cargo xtask verify --format=json link-check",
        "cargo xtask verify -q link-check",
        "cargo xtask verify --quiet link-check",
    ] {
        assert_eq!(walk(&tree, text), CitedCommand::Exists, "{text}");
    }
    assert_eq!(
        walk(&tree, "cargo xtask verify --quiet json link-check"),
        unknown("cargo xtask verify json"),
        "a switch takes no value"
    );
}

#[test]
fn a_placeholder_where_a_subcommand_belongs_ends_the_walk() {
    let tree = fixture_tree();
    for text in [
        "cargo xtask ticket <verb>",
        "cargo xtask ticket [verb] T-042",
        "cargo xtask ticket {check,show}",
        "cargo xtask ticket …",
        "cargo xtask ...",
        "cargo xtask <group> <verb>",
    ] {
        assert_eq!(
            walk(&tree, text),
            CitedCommand::ReachesPlaceholder,
            "{text}"
        );
    }
}

#[test]
fn an_alias_names_its_subcommand() {
    let tree = fixture_tree();
    assert_eq!(walk(&tree, "cargo xtask ticket ls"), CitedCommand::Exists);
}

#[test]
fn shell_syntax_ends_the_command() {
    let tree = fixture_tree();
    for text in [
        "cargo xtask ticket | tee out.log",
        "cargo xtask ticket|tee out.log",
        "cargo xtask ticket && echo done",
        "cargo xtask ticket; echo done",
        "cargo xtask ticket > out.log",
        "cargo xtask ticket 2>&1",
        "cargo xtask ticket # every verb",
        "echo $(cargo xtask ticket)",
        "echo \"run cargo xtask ticket\" first",
        "cargo xtask ticket &",
    ] {
        assert_eq!(walk(&tree, text), CitedCommand::Exists, "{text}");
    }
    assert_eq!(
        walk(&tree, "cargo xtask ticket frobnicate;"),
        unknown("cargo xtask ticket frobnicate")
    );
    for text in [
        "(cargo xtask ticket check, wave repack, ticketboard verify)",
        "Run cargo xtask ticket check.",
        "cargo xtask ticket: every verb",
    ] {
        assert_eq!(
            walk(&tree, text),
            CitedCommand::Exists,
            "{text}: prose punctuation ends the command"
        );
    }
    assert_eq!(
        walk(&tree, "cargo xtask ticket frobnicate, then check"),
        unknown("cargo xtask ticket frobnicate")
    );
    assert_eq!(
        walk(&tree, "cargo xtask \"ticket\" 'check'"),
        CitedCommand::Exists,
        "quotes around a word are the shell's, not the word's"
    );
}

#[test]
fn every_citation_in_a_text_is_found_and_hcargo_is_none() {
    assert_eq!(
        citations("cargo xtask db up && cargo  xtask db seed"),
        [" db up && cargo  xtask db seed", " db seed"]
    );
    for text in [
        "hcargo xtask ticket frobnicate",
        "$HOME/.cache/tbd-bin/hcargo xtask ticket frobnicate",
        "cargo-xtask ticket",
        "cargo xtasks ticket",
        "cargo run -p xtask -- ticket",
        "my_cargo xtask ticket",
    ] {
        assert!(citations(text).is_empty(), "{text}");
    }
    assert_eq!(citations("(cargo xtask)"), [")"]);
}

#[test]
fn a_fenced_line_ending_in_a_backslash_continues_on_the_next() {
    let block = CodeBlock {
        line: 10,
        info: "bash".to_string(),
        lines: vec![
            "cargo xtask \\".to_string(),
            "  ticket \\  ".to_string(),
            "  check".to_string(),
            "echo next".to_string(),
            "cargo xtask ticket \\".to_string(),
        ],
    };
    assert_eq!(
        shell_lines(&block),
        [
            (11, "cargo xtask    ticket    check".to_string()),
            (14, "echo next".to_string()),
            (15, "cargo xtask ticket  ".to_string()),
        ]
    );
}

#[test]
fn inline_spans_and_fenced_lines_are_judged_at_their_lines() {
    let text = "# Tool\n\nRun `cargo xtask ticket check` or `cargo xtask tickets`.\n\n\
                ```bash\n# first\ncargo xtask ticket check --strict\ncargo xtask ticket \\\n  \
                frobnicate\n```\n\n~~~text\ncargo xtask verify link-check && cargo xtask nope\n~~~\n\n\
                Plain prose cargo xtask nope is no code.\n";
    let (breaks, totals) = judge_document(text);
    assert_eq!(
        breaks,
        [
            "apps/tool/README.md:3: cited command does not exist: `cargo xtask tickets`",
            "apps/tool/README.md:8: cited command does not exist: `cargo xtask ticket \
             frobnicate`",
            "apps/tool/README.md:13: cited command does not exist: `cargo xtask nope`",
        ]
    );
    assert!(
        totals.contains(
            "6 judged in live documents — 3 name an existing command, 0 reach a placeholder, 3 \
             name no command"
        ),
        "{totals}"
    );
}

#[test]
fn the_xtask_tree_resolves_its_own_commands() {
    let tree = xtask_command_tree();
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

#[test]
fn frozen_records_are_never_judged_by_the_rule() {
    let mut fixture = FixtureCheckout::new("command-citations-frozen");
    let stale = "# Record\n\n`cargo xtask ticket frobnicate`\n";
    fixture
        .tracked(&format!("{ARCHIVE_DIR}/topic/record.md"), stale)
        .tracked("apps/tool/README.md", stale);
    let tree = fixture_tree();
    let mut rules: Vec<Box<dyn DocumentRule + '_>> = vec![Box::new(CommandCitations::new(&tree))];
    let run = judge_gate(
        fixture.root(),
        Ok(fixture.tree()),
        &[],
        BreakListing::Every,
        &mut rules,
    );
    assert_eq!(outcome_counts(&run), (1, 1, 0));
    assert!(failures(&run)[0].starts_with("FAIL: apps/tool/README.md: 1 break(s)"));
    assert!(
        run.totals
            .contains(&"    cited command does not exist: 1".to_string())
    );
}
