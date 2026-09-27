//! Source guard over the `view!` markup of the wiki, vehicle, personnel and audit-log pages.
//!
//! The `view!` macro ends an element's open tag at the first `>` outside a delimiter group, so an
//! unbraced attribute value such as `disabled=page >= page_count` stops at `page` and the rest of
//! the expression renders as the element's text. The guard reads every production source of the
//! scanned folders from disk and refuses, inside `view!`:
//! - an open tag whose closing `>` sits inside an operator (`>=`, `->`, `=>`) or is followed by
//!   something no child starts with (a child is a string literal, a braced block or an element);
//! - an unbraced attribute value that is not a literal, a path or a closure, and an unbraced
//!   closure with an ordering comparison at its top level.
//!
//! Sources are read as tokens with comments and literal contents masked, so whitespace, line
//! breaks and text inside strings never change a verdict.

use std::path::{Path, PathBuf};

/// The scanned sources of the wiki page, relative to the crate root.
const WIKI_SOURCES: [&str; 1] = ["src/v2/pages/doctrine_and_info/wiki"];
/// The scanned sources of the vehicle index.
const VEHICLES_SOURCES: [&str; 1] = ["src/v2/pages/doctrine_and_info/vehicles"];
/// The scanned sources of the personnel roster.
const PERSONNEL_SOURCES: [&str; 1] = ["src/v2/pages/administration/personnel"];
/// The scanned sources of the audit-log page and the audit stream it subscribes to.
const AUDIT_LOG_SOURCES: [&str; 3] = [
    "src/v2/pages/administration/audit_logs",
    "src/v2/core/api/audit_stream.rs",
    "src/v2/core/api/audit_stream",
];

/// Operators of more than one character; every other punctuation byte is its own token.
const COMPOUND_PUNCTUATION: [&str; 10] =
    ["::", "==", "!=", "<=", ">=", "&&", "||", "->", "=>", ".."];

/// One token of an open tag's attribute region.
#[derive(Debug, PartialEq)]
enum Token {
    /// A whole delimiter group, named by its opening byte.
    Group(u8),
    /// A string literal.
    Literal,
    /// An identifier, keyword or number.
    Word(String),
    /// An operator or other punctuation.
    Punct(String),
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// Replaces `text[from..to]` with `fill`, keeping line breaks.
fn overwrite(text: &mut [u8], from: usize, to: usize, fill: u8) {
    let to = to.min(text.len());
    for byte in &mut text[from..to] {
        if *byte != b'\n' {
            *byte = fill;
        }
    }
}

/// The offset just past the string literal whose opening quote is at `quote`.
fn string_end(src: &[u8], quote: usize) -> usize {
    let mut at = quote + 1;
    while at < src.len() && src[at] != b'"' {
        at += if src[at] == b'\\' { 2 } else { 1 };
    }
    at + 1
}

/// The offset just past the block comment that opens at `start`, honouring nesting.
fn block_comment_end(src: &[u8], start: usize) -> usize {
    let (mut depth, mut at) = (0usize, start);
    while at < src.len() {
        if src[at..].starts_with(b"/*") {
            depth += 1;
            at += 2;
        } else if src[at..].starts_with(b"*/") {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return at;
            }
        } else {
            at += 1;
        }
    }
    src.len()
}

/// The opening quote and the hash count of the raw string starting at `at`, if one does.
fn raw_string_open(src: &[u8], at: usize) -> Option<(usize, usize)> {
    let r = if src[at] == b'b' { at + 1 } else { at };
    if src.get(r) != Some(&b'r') {
        return None;
    }
    let hashes = src[r + 1..]
        .iter()
        .take_while(|&&byte| byte == b'#')
        .count();
    (src.get(r + 1 + hashes) == Some(&b'"')).then_some((r + 1 + hashes, hashes))
}

