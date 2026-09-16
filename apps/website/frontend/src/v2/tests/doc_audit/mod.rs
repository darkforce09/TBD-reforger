//! Structural and documentation audit of every production file under `src/v2`.
//!
//! **Role:** walks the v2 tree and enforces five rules on each production file — it opens with
//! a `//!` header, it is at most [`MAX_LINES`] lines, every visible item carries a doc comment,
//! it holds no inline test module, and no comment names a ticket or a wave. Also owns the dated
//! grandfather allowlist that carries a named file past three of those rules until a named date.
//! **Position:** compiled only under `cfg(test)`, declared from `v2/mod.rs`. It lives inside the
//! `tests` subtree the walk skips, so it never audits itself.
//! **Signals & state:** none. Every rule is a pure function over one file's text.
//! **Invariants:** the header rule and the documentation rule are never exemptible — a row
//! suppresses the size, inline-test-module and ticket/wave rules and nothing else. Every row
//! carries a date on which it stops exempting, and every row must name a file the walk saw.

mod allowlist;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Longest a v2 production file is allowed to be.
const MAX_LINES: usize = 500;

/// One dated exemption from the audit's three structural rules.
struct GrandfatherRow {
    /// Path of the exempt file, relative to `src/v2`, with `/` separators.
    path: &'static str,
    /// Why the file is still carried, and what clears it. An empty reason exempts nothing.
    reason: &'static str,
    /// UTC `YYYY-MM-DD` of the last day the row is in force.
    expires: &'static str,
}

/// Every `.rs` file under `dir` that is not inside a `tests/` subtree, sorted by path.
fn production_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

/// Depth-first collector behind [`production_files`].
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "tests") {
                continue;
            }
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// True when `line` is an attribute line, which may sit between a doc block and its item.
fn is_attribute(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("#[") || t.starts_with("#!")
}

/// True when `line` documents the item that follows it.
fn is_doc(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("///") || t.starts_with("#[doc")
}

/// True when `line` declares an item that the documentation rule covers: a visible
/// `fn`/`struct`/`enum`/`type`/`const`/`static`/`trait`, or a Leptos component.
fn needs_doc(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("#[component]") {
        return true;
    }
    let Some(rest) = strip_visibility(t) else {
        return false;
    };
    for kw in ["fn", "struct", "enum", "type", "const", "static", "trait"] {
        if let Some(tail) = rest.strip_prefix(kw) {
            if tail.starts_with(|c: char| !c.is_alphanumeric() && c != '_') {
                return true;
            }
        }
    }
    false
}

/// The text after a leading `pub`, `pub(crate)`, `pub(super)` … and its whitespace, if present.
fn strip_visibility(t: &str) -> Option<&str> {
    let rest = t.strip_prefix("pub")?;
    let rest = match rest.strip_prefix('(') {
        Some(inner) => {
            let close = inner.find(')')?;
            if !inner[..close].chars().all(|c| c.is_ascii_lowercase()) {
                return None;
            }
            &inner[close + 1..]
        }
        None => rest,
    };
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim_start())
}

/// True when `line` declares an inline module body (`mod foo {`), with any visibility.
fn is_inline_mod(line: &str) -> bool {
    let t = line.trim_start();
    let t = strip_visibility(t).unwrap_or(t);
    let Some(rest) = t.strip_prefix("mod ") else {
        return false;
    };
    let rest = rest.trim_start();
    let name_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    !rest[..name_end].is_empty() && rest[name_end..].trim_start().starts_with('{')
}

/// True when a comment line names a ticket or a wave — the two kinds of internal tracking
/// reference that must not appear in v2 documentation.
fn names_ticket_or_wave(line: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with("//") {
        return false;
    }
    let chars: Vec<char> = t.to_ascii_lowercase().chars().collect();
    // The ticket spelling allows dots between digit runs; the wave spellings take a bare run of
    // digits, optionally separated from the keyword by one space or dash.
    for (kw, dotted) in [("t-", true), ("wave", false), ("w", false)] {
        let k: Vec<char> = kw.chars().collect();
        for start in 0..chars.len() {
            if start + k.len() > chars.len() || chars[start..start + k.len()] != k[..] {
                continue;
            }
            if start > 0 && is_word(chars[start - 1]) {
                continue;
            }
            let mut j = start + k.len();
            if !dotted && chars.get(j).is_some_and(|c| *c == ' ' || *c == '-') {
                j += 1;
            }
            if !chars.get(j).is_some_and(char::is_ascii_digit) {
                continue;
            }
            while chars
                .get(j)
                .is_some_and(|c| c.is_ascii_digit() || (dotted && *c == '.'))
            {
                j += 1;
            }
            return true;
        }
    }
    false
}

