use super::*;

/// The merge function, plus a `.merge(` OUTSIDE its body that must stay invisible to the mount
/// cross-check the same way a `.route(` outside a route table stays invisible to the extractor.
const ROUTER: &str = r#"
fn api_v1_routes(dev: bool, version_limit: usize) -> Router<AppState> {
    Router::new()
        .merge(crate::server_infrastructure::routes())
        .merge(crate::operations::routes())
        .merge(crate::identity_and_access::routes(dev))
}
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(crate::not_a_domain::routes())
        .nest("/api/v1", api_v1_routes(dev, version_limit))
}
"#;

/// Three registrations over two `.route(` lines. The commented-out registration and the `.route(`
/// in the helper below `pub fn routes` must both stay invisible.
const SERVERS_TABLE: &str = r#"
pub fn routes() -> Router<AppState> {
    Router::new()
        // .route("/commented-out", get(handlers::x::ghost))
        .route("/servers", get(handlers::servers::list_servers))
        .route(
            "/servers/{id}/status",
            get(handlers::servers::get_server_status).post(handlers::servers::set_status),
        )
}

fn helper() -> Router<AppState> {
    Router::new().route("/outside", get(handlers::x::outside))
}
"#;

/// Two registrations chained onto one path, in a second file — the union the gate must take.
const EVENTS_TABLE: &str = r#"
pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/events",
        get(handlers::events::list_events).post(handlers::events::create_event),
    )
}
"#;

/// A third file, so the sentinel set spans three tables and a discovery bug that finds only one
/// cannot satisfy it. Its `pub fn routes` takes a parameter, as the live one does.
const ME_TABLE: &str = r#"
pub fn routes(dev: bool) -> Router<AppState> {
    Router::new().route("/me", get(handlers::me::get_me))
}
"#;

const TAGS: &str = r#"
/// @route GET /api/v1/servers
pub async fn list_servers() {}

/// @route GET /api/v1/servers/:id/status
pub async fn get_server_status() {}

/// @route POST /api/v1/servers/:id/status
pub async fn set_status() {}

/// @route GET /api/v1/events
pub async fn list_events() {}

/// @route POST /api/v1/events
pub async fn create_event() {}

/// @route GET /api/v1/me
pub async fn get_me() {}
"#;

/// The three domains the fixture router merges, paired with the table each one holds.
const TABLES: &[(&str, &str)] = &[
    ("server_infrastructure", SERVERS_TABLE),
    ("operations", EVENTS_TABLE),
    ("identity_and_access", ME_TABLE),
];

const TAGS_REL: &str = "apps/website/api_v2/src/handlers/telemetry/servers.rs";

