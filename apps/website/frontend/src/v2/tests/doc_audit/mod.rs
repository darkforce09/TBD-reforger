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

/// Every audit finding for the file at `rel`, whose source is `text`.
///
/// `exempt` is true when a live grandfather row covers the file. It suppresses the size rule,
/// the inline-test-module rule and the ticket/wave rule. The `//!` header rule and the
/// documented-item rule report regardless, because no row may ever hide undocumented code.
fn findings(rel: &str, text: &str, exempt: bool) -> Vec<String> {
    let mut bad: Vec<String> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();

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
    for (i, line) in lines.iter().enumerate() {
        if needs_doc(line) {
            let mut k = i;
            while k > 0 && is_attribute(lines[k - 1]) {
                k -= 1;
            }
            if k == 0 || !is_doc(lines[k - 1]) {
                bad.push(format!("{rel}:{}: item is undocumented", i + 1));
            }
        }
        if !exempt && line.trim_start().starts_with("#[cfg(test)]") {
            if let Some(next) = lines[i + 1..].iter().find(|l| !l.trim().is_empty()) {
                if is_inline_mod(next) {
                    bad.push(format!("{rel}:{}: inline test module", i + 1));
                }
            }
        }
        if !exempt && names_ticket_or_wave(line) {
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
