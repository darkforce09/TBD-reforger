//! Source-text scanners used by the dev-login contract pins.
//!
//! The pins assert that `src/identity_and_access/handlers/developer_login.rs` still runs the
//! race-free first-create shape: `INSERT … arma_id NULL` plus a live
//! `UPDATE users SET arma_id = COALESCE(arma_id, …)`. A raw `contains` over the handler's
//! source is not enough to prove that, because the needle can appear in text that never
//! executes. Each scanner below removes one class of such text so what is finally matched is
//! SQL the crate actually sends:
//!
//! * [`strip_rust_comments_outside_literals`] — a Rust comment holding the needle keeps a
//!   raw grep green while the live statement is gone.
//! * [`rust_fn_body`] — the needle must be inside the file-scope `dev_login` item, not a
//!   dead sibling helper and not a definition nested in a `mod` / `impl` / block.
//! * [`sqlx_query_string_payloads`] — only direct `sqlx::query("…")` string arguments are
//!   SQL; `let _decoy = "…"` and `format!("…")` are not.
//! * [`strip_sql_comments_outside_literals`] — `--` and `/* */` inside a payload are not SQL.
//! * [`blank_sql_string_literal_contents`] — the interior of a `'…'`, `"…"` or `$tag$…$tag$`
//!   literal is data, so a `SELECT` carrying the needle inside one is not an `UPDATE`.
//!
//! What these scanners cannot decide is **reachability**: whether the statement they found
//! executes. `tests/misc_integration.rs` owns that half, by driving `GET /auth/dev-login`
//! against a real database and asserting the COALESCE semantics on the stored row.

/// Index just past the Rust literal that opens at `bytes[i]`, or `None` when none does.
///
/// Handles `"…"`, `r"…"` / `r#"…"#` (any hash count), the `b` / `c` prefixes, and char
/// literals `'x'` / `'\n'`. A `'` that introduces a **lifetime** (`&'static str`) opens
/// nothing: treating it as a char literal would open a span running to the next `'` anywhere
/// later in the file, so every comment in between would survive "comment-stripped" source and
/// the brace depth [`rust_fn_body`] relies on would be fiction.
pub(crate) fn rust_literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    let n = bytes.len();
    if i >= n {
        return None;
    }
    // Char literal vs lifetime: `'a` / `'static` are lifetimes; `'x'` and `'\n'` are not.
    if bytes[i] == b'\'' {
        let escaped = bytes.get(i + 1) == Some(&b'\\');
        let single = bytes.get(i + 2) == Some(&b'\'');
        if !escaped && !single {
            return None; // lifetime (or a lone quote) — consumes nothing
        }
        let mut j = i + 1;
        while j < n {
            if bytes[j] == b'\\' && j + 1 < n {
                j += 2;
                continue;
            }
            if bytes[j] == b'\'' {
                return Some(j + 1);
            }
            j += 1;
        }
        return Some(n);
    }
    // Not mid-identifier (`foo_r"…"` is not a raw string at the `r`).
    if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        return None;
    }
    let mut p = i;
    if matches!(bytes.get(p), Some(b'b' | b'c')) && matches!(bytes.get(p + 1), Some(b'r' | b'"')) {
        p += 1;
    }
    let mut hashes = 0usize;
    let raw = bytes.get(p) == Some(&b'r');
    if raw {
        p += 1;
        while bytes.get(p) == Some(&b'#') {
            hashes += 1;
            p += 1;
        }
    }
    if bytes.get(p) != Some(&b'"') {
        return None;
    }
    let mut j = p + 1;
    if raw {
        while j < n {
            if bytes[j] == b'"' && (1..=hashes).all(|h| bytes.get(j + h) == Some(&b'#')) {
                return Some(j + 1 + hashes);
            }
            j += 1;
        }
        return Some(n);
    }
    while j < n {
        if bytes[j] == b'\\' && j + 1 < n {
            j += 2;
            continue;
        }
        if bytes[j] == b'"' {
            return Some(j + 1);
        }
        j += 1;
    }
    Some(n)
}

