//! Structural and documentation audit of every production file under `src/v2`.
//!
//! **Role:** walks the v2 tree and enforces five rules on each production file — it opens with
//! a `//!` header, it is at most [`MAX_LINES`] lines, every visible item carries a doc comment,
//! it holds no inline test module, and no comment names a ticket or a wave.
//! **Position:** compiled only under `cfg(test)`, declared from `v2/mod.rs`. It lives inside the
//! `tests` subtree the walk skips, so it never audits itself.
//! **Signals & state:** none. Every rule is a pure function over one file's text.
//! **Invariants:** all five rules apply to every production file, with no exemption path: a file
//! that breaks a rule is fixed, never listed.

use std::fs;
use std::path::{Path, PathBuf};

/// Longest a v2 production file is allowed to be.
const MAX_LINES: usize = 500;

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
fn findings(rel: &str, text: &str) -> Vec<String> {
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
    if lines.len() > MAX_LINES {
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
        if line.trim_start().starts_with("#[cfg(test)]") {
            if let Some(next) = code[i + 1..].iter().find(|l| !l.trim().is_empty()) {
                if is_inline_mod(next) {
                    bad.push(format!("{rel}:{}: inline test module", i + 1));
                }
            }
        }
        if names_ticket_or_wave(unquoted[i]) {
            bad.push(format!("{rel}:{}: comment names a ticket or wave", i + 1));
        }
    }
    bad
}

#[test]
fn v2_production_files_meet_the_documentation_standard() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/v2");
    let mut bad: Vec<String> = Vec::new();
    for path in production_files(&root) {
        let text = fs::read_to_string(&path).expect("read v2 source");
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        bad.extend(findings(&rel, &text));
    }
    assert!(
        bad.is_empty(),
        "v2 documentation audit failed:\n{}",
        bad.join("\n")
    );
}

/* ── every rule reports on every file, pinned against fixture text rather than the live tree ── */

/// Path the fixture file is audited under.
const FIXTURE_PATH: &str = "apps/editor/oversized_workspace.rs";

/// Fixture source that breaks exactly the three structural rules: it is over the line limit, it
/// holds an inline test module, and one comment names a ticket. It satisfies the header rule and
/// declares no item, so the header and documented-item rules stay silent.
fn text_breaking_the_three_structural_rules() -> String {
    let mut text = String::from("//! Fixture header.\n");
    text.push_str("// Behaviour pinned by t-123.\n");
    text.push_str("#[cfg(test)]\nmod tests {}\n");
    for _ in 0..MAX_LINES {
        text.push('\n');
    }
    text
}

/// Fixture source that breaks exactly the header and documented-item rules: no header, one bare
/// `pub` item.
fn text_breaking_the_header_and_documented_item_rules() -> &'static str {
    "pub fn mount_workspace() {}\n"
}

#[test]
fn the_fixture_breaks_the_three_structural_rules() {
    let text = text_breaking_the_three_structural_rules();
    let bad = findings(FIXTURE_PATH, &text);
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
fn a_file_at_the_line_limit_passes_and_one_line_over_fails() {
    let at_limit = format!("//! Fixture header.\n{}", "\n".repeat(MAX_LINES - 1));
    assert!(findings(FIXTURE_PATH, &at_limit).is_empty());
    let over_limit = format!("//! Fixture header.\n{}", "\n".repeat(MAX_LINES));
    let bad = findings(FIXTURE_PATH, &over_limit);
    assert_eq!(bad.len(), 1, "{bad:#?}");
    assert!(bad[0].contains("exceeds the 500-line limit"), "{bad:#?}");
}

#[test]
fn the_header_rule_and_the_documented_item_rule_report_a_bare_item() {
    let bad = findings(
        FIXTURE_PATH,
        text_breaking_the_header_and_documented_item_rules(),
    );
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
    let bad = findings(FIXTURE_PATH, text_whose_only_items_are_quoted());
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn a_public_item_in_real_code_is_still_an_undocumented_item() {
    let bad = findings(
        FIXTURE_PATH,
        "//! Fixture header.\npub fn mount_workspace() {}\n",
    );
    assert_eq!(bad.len(), 1, "{bad:#?}");
    assert!(bad[0].contains(":2: item is undocumented"), "{bad:#?}");
}

#[test]
fn an_inline_test_module_counts_in_code_and_never_inside_a_literal() {
    let quoted = findings(FIXTURE_PATH, text_quoting_an_inline_test_module());
    assert!(quoted.is_empty(), "{quoted:#?}");

    let live = findings(
        FIXTURE_PATH,
        "//! Fixture header.\n#[cfg(test)]\nmod tests {}\n",
    );
    assert_eq!(live.len(), 1, "{live:#?}");
    assert!(live[0].contains(":2: inline test module"), "{live:#?}");
}

#[test]
fn a_ticket_name_counts_in_a_comment_and_never_inside_a_literal() {
    let commented = findings(
        FIXTURE_PATH,
        "//! Fixture header.\n// Behaviour pinned by t-123.\n",
    );
    assert_eq!(commented.len(), 1, "{commented:#?}");
    assert!(
        commented[0].contains(":2: comment names a ticket or wave"),
        "{commented:#?}"
    );

    let quoted = findings(
        FIXTURE_PATH,
        "//! Fixture header.\nconst NOTE: &str = \"// Behaviour pinned by t-123.\";\n",
    );
    assert!(quoted.is_empty(), "{quoted:#?}");
}

#[test]
fn an_escaped_quote_does_not_desynchronise_the_mask() {
    let bad = findings(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         const ESCAPED: &str = \"quoting \\\"pub fn ghost() {}\\\" mid-literal\";\n\
         pub fn mount_workspace() {}\n",
    );
    assert_eq!(bad.len(), 1, "{bad:#?}");
    assert!(bad[0].contains(":3: item is undocumented"), "{bad:#?}");
}

#[test]
fn a_block_comment_hides_its_contents_and_closes_where_it_ends() {
    let flat = findings(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /* pub fn commented_out() {}\n\
         pub fn also_commented_out() {}\n\
         */\n\
         pub fn mount_workspace() {}\n",
    );
    assert_eq!(flat.len(), 1, "{flat:#?}");
    assert!(flat[0].contains(":5: item is undocumented"), "{flat:#?}");

    let nested = findings(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /* outer /* inner */ still inside the outer comment\n\
         pub fn still_commented_out() {}\n\
         */\n\
         pub fn mount_workspace() {}\n",
    );
    assert_eq!(nested.len(), 1, "{nested:#?}");
    assert!(
        nested[0].contains(":5: item is undocumented"),
        "{nested:#?}"
    );
}

#[test]
fn a_character_literal_is_masked_and_a_lifetime_is_not() {
    let quote_char = findings(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /// Documented.\n\
         pub fn opening_quote() -> char {\n\
         '\"'\n\
         }\n\
         pub fn mount_workspace() {}\n",
    );
    assert_eq!(quote_char.len(), 1, "{quote_char:#?}");
    assert!(
        quote_char[0].contains(":6: item is undocumented"),
        "{quote_char:#?}"
    );

    let lifetime = findings(
        FIXTURE_PATH,
        "//! Fixture header.\n\
         /// Documented.\n\
         pub struct Borrowed<'a>(&'a str);\n\
         pub fn mount_workspace() {}\n",
    );
    assert_eq!(lifetime.len(), 1, "{lifetime:#?}");
    assert!(
        lifetime[0].contains(":4: item is undocumented"),
        "{lifetime:#?}"
    );
}