/// True when `c` can appear inside an identifier.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// What a masking pass replaces with spaces.
#[derive(Clone, Copy)]
enum Blanked {
    /// Comments and literals both. The item, attribute and inline-module rules read this view, so
    /// that Rust source quoted inside a fixture string, or shown inside a block comment, is never
    /// mistaken for a declaration.
    CommentsAndLiterals,
    /// Literals only, with comments left as written. The doc-comment and ticket/wave rules read
    /// this view, because both ask a question about comment text and neither may be answered by
    /// text that merely sits inside a string.
    LiteralsOnly,
}

/// A same-length copy of `text` with the spans `blanked` names replaced by spaces.
///
/// Newlines survive the blanking, so the copy holds the same lines, in the same order, at the same
/// numbers as the original: a rule is applied to a line of the copy and reported against the line
/// of the file on disk. Block comments nest, as rustc allows.
fn masked(text: &str, blanked: Blanked) -> String {
    /// A space for every character but a newline, which is kept so line numbers survive.
    fn blank(c: char) -> char {
        if c == '\n' {
            c
        } else {
            ' '
        }
    }
    let keep_comments = matches!(blanked, Blanked::LiteralsOnly);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < chars.len() {
        // `// …`, to the end of the line.
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(if keep_comments {
                    chars[i]
                } else {
                    blank(chars[i])
                });
                i += 1;
            }
            continue;
        }
        // `/* … */`, counting nested pairs.
        if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            let mut depth = 0usize;
            while i < chars.len() {
                let opens = chars[i] == '/' && chars.get(i + 1) == Some(&'*');
                let closes = chars[i] == '*' && chars.get(i + 1) == Some(&'/');
                if opens {
                    depth += 1;
                } else if closes {
                    depth -= 1;
                }
                let width = if opens || closes { 2 } else { 1 };
                for c in &chars[i..(i + width).min(chars.len())] {
                    out.push(if keep_comments { *c } else { blank(*c) });
                }
                i += width;
                if closes && depth == 0 {
                    break;
                }
            }
            continue;
        }
        if let Some(end) = literal_span(&chars, i) {
            for c in &chars[i..end] {
                out.push(blank(*c));
            }
            i = end;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    assert_eq!(
        out.chars().count(),
        chars.len(),
        "the audit's mask lost alignment with the source, so no finding it produces can be \
         trusted to name the right line"
    );
    out
}

/// End index (exclusive) of the string or character literal that starts at `i`, if one starts
/// there.
///
/// Raw literals of any hash count, `\` escapes and literals spanning several lines are all
/// handled. A lifetime (`'a`) is deliberately not a literal: it has no closing quote, and blanking
/// from one would swallow the code after it. An unterminated literal runs to the end of the text.
fn literal_span(chars: &[char], i: usize) -> Option<usize> {
    // `r"…"` / `r#"…"#` / `r##"…"##`
    if chars[i] == 'r' && (i == 0 || !is_word(chars[i - 1])) {
        let mut j = i + 1;
        let mut hashes = 0usize;
        while chars.get(j) == Some(&'#') {
            hashes += 1;
            j += 1;
        }
        if chars.get(j) == Some(&'"') {
            let mut k = j + 1;
            while k < chars.len() {
                if chars[k] == '"' && (1..=hashes).all(|h| chars.get(k + h) == Some(&'#')) {
                    return Some((k + hashes + 1).min(chars.len()));
                }
                k += 1;
            }
            return Some(chars.len());
        }
    }
    if chars[i] == '"' {
        return Some(past_closing_quote(chars, i + 1, '"'));
    }
    // `'c'` and `'\n'`, but not the lifetime `'a`, which carries no closing quote.
    if chars[i] == '\'' && (chars.get(i + 1) == Some(&'\\') || chars.get(i + 2) == Some(&'\'')) {
        return Some(past_closing_quote(chars, i + 1, '\''));
    }
    None
}

