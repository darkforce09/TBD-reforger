//! python-compatibility helpers for the T-288 render half — split out of [`super::config`] for
//! SIZE-3.
//!
//! ── WHY A PORT CARRIES A PYTHON EMULATOR AT ALL ──────────────────────────────────────────────
//!
//! `deploy-staging.sh` did its JSON work in `python3` (14 call sites) because "`jq` is NOT
//! installed here (measured) and hand-rolled JSON in bash silently emits invalid documents". Those
//! call sites are gone — `serde_json` is compiled in. But four python behaviours were OBSERVABLE
//! in the script's output, and a wave log greps that output:
//!
//! | python | why it is observable | here |
//! |--------|----------------------|------|
//! | `json.dumps(..., ensure_ascii=True)` | the rendered `game.mods[]` bytes | [`ensure_ascii`] |
//! | `%r` on a `str` | every fail-closed message names the offending mod | [`py_repr`] |
//! | `type(x).__name__` | "`mods` must be an array, got dict" | [`py_type_name`] |
//! | `str(x or "")` | which values count as an empty name / workshop_id | [`py_str_or_empty`] |
//! | `json.JSONDecodeError` | the "not valid JSON" diagnostic | [`py_json_error`] |
//!
//! Every expectation in this module was MEASURED against `python3` on 2026-08-12, not inferred
//! from documentation. Where a behaviour could not be reproduced exactly, the code says so at the
//! site and falls back to `serde_json`'s own wording rather than guessing — see [`py_json_error`].

use serde_json::Value;

/// python's `json.dumps(..., ensure_ascii=True)`: every non-ASCII scalar becomes `\uXXXX`, with
/// surrogate pairs above the BMP. `serde_json` emits raw UTF-8, so without this a modpack whose
/// name carries an accent would render different bytes than the bash did.
pub(super) fn ensure_ascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii() {
            out.push(ch);
        } else {
            let mut buf = [0u16; 2];
            for unit in ch.encode_utf16(&mut buf) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

/// python's `json.JSONDecodeError` text, for the one family of failures where it is exactly
/// determinable without reimplementing python's parser.
///
/// WHY THIS EXISTS. The bash printed `%s` of python's exception, and a wave log carries that line.
/// `serde_json` words the same failures differently ("expected ident at line 1 column 2" where
/// python says "Expecting value: line 1 column 1 (char 0)"), which would show up as a diff on the
/// only offline mode whose output is not otherwise byte-identical.
///
/// WHAT IS EXACT, and how it was established. Measured against `python3 -c 'json.load(...)'` on
/// 2026-08-12 for 21 malformed inputs. When the first non-whitespace character cannot BEGIN a JSON
/// value, python always answers
///
/// ```text
/// Expecting value: line <L> column <C> (char <N>)
/// ```
///
/// pointing at that character — verified for `not json at all {`, `nul`, `tru`, `True`, `'a'`, `}`,
/// `-`, `-x`, the empty document, and a whitespace-only document (which points one past the
/// whitespace: `line 1 column 4 (char 3)` for three spaces). That condition is decidable here from
/// the text alone, so this function decides it directly and does NOT read `serde_json`'s
/// classification for it.
///
/// SECOND EXACT FAMILY: a token in a VALUE POSITION inside a container — the shape a bad
/// `TBD_GAME_PORT` or `TBD_MAX_PLAYERS` produces in the rendered config, e.g.
/// `"bindPort": not-a-port,`. python points at the first character of the offending token; serde
/// points at that character for a token that cannot start a value at all (`abc`) and one character
/// later for one that could have been a keyword (`not…` looks like the start of `null`). Walking
/// back to the token start reconciles both, and the position is only trusted when the preceding
/// non-whitespace character is `:`, `,` or `[` — i.e. somewhere a JSON value really was expected.
///
/// WHAT FALLS BACK, deliberately. Everything else — `Extra data`, `Expecting ':' delimiter`,
/// `Expecting property name enclosed in double quotes`, `Unterminated string starting at`, and any
/// truncation that serde classifies as EOF — keeps `serde_json`'s wording. Emulating those would
/// mean guessing a position python computes with a different scanner, and a message that
/// confidently names the wrong column is worse than one that is honestly worded differently. The
/// remaining divergence is reported, not hidden.
pub(super) fn py_json_error(text: &str, e: &serde_json::Error) -> String {
    // python's json whitespace set is exactly ' \t\n\r' — NOT unicode whitespace.
    let chars: Vec<char> = text.chars().collect();
    let mut p = 0usize;
    while p < chars.len() && matches!(chars[p], ' ' | '\t' | '\n' | '\r') {
        p += 1;
    }
    if !begins_json_value(&chars, p) {
        return expecting_value_at(&chars, p);
    }
    // ── the in-container family ──────────────────────────────────────────────────────────────
    if !e.is_syntax() {
        return e.to_string();
    }
    let Some(mut i) = char_index_of(&chars, e.line(), e.column()) else {
        return e.to_string();
    };
    // Walk back to the start of the token serde stopped inside.
    let tokenish = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '.');
    while i > 0 && tokenish(chars[i - 1]) && tokenish(chars[i]) {
        i -= 1;
    }
    // Only a genuine value position. This is what keeps `0x1` — where the token starts at the top
    // of the document and python answers `Extra data` — out of this family.
    let mut j = i;
    while j > 0 && matches!(chars[j - 1], ' ' | '\t' | '\n' | '\r') {
        j -= 1;
    }
    if j == 0 || !matches!(chars[j - 1], ':' | ',' | '[') {
        return e.to_string();
    }
    expecting_value_at(&chars, i)
}

