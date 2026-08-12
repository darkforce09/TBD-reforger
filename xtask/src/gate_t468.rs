//! T-468 / T-471 / T-472 / T-476 / T-486 / T-489 — the CI schema job must stay on the FULL gate
//! set, and the Class-R verify tasks must stay real (T-853 / T-881 port of
//! `scripts/mod/verify-t468-ci-schema-parity.sh`).
//!
//! Without this tripwire, someone can revert the CI schema job to bare
//! `cargo run -p xtask -- schema validate` + citations and reopen the map-object-enums
//! hole while CI stays green. The task pins close hollow `@true` / `echo PASS` /
//! `#fake` smuggles on `ci-local-schema`, `verify-t456`, and this gate's own invocation.
//!
//! ── T-853 MUTUAL PIN ─────────────────────────────────────────────────────────────────────────
//!
//! Pre-port, `bash_pins` required `^\t@?bash <exact-script-path>` for both verify-t456 and
//! verify-t468. Porting either alone fails the other on a CORRECT tree. Both bash pins were
//! dropped and re-pinned at the cargo spelling in ONE atomic change with the Makefile /
//! wave.sh / ci.yml call sites. [`VERIFY_T456`] / [`VERIFY_T468`] are the single source; test
//! fixtures derive from them (gate_t440 precedent).
//!
//! ── T-897: THE RECIPE BODIES MOVED TO `mk_ci::TASKS` ─────────────────────────────────────────
//!
//! Three pins read the root `Makefile`. T-897 deleted it — and this gate was FAIL-CLOSED on that
//! file, so it would have gone RED rather than quiet. Their successor is [`crate::mk_ci::TASKS`],
//! the table `cargo xtask ci <task>` executes:
//!
//! | pinned in `Makefile` until T-897 | pinned in `TASKS` now |
//! |---|---|
//! | `ci-local-schema:` invokes `schema-validate` + `verify-citations` | that row's `Step::Task`s |
//! | `verify-t456:` recipe is exactly the cargo verify line | that row's step echo |
//! | `verify-t468:` recipe is exactly the cargo verify line (self-pin) | `ci-local`'s DIRECT `verify t468` step |
//!
//! The self-pin's subject MOVED rather than vanishing, and the T-489 circularity it exists for is
//! now preserved by construction: there is deliberately no `verify-t468` row in `TASKS`, so
//! `ci-local` reaches this gate through a direct `Step::Xtask` and never through
//! `Step::Task("verify-t468")`. A hollowed dispatcher therefore cannot green the check that
//! polices dispatch, and that one step is what the third pin reads.
//!
//! `scripts/platform/wave.sh` is examined (not merely claimed): both `gate_slice` and `cmd_gate`
//! must carry exact `checkrun` + cargo verify lines for t456 and t468 (T-478 dual-path discipline).
//! A hollow `run "…" true` must RED this gate.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely.** The script was a heredoc owning YAML comment strip + recipe
//!    pins. Ported in-process; the `scripts/python-inventory.txt` line dies with it.
//! 2. **`pin_out="$(…)" || pin_rc=$?` swallow.** A Python crash still surfaced via non-zero
//!    status, but stderr was discarded when the capture only kept stdout. Failures print
//!    directly here.
//!
//! T-489 circularity (preserved): CI / ci-local / wave invoke this gate *directly* (cargo),
//! never through a dispatch table row named after it. A hollowed indirection must not green
//! those callers.

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use regex::Regex;

use crate::mk_ci;

/// Cargo spelling pinned into `wave.sh`'s checkrun lines for t456. Const + call sites are one
/// atomic change — see module docs.
const VERIFY_T456: &str = "cargo run -q -p xtask -- verify t456";
/// Cargo spelling pinned into `wave.sh`'s checkrun lines for t468 (self-pin).
const VERIFY_T468: &str = "cargo run -q -p xtask -- verify t468";
/// The `TASKS` step echo `verify-t456` must carry — the in-table spelling of [`VERIFY_T456`].
const TASK_ECHO_T456: &str = "cargo xtask verify t456";
/// The `TASKS` step echo `ci-local` must carry for t468. THE SELF-PIN (T-486/T-489): `ci-local`
/// must reach this gate directly, not via a `verify-t468` row that could be hollowed.
const TASK_ECHO_T468: &str = "cargo xtask verify t468";