/// Index just past the first unescaped `quote` at or after `from`, or the end of `chars`.
fn past_closing_quote(chars: &[char], from: usize, quote: char) -> usize {
    let mut k = from;
    while k < chars.len() {
        if chars[k] == '\\' {
            k += 2;
            continue;
        }
        if chars[k] == quote {
            return (k + 1).min(chars.len());
        }
        k += 1;
    }
    chars.len()
}

/// Every audit finding for the file at `rel`, whose source is `text`.
///
/// Each rule reads the view of the source that answers its question. The header and size rules
/// read the file as written; the item, attribute and inline-module rules read it with comments and
/// literals blanked, so that quoted Rust source is not audited as if it were code; the doc-comment
/// and ticket/wave rules read it with only literals blanked, so a real comment still answers them
/// and a quoted one never does.
///
/// `exempt` is true when a live grandfather row covers the file. It suppresses the size rule,
/// the inline-test-module rule and the ticket/wave rule. The `//!` header rule and the
/// documented-item rule report regardless, because no row may ever hide undocumented code.
fn findings(rel: &str, text: &str, exempt: bool) -> Vec<String> {
    let mut bad: Vec<String> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let code_text = masked(text, Blanked::CommentsAndLiterals);
    let unquoted_text = masked(text, Blanked::LiteralsOnly);
    let code: Vec<&str> = code_text.lines().collect();
    let unquoted: Vec<&str> = unquoted_text.lines().collect();
    assert_eq!(
        (code.len(), unquoted.len()),
        (lines.len(), lines.len()),
        "{rel}: the masked views hold a different number of lines from the source"
    );

    match lines.iter().find(|l| !l.trim().is_empty()) {
        Some(first) if first.trim_start().starts_with("//!") => {}
        _ => bad.push(format!("{rel}:1: file does not open with a `//!` header")),
    }
    if !exempt && lines.len() > MAX_LINES {
        bad.push(format!(
            "{rel}:{}: {} lines exceeds the {MAX_LINES}-line limit",
            lines.len(),
            lines.len()
        ));
    }
    for (i, line) in code.iter().enumerate() {
        if needs_doc(line) {
            let mut k = i;
            while k > 0 && is_attribute(code[k - 1]) {
                k -= 1;
            }
            if k == 0 || !is_doc(unquoted[k - 1]) {
                bad.push(format!("{rel}:{}: item is undocumented", i + 1));
            }
        }
        if !exempt && line.trim_start().starts_with("#[cfg(test)]") {
            if let Some(next) = code[i + 1..].iter().find(|l| !l.trim().is_empty()) {
                if is_inline_mod(next) {
                    bad.push(format!("{rel}:{}: inline test module", i + 1));
                }
            }
        }
        if !exempt && names_ticket_or_wave(unquoted[i]) {
            bad.push(format!("{rel}:{}: comment names a ticket or wave", i + 1));
        }
    }
    bad
}

/// True when `row` is well formed and still in force on `today`.
fn row_is_live(row: &GrandfatherRow, today: &str) -> bool {
    !row.reason.trim().is_empty() && expires_ok(row.expires, today)
}

/// True when some live row in `rows` covers `rel`.
fn is_exempt(rel: &str, rows: &[GrandfatherRow], today: &str) -> bool {
    rows.iter()
        .any(|row| row.path == rel && row_is_live(row, today))
}

/// True when `expires` is a real `YYYY-MM-DD` that `today` has not passed.
///
/// There is deliberately no never-expires spelling: every row names the day it dies, so a file
/// that Phase 3C has not reached starts failing the audit on its own schedule.
fn expires_ok(expires: &str, today: &str) -> bool {
    let b = expires.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                true
            } else {
                c.is_ascii_digit()
            }
        })
        && expires >= today
}

