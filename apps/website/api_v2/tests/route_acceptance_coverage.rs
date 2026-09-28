//! Route acceptance coverage: the route table read from source, the `@route` tags, and the
//! specs of every part agree.
//!
//! The route table is parsed from `src/core/http_router.rs` and every table it merges or nests
//! (`route_acceptance_support::route_table`); every registered route needs exactly one spec,
//! every spec a registered route, every spec all seven dimensions, and every success a contract
//! that resolves; a refusal status stands as a success only on a refusal-only spec whose reason
//! names the module of the route's `@route` tag. Self-tests pin the refusal-only rule and the
//! round-trip comparison (date-times as instants in the API's spelling, everything else exact)
//! on synthetic specs and schemas. The parser's own sentinels pin a known route per domain, the
//! development-only flag, the equipment data routes (a nested sub-router), and both
//! development-only shapes — a gated merge of a nested sub-router and gated route rows — on a
//! synthetic source tree. Needs no database.

mod common;
mod contract_support;
mod route_acceptance_support;

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use route_acceptance_support::contracts::{check_contract_resolves, json_violations};
use route_acceptance_support::derived_probes::{declaration_problems, refusal_problems};
use route_acceptance_support::round_trip_comparison::differences_beside_schema;
use route_acceptance_support::route_table::{
    Handler, RouteRow, crate_source_root, parse_route_table, route_table,
};
use route_acceptance_support::route_tags::{RouteTag, collect_route_tags, cross_check};
use route_acceptance_support::spec::{Access, Contract, Role, RouteSpec};
use route_acceptance_support::specs::all_specs;

fn report(what: &str, problems: &[String]) {
    assert!(
        problems.is_empty(),
        "{what}: {} problem(s):\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}

fn find<'a>(rows: &'a [RouteRow], key: &str) -> &'a RouteRow {
    rows.iter()
        .find(|row| row.key() == key)
        .unwrap_or_else(|| panic!("the route table has no `{key}`"))
}

#[test]
fn route_acceptance_route_table_matches_every_route_tag() {
    let tags = collect_route_tags(&crate_source_root())
        .unwrap_or_else(|errors| panic!("unreadable @route tags:\n  {}", errors.join("\n  ")));
    let rows = route_table();
    report("route table against @route tags", &cross_check(rows, &tags));
    let tagged_rows = rows.iter().filter(|row| row.handler_fn().is_some()).count();
    println!(
        "{} rows ({tagged_rows} served by named handlers), {} tags",
        rows.len(),
        tags.len()
    );
    assert!(
        tagged_rows >= tags.len(),
        "every tag is served by a row, so rows cannot be fewer"
    );
}

#[test]
fn route_acceptance_every_registered_route_has_exactly_one_spec() {
    let specs = all_specs();
    let mut problems = Vec::new();
    let mut missing = 0;
    for row in route_table() {
        match specs.iter().filter(|spec| spec.key == row.key()).count() {
            1 => {}
            0 => {
                missing += 1;
                problems.push(format!("no spec: {} ({})", row.key(), row.source.display()));
            }
            n => problems.push(format!("{n} specs: {}", row.key())),
        }
    }
    for spec in &specs {
        if !route_table().iter().any(|row| row.key() == spec.key) {
            problems.push(format!("spec names no registered route: {}", spec.key));
        }
    }
    println!(
        "{} routes, {} specs, {missing} routes without a spec",
        route_table().len(),
        specs.len()
    );
    report("routes against specs", &problems);
}

#[test]
fn route_acceptance_every_spec_declares_all_seven_dimensions() {
    let problems: Vec<String> = all_specs().iter().flat_map(declaration_problems).collect();
    report("spec declarations", &problems);
}

