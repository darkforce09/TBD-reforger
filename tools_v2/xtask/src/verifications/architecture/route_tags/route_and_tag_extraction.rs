//! Reading the two sides this gate compares out of source text.
//!
//! One half discovers and parses the domain route tables (`src/<domain>/routes.rs`) plus the
//! `.merge(` lines that serve them; the other sweeps the whole tree for `@route` doc tags. Both
//! sides emit a marker row — `UNPARSED …` / `ORPHAN …` — for any input they could not read, so a
//! shape this file does not understand shrinks no count in silence.

use super::*;

/// One discovered domain route table.
pub(super) struct RouteTable {
    /// The single path segment between `src/` and `routes.rs` — the domain name the merge names.
    pub domain: String,
    /// Repo-relative, for the mount-mismatch and UNPARSED messages.
    pub rel: String,
    pub text: String,
}

/// Every `src/<domain>/routes.rs` in the tree, in sorted order.
///
/// A route table is EXACTLY one directory level below `src`: anything deeper is a handler, a model
/// or a test, all of which are swept for `@route` tags but never for registrations. A missing or
/// unreadable tree is a `NotRun`, never an empty list — the same fail-closed rule the tag sweep
/// follows, and for the same reason.
pub(super) fn discover_route_files(repo_root: &Path) -> Result<Vec<RouteTable>, NotRun> {
    let src_dir = repo_root.join(SRC_DIR_REL);
    let files = scan::walk_files(&[&src_dir], |p| {
        p.file_name().is_some_and(|n| n == ROUTES_FILE)
            && p.parent()
                .and_then(|d| d.parent())
                .is_some_and(|g| g == src_dir)
    })?;
    let mut out = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).map_err(|source| NotRun::Unreadable {
            path: path.clone(),
            source,
        })?;
        let domain = path
            .parent()
            .and_then(|d| d.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        out.push(RouteTable { domain, rel, text });
    }
    Ok(out)
}

/// The `<domain>` of every `.merge(crate::<domain>::routes(` inside the [`MERGE_FN`] body.
///
/// The range runs from the column-0 `MERGE_FN` line to the next column-0 `}`, which in this
/// rustfmt-clean crate is that function's own closing brace — so a `.merge(` anywhere else in
/// `http_router.rs` is invisible, exactly as a `.route(` outside a route table is.
pub(super) fn merged_domains(router_src: &str) -> BTreeSet<String> {
    let merge =
        Regex::new(r"\.merge\(crate::([A-Za-z_][A-Za-z_0-9]*)::routes\(").expect("static regex");
    let mut out = BTreeSet::new();
    let mut in_range = false;
    for line in router_src.lines() {
        if !in_range {
            in_range = line.starts_with(MERGE_FN);
            continue;
        }
        if line.starts_with('}') {
            in_range = false;
            continue;
        }
        if let Some(c) = merge.captures(line) {
            out.insert(c[1].to_string());
        }
    }
    out
}

/// The comment-stripped body lines of a route table's `pub fn routes`, and how many such functions
/// the file declares.
///
/// `sed -n '/^pub fn routes(/,/^}/p' | sed 's://.*$::'` — the range runs to the next column-0 `}`,
/// which is the function's own closing brace in this rustfmt-clean crate. Comments go so a
/// commented-out `.route(...)` cannot read as live; the strip is naive and would also cut a `//`
/// inside a string literal, a hazard the script carried and this port keeps.
///
/// The count is returned rather than assumed, because the router side is only trustworthy if each
/// file holds EXACTLY one table: zero means the file was renamed out from under this extractor,
/// and two means half the registrations would be read with the wrong `dev`/limit parameters by
/// anyone following this parse.
///
/// LINES, not the flattened string, because bash counted `raw_routes` with `grep -cF '.route('`
/// BEFORE the `tr` — per line. Count after flattening and the answer is 1 for any input, which
/// `n_routes < raw_routes` can never trip: the guard would print, say nothing and pass forever.
pub(super) fn routes_fn_lines(table_src: &str) -> (Vec<&str>, usize) {
    let mut out = Vec::new();
    let mut declared = 0usize;
    let mut in_range = false;
    for line in table_src.lines() {
        if !in_range {
            if line.starts_with(ROUTES_FN) {
                in_range = true;
                declared += 1;
            } else {
                continue;
            }
        } else if line.starts_with('}') {
            in_range = false;
        }
        out.push(line.split_once("//").map_or(line, |(head, _)| head));
    }
    (out, declared)
}

/// `… | tr '\n' ' '`. A trailing space after EVERY line, the last one included, because sed emitted
/// a newline after each and `tr` rewrote all of them.
pub(super) fn flatten(lines: &[&str]) -> String {
    lines.iter().map(|l| format!("{l} ")).collect()
}

