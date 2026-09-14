//! Character-level reading of Rust source: word boundaries, delimiter matching, and the masked
//! copy every structural decision is taken on.
//!
//! **Role:** the lowest layer of the source scrubber. Nothing here knows what a `cfg` or a
//! constant is; it only turns text into positions that can be trusted.
//! **Position:** first stage of the scrub pipeline — every other shard reads masked text produced
//! here rather than raw source.
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** [`mask`] returns a buffer of exactly the same length as its input, so indices
//! taken on the mask address the original text. Newlines survive blanking, so line numbers do too.

/// True when `c` may appear inside a Rust identifier.
pub(crate) fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// True when `kw` sits at index `i` in `c` as a whole word — neither neighbour is an
/// identifier character.
pub(super) fn kw_at(c: &[char], i: usize, kw: &str) -> bool {
    let k: Vec<char> = kw.chars().collect();
    if i + k.len() > c.len() || c[i..i + k.len()] != k[..] {
        return false;
    }
    (i == 0 || !is_ident_char(c[i - 1]))
        && (i + k.len() >= c.len() || !is_ident_char(c[i + k.len()]))
}

/// A blanking replacement for `c`: a space, except for newlines, which are kept so that line
/// numbers in the blanked copy still match the original.
pub(super) fn blank(c: char) -> char {
    if c == '\n' {
        '\n'
    } else {
        ' '
    }
}

/// Index of the delimiter in `c` that closes the one at `at`, counting nested pairs.
///
/// Returns `None` when the opening delimiter is never closed.
pub(super) fn balanced(c: &[char], at: usize, open: char, close: char) -> Option<usize> {
    debug_assert_eq!(c[at], open);
    let mut depth = 0usize;
    for (i, ch) in c.iter().enumerate().skip(at) {
        if *ch == open {
            depth += 1;
        } else if *ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// A same-length copy of `chars` with comments blanked, and with string and character literals
/// blanked too when `blank_literals` is set.
///
/// Equal length is the point: every structural decision the scrubber takes is read off this copy,
/// so a `{` inside a string or a `fn foo(` inside a comment can never steer brace balancing, while
/// the indices still address the original text. Block comments nest, as rustc allows.
pub(super) fn mask(chars: &[char], blank_literals: bool) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0usize;
    while i < chars.len() {
        // `// …`
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(blank(chars[i]));
                i += 1;
            }
            continue;
        }
        // `/* … */`, nesting as rustc allows
        if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            let mut depth = 0usize;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                out.push(blank(chars[i]));
                i += 1;
            }
            continue;
        }
        // literal spans: `r#"…"#`, `"…"`, `'c'`
        let span = literal_span(chars, i);
        if let Some(end) = span {
            for k in i..end {
                out.push(if blank_literals {
                    blank(chars[k])
                } else {
                    chars[k]
                });
            }
            i = end;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    assert_eq!(
        out.len(),
        chars.len(),
        "scrubber mask lost alignment with the source — nothing built on it can be \
         trusted, so this is a hard failure rather than a silent skip"
    );
    out
}

/// End index (exclusive) of the string or character literal starting at `i`, if one starts
/// there. Raw strings of any hash count are handled. A lifetime (`'a`) is deliberately not a
/// literal.
fn literal_span(chars: &[char], i: usize) -> Option<usize> {
    // r"…" / r#"…"# / r##"…"##
    if chars[i] == 'r' && (i == 0 || !is_ident_char(chars[i - 1])) {
        let mut j = i + 1;
        let mut hashes = 0usize;
        while j < chars.len() && chars[j] == '#' {
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
        let mut k = i + 1;
        while k < chars.len() {
            if chars[k] == '\\' {
                k += 2;
                continue;
            }
            if chars[k] == '"' {
                return Some((k + 1).min(chars.len()));
            }
            k += 1;
        }
        return Some(chars.len());
    }
    if chars[i] == '\'' {
        let escaped = chars.get(i + 1) == Some(&'\\');
        let single = chars.get(i + 2) == Some(&'\'');
        if escaped || single {
            let mut k = i + 1;
            while k < chars.len() {
                if chars[k] == '\\' {
                    k += 2;
                    continue;
                }
                if chars[k] == '\'' {
                    return Some((k + 1).min(chars.len()));
                }
                k += 1;
            }
            return Some(chars.len());
        }
    }
    None
}
