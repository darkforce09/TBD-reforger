//! Successful test cases are deduplicated within their Cargo executable or documentation suite.

use std::{collections::BTreeSet, sync::LazyLock};

use regex::Regex;

static CARGO_RUNNER_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?m)^[ \t]*(?:Running (?:unittests [^\r\n()]+\.rs|tests/[^\r\n()]+\.rs) ",
        r"\((?P<binary>(?:target/|/)[^\r\n()]*/deps/[^/\r\n() ]+)\)",
        r"|Doc-tests (?P<documentation>[A-Za-z_][A-Za-z0-9_-]*))\r?$",
    ))
    .expect("Cargo runner header pattern is valid")
});

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CaseScope<'a> {
    Unscoped,
    Executable(&'a str),
    Documentation(&'a str),
}

/// Match the entire output so multiline and multiple-per-line success patterns remain intact.
/// A match belongs to the last recognized runner header at or before its starting byte offset.
pub(super) fn successful_cases(pattern: &Regex, output: &str) -> u64 {
    let mut scopes = vec![(0, CaseScope::Unscoped)];
    for header in CARGO_RUNNER_HEADER.captures_iter(output) {
        let scope = if let Some(binary) = header.name("binary") {
            // The executable path identifies a Cargo binary independently of its display label.
            CaseScope::Executable(binary.as_str())
        } else {
            CaseScope::Documentation(
                header
                    .name("documentation")
                    .expect("documentation header has a crate name")
                    .as_str(),
            )
        };
        scopes.push((header.get(0).unwrap().start(), scope));
    }

    let mut current_scope = 0;
    let mut unique = BTreeSet::new();
    for matched in pattern.find_iter(output) {
        while current_scope + 1 < scopes.len() && scopes[current_scope + 1].0 <= matched.start() {
            current_scope += 1;
        }
        unique.insert((&scopes[current_scope].1, matched.as_str()));
    }
    unique.len() as u64
}

#[cfg(test)]
#[path = "tests/case_count.rs"]
mod tests;
