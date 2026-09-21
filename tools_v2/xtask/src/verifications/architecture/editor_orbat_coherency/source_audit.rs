use super::*;

pub fn verify_editor_orbat_coherency(repo_root: &Path) -> Result<u8> {
    // Fail-fast: the first failure ends the run, and the two halves below keep their order.
    if let Err(msg) = static_checks(repo_root) {
        return Ok(fail(&msg));
    }
    if let Err(msg) = cargo_checks(repo_root) {
        return Ok(fail(&msg));
    }
    println!("editor-orbat-coherency: ALL PASS");
    Ok(0)
}

/// An `ok` line — stdout.
pub(super) fn ok(msg: &str) {
    println!("editor-orbat-coherency OK: {msg}");
}

/// A `FAIL` line — stderr, then exit 1. Stdout is flushed first: it is a `LineWriter` so the
/// order already holds, but every caller merges the two streams and the output contract should
/// not rest on a buffering policy.
pub(super) fn fail(msg: &str) -> u8 {
    let _ = std::io::stdout().flush();
    eprintln!("editor-orbat-coherency FAIL: {msg}");
    1 // Verdict::into_binary_exit_code()'s code, chosen deliberately — see the module docs.
}

pub(super) fn static_checks(root: &Path) -> Result<(), String> {
    for (msg, pat, ci, rels, ok_line) in BANS {
        let files: Vec<_> = rels.iter().map(|r| root.join(r)).collect();
        let refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();
        match pattern(pat, *ci) {
            Some(p) => translate(gate::ban(msg, &p, &refs), msg, root, Kind::Ban)?,
            // A pattern this file cannot compile is reported the way a matcher error is: 2.
            None => return Err(tool_status(msg, 2)),
        }
        ok(ok_line);
    }
    for (msg, pat) in PINS {
        let file = root.join(SLOTS_GPU);
        match pattern(pat, false) {
            Some(p) => translate(gate::require(msg, &p, &[&file]), msg, root, Kind::Pin)?,
            None => return Err(tool_status(msg, 2)),
        }
    }
    ok("RGBA side pins present");
    Ok(())
}

/// `None` means the pattern would not compile — a bug in the tables above, and it must not read as
/// "the ban holds".
pub(super) fn pattern(pat: &str, ci: bool) -> Option<Pattern> {
    let p = Pattern::regex(pat).ok()?;
    if ci {
        p.case_insensitive().ok()
    } else {
        Some(p)
    }
}

/// Render a [`Verdict`] as the operator-facing line for that input. Exhaustive on purpose:
/// [`Verdict`] has no `bool` conversion, so "did not run" cannot be folded into "passed" here by
/// accident, and a new `NotRun` variant in the library breaks this arm rather than going green.
pub(super) fn translate(v: Verdict, msg: &str, root: &Path, kind: Kind) -> Result<(), String> {
    match v {
        Verdict::Held => Ok(()),
        // The bare message, for a violated ban and an absent pin alike.
        Verdict::Failed(_) => Err(msg.to_string()),
        Verdict::DidNotRun(NotRun::TargetMissing(p), _) => {
            let path = p.strip_prefix(root).unwrap_or(&p).display();
            let why = match kind {
                Kind::Ban => BAN_MISSING,
                Kind::Pin => "The pin could not be checked.",
            };
            Err(format!("{msg} — target file missing: {path}. {why}"))
        }
        // All the library can otherwise report here is a file that exists and could not be read,
        // which is a matcher error: 2. `ToolAbsent` is unreachable — the matcher is compiled in,
        // so that failure class is retired rather than asserted against.
        Verdict::DidNotRun(..) => Err(tool_status(msg, 2)),
    }
}

pub(super) fn tool_status(msg: &str, status: i32) -> String {
    format!(
        "{msg} — grep exited {status} (tool absent or bad pattern). Refusing to report OK on a \
         check that did not execute."
    )
}

pub(super) fn cargo_checks(root: &Path) -> Result<(), String> {
    // `$HOME/.cargo/bin` is prepended because the cargo half runs only on the host, where cargo
    // is a rustup shim under $HOME rather than on a system PATH.
    let inherited = std::env::var("PATH").unwrap_or_default();
    let path = match std::env::var("HOME") {
        Ok(home) => format!("{home}/.cargo/bin:{inherited}"),
        Err(_) => inherited,
    };
    for (pkg, feats, lib, sel, ok_line) in CARGO_PINS {
        let mut args = vec!["test", "-p", pkg];
        if let Some(f) = feats {
            args.extend_from_slice(&["--features", f]);
        }
        if *lib {
            args.push("--lib");
        }
        args.extend_from_slice(&[sel, "--", "--quiet"]);
        cargo_test_pin(root, &path, &args)?;
        if let Some(m) = ok_line {
            ok(m);
        }
    }
    Ok(())
}

