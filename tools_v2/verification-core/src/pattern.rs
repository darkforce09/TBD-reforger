//! Patterns — the typed form of `grep`'s `-E` / `-F` / `-i` triple.
//!
//! ── WHY `regex` AND NOT A `grep` SUBPROCESS ──────────────────────────────────────────────────
//!
//! `gate-grep.sh` had already removed one external search tool. Its header records the finding:
//! `ripgrep` is installed **nowhere** on this machine, and `command -v rg` succeeds in an agent
//! shell only because the harness injects a shell *function* named `rg`. So an `rg`-based gate
//! returned a different verdict depending on who invoked it, and the fix was to fall back to
//! `grep`, which is present in the container, on the host and on every CI runner.
//!
//! Using the `regex` crate finishes that job: the search engine is now compiled into the gate
//! binary, so there is no `PATH`, no shell function, no version skew, and **exit 127 stops being
//! a reachable state for the matcher at all**. That is a dependency removed, not asserted.
//!
//! ── THE COMPATIBILITY TRAP: `^` AND `$` ──────────────────────────────────────────────────────
//!
//! `grep` is a LINE matcher. `^foo` means "some line begins with foo". The `regex` crate defaults
//! to matching against the whole text, where `^` means "the input begins with foo" — so a pattern
//! ported verbatim would quietly stop matching on every line but the first, and a ban built on
//! `^\s*unsafe` would report OK over a file full of violations.
//!
//! That is the same class of defect this crate exists to prevent, arriving through the back door
//! of a syntax difference, so `multi_line(true)` is set unconditionally and is not configurable.
//! With it, `^`/`$` are line anchors and `.` still does not cross a newline — which together are
//! exactly `grep`'s semantics.
//!
//! The remaining ERE surface is compatible for everything the ported gates use: `\(`, `\[`, `|`
//! and the POSIX classes such as `[[:space:]]` mean the same thing in both engines. `-F` becomes
//! [`Pattern::literal`], which escapes the needle rather than trusting the caller to have escaped
//! it — the honest version of `grep -F`.

use regex::{Regex, RegexBuilder};

/// A compiled search pattern with `grep`-compatible semantics.
#[derive(Debug, Clone)]
pub struct Pattern {
    re: Regex,
    /// The pattern exactly as the caller wrote it, for failure messages. The compiled form is
    /// unhelpful in a log when the source was `-F` and got escaped.
    source: String,
}

impl Pattern {
    /// An extended-regex pattern — the default engine, equivalent to `grep -E`.
    pub fn regex(pat: &str) -> Result<Pattern, regex::Error> {
        Ok(Pattern {
            re: build(pat, false)?,
            source: pat.to_string(),
        })
    }

    /// A literal pattern — equivalent to `grep -F`. The needle is escaped, so regex metacharacters
    /// in it are matched as themselves.
    pub fn literal(pat: &str) -> Pattern {
        // `regex::escape` output is always a valid pattern, so this cannot fail.
        let re = build(&regex::escape(pat), false).expect("escaped literal is always valid");
        Pattern {
            re,
            source: pat.to_string(),
        }
    }

    /// Case-fold this pattern — equivalent to adding `grep -i`.
    pub fn case_insensitive(self) -> Result<Pattern, regex::Error> {
        Ok(Pattern {
            re: build(self.re.as_str(), true)?,
            source: self.source,
        })
    }

    /// Does this pattern match anywhere in `subject`?
    pub fn is_match(&self, subject: &str) -> bool {
        self.re.is_match(subject)
    }

    /// The pattern as the caller wrote it, for diagnostics.
    pub fn source(&self) -> &str {
        &self.source
    }
}

fn build(pat: &str, case_insensitive: bool) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pat)
        // See the module docs. Non-negotiable: without it, `^`/`$` silently change meaning.
        .multi_line(true)
        .case_insensitive(case_insensitive)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_is_a_line_anchor_like_grep() {
        // THE TRAP. Default regex-crate semantics would fail this, and every ported `^`-anchored
        // ban would go quietly green over a file full of violations.
        let p = Pattern::regex("^forbidden").unwrap();
        assert!(p.is_match("ok line\nforbidden line\n"));
    }

    #[test]
    fn dollar_is_a_line_anchor_like_grep() {
        let p = Pattern::regex("trailing$").unwrap();
        assert!(p.is_match("has trailing\nmore\n"));
    }

    #[test]
    fn dot_does_not_cross_a_newline() {
        // grep is per-line, so a `.` run can never span lines. Same here.
        let p = Pattern::regex("a.*b").unwrap();
        assert!(!p.is_match("a\nb"));
        assert!(p.is_match("a x b"));
    }

    #[test]
    fn literal_escapes_metacharacters() {
        let p = Pattern::literal("foo(bar)");
        assert!(p.is_match("call foo(bar) here"));
        assert!(!p.is_match("call fooXbar here"));
    }

    #[test]
    fn case_insensitive_folds() {
        let p = Pattern::regex("unsafe")
            .unwrap()
            .case_insensitive()
            .unwrap();
        assert!(p.is_match("UNSAFE block"));
        let sensitive = Pattern::regex("unsafe").unwrap();
        assert!(!sensitive.is_match("UNSAFE block"));
    }

    #[test]
    fn posix_classes_work_as_in_ere() {
        // gate-grep.sh's header pins `[[:space:]]` as identical across engines.
        let p = Pattern::regex("foo[[:space:]]+bar").unwrap();
        assert!(p.is_match("foo   bar"));
    }

    #[test]
    fn source_survives_escaping_for_diagnostics() {
        assert_eq!(Pattern::literal("a.b").source(), "a.b");
    }

    #[test]
    fn invalid_regex_is_an_error_not_a_panic() {
        assert!(Pattern::regex("a(").is_err());
    }
}
