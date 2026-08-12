//! Unit tests for [`crate::mk_ci`] (SIZE split — keep `mk_ci.rs` under 600, as `mod_wave` does).
//!
//! The load-bearing one is [`makefile_recipes_match_the_table`]. T-896 carries `rust-ci`,
//! `ci-local-leptos`, `rust-test` and `rust-test-it` recipes that belong to T-894/T-895's lanes,
//! because `ci-local` / `test` / `build` cannot honestly report a result without running them.
//! A carried recipe is a copy, and a copy rots — so while the Makefile is still on disk, every
//! row of [`TASKS`] is diffed against the recipe it claims to reproduce, mine and theirs alike.

use super::*;

/// Repo root from the crate dir: tests run with CWD = `xtask/`, and CARGO_MANIFEST_DIR is the one
/// path that is stable regardless of how the test binary was invoked.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// Recipe lines of `<target>` from the Makefile, `$(MAKE)`/`$(WEB)` expanded, continuations
/// joined, `@#` comment-only lines dropped.
///
/// A second, deliberately independent parse: `xtask/src/wave/schema.rs` has one too, and the
/// point of two implementations is that they must agree. Same three shapes handled — blank lines,
/// column-0 `#`, backslash continuations (T-422).
fn makefile_recipe(target: &str) -> Vec<String> {
    let body = std::fs::read_to_string(root().join("Makefile")).expect("Makefile");
    let lines: Vec<&str> = body.lines().collect();
    let head = format!("{target}:");
    let mut i = match lines.iter().position(|l| l.starts_with(&head)) {
        Some(i) => i + 1,
        None => panic!("no `{target}:` target in the Makefile"),
    };
    let mut out = Vec::new();
    while i < lines.len() {
        let l = lines[i];
        if l.trim().is_empty() || l.starts_with('#') {
            i += 1;
            continue;
        }
        if !l.starts_with('\t') {
            break;
        }
        let mut line = l[1..].to_string();
        while line.trim_end().ends_with('\\') {
            let t = line.trim_end();
            line = t[..t.len() - 1].to_string();
            i += 1;
            if i >= lines.len() {
                break;
            }
            line.push_str(lines[i].trim_start_matches('\t'));
        }
        // `$$` is make's escape for a literal `$` reaching the shell (`$$db` in rust-test-it's
        // reaper loop is the shell's `$db`). Expanding it here is not cosmetic: without it the
        // parity check would demand the port carry make's escaping into a string make never
        // hands to sh.
        let line = line
            .replace("$(MAKE)", "make")
            .replace("$(WEB)", "apps/website/api")
            .replace("$$", "$");
        // `@# …` is a silenced shell comment — a no-op, not a step.
        if !line.trim_start_matches(['@', '-']).starts_with('#') {
            out.push(line);
        }
        i += 1;
    }
    out
}

/// A step rendered back into the Makefile recipe line it claims to reproduce.
fn render(s: &Step) -> Option<String> {
    Some(match s {
        Step::Task(n) => format!("make {n}"),
        Step::Cmd { line, silent } => format!("{}{line}", if *silent { "@" } else { "" }),
        Step::Xtask { echo, silent, .. } => format!(
            "{}{}",
            if *silent { "@" } else { "" },
            echo.replace("cargo xtask ", "cargo run -q -p xtask -- ")
        ),
        Step::Shell {
            script,
            silent,
            ignore_err,
        } => format!(
            "{}{script}",
            if *ignore_err {
                "-"
            } else if *silent {
                "@"
            } else {
                ""
            }
        ),
        // The one recipe with no textual equivalent: a shell pipeline replaced by a Rust gate.
        // Pinned separately by `doc_layout_recipe_message_is_pinned`.
        Step::Native { .. } => return None,
    })
}

fn squeeze(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn makefile_recipes_match_the_table() {
    if !root().join("Makefile").exists() {
        // T-897 deletes it. After that there is nothing to diff against and the transition-period
        // tripwire has done its job; the table is then the only source, as intended.
        eprintln!("Makefile absent — parity check retired (T-897 deleted it).");
        return;
    }
    for t in TASKS {
        let recipe = makefile_recipe(t.name);
        let mine: Vec<String> = t.steps.iter().filter_map(render).collect();
        let theirs: Vec<String> = match t.steps.first() {
            // verify-doc-layout: rendered as nothing, pinned by message instead.
            Some(Step::Native { .. }) => continue,
            _ => recipe,
        };
        assert_eq!(
            mine.len(),
            theirs.len(),
            "{}: {} step(s) in TASKS vs {} recipe line(s) in the Makefile\n  TASKS:    {:#?}\n  Makefile: {:#?}",
            t.name,
            mine.len(),
            theirs.len(),
            mine,
            theirs
        );
        for (got, want) in mine.iter().zip(theirs.iter()) {
            // Shell steps carry a joined continuation whose exact whitespace make and sh both
            // treat as insignificant; everything else is compared byte-for-byte.
            if got.contains(" | while read -r db;") {
                assert_eq!(squeeze(got), squeeze(want), "{}", t.name);
            } else {
                assert_eq!(got, want, "{} recipe drift", t.name);
            }
        }
    }
}

#[test]
fn help_text_matches_the_makefile() {
    if !root().join("Makefile").exists() {
        return;
    }
    let body = std::fs::read_to_string(root().join("Makefile")).unwrap();
    for t in TASKS {
        let head = format!("{}:", t.name);
        let line = body
            .lines()
            .find(|l| l.starts_with(&head))
            .unwrap_or_else(|| panic!("no `{}` target", t.name));
        let want = line.split("## ").nth(1).unwrap_or("");
        assert_eq!(t.help, want, "{}: help text drift", t.name);
    }
}

#[test]
fn doc_layout_recipe_message_is_pinned() {
    if !root().join("Makefile").exists() {
        return;
    }
    let recipe = makefile_recipe("verify-doc-layout").join(" ");
    assert!(
        recipe.starts_with('@'),
        "verify-doc-layout is expected to be @-silenced; Step::Native prints no echo"
    );
    assert!(
        recipe.contains(DOC_LAYOUT_MSG),
        "the Rust gate's message no longer matches the recipe's:\n  rust: {DOC_LAYOUT_MSG}\n  make: {recipe}"
    );
}

#[test]
fn list_gates_equals_the_makefile_schema_validate_set() {
    if !root().join("Makefile").exists() {
        return;
    }
    // Exactly what wave.sh:1598 scrapes today: the trailing word of each `schema <name>` line.
    let want: Vec<String> = makefile_recipe("schema-validate")
        .iter()
        .filter_map(|l| l.split("-p xtask -- schema ").nth(1).map(str::to_string))
        .collect();
    let got = validate_gate_names(find("schema-validate").unwrap());
    assert_eq!(
        got, want,
        "`xtask schema list-gates` drifted from the recipe"
    );
    assert_eq!(got.len(), 9, "T-420 pinned the set at nine sub-gates");
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
