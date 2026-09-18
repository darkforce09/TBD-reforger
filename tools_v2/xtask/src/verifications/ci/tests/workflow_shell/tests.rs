use super::*;

struct Tmp(PathBuf);
impl Tmp {
    fn dir(tag: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!(
            "t901-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            tag
        ));
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn write_yml(&self, name: &str, body: &str) {
        fs::write(self.0.join(name), body).unwrap();
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rc_of(yaml: &str) -> u8 {
    let d = Tmp::dir("one");
    d.write_yml("w.yml", yaml);
    run_on_workflows_dir(&d.0)
}

fn findings(yaml: &str) -> String {
    // Re-run through Report so we assert on the same printer the CLI uses. Capture by
    // checking rc plus a second pass that records headlines via a private Report.
    let d = Tmp::dir("find");
    d.write_yml("w.yml", yaml);
    let mut report = Report::new("t");
    scan_dir(&d.0, &mut report);
    format!(
        "{}:{}:{}",
        report.counts().0,
        report.counts().1,
        report.counts().2
    )
}

#[test]
fn echo_hi_is_red() {
    let yaml = "jobs:\n  j:\n    steps:\n      - run: echo hi\n";
    assert_ne!(rc_of(yaml), 0, "echo hi must not hold");
    let c = findings(yaml);
    assert!(c.contains(":1:"), "expected a violation, got {c}");
}

#[test]
fn cargo_fmt_and_true_is_red() {
    let yaml = "jobs:\n  j:\n    steps:\n      - run: cargo fmt && true\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn cargo_xtask_verify_no_shell_holds() {
    let yaml = "jobs:\n  j:\n    steps:\n      - run: cargo xtask verify no-shell\n";
    assert_eq!(rc_of(yaml), 0);
}

#[test]
fn git_lfs_pull_holds() {
    let yaml = "jobs:\n  j:\n    steps:\n      - run: git lfs pull --include foo\n";
    assert_eq!(rc_of(yaml), 0);
}

#[test]
fn planted_evil_composite_is_red() {
    let yaml = "jobs:\n  j:\n    steps:\n      - uses: evil/composite@v1\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn defaults_run_without_step_run_does_not_false_fail() {
    // THE LANDMINE. ci.yml has defaults.run.working-directory. A regex count of `run:` keys
    // treats that as a step; the production walker must not.
    let yaml = "\
jobs:\n  j:\n    defaults:\n      run:\n        working-directory: apps/website/api\n    steps:\n      - uses: actions/checkout@v7\n";
    assert_eq!(rc_of(yaml), 0, "defaults.run must not be a step");
}

#[test]
fn multiline_run_with_if_and_set_dash_is_red() {
    let yaml = "\
jobs:\n  j:\n    steps:\n      - run: |\n          set -euo pipefail\n          if true; then echo x; fi\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn unreadable_yaml_is_fail_closed() {
    let yaml = "jobs: [\n  this is not a workflow\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn uses_plus_run_is_red() {
    let yaml = "\
jobs:\n  j:\n    steps:\n      - uses: actions/checkout@v7\n        run: cargo xtask verify no-shell\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn more_than_three_logical_lines_is_red() {
    let yaml = "\
jobs:\n  j:\n    steps:\n      - run: |\n          cargo xtask a\n          cargo xtask b\n          cargo xtask c\n          cargo xtask d\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn defaults_run_only_no_steps_is_red() {
    // BLOCKER: a job with only defaults.run and no steps used to print OK — 0 check(s).
    let yaml = "\
jobs:\n  j:\n    defaults:\n      run:\n        working-directory: apps/website/api\n";
    assert_ne!(rc_of(yaml), 0, "no-steps job must not be OK with 0 checks");
}

#[test]
fn job_level_evil_uses_no_steps_is_red() {
    let yaml = "jobs:\n  j:\n    uses: evil/composite@v1\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn ampersand_background_is_red() {
    let yaml = "jobs:\n  j:\n    steps:\n      - run: cargo xtask verify no-shell & echo pwned\n";
    assert_ne!(rc_of(yaml), 0);
}

#[test]
fn empty_workflows_dir_is_red() {
    let d = Tmp::dir("empty");
    assert_ne!(run_on_workflows_dir(&d.0), 0);
}

#[test]
fn production_parser_is_the_fixture_parser() {
    // Guard against a "test parser" fork: both entry points share scan_dir.
    let yaml = "jobs:\n  j:\n    steps:\n      - run: echo hi\n";
    let d = Tmp::dir("same");
    d.write_yml("w.yml", yaml);
    assert_ne!(run_on_workflows_dir(&d.0), 0);
    let mut report = Report::new("t");
    scan_dir(&d.0, &mut report);
    assert!(!report.clean());
}
