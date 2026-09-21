//! Executable statements of this crate's prose rules, checked against the text of `src/`,
//! `tests/`, `.env.example`, the seeds, and the comment lines of the migrations.
//!
//! Comments, docstrings, fixtures and templates describe the code as it stands. A ticket id is a
//! pointer into a registry the reader of this crate does not have; a comparison to another
//! implementation dates on the day that implementation stops existing; the vocabulary of a
//! delivery process (waves, slices, gates, perturbations) means nothing to a reader who was not
//! in it; and a path that no longer exists sends the reader looking for a file this repository
//! does not contain. Each rule reads the tree itself, so a new file is covered the moment it is
//! added — nothing has to be registered here.
//!
//! **Excluded scopes.** `missions/contract/generated/` is emitted by `cargo xtask ci
//! schema-codegen`, so its prose belongs to the generator; this file and
//! `architecture_rules.rs` hold every forbidden token as a literal needle. SQL files are read on
//! their `--` comment lines only: a migration's statements carry data values (a quarantine
//! table's provenance column, a seeded title) that are not prose.

use std::path::{Path, PathBuf};

/// The rule files, relative to `src/` — excluded from their own scans.
const RULE_FILES: [&str; 2] = ["tests/architecture_rules.rs", "tests/prose_rules.rs"];

/// Codegen output, relative to `src/`.
const GENERATED_SUBTREE: &str = "missions/contract/generated";

/// Floors for each walk: a scan that returns fewer files than this has lost the tree, and every
/// rule over it would pass vacuously.
const SOURCE_FLOOR: usize = 100;
const SUITE_FLOOR: usize = 40;
const SEED_FLOOR: usize = 5;
const MIGRATION_FLOOR: usize = 20;

/// Prose that describes this crate in terms of another implementation.
const OTHER_IMPLEMENTATION: [&str; 11] = [
    "Rust port",
    "Go API",
    "Go's",
    "Go:",
    "Go-style",
    "Go service",
    "Go time",
    "Go `",
    "internal/",
    "GORM",
    "parsePage",
];

/// Vocabulary of the delivery process that produced the code, not of the code.
const PROCESS_VOCABULARY: [&str; 14] = [
    "ticket",
    "wave gate",
    "this wave",
    "the wave",
    "this slice",
    "Class-R",
    "perturbation:",
    "RED perturbation",
    "bait comment",
    "differential",
    "proof-ledger",
    ".ai/artifacts",
    "Makefile",
    "wave.sh",
];

/// Paths of files this crate no longer has: the layer-split modules and the merged suites.
const RETIRED_PATHS: [&str; 16] = [
    "src/config.rs",
    "src/db.rs",
    "src/app.rs",
    "src/state.rs",
    "src/realtime.rs",
    "handlers/oauth.rs",
    "handlers/admin.rs",
    "handlers/events.rs",
    "handlers/missions.rs",
    "handlers/telemetry.rs",
    "handlers/me.rs",
    "services/mortar",
    "services/discord.rs",
    "ratelimit_gc",
    "misc_integration",
    "null_tolerance.rs",
];

/// One scanned line: where it is, and its text as the rule should read it.
struct Line {
    location: String,
    text: String,
}

/// The crate directory, resolved from the manifest so the tests do not depend on the working
/// directory a test runner happens to use.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.rs` file under `root`, sorted by path, skipping the excluded scopes.
fn rust_files(root: &Path, skip_generated: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(root, "rs", &mut files);
    files.retain(|path| {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        !(skip_generated && relative.starts_with(GENERATED_SUBTREE))
            && !RULE_FILES.contains(&relative.as_str())
    });
    files.sort();
    files
}

