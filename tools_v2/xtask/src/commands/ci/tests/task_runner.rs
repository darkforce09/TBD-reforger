//! Unit tests for [`crate::commands::ci::task_runner`] and the task table it interprets.
//!
//! ── EVERY PIN HERE READS A SUBJECT THAT ALWAYS EXISTS ────────────────────────────────────────
//!
//! `ci-local` is the local replay of `ci.yml`, and the way it goes wrong is silent subtraction: a
//! step is dropped, the composite still exits 0, and the gate it used to run stops running with
//! nothing going red. [`ci_local_step_set_is_frozen`] freezes the composite's step list by name,
//! [`list_gates_equals_the_wave_gate_constant`] holds the gate list against the wave gate's own
//! constant, and [`verify_documentation_runs_the_three_documentation_gates`] freezes the
//! documentation composite's gates by the commands they echo.
//!
//! None of them skips itself when a path it wants is absent, because a pin that returns early
//! goes QUIET instead of red — the exact defect class this program exists to kill. `help`
//! renders FROM [`TASKS`], so there is no second copy of the help text left to drift.

use super::*;

/// Repo root from the crate dir: tests run with CWD = `tools_v2/xtask/`, and CARGO_MANIFEST_DIR is the one
/// path that is stable regardless of how the test binary was invoked.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools_v2/xtask has a parent")
        .parent()
        .expect("tools_v2/xtask/ has a parent")
        .to_path_buf()
}

/// `ci-local`'s steps, in order, as `Step::Task` names plus the echo of anything that is not one.
///
/// `ci-local` is the local replay of `ci.yml`, and the way it goes wrong is silent subtraction: a
/// step is dropped, the composite still exits 0, and the gate that step ran stops running with
/// nothing going red. The step list is frozen HERE so a subtraction is a failing test.
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
            "verify-ci-shell",
            // documentation_v2/standards/engine_boundary_rules.md §5 rules 1, 2 (phase 1F),
            // 3b (2B), 3a (2C), 4 and 7 (2D). Sits with the language gates because
            // it is the same shape: a seconds-long source scan of a wall the compiler cannot see.
            "verify-engine-layers",
            "rust-ci",
            "verify-coding-standards",
            "verify-documentation",
            "ci-local-leptos",
            "ci-local-schema",
            "verify-staging-compose-paths",
            "verify-mission-rest-size-limits",
            // A direct call, deliberately NOT Step::Task("verify-ci-schema-parity"). That gate
            // enforces the same thing at runtime; this pins the ORDER and the full set with it.
            "cargo xtask verify ci-schema-parity",
        ],
        "ci-local lost or gained a step — a dropped step silently stops running a gate"
    );
}

#[test]
fn ci_local_schema_checks_freshness_first_without_regenerating_outputs() {
    let schema = find("ci-local-schema").expect("ci-local-schema row");
    assert_eq!(
        step_names(schema),
        [
            "verify-codegen-fresh",
            "schema-validate",
            "verify-citations"
        ],
        "schema validation must fail on stale generated files before running other checks"
    );
    let mut pending = vec![schema];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(task) = pending.pop() {
        if !visited.insert(task.name) {
            continue;
        }
        assert_ne!(
            task.name, "schema-codegen",
            "schema verification must not repair the stale outputs it is checking"
        );
        for step in task.steps {
            if let Step::Task(name) = step {
                pending.push(find(name).expect("schema dependency must resolve"));
            } else if let Some(command) = step_echo(step) {
                assert!(
                    !command.contains("schema-codegen") && !command.contains("schema codegen"),
                    "{} invokes mutating codegen: {command}",
                    task.name
                );
            }
        }
    }
}

#[test]
fn list_gates_equals_the_wave_gate_constant() {
    // `gate_schema` refuses to report PASS unless its hardcoded set agrees with the live task
    // table, and `xtask schema list-gates` is what prints that table's sub-gate set. The second
    // source is VALIDATE_GATES in schema.rs — the same pin gate_schema diffs against. They must
    // be equal here, or the tripwire is handed a set nobody has ever compared, which is exactly
    // the failure it exists to catch.
    let body = std::fs::read_to_string(
        root().join("tools_v2/xtask/src/commands/platform/wave_execution/schema.rs"),
    )
    .expect("schema.rs");
    let key = "const VALIDATE_GATES: &[&str] = &[";
    let start = body.find(key).expect("VALIDATE_GATES in schema.rs") + key.len();
    let block = body[start..]
        .split_once(']')
        .expect("VALIDATE_GATES close")
        .0;
    let mut want: Vec<&str> = block
        .lines()
        .filter_map(|l| {
            let t = l.trim().trim_end_matches(',');
            t.strip_prefix('"').and_then(|x| x.strip_suffix('"'))
        })
        .collect();
    let mut got = validate_gate_names(find("schema-validate").unwrap());
    want.sort_unstable();
    got.sort();
    assert_eq!(
        got, want,
        "`xtask schema list-gates` disagrees with schema.rs VALIDATE_GATES"
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
            "verify-ci-shell",
            "verify-engine-layers",
            "rust-ci",
            "verify-coding-standards",
            "verify-documentation",
            "ci-local-leptos",
            "ci-local-schema",
            "verify-staging-compose-paths",
            "verify-mission-rest-size-limits",
            // Direct, never `Step::Task` — the tripwire that polices hollow task bodies must not
            // be reachable only through the dispatcher it polices.
            "cargo xtask verify ci-schema-parity",
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
/// A passing run is not evidence: the only way to believe a green composite is to have watched a
/// red one. The synthetic table is the real recursion — `run_task_in` is what `run_task` calls.
#[test]
fn a_failing_leaf_fails_the_composite() {
    let marker = std::env::temp_dir().join(format!("task-runner-after-{}", std::process::id()));
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

/// `verify-documentation` runs the three documentation gates in process, in this order, over the
/// committed tree; dropping one silently stops CI's local replay from judging that rule.
#[test]
fn verify_documentation_runs_the_three_documentation_gates() {
    let t = find("verify-documentation").expect("verify-documentation row");
    assert_eq!(t.group, "verify");
    assert_eq!(t.lane, Lane::Ci);
    assert_eq!(
        step_names(t),
        vec![
            "cargo xtask verify readme-coverage",
            "cargo xtask verify link-check",
            "cargo xtask verify markdown-placement",
        ],
        "verify-documentation lost, gained or reordered a documentation gate"
    );
    assert!(
        t.steps.iter().all(|s| matches!(s, Step::Xtask { .. })),
        "each documentation gate runs in process as the CLI's own function"
    );
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
        split_cmd("cd apps/website/api_v2 && cargo build --release --bin api"),
        (
            Some("apps/website/api_v2"),
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
