//! The personnel roster page, `GET /api/v1/admin/users?q&page&per_page`, driven through the real
//! router against `personnel-roster.schema.json`.
//!
//! **Role:** the `administration_personnel_pagination` requirement: the roster's total, page
//! window, `per_page` clamp, past-the-end page, total order, literal search, query refusals and
//! administrator gate.
//! **Position:** boots `api_server::router::router` over this binary's own database; members come from
//! `common::seed_user`, callers from `common::access_token`; every 200 answer is validated
//! against `PersonnelPage` through `contract_support`.
//! **Signals & state:** the database is shared by the cases of this binary, which run in
//! parallel. Each case owns a random username token, carried by every member it seeds and passed
//! as `q`, so a case only ever counts its own members.
//! **Invariants:** usernames and Discord ids are built so that byte order and the database
//! collation agree wherever an order is asserted (same length, alphanumeric, zero-padded).
//!
//! Red check: answering `total` as the served page's item count turns
//! `personnel_pagination_twenty_five_members_total_and_paging_reach_the_last_member` red.

use crate::{common, contract_support};

use api_configuration::configuration::Config;
use api_server::router::router;
use api_state::AppState;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const SUITE: &str = "personnel_pagination";
const SCHEMA: &str = "personnel-roster.schema.json";

/// One case's router, database and administrator, plus the username token its members carry.
struct Roster {
    state: AppState,
    app: Router,
    pool: PgPool,
    admin: String,
    /// `pp` and twelve hex digits: no other member, actor or Arma id in the database contains it.
    token: String,
}

impl Roster {
    async fn boot() -> Self {
        let url = common::require_test_database_url()
            .expect("personnel pagination requires the isolated PostgreSQL test database");
        let pool = api_database::connect(&url)
            .await
            .expect("connect test database");
        api_database::migrate(&pool)
            .await
            .expect("migrate test database");
        let state = api_server::composition::application_state(
            pool.clone(),
            Config::for_tests(url, "personnel-pagination"),
        );
        let app = router(state.clone());
        let admin = common::access_token(
            &state,
            SUITE,
            &format!("{SUITE}-admin-{}", Uuid::new_v4()),
            "admin",
            true,
        )
        .await;
        let token = format!("pp{}", &Uuid::new_v4().simple().to_string()[..12]);
        Self {
            state,
            app,
            pool,
            admin,
            token,
        }
    }

    /// A caller of `role` with a verified membership snapshot.
    async fn caller(&self, role: &str) -> String {
        let id = format!("{SUITE}-{role}-{}", Uuid::new_v4());
        common::access_token(&self.state, SUITE, &id, role, true).await
    }

    /// Seeds one member whose Discord id is the case token followed by `label`, and answers the
    /// `(username, discord_id)` pair.
    async fn seed(&self, label: &str, username: &str) -> (String, String) {
        let discord_id = format!("{}{label}", self.token);
        common::seed_user(
            &self.pool,
            &discord_id,
            username,
            &common::unique_arma(SUITE),
            "enlisted",
        )
        .await;
        (username.to_owned(), discord_id)
    }

    /// Seeds `count` members named `<token>m<nnn>` (odd numbers in upper case), newest first,
    /// so neither insertion order nor letter case is the served order.
    async fn seed_numbered(&self, count: usize) -> Vec<(String, String)> {
        let mut members = Vec::with_capacity(count);
        for n in (0..count).rev() {
            let username = if n % 2 == 0 {
                format!("{}m{n:03}", self.token)
            } else {
                format!("{}M{n:03}", self.token.to_uppercase())
            };
            members.push(self.seed(&format!("u{n:03}"), &username).await);
        }
        members
    }

    async fn get(&self, uri: &str, bearer: Option<&str>) -> (StatusCode, Value) {
        let mut request = Request::builder().method("GET").uri(uri);
        if let Some(bearer) = bearer {
            request = request.header(header::AUTHORIZATION, format!("Bearer {bearer}"));
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::empty()).expect("the request builds"))
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the response body reads to the end");
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// The roster answer for a raw query string, as the administrator.
    async fn query(&self, query: &str) -> (StatusCode, Value) {
        self.get(&format!("/api/v1/admin/users?{query}"), Some(&self.admin))
            .await
    }

