//! T-901 — `cargo xtask verify ci-shell`.
//!
//! ── WHY THIS GATE EXISTS ─────────────────────────────────────────────────────────────────────
//!
//! The ban is not on bash the language. It is on **logic that no type checker and no local
//! replay can see**. After T-897 deleted the Makefile, the remaining hiding place for that
//! logic is `run:` blocks in `.github/workflows/`. A step that `if`/`|| true`/`set -euo pipefail`s
//! its way to a green check is the same defect class as `verify-no-python` grepping with an
//! absent `rg` and reporting OK — except this time the script is not even a file `git grep` can
//! name. It lives inside YAML, and the only way to see it locally is to parse the YAML.
//!
//! ── PARSE, DO NOT GREP ───────────────────────────────────────────────────────────────────────
//!
//! Counting `run:` with a regex is how this repo keeps shipping false greens: `defaults.run` in
//! `ci.yml` (a `working-directory`, not a step) matches `^\s+run:`, and a composite action's
//! inner script matches too. This gate walks `jobs.*.steps[*]` on a [`serde_norway`] document.
//! `defaults.run` is a sibling of `steps`, never a step, and is ignored.
//!
//! ── FAIL-CLOSED ──────────────────────────────────────────────────────────────────────────────
//!
//! Unreadable or unparseable YAML is a failure, not an empty result. A workflows directory that
//! does not exist is [`tbd_gate::NotRun`], never Held.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_norway::{Mapping, Value};
use tbd_gate::report::Report;
use tbd_gate::verdict::{Kind, NotRun, Verdict};

use crate::root::find_repo_root;
use crate::verify_ci_shell_rules::{line_reason, logical_lines, shell_reason, uses_reason};

const WORKFLOWS: &str = ".github/workflows";
const MAX_LOGICAL: usize = 3;

/// CLI entry: `cargo xtask verify ci-shell`.
pub fn verify_ci_shell() -> Result<u8> {
    let root = find_repo_root()?;
    Ok(run_on_workflows_dir(&root.join(WORKFLOWS)))
}

/// Production walker. Tests call this on throwaway directories so they exercise the same code
/// path as `verify ci-shell` against the real tree — there is no second parser.
pub fn run_on_workflows_dir(dir: &Path) -> u8 {
    let mut report = Report::new("verify-ci-shell");
    scan_dir(dir, &mut report);
    u8::try_from(report.finish()).unwrap_or(2)
}

fn scan_dir(dir: &Path, report: &mut Report) {
    if !dir.is_dir() {
        report.check(Verdict::did_not_run(
            "workflows directory",
            Kind::Ban,
            NotRun::TargetMissing(dir.to_path_buf()),
        ));
        return;
    }
    let mut files = match list_workflow_files(dir) {
        Ok(f) => f,
        Err(source) => {
            report.check(Verdict::did_not_run(
                "workflows directory",
                Kind::Ban,
                NotRun::Unreadable {
                    path: dir.to_path_buf(),
                    source,
                },
            ));
            return;
        }
    };
    files.sort();
    if files.is_empty() {
        report.check(Verdict::failed(format!(
            "{}: no .yml/.yaml workflow files — refusing to report OK over an empty input",
            dir.display()
        )));
        return;
    }
    let mut run_keys = 0u32;
    for path in &files {
        let rel = rel_display(path);
        check_file(path, &rel, report, &mut run_keys);
    }
    if report.clean() {
        println!(
            "verify-ci-shell: {run_keys} step `run:` key(s) parsed (defaults.run is not a step)"
        );
    }
}

fn list_workflow_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "yml" || ext == "yaml" {
            out.push(path);
        }
    }
    Ok(out)
}

fn rel_display(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Some(i) = s.find(".github/workflows/") {
        return s[i..].to_string();
    }
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| s.into_owned())
}

fn check_file(path: &Path, rel: &str, report: &mut Report, run_keys: &mut u32) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(source) => {
            report.check(Verdict::did_not_run(
                format!("{rel}: could not read"),
                Kind::Ban,
                NotRun::Unreadable {
                    path: path.to_path_buf(),
                    source,
                },
            ));
            return;
        }
    };
    let doc: Value = match serde_norway::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            report.check(Verdict::failed(format!("{rel}: YAML parse error: {e}")));
            return;
        }
    };
    check_doc(rel, &doc, report, run_keys);
}