/// Findings that report a row naming a path none of the `audited` files carries.
///
/// A row must always name a file the walk saw, so that splitting, moving or deleting that file
/// forces the row to be updated or removed rather than quietly outliving its subject.
fn rows_without_a_file(rows: &[GrandfatherRow], audited: &[String]) -> Vec<String> {
    rows.iter()
        .filter(|row| !audited.iter().any(|seen| seen == row.path))
        .map(|row| {
            format!(
                "{}: allowlist row names a path that no v2 production file has",
                row.path
            )
        })
        .collect()
}

/// The findings for one file with the allowlist applied — the composition the walk runs per file.
fn audit_one(rel: &str, text: &str, rows: &[GrandfatherRow], today: &str) -> Vec<String> {
    findings(rel, text, is_exempt(rel, rows, today))
}

/// Today's UTC civil date as `YYYY-MM-DD`, read from the system clock.
fn today_ymd() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock reads at or after the unix epoch")
        .as_secs()
        / 86_400;
    civil_ymd(days)
}

/// UTC `YYYY-MM-DD` from a count of days since the unix epoch (Hinnant's `civil_from_days`).
fn civil_ymd(unix_days: u64) -> String {
    let z = unix_days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[test]
fn v2_production_files_meet_the_documentation_standard() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/v2");
    let today = today_ymd();
    let mut audited: Vec<String> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    for path in production_files(&root) {
        let text = fs::read_to_string(&path).expect("read v2 source");
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        bad.extend(audit_one(&rel, &text, allowlist::ROWS, &today));
        audited.push(rel);
    }
    bad.extend(rows_without_a_file(allowlist::ROWS, &audited));
    assert!(
        bad.is_empty(),
        "v2 documentation audit failed:\n{}",
        bad.join("\n")
    );
}

/* ── the allowlist mechanism, pinned against fixture text rather than the live tree ── */

/// The fixed civil date the mechanism tests reason from, so no test depends on the wall clock.
const FIXTURE_TODAY: &str = "2026-09-17";

/// Path the fixture file is audited under.
const FIXTURE_PATH: &str = "apps/editor/oversized_workspace.rs";

/// A reason of the shape a real row carries.
const FIXTURE_REASON: &str = "Split into per-dock modules by the editor decomposition work.";

/// A row built from parts, for the mechanism tests.
fn row(path: &'static str, reason: &'static str, expires: &'static str) -> GrandfatherRow {
    GrandfatherRow {
        path,
        reason,
        expires,
    }
}

/// Fixture source that breaks exactly the three exemptible rules: it is over the line limit, it
/// holds an inline test module, and one comment names a ticket. It satisfies the header rule and
/// declares no item, so the two unexemptible rules stay silent.
fn text_breaking_only_the_exemptible_rules() -> String {
    let mut text = String::from("//! Fixture header.\n");
    text.push_str("// Behaviour pinned by t-123.\n");
    text.push_str("#[cfg(test)]\nmod tests {}\n");
    for _ in 0..MAX_LINES {
        text.push('\n');
    }
    text
}

/// Fixture source that breaks exactly the two unexemptible rules: no header, one bare `pub` item.
fn text_breaking_only_the_unexemptible_rules() -> &'static str {
    "pub fn mount_workspace() {}\n"
}

#[test]
fn the_fixture_breaks_the_three_exemptible_rules_when_no_row_covers_it() {
    let text = text_breaking_only_the_exemptible_rules();
    let bad = audit_one(FIXTURE_PATH, &text, &[], FIXTURE_TODAY);
    assert_eq!(bad.len(), 3, "{bad:#?}");
    assert!(bad.iter().any(|b| b.contains("exceeds")), "{bad:#?}");
    assert!(
        bad.iter().any(|b| b.contains("inline test module")),
        "{bad:#?}"
    );
    assert!(
        bad.iter().any(|b| b.contains("names a ticket or wave")),
        "{bad:#?}"
    );
}

