//! The filtered command runner.
//!
//! **Role:** [`run_filtered_command`] runs a command through `sh -c` and prints a bounded view of
//! its combined output: every line that carries a verdict or a failure marker, up to 50
//! non-chatter lines after each such line, the last 20 lines, and a count line
//! `[xtask ai run] exit=<code>  lines: <in> in, <shown> shown, <filtered> filtered`. Compiler
//! progress and passing `test … ok` lines are chatter.
//! **Position:** behind `cargo xtask ai run -- <command...>`, for agents and people running a
//! noisy build or test.
//! **Signals & state:** none held; spawns one `sh` child and waits for it.
//! **Invariants:** the filter is a token filter, never a verdict filter: a non-zero exit also
//! prints the raw last 80 lines unfiltered, the exit code returned is the command's own, and the
//! dropped-line count is always printed. A filter that hid a failure would be the defect this
//! repository's gates exist to refuse, a tool reporting success over input it never examined
//! (`documentation/runbooks/factory_waves/README.md`).

use crate::error::Result;

/// Lines after a load-bearing line that are kept with it, chatter excepted.
const CONTEXT_LINES_AFTER_LOAD_BEARING: usize = 50;
/// Final lines that are always kept, since verdict blocks live there.
const ALWAYS_KEPT_TAIL_LINES: usize = 20;
/// Final lines printed unfiltered after a non-zero exit.
const RAW_TAIL_LINES_ON_FAILURE: usize = 80;

/// Lines that must ALWAYS survive the filter. Verdicts, refusals, failures, and anything the
/// wave gate or the report schema requires pasted. When in doubt the line is kept — this list is
/// allowed to be over-inclusive, never under-inclusive.
fn is_load_bearing(l: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "GATE:",
        "SLICE GATE:",
        "REFUSING",
        "REFUSED",
        "CONTRADICTED",
        "error",
        "Error",
        "ERROR",
        "warning:",
        "panicked",
        "FAILED",
        "failed",
        "failures:",
        "test result:",
        "skip:", // a test printing `skip:` is a FAIL in this repository, never a pass
        "assertion",
        "left:",
        "right:",
        "-->",
        "cannot",
        "not found",
        "No such file",
        "Blocking waiting for file lock",
    ];
    NEEDLES.iter().any(|n| l.contains(n))
}

/// Lines that are pure progress chatter with no diagnostic value.
fn is_noise(l: &str) -> bool {
    let t = l.trim_start();
    if t.starts_with("Compiling")
        || t.starts_with("Downloaded")
        || t.starts_with("Downloading")
        || t.starts_with("Updating")
        || t.starts_with("Fresh")
        || t.starts_with("Installing")
    {
        return true;
    }
    // `test some::name ... ok` — a passing test tells nothing the summary does not.
    if t.starts_with("test ") && (t.ends_with("... ok") || t.ends_with("... ignored")) {
        return true;
    }
    if t.starts_with("running ") && t.contains(" test") {
        return true;
    }
    false
}

/// Runs `args`, joined with spaces, through `sh -c`, prints the filtered view of its output and
/// returns its exit code (1 when it has none, or when no command is given).
///
/// # Errors
///
/// [`crate::Error::Io`] when `sh` cannot be spawned or its output cannot be collected.
pub fn run_filtered_command(args: &[String]) -> Result<u8> {
    if args.is_empty() {
        eprintln!("usage: xtask ai run -- <command...>");
        return Ok(1);
    }
    let joined = args.join(" ");
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(&joined)
        .output()?;

    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    // Carriage-return progress bars collapse to their final state.
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.rsplit('\r').next().unwrap_or(l))
        .collect();

    let code = out.status.code().unwrap_or(1);
    // Decide per line index whether it survives, so the reported counts are derived from the
    // same decision the output is — a filter that miscounts what it dropped is not auditable.
    let mut keep = vec![false; lines.len()];
    // Keep a context window after any load-bearing line: a rustc error is useless without the
    // lines that follow it.
    let mut context_left = 0usize;
    for (i, l) in lines.iter().enumerate() {
        if is_load_bearing(l) {
            context_left = CONTEXT_LINES_AFTER_LOAD_BEARING;
            keep[i] = true;
        } else if context_left > 0 && !is_noise(l) {
            context_left -= 1;
            keep[i] = true;
        }
    }
    let tail_start = lines.len().saturating_sub(ALWAYS_KEPT_TAIL_LINES);
    for k in keep.iter_mut().skip(tail_start) {
        *k = true;
    }

    let kept: Vec<&str> = lines
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(l, _)| *l)
        .collect();
    let dropped = lines.len() - kept.len();

    for l in &kept {
        println!("{l}");
    }
    println!(
        "\n[xtask ai run] exit={code}  lines: {} in, {} shown, {dropped} filtered",
        lines.len(),
        kept.len()
    );

    // FAIL OPEN: a non-zero exit gets the raw tail on top of the filtered view, so no failure
    // can ever be hidden by this filter.
    if code != 0 {
        println!("\n[xtask ai run] NON-ZERO EXIT — raw tail follows, unfiltered:");
        let raw_from = lines.len().saturating_sub(RAW_TAIL_LINES_ON_FAILURE);
        for l in &lines[raw_from..] {
            println!("{l}");
        }
    }

    Ok(u8::try_from(code).unwrap_or(1))
}

#[cfg(test)]
#[path = "tests/output_filter_tests.rs"]
mod tests;