    /// One page for `q`; the answer must be 200 and a valid `PersonnelPage`.
    async fn page(&self, q: &str, page: i64, per_page: i64) -> Value {
        let query = format!("q={}&page={page}&per_page={per_page}", encode(q));
        let (status, body) = self.query(&query).await;
        assert_eq!(status, StatusCode::OK, "GET ?{query}: {body}");
        assert_personnel_page(&body);
        body
    }
}

/// `value` percent-encoded for a query string.
fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// The page validates against the root contract and its `PersonnelPage` definition.
fn assert_personnel_page(body: &Value) {
    contract_support::assert_valid(SCHEMA, None, body);
    contract_support::assert_valid(SCHEMA, Some("PersonnelPage"), body);
}

fn items(body: &Value) -> &Vec<Value> {
    body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("a page carries an items array: {body}"))
}

fn discord_ids(body: &Value) -> Vec<String> {
    items(body)
        .iter()
        .map(|row| {
            row["discord_id"]
                .as_str()
                .expect("the `discord_id` field is a string")
                .to_owned()
        })
        .collect()
}

/// The Discord ids of `members` in the served order: `lower(username)`, then `discord_id`.
fn served_order(members: &[(String, String)]) -> Vec<String> {
    let mut sorted: Vec<(String, String)> = members
        .iter()
        .map(|(username, id)| (username.to_lowercase(), id.clone()))
        .collect();
    sorted.sort();
    sorted.into_iter().map(|(_, id)| id).collect()
}

/// A 400 answered in the `{error, details?}` envelope.
fn assert_error_envelope(what: &str, body: &Value) {
    let object = body
        .as_object()
        .unwrap_or_else(|| panic!("{what}: the refusal is a JSON object, got {body}"));
    assert!(
        object["error"]
            .as_str()
            .is_some_and(|error| !error.is_empty()),
        "{what}: the envelope names the error: {body}"
    );
    assert!(
        object.keys().all(|key| key == "error" || key == "details"),
        "{what}: the envelope carries only error and details: {body}"
    );
}

#[tokio::test]
async fn personnel_pagination_twenty_five_members_total_and_paging_reach_the_last_member() {
    let roster = Roster::boot().await;
    let members = roster.seed_numbered(25).await;
    let expected = served_order(&members);

    let mut served = Vec::new();
    for (page, served_items) in [(1, 10), (2, 10), (3, 5)] {
        let body = roster.page(&roster.token, page, 10).await;
        assert_eq!(body["total"], json!(25), "page {page} counts every member");
        assert_eq!(body["page"], json!(page));
        assert_eq!(body["per_page"], json!(10));
        assert_eq!(items(&body).len(), served_items, "page {page}: {body}");
        served.extend(discord_ids(&body));
    }
    assert_eq!(served, expected, "three pages list the 25 members in order");
    assert_eq!(
        served.last(),
        Some(&expected[24]),
        "the 25th member is reached on the last page"
    );

    // Without page or per_page the first 20 are served.
    let (status, body) = roster.query(&format!("q={}", encode(&roster.token))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_personnel_page(&body);
    assert_eq!(body["page"], json!(1));
    assert_eq!(body["per_page"], json!(20));
    assert_eq!(body["total"], json!(25));
    assert_eq!(discord_ids(&body), expected[..20].to_vec());
}

#[tokio::test]
async fn personnel_pagination_non_administrators_are_refused() {
    let roster = Roster::boot().await;
    let uri = "/api/v1/admin/users?page=1&per_page=5";

    for role in ["enlisted", "leader", "mission_maker"] {
        let bearer = roster.caller(role).await;
        let (status, body) = roster.get(uri, Some(&bearer)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{role} is refused: {body}");
        assert_error_envelope(role, &body);
    }

    let (status, body) = roster.get(uri, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "anonymous: {body}");
    assert_error_envelope("anonymous", &body);

    let (status, body) = roster.get(uri, Some("not-a-token")).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a malformed token: {body}"
    );
    assert_error_envelope("malformed token", &body);

    let (status, body) = roster.get(uri, Some(&roster.admin)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the administrator is served: {body}"
    );
    assert_personnel_page(&body);
}