#[test]
fn a_row_that_has_not_expired_exempts_the_three_structural_rules() {
    let text = text_breaking_only_the_exemptible_rules();
    let rows = [row(FIXTURE_PATH, FIXTURE_REASON, "2999-01-01")];
    let bad = audit_one(FIXTURE_PATH, &text, &rows, FIXTURE_TODAY);
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn an_expired_row_stops_exempting() {
    let text = text_breaking_only_the_exemptible_rules();
    let rows = [row(FIXTURE_PATH, FIXTURE_REASON, "2020-01-01")];
    assert!(!is_exempt(FIXTURE_PATH, &rows, FIXTURE_TODAY));
    let bad = audit_one(FIXTURE_PATH, &text, &rows, FIXTURE_TODAY);
    assert_eq!(bad.len(), 3, "{bad:#?}");
}

#[test]
fn a_row_whose_reason_is_empty_exempts_nothing() {
    let text = text_breaking_only_the_exemptible_rules();
    for blank in ["", "   "] {
        let rows = [GrandfatherRow {
            path: FIXTURE_PATH,
            reason: blank,
            expires: "2999-01-01",
        }];
        assert!(!is_exempt(FIXTURE_PATH, &rows, FIXTURE_TODAY));
        let bad = audit_one(FIXTURE_PATH, &text, &rows, FIXTURE_TODAY);
        assert_eq!(bad.len(), 3, "reason {blank:?} must not exempt: {bad:#?}");
    }
}

#[test]
fn a_row_naming_a_path_no_file_has_is_a_failure() {
    let rows = [row(FIXTURE_PATH, FIXTURE_REASON, "2999-01-01")];
    let present = vec![FIXTURE_PATH.to_string()];
    assert!(rows_without_a_file(&rows, &present).is_empty());

    let absent = vec!["apps/editor/somewhere_else.rs".to_string()];
    let bad = rows_without_a_file(&rows, &absent);
    assert_eq!(bad.len(), 1, "{bad:#?}");
    assert!(bad[0].contains(FIXTURE_PATH), "{bad:#?}");
}

#[test]
fn a_row_never_exempts_the_header_rule_or_the_documented_item_rule() {
    let text = text_breaking_only_the_unexemptible_rules();
    let rows = [row(FIXTURE_PATH, FIXTURE_REASON, "2999-01-01")];
    assert!(is_exempt(FIXTURE_PATH, &rows, FIXTURE_TODAY));
    let bad = audit_one(FIXTURE_PATH, text, &rows, FIXTURE_TODAY);
    assert_eq!(bad.len(), 2, "{bad:#?}");
    assert!(
        bad.iter().any(|b| b.contains("does not open with a `//!`")),
        "{bad:#?}"
    );
    assert!(
        bad.iter().any(|b| b.contains("item is undocumented")),
        "{bad:#?}"
    );
}

#[test]
fn no_spelling_of_expires_can_outlive_a_date() {
    for spelling in ["MC-perf", "never", "forever", "", "2026/12/31", "2026-12-3"] {
        assert!(
            !expires_ok(spelling, FIXTURE_TODAY),
            "{spelling:?} must not be accepted as an expiry"
        );
    }
    assert!(expires_ok("2026-12-31", FIXTURE_TODAY));
    assert!(
        expires_ok(FIXTURE_TODAY, FIXTURE_TODAY),
        "expiry day is live"
    );
    assert!(
        !expires_ok("2026-09-16", FIXTURE_TODAY),
        "a row whose last day has passed is dead"
    );
}

#[test]
fn every_shipped_row_is_well_formed_and_dated() {
    for row in allowlist::ROWS {
        assert!(
            !row.reason.trim().is_empty(),
            "{}: allowlist row carries no reason",
            row.path
        );
        assert!(
            expires_ok(row.expires, "1970-01-01"),
            "{}: `{}` is not a real YYYY-MM-DD expiry",
            row.path,
            row.expires
        );
    }
}

#[test]
fn the_civil_date_arithmetic_tracks_the_utc_calendar() {
    assert_eq!(civil_ymd(0), "1970-01-01");
    assert_eq!(civil_ymd(20_454), "2026-01-01");
    assert_eq!(civil_ymd(20_818), "2026-12-31");
}

#[test]
fn the_clock_reads_a_ten_character_civil_date() {
    let today = today_ymd();
    assert_eq!(today.len(), 10, "{today}");
    assert!(expires_ok(&today, &today), "{today}");
}

/* ── the rules read code, not the Rust source quoted inside literals and comments ── */

/// Fixture whose only `pub fn` declarations sit inside string literals, one of each shape this
/// tree writes: a plain literal, a raw literal with hashes, and a literal spanning lines.
fn text_whose_only_items_are_quoted() -> &'static str {
    r##"//! Fixture header.
const PLAIN: &str = "pub fn mount_plain() {}";
const RAW: &str = r#"pub fn mount_raw() {}"#;
const SPANNING: &str = "
pub fn mount_spanning() {}
";
"##
}