#[test]
fn route_acceptance_every_json_success_names_a_contract() {
    let tags = collect_route_tags(&crate_source_root())
        .unwrap_or_else(|errors| panic!("unreadable @route tags:\n  {}", errors.join("\n  ")));
    let mut problems = Vec::new();
    for spec in all_specs() {
        let Some((status, contract)) = &spec.success else {
            problems.push(format!("{}: no success contract", spec.key));
            continue;
        };
        if spec.refusal_reason.is_some() {
            problems.extend(refusal_problems(&spec, &tags));
        } else if !(200..400).contains(status) {
            problems.push(format!(
                "{}: success status {status} is not a success (a route that refuses every \
                 request by documented intent declares `.refusal_only`)",
                spec.key
            ));
        }
        for contract in std::iter::once(contract).chain(spec.request_contract.as_ref()) {
            if let Err(problem) = check_contract_resolves(contract) {
                problems.push(format!("{}: {problem}", spec.key));
            }
        }
    }
    report("success contracts", &problems);
}

#[test]
fn route_acceptance_development_only_rows_match_their_access_class() {
    let mut problems = Vec::new();
    for spec in all_specs() {
        let Some(row) = route_table().iter().find(|row| row.key() == spec.key) else {
            continue;
        };
        if row.dev_only != (spec.access == Access::DevelopmentOnly) {
            problems.push(format!(
                "{}: registered development-only = {}, spec access = {:?}",
                spec.key, row.dev_only, spec.access
            ));
        }
    }
    report("development-only rows", &problems);
}

#[test]
fn route_acceptance_route_table_holds_a_known_route_per_domain() {
    let rows = route_table();
    for (key, handler) in [
        ("GET /api/v1/me", "get_me"),
        (
            "GET /api/v1/game-runtime/events/{id}/roster",
            "event_roster",
        ),
        ("GET /api/v1/missions", "list_missions"),
        ("GET /api/v1/servers/{id}/commands", "list_server_commands"),
        ("GET /api/v1/admin/audit-logs", "list_audit_logs"),
        ("GET /api/v1/matches/{matchId}/events", "list_match_events"),
        ("GET /api/v1/users/{discordId}/stats", "get_user_stats"),
        ("GET /api/v1/cms/announcements", "list_cms_announcements"),
    ] {
        assert_eq!(find(rows, key).handler_fn(), Some(handler), "{key}");
    }
    assert_eq!(find(rows, "GET /healthz").handler, Handler::Closure);
    assert_eq!(find(rows, "GET /metrics").handler, Handler::Closure);
    for mount in ["/uploads", "/map-assets", "/map-assets/glyphs"] {
        let row = find(rows, &format!("GET {mount}/{{*path}}"));
        assert_eq!(row.handler, Handler::StaticFiles, "{mount}");
    }
    let uploads = find(rows, "POST /api/v1/cms/uploads");
    assert_eq!(
        uploads.body_limit.as_deref(),
        Some("middleware::MAX_MULTIPART_BODY")
    );
    let versions = find(rows, "POST /api/v1/missions/{id}/versions");
    assert_eq!(versions.body_limit.as_deref(), Some("version_limit"));
    assert_eq!(find(rows, "GET /api/v1/me").body_limit, None);
}

#[test]
fn route_acceptance_route_table_flags_the_development_login_as_development_only() {
    let rows = route_table();
    let dev_login = find(rows, "GET /api/v1/auth/dev-login");
    assert!(
        dev_login.dev_only,
        "the development login is registered only in development"
    );
    assert_eq!(dev_login.handler_fn(), Some("dev_login"));
    assert!(!find(rows, "GET /api/v1/me").dev_only);
    assert!(!find(rows, "GET /healthz").dev_only);
}

#[test]
fn route_acceptance_route_table_reads_the_equipment_data_routes() {
    let prefix = "/api/v1/debug/equipment-data/";
    let nested: Vec<&RouteRow> = route_table()
        .iter()
        .filter(|row| row.path.starts_with(prefix))
        .collect();
    let handlers: Vec<&str> = nested.iter().filter_map(|row| row.handler_fn()).collect();
    assert_eq!(
        handlers,
        [
            "status",
            "overview",
            "resources",
            "relationships",
            "fields",
            "resource_cards",
            "selection",
            "containers",
            "properties",
            "values",
            "documents",
            "download",
        ],
        "the equipment data routes, in registration order"
    );
    assert!(nested.iter().all(|row| row.method == "GET"));
}

