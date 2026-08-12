//! Unit tests for [`crate::mk_ci`] (SIZE split — keep `mk_ci.rs` under 600, as `mod_wave` does).
//!
//! ── WHAT DIED WITH THE MAKEFILE, AND WHAT REPLACED IT (T-897) ────────────────────────────────
//!
//! T-896's load-bearing test was `makefile_recipes_match_the_table`: it parsed the root Makefile
//! and diffed every [`TASKS`] row against the recipe it claimed to reproduce, because `ci-local` /
//! `test` / `build` carry recipes belonging to T-894/T-895's lanes and a carried copy rots. Three
//! more (`help_text_matches_the_makefile`, `doc_layout_recipe_message_is_pinned`,
//! `list_gates_equals_the_makefile_schema_validate_set`) had the same subject.
//!
//! T-897 deleted that file. Each of the four was written with an `if !Makefile.exists() { return }`
//! guard, i.e. each would have gone QUIET rather than red — the exact defect class this program
//! exists to kill, sitting inside the tests written to prevent it. They are DELETED, not left to
//! return early, and the properties worth keeping moved to subjects that still exist:
//!
//! | retired test | successor |
//! |---|---|
//! | `makefile_recipes_match_the_table` | [`ci_local_step_set_is_frozen`] — the composite's step list, by name |
//! | `list_gates_equals_the_makefile_schema_validate_set` | [`list_gates_equals_the_wave_gate_constant`] (already existed, unguarded) |
//! | `doc_layout_recipe_message_is_pinned` | [`doc_layout_predicate_reproduces_finds_globs`] — behaviour, not message text |
//! | `help_text_matches_the_makefile` | nothing needed: `help` renders FROM `TASKS`, so there is no second copy left to drift |

use super::*;

/// Repo root from the crate dir: tests run with CWD = `xtask/`, and CARGO_MANIFEST_DIR is the one
/// path that is stable regardless of how the test binary was invoked.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// `ci-local`'s steps, in order, as `Step::Task` names plus the echo of anything that is not one.
///
/// THE SUCCESSOR PIN. `ci-local` is the local replay of `ci.yml`, and the way it goes wrong is
/// silent subtraction: a step is dropped, the composite still exits 0, and the gate it used to run
/// stops running with nothing going red. The Makefile parity test caught that by diffing against
/// the recipe; with no recipe left, the list is frozen HERE.
fn step_names(t: &Task) -> Vec<String> {
    t.steps
        .iter()
        .map(|s| match s {
            Step::Task(n) => (*n).to_string(),
            other => step_echo(other).unwrap_or("<native>").to_string(),
        })
        .collect()
}

#[test]
fn ci_local_step_set_is_frozen() {
    let t = find("ci-local").expect("ci-local row");
    assert_eq!(
        step_names(t),
        vec![
            "verify-editorconfig",
            "verify-no-python",
            "verify-no-node",
            "verify-no-shell",
            "rust-ci",
            "verify-coding-standards",
            "ci-local-leptos",
            "ci-local-schema",
            "verify-t438",
            "verify-t456",
            // T-489/T-881: a direct call, deliberately NOT Step::Task("verify-t468"). `gate_t468`
            // enforces the same thing at runtime; this pins the ORDER and the full set with it.
            "cargo xtask verify t468",
        ],
        "ci-local lost or gained a step — a dropped step silently stops running a gate"
    );
}

#[test]
fn list_gates_equals_the_wave_gate_constant() {
    // The Makefile-parity test above dies with the Makefile. THIS one is the one that has to
    // survive: `wave.sh`'s `gate_schema` refuses to report PASS unless its hardcoded set agrees
    // with a parse of the recipe, and T-897 takes that recipe away. `xtask schema list-gates` is
    // the replacement input, so it must equal the constant TODAY — otherwise the repoint would
    // hand the tripwire a set nobody had ever compared, which is the failure it exists to catch.
    let body = std::fs::read_to_string(root().join("scripts/platform/wave.sh")).expect("wave.sh");
    let line = body
        .lines()
        .find(|l| l.starts_with("GATE_SCHEMA_VALIDATE_GATES="))
        .expect("GATE_SCHEMA_VALIDATE_GATES in wave.sh");
    let mut want: Vec<&str> = line
        .split('"')
        .nth(1)
        .expect("quoted value")
        .split_whitespace()
        .collect();
    let mut got = validate_gate_names(find("schema-validate").unwrap());
    want.sort_unstable();
    got.sort();
    assert_eq!(
        got, want,
        "`xtask schema list-gates` disagrees with wave.sh's GATE_SCHEMA_VALIDATE_GATES"
    );
}

#[test]
fn every_composite_step_resolves() {
    // Structural non-hollowness: a `Step::Task` that names nothing would make a composite skip a
    // step at runtime. There is no arrangement of this table in which that compiles away silently.
    for t in TASKS {
        for s in t.steps {
            if let Step::Task(n) = s {
                assert!(
                    find(n).is_some(),
                    "{}: step `{n}` resolves to no task",
                    t.name
                );
            }
        }
    }
}

#[test]
fn ci_local_runs_the_leaves_not_a_copy_of_them() {
    let ci = find("ci-local").unwrap();
    let names: Vec<&str> = ci
        .steps
        .iter()
        .map(|s| match s {
            Step::Task(n) => *n,
            Step::Xtask { echo, .. } => echo,
            _ => "?",
        })
        .collect();
    assert_eq!(
        names,
        vec![
            "verify-editorconfig",
            "verify-no-python",
            "verify-no-node",
            "verify-no-shell",
            "rust-ci",
            "verify-coding-standards",
            "ci-local-leptos",
            "ci-local-schema",
            "verify-t438",
            "verify-t456",
            // T-489/T-881: direct, never `Step::Task` — the tripwire that polices hollow recipes
            // must not be reachable only through the dispatcher it polices.
            "cargo xtask verify t468",
        ]
    );
}

