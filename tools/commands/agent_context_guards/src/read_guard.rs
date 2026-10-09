//! The Read rules of the tool-call guard and the per-session read set they consult.
//!
//! **Role:** [`guard_read`] answers one Read call: a ranged read is always allowed, a whole-file
//! read of a path this session read in an earlier turn is denied, and so is a whole-file read of
//! a file over [`BIG_FILE_LINES`] lines or over 4 MB. Read results are the largest share of an
//! agent's resident context in the measured baseline of this repository's agent waves (43.3% of
//! all input-side tokens, re-reads of an already-read path alone 7.1%), which is what the rules
//! bound.
//! **Position:** called by `crate::tool_call_guard` for every `Read` hook payload; reads the wall
//! clock through the [`Clock`] it is handed (the platform clock live, a manual clock in tests).
//! **Signals & state:** the session's read set, `tbd-aiguard/<session_id>.reads` under the system
//! temp folder, one `<unix milliseconds>\t<path>` line appended per recorded read; keyed by the
//! harness session id, so concurrent agents never share one.
//! **Invariants:** fails open: an unreadable read set, an unreadable or missing target or a
//! payload without `file_path` allows the call; a second firing of the same call within
//! [`SAME_CALL_WINDOW_MS`] is allowed and not recorded again.

use serde_json::Value;
use std::path::{Path, PathBuf};
use time_source::Clock;

/// Whole-file reads above this many lines are refused without an explicit range. Measured: the
/// 131 Read calls that each exceeded 4k tokens account for 37% of all tool-result residency.
pub(crate) const BIG_FILE_LINES: usize = 400;

/// A hook registered at both user and project level fires twice for one tool call. The
/// PreToolUse payload carries no tool-call id, so the discriminator is time: a double firing
/// lands within milliseconds, while a genuine re-read is at least one model turn (seconds) later.
pub(crate) const SAME_CALL_WINDOW_MS: u64 = 2_000;

/// Files larger than this are refused on size alone, without being read to count their lines.
const BIG_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// Where the read set of `session_id` lives; characters other than ASCII letters, digits, `-`
/// and `_` are dropped from the id.
fn state_path(session_id: &str) -> PathBuf {
    let safe: String = session_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    std::env::temp_dir()
        .join("tbd-aiguard")
        .join(format!("{safe}.reads"))
}

/// Milliseconds since this path was last recorded in this session, or None if never.
fn last_read_ms_ago(session: &str, path: &str, clock: &dyn Clock) -> Option<u64> {
    let body = std::fs::read_to_string(state_path(session)).ok()?;
    let now = clock.now_unix_ms();
    body.lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(_, p)| *p == path)
        .filter_map(|(ts, _)| ts.parse::<u64>().ok())
        .map(|ts| now.saturating_sub(ts))
        .min()
}

/// Appends `path` to the session's read set, stamped with the clock's reading; best effort.
fn record_read(session: &str, path: &str, clock: &dyn Clock) {
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
        let _ = writeln!(f, "{}\t{path}", clock.now_unix_ms());
    }
}

/// The file's line count; `usize::MAX` for a file over [`BIG_FILE_BYTES`]; None when it cannot
/// be read.
fn line_count(path: &Path) -> Option<usize> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > BIG_FILE_BYTES {
        return Some(usize::MAX);
    }
    let body = std::fs::read_to_string(path).ok()?;
    Some(body.lines().count())
}

/// The Read guard. Returns a deny message, or None to allow.
pub(crate) fn guard_read(session: &str, input: &Value, clock: &dyn Clock) -> Option<String> {
    let path = input.get("file_path")?.as_str()?;

    // A deliberate ranged read is ALWAYS legal — that is the behaviour the guard steers toward,
    // and re-reading a different span of a file already seen is legitimate work.
    let ranged = input.get("offset").is_some() || input.get("limit").is_some();
    if ranged {
        record_read(session, path, clock);
        return None;
    }

    // Same tool call, hook registered twice (user level + project level): allow and do not
    // re-record. Only a read from an EARLIER turn counts as a re-read.
    let seen_ms_ago = last_read_ms_ago(session, path, clock);
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

    if let Some(n) = line_count(Path::new(path))
        && n > BIG_FILE_LINES
    {
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

    record_read(session, path, clock);
    None
}

#[cfg(test)]
#[path = "tests/read_guard_tests.rs"]
mod tests;