fn check_doc(rel: &str, doc: &Value, report: &mut Report, run_keys: &mut u32) {
    let Some(jobs) = mapping(doc)
        .and_then(|m| map_get(m, "jobs"))
        .and_then(mapping)
    else {
        report.check(Verdict::failed(format!("{rel}: missing jobs: mapping")));
        return;
    };
    if jobs.is_empty() {
        report.check(Verdict::failed(format!("{rel}: jobs: is empty")));
        return;
    }
    for (jk, jv) in jobs {
        let job = key_str(jk);
        let Some(job_map) = mapping(jv) else {
            report.check(Verdict::failed(format!(
                "{rel}:{job}: job value is not a mapping"
            )));
            continue;
        };
        // `defaults.run` lives here. We read the mapping so a missing `steps` is still examined,
        // and we never treat `defaults.run` as a step.
        let Some(steps_v) = map_get(job_map, "steps") else {
            continue;
        };
        let Some(steps) = steps_v.as_sequence() else {
            report.check(Verdict::failed(format!(
                "{rel}:{job}: steps: is not a sequence"
            )));
            continue;
        };
        for (idx, step_v) in steps.iter().enumerate() {
            check_step(rel, &job, idx, step_v, report, run_keys);
        }
    }
}

fn check_step(
    rel: &str,
    job: &str,
    idx: usize,
    step_v: &Value,
    report: &mut Report,
    run_keys: &mut u32,
) {
    let Some(step) = mapping(step_v) else {
        report.check(Verdict::failed(format!(
            "{rel}:{job}:[{idx}]: step is not a mapping"
        )));
        return;
    };
    let label = step_label(step, idx);
    let uses = map_get(step, "uses").and_then(as_string);
    let run = map_get(step, "run");
    let shell = map_get(step, "shell").and_then(as_string);

    if run.is_some() {
        *run_keys += 1;
    }

    if uses.is_some() && run.is_some() {
        report.check(Verdict::failed(format!(
            "{rel}:{job}:{label}: step has both uses: and run:"
        )));
    }

    if let Some(u) = uses.as_deref() {
        if let Some(why) = uses_reason(u) {
            report.check(Verdict::failed(format!("{rel}:{job}:{label}:{why}")));
        } else if run.is_none() {
            report.check(Verdict::Held);
        }
    }

    if let Some(sh) = shell.as_deref()
        && let Some(why) = shell_reason(sh)
    {
        report.check(Verdict::failed(format!("{rel}:{job}:{label}:{why}")));
    }

    if let Some(run_v) = run {
        let Some(script) = as_string(run_v) else {
            report.check(Verdict::failed(format!(
                "{rel}:{job}:{label}: run: is not a string (defaults.run mappings are not steps; a step run: must be)"
            )));
            return;
        };
        check_run_script(rel, job, &label, &script, report);
    }

    if uses.is_none() && run.is_none() {
        // A name-only / if-only step is not a hiding place. Held so the check ran.
        report.check(Verdict::Held);
    }
}

fn check_run_script(rel: &str, job: &str, label: &str, script: &str, report: &mut Report) {
    let lines = logical_lines(script);
    if lines.len() > MAX_LOGICAL {
        report.check(Verdict::failed(format!(
            "{rel}:{job}:{label}: more than {MAX_LOGICAL} logical lines ({})",
            lines.len()
        )));
        return;
    }
    if lines.is_empty() {
        report.check(Verdict::failed(format!("{rel}:{job}:{label}: empty run:")));
        return;
    }
    let mut any = false;
    for (i, line) in lines.iter().enumerate() {
        if let Some(why) = line_reason(line) {
            report.check(Verdict::failed(format!(
                "{rel}:{job}:{label}:line {}: {why}",
                i + 1
            )));
            any = true;
        }
    }
    if !any {
        report.check(Verdict::Held);
    }
}

fn step_label(step: &Mapping, idx: usize) -> String {
    map_get(step, "name")
        .and_then(as_string)
        .unwrap_or_else(|| format!("[{idx}]"))
}

fn mapping(v: &Value) -> Option<&Mapping> {
    v.as_mapping()
}

fn map_get<'a>(m: &'a Mapping, k: &str) -> Option<&'a Value> {
    m.get(Value::String(k.to_string()))
}

fn key_str(k: &Value) -> String {
    match k {
        Value::String(s) => s.clone(),
        other => other.as_str().unwrap_or("?").to_string(),
    }
}

fn as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
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
}