/// `source` with comments blanked and every literal's contents replaced by `s`, byte for byte, so
/// the brackets and `>` that remain are code.
fn masked(source: &str) -> Vec<u8> {
    let src = source.as_bytes();
    let mut out = src.to_vec();
    let mut at = 0;
    while at < src.len() {
        let after_ident = at > 0 && is_ident_byte(src[at - 1]);
        if src[at..].starts_with(b"//") {
            let end = src[at..]
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(src.len(), |offset| at + offset);
            overwrite(&mut out, at, end, b' ');
            at = end;
        } else if src[at..].starts_with(b"/*") {
            let end = block_comment_end(src, at);
            overwrite(&mut out, at, end, b' ');
            at = end;
        } else if let Some((quote, hashes)) = raw_string_open(src, at).filter(|_| !after_ident) {
            let closing: Vec<u8> = std::iter::once(b'"')
                .chain(std::iter::repeat_n(b'#', hashes))
                .collect();
            let close = src[quote + 1..]
                .windows(closing.len())
                .position(|window| window == closing.as_slice())
                .map_or(src.len() - 1, |offset| quote + 1 + offset);
            overwrite(&mut out, at, quote, b' ');
            overwrite(&mut out, quote + 1, close, b's');
            overwrite(&mut out, close + 1, close + 1 + hashes, b' ');
            at = close + 1 + hashes;
        } else if src[at] == b'b' && !after_ident && src.get(at + 1) == Some(&b'"') {
            out[at] = b' ';
            at += 1;
        } else if src[at] == b'"' {
            let end = string_end(src, at);
            overwrite(&mut out, at + 1, end - 1, b's');
            at = end;
        } else if src[at] == b'\'' && src.get(at + 1) == Some(&b'\\') {
            let end = src
                .get(at + 3..)
                .and_then(|rest| rest.iter().position(|&byte| byte == b'\''))
                .map_or(src.len(), |offset| at + 3 + offset + 1);
            overwrite(&mut out, at + 1, end - 1, b's');
            at = end;
        } else if src[at] == b'\'' {
            let width = source[at + 1..].chars().next().map_or(1, char::len_utf8);
            if src.get(at + 1 + width) == Some(&b'\'') {
                overwrite(&mut out, at + 1, at + 1 + width, b's');
                at += width + 2;
            } else {
                at += 1;
            }
        } else {
            at += 1;
        }
    }
    out
}

/// The offset just past the delimiter group that opens at `start` in masked text.
fn group_end(text: &[u8], start: usize) -> usize {
    let mut depth = 0usize;
    for (offset, &byte) in text[start..].iter().enumerate() {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return start + offset + 1;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// The offset just past the masked string literal whose opening quote is at `quote`.
fn masked_string_end(text: &[u8], quote: usize) -> usize {
    text[quote + 1..]
        .iter()
        .position(|&byte| byte == b'"')
        .map_or(text.len(), |offset| quote + offset + 2)
}

/// The tokens of an open tag's attribute region, in masked text.
fn tokens(region: &[u8]) -> Vec<Token> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < region.len() {
        let byte = region[at];
        if byte.is_ascii_whitespace() {
            at += 1;
        } else if matches!(byte, b'(' | b'[' | b'{') {
            out.push(Token::Group(byte));
            at = group_end(region, at);
        } else if byte == b'"' {
            out.push(Token::Literal);
            at = masked_string_end(region, at);
        } else if is_ident_byte(byte) {
            let start = at;
            while at < region.len() && is_ident_byte(region[at]) {
                at += 1;
            }
            out.push(Token::Word(
                String::from_utf8_lossy(&region[start..at]).into_owned(),
            ));
        } else {
            let width = COMPOUND_PUNCTUATION
                .iter()
                .find(|operator| region[at..].starts_with(operator.as_bytes()))
                .map_or(1, |operator| operator.len());
            out.push(Token::Punct(
                String::from_utf8_lossy(&region[at..at + width]).into_owned(),
            ));
            at += width;
        }
    }
    out
}

fn is_punct(token: Option<&Token>, expected: &str) -> bool {
    matches!(token, Some(Token::Punct(found)) if found == expected)
}

fn is_word(token: Option<&Token>) -> bool {
    matches!(token, Some(Token::Word(_)))
}

/// The token count of the attribute key starting at `at` through its `=`, or 0 when no key
/// (`class`, `on:click`, `aria-current`, `class:active`) starts there.
fn key_length(tokens: &[Token], at: usize) -> usize {
    let continues_a_path = at > 0
        && [".", "::", ":", "-"]
            .iter()
            .any(|joint| is_punct(tokens.get(at - 1), joint));
    if !is_word(tokens.get(at)) || continues_a_path {
        return 0;
    }
    let mut next = at + 1;
    while (is_punct(tokens.get(next), ":") || is_punct(tokens.get(next), "-"))
        && is_word(tokens.get(next + 1))
    {
        next += 2;
    }
    if is_punct(tokens.get(next), "=") {
        next - at + 1
    } else {
        0
    }
}

/// Why an unbraced attribute value is refused, if it is: only a literal, a path (`NAME`,
/// `state.open`, `Type::CONST`) or a closure may go unbraced, and a closure only without an
/// ordering comparison at its top level.
fn refused_value(value: &[Token]) -> Option<&'static str> {
    let is_closure = matches!(value.first(), Some(Token::Word(word)) if word == "move")
        || is_punct(value.first(), "|")
        || is_punct(value.first(), "||");
    let is_path = value.iter().step_by(2).all(|token| is_word(Some(token)))
        && value
            .iter()
            .skip(1)
            .step_by(2)
            .all(|token| is_punct(Some(token), ".") || is_punct(Some(token), "::"))
        && is_word(value.last());
    match value.first() {
        None | Some(Token::Literal | Token::Group(b'{')) => None,
        Some(_) if is_closure => value
            .iter()
            .any(|token| {
                ["<", "<=", ">", ">="]
                    .iter()
                    .any(|op| is_punct(Some(token), op))
            })
            .then_some("an unbraced closure compares with `<` or `>` at its top level"),
        Some(_) if is_path => None,
        Some(_) => Some("an unbraced expression; wrap it in braces"),
    }
}

