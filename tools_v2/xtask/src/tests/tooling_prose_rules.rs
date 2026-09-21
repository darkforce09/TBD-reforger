//! The prose rules over every tracked file under `tools_v2`.
//!
//! Comments, doc comments, help strings and documents describe what the code does now and why: the
//! invariant, the measurement, the refusal reason. They carry no ticket identifiers, no names of
//! files that no longer exist, no deleted script names and no narrative about how the code got
//! here. Commit history owns history, and a reader who cannot see the history is the reader these
//! files are written for.
//!
//! The walk is `git ls-files tools_v2`, so an untracked build tree — an installed `node_modules`
//! among them — never enters, and a rule can never be satisfied by deleting a file from the index
//! while leaving it on disk.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use regex::{Regex, RegexBuilder};

/// A ticket identifier: `T-` and two to four digits, with optional dotted child parts.
const TICKET_IDENTIFIER: &str = r"\bT-[0-9]{2,4}(\.[0-9]+)*\b";

/// Names that no longer describe anything in this repository: retired crate and directory
/// spellings, ticket-numbered file stems, and module names that moved.
///
/// Each is held in halves and joined at runtime, so this file does not match its own needles —
/// the same discipline `ticket_engine::validation::references` uses for its fossil-path guard.
/// [`every_rule_fires_on_a_line_that_breaks_it`] is what proves the joined pattern still bites.
const RETIRED_SPELLINGS: &[(&str, &str)] = &[
    ("tbd", "-tools"),
    ("tbd", "_tools"),
    ("map", "-engine-core"),
    ("map", "_engine_core"),
    ("crates/", "(tbd|map)"),
    ("tools/", "tbd"),
    ("map", "_blueprint"),
    ("schema", "_gates"),
    ("sql", "_gates"),
    ("gate", "_t[0-9]{3}"),
    ("smokes", r"\.rs"),
    ("aux", r"\.rs"),
    ("tests/", "tbds_v2"),
    ("tests/", "edds"),
    ("tests/", "jsval"),
    ("x", "tools"),
    (r"\.tbd", r"-gate\.lock"),
    ("tbd", "_gate_t[0-9]"),
    ("tbd", "-gate-(lock|scan|pg|[0-9])"),
    ("x", "_height_labels"),
    ("lega", "cy_storage"),
    ("lega", "cy_plan"),
    ("t", "159"),
    ("Verify", "T152"),
];

/// Narrative words, in halves for the same reason as [`RETIRED_SPELLINGS`]. A comment that reaches
/// for one of these is describing a past state rather than the present one.
const HISTORY_WORDS: &[(&str, &str)] = &[
    ("former", "ly"),
    ("previous", "ly"),
    ("used to ", "be"),
    ("rewritten ", "from"),
    ("ported ", "from"),
    ("migrated ", "(from|to)"),
    ("renamed ", "from"),
    ("lega", "cy"),
];

fn joined(halves: &[(&str, &str)]) -> String {
    halves
        .iter()
        .map(|(head, tail)| format!("{head}{tail}"))
        .collect::<Vec<_>>()
        .join("|")
}

fn dead_name_pattern() -> Regex {
    Regex::new(&joined(RETIRED_SPELLINGS)).expect("retired spellings")
}

fn history_word_pattern() -> Regex {
    RegexBuilder::new(&format!(r"\b({})\b", joined(HISTORY_WORDS)))
        .case_insensitive(true)
        .build()
        .expect("history words")
}

/// A shell, Python or Node source file name. The tooling ships none of them.
const SCRIPT_FILE_NAME: &str = r"[A-Za-z0-9_./-]+\.(sh|py|mjs|cjs)\b";

/// The one script name that is not a deleted script: `render_agent_files` writes it onto the game
/// host on every staging deploy, systemd socket-activates it there, and
/// `apps/website/api_v2/tests/game_agent_rcon.rs` asserts the file name. Renaming it would change a
/// live remote artifact, so the rule names it instead.
const LIVE_REMOTE_SCRIPT: &str = "tbd-reforger-agent.sh";

/// A repository path literal. Only a layout module may spell one.
const REPOSITORY_PATH_LITERAL: &str = r#""(scripts|docs|\.ai|documentation_v2)/"#;

/// Two literal shapes under a repository-path prefix that name another system, not a location in
/// this checkout: a `.pak` archive's own internal script tree, spelled as a bare prefix, and an
/// Enfusion compiler diagnostic, which cites the addon's script tree lowercase inside `@"…"`.
/// Neither belongs in a layout module, because neither resolves against a checkout root.
fn names_another_system(line: &str) -> bool {
    line.contains("default_value = \"scripts/\"") || line.contains("(E): @\"scripts/")
}

/// The three modules that own every repository path their crate spells.
const LAYOUT_MODULES: [&str; 3] = [
    "tools_v2/ticket-engine/src/repository.rs",
    "tools_v2/xtask/src/core/repository_layout.rs",
    "tools_v2/developer-tools/src/repository_layout.rs",
];

