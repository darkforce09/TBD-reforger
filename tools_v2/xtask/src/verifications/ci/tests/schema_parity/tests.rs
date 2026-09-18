use super::*;

/// Minimal CI schema job that satisfies the run-step pin.
fn ci_ok() -> String {
    format!(
        "jobs:\n  schema:\n    steps:\n      - run: {GOOD_RUN}\n  other:\n    steps:\n      - run: true\n"
    )
}

/// Dual-path fixture derived from the live gate.rs needles (T-902).
fn wave_ok() -> String {
    format!(
        "const VERIFY_STEPS: &[(&str, &str)] = &[\n    {ROW_T456},\n    {ROW_T468},\n];\n\npub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {{\n    {VERIFY_LOOP} {{\n        r.run(label, || {{\n            checkrun(\n                ctx,\n                {CHECKRUN_ARGV},\n            )\n        }});\n    }}\n    0\n}}\n\npub fn cmd_gate(ctx: &Ctx, base_arg: &str) -> u8 {{\n    {VERIFY_LOOP} {{\n        r.run(label, || {{\n            checkrun(\n                ctx,\n                {CHECKRUN_ARGV},\n            )\n        }});\n    }}\n    0\n}}\n"
    )
}

fn pins(wave: &str) -> i32 {
    run_pins(&ci_ok(), Some(wave))
}

#[test]
fn live_shaped_pins_hold() {
    assert_eq!(pins(&wave_ok()), 0);
}

/// THE THREE RECIPE-BODY PINS, against the LIVE table (T-897).
///
/// They cannot be fixture-driven the way the Makefile pins were: `TASKS` is a `static`, so
/// there is no perturbed copy to hand in. That is the point — the table this asserts on is the
/// one `cargo xtask ci` runs, and `task_pins` is the same function the gate calls. Hollowing
/// any of the three rows turns this red, which is the perturbation proof (`ci-local-schema`
/// losing `verify-citations`, `verify-t456` losing its echo, `ci-local` losing its direct
/// `verify t468` step — each was run by hand at T-897 and each RED'd here).
#[test]
fn the_live_task_table_satisfies_the_recipe_pins() {
    assert_eq!(task_pins(), 0);
}

/// T-489 by construction: no `verify-t468` row exists for `ci-local` to reach instead of the
/// direct call. `task_pins` enforces this at runtime; asserting it here names why.
#[test]
fn t468_stays_off_the_dispatch_table_it_polices() {
    assert!(crate::commands::ci::task_runner::find("verify-t468").is_none());
    let ci_local = crate::commands::ci::task_runner::find("ci-local").expect("ci-local row");
    assert!(!crate::commands::ci::task_runner::invoked_tasks(ci_local).contains(&"verify-t468"));
    assert!(
        ci_local
            .steps
            .iter()
            .any(|s| crate::commands::ci::task_runner::step_echo(s) == Some(TASK_ECHO_T468))
    );
}

/// The two echoes are the in-table spellings of the two cargo consts, and the tests would be
/// worthless if they drifted apart silently.
#[test]
fn task_echoes_name_the_same_gates_as_the_wave_consts() {
    assert!(VERIFY_T456.ends_with("verify t456") && TASK_ECHO_T456.ends_with("verify t456"));
    assert!(VERIFY_T468.ends_with("verify t468") && TASK_ECHO_T468.ends_with("verify t468"));
}

#[test]
fn schema_job_without_ci_local_schema_fails() {
    let ci = "jobs:\n  schema:\n    steps:\n      - run: cargo run -p xtask -- schema validate\n  other:\n    steps:\n      - run: true\n";
    assert_ne!(run_pins(ci, Some(&wave_ok())), 0);
}

/// The former `make ci-local-schema` spelling names a target that no longer exists, so it must
/// no longer satisfy the CI pin — otherwise a stale workflow would read as covered.
#[test]
fn the_make_spelling_no_longer_satisfies_the_ci_pin() {
    let ci = "jobs:\n  schema:\n    steps:\n      - run: make ci-local-schema\n  other:\n    steps:\n      - run: true\n";
    assert_ne!(run_pins(ci, Some(&wave_ok())), 0);
}

/// Both cargo spellings of the same command are accepted; nothing looser is.
#[test]
fn ci_run_accepts_the_alias_and_the_long_form_only() {
    assert!(ci_run_is_good("cargo xtask ci ci-local-schema"));
    assert!(ci_run_is_good(
        "cargo run -q -p xtask -- ci ci-local-schema"
    ));
    assert!(ci_run_is_good("cargo  xtask   ci  ci-local-schema"));
    assert!(!ci_run_is_good("cargo xtask ci ci-local-schema || true"));
    assert!(!ci_run_is_good("cargo xtask ci ci-local-schema --help"));
    assert!(!ci_run_is_good("cargo xtask ci schema-validate"));
}