struct Repo(PathBuf);
impl Repo {
    fn new(name: &str) -> Repo {
        let mut p = std::env::temp_dir();
        p.push(format!("tbd-rt-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(p.join("apps/website/api_v2/src/handlers/telemetry")).unwrap();
        std::fs::create_dir_all(p.join("apps/website/api_v2/src/core")).unwrap();
        let r = Repo(p);
        r.router(ROUTER);
        for (domain, body) in TABLES {
            r.table(domain, body);
        }
        r.tags(TAGS);
        r
    }
    fn router(&self, body: &str) {
        std::fs::write(
            self.0.join("apps/website/api_v2/src/core/http_router.rs"),
            body,
        )
        .unwrap();
    }
    fn table(&self, domain: &str, body: &str) {
        let dir = self.0.join("apps/website/api_v2/src").join(domain);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("routes.rs"), body).unwrap();
    }
    fn drop_table(&self, domain: &str) {
        std::fs::remove_file(
            self.0
                .join("apps/website/api_v2/src")
                .join(domain)
                .join("routes.rs"),
        )
        .unwrap();
    }
    fn tags(&self, body: &str) {
        std::fs::write(self.0.join(TAGS_REL), body).unwrap();
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
    // The counts ARE the anti-vacuity assertion: six registrations across three tables, six tags,
    // no drift — and the file count proves all three tables were discovered, not just one.
    let all = Repo::new("clean").expect(
        0,
        &[
            "checked 6 @route tag(s) against 6 registered route(s) in 3 route file(s) under apps/website/api_v2/src",
            "  none — all 6 tag(s) resolve to a registered route.",
            "  none — all 6 registered route(s) are documented.",
            "ROUTE-TAG CHECK: PASS",
        ],
    );
    // A commented-out registration, one below `pub fn routes`, and a `.merge(` outside the merge
    // function are all invisible.
    assert!(
        !all.contains("ghost") && !all.contains("outside") && !all.contains("not_a_domain"),
        "{all}"
    );
}

/// DIRECTION A — a tag naming a route that is not registered (the T-586 servers triple).
#[test]
fn a_tag_pointing_at_no_route_fails() {
    let r = Repo::new("dir-a");
    let extra = "/// @route DELETE /api/v1/servers/:id\npub async fn deactivate_server() {}\n";
    r.tags(&format!("{TAGS}\n{extra}"));
    r.expect(1, &[
            "  apps/website/api_v2/src/handlers/telemetry/servers.rs:20",
            "      @route DELETE /api/v1/servers/{id}  ->  handler `deactivate_server` is NOT registered in the api_v2 domain route tables on that method+path.",
            "checked 7 @route tag(s) against 6 registered route(s)",
            "ROUTE-TAG CHECK: FAIL — 1 unwired tag(s), 0 undocumented route(s)",
        ]);
}

/// DIRECTION B — a registered route whose handler carries no tag (the T-586 `submit_mission`).
///
/// `set_status` is the target because the sentinels cannot be: untag one of those and the sentinel
/// guard fires first, so the cross-check is never reached. That is the guard working, but it makes
/// the sentinel handlers useless for exercising direction B.
#[test]
fn a_route_with_no_tag_fails() {
    let r = Repo::new("dir-b");
    r.tags(&TAGS.replace("/// @route POST /api/v1/servers/:id/status\n", ""));
    r.expect(1, &[
            "  POST /api/v1/servers/{id}/status",
            "      registered to `set_status`, which carries no matching @route tag (GO-7 requires one).",
            "checked 5 @route tag(s) against 6 registered route(s)",
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

/// The union across tables is what the cross-check reads, and it is ordered by [`collate_cmp`]
/// rather than by discovery order — so the report is the same bytes whatever `read_dir` returns.
#[test]
fn routes_spanning_two_files_are_unioned_and_sorted_stably() {
    let r = Repo::new("union");
    // Untag the two non-sentinel handlers, one per table, so direction B must list a route from
    // EACH file. A gate reading only one table lists only one of them.
    let stripped = TAGS
        .replace("/// @route POST /api/v1/servers/:id/status\n", "")
        .replace("/// @route GET /api/v1/events\n", "");
    r.tags(&stripped);
    let all = r.expect(
        1,
        &[
            "  GET /api/v1/events",
            "  POST /api/v1/servers/{id}/status",
            "checked 4 @route tag(s) against 6 registered route(s) in 3 route file(s)",
            "ROUTE-TAG CHECK: FAIL — 0 unwired tag(s), 2 undocumented route(s)",
        ],
    );
    let events = all.find("  GET /api/v1/events").expect("events row");
    let servers = all
        .find("  POST /api/v1/servers/{id}/status")
        .expect("servers row");
    assert!(
        events < servers,
        "collation ignores punctuation at the primary level, so GET sorts before POST:\n{all}"
    );
}

/// THE ANTI-VACUITY CASE. A missing input must never read as "0 tags, 0 routes, all agree".
#[test]
fn inputs_that_were_never_read_do_not_pass() {
    let (code, out) = super::run(Path::new("/nonexistent/tbd-route-tags/repo"));
    let all = out.join("\n");
    assert_eq!(code, 2, "a check that never ran must not exit 0:\n{all}");
    assert!(
        all.contains("target file missing: apps/website/api_v2/src/core/http_router.rs"),
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
            "FAIL: parsed NOTHING — 0 raw @route tag(s), 4 raw .route( registration(s).",
            NOTHING_TAIL[0],
            PARSE_FAIL,
        ],
    );
}

/// The other half of the zero-input case, which only the split into tables made reachable: the
/// tags are all there, and the ROUTER side is empty because nothing was discovered.
#[test]
fn zero_route_files_is_not_a_pass() {
    let r = Repo::new("no-tables");
    for (domain, _) in TABLES {
        r.drop_table(domain);
    }
    r.expect(
        1,
        &[
            "FAIL: parsed NOTHING — 6 raw @route tag(s), 0 raw .route( registration(s).",
            "FAIL: api_v1_routes merges server_infrastructure::routes but src/server_infrastructure/routes.rs does not exist",
            "FAIL: api_v1_routes merges operations::routes but src/operations/routes.rs does not exist",
            "FAIL: api_v1_routes merges identity_and_access::routes but src/identity_and_access/routes.rs does not exist",
            PARSE_FAIL,
        ],
    );
}

/// MOUNT CROSS-CHECK, table → merge. A table nobody merges serves no traffic, yet its routes are
/// still demanded of the tag sweep — a whole domain can go dark with both counts still agreeing.
#[test]
fn an_unmerged_route_file_fails() {
    let r = Repo::new("unmerged");
    r.table(
        "community_content",
        "pub fn routes() -> Router<AppState> {\n    Router::new().route(\"/wiki\", get(handlers::wiki::list_wiki))\n}\n",
    );
    r.expect(
        1,
        &[
            "FAIL: route file src/community_content/routes.rs is not merged by api_v1_routes",
            MOUNT_TAIL,
            PARSE_FAIL,
        ],
    );
}

/// MOUNT CROSS-CHECK, merge → table. The merge names a table this gate never read, so the router
/// side it compared against was short by a whole domain.
#[test]
fn a_merge_without_a_route_file_fails() {
    let r = Repo::new("phantom-merge");
    r.router(&ROUTER.replace(
        "        .merge(crate::operations::routes())\n",
        "        .merge(crate::operations::routes())\n        .merge(crate::missions::routes(version_limit))\n",
    ));
    r.expect(
        1,
        &[
            "FAIL: api_v1_routes merges missions::routes but src/missions/routes.rs does not exist",
            MOUNT_TAIL,
            PARSE_FAIL,
        ],
    );
}

#[test]
fn a_broken_router_shape_is_a_failure_not_a_pass() {
    let r = Repo::new("shape");
    r.router(&ROUTER.replace("fn api_v1_routes", "fn v1_routes"));
    r.expect(
        1,
        &[
            "http_router.rs no longer defines `fn api_v1_routes`",
            SELF_REL,
            SHAPE_FAIL,
        ],
    );
    r.router(&ROUTER.replace(".nest(\"/api/v1\"", ".nest(\"/api/v2\""));
    r.expect(
        1,
        &[
            "http_router.rs no longer nests api_v1_routes at `/api/v1`",
            SHAPE_FAIL,
        ],
    );
}

#[test]
fn an_unreadable_parse_is_named_not_skipped() {
    // A tag with no handler beneath it: the counts diverge and the orphan is printed by name.
    let r = Repo::new("orphan");
    r.tags(&format!("{TAGS}\n/// @route GET /api/v1/orphaned-claim\n"));
    r.expect(1, &[
            "FAIL: 7 @route tag(s) in the tree but 6 parsed into (METHOD, PATH, HANDLER).",
            "      orphan: ORPHAN apps/website/api_v2/src/handlers/telemetry/servers.rs:20 GET /api/v1/orphaned-claim",
            ORPHAN_TAIL,
            PARSE_FAIL,
        ]);
    // A table holding no `pub fn routes` at all is a shape this extractor cannot read, and is
    // NAMED rather than silently contributing nothing.
    let n = Repo::new("no-routes-fn");
    n.table("operations", "fn private_routes() -> Router<AppState> {\n    Router::new().route(\"/events\", get(handlers::events::list_events))\n}\n");
    n.expect(
        1,
        &[
            "      UNPARSED apps/website/api_v2/src/operations/routes.rs-declares-0-column-0-pub-fn-routes",
            UNPARSED_TAIL,
            PARSE_FAIL,
        ],
    );
    // Renaming a sentinel handler on BOTH sides keeps the counts agreeing (6 == 6) and both
    // directions cross-checking clean, so only the sentinel can catch it.
    let s = Repo::new("sentinel");
    s.table(
        "server_infrastructure",
        &SERVERS_TABLE.replace("list_servers", "index_servers"),
    );
    s.tags(&TAGS.replace("list_servers", "index_servers"));
    s.expect(1, &[
            "FAIL: sentinel absent from the router extraction: 'GET /api/v1/servers list_servers' — the parser lost a route that is known to be there.",
            "FAIL: sentinel absent from the tags extraction: 'GET /api/v1/servers list_servers'",
        ]);
}

#[test]
fn the_router_extractor_chains_and_marks() {
    let rows = |src: &str| extract_router(&flatten(&routes_fn_lines(src).0));
    assert_eq!(
        rows(SERVERS_TABLE),
        [
            "GET /api/v1/servers list_servers",
            "GET /api/v1/servers/{id}/status get_server_status",
            "POST /api/v1/servers/{id}/status set_status",
        ]
    );
    assert_eq!(rows(ME_TABLE), ["GET /api/v1/me get_me"]);
    // No quoted path, and a path with no method — both must surface, neither may be dropped.
    let src =
        "pub fn routes() {\n  Router::new().route(NO_PATH, get(h::a)).route(\"/x\", z(q));\n}\n";
    assert_eq!(
        rows(src),
        [
            "UNPARSED no-path-literal-in-registration-2",
            "UNPARSED no-method-handler-for-path-/x",
        ]
    );
    // EXACTLY ONE table per file, counted rather than assumed: zero means the extractor is reading
    // a file that no longer holds a table, two means it would read half of one.
    assert_eq!(routes_fn_lines(SERVERS_TABLE).1, 1);
    assert_eq!(routes_fn_lines("fn routes() {\n}\n").1, 0);
    assert_eq!(
        routes_fn_lines(&format!("{ME_TABLE}{ME_TABLE}")).1,
        2,
        "a second column-0 `pub fn routes` must be seen, not swallowed by the first range"
    );
    // THE COUNT THE GUARD USES. bash grepped `.route(` BEFORE flattening, so it is per-LINE:
    // two lines here (the commented-out one is stripped, the `/outside` one is below the table)
    // against three parsed registrations. Counted after `flatten` it is 1 for ANY input, and
    // `n_routes < 1` can never trip — the guard would print, say nothing, and pass forever.
    let raw = routes_fn_lines(SERVERS_TABLE)
        .0
        .iter()
        .filter(|l| l.contains(".route("))
        .count();
    assert_eq!(raw, 2);
}

#[test]
fn the_merge_extractor_reads_only_the_merge_function() {
    let merged = merged_domains(ROUTER);
    assert_eq!(
        merged.iter().map(String::as_str).collect::<Vec<_>>(),
        ["identity_and_access", "operations", "server_infrastructure"],
        "the `.merge(` below `fn router` is outside the range and must not be read"
    );
    assert!(
        merged_domains("fn something_else() {\n    .merge(crate::x::routes())\n}\n").is_empty()
    );
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
