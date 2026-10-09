//! The recipes this lane reproduces, as text.
//!
//! **Role:** the lane's recipe lines rendered as text, and a reader of a `Makefile` recipe body.
//! **Position:** a child of [`crate::local_database`]; [`super::selftest`]'s arms 1 and 2 compare
//! these renderings with the frozen baseline and a `Makefile`.
//! **Signals & state:** none; pure functions of the lane's constants and argument builders.
//! **Invariants:** every rendering derives from the constants and builders the runners use, so what
//! the lane runs and what it claims to run cannot drift apart.
//!
//! Split out of [`super`] because the text outlives any `Makefile`: `selftest`'s arm 1 diffs
//! these renderings against the pinned recipe text it holds, and arm 2 diffs them against a
//! `Makefile` should one return to the checkout.
//!
//! Everything here is derived from the same consts and argument builders the runners use
//! ([`super::WEB`], [`super::SEEDS`], [`super::IT_BASE_DB`],
//! [`super::recipe_execution::seed_psql_arguments`], [`super::development_compose::ComposeLine`],
//! `super::reap_select`), so a change to what the
//! port RUNS necessarily changes what it CLAIMS to run — the two cannot drift apart quietly, which
//! is the failure mode a hand-copied "expected output" table always ends in.

use super::development_compose::ComposeLine;
use super::recipe_execution::seed_psql_arguments;
use super::test_it::reap_select;
use super::{IT_BASE_DB, IT_MAINT_DB, SEEDS, WEB, seed_file};

/// Every line the lane echoes, rendered with `podman` as the runtime and `crates/api/api_server` as the API
/// folder ([`WEB`]). The compose lines come from the [`ComposeLine`] the runner echoes with: they
/// enter the development compose file's folder and name the file with `-f`.
///
/// The four `deploy db` wrappers are deliberately absent: their recipes are make's own
/// `cargo run -q -p xtask -- deploy db …` transport, which the port replaces with an in-process
/// call rather than reproducing. Their argv mapping is proved by running both sides — see the
/// slice's acceptance notes — not by a text pin of a command the port never issues.
pub(crate) fn rendered_recipes() -> Vec<(&'static str, Vec<String>)> {
    let compose = ComposeLine::of_checkout();
    let seed_arguments = seed_psql_arguments();
    let seed_arguments: Vec<&str> = seed_arguments.iter().map(String::as_str).collect();
    let seed_lines: Vec<String> = SEEDS
        .iter()
        .map(|f| compose.render("podman", &seed_arguments, Some(&seed_file(f))))
        .collect();
    vec![
        (
            "db-up",
            vec![compose.render("podman", &["up", "-d", "db"], None)],
        ),
        ("db-down", vec![compose.render("podman", &["down"], None)]),
        (
            "db-logs",
            vec![compose.render("podman", &["logs", "-f", "db"], None)],
        ),
        ("seed", seed_lines),
        (
            "rust-test-it",
            vec![
                format!(
                    "podman exec tbd_reforger_db psql -U tbd -d {IT_MAINT_DB} -qc \"DROP DATABASE IF EXISTS {IT_BASE_DB} WITH (FORCE);\""
                ),
                format!(
                    "podman exec tbd_reforger_db psql -U tbd -d {IT_MAINT_DB} -qc \"CREATE DATABASE {IT_BASE_DB};\""
                ),
                format!(
                    "cd {WEB} && TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/{IT_BASE_DB}?sslmode=disable cargo test"
                ),
                format!(
                    "podman exec tbd_reforger_db psql -U tbd -d {IT_MAINT_DB} -Atc \"{}\"",
                    reap_select(IT_BASE_DB)
                ),
            ],
        ),
    ]
}

/// The body of one recipe, by target name: every line from the target header to the next one.
///
/// Two make-isms are stripped because they are directives to make, not part of the command:
/// `@` (do not echo) and `-` (ignore the exit status). Both are load-bearing elsewhere in this
/// port — `rust-test-it`'s first `DROP` carries the `-` and its reap block carries the `@` — but
/// what this function returns is the SHELL command, which is what the port's renderings are.
///
/// Comment lines inside a recipe (`\t@# …`) are skipped: make hands them to a shell that does
/// nothing with them, and `rust-test-it` has three.
///
/// A backslash-continued line is joined into ONE entry, because that is what make does: the whole
/// continuation goes to a single shell as a single logical command. `rust-test-it`'s reap is four
/// physical lines and one command, and treating them as four would compare a pipeline against its
/// own first fragment. Exactly ONE leading tab is removed from each physical line — make's own
/// rule, and the reason `make -n` prints the continuation lines still indented.
pub(crate) fn recipe_body(makefile: &str, target: &str) -> Vec<String> {
    let mut body: Vec<String> = Vec::new();
    let mut in_recipe = false;
    let header = format!("{target}:");
    for line in makefile.lines() {
        if line.starts_with(&header) {
            in_recipe = true;
            continue;
        }
        if !in_recipe {
            continue;
        }
        // A line starting with anything other than a comment or whitespace ends the recipe —
        // i.e. the next target. The three characters are spelled out rather than expressed as a
        // bracket class, where a backslash escape would be ambiguous.
        if line
            .chars()
            .next()
            .is_some_and(|c| c != '#' && c != ' ' && c != '\t')
        {
            break;
        }
        let Some(rest) = line.strip_prefix('\t') else {
            continue;
        };
        // Continuation of the previous logical line: keep the newline and the (already
        // tab-stripped) indent, which is byte-for-byte what `make -n` echoes.
        if let Some(prev) = body.last_mut()
            && prev.ends_with('\\')
        {
            prev.push('\n');
            prev.push_str(rest);
            continue;
        }
        let rest = rest.trim_start_matches(['@', '-']);
        if rest.starts_with('#') {
            continue;
        }
        body.push(rest.to_string());
    }
    body
}

/// Expand the two make variables the db lane uses, the way `make` itself would.
///
/// This is NOT a make implementation and must never become one: `$(WEB)` and `$(COMPOSE)` are the
/// only variables in these recipes (Makefile:2-3), and `$(COMPOSE)` resolves to `podman compose`
/// on any host without docker — which is every machine this repo has been measured on.
///
/// Anything else of the form `$(…)` is deliberately left UNEXPANDED so the comparison it feeds
/// fails loudly. A silent pass-through would let a new variable make the pin compare two strings
/// that no longer describe the same command.
/// `$$` is also expanded — it is how a recipe writes a literal `$` for the shell (`$$db` in the
/// reap loop is the shell variable `$db`), so leaving it would compare make's source against the
/// shell's view of it.
pub(crate) fn expand_make_vars(line: &str) -> String {
    line.replace("$(WEB)", WEB)
        .replace("$(COMPOSE)", "podman compose")
        .replace("$$", "$")
}

#[cfg(test)]
#[path = "tests/recipes/tests.rs"]
mod tests;
