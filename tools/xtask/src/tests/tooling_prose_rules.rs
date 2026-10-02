//! The prose rules over every tracked file under `tools`.
//!
//! Comments, doc comments, help strings and documents describe what the code does now and why: the
//! invariant, the measurement, the refusal reason. They carry no ticket identifiers, no names of
//! files that no longer exist, no deleted script names and no narrative about how the code got
//! here. Commit history owns history, and a reader who cannot see the history is the reader these
//! files are written for.
//!
//! One name is exempt from the history vocabulary: the workspace restructure's parking folder,
//! spelled as its path segment or as the string literal naming it ([`PARKING_FOLDER_NAME`]).
//!
//! The walk is `git ls-files tools`, so an untracked build tree — an installed `node_modules`
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
/// The retired crate folders under `crates/` are the hyphenated `tbd-` and `map-` ones; the
/// underscore category folders of the planned crate layout (a map rendering or map overlay
/// category) are live names, so the needle ends at the hyphen.
///
/// Each is held in halves and joined at runtime, so this file does not match its own needles —
/// the same discipline `ticket_engine::validation::references` uses for its fossil-path guard.
/// [`every_rule_fires_on_a_line_that_breaks_it`] is what proves the joined pattern still bites.
const RETIRED_SPELLINGS: &[(&str, &str)] = &[
    ("tbd", "-tools"),
    ("tbd", "_tools"),
    ("map", "-engine-core"),
    ("map", "_engine_core"),
    ("crates/", "(tbd|map)-"),
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

/// The name of the workspace restructure's parking folder, which keeps the spelling `legacy/` by
/// operator decision: the path segment `legacy/` and the string literal `"legacy"` that names the
/// folder. Upper-case identifiers such as `LEGACY_ROOT` and snake-case ones never meet the
/// history needle's word boundary. The bare word, in any case, stays history vocabulary.
const PARKING_FOLDER_NAME: &str = r#"\blegacy/|"legacy""#;

fn parking_folder_name_pattern() -> Regex {
    Regex::new(PARKING_FOLDER_NAME).expect("parking folder name")
}

/// Does `line` narrate history once every spelling of the parking folder's name is blanked? Each
/// spelling becomes a space, so blanking never joins two words into a new match.
fn narrates_history(line: &str, history: &Regex, parking_folder: &Regex) -> bool {
    history.is_match(&parking_folder.replace_all(line, " "))
}

/// A shell, Python or Node source file name. The tooling ships none of them.
const SCRIPT_FILE_NAME: &str = r"[A-Za-z0-9_./-]+\.(sh|py|mjs|cjs)\b";

/// A private-network IPv4 address (10/8, 172.16/12, 192.168/16): a machine on one LAN, whose
/// address moves with its DHCP lease. The deploy host lives in `deploy.env` as `TBD_SSH_HOST`.
const PRIVATE_NETWORK_ADDRESS: &str =
    r"\b(10\.[0-9]{1,3}|172\.(1[6-9]|2[0-9]|3[01])|192\.168)\.[0-9]{1,3}\.[0-9]{1,3}\b";

/// A repository path literal. Only a layout module may spell one.
const REPOSITORY_PATH_LITERAL: &str = r#""(scripts|docs|\.ai|documentation)/"#;

/// Two literal shapes under a repository-path prefix that name another system, not a location in
/// this checkout: a `.pak` archive's own internal script tree, spelled as a bare prefix, and an
/// Enfusion compiler diagnostic, which cites the addon's script tree lowercase inside `@"…"`.
/// Neither belongs in a layout module, because neither resolves against a checkout root.
fn names_another_system(line: &str) -> bool {
    line.contains("default_value = \"scripts/\"") || line.contains("(E): @\"scripts/")
}

/// The three modules that own every repository path their crate spells.
const LAYOUT_MODULES: [&str; 3] = [
    "tools/ticket_engine/src/repository.rs",
    "tools/xtask/src/core/repository_layout.rs",
    "tools/developer_tools/src/repository_layout.rs",
];

/// Fixture trees whose content is test data rather than prose: synthetic ticket corpora and the
/// captured pages the browser oracle compares against.
const FIXTURE_TREES: [&str; 4] = [
    "tools/developer_tools/fixtures/",
    "tools/developer_tools/test_fixtures/",
    "tools/ticket_engine/tests/fixtures/",
    "tools/ticket_engine/tests/fail/",
];

/// The language-ban tests synthesise the offenders their gates must catch, so they are the one
/// place a script file name is data.
const SYNTHETIC_OFFENDER_TESTS: &str = "tools/xtask/src/verifications/language_bans/tests/";

/// Every tracked file under `tools`, repository-relative.
fn tracked_tooling_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "tools"])
        .output()
        .expect("git ls-files");
    assert!(output.status.success(), "git ls-files tools failed");
    let listing: Vec<String> = String::from_utf8(output.stdout)
        .expect("git ls-files output is UTF-8")
        .lines()
        .map(str::to_string)
        .collect();
    assert!(
        listing.len() > 500,
        "git ls-files tools returned {} paths, which is too few to be the tooling tree",
        listing.len()
    );
    listing
}