const CI_REL: &str = ".github/workflows/ci.yml";
const WAVE_REL: &str = "scripts/platform/wave.sh";
/// What the ci.yml `schema` job must run. `cargo xtask` is the `.cargo/config.toml` alias for
/// `cargo run --package xtask --`; [`ci_run_is_good`] accepts either spelling.
const GOOD_RUN: &str = "cargo xtask ci ci-local-schema";

/// Entry point. Bash collapsed any Python non-zero to exit 1 after printing FAIL — same here.
pub fn verify_t468(repo_root: &Path) -> Result<u8> {
    let ci_path = repo_root.join(CI_REL);
    if !ci_path.is_file() {
        println!("FAIL: missing {}", ci_path.display());
        return Ok(1);
    }

    let src = match std::fs::read_to_string(&ci_path) {
        Ok(s) => s,
        Err(e) => {
            println!("FAIL: cannot read {}: {e}", ci_path.display());
            return Ok(1);
        }
    };

    let wave_path = repo_root.join(WAVE_REL);
    let wave = if wave_path.is_file() {
        match std::fs::read_to_string(&wave_path) {
            Ok(s) => Some(s),
            Err(e) => {
                println!("FAIL: cannot read {}: {e}", wave_path.display());
                return Ok(1);
            }
        }
    } else {
        println!("FAIL: missing {}", wave_path.display());
        return Ok(1);
    };

    let fail = run_pins(&src, wave.as_deref());
    if fail != 0 {
        println!("verify-t468-ci-schema-parity: FAIL");
        return Ok(1);
    }
    println!("verify-t468-ci-schema-parity: PASS");
    Ok(0)
}

/// Port of the Python pin block. Returns `0` when clean, `1` when any pin failed.
fn run_pins(ci_src: &str, wave: Option<&str>) -> i32 {
    let stripped = strip_yaml_hash_comments(ci_src);
    let lines: Vec<&str> = stripped.lines().collect();

    let schema_job_re = Regex::new(r"^  schema:\s*$").expect("schema job");
    let schema_start = lines.iter().position(|line| schema_job_re.is_match(line));
    let Some(schema_start) = schema_start else {
        println!("FAIL: no top-level `schema:` job in .github/workflows/ci.yml");
        return 1;
    };

    let job_key = Regex::new(r"^  [A-Za-z0-9_-]+:\s*$").expect("job key");
    let mut schema_lines: Vec<&str> = Vec::new();
    for line in &lines[schema_start + 1..] {
        if job_key.is_match(line) {
            break;
        }
        schema_lines.push(line);
    }
    if schema_lines.is_empty() {
        println!("FAIL: schema job has empty body");
        return 1;
    }

    let run_re = Regex::new(r"^\s+-\s+run:\s*(.+?)\s*$|^\s+run:\s*(.+?)\s*$").expect("run step");
    let mut runs: Vec<String> = Vec::new();
    for line in &schema_lines {
        let Some(caps) = run_re.captures(line) else {
            continue;
        };
        let mut cmd = caps
            .get(1)
            .or_else(|| caps.get(2))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        if (cmd.starts_with('\'') && cmd.ends_with('\''))
            || (cmd.starts_with('"') && cmd.ends_with('"'))
        {
            cmd = cmd[1..cmd.len() - 1].to_string();
        }
        runs.push(cmd);
    }

    let mut fail = 0;

    if runs.is_empty() {
        println!("FAIL: schema job has no `run:` steps after comment strip");
        fail = 1;
    }

    let has_good = runs.iter().any(|r| ci_run_is_good(r));

    if !has_good {
        println!("FAIL: schema job must run `{GOOD_RUN}` (full gate set)");
        println!("      found run steps:");
        for r in &runs {
            println!("        - {r}");
        }
        println!("      Pre-T-434 hole: validate + citations alone misses map-object-enums.");
        fail = 1;
    }

    let narrow_re = Regex::new(r"schema\s+validate\b").expect("narrow");
    let narrow: Vec<&String> = runs
        .iter()
        .filter(|r| {
            narrow_re.is_match(r)
                && !r.contains("ci-local-schema")
                && !r.contains("schema-validate")
        })
        .collect();
    if !narrow.is_empty() && !has_good {
        println!("FAIL: schema job uses narrow `schema validate` without ci-local-schema:");
        for r in &narrow {
            println!("      {r}");
        }
        fail = 1;
    }

    fail |= task_pins();

    match wave {
        None => {
            println!("FAIL: missing {WAVE_REL} (T-456/T-468 dual-path pin vacuous)");
            fail = 1;
        }
        Some(w) => {
            fail |= wave_pins(w);
        }
    }

    fail
}