/// Every file with `extension` under `dir`, depth-first.
fn collect(dir: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path());
    for path in entries {
        if path.is_dir() {
            collect(&path, extension, files);
        } else if path.extension().is_some_and(|ext| ext == extension) {
            files.push(path);
        }
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn location(path: &Path, index: usize) -> String {
    let relative = path.strip_prefix(crate_root()).unwrap_or(path);
    format!("{}:{}", relative.display(), index + 1)
}

/// Every line of every `.rs` file under `src/` and `tests/`.
fn rust_lines() -> Vec<Line> {
    let root = crate_root();
    let sources = rust_files(&root.join("src"), true);
    let suites = rust_files(&root.join("tests"), false);
    assert_floor(sources.len(), SOURCE_FLOOR, "src/");
    assert_floor(suites.len(), SUITE_FLOOR, "tests/");
    sources
        .iter()
        .chain(suites.iter())
        .flat_map(|path| lines_of(path, |_| true))
        .collect()
}

/// Every line of `.env.example`.
fn env_template_lines() -> Vec<Line> {
    lines_of(&crate_root().join(".env.example"), |_| true)
}

/// The `--` comment lines of every seed and migration.
fn sql_comment_lines() -> Vec<Line> {
    let root = crate_root();
    let mut seeds = Vec::new();
    collect(&root.join("seeds"), "sql", &mut seeds);
    let mut migrations = Vec::new();
    collect(&root.join("migrations"), "sql", &mut migrations);
    assert_floor(seeds.len(), SEED_FLOOR, "seeds/");
    assert_floor(migrations.len(), MIGRATION_FLOOR, "migrations/");
    seeds.sort();
    migrations.sort();
    seeds
        .iter()
        .chain(migrations.iter())
        .flat_map(|path| lines_of(path, |line| line.trim_start().starts_with("--")))
        .collect()
}

fn lines_of(path: &Path, keep: impl Fn(&str) -> bool) -> Vec<Line> {
    read(path)
        .lines()
        .enumerate()
        .filter(|(_, line)| keep(line))
        .map(|(index, line)| Line {
            location: location(path, index),
            text: line.to_string(),
        })
        .collect()
}

fn assert_floor(count: usize, floor: usize, scope: &str) {
    assert!(
        count >= floor,
        "the {scope} walk returned only {count} file(s), fewer than the floor of {floor} — \
         the walker has lost the tree and every rule over it would pass vacuously"
    );
}

/// All prose the rules read: the Rust files, the environment template, the SQL comments.
fn all_prose() -> Vec<Line> {
    let mut lines = rust_lines();
    lines.extend(env_template_lines());
    lines.extend(sql_comment_lines());
    lines
}

/// Fail with every offending `path:line — reason` listed, one per line.
fn assert_no_offenders(rule: &str, lines: &[Line], mut hit: impl FnMut(&str) -> Option<String>) {
    let offenders: Vec<String> = lines
        .iter()
        .filter_map(|line| hit(&line.text).map(|reason| format!("{} — {reason}", line.location)))
        .collect();
    assert!(
        offenders.is_empty(),
        "{rule}: {} violation(s)\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// `T-` followed by a digit at the start of a word, so `DEFAULT-0` does not read as an id.
fn names_a_ticket(line: &str) -> bool {
    let bytes = line.as_bytes();
    (0..bytes.len().saturating_sub(2)).any(|at| {
        bytes[at] == b'T'
            && bytes[at + 1] == b'-'
            && bytes[at + 2].is_ascii_digit()
            && (at == 0 || !bytes[at - 1].is_ascii_alphanumeric())
    })
}

/// `.go` not continued by another word character, so `.google` does not read as a filename.
fn names_a_go_file(line: &str) -> bool {
    let mut rest = line;
    while let Some(at) = rest.find(".go") {
        let after = &rest[at + 3..];
        if !after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            return true;
        }
        rest = after;
    }
    false
}

/// `apps/website/api` not continued by `_` — the crate's former directory.
fn names_the_retired_crate_path(line: &str) -> bool {
    let mut rest = line;
    while let Some(at) = rest.find("apps/website/api") {
        let after = &rest[at + "apps/website/api".len()..];
        if !after.starts_with('_') {
            return true;
        }
        rest = after;
    }
    false
}

/// The word `ticket` on its own: `tickets`, `ticketed` and `ticket_` count, `sticker` does not.
fn names_the_ticket_process(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(at) = rest.find("ticket") {
        let before = rest[..at].chars().next_back();
        if !before.is_some_and(|c| c.is_alphanumeric()) {
            return true;
        }
        rest = &rest[at + "ticket".len()..];
    }
    false
}

#[test]
fn no_ticket_ids_in_prose() {
    assert_no_offenders("no_ticket_ids_in_prose", &all_prose(), |line| {
        names_a_ticket(line)
            .then(|| "ticket id; say what the code does, not which ticket changed it".to_string())
    });
}

#[test]
fn no_other_implementation_narrative() {
    assert_no_offenders("no_other_implementation_narrative", &all_prose(), |line| {
        if let Some(phrase) = OTHER_IMPLEMENTATION.iter().find(|p| line.contains(**p)) {
            return Some(format!("`{phrase}` — describe this crate, not another one"));
        }
        names_a_go_file(line).then(|| "names a `.go` file — this crate has none".to_string())
    });
}

#[test]
fn no_delivery_process_vocabulary() {
    assert_no_offenders("no_delivery_process_vocabulary", &all_prose(), |line| {
        if let Some(phrase) = PROCESS_VOCABULARY
            .iter()
            .filter(|p| **p != "ticket")
            .find(|p| line.contains(**p))
        {
            return Some(format!(
                "`{phrase}` — the process that produced the code is not the code"
            ));
        }
        names_the_ticket_process(line)
            .then(|| "`ticket` — the process that produced the code is not the code".to_string())
    });
}

#[test]
fn no_retired_paths() {
    assert_no_offenders("no_retired_paths", &all_prose(), |line| {
        if names_the_retired_crate_path(line) {
            return Some("names `apps/website/api`, which is `apps/website/api_v2`".to_string());
        }
        RETIRED_PATHS.iter().find(|p| line.contains(**p)).map(|p| {
            format!("`{p}` — a file this crate no longer has; name the module that holds it now")
        })
    });
}