/// Fixture trees whose content is test data rather than prose: synthetic ticket corpora and the
/// captured pages the browser oracle compares against.
const FIXTURE_TREES: [&str; 4] = [
    "tools_v2/developer-tools/fixtures/",
    "tools_v2/developer-tools/test_fixtures/",
    "tools_v2/ticket-engine/tests/fixtures/",
    "tools_v2/ticket-engine/tests/fail/",
];

/// The language-ban tests synthesise the offenders their gates must catch, so they are the one
/// place a script file name is data.
const SYNTHETIC_OFFENDER_TESTS: &str = "tools_v2/xtask/src/verifications/language_bans/tests/";

/// Every tracked file under `tools_v2`, repository-relative.
fn tracked_tooling_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "tools_v2"])
        .output()
        .expect("git ls-files");
    assert!(output.status.success(), "git ls-files tools_v2 failed");
    let listing: Vec<String> = String::from_utf8(output.stdout)
        .expect("git ls-files output is UTF-8")
        .lines()
        .map(str::to_string)
        .collect();
    assert!(
        listing.len() > 500,
        "git ls-files tools_v2 returned {} paths, which is too few to be the tooling tree",
        listing.len()
    );
    listing
}

/// Is this a test source rather than a production one?
fn is_test_source(path: &str) -> bool {
    path.contains("/tests/") || path.ends_with("_tests.rs") || path.ends_with("/tests.rs")
}

/// Is this line a comment, by the only rule a text walk can apply: what it starts with?
fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

fn in_any(path: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| path.starts_with(prefix))
}

/// Read one tracked file, or `None` when it is not UTF-8 text (an image, an archive).
fn read_text(root: &Path, path: &str) -> Option<String> {
    std::fs::read(root.join(path))
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// Every `path:line` where `pattern` matches a line `select` accepts.
fn offences(
    root: &Path,
    files: &[String],
    pattern: &Regex,
    select: &dyn Fn(&str, &str) -> bool,
) -> Vec<String> {
    let mut found = Vec::new();
    for path in files {
        let Some(text) = read_text(root, path) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            if select(path, line) && pattern.is_match(line) {
                found.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    found
}

fn assert_clean(rule: &str, offences: &[String]) {
    assert!(
        offences.is_empty(),
        "{rule}\n{}",
        offences
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn production_sources_carry_no_ticket_identifiers() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = Regex::new(TICKET_IDENTIFIER).expect("ticket identifier");
    let select = |path: &str, _line: &str| {
        path.ends_with(".rs") && !is_test_source(path) && !in_any(path, &FIXTURE_TREES)
    };
    assert_clean(
        "a production source names a ticket identifier; state the fact it was cited for instead:",
        &offences(&root, &files, &pattern, &select),
    );
}

#[test]
fn test_sources_carry_no_ticket_identifiers_in_comments() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = Regex::new(TICKET_IDENTIFIER).expect("ticket identifier");
    let select = |path: &str, line: &str| {
        path.ends_with(".rs")
            && is_test_source(path)
            && !in_any(path, &FIXTURE_TREES)
            && is_comment_line(line)
    };
    assert_clean(
        "a test comment names a ticket identifier (string literals may carry synthetic ids):",
        &offences(&root, &files, &pattern, &select),
    );
}

#[test]
fn documents_carry_no_ticket_identifiers() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = Regex::new(TICKET_IDENTIFIER).expect("ticket identifier");
    let select = |path: &str, _line: &str| {
        (path.ends_with(".md") || path.ends_with(".toml") || path.ends_with(".json"))
            && !in_any(path, &FIXTURE_TREES)
    };
    assert_clean(
        "a document names a ticket identifier:",
        &offences(&root, &files, &pattern, &select),
    );
}

#[test]
fn nothing_names_a_retired_spelling() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = dead_name_pattern();
    let select = |_path: &str, _line: &str| true;
    assert_clean(
        "a retired crate, directory or module spelling survives; use the live name:",
        &offences(&root, &files, &pattern, &select),
    );
}

#[test]
fn nothing_names_a_script_file_the_tooling_does_not_ship() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = Regex::new(SCRIPT_FILE_NAME).expect("script file name");
    let select = |path: &str, line: &str| {
        (path.ends_with(".rs") || path.ends_with(".md") || path.ends_with(".toml"))
            && !path.starts_with(SYNTHETIC_OFFENDER_TESTS)
            && !line.contains(LIVE_REMOTE_SCRIPT)
    };
    assert_clean(
        "a shell, Python or Node file name survives; name the command that does the work:",
        &offences(&root, &files, &pattern, &select),
    );
}

#[test]
fn only_a_layout_module_spells_a_repository_path() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = Regex::new(REPOSITORY_PATH_LITERAL).expect("repository path literal");
    let select = |path: &str, line: &str| {
        path.ends_with(".rs")
            && !is_test_source(path)
            && !LAYOUT_MODULES.contains(&path)
            && !names_another_system(line)
    };
    assert_clean(
        "a production source spells a repository path; put it in its crate's layout module:",
        &offences(&root, &files, &pattern, &select),
    );
}