/// The pin's arguments joined by a space, quoting dropped, so `--features "scenario store"`
/// renders as `--features scenario store`.
pub(super) fn shown(args: &[&str]) -> String {
    args[1..].join(" ")
}

pub(super) fn cargo_test_pin(root: &Path, path_env: &str, args: &[&str]) -> Result<(), String> {
    let label = shown(args);
    // `merged_output`, not `output`: cargo writes `Running unittests` to stderr while libtest
    // writes `running N tests` to stdout, and re-joining two separately-drained strings invents an
    // interleaving the child never produced. See the note on `Run::merged_output`.
    let run = verification_core::proc::Run::new("cargo")
        .args(args)
        .cwd(root)
        .env("PATH", path_env);
    match run.merged_output() {
        Ok(verification_core::proc::Merged { code, text, .. }) => {
            // EVERY trailing newline is stripped, which is why libtest's blank line after
            // `test result:` never appears between two pins in the log.
            println!("{}", text.trim_end_matches('\n'));
            classify(&label, code, &text)
        }
        Err(cause) => Err(format!("cargo test {label} — {}", not_run_clause(&cause))),
    }
}

/// Names the two states a single "exited N" line would misreport, both unreachable while cargo
/// runs at all.
///
/// **A child killed by SIGKILL has no exit code**: reporting a synthesised 128+n as an ordinary
/// numeric failure turns "the OOM killer shot the gate" into "the gate found a problem". And an
/// absent cargo reported as `exited 127` is the same tool-absent lie this gate exists to refuse.
/// Both are named causes here. The exit status is still 1, so callers behave identically; only
/// the text differs, on the inputs a bare status misdescribes. Exhaustive: a new `NotRun` variant
/// is a compile error, not a silent default.
pub(super) fn not_run_clause(cause: &NotRun) -> String {
    let tail = "Refusing to report OK on a check that did not execute.";
    match cause {
        NotRun::TargetMissing(p) => format!("target missing: {}. {tail}", p.display()),
        NotRun::Unreadable { path, source } => {
            format!("unreadable: {} ({source}). {tail}", path.display())
        }
        NotRun::ToolAbsent(tool) => format!("`{tool}` is ABSENT. {tail}"),
        NotRun::ToolError { tool, status, .. } => format!("`{tool}` failed ({status}). {tail}"),
        NotRun::Signalled { tool, signal } => format!(
            "`{tool}` was killed by signal {signal} — the process died, it did not report. {tail}"
        ),
        NotRun::Timeout { tool, secs } => {
            format!("`{tool}` exceeded {secs}s and was killed. {tail}")
        }
    }
}

/// The three `cargo_test_pin` verdicts, as a pure function of what cargo returned.
///
/// Split from the spawn so its arms are testable without a two-minute build — the whole point of
/// the wrapper is the case where cargo exits **0**, and a test that had to compile the map engine
/// to reach it would not get written.
pub(super) fn classify(label: &str, status: i32, out: &str) -> Result<(), String> {
    if status != 0 {
        return Err(format!("cargo test {label} exited {status}"));
    }
    // "No result line" and "0 passed" are different failures and both must be loud, so they are
    // classified separately.
    let counts = passed_counts(out);
    if counts.is_empty() {
        return Err(format!(
            "cargo test {label} — no 'test result: N passed' line. Refusing to report OK on a \
             check that did not execute."
        ));
    }
    if counts.iter().sum::<u64>() < 1 {
        return Err(format!(
            "cargo test {label} — 0 tests passed (selector matched nothing). A renamed/typo'd pin \
             must not silently empty."
        ));
    }
    Ok(())
}

/// One entry per matching LINE: how many result lines there are and how many tests they sum to
/// answer different questions, and both are used. `regex::Regex` rather than [`Pattern`]: this is
/// a parse needing a capture group, not a gate, and the per-line loop makes `multi_line` moot.
/// The leading `.*` is greedy, so a line carrying two `N passed` yields the LAST.
pub(super) fn passed_counts(out: &str) -> Vec<u64> {
    let re = Regex::new(r"test result:.* ([0-9][0-9]*) passed").expect("literal pattern compiles");
    out.lines()
        .filter_map(|l| re.captures(l))
        .filter_map(|c| c[1].parse().ok())
        .collect()
}