/// Could a JSON value START at `p`, by python's decoder's rules?
fn begins_json_value(chars: &[char], p: usize) -> bool {
    if p >= chars.len() {
        return false;
    }
    let c = chars[p];
    if c == '{' || c == '[' || c == '"' || c.is_ascii_digit() {
        return true;
    }
    // python's NUMBER_RE needs a digit after the sign, so a lone `-` is not a value.
    if c == '-' && chars.get(p + 1).is_some_and(|n| n.is_ascii_digit()) {
        return true;
    }
    // python's decoder accepts these non-standard literals, and `-Infinity`.
    let rest: String = chars[p..].iter().collect();
    ["true", "false", "null", "NaN", "Infinity", "-Infinity"]
        .iter()
        .any(|k| rest.starts_with(k))
}

/// python's line/column are 1-based and counted in CHARACTERS; `char` is the 0-based index.
fn expecting_value_at(chars: &[char], p: usize) -> String {
    let before: String = chars[..p.min(chars.len())].iter().collect();
    let line = before.matches('\n').count() + 1;
    let col = match before.rfind('\n') {
        Some(i) => before[i + 1..].chars().count() + 1,
        None => p + 1,
    };
    format!("Expecting value: line {line} column {col} (char {p})")
}

/// `serde_json`'s 1-based (line, column) back to a 0-based character index.
fn char_index_of(chars: &[char], line: usize, column: usize) -> Option<usize> {
    let mut cur_line = 1usize;
    let mut cur_col = 1usize;
    for (i, c) in chars.iter().enumerate() {
        if cur_line == line && cur_col == column {
            return Some(i);
        }
        if *c == '\n' {
            cur_line += 1;
            cur_col = 1;
        } else {
            cur_col += 1;
        }
    }
    None
}

/// python's `%r` for a `str`: single quotes, unless the value contains `'` and no `"`.
pub(super) fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// python's `type(x).__name__` for the values `json.load` can produce.
pub(super) fn py_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) => {
            if n.is_f64() {
                "float"
            } else {
                "int"
            }
        }
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// python's `str(m.get(k) or "")`. Falsy values (null, false, 0, "", [], {}) collapse to "".
pub(super) fn py_str_or_empty(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => String::new(),
        Some(Value::Bool(true)) => "True".into(),
        Some(Value::Number(n)) => {
            if n.as_f64() == Some(0.0) {
                String::new()
            } else {
                n.to_string()
            }
        }
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) if a.is_empty() => String::new(),
        Some(Value::Object(o)) if o.is_empty() => String::new(),
        Some(other) => other.to_string(),
    }
}