/// Is this a test source rather than a production one?
fn is_test_source(path: &str) -> bool {
    path.contains("/tests/") || path.ends_with("_tests.rs") || path.ends_with("/tests.rs")
}

/// The prose a line carries: the whole line when it opens a comment, and otherwise everything from
/// its first `//` outside a string literal to the end of the line. A line that carries no comment
/// answers with the empty string.
///
/// A text walk cannot parse Rust, so this tracks quoting one byte at a time and treats a backslash
/// inside a string as an escape. That is what separates a comment a reader reads as prose from a
/// `//` inside a URL or inside the synthetic data a test feeds its subject.
fn comment_text(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
        return trimmed;
    }
    let bytes = line.as_bytes();
    let mut inside_string = false;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if inside_string => index += 1,
            b'"' => inside_string = !inside_string,
            b'/' if !inside_string && bytes.get(index + 1) == Some(&b'/') => return &line[index..],
            _ => {}
        }
        index += 1;
    }
    ""
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
    let mut found = Vec::new();
    for path in &files {
        if !path.ends_with(".rs") || !is_test_source(path) || in_any(path, &FIXTURE_TREES) {
            continue;
        }
        let Some(text) = read_text(&root, path) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            if pattern.is_match(comment_text(line)) {
                found.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    assert_clean(
        "a test comment names a ticket identifier (string literals may carry synthetic ids):",
        &found,
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
    let select = |path: &str, _line: &str| {
        (path.ends_with(".rs") || path.ends_with(".md") || path.ends_with(".toml"))
            && !path.starts_with(SYNTHETIC_OFFENDER_TESTS)
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
    let parking_folder = parking_folder_name_pattern();
    let select = |_path: &str, line: &str| narrates_history(line, &pattern, &parking_folder);
    assert_clean(
        "prose narrates a past state; describe the present one:",
        &offences(&root, &files, &pattern, &select),
    );
}

#[test]
fn no_production_tooling_file_names_a_private_network_address() {
    let root = crate::core::repository_root::test_repo_root();
    let files = tracked_tooling_files(&root);
    let pattern = Regex::new(PRIVATE_NETWORK_ADDRESS).expect("private network address");
    let select = |path: &str, _line: &str| !is_test_source(path) && !in_any(path, &FIXTURE_TREES);
    assert_clean(
        "a production file names a LAN address; name TBD_SSH_HOST or a documentation address:",
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
        "// see {}{} for why\n{} {}{} the old {}{} crate\n{}\n{}{}{}{} first\n// the whole point of {}{}\n",
        "T-",
        "123.4",
        "//!",
        "ported ",
        "from",
        "map",
        "-engine-core",
        "let path = \"docs/specs/whatever.md\";",
        "// run ",
        "scripts",
        "/mod/compile",
        ".sh",
        "hostrun",
        ".rs",
    );
    let lines: Vec<&str> = fixture.lines().collect();
    assert_eq!(lines.len(), 5);

    let ticket = Regex::new(TICKET_IDENTIFIER).expect("ticket identifier");
    assert_eq!(lines.iter().filter(|l| ticket.is_match(l)).count(), 1);
    assert!(ticket.is_match(comment_text(lines[0])));

    // A comment reaches the rule wherever it opens, and data on the same line never does.
    let trailing = format!("    rows.push((sha, 18)); // {}{} sums to 30", "T-", "003");
    assert!(ticket.is_match(comment_text(&trailing)));
    let data_only = format!("    corpus.insert(\"{}{}\", row);", "T-", "003");
    assert!(comment_text(&data_only).is_empty());
    assert!(comment_text("    let base = \"https://host.invalid/x\";").is_empty());

    let dead = dead_name_pattern();
    assert_eq!(lines.iter().filter(|l| dead.is_match(l)).count(), 1);
    assert_eq!(RETIRED_SPELLINGS.len(), 24);

    let history = history_word_pattern();
    assert_eq!(lines.iter().filter(|l| history.is_match(l)).count(), 1);
    assert_eq!(HISTORY_WORDS.len(), 8);

    let private = Regex::new(PRIVATE_NETWORK_ADDRESS).expect("private network address");
    for (octets, expected) in [
        (("192", "168", "0", "129"), true),
        (("10", "0", "0", "1"), true),
        (("172", "20", "1", "2"), true),
        (("172", "32", "1", "2"), false),
        (("192", "0", "2", "10"), false),
        (("198", "51", "100", "7"), false),
    ] {
        let line = format!(
            "ssh deploy@{}.{}.{}.{}",
            octets.0, octets.1, octets.2, octets.3
        );
        assert_eq!(private.is_match(&line), expected, "{line}");
    }
    assert!(!private.is_match("engine 1.7.0.54"));

    let literal = Regex::new(REPOSITORY_PATH_LITERAL).expect("repository path literal");
    assert_eq!(lines.iter().filter(|l| literal.is_match(l)).count(), 1);

    let script = Regex::new(SCRIPT_FILE_NAME).expect("script file name");
    assert_eq!(lines.iter().filter(|l| script.is_match(l)).count(), 1);

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
        "tools/xtask/src/commands/db/tests/operations.rs"
    ));
    assert!(is_test_source("tools/xtask/src/core/runner_tests.rs"));
    assert!(!is_test_source("tools/xtask/src/core/runner.rs"));
    assert!(in_any(
        "tools/ticket_engine/tests/fail/a.rs",
        &FIXTURE_TREES
    ));
    assert!(!in_any("tools/ticket_engine/src/store.rs", &FIXTURE_TREES));
}

/// The parking folder's name, as a path segment or as the string literal that names the folder,
/// is not history vocabulary.
#[test]
fn prose_rules_legacy_folder_name_is_not_history() {
    let history = history_word_pattern();
    let parking_folder = parking_folder_name_pattern();
    for line in [
        "//! no member outside `legacy/` depends on a member under `legacy/`",
        "    workspace.member(\"legacy/website_map_engine\", &manifest);",
        "website_map_engine = { path = \"../../legacy/website_map_engine\" }",
        r#"pub const MANIFEST_SWEEP_ROOTS: &[&str] = &["apps", "crates", "legacy"];"#,
        r#"pub const LEGACY_ROOT: &str = "legacy";"#,
        "    findings.extend(strangler::legacy_dependency_findings(&members));",
    ] {
        assert!(!narrates_history(line, &history, &parking_folder), "{line}");
    }
}

/// The bare word stays history vocabulary, in any case and on a line that also names the folder.
#[test]
fn prose_rules_legacy_folder_word_stays_banned_as_history() {
    let history = history_word_pattern();
    let parking_folder = parking_folder_name_pattern();
    // Assembled from halves, so this file carries no live offender.
    let word = format!("{}{}", "lega", "cy");
    for line in [
        format!("// the {word} implementation"),
        format!("/// nothing new depends on {word}"),
        format!("//! a {word} member sits under `legacy/`"),
        format!("{}{} code paths", "Lega", "cy"),
        format!("let path = \"{word}\\\\tools\";"),
        format!("    let {word}: Vec<&Member> = members;"),
        format!("(apps or {word})"),
    ] {
        assert!(narrates_history(&line, &history, &parking_folder), "{line}");
    }
}

/// Blanking the folder's name leaves a space, so it never fuses two words into a match.
#[test]
fn prose_rules_legacy_folder_blanking_never_joins_words() {
    let parking_folder = parking_folder_name_pattern();
    let fused = format!("{}legacy/{}", "used to ", "be");
    assert_eq!(parking_folder.replace_all(&fused, " "), "used to  be");
    assert!(!narrates_history(
        &fused,
        &history_word_pattern(),
        &parking_folder
    ));
    assert!(!parking_folder.is_match(&format!("{}{}/", "LEG", "ACY")));
}

/// The retired crate needle still bites on the hyphenated retired folders and leaves the planned
/// underscore categories alone.
#[test]
fn prose_rules_retired_crate_folders_fire_and_planned_categories_pass() {
    let dead = dead_name_pattern();
    for (head, tail) in [
        ("map", "-engine-core"),
        ("map", "-engine-render"),
        ("tbd", "-gate"),
        ("tbd", "-tickets"),
    ] {
        assert!(
            dead.is_match(&format!("{}{head}{tail}", "crates/")),
            "{head}{tail}"
        );
    }
    for planned in ["map_rendering", "map_overlay", "mission", "geometry"] {
        let line = format!("{}{planned}", "crates/");
        assert!(!dead.is_match(&line), "{line}");
    }
}