/// Does this ci.yml `run:` line invoke the full schema gate set?
///
/// Both spellings of the same command are accepted, because `.cargo/config.toml` defines
/// `xtask = "run --package xtask --"` and CI may use either. Nothing looser: a `|| true` or
/// `--help` suffix must not satisfy it, which is why this is equality after normalisation and
/// not a `contains`.
fn ci_run_is_good(run: &str) -> bool {
    let normalised = run.split_whitespace().collect::<Vec<_>>().join(" ");
    normalised == GOOD_RUN || normalised == "cargo run -q -p xtask -- ci ci-local-schema"
}

/// THE RECIPE-BODY PINS, post-T-897 (see module docs for the before/after table).
///
/// Reads [`mk_ci::TASKS`] in-process. That is not a weaker subject than reading a file: the table
/// is what `cargo xtask ci` executes, so hollowing it is the only way to hollow the tasks, and a
/// hollowed row fails here. The old Makefile pins could only ever check TEXT that make happened
/// to run; these check the thing that runs.
fn task_pins() -> i32 {
    let mut fail = 0;

    // Pin 1 — `ci-local-schema` must delegate to both halves of the T-434 set.
    match mk_ci::find("ci-local-schema") {
        None => {
            println!("FAIL: mk_ci::TASKS missing `ci-local-schema` (the CI pin would be vacuous)");
            fail = 1;
        }
        Some(t) => {
            let invoked: HashSet<&str> = mk_ci::invoked_tasks(t).into_iter().collect();
            let missing: Vec<&str> = ["schema-validate", "verify-citations"]
                .into_iter()
                .filter(|need| !invoked.contains(need))
                .collect();
            if missing.is_empty() {
            } else {
                println!(
                    "FAIL: `ci-local-schema` must invoke: {} (T-472: real Step::Task rows, \
                     not an echo and not a comment)",
                    missing.join(", ")
                );
                println!("      found steps:");
                for s in t.steps {
                    println!("        {}", describe_step(s));
                }
                fail = 1;
            }
        }
    }

    // Pin 2 — `verify-t456` must still carry the cargo verify call.
    match mk_ci::find("verify-t456") {
        None => {
            println!("FAIL: mk_ci::TASKS missing `verify-t456` (T-467/T-476 pin vacuous)");
            fail = 1;
        }
        Some(t) => {
            if !t
                .steps
                .iter()
                .any(|s| mk_ci::step_echo(s) == Some(TASK_ECHO_T456))
            {
                println!("FAIL: `verify-t456` must invoke: {TASK_ECHO_T456}");
                println!("      found steps:");
                for s in t.steps {
                    println!("        {}", describe_step(s));
                }
                println!(
                    "      T-476/T-486: exact echo, not a hollow Step::Cmd/echo and not a rename."
                );
                fail = 1;
            }
        }
    }

    // Pin 3 — THE SELF-PIN. `ci-local` must reach this gate DIRECTLY.
    //
    // T-489/T-881 circularity: routing t468 through a `Step::Task("verify-t468")` would let a
    // hollowed dispatcher green the very tripwire that polices dispatch. So the row must carry an
    // echoing step, and there must be no `verify-t468` task for anyone to reach instead.
    match mk_ci::find("ci-local") {
        None => {
            println!("FAIL: mk_ci::TASKS missing `ci-local` (T-486 self-pin vacuous)");
            fail = 1;
        }
        Some(t) => {
            if !t
                .steps
                .iter()
                .any(|s| mk_ci::step_echo(s) == Some(TASK_ECHO_T468))
            {
                println!("FAIL: `ci-local` must invoke `{TASK_ECHO_T468}` directly (T-486/T-489)");
                println!("      found steps:");
                for s in t.steps {
                    println!("        {}", describe_step(s));
                }
                fail = 1;
            }
            if mk_ci::invoked_tasks(t).contains(&"verify-t468") {
                println!(
                    "FAIL: `ci-local` reaches t468 through Step::Task(\"verify-t468\") — the \
                     T-489 circularity is back"
                );
                println!(
                    "      A hollowed dispatch table would then green the gate that polices it."
                );
                fail = 1;
            }
        }
    }
    if mk_ci::find("verify-t468").is_some() {
        println!(
            "FAIL: mk_ci::TASKS grew a `verify-t468` row — T-489 requires t468 stay off the \
             dispatch table it polices"
        );
        fail = 1;
    }

    fail
}

