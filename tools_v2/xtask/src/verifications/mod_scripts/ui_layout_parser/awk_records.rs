/// awk record splitting with `RS="\n"`.
///
/// NOT `str::lines()`: that strips a trailing `\r`, so on a CRLF layout the line content would be
/// silently altered and a `SizeX 5\r` would print differently from what awk prints.
pub(super) fn awk_records(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut recs: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        recs.pop();
    }
    recs
}

/// `/^[ \t]*[A-Za-z_]+WidgetClass[ \t{]/`
pub(super) fn is_widget_decl(line: &str) -> bool {
    let s = trim_leading_blank(line);
    let b = s.as_bytes();
    let mut n = 0;
    while n < b.len() && (b[n].is_ascii_alphabetic() || b[n] == b'_') {
        n += 1;
    }
    // `[A-Za-z_]+` needs at least one character BEFORE the literal, so a bare `WidgetClass {` is
    // not a declaration. The identifier run is greedy, so the suffix test applies to the whole run.
    if n < "WidgetClass".len() + 1 || !s[..n].ends_with("WidgetClass") {
        return false;
    }
    matches!(b.get(n), Some(b' ' | b'\t' | b'{'))
}

/// `/^[ \t]*Slot[ \t]+[A-Za-z_]+/` plus the three `sub()`s that isolate the class name.
pub(super) fn slot_class(line: &str) -> Option<String> {
    let s = trim_leading_blank(line).strip_prefix("Slot")?;
    let rest = s.trim_start_matches([' ', '\t']);
    if rest.len() == s.len() {
        return None; // `[ \t]+` requires at least one separator
    }
    let name = cut_at_blank_or_brace(rest);
    // `[A-Za-z_]+` must match at least once for awk's `match()` to fire.
    let first = name.as_bytes().first()?;
    if !(first.is_ascii_alphabetic() || *first == b'_') {
        return None;
    }
    Some(name.to_string())
}

/// `/^[ \t]*KEYWORD[ \t]/` — the trailing separator is part of the bash alternation and is what
/// stops `PositionX` matching a line beginning `PositionXY`.
pub(super) fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    match trim_leading_blank(line).strip_prefix(keyword) {
        Some(rest) => rest.starts_with(' ') || rest.starts_with('\t'),
        None => false,
    }
}

/// `sub(/^[ \t]*[A-Za-z]+[ \t]+/, "", v); sub(/[ \t]*$/, "", v)` — the value half of a
/// `KEYWORD VALUE` line, right-trimmed of blanks only (a trailing `\r` survives, as in awk).
pub(super) fn keyword_value(line: &str) -> &str {
    let s = trim_leading_blank(line);
    let b = s.as_bytes();
    let mut n = 0;
    while n < b.len() && b[n].is_ascii_alphabetic() {
        n += 1;
    }
    let rest = &s[n..];
    let trimmed = rest.trim_start_matches([' ', '\t']);
    if trimmed.len() == rest.len() {
        return s.trim_end_matches([' ', '\t']); // no `[ \t]+`: the first sub did not fire
    }
    trimmed.trim_end_matches([' ', '\t'])
}

/// `sub(/^[ \t]*/, "", x)`
pub(super) fn trim_leading_blank(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
}

/// `sub(/[ \t].*$/, "", x); sub(/\{.*$/, "", x)`
pub(super) fn cut_at_blank_or_brace(s: &str) -> &str {
    let end = s
        .find([' ', '\t'])
        .unwrap_or(s.len())
        .min(s.find('{').unwrap_or(s.len()));
    &s[..end]
}

/// `gsub(/"[^"]*"/, "", bl)` — THE fix that stopped GUIDs desyncing the brace counter.
///
/// Non-overlapping, left to right. An unterminated literal is left alone, exactly as the ERE leaves
/// it, so `Text "a { b` still contributes its `{` to the count. See the module docs: without this,
/// the gate reports clean over the very files it was written to reject.
pub(super) fn strip_quoted(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('"') {
        match rest[open + 1..].find('"') {
            Some(rel) => {
                out.push_str(&rest[..open]);
                rest = &rest[open + 1 + rel + 1..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// awk's `v + 0`: the longest leading decimal prefix, else 0.
///
/// Decimal-only on purpose — gawk does not read hex from input data without `--non-decimal-data`.
/// This is why a `SizeMode Fill` line contributes `0` rather than raising an error.
pub(super) fn awk_to_number(s: &str) -> f64 {
    let t = s.trim_start_matches([' ', '\t', '\n', '\r']);
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    let mut end = i;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            end = j;
        }
    }
    t[..end].parse::<f64>().unwrap_or(0.0)
}

/// awk's number→string under `%s`: integral values print with `%d`, everything else with `CONVFMT`
/// (`%.6g`). This is why `-480` renders as `-480` and not `-480.000000`.
pub(super) fn awk_to_string(x: f64) -> String {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 1e16 {
        // `-(0 + 0)` is IEEE `-0.0`; awk prints it as `0`, and so does the cast.
        return format!("{}", x as i64);
    }
    format_g6(x)
}

/// `%.6g`, which Rust's formatter does not provide.
pub(super) fn format_g6(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    let exp = x.abs().log10().floor() as i32;
    if !(-4..6).contains(&exp) {
        let mantissa = x / 10f64.powi(exp);
        let m = trim_float(&format!("{mantissa:.5}"));
        return format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    trim_float(&format!("{:.*}", (5 - exp).max(0) as usize, x))
}

pub(super) fn trim_float(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
