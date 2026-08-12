//! GO-7 — every `@route` tag resolves to a registered Axum route, and every route back to a tag
//! (T-853 port of `scripts/verify-route-tags.sh`, the check restored at T-586/T-590).
//!
//! ── WHY A SCRIPT HAD TO EXIST AT ALL ─────────────────────────────────────────────────────────
//!
//! CODING_STANDARDS.md GO-7: *"Every exported handler func SHALL carry `@route` in its Godoc, and
//! the tag MUST match the wired route in `handlers.go` `Register()` (method + path)."* §2 of that
//! document classes GO-7 **CI-SCRIPT** rather than lint-enforced, and that is the whole reason:
//! clippy does not read doc comments and `cargo fmt` only reflows them, so nothing in the compiler
//! can see a comment claiming a route. The Go gate was `verify-contract-citations.mjs` — presence
//! AND route-match across all 82 Go handlers. T-145's Go→Rust rewrite deleted `Register()` and
//! every Go handler, GO-7 died with them, and nothing replaced it: `Makefile:304` still claimed the
//! GO-2..9 analogs were "enforced by clippy + the centralized ApiError type + `cargo fmt`", none of
//! which can see a doc comment.
//!
//! MEASURED CONSEQUENCE (T-586, found by T-576): `handlers/servers.rs` carried `@route` tags on
//! THREE handlers — `create_server` (POST), `update_server` (PATCH), `deactivate_server` (DELETE) —
//! that `app.rs` never registered. The whole admin server-CRUD triple was documented, tested and
//! unreachable, and nothing went red. In the other direction `submit_mission` was a live registered
//! route carrying no tag at all. A documentation tag nobody checks is a claim, not a contract.
//!
//! ── THE TWO DIRECTIONS, BOTH HARD FAILURES ───────────────────────────────────────────────────
//!
//!   A. TAG → ROUTER  every `@route METHOD PATH` must be registered, on that method, for that
//!      handler — the T-586 triple: a claim to a door that is not in the wall.
//!   B. ROUTER → TAG  every registered route must carry a matching `@route` on the handler it
//!      names. GO-7's "presence" half, and not optional: DOCUMENTATION_STANDARDS.md §3.1 makes
//!      `@route` REQUIRED on the serving handler, one leg of the three-way triangulation a mod
//!      author greps.
//!
//! Both keys are (METHOD, PATH, HANDLER FN), not just the path — which is what makes a tag moved
//! onto the wrong handler, or a handler rewired elsewhere, fail as loudly as one never wired.
//!
//! ── VACUITY GUARDS (T-556: a gate reporting nothing == a gate checking nothing) ───────────────
//!
//! A verifier that passes because it parsed zero inputs is the T-586 defect in a new hat, so the
//! parse is checked against itself before any verdict is issued: every raw `@route` line must
//! become exactly one parsed tuple; every `.route(` line must yield at least one registration;
//! `app.rs` must still have the shape the extractor parses; and two sentinel routes present on both
//! sides must survive the pipeline. Each is a FAIL, never a SKIP.
//!
//! ── WHAT THE PORT FIXES ──────────────────────────────────────────────────────────────────────
//!
//! 1. **Exit 127 is unreachable for the matcher.** The script's header warns at length about search
//!    tools that answer differently depending on WHO invoked them: `rg` is installed nowhere here
//!    and exists in an agent shell only as an injected function (T-556), and one layer down `grep`
//!    is *ugrep 7.5.0* as an agent-shell function but GNU grep 3.8 under `bash script.sh` (measured
//!    2026-07-31, T-586). ugrep rejects an unescaped `{` in an ERE ("invalid repeat", exit 2) where
//!    GNU grep takes it literally — and **every route path here contains `{id}`**. bash survived by
//!    routing every route-shaped comparison through `-F`. Here the engine is the `regex` crate
//!    compiled in: no `PATH`, no shell function, no skew. [`Pattern::literal`] is kept wherever
//!    bash wrote `-F`, so the mapping stays reviewable 1:1.
//! 2. **`2>/dev/null || true` on the tag sweep is closed.** It turned "the handlers tree moved"
//!    into an empty file list. The script did not go green on that — its own vacuity guard caught
//!    the zero — but it then reported *"parsed NOTHING"* when the truth was *"I could not read the
//!    tree"*, which sends the reader to the wrong file. [`scan::walk_files`] makes it a [`NotRun`].
//! 3. **Deterministic ordering.** MEASURED 2026-08-12: the script's `sort`s run under the ambient
//!    locale and no caller pins one (nothing matches `LC_ALL|LC_COLLATE` in `Makefile`,
//!    `scripts/platform/wave.sh` or `.github/workflows/`). Under `LANG=en_AU.UTF-8` glibc ignores
//!    punctuation at the primary level, so `DELETE …/{id}/bookmark` lists BEFORE `DELETE …/{id}`;
//!    under `LANG=C` it lists after — so the report order depended on the operator's environment,
//!    rule 1 of `.cursor/rules/acceptance-gates-reproducible.mdc`, in the one script whose header is
//!    a sermon against exactly that. [`collate_cmp`] reproduces the measured `en_AU.UTF-8` order
//!    with no locale input at all: the same bytes on every machine, and the same bytes the
//!    committed baseline was captured with.
//!
//! ── DELIBERATE DEVIATIONS (everything else is byte-for-byte) ─────────────────────────────────
//!
//! * **`$0` in the two shape-pin messages** becomes [`SELF_REL`]. That sentence tells the reader
//!   where the extractor they must re-point lives, and after T-853 that is this file; naming a
//!   script the migration removes would be actively misleading. Reachable only once `fn api_routes`
//!   has been renamed — never on a clean tree.
//! * **Exit 2, not 1, when the check DID NOT RUN** (missing/unreadable `app.rs` or `src/`), as in
//!   `sql_gates.rs` and `gate_t439.rs`. `Makefile:328` and `wave.sh:2562`/`:2833` test `rc -eq 0`,
//!   so any nonzero is still FAIL there, and the bash headline stays verbatim on line 1 so a grep
//!   for `FAIL: app.rs no longer …` still hits. A real A/B violation still exits **1**.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use tbd_gate::{Kind, NotRun, Pattern, Verdict, gate, scan};

