//! Agent context guards and output filtering (token-efficiency rework).
//!
//! MEASURED BASELINE — editor waves 100-120, one 13-wave run, 2,355,581,888 input-side tokens (P).
//! Derived exactly from the `usage` records in the session transcripts; no token estimation.
//! Cumulative cost is `sum over turns of (context at that turn)`, so a token entering context at
//! turn n of a T-turn agent is re-read T-n times. That multiplier is the whole problem:
//!
//!   Read tool results resident   1,019,060,802   43.3% of P
//!     of which RE-READS of a path the same agent already read:
//!                                  166,485,070    7.1% of P   (618 of 987 Read calls — 63%)
//!   Bash tool results resident     409,407,866   17.4% of P
//!     grep/rg                                     8.96% of P
//!     head/sed/cat/tail extraction                6.25% of P
//!     cargo/make/trunk/wave.sh gate               1.95% of P
//!
//! Prompt instructions do not hold here: an agent that is stuck re-reads the file anyway. These
//! guards are PreToolUse hooks, so the harness refuses the call regardless of intent.
//!
//! TWO NON-NEGOTIABLES, both learned from this repo's own history:
//!
//! 1. FAIL OPEN. A guard that wedges an agent costs far more than the tokens it saves. Every
//!    unexpected condition — unparseable input, unreadable file, missing state dir — exits 0.
//!    The guard may only ever deny on a rule it positively matched.
//! 2. THE FILTER MAY NEVER HIDE A FAILURE. `ai run` is a token filter, not a verdict filter. A
//!    non-zero exit prints the raw tail; gate verdict lines pass through byte-for-byte. The
//!    recurring defect in this codebase is a tool reporting success over input it never examined
//!    (PLATFORM_FACTORY.md — always a BLOCKER); a filter that swallowed a red would be exactly
//!    that defect, built on purpose.

use anyhow::Result;
use serde_json::Value;
use std::io::Read as _;
use std::path::{Path, PathBuf};

/// Whole-file reads above this many lines are refused without an explicit range.
/// 400 is a starting point, tuned on wave 121 against the measured baseline of 131 Read calls
/// that each exceeded 4k tokens and together account for 37% of all tool-result residency.
const BIG_FILE_LINES: usize = 400;

/// Where the per-session read-set lives. Keyed by the harness `session_id`, so two concurrent
/// slice agents never share a set.
fn state_path(session_id: &str) -> PathBuf {
    let safe: String = session_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    std::env::temp_dir()
        .join("tbd-aiguard")
        .join(format!("{safe}.reads"))
}

/// A hook registered at BOTH user and project level fires twice for one tool call. Without a
/// guard against that, the first invocation records the path and the second denies it — every
/// first read in the project would be refused. There is no tool-call id in the PreToolUse
/// payload to key on, so the discriminator is time: a double-fire lands within milliseconds,
/// while a genuine re-read is at least one model turn (seconds) later.
const SAME_CALL_WINDOW_MS: u128 = 2_000;

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Milliseconds since this path was last recorded in this session, or None if never.
fn last_read_ms_ago(session: &str, path: &str) -> Option<u128> {
    let body = std::fs::read_to_string(state_path(session)).ok()?;
    let now = now_ms();
    body.lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(_, p)| *p == path)
        .filter_map(|(ts, _)| ts.parse::<u128>().ok())
        .map(|ts| now.saturating_sub(ts))
        .min()
}

fn record_read(session: &str, path: &str) {
    let p = state_path(session);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    use std::io::Write as _;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
    {
        let _ = writeln!(f, "{}\t{path}", now_ms());
    }
}

fn line_count(path: &Path) -> Option<usize> {
    let meta = std::fs::metadata(path).ok()?;
    // Do not stat-and-read anything enormous just to count lines; a >4 MB file is refused on size.
    if meta.len() > 4 * 1024 * 1024 {
        return Some(usize::MAX);
    }
    let body = std::fs::read_to_string(path).ok()?;
    Some(body.lines().count())
}

