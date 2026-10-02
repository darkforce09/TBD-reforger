//! The personnel roster page, `GET /api/v1/admin/users?q&page&per_page`, driven through the real
//! router against `personnel-roster.schema.json`.
//!
//! **Role:** the `administration_personnel_pagination` requirement: the roster's total, page
//! window, `per_page` clamp, past-the-end page, total order, literal search, query refusals and
//! administrator gate.
//! **Position:** boots `http_router::router` over this binary's own database; members come from
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

mod common;
mod contract_support;

use std::collections::BTreeSet;

use api::core::{application_state::AppState, configuration::Config, database, http_router};
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
        let pool = database::connect(&url)
            .await
            .expect("connect test database");
        database::migrate(&pool)
            .await
            .expect("migrate test database");
        let state = AppState::new(pool.clone(), Config::for_tests(url, "personnel-pagination"));
        let app = http_router::router(state.clone());
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
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
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
        .map(|row| row["discord_id"].as_str().unwrap().to_owned())
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
async fn personnel_pagination_per_page_one_hundred_serves_one_hundred_of_one_hundred_twenty() {
    let roster = Roster::boot().await;
    let members = roster.seed_numbered(120).await;
    let expected = served_order(&members);

    let first = roster.page(&roster.token, 1, 100).await;
    assert_eq!(first["total"], json!(120));
    assert_eq!(first["per_page"], json!(100));
    assert_eq!(items(&first).len(), 100, "a full page of 100");

    let second = roster.page(&roster.token, 2, 100).await;
    assert_eq!(second["total"], json!(120));
    assert_eq!(items(&second).len(), 20, "the remaining 20");

    let served: Vec<String> = discord_ids(&first)
        .into_iter()
        .chain(discord_ids(&second))
        .collect();
    assert_eq!(served, expected, "both pages together list all 120 once");
}

#[tokio::test]
async fn personnel_pagination_per_page_above_one_hundred_clamps_to_one_hundred() {
    let roster = Roster::boot().await;
    let members = roster.seed_numbered(105).await;
    let expected = served_order(&members);

    for per_page in [101, 500, i64::MAX] {
        let body = roster.page(&roster.token, 1, per_page).await;
        assert_eq!(
            body["per_page"],
            json!(100),
            "per_page={per_page} is served as 100"
        );
        assert_eq!(body["total"], json!(105));
        assert_eq!(discord_ids(&body), expected[..100].to_vec());
    }

    // The clamped size is also the offset step: page 2 starts at the 101st member.
    let second = roster.page(&roster.token, 2, 500).await;
    assert_eq!(second["per_page"], json!(100));
    assert_eq!(discord_ids(&second), expected[100..].to_vec());
}

#[tokio::test]
async fn personnel_pagination_page_past_the_end_is_empty_with_the_real_total() {
    let roster = Roster::boot().await;
    let members = roster.seed_numbered(12).await;
    let expected = served_order(&members);

    let last = roster.page(&roster.token, 3, 5).await;
    assert_eq!(discord_ids(&last), expected[10..].to_vec());

    for page in [4, 1000, i64::MAX] {
        let body = roster.page(&roster.token, page, 5).await;
        assert!(
            items(&body).is_empty(),
            "page {page} is past the end: {body}"
        );
        assert_eq!(body["total"], json!(12), "page {page} keeps the real total");
        assert_eq!(body["page"], json!(page), "page {page} is echoed");
        assert_eq!(body["per_page"], json!(5));
    }
}

#[tokio::test]
async fn personnel_pagination_duplicate_usernames_keep_a_stable_order_covering_every_member_once() {
    let roster = Roster::boot().await;
    let token = roster.token.clone();
    let mut members = vec![
        roster.seed("a00", &format!("{token}aaa")).await,
        roster.seed("z00", &format!("{token}zzz")).await,
    ];
    // Seven members whose usernames are equal ignoring case, seeded out of Discord id order.
    for (n, spelling) in [
        (4, "dup"),
        (1, "DUP"),
        (6, "Dup"),
        (0, "dUp"),
        (5, "dup"),
        (3, "DUP"),
        (2, "duP"),
    ] {
        members.push(
            roster
                .seed(&format!("d{n:02}"), &format!("{token}{spelling}"))
                .await,
        );
    }
    let expected = served_order(&members);
    assert_eq!(expected.len(), 9);

    let mut passes = Vec::new();
    for _ in 0..2 {
        let mut served = Vec::new();
        for page in 1..=3 {
            let body = roster.page(&token, page, 3).await;
            assert_eq!(body["total"], json!(9));
            assert_eq!(items(&body).len(), 3, "page {page}: {body}");
            served.extend(discord_ids(&body));
        }
        let distinct: BTreeSet<&String> = served.iter().collect();
        assert_eq!(distinct.len(), 9, "no member is served twice: {served:?}");
        assert_eq!(
            served, expected,
            "ties on lower(username) order by discord_id"
        );
        passes.push(served);
    }
    assert_eq!(passes[0], passes[1], "the order is the same on every read");
}