/// One [`mk_ci::Step`] in the evidence dump. `wave.sh` tails 15 lines of a failed gate, so the
/// operator has to be able to see WHICH step was mistaken for an invocation.
fn describe_step(s: &mk_ci::Step) -> String {
    match mk_ci::step_echo(s) {
        Some(echo) => format!("'{}'", py_repr_ascii(echo)),
        None => match s {
            mk_ci::Step::Task(n) => format!("Step::Task({n:?})"),
            _ => "Step::Native".to_string(),
        },
    }
}

/// Pin `wave.sh` `gate_slice` + `cmd_gate` to exact `checkrun` + cargo verify for t456/t468
/// (gate_t440 dual-path discipline). Hollow `run "…" true` must fail.
fn wave_pins(wave: &str) -> i32 {
    let stripped = strip_hash_comments(wave);
    let mut fail = 0;
    let pins = [("T-456", VERIFY_T456), ("T-468", VERIFY_T468)];
    for (name, role) in [("gate_slice", "slice gate"), ("cmd_gate", "cold gate")] {
        let Some(body) = extract_fn_body(&stripped, name) else {
            println!("FAIL: wave.sh missing `{name}()` ({role}) after comment strip");
            fail = 1;
            continue;
        };
        for (ticket, cmd) in pins {
            let checkrun_cmd = format!("checkrun {cmd}");
            if !body.contains(&checkrun_cmd) {
                println!(
                    "FAIL: wave.sh `{name}()` ({role}) does not invoke {checkrun_cmd} \
                     ({ticket} dual-path pin)"
                );
                fail = 1;
                continue;
            }
            // End-anchored run line: reject `checkrun … --help` / `|| true` smuggles.
            let re = Regex::new(&format!(
                r#"(?m)^\s*run\s+"{ticket}[^"]*"\s+checkrun\s+{}\s*$"#,
                regex::escape(cmd)
            ))
            .expect("wave pin re");
            if !re.is_match(body) {
                println!("FAIL: wave.sh `{name}()` ({role}) missing exact checkrun line for {cmd}");
                fail = 1;
            }
        }
    }
    fail
}