/// Scans the open tag whose `<` is at `open` and returns the offset just past its `>`.
fn scan_open_tag(text: &[u8], open: usize, end: usize, findings: &mut Vec<String>) -> usize {
    let name_end = open
        + 1
        + text[open + 1..end]
            .iter()
            .take_while(|&&byte| is_ident_byte(byte) || byte == b':' || byte == b'-')
            .count();
    let mut close = name_end;
    while close < end && text[close] != b'>' {
        close = match text[close] {
            b'(' | b'[' | b'{' => group_end(text, close),
            b'"' => masked_string_end(text, close),
            _ => close + 1,
        };
    }
    if close >= end {
        return end;
    }
    let name = String::from_utf8_lossy(&text[open + 1..name_end]).into_owned();
    let line = text[..open].iter().filter(|&&byte| byte == b'\n').count() + 1;
    let self_closing = text[close - 1] == b'/';
    let first_child = text[close + 1..end]
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .copied();
    if matches!(text[close - 1], b'-' | b'=') || text.get(close + 1) == Some(&b'=') {
        findings.push(format!(
            "line {line}: <{name}> closes inside `->`, `=>` or `>=`"
        ));
    } else if !self_closing && !matches!(first_child, None | Some(b'{' | b'"' | b'<' | b'}')) {
        findings.push(format!(
            "line {line}: <{name}> closes in the middle of an expression"
        ));
    }
    let region_end = if self_closing { close - 1 } else { close };
    let tokens = tokens(&text[name_end..region_end]);
    let mut at = 0;
    while at < tokens.len() {
        let key = key_length(&tokens, at);
        if key == 0 {
            at += 1;
            continue;
        }
        let value_end = (at + key..tokens.len())
            .find(|&next| key_length(&tokens, next) > 0)
            .unwrap_or(tokens.len());
        if let Some(reason) = refused_value(&tokens[at + key..value_end]) {
            let attribute: String = tokens[at..at + key - 1]
                .iter()
                .map(|token| match token {
                    Token::Word(part) | Token::Punct(part) => part.as_str(),
                    Token::Group(_) | Token::Literal => "",
                })
                .collect();
            findings.push(format!("line {line}: <{name} {attribute}=…>: {reason}"));
        }
        at = value_end;
    }
    close + 1
}

/// Every refusal in the `view!` invocations of `source`.
fn view_attribute_findings(source: &str) -> Vec<String> {
    let text = masked(source);
    let mut findings = Vec::new();
    let mut search = 0;
    while let Some(offset) = text[search..]
        .windows(5)
        .position(|window| window == b"view!")
    {
        let bang = search + offset;
        search = bang + 5;
        if bang > 0 && is_ident_byte(text[bang - 1]) {
            continue;
        }
        let Some(open) = (search..text.len()).find(|&at| !text[at].is_ascii_whitespace()) else {
            break;
        };
        if !matches!(text[open], b'(' | b'[' | b'{') {
            continue;
        }
        let end = group_end(&text, open) - 1;
        let mut at = open + 1;
        while at < end {
            at = match text[at] {
                b'(' | b'[' | b'{' => group_end(&text, at),
                b'"' => masked_string_end(&text, at),
                b'<' if text[at + 1].is_ascii_alphabetic() || text[at + 1] == b'_' => {
                    scan_open_tag(&text, at, end, &mut findings)
                }
                _ => at + 1,
            };
        }
    }
    findings
}

/// Every production `.rs` file under `path` (a folder or one file), outside `tests/` folders.
fn collect_production_sources(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if path.file_name().is_some_and(|name| name == "tests") {
            return;
        }
        for entry in std::fs::read_dir(path).expect("the scanned folder is readable") {
            collect_production_sources(&entry.expect("a readable entry").path(), out);
        }
    } else if path.extension().is_some_and(|extension| extension == "rs")
        && !path.to_string_lossy().ends_with("_tests.rs")
    {
        out.push(path.to_path_buf());
    }
}

