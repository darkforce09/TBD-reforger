use super::*;

pub fn verify_route_tags(repo_root: &Path) -> Result<u8> {
    let (code, out) = run(repo_root);
    for line in out {
        println!("{line}");
    }
    Ok(code)
}

/// Push a fixed block. `""` is a deliberate blank line, which `str::lines` would swallow.
pub(super) fn say(o: &mut Vec<String>, lines: &[&str]) {
    o.extend(lines.iter().map(|s| (*s).to_string()));
}

/// The gate proper, writing into a sink so the tests assert on exact bytes instead of scraping
/// stdout. Multi-line [`Verdict`] renderings go in as single elements; `println!` reproduces them.
pub(super) fn run(repo_root: &Path) -> (u8, Vec<String>) {
    let mut o: Vec<String> = Vec::new();

    // Probe the matcher over a subject whose answer is known, BEFORE it decides anything. bash:
    // `gate_probe_str -F "tbd" "tbd-reforger"` with `case` arms for 127 (tool absent) and 2
    // (pattern error) — neither reachable now, which is what compiling the engine in buys and what
    // T-620 shows it is worth (`verify-no-python` stayed green over `rg: command not found` for
    // four waves). The probe stays because "the matcher works" is still a claim, and `probe_str`
    // returning `Result` forces the dead arm to be written rather than assumed.
    match gate::probe_str(&Pattern::literal("tbd"), "tbd-reforger") {
        Ok(true) => {}
        Ok(false) => {
            say(&mut o, PROBE_FAIL);
            return (1, o);
        }
        Err(cause) => {
            o.push(Verdict::did_not_run("route-tag self-probe", Kind::Pin, cause).to_string());
            return (2, o);
        }
    }

    // ── Shape assertions on app.rs ───────────────────────────────────────────────────────────
    //
    // The extractor reads ONE function and prefixes ONE nest path; both are load-bearing, so both
    // are pinned. bash's `gate_require … "$APP_RS"` is a stat plus a content match, split here into
    // an explicit read plus `gate::require_str` for one reason: the script `cd`s to `$ROOT` and so
    // printed `apps/website/api/src/app.rs`, while xtask takes an absolute root and may be invoked
    // from any subdirectory. Reading first lets the missing-target `Finding` carry that same
    // relative path, with the same `Verdict` shapes.
    let nest = format!(".nest(\"{API_PREFIX}\", api_routes(");
    let pins: [(String, &str); 2] = [
        (
            format!(
                "app.rs no longer defines `fn api_routes` — the route extractor in {SELF_REL} reads that function by name, so it is now parsing nothing. Re-point it before trusting any verdict."
            ),
            "fn api_routes",
        ),
        (
            format!(
                "app.rs no longer nests api_routes at `{API_PREFIX}` — every @route tag in the crate is written with that prefix, so the extracted paths would all be wrong."
            ),
            nest.as_str(),
        ),
    ];
    let app_path = repo_root.join(APP_RS_REL);
    let app_src = match std::fs::read_to_string(&app_path)
        .ok()
        .filter(|_| app_path.is_file())
    {
        Some(text) => text,
        None => {
            // bash ran both `gate_require`s and both reported the same missing file, so both lines
            // print. Reproduced rather than collapsed: the second names the nest prefix, and a
            // reader who has lost app.rs still needs to know both invariants exist.
            for (msg, _) in &pins {
                let cause = NotRun::TargetMissing(PathBuf::from(APP_RS_REL));
                o.push(Verdict::did_not_run(msg.clone(), Kind::Pin, cause).to_string());
            }
            say(&mut o, &["", SHAPE_FAIL]);
            return (2, o);
        }
    };
    let mut shape_bad = false;
    for (msg, needle) in &pins {
        // `require_str` has no NotRun path — the subject is in hand — so the only failing arm is
        // `Failed`, which renders bash's bare `FAIL: $msg`.
        if let v @ (Verdict::Failed(_) | Verdict::DidNotRun(..)) =
            gate::require_str(msg, &Pattern::literal(needle), &app_src)
        {
            o.push(v.to_string());
            shape_bad = true;
        }
    }
    if shape_bad {
        say(&mut o, &["", SHAPE_FAIL]);
        return (1, o);
    }

    // ── Extract both sides ───────────────────────────────────────────────────────────────────
    let lines = api_routes_lines(&app_src);
    let mut router = extract_router(&flatten(&lines));
    // `grep -cF '.route('` counted LINES, not occurrences: a chained `get(a).post(b)` on one line
    // is one raw route but two registrations, which is why the guard below is `<` and not `!=`.
    let raw_routes = lines.iter().filter(|l| l.contains(".route(")).count();

    let (mut tags, raw_tags) = match extract_all_tags(repo_root) {
        Ok(pair) => pair,
        Err(cause) => {
            // The case `2>/dev/null || true` could not tell apart from "no tags exist".
            let msg = format!("the @route sweep could not read {SRC_DIR_REL}");
            o.push(Verdict::did_not_run(msg, Kind::Pin, cause).to_string());
            say(&mut o, &["", PARSE_FAIL]);
            return (2, o);
        }
    };
    router.sort_by(|a, b| collate_cmp(a, b));
    tags.sort_by(|a, b| collate_cmp(a, b));

    // ── Vacuity guards — BEFORE any verdict, because a verdict over an empty parse IS the defect ─
    let router_bad = marked(&router, "UNPARSED ").count();
    let tag_bad = marked(&tags, "ORPHAN ").count();
    let n_routes = router.len() - router_bad;
    let n_tags = tags.len() - tag_bad;
    let mut fail = false;

    if raw_tags == 0 || raw_routes == 0 {
        o.push(format!("FAIL: parsed NOTHING — {raw_tags} raw @route tag(s), {raw_routes} raw .route( registration(s)."));
        say(&mut o, NOTHING_TAIL);
        fail = true;
    }
    // Exact and self-scaling: no floor to go stale, and a tag the parser could not read is NAMED.
    if n_tags != raw_tags {
        o.push(format!("FAIL: {raw_tags} @route tag(s) in the tree but {n_tags} parsed into (METHOD, PATH, HANDLER)."));
        o.extend(marked(&tags, "ORPHAN ").map(|l| format!("      orphan: {l}")));
        say(&mut o, &[ORPHAN_TAIL]);
        fail = true;
    }
    if n_routes < raw_routes || router_bad != 0 {
        o.push(format!(
            "FAIL: {raw_routes} .route( registration(s) in api_routes but only {n_routes} parsed."
        ));
        o.extend(marked(&router, "UNPARSED ").map(|l| format!("      {l}")));
        say(&mut o, &[UNPARSED_TAIL]);
        fail = true;
    }

    // Sentinels: keyed, literal, present on BOTH sides — the last defence against a pipeline that
    // produced two lists which are wrong in the same direction. bash probed the extraction FILES
    // with `gate_probe_file -F`; the in-memory twin is the same literal substring test over the
    // same text, and its tool-failure arm went away with the subprocess.
    let (router_text, tags_text) = (joined(&router), joined(&tags));
    for s in SENTINELS {
        for (side, text) in [("router", &router_text), ("tags", &tags_text)] {
            if !probe(s, text) {
                o.push(format!("FAIL: sentinel absent from the {side} extraction: '{s}' — the parser lost a route that is known to be there."));
                fail = true;
            }
        }
    }
    if fail {
        say(&mut o, &["", PARSE_FAIL]);
        return (1, o);
    }

    // ── The cross-check ──────────────────────────────────────────────────────────────────────
    //
    // Keys are wrapped in `|` on both sides so a literal (substring) probe implies a whole-line
    // match: `|GET /api/v1/servers list_servers|` cannot be a substring of any other key. That is
    // what let the comparison stay literal — and it HAD to stay literal, because a path's `{id}` is
    // an invalid ERE repeat under ugrep. bash's `sort -u` here was dedupe only; membership does not
    // care about order, so one `BTreeSet` covers both.
    let router_key = key_text(&router, "UNPARSED ");
    let tags_key = key_text(&tags, "ORPHAN ");

    let mut a_bad = 0usize;
    o.push(format!(
        "── A. @route tags with no matching route in {APP_RS_REL} ──"
    ));
    for line in &tags {
        // bash `read -r m p fn loc`: three fields plus "the rest" as the location.
        let (m, p, f, loc) = read4(line);
        if m == "ORPHAN" || probe(&format!("|{m} {p} {f}|"), &router_key) {
            continue;
        }
        o.push(format!("  {loc}"));
        o.push(format!("      @route {m} {p}  ->  handler `{f}` is NOT registered in {APP_RS_REL} on that method+path."));
        a_bad += 1;
    }
    if a_bad == 0 {
        o.push(format!(
            "  none — all {n_tags} tag(s) resolve to a registered route."
        ));
    }

    let mut b_bad = 0usize;
    o.push("── B. registered routes with no matching @route tag ──".into());
    for line in &router {
        let (m, p, f, _) = read4(line);
        if m == "UNPARSED" || probe(&format!("|{m} {p} {f}|"), &tags_key) {
            continue;
        }
        o.push(format!("  {m} {p}"));
        o.push(format!(
            "      registered to `{f}`, which carries no matching @route tag (GO-7 requires one)."
        ));
        b_bad += 1;
    }
    if b_bad == 0 {
        o.push(format!(
            "  none — all {n_routes} registered route(s) are documented."
        ));
    }

    o.push(String::new());
    o.push(format!(
        "checked {n_tags} @route tag(s) against {n_routes} registered route(s) in {APP_RS_REL}"
    ));
    if a_bad != 0 || b_bad != 0 {
        o.push(format!(
            "ROUTE-TAG CHECK: FAIL — {a_bad} unwired tag(s), {b_bad} undocumented route(s)"
        ));
        say(&mut o, VERDICT_TAIL);
        return (1, o);
    }
    o.push("ROUTE-TAG CHECK: PASS".into());
    (0, o)
}

