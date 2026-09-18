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

    // ── Shape assertions on http_router.rs ───────────────────────────────────────────────────
    //
    // The extractor reads the MERGE function by name and prefixes ONE nest path; both are
    // load-bearing, so both are pinned. bash's `gate_require … "$APP_RS"` is a stat plus a content
    // match, split here into an explicit read plus `gate::require_str` for one reason: the script
    // `cd`s to `$ROOT` and so printed `apps/website/api_v2/src/core/http_router.rs`, while xtask
    // takes an absolute root and may be invoked from any subdirectory. Reading first lets the
    // missing-target `Finding` carry that same relative path, with the same `Verdict` shapes.
    let nest = format!(".nest(\"{API_PREFIX}\", api_v1_routes(");
    let pins: [(String, &str); 2] = [
        (
            format!(
                "http_router.rs no longer defines `{MERGE_FN}` — the mount cross-check in {SELF_REL} reads that function by name to learn which domain route tables are actually served, so it is now reading nothing. Re-point it before trusting any verdict."
            ),
            MERGE_FN,
        ),
        (
            format!(
                "http_router.rs no longer nests {MERGE_FN_NAME} at `{API_PREFIX}` — every @route tag in the crate is written with that prefix, so the extracted paths would all be wrong."
            ),
            nest.as_str(),
        ),
    ];
    let router_path = repo_root.join(ROUTER_RS_REL);
    let router_src = match std::fs::read_to_string(&router_path)
        .ok()
        .filter(|_| router_path.is_file())
    {
        Some(text) => text,
        None => {
            // bash ran both `gate_require`s and both reported the same missing file, so both lines
            // print. Reproduced rather than collapsed: the second names the nest prefix, and a
            // reader who has lost http_router.rs still needs to know both invariants exist.
            for (msg, _) in &pins {
                let cause = NotRun::TargetMissing(PathBuf::from(ROUTER_RS_REL));
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
            gate::require_str(msg, &Pattern::literal(needle), &router_src)
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
    //
    // The router side is the UNION of every `src/<domain>/routes.rs` in the tree, not one
    // function: each table is parsed on its own so a `.route(` in one cannot bleed into another
    // through the flattening.
    let tables = match discover_route_files(repo_root) {
        Ok(files) => files,
        Err(cause) => {
            let msg =
                format!("the domain route tables could not be discovered under {SRC_DIR_REL}");
            o.push(Verdict::did_not_run(msg, Kind::Pin, cause).to_string());
            say(&mut o, &["", PARSE_FAIL]);
            return (2, o);
        }
    };
    let mut router: Vec<String> = Vec::new();
    // `grep -cF '.route('` counted LINES, not occurrences: a chained `get(a).post(b)` on one line
    // is one raw route but two registrations, which is why the guard below is `<` and not `!=`.
    let mut raw_routes = 0usize;
    for table in &tables {
        let (lines, declared) = routes_fn_lines(&table.text);
        if declared != 1 {
            // A file that does not hold exactly one column-0 `pub fn routes` is a shape this
            // extractor cannot read, and must be NAMED rather than contribute nothing in silence.
            router.push(format!(
                "UNPARSED {}-declares-{declared}-column-0-pub-fn-routes",
                table.rel
            ));
            continue;
        }
        raw_routes += lines.iter().filter(|l| l.contains(".route(")).count();
        router.extend(extract_router(&flatten(&lines)));
    }

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
    // The mount cross-check, both directions. A table nobody merges still feeds side B, so its
    // routes would be demanded of the tag sweep while serving no traffic; a merge with no table
    // behind it means the router side shrank without the count moving. Neither is visible to the
    // counting guards above, because both sides stay internally consistent.
    let merged = merged_domains(&router_src);
    let discovered: BTreeSet<String> = tables.iter().map(|t| t.domain.clone()).collect();
    for domain in discovered.difference(&merged) {
        o.push(format!(
            "FAIL: route file src/{domain}/{ROUTES_FILE} is not merged by {MERGE_FN_NAME}"
        ));
        fail = true;
    }
    for domain in merged.difference(&discovered) {
        o.push(format!(
            "FAIL: {MERGE_FN_NAME} merges {domain}::routes but src/{domain}/{ROUTES_FILE} does not exist"
        ));
        fail = true;
    }
    if discovered != merged {
        say(&mut o, &[MOUNT_TAIL]);
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
            "FAIL: {raw_routes} .route( registration(s) in {ROUTE_TABLES} but only {n_routes} parsed."
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
        "── A. @route tags with no matching route in {ROUTE_TABLES} ──"
    ));
    for line in &tags {
        // bash `read -r m p fn loc`: three fields plus "the rest" as the location.
        let (m, p, f, loc) = read4(line);
        if m == "ORPHAN" || probe(&format!("|{m} {p} {f}|"), &router_key) {
            continue;
        }
        o.push(format!("  {loc}"));
        o.push(format!("      @route {m} {p}  ->  handler `{f}` is NOT registered in {ROUTE_TABLES} on that method+path."));
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
    let n_files = tables.len();
    o.push(format!(
        "checked {n_tags} @route tag(s) against {n_routes} registered route(s) in {n_files} route file(s) under {SRC_DIR_REL}"
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
