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
use crate::commands::ci::workspace_member_tests::{DEDICATED_TEST_TASKS, member_packages_except};

/// Repo root from the crate dir: tests run with CWD = `tools/xtask/`, and CARGO_MANIFEST_DIR is the one
/// path that is stable regardless of how the test binary was invoked.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools/xtask has a parent")
        .parent()
        .expect("tools/xtask/ has a parent")
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
            // All eight rules of documentation/standards/engine_boundary_rules.md §5 (1, 2,
            // 3a, 3b, 4, 5, 6 and 7). Sits with the language gates because it is the same
            // shape: a seconds-long source scan of a wall the compiler cannot see.
            "verify-engine-layers",
            "verify-workspace-laws",
            "rust-ci",
            // ci.yml's api job tests the developer tools' library; the derived lane tests every
            // workspace member no dedicated task tests.
            "developer-tools-test",
            "workspace-member-tests",
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
        root().join("tools/xtask/src/commands/platform/wave_execution/schema.rs"),
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
            "verify-workspace-laws",
            "rust-ci",
            // ci.yml's api job tests the developer tools' library; the derived lane tests every
            // workspace member no dedicated task tests.
            "developer-tools-test",
            "workspace-member-tests",
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
        vec![Step::Cmd {
            line: Box::leak(touch.clone().into_boxed_str()),
            silent: true,
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
    // needs a shell: quoting, redirection, globbing, `;`, `|`, `$`. The table has no shell step:
    // anything that needs one runs in process instead. Without this pin, adding one quoted
    // argument would silently pass the quote characters through as part of an argv element.
    for t in TASKS {
        for s in t.steps {
            if let Step::Cmd { line, .. } = s {
                let rest = line
                    .strip_prefix("cd ")
                    .map_or(*line, |r| r.split_once(" && ").map_or(r, |(_, tail)| tail));
                assert!(
                    !rest.contains(['"', '\'', '|', ';', '$', '>', '<', '*', '`']),
                    "{}: `{line}` needs a shell — run it in process instead",
                    t.name
                );
            }
        }
    }
    // …and the `cd <dir> && <cmd>` split is the only shell idiom that IS honoured.
    assert_eq!(
        split_cmd("cd apps/api && cargo build --release --bin api"),
        (
            Some("apps/api"),
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

/// The container runtime names a step must never run itself.
const CONTAINER_RUNTIMES: &[&str] = &["podman", "docker", "podman-compose", "docker-compose"];

/// Every `<task>: <line>` whose echoed line names a container runtime as a word, wherever in the
/// line it stands: first, after `cd … &&`, or inside a pipeline.
fn bare_container_runtime_steps(tasks: &[Task]) -> Vec<String> {
    let mut offences = Vec::new();
    for t in tasks {
        for line in t.steps.iter().filter_map(step_echo) {
            let named = line
                .split(|c: char| c.is_whitespace() || ";|&()<>\"'`\\".contains(c))
                .any(|word| CONTAINER_RUNTIMES.contains(&word));
            if named {
                offences.push(format!("{}: {line}", t.name));
            }
        }
    }
    offences
}

/// The database lane resolves the container runtime (`TBD_CONTAINER_RUNTIME`, `podman`, `docker`,
/// or either through the distrobox bridge). A ci step that names a runtime itself fails wherever
/// the runtime is reachable only through the bridge, so every step reaches the database through a
/// `cargo xtask db` command instead.
#[test]
fn no_task_step_names_a_bare_container_runtime() {
    let offences = bare_container_runtime_steps(TASKS);
    assert!(
        offences.is_empty(),
        "these steps run a container runtime themselves instead of a `cargo xtask db` command:\n{}",
        offences.join("\n")
    );
}

/// The scan sees a runtime at every position a shell would run it, and only as a whole word.
#[test]
fn the_runtime_scan_sees_a_runtime_wherever_a_shell_runs_it() {
    let flagged = [
        "podman exec tbd_reforger_db psql",
        "cd deploy && docker compose up -d db",
        "psql -Atc x | while read -r db; do podman exec c true; done",
        "podman-compose up",
    ];
    for line in flagged {
        let table = [task(
            "probe",
            vec![Step::Cmd {
                line,
                silent: false,
            }],
        )];
        assert_eq!(
            bare_container_runtime_steps(&table).len(),
            1,
            "the scan missed `{line}`"
        );
    }
    let clean = [task(
        "probe",
        vec![
            Step::Cmd {
                line: "cargo xtask db test-it",
                silent: false,
            },
            Step::Cmd {
                line: "cargo test -p podman_free_package",
                silent: false,
            },
        ],
    )];
    assert_eq!(bare_container_runtime_steps(&clean), Vec::<String>::new());
}

/// Every line the task `name` runs, its `Step::Task` rows followed, and every row it reaches.
fn reachable(name: &str) -> (Vec<&'static str>, std::collections::BTreeSet<&'static str>) {
    let mut lines = Vec::new();
    let mut rows = std::collections::BTreeSet::new();
    let mut pending = vec![find(name).unwrap_or_else(|| panic!("no task {name}"))];
    while let Some(task) = pending.pop() {
        if !rows.insert(task.name) {
            continue;
        }
        for step in task.steps {
            match step {
                Step::Task(next) => {
                    pending.push(find(next).unwrap_or_else(|| panic!("no task {next}")))
                }
                other => lines.extend(step_echo(other)),
            }
        }
    }
    (lines, rows)
}

/// Whether `line` runs the tests of the workspace member at `path` whose package is `package`:
/// a `cargo test` naming the package, a `cargo test` inside the member's folder, or the database
/// lane, which runs the tests of the API's folder.
fn line_tests_member(line: &str, package: &str, path: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    let names_package = words
        .windows(2)
        .any(|pair| matches!(pair[0], "-p" | "--package") && pair[1] == package);
    let in_member_folder = line.starts_with(&format!("cd {path} && cargo test"));
    let database_lane =
        line == "cargo xtask db test-it" && path == crate::commands::db::operations::WEB;
    (line.contains("cargo test") && (names_package || in_member_folder)) || database_lane
}

/// The workspace members no line or row reachable from `tasks` tests, as `package (path)`. The
/// `workspace-member-tests` row tests every member outside its dedicated packages.
fn untested_members(tasks: &[&str]) -> Vec<String> {
    let members =
        verification_core::repository_laws::workspace_members::read_workspace_members(&root())
            .expect("the workspace members read");
    let mut lines = Vec::new();
    let mut rows = std::collections::BTreeSet::new();
    for task in tasks {
        let (task_lines, task_rows) = reachable(task);
        lines.extend(task_lines);
        rows.extend(task_rows);
    }
    let derived: Vec<String> = if rows.contains("workspace-member-tests") {
        let dedicated: Vec<&str> = DEDICATED_TEST_TASKS.iter().map(|(p, _)| *p).collect();
        member_packages_except(&root(), &dedicated).expect("the derived lane reads")
    } else {
        Vec::new()
    };
    members
        .iter()
        .filter(|member| {
            !derived.contains(&member.package_name)
                && !lines
                    .iter()
                    .any(|line| line_tests_member(line, &member.package_name, &member.path))
        })
        .map(|member| format!("{} ({})", member.package_name, member.path))
        .collect()
}

/// Each dedicated package's task really tests it, so leaving the package out of the derived lane
/// never leaves it untested.
#[test]
fn every_dedicated_test_task_tests_its_package() {
    let members =
        verification_core::repository_laws::workspace_members::read_workspace_members(&root())
            .expect("the workspace members read");
    for (package, task) in DEDICATED_TEST_TASKS {
        let member = members
            .iter()
            .find(|member| member.package_name == package)
            .unwrap_or_else(|| panic!("{package} is no workspace member"));
        let (lines, _) = reachable(task);
        assert!(
            lines
                .iter()
                .any(|line| line_tests_member(line, package, &member.path)),
            "`{task}` does not test {package}: {lines:?}"
        );
    }
}

/// The ci.yml steps that run a task of this table: `cargo xtask ci <task>` or, for the build
/// lane's recipes this table repeats, `cargo xtask mk <task>`.
fn ci_workflow_tasks() -> Vec<String> {
    let workflow = std::fs::read_to_string(root().join(".github/workflows/ci.yml"))
        .expect("read .github/workflows/ci.yml");
    workflow
        .lines()
        .filter_map(|line| line.trim().trim_start_matches("- ").strip_prefix("run: "))
        .filter_map(|run| {
            run.strip_prefix("cargo xtask ci ")
                .or_else(|| run.strip_prefix("cargo xtask mk "))
        })
        .map(|task| task.trim().to_string())
        .collect()
}

/// `ci-local` tests every workspace member: a member the workspace gains is tested from the
/// moment the root manifest names it, never only once someone extends a list.
#[test]
fn ci_local_tests_every_workspace_member() {
    let untested = untested_members(&["ci-local"]);
    assert!(
        untested.is_empty(),
        "no step `ci-local` reaches tests these workspace members:\n  {}",
        untested.join("\n  ")
    );
}

/// The ci.yml jobs test every workspace member, through the tasks of this table they run.
#[test]
fn the_ci_workflow_tests_every_workspace_member() {
    let tasks = ci_workflow_tasks();
    assert!(
        tasks.iter().any(|task| task == "api-test"),
        "the ci.yml task list was not read: {tasks:?}"
    );
    let names: Vec<&str> = tasks.iter().map(String::as_str).collect();
    let untested = untested_members(&names);
    assert!(
        untested.is_empty(),
        "no ci.yml step tests these workspace members:\n  {}",
        untested.join("\n  ")
    );
}