/// Strip `//` line and `/* */` block comments outside string/char literals.
///
/// A pin that greps raw source for `COALESCE(arma_id` is hollow: a comment alone keeps it
/// green while the live UPDATE is gone. Always match against this instead of raw source.
///
/// Literal spans come from [`rust_literal_end`], so raw strings are copied whole (a `//`
/// inside `r#"…"#` is text, not a comment) and a lifetime does not open a char span that
/// swallows the comments after it.
pub(crate) fn strip_rust_comments_outside_literals(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(end) = rust_literal_end(bytes, i) {
            for &b in &bytes[i..end] {
                out.push(b as char);
            }
            i = end;
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Byte length of the PostgreSQL dollar-quote delimiter opening at `bytes[i]`
/// (`$$` → 2, `$decoy$` → 7), or `None` when one does not open there.
///
/// `$$…$$` / `$tag$…$tag$` is a string literal in PostgreSQL exactly as `'…'` is, so
/// [`blank_sql_string_literal_contents`] must recognise it — otherwise
/// `sqlx::query("SELECT $d$UPDATE users SET arma_id = COALESCE(arma_id, $2)$d$")` reaches the
/// needle match with its payload intact and keeps the pin green over a SELECT.
///
/// The tag rule is PostgreSQL's own: empty, or `[A-Za-z_][A-Za-z0-9_]*`. That is what keeps
/// the crate's real `$1` / `$2` bind placeholders from being read as delimiters — a digit
/// cannot start a tag, so `$2), updated_at` is not an opener and the live UPDATE is
/// unaffected.
pub(crate) fn pg_dollar_delim_len(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'$') {
        return None;
    }
    if bytes.get(i + 1) == Some(&b'$') {
        return Some(2);
    }
    let mut j = i + 1;
    if !bytes
        .get(j)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
    {
        return None;
    }
    while bytes
        .get(j)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    {
        j += 1;
    }
    if bytes.get(j) == Some(&b'$') {
        Some(j + 1 - i)
    } else {
        None
    }
}

/// End index (exclusive) of the dollar-quoted literal opening at `i`, or `None` when the
/// delimiter is never repeated — an unclosed `$tag$` is not a literal, so it is left alone
/// rather than swallowing the rest of the payload.
pub(crate) fn pg_dollar_literal_end(bytes: &[u8], i: usize, delim_len: usize) -> Option<usize> {
    let delim = &bytes[i..i + delim_len];
    let mut j = i + delim_len;
    while j + delim_len <= bytes.len() {
        if &bytes[j..j + delim_len] == delim {
            return Some(j + delim_len);
        }
        j += 1;
    }
    None
}

/// Strip SQL `--` line and `/* */` block comments outside string literals.
///
/// A `sqlx::query("SELECT 1 -- SET arma_id = COALESCE…")` payload carries the needle in text
/// the server never executes, so comment-strip the payload before matching the first-create
/// UPDATE shape.
///
/// Dollar-quoted literals are copied whole, so a `--` *inside* one cannot eat its closing
/// delimiter and leave the literal unrecognised by [`blank_sql_string_literal_contents`].
pub(crate) fn strip_sql_comments_outside_literals(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    let mut in_quote: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if in_quote.is_none()
            && let Some(dl) = pg_dollar_delim_len(bytes, i)
            && let Some(end) = pg_dollar_literal_end(bytes, i, dl)
        {
            for &b in &bytes[i..end] {
                out.push(b as char);
            }
            i = end;
            continue;
        }
        if let Some(q) = in_quote {
            out.push(c as char);
            if c == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if c == q {
                in_quote = None;
            }
            i += 1;
            continue;
        }
        if c == b'\'' || c == b'"' {
            in_quote = Some(c);
            out.push(c as char);
            i += 1;
            continue;
        }
        if c == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Blank interiors of SQL `'…'` / `"…"` / `$tag$…$tag$` literals (keep the delimiters).
///
/// String-literal contents survive comment strip, so a SELECT whose WHERE clause embeds
/// `'UPDATE users SET arma_id = COALESCE…'` — or the same text in PostgreSQL's dollar-quoting
/// syntax, `$decoy$UPDATE users SET arma_id = COALESCE(arma_id, $2)$decoy$` — still matches
/// the UPDATE needle with no live UPDATE anywhere. Blanking the interiors leaves only SQL the
/// server would treat as statement text. See [`pg_dollar_delim_len`] for why `$1` / `$2`
/// binds are not delimiters.
pub(crate) fn blank_sql_string_literal_contents(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(dl) = pg_dollar_delim_len(bytes, i)
            && let Some(end) = pg_dollar_literal_end(bytes, i, dl)
        {
            for &b in &bytes[i..i + dl] {
                out.push(b as char);
            }
            for _ in i + dl..end - dl {
                out.push(' ');
            }
            for &b in &bytes[end - dl..end] {
                out.push(b as char);
            }
            i = end;
            continue;
        }
        if c == b'\'' || c == b'"' {
            let q = c;
            out.push(c as char);
            i += 1;
            while i < bytes.len() {
                let d = bytes[i];
                // SQL standard: doubled quote escapes inside the same delimiter.
                if d == q && i + 1 < bytes.len() && bytes[i + 1] == q {
                    i += 2;
                    continue;
                }
                if d == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if d == q {
                    out.push(d as char);
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Collapse whitespace so `UPDATE  users\nSET…` matches the product UPDATE shape.
pub(crate) fn flatten_sql_ws(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Brace-balanced body of the **file-scope** item `fn {name}` in comment-stripped source
/// (opening `{` … matching `}`).
///
/// The COALESCE pin must bind to the live first-create path (`dev_login`), and "live" has two
/// enemies that a textual search cannot tell apart from the real item:
///
/// * a dead sibling helper that retains the UPDATE while the executed path uses `$2`;
/// * a *nested* definition that wins `str::find`, as in
///   `mod decoy { async fn dev_login() { …COALESCE… } }` sitting above the real
///   `pub async fn dev_login(…)`.
///
/// Both are excluded by construction rather than by enumeration: braces are counted (literal
/// spans skipped via [`rust_literal_end`]) and only markers at **depth 0** are candidates, so
/// anything inside a `mod`, `impl`, `fn` or block is not the item the crate calls.
///
/// Requiring exactly one such definition costs nothing — two file-scope `fn dev_login`s do
/// not compile — and turns "which one did it read?" into a named failure.
pub(crate) fn rust_fn_body<'a>(code: &'a str, name: &str) -> &'a str {
    let marker = format!("fn {name}");
    let m = marker.as_bytes();
    let bytes = code.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    let mut starts: Vec<usize> = Vec::new();
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {
                if depth == 0
                    && bytes[i..].starts_with(m)
                    && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'))
                    && bytes
                        .get(i + m.len())
                        .is_none_or(|c| !(c.is_ascii_alphanumeric() || *c == b'_'))
                {
                    starts.push(i);
                }
            }
        }
        i += 1;
    }
    let start = match starts.as_slice() {
        [only] => *only,
        [] => panic!(
            "no FILE-SCOPE `{marker}` in comment-stripped source. A definition nested in a \
             `mod`/`impl`/block is not the item the crate calls — the pin binds to the \
             top-level one on purpose."
        ),
        many => panic!(
            "{} file-scope `{marker}` definitions (offsets {many:?}) — a Rust file cannot have \
             two, so this source is not what the crate compiles.",
            many.len()
        ),
    };
    let after = &code[start..];
    let open = after
        .find('{')
        .unwrap_or_else(|| panic!("`{marker}` has no opening brace"));
    let bytes = after.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        if bytes[i] == b'{' {
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return &after[open..=i];
            }
        }
        i += 1;
    }
    panic!("`{marker}` body not closed");
}

/// True when a direct `sqlx::query("…")` payload is the first-create arma UPDATE
/// (`UPDATE users SET arma_id = COALESCE(arma_id…`), not a SELECT/comment/string decoy.
///
/// An any-payload `contains("SET arma_id = COALESCE(arma_id")` is hollow: a SELECT with the
/// needle in a SQL comment, or inside a SQL string literal, stays green with the real UPDATE
/// deleted. Both classes are removed before the statement shape is matched.
pub(crate) fn sqlx_payload_is_arma_coalesce_update(payload: &str) -> bool {
    let comments_gone = strip_sql_comments_outside_literals(payload);
    let literals_blank = blank_sql_string_literal_contents(&comments_gone);
    let flat = flatten_sql_ws(&literals_blank);
    flat.contains("UPDATE users SET arma_id = COALESCE(arma_id")
}

/// True when any payload in the set is the first-create arma UPDATE.
pub(crate) fn sqlx_queries_have_arma_coalesce_update(payloads: &[String]) -> bool {
    payloads
        .iter()
        .any(|p| sqlx_payload_is_arma_coalesce_update(p))
}

/// Interiors of `"…"` / concatenated `"…" "…"` literals that are *direct* args to
/// `sqlx::query(`. Skips `format!("…")`, `let _decoy = "…"`, and `&format!(…)` wrappers.
///
/// The live COALESCE path is SQL text inside `sqlx::query("…")`. A decoy string binding or a
/// `format!` carrying the same needle is not SQL the crate sends, so it must not satisfy the
/// pin.
pub(crate) fn sqlx_query_string_payloads(code: &str) -> Vec<String> {
    let bytes = code.as_bytes();
    let key = b"sqlx::query";
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + key.len() <= bytes.len() {
        if &bytes[i..i + key.len()] != key {
            i += 1;
            continue;
        }
        let before_ok = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        let mut j = i + key.len();
        // Reject `sqlx::query_as` / `sqlx::query_scalar` — only bare `sqlx::query(`.
        if j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
            i = j;
            continue;
        }
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if !before_ok || j >= bytes.len() || bytes[j] != b'(' {
            i += key.len();
            continue;
        }
        j += 1; // past '('
        let mut payload = String::new();
        loop {
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= bytes.len() || bytes[j] != b'"' {
                break;
            }
            j += 1; // opening "
            while j < bytes.len() {
                let c = bytes[j];
                if c == b'\\' && j + 1 < bytes.len() {
                    let n = bytes[j + 1];
                    // Rust string line-continuation: `\` + newline (optional CR).
                    if n == b'\n' {
                        j += 2;
                        continue;
                    }
                    if n == b'\r' && j + 2 < bytes.len() && bytes[j + 2] == b'\n' {
                        j += 3;
                        continue;
                    }
                    payload.push(n as char);
                    j += 2;
                    continue;
                }
                if c == b'"' {
                    j += 1;
                    break;
                }
                payload.push(c as char);
                j += 1;
            }
        }
        if !payload.is_empty() {
            out.push(payload);
        }
        i = j.max(i + key.len());
    }
    out
}