/// Fixture that quotes an inline test module inside a raw literal, the way a source-inspection
/// pin holds the shape it is testing for.
fn text_quoting_an_inline_test_module() -> &'static str {
    r##"//! Fixture header.
const FIXTURE: &str = r#"
#[cfg(test)]
mod tests {}
"#;
"##
}

#[test]
fn a_public_item_inside_a_literal_is_not_an_undocumented_item() {
    let bad = audit_one(
        FIXTURE_PATH,
        text_whose_only_items_are_quoted(),
        &[],
        FIXTURE_TODAY,
    );
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn a_public_item_in_real_code_is_still_an_undocumented_item() {
    let bad = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\npub fn mount_workspace() {}\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(bad.len(), 1, "{bad:#?}");
    assert!(bad[0].contains(":2: item is undocumented"), "{bad:#?}");
}

#[test]
fn an_inline_test_module_counts_in_code_and_never_inside_a_literal() {
    let quoted = audit_one(
        FIXTURE_PATH,
        text_quoting_an_inline_test_module(),
        &[],
        FIXTURE_TODAY,
    );
    assert!(quoted.is_empty(), "{quoted:#?}");

    let live = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\n#[cfg(test)]\nmod tests {}\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(live.len(), 1, "{live:#?}");
    assert!(live[0].contains(":2: inline test module"), "{live:#?}");
}

#[test]
fn a_ticket_name_counts_in_a_comment_and_never_inside_a_literal() {
    let commented = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\n// Behaviour pinned by t-123.\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(commented.len(), 1, "{commented:#?}");
    assert!(
        commented[0].contains(":2: comment names a ticket or wave"),
        "{commented:#?}"
    );

    let quoted = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\nconst NOTE: &str = \"// Behaviour pinned by t-123.\";\n",
        &[],
        FIXTURE_TODAY,
    );
    assert!(quoted.is_empty(), "{quoted:#?}");
}

#[test]
fn an_escaped_quote_does_not_desynchronise_the_mask() {
    let bad = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         const ESCAPED: &str = \"quoting \\\"pub fn ghost() {}\\\" mid-literal\";\n\
         pub fn mount_workspace() {}\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(bad.len(), 1, "{bad:#?}");
    assert!(bad[0].contains(":3: item is undocumented"), "{bad:#?}");
}

#[test]
fn a_block_comment_hides_its_contents_and_closes_where_it_ends() {
    let flat = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /* pub fn commented_out() {}\n\
         pub fn also_commented_out() {}\n\
         */\n\
         pub fn mount_workspace() {}\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(flat.len(), 1, "{flat:#?}");
    assert!(flat[0].contains(":5: item is undocumented"), "{flat:#?}");

    let nested = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /* outer /* inner */ still inside the outer comment\n\
         pub fn still_commented_out() {}\n\
         */\n\
         pub fn mount_workspace() {}\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(nested.len(), 1, "{nested:#?}");
    assert!(
        nested[0].contains(":5: item is undocumented"),
        "{nested:#?}"
    );
}

#[test]
fn a_character_literal_is_masked_and_a_lifetime_is_not() {
    let quote_char = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /// Documented.\n\
         pub fn opening_quote() -> char {\n\
         '\"'\n\
         }\n\
         pub fn mount_workspace() {}\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(quote_char.len(), 1, "{quote_char:#?}");
    assert!(
        quote_char[0].contains(":6: item is undocumented"),
        "{quote_char:#?}"
    );

    let lifetime = audit_one(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /// Documented.\n\
         pub struct Borrowed<'a>(&'a str);\n\
         pub fn mount_workspace() {}\n",
        &[],
        FIXTURE_TODAY,
    );
    assert_eq!(lifetime.len(), 1, "{lifetime:#?}");
    assert!(
        lifetime[0].contains(":4: item is undocumented"),
        "{lifetime:#?}"
    );
}