/// One `METHOD PATH FN` row per registration, or an `UNPARSED …` marker.
///
/// The flattened body is split on the literal `.route(`; each piece holds exactly one registration,
/// whose path is its first quoted string and whose method/handler pairs are every
/// `method(path::to::fn` in it. That is what makes chained `get(a).post(b)` and
/// `axum::routing::patch(a).delete(b)` both fall out correctly, and splitting on `.route(` cannot
/// catch `.route_layer(` (the next character is `_`, not `(`). A piece yielding no method, or no
/// path, is EMITTED as a marker rather than dropped — the difference between this and a check that
/// silently shrinks.
pub(super) fn extract_router(body: &str) -> Vec<String> {
    let quoted = Regex::new(r#""[^"]*""#).expect("static regex");
    // Verbatim from the awk, `[ ]*` included: literal spaces only, the body having no newlines now.
    let meth = Regex::new(r"(get|post|put|patch|delete|head|options|trace)\([ ]*[A-Za-z_:0-9]+")
        .expect("static regex");
    let mut out = Vec::new();
    for (i, rec) in body.split(".route(").enumerate() {
        if i == 0 {
            continue; // everything before the first `.route(` is the fn signature
        }
        let nr = i + 1; // awk's NR over the same record split, preserved for the marker text
        let Some(q) = quoted.find(rec) else {
            out.push(format!("UNPARSED no-path-literal-in-registration-{nr}"));
            continue;
        };
        let path = &q.as_str()[1..q.as_str().len() - 1];
        let (mut s, mut n) = (rec, 0);
        while let Some(m) = meth.find(s) {
            let hit = m.as_str();
            s = &s[m.end()..];
            let (method, rest) = hit.split_once('(').expect("the regex matched a `(`");
            let f = rest.trim_matches(' ').rsplit("::").next().unwrap_or("");
            out.push(format!(
                "{} {API_PREFIX}{path} {f}",
                method.to_ascii_uppercase()
            ));
            n += 1;
        }
        if n == 0 {
            out.push(format!("UNPARSED no-method-handler-for-path-{path}"));
        }
    }
    out
}

/// Sweep `src/` for `@route` tags. Returns the parsed rows plus the RAW tag-line count the vacuity
/// guard compares for exact equality. A missing or unreadable tree is a `NotRun`, which closes
/// the script's `2>/dev/null || true`.
pub(super) fn extract_all_tags(repo_root: &Path) -> Result<(Vec<String>, usize), NotRun> {
    let src_dir = repo_root.join(SRC_DIR_REL);
    let files = scan::walk_files(&[&src_dir], scan::with_extension(&["rs"]))?;
    let tag_re = Regex::new(TAG_RE).expect("static regex");
    let (mut rows, mut raw) = (Vec::new(), 0usize);
    for path in files {
        let text = std::fs::read_to_string(&path).map_err(|source| NotRun::Unreadable {
            path: path.clone(),
            source,
        })?;
        raw += text.lines().filter(|l| tag_re.is_match(l)).count();
        // `grep -rl "$SRC_DIR"` printed paths relative to the root the script `cd`'d into.
        let rel = path.strip_prefix(repo_root).unwrap_or(&path);
        rows.extend(extract_tags(&rel.to_string_lossy(), &text));
    }
    Ok((rows, raw))
}

/// One `METHOD PATH FN FILE:LINE` row per tag, or an `ORPHAN …` marker.
///
/// A tag binds to the next `pub fn` / `pub async fn` below it. A tag with no handler under it is an
/// ORPHAN — emitted, never dropped, because that is a malformed claim and the vacuity guard's job
/// is to notice claims this parser could not read.
pub(super) fn extract_tags(file: &str, text: &str) -> Vec<String> {
    let tag_re = Regex::new(TAG_RE).expect("static regex");
    let tag_cut = Regex::new(&format!("{TAG_RE}+")).expect("static regex");
    let fn_re = Regex::new(FN_RE).expect("static regex");
    let fn_cut = Regex::new(&format!("{FN_RE}+")).expect("static regex");
    let ws = Regex::new("[[:space:]]+").expect("static regex");
    // `:id` -> `{:id}` -> `{id}`. The name is PRESERVED, so `:id` documented against a wired
    // `{mission_id}` still fails. awk's ERE, not grep's, so the ugrep brace hazard never applied.
    let param = Regex::new(r":[A-Za-z_][A-Za-z_0-9]*").expect("static regex");

    let mut out = Vec::new();
    let (mut pend, mut pm, mut pp, mut pline) = (false, String::new(), String::new(), 0usize);
    for (idx, line) in text.lines().enumerate() {
        let nr = idx + 1;
        if tag_re.is_match(line) {
            if pend {
                out.push(format!("ORPHAN {file}:{pline} {pm} {pp}"));
            }
            let rest = tag_cut.replace(line, "");
            // awk's `split("", a, re)` is 0, not 1. Inert here (both take the `n < 2` branch), but
            // copied rather than approximated.
            let f: Vec<&str> = if rest.is_empty() {
                Vec::new()
            } else {
                ws.split(&rest).collect()
            };
            pm = f.first().unwrap_or(&"").to_ascii_uppercase();
            pp = (*f.get(1).unwrap_or(&"")).to_string();
            if f.len() < 2 || pm.is_empty() || pp.is_empty() {
                out.push(format!("ORPHAN {file}:{nr} malformed-tag"));
                pend = false;
                continue;
            }
            pp = param.replace_all(&pp, "{${0}}").replace("{:", "{");
            (pend, pline) = (true, nr);
            continue; // awk's `next`: the pub-fn rule cannot also fire on this line
        }
        if pend && fn_re.is_match(line) {
            let l = fn_cut.replace(line, "");
            let name = &l[..l.find(['(', '<']).unwrap_or(l.len())];
            out.push(format!("{pm} {pp} {name} {file}:{pline}"));
            pend = false;
        }
    }
    if pend {
        out.push(format!("ORPHAN {file}:{pline} {pm} {pp}"));
    }
    out
}