#[tokio::test]
async fn personnel_pagination_search_narrows_the_total_and_matches_pattern_characters_literally() {
    let roster = Roster::boot().await;
    let token = roster.token.clone();
    let literal_one = roster.seed("l01", &format!("{token}_x%y1")).await.1;
    let literal_two = roster.seed("l02", &format!("{token}_x%y2")).await.1;
    // Matches `<token>_x%y` only if `_` and `%` act as wildcards.
    let wildcard_decoy = roster.seed("w01", &format!("{token}Qx7y1")).await.1;
    let backslash = roster.seed("b01", &format!("{token}\\z1")).await.1;
    let plain = roster.seed("p01", &format!("{token}plain")).await.1;

    // Only the unfiltered token searches may serve the decoy and the plain member.
    let everyone = BTreeSet::from([
        literal_one.clone(),
        literal_two.clone(),
        wildcard_decoy,
        backslash.clone(),
        plain,
    ]);
    let literals = BTreeSet::from([literal_one, literal_two]);
    let cases: [(String, BTreeSet<String>); 7] = [
        (token.clone(), everyone.clone()),
        (token.to_uppercase(), everyone),
        (format!("{token}_x%y"), literals.clone()),
        (format!("{token}_"), literals.clone()),
        (format!("  {token}_x%y  "), literals),
        (format!("{token}%"), BTreeSet::new()),
        (format!("{token}\\z"), BTreeSet::from([backslash])),
    ];
    for (q, expected) in cases {
        let body = roster.page(&q, 1, 100).await;
        let served: BTreeSet<String> = discord_ids(&body).into_iter().collect();
        assert_eq!(
            body["total"],
            json!(expected.len()),
            "q={q:?} narrows the total: {body}"
        );
        assert_eq!(served, expected, "q={q:?} matches literally");
    }
}

#[tokio::test]
async fn personnel_pagination_invalid_page_values_answer_400_with_the_error_envelope() {
    let roster = Roster::boot().await;
    roster.seed_numbered(3).await;
    let q = encode(&roster.token);

    for query in [
        "page=0",
        "per_page=0",
        "page=-1",
        "per_page=-20",
        "page=abc",
        "per_page=ten",
        "page=1.5",
        "per_page=",
        "page=99999999999999999999",
    ] {
        let (status, body) = roster.query(&format!("q={q}&{query}")).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "?{query} answers 400: {body}"
        );
        assert_error_envelope(query, &body);
    }

    // The smallest valid window is served.
    let body = roster.page(&roster.token, 1, 1).await;
    assert_eq!(items(&body).len(), 1);
    assert_eq!(body["total"], json!(3));
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

#[tokio::test]
async fn personnel_pagination_pages_validate_against_the_personnel_page_contract() {
    let roster = Roster::boot().await;
    let token = roster.token.clone();
    let (_, banned) = roster.seed("c01", &format!("{token}c01")).await;
    let (_, leader) = roster.seed("c02", &format!("{token}c02")).await;
    let (_, unlinked) = roster.seed("c03", &format!("{token}c03")).await;

    sqlx::query(
        "UPDATE users SET is_banned = true, role = 'guest'::user_role, total_deployments = 7, \
         arma_character = '[TBD] Banned' WHERE discord_id = $1",
    )
    .bind(&banned)
    .execute(&roster.pool)
    .await
    .expect("shape the banned member");
    for reason in ["late", "absent"] {
        sqlx::query(
            "INSERT INTO warnings (discord_id, issued_by, reason, created_at) \
             VALUES ($1, $2, $3, now())",
        )
        .bind(&banned)
        .bind(common::DEV_LOGIN_USER)
        .bind(reason)
        .execute(&roster.pool)
        .await
        .expect("warn the banned member");
    }
    sqlx::query("UPDATE users SET role = 'leader'::user_role WHERE discord_id = $1")
        .bind(&leader)
        .execute(&roster.pool)
        .await
        .expect("promote the leader");
    sqlx::query(
        "UPDATE users SET arma_id = NULL, discord_handle = NULL, arma_character = NULL \
         WHERE discord_id = $1",
    )
    .bind(&unlinked)
    .execute(&roster.pool)
    .await
    .expect("unlink the member");

    let body = roster.page(&token, 1, 20).await;
    assert_eq!(discord_ids(&body), vec![banned.clone(), leader, unlinked]);
    let rows = items(&body);
    for row in rows {
        contract_support::assert_valid(SCHEMA, Some("PersonnelRow"), row);
    }
    assert_eq!(rows[0]["is_banned"], json!(true));
    assert_eq!(rows[0]["role"], json!("guest"));
    assert_eq!(rows[0]["warnings"], json!(2));
    assert_eq!(rows[0]["total_deployments"], json!(7));
    assert_eq!(rows[0]["arma_character"], json!("[TBD] Banned"));
    assert_eq!(rows[1]["role"], json!("leader"));
    assert_eq!(rows[1]["warnings"], json!(0));
    assert_eq!(
        rows[2]["arma_id"],
        Value::Null,
        "an unlinked member's arma_id is null"
    );
    assert_eq!(rows[2]["discord_handle"], json!(""));
    assert_eq!(rows[2]["arma_character"], json!(""));

    // A past-the-end page is a valid page too.
    roster.page(&token, 9, 20).await;

    // The contract is strict enough to tell a wrong page from a right one.
    let mut unclamped = body.clone();
    unclamped["per_page"] = json!(500);
    contract_support::assert_invalid(SCHEMA, Some("PersonnelPage"), &unclamped);
    let mut extra_key = body.clone();
    extra_key["next_cursor"] = json!(2);
    contract_support::assert_invalid(SCHEMA, Some("PersonnelPage"), &extra_key);
    let mut missing_count = body.clone();
    missing_count["items"][0]
        .as_object_mut()
        .unwrap()
        .remove("warnings");
    contract_support::assert_invalid(SCHEMA, Some("PersonnelPage"), &missing_count);
}