/// `modpack_mods_json` — the `game.mods[]` array (JSON text) for a modpack document, or fail closed.
///
/// Returns the string ALREADY re-indented for substitution into the config template: python did
/// `"\n".join(ln if i == 0 else "    " + ln …)`, so the opening `[` sits flush after `"mods": ` and
/// python `%r` over a JSON scalar, for the a2s/bindPort error. Ints print bare, strings quoted.
pub(super) fn json_repr(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "None".into(),
        Some(Value::String(s)) => py_repr(s),
        Some(Value::Bool(true)) => "True".into(),
        Some(Value::Bool(false)) => "False".into(),
        Some(other) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_json_error_reproduces_the_measured_python_messages() {
        // FIRST FAMILY: the first non-whitespace token cannot begin a value. Every expectation was
        // measured against python3 on 2026-08-12.
        let exact = [
            (
                "not json at all {",
                "Expecting value: line 1 column 1 (char 0)",
            ),
            ("nul", "Expecting value: line 1 column 1 (char 0)"),
            ("tru", "Expecting value: line 1 column 1 (char 0)"),
            ("True", "Expecting value: line 1 column 1 (char 0)"),
            ("'a'", "Expecting value: line 1 column 1 (char 0)"),
            ("}", "Expecting value: line 1 column 1 (char 0)"),
            ("-", "Expecting value: line 1 column 1 (char 0)"),
            ("-x", "Expecting value: line 1 column 1 (char 0)"),
            ("", "Expecting value: line 1 column 1 (char 0)"),
            ("   ", "Expecting value: line 1 column 4 (char 3)"),
            ("\n\n  x", "Expecting value: line 3 column 3 (char 4)"),
        ];
        for (input, want) in exact {
            let e = serde_json::from_str::<Value>(input).expect_err("must not parse");
            assert_eq!(py_json_error(input, &e), want, "input={input:?}");
        }
        // SECOND FAMILY: a bad token in a value position inside a container — the shape a bad
        // TBD_GAME_PORT or TBD_MAX_PLAYERS produces in the rendered config.
        let in_container = [
            ("[1,]", "Expecting value: line 1 column 4 (char 3)"),
            (r#"{"a":}"#, "Expecting value: line 1 column 6 (char 5)"),
            (r#"{"a": }"#, "Expecting value: line 1 column 7 (char 6)"),
            (r#"{"a": ,}"#, "Expecting value: line 1 column 7 (char 6)"),
            (
                r#"{"a": nope}"#,
                "Expecting value: line 1 column 7 (char 6)",
            ),
            (
                r#"{"a": abc, "b":1}"#,
                "Expecting value: line 1 column 7 (char 6)",
            ),
            ("[1, oops]", "Expecting value: line 1 column 5 (char 4)"),
            (
                r#"{"p": not-a-port,}"#,
                "Expecting value: line 1 column 7 (char 6)",
            ),
            // Multi-line, which is the shape the rendered server config actually has.
            (
                "{\n  \"a\": 1,\n  \"b\": zzz\n}",
                "Expecting value: line 3 column 8 (char 19)",
            ),
        ];
        for (input, want) in in_container {
            let e = serde_json::from_str::<Value>(input).expect_err("must not parse");
            assert_eq!(py_json_error(input, &e), want, "input={input:?}");
        }
        // The declared fallbacks. python answers `Extra data`, `Expecting ':' delimiter`,
        // `Expecting property name…`, `Unterminated string…` or (for `[1,`) an `Expecting value`
        // that serde classifies as EOF rather than syntax. None of those positions is derivable
        // here, so serde's own wording stands and the divergence is declared rather than guessed.
        for input in ["{", r#"{"a""#, "[1,", "1 2", "\"abc", "0x1", "01", "{}x"] {
            let e = serde_json::from_str::<Value>(input).expect_err("must not parse");
            let got = py_json_error(input, &e);
            assert!(
                !got.starts_with("Expecting value:"),
                "input={input:?} must fall back, got {got}"
            );
        }
    }

    #[test]
    fn py_repr_follows_pythons_quote_choice() {
        assert_eq!(py_repr("B"), "'B'");
        assert_eq!(py_repr("<unnamed>"), "'<unnamed>'");
        // repr switches to double quotes when the value has a single quote and no double.
        assert_eq!(py_repr("it's"), "\"it's\"");
        // Both present -> single quotes, with the inner single quote escaped.
        assert_eq!(py_repr("it's \"x\""), "'it\\'s \"x\"'");
    }

    #[test]
    fn ensure_ascii_matches_json_dumps_default() {
        assert_eq!(ensure_ascii("plain"), "plain");
        assert_eq!(ensure_ascii("Café"), "Caf\\u00e9");
        // Above the BMP -> a surrogate pair, exactly as python emits.
        assert_eq!(ensure_ascii("\u{1F600}"), "\\ud83d\\ude00");
    }

    #[test]
    fn py_type_names_match() {
        assert_eq!(py_type_name(&serde_json::json!([])), "list");
        assert_eq!(py_type_name(&serde_json::json!({})), "dict");
        assert_eq!(py_type_name(&serde_json::json!("s")), "str");
        assert_eq!(py_type_name(&serde_json::json!(1)), "int");
        assert_eq!(py_type_name(&serde_json::json!(1.5)), "float");
        assert_eq!(py_type_name(&Value::Null), "NoneType");
    }

    #[test]
    fn py_str_or_empty_follows_pythons_truthiness() {
        // `str(m.get(k) or "")` — every falsy value collapses to the empty string, which is what
        // makes `mods[i].name is empty` fire for `null`, `false`, `0` and `""` alike.
        assert_eq!(py_str_or_empty(None), "");
        assert_eq!(py_str_or_empty(Some(&Value::Null)), "");
        assert_eq!(py_str_or_empty(Some(&serde_json::json!(false))), "");
        assert_eq!(py_str_or_empty(Some(&serde_json::json!(0))), "");
        assert_eq!(py_str_or_empty(Some(&serde_json::json!(""))), "");
        assert_eq!(py_str_or_empty(Some(&serde_json::json!([]))), "");
        // …and every truthy one keeps python's `str()`.
        assert_eq!(py_str_or_empty(Some(&serde_json::json!(true))), "True");
        assert_eq!(py_str_or_empty(Some(&serde_json::json!(5))), "5");
        assert_eq!(py_str_or_empty(Some(&serde_json::json!("x"))), "x");
    }

    #[test]
    fn json_repr_of_a_scalar() {
        assert_eq!(json_repr(None), "None");
        assert_eq!(json_repr(Some(&serde_json::json!(2001))), "2001");
        assert_eq!(json_repr(Some(&serde_json::json!("2001"))), "'2001'");
        assert_eq!(json_repr(Some(&serde_json::json!(true))), "True");
    }
}
