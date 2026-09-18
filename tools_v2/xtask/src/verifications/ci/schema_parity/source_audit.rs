use super::*;

pub(super) use crate::verifications::architecture::wave_gate_sources::WAVE_CHILDREN;
use crate::verifications::architecture::wave_gate_sources::wave_children_are_linked;

/// Checks the CI recipe and both linked wave gate implementations, failing on unreadable inputs.
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
    let Some(mut wave) = read_wave_source(&wave_path) else {
        return Ok(1);
    };
    if !wave_children_are_linked(&wave) {
        println!("FAIL: gate.rs must declare and publicly export both wave gate implementations");
        return Ok(1);
    }
    for (module, _) in WAVE_CHILDREN {
        let child_path = wave_path.with_extension("").join(format!("{module}.rs"));
        let Some(child) = read_wave_source(&child_path) else {
            return Ok(1);
        };
        wave.push('\n');
        wave.push_str(&child);
    }

    let fail = run_pins(&src, Some(&wave));
    if fail != 0 {
        println!("verify-t468-ci-schema-parity: FAIL");
        return Ok(1);
    }
    println!("verify-t468-ci-schema-parity: PASS");
    Ok(0)
}

/// Reads each source owner independently so a facade cannot hide a missing implementation.
fn read_wave_source(path: &Path) -> Option<String> {
    if !path.is_file() {
        println!("FAIL: missing {}", path.display());
        return None;
    }
    match std::fs::read_to_string(path) {
        Ok(source) => Some(source),
        Err(error) => {
            println!("FAIL: cannot read {}: {error}", path.display());
            None
        }
    }
}

/// Port of the Python pin block. Returns `0` when clean, `1` when any pin failed.
pub(super) fn run_pins(ci_src: &str, wave: Option<&str>) -> i32 {
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
pub(super) fn ci_run_is_good(run: &str) -> bool {
    let normalised = run.split_whitespace().collect::<Vec<_>>().join(" ");
    normalised == GOOD_RUN || normalised == "cargo run -q -p xtask -- ci ci-local-schema"
}

/// THE RECIPE-BODY PINS, post-T-897 (see module docs for the before/after table).
///
/// Reads [`crate::commands::ci::task_runner::TASKS`] in-process. That is not a weaker subject than reading a file: the table
/// is what `cargo xtask ci` executes, so hollowing it is the only way to hollow the tasks, and a
/// hollowed row fails here. The old Makefile pins could only ever check TEXT that make happened
/// to run; these check the thing that runs.
pub(super) fn task_pins() -> i32 {
    let mut fail = 0;

    // Pin 1 — `ci-local-schema` must delegate to both halves of the T-434 set.
    match crate::commands::ci::task_runner::find("ci-local-schema") {
        None => {
            println!("FAIL: mk_ci::TASKS missing `ci-local-schema` (the CI pin would be vacuous)");
            fail = 1;
        }
        Some(t) => {
            let invoked: HashSet<&str> = crate::commands::ci::task_runner::invoked_tasks(t)
                .into_iter()
                .collect();
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
    match crate::commands::ci::task_runner::find("verify-t456") {
        None => {
            println!("FAIL: mk_ci::TASKS missing `verify-t456` (T-467/T-476 pin vacuous)");
            fail = 1;
        }
        Some(t) => {
            if !t
                .steps
                .iter()
                .any(|s| crate::commands::ci::task_runner::step_echo(s) == Some(TASK_ECHO_T456))
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
    match crate::commands::ci::task_runner::find("ci-local") {
        None => {
            println!("FAIL: mk_ci::TASKS missing `ci-local` (T-486 self-pin vacuous)");
            fail = 1;
        }
        Some(t) => {
            if !t
                .steps
                .iter()
                .any(|s| crate::commands::ci::task_runner::step_echo(s) == Some(TASK_ECHO_T468))
            {
                println!("FAIL: `ci-local` must invoke `{TASK_ECHO_T468}` directly (T-486/T-489)");
                println!("      found steps:");
                for s in t.steps {
                    println!("        {}", describe_step(s));
                }
                fail = 1;
            }
            if crate::commands::ci::task_runner::invoked_tasks(t).contains(&"verify-t468") {
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
    if crate::commands::ci::task_runner::find("verify-t468").is_some() {
        println!(
            "FAIL: mk_ci::TASKS grew a `verify-t468` row — T-489 requires t468 stay off the \
             dispatch table it polices"
        );
        fail = 1;
    }

    fail
}

/// One [`crate::commands::ci::task_runner::Step`] in the evidence dump. the wave driver tails 15 lines of a failed gate, so the
/// operator has to be able to see WHICH step was mistaken for an invocation.
pub(super) fn describe_step(s: &crate::commands::ci::task_runner::Step) -> String {
    match crate::commands::ci::task_runner::step_echo(s) {
        Some(echo) => format!("'{}'", py_repr_ascii(echo)),
        None => match s {
            crate::commands::ci::task_runner::Step::Task(n) => format!("Step::Task({n:?})"),
            _ => "Step::Native".to_string(),
        },
    }
}

/// Pin rust `gate_slice` + `cmd_gate` to VERIFY_STEPS rows + checkrun argv for t456/t468
/// (gate_t440 dual-path discipline). Hollow `r.run(label, || 0)` must fail.
pub(super) fn wave_pins(wave: &str) -> i32 {
    let stripped = strip_hash_comments(wave);
    let mut fail = 0;
    for (ticket, row) in [("T-456", ROW_T456), ("T-468", ROW_T468)] {
        if !stripped.contains(row) {
            println!("FAIL: gate.rs VERIFY_STEPS missing {ticket} row (dual-path pin)");
            fail = 1;
        }
    }
    for (name, role) in [("gate_slice", "slice gate"), ("cmd_gate", "cold gate")] {
        let Some(body) = extract_fn_body(&stripped, name) else {
            println!("FAIL: gate.rs missing `{name}()` ({role}) after comment strip");
            fail = 1;
            continue;
        };
        if !body.contains(VERIFY_LOOP) {
            println!(
                "FAIL: gate.rs `{name}()` ({role}) does not iterate VERIFY_STEPS                  (T-456/T-468 dual-path pin)"
            );
            fail = 1;
        }
        if !body.contains(CHECKRUN_ARGV) {
            println!(
                "FAIL: gate.rs `{name}()` ({role}) does not invoke checkrun verify via                  VERIFY_STEPS name"
            );
            fail = 1;
        }
    }
    fail
}

/// Strip `#` comments outside quotes — same discipline as gate_t440 so a commented
/// `run "T-456 …"` cannot satisfy the pin.
pub(super) fn strip_hash_comments(text: &str) -> String {
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
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
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
pub(super) fn extract_fn_body<'a>(src: &'a str, fn_name: &str) -> Option<&'a str> {
    let opener = Regex::new(&format!(
        r"(?m)^(?:pub\s+)?fn {}\s*\(",
        regex::escape(fn_name),
    ))
    .expect("fn opener");
    let m = opener.find(src)?;
    // T-902: rust `pub fn name(` — signature may span lines (`-> u8 {`).
    let brace = src[m.start()..].find('{')?;
    let start = m.start() + brace;
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
pub(super) fn py_repr_ascii(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\'', "\\'")
}