/// A throwaway source tree under the system temporary directory, removed on drop.
struct SourceTree(PathBuf);

impl SourceTree {
    fn new(name: &str, files: &[(&str, &str)]) -> SourceTree {
        let root = std::env::temp_dir()
            .join(format!("route-acceptance-{name}-{}", std::process::id()))
            .join("src");
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("create directory");
            std::fs::write(&file, text).expect("write source");
        }
        SourceTree(root)
    }
    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for SourceTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().expect("a parent"));
    }
}

const SYNTHETIC_ROUTER: &str = r#"
pub fn router(state: AppState) -> Router {
    let dev = state.cfg.is_development();
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api/v1", api_v1_routes(dev))
}
fn api_v1_routes(dev: bool) -> Router<AppState> {
    Router::new().merge(crate::alpha::routes(dev)).merge(crate::beta::routes(dev))
}
"#;

fn synthetic_tree(name: &str, beta_condition: &str) -> SourceTree {
    let beta_routes = format!(
        r#"use super::handlers;
pub fn routes(dev: bool) -> Router<AppState> {{
    let mut r = Router::new().route(
        "/beta/{{id}}",
        get(handlers::items::get_item).delete(handlers::items::delete_item),
    );
    // A comment naming .route("/ignored", get(nothing)) registers nothing.
    if {beta_condition} {{
        r = r
            .route("/debug/beta", get(handlers::items::debug_item))
            .route(
                "/debug/beta/{{id}}",
                post(handlers::items::debug_post)
                    .layer(DefaultBodyLimit::max(middleware::MAX_MULTIPART_BODY)),
            );
    }}
    r
}}
"#
    );
    SourceTree::new(
        name,
        &[
            ("core/http_router.rs", SYNTHETIC_ROUTER),
            (
                "alpha/mod.rs",
                "pub mod handlers;\npub mod routes;\npub use routes::routes;\n",
            ),
            (
                "alpha/routes.rs",
                "use super::handlers;\npub fn routes(dev: bool) -> Router<AppState> {\n    let mut r = Router::new().route(\"/alpha\", get(handlers::reads::list));\n    if dev {\n        r = r.merge(handlers::debug::routes());\n    }\n    r\n}\n",
            ),
            ("alpha/handlers/mod.rs", "pub mod debug;\npub mod reads;\n"),
            (
                "alpha/handlers/reads.rs",
                "/// @route GET /api/v1/alpha\npub async fn list() {}\n",
            ),
            (
                "alpha/handlers/debug/mod.rs",
                "mod data;\npub fn routes() -> Router<AppState> {\n    Router::new().nest(\"/debug/data\", Router::new().route(\"/status\", get(data::status)))\n}\n",
            ),
            (
                "alpha/handlers/debug/data.rs",
                "/// @route GET /api/v1/debug/data/status\npub async fn status() {}\n",
            ),
            (
                "beta/mod.rs",
                "pub mod handlers;\nmod routes;\npub use routes::routes;\n",
            ),
            ("beta/routes.rs", &beta_routes),
            ("beta/handlers/mod.rs", "pub mod items;\n"),
            (
                "beta/handlers/items.rs",
                "/// @route GET /api/v1/beta/:id\npub async fn get_item() {}\n/// @route DELETE /api/v1/beta/:id\npub async fn delete_item() {}\n/// @route GET /api/v1/debug/beta\npub async fn debug_item() {}\n/// @route POST /api/v1/debug/beta/:id\npub async fn debug_post() {}\n",
            ),
        ],
    )
}

