//! Rust-source scanners: the GET routes the route tables register, and the `SELECT` literals
//! handed to `query_as` / `QueryBuilder::new` across `src/`.
//!
//! Text scanning is what lets the enumerating half reach code no seed can satisfy a predicate
//! for — including handlers no behavioural test exercises at all.

use std::collections::{BTreeMap, BTreeSet};

pub fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Every source file that registers a route: the eight domain route tables, which hold the whole
/// of `/api/v1`, plus `core/http_router.rs`, which holds `/healthz` and `/metrics`.
///
/// Concatenated rather than parsed one file at a time because [`registered_get_routes`] keys on
/// the path alone and a `.route(` call never spans two files.
pub fn router_source() -> String {
    const PARTS: [&str; 9] = [
        include_str!("../../src/core/http_router.rs"),
        include_str!("../../src/identity_and_access/routes.rs"),
        include_str!("../../src/operations/routes.rs"),
        include_str!("../../src/missions/routes.rs"),
        include_str!("../../src/server_infrastructure/routes.rs"),
        include_str!("../../src/administration/routes.rs"),
        include_str!("../../src/match_telemetry/routes.rs"),
        include_str!("../../src/command_center/routes.rs"),
        include_str!("../../src/community_content/routes.rs"),
    ];
    PARTS.concat()
}

/// Parse the GET route paths [`router_source`] registers.
///
/// Matches `.route("<path>", ... get( ... )` including the rustfmt-wrapped multi-line form, by
/// taking the literal and then checking for `get(` inside that `.route(` call's balanced parens.
pub fn registered_get_routes(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (idx, _) in src.match_indices(".route(") {
        let open = idx + ".route(".len() - 1;
        let Some(end) = balanced_end(src, open) else {
            continue;
        };
        let call = &src[open..end];
        let Some((_, path)) = string_literal(call, 0) else {
            continue;
        };
        if call.contains("get(") {
            out.insert(path);
        }
    }
    out
}

/// Index just past the `)` matching the `(` at `open`.
pub fn balanced_end(src: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in src[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Read the first Rust string literal at/after `from`, resolving the escapes that appear in
/// these SQL literals: `\`+newline (rustfmt line continuation) collapses away, `\n`/`\t` become
/// spaces, `\"` and `\'` are literal. Returns `(index past the closing quote, contents)`.
pub fn string_literal(src: &str, from: usize) -> Option<(usize, String)> {
    let bytes: Vec<char> = src.chars().collect();
    let idx: Vec<usize> = src.char_indices().map(|(i, _)| i).collect();
    let mut i = idx.iter().position(|&b| b >= from)?;
    while i < bytes.len() && bytes[i] != '"' {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    i += 1;
    let mut out = String::new();
    while i < bytes.len() {
        match bytes[i] {
            '\\' => {
                match bytes.get(i + 1) {
                    Some('\n') => {}
                    Some('n') | Some('t') => out.push(' '),
                    Some(c) => out.push(*c),
                    None => {}
                }
                i += 2;
            }
            '"' => {
                let end = idx.get(i).copied().unwrap_or(src.len()) + 1;
                return Some((end, out));
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    None
}

/// Whitespace-normalised `SELECT ...` literals passed to `query_as` / `QueryBuilder::new`,
/// with the 1-based source line of the call.
pub fn select_literals(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for marker in ["query_as", "QueryBuilder::new"] {
        for (idx, _) in src.match_indices(marker) {
            let Some((_, raw)) = string_literal(src, idx + marker.len()) else {
                continue;
            };
            let sql = raw.split_whitespace().collect::<Vec<_>>().join(" ");
            if sql.len() >= 6 && sql[..6].eq_ignore_ascii_case("SELECT") {
                out.push((src[..idx].matches('\n').count() + 1, sql));
            }
        }
    }
    out
}

/// `alias -> table` for every `FROM`/`JOIN` in the statement. A table with no alias maps to
/// itself, so both `missions.updated_at` and `m.updated_at` resolve.
pub fn table_aliases(sql: &str) -> BTreeMap<String, String> {
    const NOISE: &[&str] = &[
        "on", "where", "order", "group", "limit", "left", "right", "inner", "outer", "join",
        "using", "set", "as", "and", "or", "having", "offset", "union",
    ];
    let mut out = BTreeMap::new();
    let toks: Vec<&str> = sql.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        if !t.eq_ignore_ascii_case("FROM") && !t.eq_ignore_ascii_case("JOIN") {
            continue;
        }
        let Some(raw) = toks.get(i + 1) else { continue };
        let table = raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let table = table.strip_prefix("public.").unwrap_or(table).to_string();
        if table.is_empty() || !table.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            continue;
        }
        out.insert(table.clone(), table.clone());
        if let Some(next) = toks.get(i + 2) {
            let alias = next.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !alias.is_empty()
                && alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !NOISE.iter().any(|n| alias.eq_ignore_ascii_case(n))
            {
                out.insert(alias.to_string(), table);
            }
        }
    }
    out
}

/// Top-level, comma-separated items of the select list (everything between `SELECT` and the
/// statement's top-level `FROM`). Depth-aware, so `count(*)` and scalar subqueries are one item.
pub fn select_items(sql: &str) -> Vec<String> {
    let upper = sql.to_uppercase();
    let mut depth = 0i32;
    let mut from_at = None;
    let b = upper.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'F' if depth == 0 && upper[i..].starts_with("FROM") => {
                let before_ok = i == 0 || !b[i - 1].is_ascii_alphanumeric() && b[i - 1] != b'_';
                let after = b.get(i + 4).copied().unwrap_or(b' ');
                if before_ok && !after.is_ascii_alphanumeric() && after != b'_' {
                    from_at = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let Some(end) = from_at else {
        return Vec::new();
    };
    let list = &sql[6..end];
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in list.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The column name a select item produces, its defining expression, and its table qualifier if
/// it is a plain qualified reference. `COALESCE(m.tag,'') AS tag` → `("tag", "COALESCE(..)", None)`;
/// `m.updated_at` → `("updated_at", "m.updated_at", Some("m"))`.
pub fn produced_column(item: &str) -> Option<(String, String, Option<&str>)> {
    let ident = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !s.starts_with(|c: char| c.is_ascii_digit())
    };
    // `<expr> AS <alias>`
    let up = item.to_uppercase();
    if let Some(pos) = up.rfind(" AS ") {
        let alias = item[pos + 4..].trim();
        if ident(alias) {
            return Some((alias.to_ascii_lowercase(), item[..pos].to_string(), None));
        }
    }
    // `table.col` / `col`
    let (qual, bare) = match item.split_once('.') {
        Some((q, c)) if ident(q) && ident(c) => (Some(q), c),
        _ if ident(item) => (None, item),
        _ => return None,
    };

    Some((bare.to_ascii_lowercase(), item.to_string(), qual))
}