/// The router. Relative, because the script `cd`s to `$ROOT` and printed relative paths.
const APP_RS_REL: &str = "apps/website/api/src/app.rs";
/// The tree swept for `@route` tags — the whole `src/`, not just `handlers/`.
const SRC_DIR_REL: &str = "apps/website/api/src";
/// The nest prefix every `@route` tag is written against. Asserted, never assumed: if `app.rs`
/// stops nesting `api_routes` here, every extracted path is silently wrong.
const API_PREFIX: &str = "/api/v1";
/// bash interpolated `$0`. See the module docs on the one deliberate text deviation.
const SELF_REL: &str = "xtask/src/gate_route_tags.rs";
/// Two routes registered AND tagged today, one with a path parameter and one without. If the
/// extractor breaks in a way the counting guards miss, these vanish and the run fails rather than
/// quietly comparing two short lists that happen to agree.
const SENTINELS: &[&str] = &[
    "GET /api/v1/servers list_servers",
    "GET /api/v1/servers/{id}/status get_server_status",
];

// Fixed output blocks, as consts because rustfmt cannot break a string literal — and every byte
// here is contract: `wave.sh` scrapes these logs and T-853 accepts ports by diffing stdout.
const SHAPE_FAIL: &str = "ROUTE-TAG CHECK: FAIL (router shape changed — the extractor was not run)";
const PARSE_FAIL: &str =
    "ROUTE-TAG CHECK: FAIL (the parse could not be trusted — no tag/route verdict was issued)";
const PROBE_FAIL: &[&str] = &[
    "FAIL: grep self-probe returned 1 over a subject it must match.",
    "      The search engine is broken or missing. A check that cannot run is not a pass.",
];
const NOTHING_TAIL: &[&str] = &[
    "      A route-tag check with no inputs is not a pass. Either the crate moved or this",
    "      script's extractor is broken; both are red.",
];
const ORPHAN_TAIL: &str =
    "      A tag with no handler beneath it, or a malformed tag, is an unreadable claim.";
const UNPARSED_TAIL: &str =
    "      A registration shape this extractor cannot read must not be silently skipped.";
const VERDICT_TAIL: &[&str] = &[
    "  A @route tag is a contract with the router, not a comment. Wire the route, move the",
    "  tag onto the handler that really serves it, or delete the claim.",
];