#[test]
fn route_acceptance_route_table_reads_both_development_only_shapes() {
    let tree = synthetic_tree("both-shapes", "dev");
    let rows = parse_route_table(tree.root()).expect("the synthetic tree parses");
    let summary: Vec<(String, bool)> = rows.iter().map(|row| (row.key(), row.dev_only)).collect();
    let expected = [
        ("GET /healthz", false),
        ("GET /api/v1/alpha", false),
        ("GET /api/v1/debug/data/status", true),
        ("GET /api/v1/beta/{id}", false),
        ("DELETE /api/v1/beta/{id}", false),
        ("GET /api/v1/debug/beta", true),
        ("POST /api/v1/debug/beta/{id}", true),
    ];
    let expected: Vec<(String, bool)> = expected
        .iter()
        .map(|(key, dev)| ((*key).to_string(), *dev))
        .collect();
    assert_eq!(summary, expected);
    let limited = find(&rows, "POST /api/v1/debug/beta/{id}");
    assert_eq!(
        limited.body_limit.as_deref(),
        Some("middleware::MAX_MULTIPART_BODY")
    );
    let tags = collect_route_tags(tree.root()).expect("the synthetic tags parse");
    report("synthetic tree tags", &cross_check(&rows, &tags));
}

#[test]
fn route_acceptance_route_table_refuses_a_registration_under_an_unknown_condition() {
    let tree = synthetic_tree("unknown-condition", "flag_enabled");
    let error = parse_route_table(tree.root()).expect_err("an unknown condition must not parse");
    assert!(error.contains("not the development flag"), "{error}");
}

/// A route tag for the refusal-only self-test, documented in `things/handlers/refusals.rs`.
fn thing_tag() -> RouteTag {
    RouteTag {
        method: "PATCH".into(),
        path: "/api/v1/things/{id}".into(),
        handler_fn: "update_thing".into(),
        file: PathBuf::from("things/handlers/refusals.rs"),
        line: 1,
    }
}

fn refusal_spec(status: u16, error: &'static str, reason: &'static str) -> RouteSpec {
    RouteSpec::role("PATCH /api/v1/things/{id}", Role::Admin).refusal_only(status, error, reason)
}

#[test]
fn route_acceptance_refusal_only_specs_name_their_documenting_module() {
    let tags = [thing_tag()];
    let documented = "things are read-only, documented in things/handlers/refusals.rs";
    assert_eq!(
        refusal_problems(&refusal_spec(409, "read-only", documented), &tags),
        Vec::<String>::new()
    );
    for (spec, expected) in [
        (refusal_spec(409, "read-only", ""), "empty reason"),
        (refusal_spec(409, "read-only", "   "), "empty reason"),
        (
            refusal_spec(409, "read-only", "documented in the handler"),
            "names none of the documenting modules",
        ),
        (
            refusal_spec(200, "read-only", documented),
            "is not a 4xx refusal",
        ),
        (refusal_spec(409, " ", documented), "names no error message"),
    ] {
        let problems = refusal_problems(&spec, &tags);
        assert!(
            problems.len() == 1 && problems[0].contains(expected),
            "expected one problem containing `{expected}`, got {problems:?}"
        );
    }
    let undocumented = refusal_problems(&refusal_spec(409, "read-only", documented), &[]);
    assert!(
        undocumented.len() == 1 && undocumented[0].contains("no `@route` tag"),
        "{undocumented:?}"
    );
}

#[test]
fn route_acceptance_array_and_refusal_contracts_check_their_bodies() {
    let participants = Contract::schema_items(
        "event-access-administration.schema.json",
        "ParticipantAccessExplanation",
    );
    assert_eq!(
        json_violations(&participants, &json!([])),
        Vec::<String>::new()
    );
    let object = json_violations(&participants, &json!({}));
    assert!(
        object.len() == 1 && object[0].contains("not a JSON array"),
        "{object:?}"
    );
    let element = json_violations(&participants, &json!([{}]));
    assert!(
        !element.is_empty() && element.iter().all(|v| v.contains("`/0")),
        "{element:?}"
    );
    let refusal = Contract::RefusalEnvelope { error: "read-only" };
    assert_eq!(
        json_violations(&refusal, &json!({ "error": "read-only" })),
        Vec::<String>::new()
    );
    for body in [
        json!({ "error": "read-only", "details": {} }),
        json!({ "error": "read only" }),
        json!("read-only"),
    ] {
        assert_eq!(json_violations(&refusal, &body).len(), 1, "{body}");
    }
}

