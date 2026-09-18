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
        std::fs::create_dir_all(p.join("apps/website/api_v2/src/handlers/telemetry")).unwrap();
        std::fs::create_dir_all(p.join("apps/website/api_v2/src/core")).unwrap();
        let r = Repo(p);
        r.app(APP);
        r.tags(TAGS);
        r
    }
    fn app(&self, body: &str) {
        std::fs::write(
            self.0.join("apps/website/api_v2/src/core/http_router.rs"),
            body,
        )
        .unwrap();
    }
    fn tags(&self, body: &str) {
        let p = self
            .0
            .join("apps/website/api_v2/src/handlers/telemetry/servers.rs");
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
    let all = Repo::new("clean").expect(
        0,
        &[
            "checked 3 @route tag(s) against 3 registered route(s) in apps/website/api_v2/src/core/http_router.rs",
            "  none — all 3 tag(s) resolve to a registered route.",
            "  none — all 3 registered route(s) are documented.",
            "ROUTE-TAG CHECK: PASS",
        ],
    );
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
            "  apps/website/api_v2/src/handlers/telemetry/servers.rs:11",
            "      @route DELETE /api/v1/servers/{id}  ->  handler `deactivate_server` is NOT registered in apps/website/api_v2/src/core/http_router.rs on that method+path.",
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
            "http_router.rs no longer defines `fn api_routes`",
            SELF_REL,
            SHAPE_FAIL,
        ],
    );
    r.app(&APP.replace(".nest(\"/api/v1\"", ".nest(\"/api/v2\""));
    r.expect(
        1,
        &[
            "http_router.rs no longer nests api_routes at `/api/v1`",
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
            "FAIL: 4 @route tag(s) in the tree but 3 parsed into (METHOD, PATH, HANDLER).",
            "      orphan: ORPHAN apps/website/api_v2/src/handlers/telemetry/servers.rs:11 GET /api/v1/orphaned-claim",
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
    let src =
        "fn api_routes() {\n  Router::new().route(NO_PATH, get(h::a)).route(\"/x\", z(q));\n}\n";
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