pub fn verify_route_tags(repo_root: &Path) -> Result<u8> {
    let (code, out) = run(repo_root);
    for line in out {
        println!("{line}");
    }
    Ok(code)
}

/// Push a fixed block. `""` is a deliberate blank line, which `str::lines` would swallow.
fn say(o: &mut Vec<String>, lines: &[&str]) {
    o.extend(lines.iter().map(|s| (*s).to_string()));
}

/// The gate proper, writing into a sink so the tests assert on exact bytes instead of scraping
/// stdout. Multi-line [`Verdict`] renderings go in as single elements; `println!` reproduces them.
fn run(repo_root: &Path) -> (u8, Vec<String>) {
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

// ── Extraction ───────────────────────────────────────────────────────────────────────────────

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
fn api_routes_lines(app_src: &str) -> Vec<&str> {
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
fn flatten(lines: &[&str]) -> String {
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
fn extract_router(body: &str) -> Vec<String> {
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

/// Both patterns are anchored at column 0, exactly as the awk wrote them: an indented `/// @route`
/// inside an `impl` block is invisible to this gate. Measured 2026-08-12 there are none
/// (`grep -rhE '^[[:space:]]+///[[:space:]]*@route'` → 0), so the anchor costs nothing today and is
/// preserved rather than widened, because widening it is a behaviour change, not a port.
const TAG_RE: &str = r"^///[[:space:]]*@route[[:space:]]";
const FN_RE: &str = r"^pub[[:space:]]+(async[[:space:]]+)?fn[[:space:]]";

/// Sweep `src/` for `@route` tags. Returns the parsed rows plus the RAW tag-line count the vacuity
/// guard compares for exact equality. A missing or unreadable tree is a [`NotRun`], which closes
/// the script's `2>/dev/null || true`.
fn extract_all_tags(repo_root: &Path) -> Result<(Vec<String>, usize), NotRun> {
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
fn extract_tags(file: &str, text: &str) -> Vec<String> {
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

// ── Small helpers, one shell construct each ──────────────────────────────────────────────────

/// `gate_probe_file -F "$key" "$file"` — a literal substring test. Infallible here: the subject is
/// already in memory and the engine is compiled in, which is the `NotRun` arm bash needed.
fn probe(needle: &str, haystack: &str) -> bool {
    gate::probe_str(&Pattern::literal(needle), haystack).unwrap_or(false)
}

/// `grep '^ORPHAN ' file` / `grep '^UNPARSED ' file`.
fn marked<'a>(rows: &'a [String], tag: &'a str) -> impl Iterator<Item = &'a String> {
    rows.iter().filter(move |l| l.starts_with(tag))
}

fn joined(rows: &[String]) -> String {
    let mut s = rows.join("\n");
    s.push('\n');
    s
}

/// `awk '!/^SKIP/{print "|" $1 " " $2 " " $3 "|"}' | sort -u`
fn key_text(rows: &[String], skip: &str) -> String {
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
fn read4(line: &str) -> (&str, &str, &str, &str) {
    let mut i = line.split_whitespace();
    let (a, b, c, d) = (i.next(), i.next(), i.next(), i.next());
    (
        a.unwrap_or(""),
        b.unwrap_or(""),
        c.unwrap_or(""),
        d.unwrap_or(""),
    )
}

/// glibc `en_AU.UTF-8` `strcoll`, over the ASCII these extractions contain.
///
/// MEASURED 2026-08-12 against `sort` on this host, which is how the committed baseline was
/// captured. Four levels, as glibc implements ISO 14651:
///
/// * **L1** alphanumerics only, case-folded — punctuation and space are *ignorable*. This is why
///   `aab` < `a b`: primary `aab` against primary `ab`.
/// * **L2** accents, constant across ASCII, so absent here.
/// * **L3** case per surviving character, lowercase first (`ab` < `aB` < `Ab` < `AB`).
/// * **L4** the ignored characters as (position, code point), compared **position first** —
///   `a}bc` < `ab c` even though `}` > space. glibc declares this level `position`, so a string
///   that runs out of ignorables sorts **last**: `a b c` < `a bc` < `ab c` < `abc`.
///
/// The trailing byte compare is GNU `sort`'s last-resort `strcmp`. Unreachable for distinct inputs
/// — L1+L3+L4 together reconstruct the string — and kept so a tie cannot become nondeterminism.
fn collate_cmp(a: &str, b: &str) -> Ordering {
    fn primary(s: &str) -> Vec<u8> {
        let keep = s.bytes().filter(u8::is_ascii_alphanumeric);
        keep.map(|c| c.to_ascii_lowercase()).collect()
    }
    fn case(s: &str) -> Vec<u8> {
        let keep = s.bytes().filter(u8::is_ascii_alphanumeric);
        keep.map(|c| u8::from(c.is_ascii_uppercase())).collect()
    }
    fn ignorable(s: &str) -> Vec<(usize, u8)> {
        let all = s.bytes().enumerate();
        all.filter(|(_, c)| !c.is_ascii_alphanumeric()).collect()
    }
    let ord = primary(a)
        .cmp(&primary(b))
        .then_with(|| case(a).cmp(&case(b)));
    if ord != Ordering::Equal {
        return ord;
    }
    let (ia, ib) = (ignorable(a), ignorable(b));
    let (mut x, mut y) = (ia.iter(), ib.iter());
    loop {
        match (x.next(), y.next()) {
            (None, None) => return a.as_bytes().cmp(b.as_bytes()),
            // Exhausted sorts LAST — the `position` level, not a prefix comparison.
            (None, Some(_)) => return Ordering::Greater,
            (Some(_), None) => return Ordering::Less,
            (Some(p), Some(q)) if p != q => return p.cmp(q),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three registrations over two `.route(` lines, all three tagged. The commented-out
    /// registration and the `.route(` outside `api_routes` must both stay invisible.
    const APP: &str = r#"
fn api_routes(dev: bool) -> Router<AppState> {
    let mut r = Router::new()
        // .route("/commented-out", get(handlers::x::ghost))
        .route("/servers", get(handlers::servers::list_servers))
        .route(
            "/servers/{id}/status",
            get(handlers::servers::get_server_status).post(handlers::servers::set_status),
        );
    r
}
fn other() -> Router {
    Router::new().route("/outside", get(handlers::x::outside))
}
        .nest("/api/v1", api_routes(dev))
"#;
    const TAGS: &str = r#"
/// @route GET /api/v1/servers
pub async fn list_servers() {}

/// @route GET /api/v1/servers/:id/status
pub async fn get_server_status() {}

/// @route POST /api/v1/servers/:id/status
pub async fn set_status() {}
"#;

    struct Repo(PathBuf);
    impl Repo {
        fn new(name: &str) -> Repo {
            let mut p = std::env::temp_dir();
            p.push(format!("tbd-rt-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(p.join("apps/website/api/src/handlers")).unwrap();
            let r = Repo(p);
            r.app(APP);
            r.tags(TAGS);
            r
        }
        fn app(&self, body: &str) {
            std::fs::write(self.0.join("apps/website/api/src/app.rs"), body).unwrap();
        }
        fn tags(&self, body: &str) {
            let p = self.0.join("apps/website/api/src/handlers/servers.rs");
            std::fs::write(p, body).unwrap();
        }
        /// Run; assert the exit code and every expected line; hand back the joined output.
        fn expect(&self, code: u8, want: &[&str]) -> String {
            let (got, out) = super::run(&self.0);
            let all = out.join("\n");
            assert_eq!(got, code, "{all}");
            for w in want {
                assert!(all.contains(w), "missing {w:?} in:\n{all}");
            }
            all
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn clean_tree_passes_and_counts_exactly() {
        // The counts ARE the anti-vacuity assertion: three registrations, three tags, no drift.
        let all = Repo::new("clean").expect(0, &[
            "checked 3 @route tag(s) against 3 registered route(s) in apps/website/api/src/app.rs",
            "  none — all 3 tag(s) resolve to a registered route.",
            "  none — all 3 registered route(s) are documented.",
            "ROUTE-TAG CHECK: PASS",
        ]);
        // A commented-out registration, and one outside `api_routes`, are both invisible.
        assert!(!all.contains("ghost") && !all.contains("outside"), "{all}");
    }

    /// DIRECTION A — a tag naming a route that is not registered (the T-586 servers triple).
    #[test]
    fn a_tag_pointing_at_no_route_fails() {
        let r = Repo::new("dir-a");
        let extra = "/// @route DELETE /api/v1/servers/:id\npub async fn deactivate_server() {}\n";
        r.tags(&format!("{TAGS}\n{extra}"));
        r.expect(1, &[
            "  apps/website/api/src/handlers/servers.rs:11",
            "      @route DELETE /api/v1/servers/{id}  ->  handler `deactivate_server` is NOT registered in apps/website/api/src/app.rs on that method+path.",
            "checked 4 @route tag(s) against 3 registered route(s)",
            "ROUTE-TAG CHECK: FAIL — 1 unwired tag(s), 0 undocumented route(s)",
        ]);
    }

    /// DIRECTION B — a registered route whose handler carries no tag (the T-586 `submit_mission`).
    ///
    /// `set_status` is the target because the other two handlers ARE the sentinels: untag one of
    /// those and the sentinel guard fires first, so the cross-check is never reached. That is the
    /// guard working, but it makes those two useless for exercising direction B.
    #[test]
    fn a_route_with_no_tag_fails() {
        let r = Repo::new("dir-b");
        r.tags(&TAGS.replace("/// @route POST /api/v1/servers/:id/status\n", ""));
        r.expect(1, &[
            "  POST /api/v1/servers/{id}/status",
            "      registered to `set_status`, which carries no matching @route tag (GO-7 requires one).",
            "checked 2 @route tag(s) against 3 registered route(s)",
            "ROUTE-TAG CHECK: FAIL — 0 unwired tag(s), 1 undocumented route(s)",
        ]);
        // A tag on the right path but the WRONG method is A *and* B at once — the reason the key
        // is (METHOD, PATH, FN) and not the path alone.
        r.tags(&TAGS.replace(
            "@route POST /api/v1/servers/:",
            "@route PUT /api/v1/servers/:",
        ));
        r.expect(
            1,
            &["ROUTE-TAG CHECK: FAIL — 1 unwired tag(s), 1 undocumented route(s)"],
        );
    }

    /// THE ANTI-VACUITY CASE. A missing input must never read as "0 tags, 0 routes, all agree".
    #[test]
    fn inputs_that_were_never_read_do_not_pass() {
        let (code, out) = super::run(Path::new("/nonexistent/tbd-route-tags/repo"));
        let all = out.join("\n");
        assert_eq!(code, 2, "a check that never ran must not exit 0:\n{all}");
        assert!(
            all.contains("target file missing: apps/website/api/src/app.rs"),
            "{all}"
        );
        assert!(
            all.contains("The pin could not run.") && !all.contains("PASS"),
            "{all}"
        );
        // `src/` present but empty of tags: the zero-input vacuity guard, still a hard FAIL.
        let r = Repo::new("no-tags");
        r.tags("// no tags here\n");
        r.expect(
            1,
            &[
                "FAIL: parsed NOTHING — 0 raw @route tag(s), 2 raw .route( registration(s).",
                NOTHING_TAIL[0],
                PARSE_FAIL,
            ],
        );
    }

    #[test]
    fn a_broken_router_shape_is_a_failure_not_a_pass() {
        let r = Repo::new("shape");
        r.app(&APP.replace("fn api_routes", "fn v1_routes"));
        r.expect(
            1,
            &[
                "app.rs no longer defines `fn api_routes`",
                SELF_REL,
                SHAPE_FAIL,
            ],
        );
        r.app(&APP.replace(".nest(\"/api/v1\"", ".nest(\"/api/v2\""));
        r.expect(
            1,
            &["app.rs no longer nests api_routes at `/api/v1`", SHAPE_FAIL],
        );
    }

    #[test]
    fn an_unreadable_parse_is_named_not_skipped() {
        // A tag with no handler beneath it: the counts diverge and the orphan is printed by name.
        let r = Repo::new("orphan");
        r.tags(&format!("{TAGS}\n/// @route GET /api/v1/orphaned-claim\n"));
        r.expect(1, &[
            "FAIL: 4 @route tag(s) in the tree but 3 parsed into (METHOD, PATH, HANDLER).",
            "      orphan: ORPHAN apps/website/api/src/handlers/servers.rs:11 GET /api/v1/orphaned-claim",
            ORPHAN_TAIL,
            PARSE_FAIL,
        ]);
        // Renaming a sentinel handler on BOTH sides keeps the counts agreeing (3 == 3) and both
        // directions cross-checking clean, so only the sentinel can catch it.
        let s = Repo::new("sentinel");
        s.app(&APP.replace("list_servers", "index_servers"));
        s.tags(&TAGS.replace("list_servers", "index_servers"));
        s.expect(1, &[
            "FAIL: sentinel absent from the router extraction: 'GET /api/v1/servers list_servers' — the parser lost a route that is known to be there.",
            "FAIL: sentinel absent from the tags extraction: 'GET /api/v1/servers list_servers'",
        ]);
    }

    #[test]
    fn the_router_extractor_chains_and_marks() {
        let rows = |src: &str| extract_router(&flatten(&api_routes_lines(src)));
        assert_eq!(
            rows(APP),
            [
                "GET /api/v1/servers list_servers",
                "GET /api/v1/servers/{id}/status get_server_status",
                "POST /api/v1/servers/{id}/status set_status",
            ]
        );
        // No quoted path, and a path with no method — both must surface, neither may be dropped.
        let src = "fn api_routes() {\n  Router::new().route(NO_PATH, get(h::a)).route(\"/x\", z(q));\n}\n";
        assert_eq!(
            rows(src),
            [
                "UNPARSED no-path-literal-in-registration-2",
                "UNPARSED no-method-handler-for-path-/x",
            ]
        );
        // THE COUNT THE GUARD USES. bash grepped `.route(` BEFORE flattening, so it is per-LINE:
        // two lines here (the commented-out one is stripped, the `/outside` one is out of range)
        // against three parsed registrations. Counted after `flatten` it is 1 for ANY input, and
        // `n_routes < 1` can never trip — the guard would print, say nothing, and pass forever.
        let raw = api_routes_lines(APP)
            .iter()
            .filter(|l| l.contains(".route("))
            .count();
        assert_eq!(raw, 2);
    }

    #[test]
    fn tag_parsing_edge_cases() {
        // `:id` normalises but keeps its NAME, so `:id` against a wired `{mission_id}` still fails.
        let t = "/// @route GET /api/v1/m/:mission_id/v/:id\npub async fn g() {}\n";
        assert_eq!(
            extract_tags("f.rs", t),
            ["GET /api/v1/m/{mission_id}/v/{id} g f.rs:1"]
        );
        // A tag with no path is malformed, and is named as such rather than silently dropped.
        let m = extract_tags("f.rs", "/// @route GET\npub async fn g() {}\n");
        assert_eq!(m, ["ORPHAN f.rs:1 malformed-tag"]);
        // Documented bash behaviour: an indented tag is invisible. Widening it is a change.
        let i = extract_tags("f.rs", "    /// @route GET /api/v1/x\n    pub fn g() {}\n");
        assert!(i.is_empty(), "{i:?}");
    }

    /// The collation, pinned against the `sort` output measured under `LANG=en_AU.UTF-8`.
    #[test]
    fn collation_reproduces_measured_glibc_order() {
        let sorted = |mut v: Vec<&'static str>| {
            v.sort_by(|a, b| collate_cmp(a, b));
            v.join(",")
        };
        // L1 ignores punctuation, so `aab` beats every `ab`-primary string; among those, L4 orders
        // by (position, code point) and "no ignorables left" sorts LAST.
        let punct = vec![
            "ab", "a b", "a/b", "a{b", "a}b", "a-b", "a_b", "a:b", "a.b", "aab", "a0b",
        ];
        assert_eq!(sorted(punct), "a0b,aab,a b,a-b,a.b,a/b,a:b,a_b,a{b,a}b,ab");
        assert_eq!(
            sorted(vec!["abc", "ab c", "a bc", "a b c"]),
            "a b c,a bc,ab c,abc"
        );
        assert_eq!(
            sorted(vec!["a}bc", "ab c"]),
            "a}bc,ab c",
            "L4 is position-before-weight"
        );
        assert_eq!(
            sorted(vec!["AB", "Ab", "aB", "ab"]),
            "ab,aB,Ab,AB",
            "L3: lower before upper"
        );
        // The real shape this exists for — punctuation-ignoring order over two live route keys.
        let live = vec![
            "DELETE /api/v1/missions/{id} delete_mission",
            "DELETE /api/v1/missions/{id}/bookmark remove_bookmark",
        ];
        assert!(sorted(live).starts_with("DELETE /api/v1/missions/{id}/bookmark"));
    }
}