/// Split a SQL column/value list on top-level commas (parens + quotes aware).
pub(crate) fn split_sql_list(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_quote: Option<char> = None;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = in_quote {
            cur.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
                continue;
            }
            if c == q {
                in_quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                in_quote = Some(c);
                cur.push(c);
            }
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
    out
}

/// Locate `INSERT INTO users (…) VALUES (…)` in comment-stripped handler source and
/// return the `arma_id` VALUES expression (product must be `NULL`).
pub(crate) fn users_insert_arma_id_value(code: &str) -> Option<String> {
    // Collapse Rust string line-continuations so the SQL reads as one line.
    let flat: String = code
        .replace("\\\r\n", " ")
        .replace("\\\n", " ")
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let insert_key = "INSERT INTO users";
    let start = flat.find(insert_key)?;
    let after_insert = &flat[start + insert_key.len()..];
    let cols_open = after_insert.find('(')?;
    let cols_close = {
        let mut depth = 0i32;
        let bytes: Vec<char> = after_insert.chars().collect();
        let mut close = None;
        for (i, &c) in bytes.iter().enumerate().skip(cols_open) {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        close?
    };
    let cols = split_sql_list(&after_insert[cols_open + 1..cols_close]);
    let arma_idx = cols.iter().position(|c| c == "arma_id")?;
    let rest = &after_insert[cols_close + 1..];
    let values_key = "VALUES";
    let vpos = rest.find(values_key)?;
    let after_values = &rest[vpos + values_key.len()..];
    let v_open = after_values.find('(')?;
    let v_close = {
        let bytes: Vec<char> = after_values.chars().collect();
        let mut depth = 0i32;
        let mut close = None;
        let mut in_quote: Option<char> = None;
        let mut i = v_open;
        while i < bytes.len() {
            let c = bytes[i];
            if let Some(q) = in_quote {
                if c == '\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if c == q {
                    in_quote = None;
                }
                i += 1;
                continue;
            }
            match c {
                '\'' | '"' => in_quote = Some(c),
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        close?
    };
    let vals = split_sql_list(&after_values[v_open + 1..v_close]);
    vals.get(arma_idx).cloned()
}