/* ───────────────── the behavioural proof: a composite is not hollow ───────────────── */

const OK: Step = Step::Cmd {
    line: "true",
    silent: true,
};
const FAIL: Step = Step::Cmd {
    line: "false",
    silent: true,
};

/// `Task.steps` is `&'static [Step]`, so a synthetic table has to outlive the test. Leaked: a few
/// bytes for the process lifetime, and the alternative (making the field non-static) would change
/// the production type purely to suit a test.
fn task(name: &'static str, steps: Vec<Step>) -> Task {
    Task {
        name,
        help: "",
        group: "CI",
        lane: Lane::Ci,
        steps: Box::leak(steps.into_boxed_slice()),
    }
}

/// A composite whose leaf fails must fail, and must not run the steps after it.
///
/// T-489's `@true` defect and T-556's "a passing run is not evidence" are the same lesson: the
/// only way to believe a green composite is to have watched a red one. The synthetic table is
/// the real recursion — `run_task_in` is what `run_task` calls.
#[test]
fn a_failing_leaf_fails_the_composite() {
    let marker = std::env::temp_dir().join(format!("t896-after-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let touch = format!("touch {}", marker.display());

    let touch_step = || {
        vec![Step::Shell {
            silent: true,
            script: Box::leak(touch.clone().into_boxed_str()),
            ignore_err: false,
        }]
    };
    let composite = || vec![Step::Task("leaf"), Step::Task("after")];

    let red = [
        task("leaf", vec![OK, FAIL]),
        task("after", touch_step()),
        task("composite", composite()),
    ];
    let rc = run_task_in(red.iter().find(|t| t.name == "composite").unwrap(), &red);
    assert_eq!(rc, 1, "a composite whose leaf exits 1 must exit 1");
    assert!(
        !marker.exists(),
        "fail-fast broken: the step after the failing leaf ran anyway"
    );

    // …and the same composite over leaves that all hold is green, so the assertion above is
    // measuring the failure and not a structurally-red harness.
    let green = [
        task("leaf", vec![OK]),
        task("after", touch_step()),
        task("composite", composite()),
    ];
    let rc = run_task_in(
        green.iter().find(|t| t.name == "composite").unwrap(),
        &green,
    );
    assert_eq!(rc, 0, "an all-holding composite must exit 0");
    assert!(marker.exists(), "the green arm never reached the last step");
    let _ = std::fs::remove_file(&marker);
}

/* ───────────────────────── verify-doc-layout glob semantics ───────────────────────── */

#[test]
fn doc_layout_predicate_reproduces_finds_globs() {
    // `-path '*/docs/*.md'` — `*` crosses `/`, so depth is irrelevant on either side.
    assert!(is_forbidden_doc("apps/website/docs/spec.md"));
    assert!(is_forbidden_doc("packages/tbd-schema/docs/a/b/c.md"));
    assert!(is_forbidden_doc("apps/mod/x/docs/y.md"));
    // `! -path '*/node_modules/*'`
    assert!(!is_forbidden_doc("apps/x/node_modules/p/docs/readme.md"));
    // not markdown, and not under a docs/ directory
    assert!(!is_forbidden_doc("apps/website/docs/spec.txt"));
    assert!(!is_forbidden_doc("apps/website/api/README.md"));
    // `docs` as a filename fragment is not a `docs/` directory
    assert!(!is_forbidden_doc("apps/website/docsite/a.md"));
}

#[test]
fn cmd_lines_are_shell_free() {
    // `Step::Cmd` splits on whitespace and spawns directly, which is only correct while no line
    // needs a shell: quoting, redirection, globbing, `;`, `|`, `$`. Anything that does is a
    // `Step::Shell`. Without this pin, adding one quoted argument would silently pass the quote
    // characters through as part of an argv element.
    for t in TASKS {
        for s in t.steps {
            if let Step::Cmd { line, .. } = s {
                let rest = line
                    .strip_prefix("cd ")
                    .map_or(*line, |r| r.split_once(" && ").map_or(r, |(_, tail)| tail));
                assert!(
                    !rest.contains(['"', '\'', '|', ';', '$', '>', '<', '*', '`']),
                    "{}: `{line}` needs a shell — make it a Step::Shell",
                    t.name
                );
            }
        }
    }
    // …and the `cd <dir> && <cmd>` split is the only shell idiom that IS honoured.
    assert_eq!(
        split_cmd("cd apps/website/api && cargo build --release --bin api"),
        (
            Some("apps/website/api"),
            vec!["cargo", "build", "--release", "--bin", "api"]
        )
    );
    assert_eq!(
        split_cmd("editorconfig-checker"),
        (None, vec!["editorconfig-checker"])
    );
}

#[test]
fn help_lists_every_task() {
    // `make help`'s successor cannot be allowed to go stale: the whole reason it is rendered from
    // TASKS is that a task must not be able to exist and be undiscoverable.
    let groups = ["CI", "schema", "verify", "map", "build", "db"];
    for t in TASKS {
        assert!(
            groups.contains(&t.group),
            "{}: group `{}` is not printed by help()",
            t.name,
            t.group
        );
    }
}