/// Strip `#` comments outside quotes — same discipline as gate_t440 so a commented
/// `run "T-456 …"` cannot satisfy the pin.
fn strip_hash_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let (mut i, mut in_squote, mut in_dquote) = (0usize, false, false);
    while i < n {
        let c = chars[i];
        if in_squote {
            out.push(c);
            if c == '\'' && !(i + 1 < n && chars[i + 1] == '\'') {
                in_squote = false;
            } else if c == '\'' && i + 1 < n && chars[i + 1] == '\'' {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_dquote {
            out.push(c);
            if c == '\\' && i + 1 < n {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_squote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_dquote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Brace-balanced `{ … }` body of a shell function (gate_t440 precedent).
fn extract_fn_body<'a>(src: &'a str, fn_name: &str) -> Option<&'a str> {
    let opener =
        Regex::new(&format!(r"(?m)^{}\(\)\s*\{{", regex::escape(fn_name))).expect("fn opener");
    let m = opener.find(src)?;
    let start = m.end() - 1;
    let mut depth = 0i32;
    for (offset, ch) in src[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[start..start + offset + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Python `!r` for the ASCII recipe lines this gate dumps (tab → `\t`, single quotes).
fn py_repr_ascii(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\'', "\\'")
}

/// Strip `#` comments outside quotes. Preserves newlines for line structure.
fn strip_yaml_hash_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_squote = false;
    let mut in_dquote = false;
    while i < n {
        let c = chars[i];
        if in_squote {
            out.push(c);
            if c == '\'' && !(i + 1 < n && chars[i + 1] == '\'') {
                in_squote = false;
            } else if c == '\'' && i + 1 < n && chars[i + 1] == '\'' {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_dquote {
            out.push(c);
            if c == '\\' && i + 1 < n {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_squote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_dquote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal CI schema job that satisfies the run-step pin.
    fn ci_ok() -> String {
        format!(
            "jobs:\n  schema:\n    steps:\n      - run: {GOOD_RUN}\n  other:\n    steps:\n      - run: true\n"
        )
    }

    /// Dual-path wave fixture derived from VERIFY_* consts (gate_t440 precedent).
    fn wave_ok() -> String {
        format!(
            "gate_slice() {{\n  run \"T-456 REST size gate\"  checkrun {VERIFY_T456}\n  run \"T-468 CI schema parity\" checkrun {VERIFY_T468}\n}}\n\ncmd_gate() {{\n  run \"T-456 REST size gate\"  checkrun {VERIFY_T456}\n  run \"T-468 CI schema parity\" checkrun {VERIFY_T468}\n}}\n"
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
        assert!(mk_ci::find("verify-t468").is_none());
        let ci_local = mk_ci::find("ci-local").expect("ci-local row");
        assert!(!mk_ci::invoked_tasks(ci_local).contains(&"verify-t468"));
        assert!(
            ci_local
                .steps
                .iter()
                .any(|s| mk_ci::step_echo(s) == Some(TASK_ECHO_T468))
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

    /// M2: hollow wave `run "…" true` must RED (checkrun cargo gone).
    #[test]
    fn wave_hollow_t456_true_fails() {
        let wave = wave_ok().replacen(&format!("checkrun {VERIFY_T456}"), "true", 1);
        assert_ne!(pins(&wave), 0);
    }

    #[test]
    fn wave_hollow_both_paths_required() {
        let wave = wave_ok().replacen(&format!("checkrun {VERIFY_T456}"), "true", 1);
        // Only gate_slice hollowed; cmd_gate still has checkrun — still must FAIL.
        assert!(wave.matches(&format!("checkrun {VERIFY_T456}")).count() == 1);
        assert_ne!(pins(&wave), 0);
    }

    /// B1: arbitrary suffixes must NOT satisfy the wave checkrun pin.
    #[test]
    fn wave_suffix_smuggles_fail_pin() {
        for suffix in [" --help", " || true", " && true", " 2>/dev/null"] {
            for cmd in [VERIFY_T456, VERIFY_T468] {
                let wave = wave_ok().replace(cmd, &format!("{cmd}{suffix}"));
                assert_ne!(
                    pins(&wave),
                    0,
                    "suffix {suffix:?} on {cmd} must FAIL the pin"
                );
            }
        }
    }

    #[test]
    fn wave_commented_run_does_not_satisfy() {
        let wave = wave_ok().replacen("  run \"T-456", "  # run \"T-456", 1);
        assert_ne!(pins(&wave), 0);
    }

    #[test]
    fn missing_wave_fails() {
        assert_ne!(run_pins(&ci_ok(), None), 0);
    }
}
