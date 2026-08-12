//! T-894 — the Makefile recipes this lane reproduces, as text.
//!
//! Split out of `mk_db.rs` for one reason: this is the half of the port that must OUTLIVE the
//! Makefile. `selftest`'s arm 2 diffs these renderings against the live `Makefile` today; when
//! T-897 deletes that file, arm 1 keeps diffing them against a frozen copy of what `make -n`
//! printed on 2026-08-12, and this module is the thing on the other side of that comparison.
//!
//! Everything here is derived from the same consts the runners use ([`super::WEB`],
//! [`super::SEEDS`], [`super::IT_BASE_DB`], [`super::reap_select`]), so a change to what the port
//! RUNS necessarily changes what it CLAIMS to run — the two cannot drift apart quietly, which is
//! the failure mode a hand-copied "expected output" table always ends in.

use super::test_it::reap_select;
use super::{IT_BASE_DB, IT_MAINT_DB, SEEDS, WEB};

/// Every recipe line the port reproduces, rendered with make's own variable values
/// (`$(WEB)` → `apps/website/api`, `$(COMPOSE)` → `podman compose` on a host with no docker).
///
/// The four `deploy db` wrappers are deliberately absent: their recipes are make's own
/// `cargo run -q -p xtask -- deploy db …` transport, which the port replaces with an in-process
/// call rather than reproducing. Their argv mapping is proved by running both sides — see the
/// slice's acceptance notes — not by a text pin of a command the port never issues.
pub(crate) fn rendered_recipes() -> Vec<(&'static str, Vec<String>)> {
    let c = "podman compose";
    let seed_lines: Vec<String> = SEEDS
        .iter()
        .map(|f| format!("cd {WEB} && {c} exec -T db psql -U tbd -d {IT_MAINT_DB} < seeds/{f}"))
        .collect();
    vec![
        ("db-up", vec![format!("cd {WEB} && {c} up -d db")]),
        ("db-down", vec![format!("cd {WEB} && {c} down")]),
        ("db-logs", vec![format!("cd {WEB} && {c} logs -f db")]),
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

/// `gate_t444`'s `awk '/^target:/,/^[^#[:space:]]/'` recipe extractor, generalised over the
/// target name.
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
        // i.e. the next target. (`gate_t444` notes the POSIX bracket-expression hazard that made
        // the original awk class ambiguous; spelling the three characters out avoids it.)
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
mod tests {
    use super::*;

    #[test]
    fn expand_covers_web_and_compose_and_nothing_else() {
        assert_eq!(
            expand_make_vars("cd $(WEB) && $(COMPOSE) up -d db"),
            "cd apps/website/api && podman compose up -d db"
        );
        assert_eq!(expand_make_vars("$(CURDIR)/x"), "$(CURDIR)/x");
    }

    #[test]
    fn recipe_body_strips_make_prefixes_and_comments() {
        let mk = "seed: ## doc\n\t@# comment\n\t-cmd one\n\t@cmd two\nnext:\n\tcmd three\n";
        assert_eq!(recipe_body(mk, "seed"), vec!["cmd one", "cmd two"]);
    }

    /// `seed:` must not swallow a following target's recipe, and `seed-dev:` must not open it.
    #[test]
    fn recipe_body_stops_at_the_next_target() {
        let mk = "seed-dev:\n\tnope\nseed:\n\tyes\nnext:\n\tno\n";
        assert_eq!(recipe_body(mk, "seed"), vec!["yes"]);
    }

    /// The rendered `seed` lane is the gate_t444 contract: five files, wiki last.
    #[test]
    fn seed_recipe_keeps_all_five_appliers_in_order() {
        let all = rendered_recipes();
        let (_, seed) = all.iter().find(|(t, _)| *t == "seed").expect("seed lane");
        assert_eq!(seed.len(), 5);
        assert!(seed[0].ends_with("< seeds/discord_roles.sql"));
        assert!(
            seed[4].ends_with("< seeds/wiki_pages.sql"),
            "gate_t444 pins the wiki seed to this recipe"
        );
    }
}