/// `sed -n '/^fn api_routes/,/^}/p' | sed 's://.*$::'` — the comment-stripped range, still LINES.
///
/// The crate is rustfmt-clean, so the next column-0 `}` is the function's own closing brace; sed
/// restarts the range afterwards, so a second `fn api_routes…` would also be taken — kept, because
/// dropping it would be a silent narrowing. Comments go so a commented-out `.route(...)` cannot
/// read as live; the strip is naive and would also cut a `//` inside a string literal, a hazard the
/// script carried and this port keeps (measured 2026-08-12, `grep -n '"[^"]*//'` over the range
/// finds nothing — a URL literal landing there later would drop the rest of its line).
///
/// LINES, not the flattened string, because bash counted `raw_routes` with `grep -cF '.route('`
/// BEFORE the `tr` — per line. Count after flattening and the answer is 1 for any input, which
/// `n_routes < raw_routes` can never trip: the guard would print, say nothing and pass forever.
/// A unit test caught exactly that, which is the whole argument for having them.
pub(super) fn api_routes_lines(app_src: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut in_range = false;
    for line in app_src.lines() {
        if !in_range {
            if line.starts_with("fn api_routes") {
                in_range = true;
            } else {
                continue;
            }
        } else if line.starts_with('}') {
            in_range = false;
        }
        out.push(line.split_once("//").map_or(line, |(head, _)| head));
    }
    out
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

/// `gate_probe_file -F "$key" "$file"` — a literal substring test. Infallible here: the subject is
/// already in memory and the engine is compiled in, which is the `NotRun` arm bash needed.
pub(super) fn probe(needle: &str, haystack: &str) -> bool {
    gate::probe_str(&Pattern::literal(needle), haystack).unwrap_or(false)
}

/// `grep '^ORPHAN ' file` / `grep '^UNPARSED ' file`.
pub(super) fn marked<'a>(rows: &'a [String], tag: &'a str) -> impl Iterator<Item = &'a String> {
    rows.iter().filter(move |l| l.starts_with(tag))
}

pub(super) fn joined(rows: &[String]) -> String {
    let mut s = rows.join("\n");
    s.push('\n');
    s
}

/// `awk '!/^SKIP/{print "|" $1 " " $2 " " $3 "|"}' | sort -u`
pub(super) fn key_text(rows: &[String], skip: &str) -> String {
    let keys: BTreeSet<String> = rows
        .iter()
        .filter(|l| !l.starts_with(skip))
        .map(|l| {
            let (m, p, f, _) = read4(l);
            format!("|{m} {p} {f}|")
        })
        .collect();
    joined(&keys.into_iter().collect::<Vec<_>>())
}

/// bash `read -r a b c rest`: split on whitespace runs, the last variable taking the remainder.
/// Every row this gate builds has a single-token remainder, so returning that token is exact.
pub(super) fn read4(line: &str) -> (&str, &str, &str, &str) {
    let mut i = line.split_whitespace();
    let (a, b, c, d) = (i.next(), i.next(), i.next(), i.next());
    (
        a.unwrap_or(""),
        b.unwrap_or(""),
        c.unwrap_or(""),
        d.unwrap_or(""),
    )
}