/// A shell command split into top-level segments — `;`, `&&`, `||`, `|`. Crude on purpose: it is
/// only used to answer "is the FIRST word of some segment a bare file-reader", and it must never
/// be clever enough to produce a false deny.
fn segments(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = cmd.chars().peekable();
    let (mut sq, mut dq) = (false, false);
    while let Some(c) = chars.next() {
        match c {
            '\'' if !dq => {
                sq = !sq;
                cur.push(c);
            }
            '"' if !sq => {
                dq = !dq;
                cur.push(c);
            }
            ';' | '|' | '&' if !sq && !dq => {
                // consume a doubled operator (&& / ||)
                if chars.peek() == Some(&c) {
                    chars.next();
                }
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn first_word(seg: &str) -> String {
    seg.split_whitespace().next().unwrap_or("").to_string()
}

/// The Bash guard.
///
/// Denies only two unambiguous shapes, both with a bounded built-in tool as the replacement:
///
///   * a search whose output is not capped        -> the Grep tool (`head_limit`, gitignore-aware)
///   * a bare file-reader as the WHOLE command    -> the Read tool (`offset`/`limit`)
///
/// Explicitly NOT denied, because each is either legitimate or already the behaviour we want:
///   `git log --grep=`/`git grep` (git, not a repo scan) · anything piped into `head`/`tail`
///   (that IS the cap) · `grep` reading from a pipe (already bounded by its producer) · `wc -l`.
fn guard_bash(cmd: &str) -> Option<String> {
    let segs = segments(cmd);
    for (i, seg) in segs.iter().enumerate() {
        let seg_trim = seg.trim();
        let w = first_word(seg_trim);
        let base = w.rsplit('/').next().unwrap_or(&w);
        let piped_from_previous = i > 0;

        // 1. Uncapped repo search.
        if matches!(base, "rg" | "grep" | "egrep" | "fgrep" | "ag") && !piped_from_previous {
            // Reading stdin from a prior segment is already bounded; only a fresh scan is a risk.
            let capped_later = segs[i + 1..]
                .iter()
                .any(|s| matches!(first_word(s.trim()).as_str(), "head" | "wc" | "tail"));
            let self_capped = seg_trim.contains(" -m ")
                || seg_trim.contains("--max-count")
                || seg_trim.contains(" -c ")
                || seg_trim.contains(" --count");
            if !capped_later && !self_capped {
                return Some(format!(
                    "Uncapped repo search: `{}`.\n\
                     Use the Grep tool instead — it caps output with head_limit, honours \
                     .gitignore, and supports output_mode=files_with_matches / count / content.\n\
                     If you must use Bash here, cap it: append `| head -50`, or pass `-m 50`.\n\
                     (Measured: uncapped grep/rg is 8.96% of this program's entire token bill.)",
                    seg_trim.chars().take(120).collect::<String>()
                ));
            }
        }

        // 2. A bare file reader used as the whole command.
        if matches!(base, "cat" | "head" | "tail" | "sed" | "nl") && segs.len() == 1 {
            let reads_a_file = seg_trim
                .split_whitespace()
                .skip(1)
                .any(|a| !a.starts_with('-') && (a.contains('/') || a.contains('.')));
            if reads_a_file {
                return Some(format!(
                    "Bare file read via Bash: `{}`.\n\
                     Use the Read tool with `offset`/`limit` — it is range-bounded and its result \
                     is what the transcript keeps.\n\
                     (Measured: head/sed/cat/tail file extraction is 6.25% of this program's \
                     entire token bill.)",
                    seg_trim.chars().take(120).collect::<String>()
                ));
            }
        }
    }
    None
}

/// The Read guard. Returns a deny message, or None to allow.
fn guard_read(session: &str, input: &Value) -> Option<String> {
    let path = input.get("file_path")?.as_str()?;

    // A deliberate ranged read is ALWAYS legal — that is the behaviour we are steering toward,
    // and re-reading a different span of a file you have seen is legitimate work.
    let ranged = input.get("offset").is_some() || input.get("limit").is_some();
    if ranged {
        record_read(session, path);
        return None;
    }

    // Same tool call, hook registered twice (user level + project level): allow and do not
    // re-record. Only a read from an EARLIER turn counts as a re-read.
    let seen_ms_ago = last_read_ms_ago(session, path);
    if seen_ms_ago.is_some_and(|ms| ms < SAME_CALL_WINDOW_MS) {
        return None;
    }

    if seen_ms_ago.is_some() {
        return Some(format!(
            "Already read in full this session: {path}\n\
             Scroll back in the transcript — the content is still there. If you need a specific \
             span again, re-read it with `offset`/`limit`, which is allowed.\n\
             (Measured: re-reads were 618 of 987 Read calls and 7.1% of this program's entire \
             token bill.)"
        ));
    }

    if let Some(n) = line_count(Path::new(path)) {
        if n > BIG_FILE_LINES {
            let shown = if n == usize::MAX {
                ">4MB".into()
            } else {
                n.to_string()
            };
            return Some(format!(
                "Whole-file read of a large file: {path} ({shown} lines).\n\
                 Locate first, then read the span: use the Grep tool for the symbol, then Read \
                 with `offset`/`limit`. Pass either one and this call is allowed.\n\
                 (Measured: 131 reads over 4k tokens accounted for 37% of all tool-result \
                 residency; the worst single call cost 4.77M token-turns.)"
            ));
        }
    }

    record_read(session, path);
    None
}

/// PreToolUse hook entry point. Reads the harness hook JSON on stdin.
/// exit 0 = allow, exit 2 = deny with the reason on stderr. Anything unexpected = allow.
pub fn cmd_guard() -> u8 {
    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() {
        return 0; // fail open
    }
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return 0; // fail open
    };

    let tool = v.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let session = v
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("nosession");
    let empty = Value::Object(Default::default());
    let input = v.get("tool_input").unwrap_or(&empty);

    let deny = match tool {
        "Read" => guard_read(session, input),
        "Bash" => input
            .get("command")
            .and_then(Value::as_str)
            .and_then(guard_bash),
        _ => None,
    };

    match deny {
        Some(msg) => {
            eprintln!("{msg}");
            // 2 is the PreToolUse "block this call" contract; stderr is shown to the agent.
            2
        }
        None => 0,
    }
}

// ---------------------------------------------------------------------------
// Output filter
// ---------------------------------------------------------------------------

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
        "skip:", // a test printing `skip:` is a FAIL in this program, never a pass
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
    // `test some::name ... ok` — a passing test tells us nothing the summary does not.
    if t.starts_with("test ") && (t.ends_with("... ok") || t.ends_with("... ignored")) {
        return true;
    }
    if t.starts_with("running ") && t.contains(" test") {
        return true;
    }
    false
}

/// `xtask ai run -- <command...>` — run a command, print a filtered view of its output.
///
/// Passing tests and progress chatter are dropped; failures, verdicts, and the tail always
/// survive. On a non-zero exit the raw tail is printed as well, so a red can never be filtered
/// into a green. The dropped-line count is always reported, so the filter can never quietly
/// swallow something without saying it did.
pub fn cmd_run(args: &[String]) -> Result<u8> {
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
            context_left = 50;
            keep[i] = true;
        } else if context_left > 0 && !is_noise(l) {
            context_left -= 1;
            keep[i] = true;
        }
    }
    // The tail always survives — verdict blocks live there.
    let tail_start = lines.len().saturating_sub(20);
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
        let raw_from = lines.len().saturating_sub(80);
        for l in &lines[raw_from..] {
            println!("{l}");
        }
    }

    Ok(u8::try_from(code).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- the guard denies what it should ----

    #[test]
    fn uncapped_grep_is_denied() {
        assert!(guard_bash("rg 'fn place_at' apps/website/frontend/src").is_some());
        assert!(guard_bash("grep -rn TODO .").is_some());
    }

    #[test]
    fn bare_file_read_is_denied() {
        assert!(guard_bash("cat apps/website/frontend/src/mission_editor.rs").is_some());
        assert!(guard_bash("sed -n '1,200p' xtask/src/main.rs").is_some());
    }

    // ---- the guard permits what it must (a guard that traps an agent is worse than the
    // tokens it saves, so these are the load-bearing cases) ----

    #[test]
    fn capped_search_is_allowed() {
        assert!(guard_bash("rg 'fn place_at' src | head -50").is_none());
        assert!(guard_bash("rg -m 20 'fn place_at' src").is_none());
        assert!(guard_bash("grep --count TODO src").is_none());
    }

    #[test]
    fn git_is_never_touched() {
        assert!(guard_bash("git log --grep='^wave [0-9]+ CLOSED' -1").is_none());
        assert!(guard_bash("git grep -n foo").is_none());
    }

    #[test]
    fn grep_reading_from_a_pipe_is_allowed() {
        // Already bounded by whatever produced the stream.
        assert!(guard_bash("cargo test 2>&1 | grep FAILED").is_none());
    }

    #[test]
    fn head_as_a_cap_is_allowed() {
        assert!(guard_bash("ls -la | head -20").is_none());
    }

    #[test]
    fn ranged_read_is_always_allowed_even_when_repeated() {
        let s = "test-ranged";
        let _ = std::fs::remove_file(state_path(s));
        let inp = json!({"file_path": "/etc/hostname", "offset": 1, "limit": 10});
        assert!(guard_read(s, &inp).is_none());
        assert!(
            guard_read(s, &inp).is_none(),
            "ranged re-read must stay legal"
        );
    }

    /// Seed the state file with a read that happened `ms_ago` milliseconds ago, so re-read
    /// semantics can be tested without sleeping.
    fn seed_read(session: &str, path: &str, ms_ago: u128) {
        let p = state_path(session);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, format!("{}\t{path}\n", now_ms().saturating_sub(ms_ago))).unwrap();
    }

    #[test]
    fn whole_read_of_a_path_read_in_an_earlier_turn_is_denied() {
        let s = "test-reread";
        let _ = std::fs::remove_file(state_path(s));
        let f = std::env::temp_dir().join("tbd-aiguard-reread.txt");
        std::fs::write(&f, "one\ntwo\n").unwrap();
        let inp = json!({ "file_path": f.to_str().unwrap() });
        assert!(guard_read(s, &inp).is_none(), "first read allowed");
        // A genuine re-read happens at least one model turn later.
        seed_read(s, f.to_str().unwrap(), SAME_CALL_WINDOW_MS + 1_000);
        assert!(
            guard_read(s, &inp).is_some(),
            "re-read from an earlier turn must be denied"
        );
    }

    /// THE REGRESSION THAT MOTIVATED THE TIME WINDOW: with the hook registered at both user
    /// and project level, one tool call invokes the guard twice within milliseconds. Before the
    /// window existed, the second invocation denied the first read of every file in the project.
    #[test]
    fn hook_registered_twice_does_not_deny_a_first_read() {
        let s = "test-doublefire";
        let _ = std::fs::remove_file(state_path(s));
        let f = std::env::temp_dir().join("tbd-aiguard-double.txt");
        std::fs::write(&f, "one\ntwo\n").unwrap();
        let inp = json!({ "file_path": f.to_str().unwrap() });
        assert!(guard_read(s, &inp).is_none(), "user-level hook allows");
        assert!(
            guard_read(s, &inp).is_none(),
            "project-level hook fires for the SAME call and must also allow"
        );
        // ...and a third, still inside the window (three hook layers) is fine too.
        assert!(guard_read(s, &inp).is_none());
    }

    #[test]
    fn large_whole_file_read_is_denied_but_ranged_is_not() {
        let s = "test-big";
        let _ = std::fs::remove_file(state_path(s));
        let f = std::env::temp_dir().join("tbd-aiguard-big.txt");
        std::fs::write(&f, "x\n".repeat(BIG_FILE_LINES + 1)).unwrap();
        let whole = json!({ "file_path": f.to_str().unwrap() });
        assert!(guard_read(s, &whole).is_some());
        let ranged = json!({ "file_path": f.to_str().unwrap(), "limit": 50 });
        assert!(guard_read("test-big-2", &ranged).is_none());
    }

    #[test]
    fn missing_file_fails_open() {
        let s = "test-missing";
        let _ = std::fs::remove_file(state_path(s));
        let inp = json!({"file_path": "/nonexistent/nope.rs"});
        assert!(
            guard_read(s, &inp).is_none(),
            "unreadable target must fail open"
        );
    }

    // ---- the filter cannot hide a failure ----

    #[test]
    fn verdict_and_failure_lines_always_survive() {
        for l in [
            "GATE: FAIL",
            "SLICE GATE: PASS",
            "test result: FAILED. 1 passed; 1 failed",
            "thread 'x' panicked at src/lib.rs:1:1",
            "error[E0308]: mismatched types",
            "skip: db unavailable",
            "REFUSING to pass — resolved to NO crate",
        ] {
            assert!(is_load_bearing(l), "must never be filtered: {l}");
        }
    }

    #[test]
    fn only_chatter_is_treated_as_noise() {
        assert!(is_noise("   Compiling website-frontend v0.1.0"));
        assert!(is_noise("test mission::places_entity ... ok"));
        assert!(!is_noise("test result: FAILED. 0 passed; 3 failed"));
        assert!(!is_noise("error: could not compile"));
    }
}