#[test]
fn verify_consts_are_the_cargo_spelling() {
    assert_eq!(VERIFY_T456, "cargo run -q -p xtask -- verify t456");
    assert_eq!(VERIFY_T468, "cargo run -q -p xtask -- verify t468");
}

/// M2: hollow checkrun argv must RED.
#[test]
fn wave_hollow_t456_true_fails() {
    let wave = wave_ok().replacen(CHECKRUN_ARGV, "&[]", 1);
    assert_ne!(pins(&wave), 0);
}

#[test]
fn wave_hollow_both_paths_required() {
    let wave = wave_ok().replacen(CHECKRUN_ARGV, "&[]", 1);
    // Only gate_slice hollowed; cmd_gate still has the argv — still must FAIL.
    assert_eq!(wave.matches(CHECKRUN_ARGV).count(), 1);
    assert_ne!(pins(&wave), 0);
}

/// B1: extra argv entries must NOT satisfy the checkrun pin.
#[test]
fn wave_suffix_smuggles_fail_pin() {
    let smuggled = r#"&["cargo", "run", "-q", "-p", "xtask", "--", "verify", name, "--help"]"#;
    let wave = wave_ok().replace(CHECKRUN_ARGV, smuggled);
    assert_ne!(pins(&wave), 0, "extra --help argv must FAIL the pin");
}

#[test]
fn wave_commented_run_does_not_satisfy() {
    let wave = wave_ok().replacen(ROW_T456, &format!("// {ROW_T456}"), 1);
    assert_ne!(pins(&wave), 0);
}

#[test]
fn missing_wave_fails() {
    assert_ne!(run_pins(&ci_ok(), None), 0);
}

/// Disposable copy of the exact source files the runtime gate inspects.
struct SourceFixture(std::path::PathBuf);

impl SourceFixture {
    fn live() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let suffix = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "tbd-ci-schema-parity-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir(&root).expect("unique fixture directory");
        let fixture = Self(root);
        let live_root = crate::core::repository_root::test_repo_root();
        let mut paths = vec![
            Path::new(CI_REL).to_path_buf(),
            Path::new(WAVE_REL).to_path_buf(),
        ];
        for (module, _) in source_audit::WAVE_CHILDREN {
            paths.push(
                Path::new(WAVE_REL)
                    .with_extension("")
                    .join(format!("{module}.rs")),
            );
        }
        for relative in paths {
            let destination = fixture.0.join(&relative);
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::copy(live_root.join(relative), destination).unwrap();
        }
        fixture
    }

    fn child(&self, module: &str) -> std::path::PathBuf {
        self.0
            .join(WAVE_REL)
            .with_extension("")
            .join(format!("{module}.rs"))
    }
}

impl Drop for SourceFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn live_source_owners_satisfy_the_runtime_gate() {
    let fixture = SourceFixture::live();
    assert_eq!(verify_t468(&fixture.0).unwrap(), 0);
}

#[test]
fn each_missing_wave_child_fails_the_runtime_gate() {
    for (module, _) in source_audit::WAVE_CHILDREN {
        let fixture = SourceFixture::live();
        std::fs::remove_file(fixture.child(module)).unwrap();
        assert_eq!(verify_t468(&fixture.0).unwrap(), 1, "missing {module}");
    }
}

#[test]
fn facade_exports_cannot_replace_implementation_bodies() {
    let fixture = SourceFixture::live();
    for (module, function) in source_audit::WAVE_CHILDREN {
        std::fs::write(
            fixture.child(module),
            format!("pub use elsewhere::{function};\n"),
        )
        .unwrap();
    }
    assert_eq!(verify_t468(&fixture.0).unwrap(), 1);
}

#[test]
fn disconnected_or_conditionally_disabled_children_fail() {
    for (module, function) in source_audit::WAVE_CHILDREN {
        for statement in [
            format!("mod {module};"),
            format!("pub use {module}::{function};"),
        ] {
            for replacement in [String::new(), format!("#[cfg(any())]\n{statement}")] {
                let fixture = SourceFixture::live();
                let facade_path = fixture.0.join(WAVE_REL);
                let facade = std::fs::read_to_string(&facade_path).unwrap();
                assert!(facade.contains(&statement));
                std::fs::write(&facade_path, facade.replacen(&statement, &replacement, 1)).unwrap();
                assert_eq!(verify_t468(&fixture.0).unwrap(), 1, "disabled {statement}");
            }
        }
    }
}

#[test]
fn each_hollowed_live_wave_body_fails_the_runtime_gate() {
    for (module, _) in source_audit::WAVE_CHILDREN {
        let fixture = SourceFixture::live();
        let path = fixture.child(module);
        let source = std::fs::read_to_string(&path).unwrap();
        assert!(source.contains(CHECKRUN_ARGV));
        std::fs::write(path, source.replace(CHECKRUN_ARGV, "&[]")).unwrap();
        assert_eq!(verify_t468(&fixture.0).unwrap(), 1, "hollowed {module}");
    }
}