#[test]
fn nothing_narrates_its_own_history() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = history_word_pattern();
    let select = |_path: &str, _line: &str| true;
    assert_clean(
        "prose narrates a past state; describe the present one:",
        &offences(&root, &files, &pattern, &select),
    );
}

/// Every Rust file name in the workspace, by basename.
fn workspace_rust_file_names(root: &Path) -> BTreeSet<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files"])
        .output()
        .expect("git ls-files");
    assert!(output.status.success(), "git ls-files failed");
    let names: BTreeSet<String> = String::from_utf8(output.stdout)
        .expect("git ls-files output is UTF-8")
        .lines()
        .filter(|path| path.ends_with(".rs"))
        .filter_map(|path| {
            Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into())
        })
        .collect();
    assert!(
        names.len() > 500,
        "the workspace holds {} Rust file names, which is too few",
        names.len()
    );
    names
}

#[test]
fn every_rust_file_named_in_prose_exists() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let existing = workspace_rust_file_names(&root);
    let token = Regex::new(r"\b[a-z0-9_]{2,}\.rs\b").expect("rust file token");
    let mut offences = Vec::new();
    for path in &files {
        if !(path.ends_with(".rs") || path.ends_with(".md"))
            || is_test_source(path)
            || in_any(path, &FIXTURE_TREES)
        {
            continue;
        }
        let Some(text) = read_text(&root, path) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            for name in token.find_iter(line) {
                if !existing.contains(name.as_str()) {
                    offences.push(format!("{path}:{}: {}", number + 1, name.as_str()));
                }
            }
        }
    }
    assert_clean(
        "prose names a Rust file that is nowhere in the workspace:",
        &offences,
    );
}

/// Each rule fires on a line that breaks it, so a green suite means the rules looked and found
/// nothing rather than that they never looked.
#[test]
fn every_rule_fires_on_a_line_that_breaks_it() {
    // Assembled the way the needles are, so the fixture does not put a live offender in this file.
    let fixture = format!(
        "// see {}{} for why\n{} {}{} the old {}{} crate\n{}\n{}{}{} first\n// the whole point of {}{}\n",
        "T-",
        "123.4",
        "//!",
        "ported ",
        "from",
        "map",
        "-engine-core",
        "let path = \"docs/specs/whatever.md\";",
        "// run ",
        "scripts/mod/compile",
        ".sh",
        "hostrun",
        ".rs",
    );
    let lines: Vec<&str> = fixture.lines().collect();
    assert_eq!(lines.len(), 5);

    let ticket = Regex::new(TICKET_IDENTIFIER).expect("ticket identifier");
    assert_eq!(lines.iter().filter(|l| ticket.is_match(l)).count(), 1);
    assert!(is_comment_line(lines[0]));

    let dead = dead_name_pattern();
    assert_eq!(lines.iter().filter(|l| dead.is_match(l)).count(), 1);
    assert_eq!(RETIRED_SPELLINGS.len(), 24);

    let history = history_word_pattern();
    assert_eq!(lines.iter().filter(|l| history.is_match(l)).count(), 1);
    assert_eq!(HISTORY_WORDS.len(), 8);

    let literal = Regex::new(REPOSITORY_PATH_LITERAL).expect("repository path literal");
    assert_eq!(lines.iter().filter(|l| literal.is_match(l)).count(), 1);

    let script = Regex::new(SCRIPT_FILE_NAME).expect("script file name");
    assert_eq!(lines.iter().filter(|l| script.is_match(l)).count(), 1);
    let rendered = format!("writes {LIVE_REMOTE_SCRIPT} onto the host");
    assert!(script.is_match(&rendered));
    assert!(rendered.contains(LIVE_REMOTE_SCRIPT));

    let token = Regex::new(r"\b[a-z0-9_]{2,}\.rs\b").expect("rust file token");
    let named: Vec<&str> = lines
        .iter()
        .flat_map(|line| token.find_iter(line))
        .map(|m| m.as_str())
        .collect();
    assert_eq!(named, [format!("{}{}", "hostrun", ".rs").as_str()]);

    assert!(names_another_system(
        "        #[arg(long, default_value = \"scripts/\")]"
    ));
    assert!(!names_another_system(
        "    let plans = root.join(\"docs/plans\");"
    ));
    assert!(is_test_source(
        "tools_v2/xtask/src/commands/db/tests/operations.rs"
    ));
    assert!(is_test_source("tools_v2/xtask/src/core/runner_tests.rs"));
    assert!(!is_test_source("tools_v2/xtask/src/core/runner.rs"));
    assert!(in_any(
        "tools_v2/ticket-engine/tests/fail/a.rs",
        &FIXTURE_TREES
    ));
    assert!(!in_any(
        "tools_v2/ticket-engine/src/store.rs",
        &FIXTURE_TREES
    ));
}