/// Fails with every refusal found in the production sources under `scanned`.
fn assert_view_attributes_are_well_formed(scanned: &[&str]) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut failures = Vec::new();
    for relative in scanned {
        let mut files = Vec::new();
        collect_production_sources(&root.join(relative), &mut files);
        assert!(!files.is_empty(), "{relative} holds no production source");
        for file in files {
            let source = std::fs::read_to_string(&file).expect("the source is readable");
            let shown = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .display()
                .to_string();
            failures.extend(
                view_attribute_findings(&source)
                    .into_iter()
                    .map(|finding| format!("{shown} {finding}")),
            );
        }
    }
    assert!(
        failures.is_empty(),
        "view! attributes that end their tag early or go unbraced:\n{}",
        failures.join("\n")
    );
}

#[test]
fn view_attributes_in_the_wiki_page_are_braced_where_they_must_be() {
    assert_view_attributes_are_well_formed(&WIKI_SOURCES);
}

#[test]
fn view_attributes_in_the_vehicles_page_are_braced_where_they_must_be() {
    assert_view_attributes_are_well_formed(&VEHICLES_SOURCES);
}

#[test]
fn view_attributes_in_the_personnel_page_are_braced_where_they_must_be() {
    assert_view_attributes_are_well_formed(&PERSONNEL_SOURCES);
}

#[test]
fn view_attributes_in_the_audit_log_page_are_braced_where_they_must_be() {
    assert_view_attributes_are_well_formed(&AUDIT_LOG_SOURCES);
}

/// The pager that leaked `= page_count on:click=…` as the "Older" button's text, laid out two
/// ways: the verdict does not depend on line breaks.
#[test]
fn view_attributes_guard_refuses_a_comparison_that_closes_the_tag() {
    let stacked = "fn pager() -> impl IntoView {
        view! {
            <button
                type=\"button\"
                disabled=page >= page_count
                on:click=move |_| state.page.set((page + 1).min(page_count))
            >
                \"Older\"
            </button>
        }
    }";
    let inline =
        "view! { <button disabled=page >= page_count on:click=move |_| next()>\"Older\"</button> }";
    for sample in [stacked, inline] {
        let findings = view_attribute_findings(sample);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("<button> closes inside"),
            "{findings:?}"
        );
    }
    let greater = view_attribute_findings("view! { <input prop:disabled=move || count > 3 /> }");
    assert!(
        greater
            .iter()
            .any(|finding| finding.contains("in the middle of an expression")),
        "{greater:?}"
    );
}

#[test]
fn view_attributes_guard_refuses_unbraced_comparisons_and_calls() {
    for (sample, attribute) in [
        (
            "view! { <button disabled=page <= 1>\"Newer\"</button> }",
            "disabled",
        ),
        ("view! { <b hidden=count == 0 /> }", "hidden"),
        ("view! { <b hidden=ready && open /> }", "hidden"),
        ("view! { <input value=id.to_string() /> }", "value"),
        ("view! { <span class=badge(level)>\"x\"</span> }", "class"),
        (
            "view! { <Row title=view! { {name} }.into_any() /> }",
            "title",
        ),
        ("view! { <b hidden=move || count < 3 /> }", "hidden"),
    ] {
        let findings = view_attribute_findings(sample);
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains(&format!(" {attribute}=…>"))),
            "{sample}: {findings:?}"
        );
    }
}

#[test]
fn view_attributes_guard_accepts_braced_values_literals_paths_and_closures() {
    let sample = r##"
        // A comment may read disabled=page >= page_count without tripping the guard.
        fn pager() -> impl IntoView {
            let note = "a >= b -> c";
            view! {
                <nav class=PAGER_CLASS aria-label="a > b" data-note=r#"x >= y"#>
                    <button
                        disabled={page >= page_count}
                        on:click=move |_| state.page.set((page + 1).min(page_count))
                    >
                        "Older >"
                    </button>
                    <Select value={Signal::derive(move || query.get().to_string())} open=state.open />
                    <span class:active=move || selected.get() == id>{format!("{page} > {count}")}</span>
                    <Dialog open=open title=Titles::DELETE>{if c == '>' { "gt" } else { "" }}</Dialog>
                    <MaterialIcon name="add" class="text-sm" />
                </nav>
            }
        }
    "##;
    assert_eq!(view_attribute_findings(sample), Vec::<String>::new());
}
