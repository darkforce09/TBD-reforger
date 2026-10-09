//! The Bash rules of the tool-call guard.
//!
//! **Role:** [`guard_bash`] denies two unambiguous command shapes, each with a bounded built-in
//! tool as the replacement: a repository search whose output is not capped (the Grep tool) and a
//! bare file reader as the whole command (the Read tool). In the measured baseline of this
//! repository's agent waves, uncapped `grep`/`rg` output is 8.96% of all input-side tokens and
//! `head`/`sed`/`cat`/`tail` file extraction 6.25%.
//! **Position:** called by `crate::tool_call_guard` with the `command` of every `Bash` hook
//! payload.
//! **Signals & state:** none; pure functions over the command text.
//! **Invariants:** a deny needs a positive match; `git grep`, `git log --grep`, a search piped
//! into `head`, `tail` or `wc`, a search reading from a pipe and anything it cannot split
//! cleanly are allowed. The segment split is crude on purpose: it must never be clever enough to
//! produce a false deny.

/// A shell command split into top-level segments at `;`, `&&`, `||`, `|` and `&` outside quotes.
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

/// The first whitespace-separated word of a segment, or the empty string.
fn first_word(seg: &str) -> String {
    seg.split_whitespace().next().unwrap_or("").to_string()
}

/// The Bash guard. Returns a deny message, or None to allow.
///
/// Denies only two unambiguous shapes, both with a bounded built-in tool as the replacement:
///
///   * a search whose output is not capped        -> the Grep tool (`head_limit`, gitignore-aware)
///   * a bare file-reader as the WHOLE command    -> the Read tool (`offset`/`limit`)
///
/// Explicitly NOT denied, because each is either legitimate or already the wanted behaviour:
///   `git log --grep=`/`git grep` (git, not a repository scan) · anything piped into
///   `head`/`tail` (that IS the cap) · `grep` reading from a pipe (already bounded by its
///   producer) · `wc -l`.
pub(crate) fn guard_bash(cmd: &str) -> Option<String> {
    let segs = segments(cmd);
    for (i, seg) in segs.iter().enumerate() {
        let seg_trim = seg.trim();
        let w = first_word(seg_trim);
        let base = w.rsplit('/').next().unwrap_or(&w);
        let piped_from_previous = i > 0;

        // 1. Uncapped repository search.
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

#[cfg(test)]
#[path = "tests/bash_command_guard_tests.rs"]
mod tests;