/// Rows whose `at` is a date-time through a `$ref`, whose `maybe` is a nullable date-time
/// through `anyOf`, and whose `label` is a plain string.
fn instant_rows_schema() -> Value {
    json!({
        "definitions": {
            "Instant": { "type": "string", "format": "date-time" },
            "Row": {
                "type": "object",
                "properties": {
                    "at": { "$ref": "#/definitions/Instant" },
                    "maybe": { "anyOf": [{ "$ref": "#/definitions/Instant" }, { "type": "null" }] },
                    "label": { "type": "string" }
                }
            }
        },
        "type": "array",
        "items": { "$ref": "#/definitions/Row" }
    })
}

fn row_differences(live: Value, again: Value) -> Vec<String> {
    let schema = instant_rows_schema();
    differences_beside_schema(&schema, &schema, &json!([live]), &json!([again]))
}

#[test]
fn route_acceptance_round_trip_compares_date_times_as_instants_in_the_api_spelling() {
    let live = json!({
        "at": "2026-09-28T12:00:00.25Z",
        "maybe": "2026-09-28T12:00:00Z",
        "label": "x"
    });
    let again = json!({
        "at": "2026-09-28T12:00:00.250Z",
        "maybe": "2026-09-28T12:00:00Z",
        "label": "x"
    });
    assert_eq!(row_differences(live, again), Vec::<String>::new());
    let live = json!({ "at": "2026-09-28T12:00:00.123456789Z", "maybe": null });
    let again = json!({ "at": "2026-09-28T12:00:00.123456789Z", "maybe": null });
    assert_eq!(row_differences(live, again), Vec::<String>::new());
}

#[test]
fn route_acceptance_round_trip_refuses_changed_instants_spellings_and_keys() {
    let at = |text: &str| json!({ "at": text, "label": "x" });
    for (live, again, expected) in [
        (
            at("2026-09-28T12:00:00Z"),
            at("2026-09-28T12:00:01Z"),
            "different instants",
        ),
        (
            at("2026-09-28T12:00:00.250Z"),
            at("2026-09-28T12:00:00.25Z"),
            "not the API spelling",
        ),
        (
            at("2026-09-28T12:00:00+00:00"),
            at("2026-09-28T12:00:00Z"),
            "not the API spelling",
        ),
        (
            at("yesterday"),
            at("2026-09-28T12:00:00Z"),
            "is not RFC 3339",
        ),
        (
            json!({ "maybe": "2026-09-28T12:00:00Z" }),
            json!({ "maybe": "2026-09-28T13:00:00Z" }),
            "different instants",
        ),
        (
            json!({ "label": "2026-09-28T12:00:00.25Z" }),
            json!({ "label": "2026-09-28T12:00:00.250Z" }),
            "sent",
        ),
        (
            at("2026-09-28T12:00:00Z"),
            json!({ "at": "2026-09-28T12:00:00Z" }),
            "drops",
        ),
        (
            at("2026-09-28T12:00:00Z"),
            json!({ "at": "2026-09-28T12:00:00Z", "label": "x", "extra": 1 }),
            "adds",
        ),
    ] {
        let differences = row_differences(live, again);
        assert!(
            differences.len() == 1 && differences[0].contains(expected),
            "expected one difference containing `{expected}`, got {differences:?}"
        );
    }
    let schema = instant_rows_schema();
    let shorter = differences_beside_schema(&schema, &schema, &json!([{}, {}]), &json!([{}]));
    assert_eq!(
        shorter.len(),
        1,
        "an array that loses an element differs: {shorter:?}"
    );
}
