//! Owned fixtures for the doctrine wiki suite (`wiki_features`): suite-owned administrators and
//! members, requests through the real HTTP router, save bodies, the stored page, revision and
//! audit state read straight from PostgreSQL, the refusal checks against the wiki and content
//! contracts, the expected-tree builders, and raising triggers that inject a storage failure
//! into one page's save only.
//!
//! Compiled into each suite that writes `mod wiki_support;` (next to `mod common;` and
//! `mod contract_support;`); it adds no test binary.

#![allow(dead_code)]

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

use crate::{common, contract_support};

/// The contract every wiki answer is checked against.
pub(crate) const WIKI_CONTRACT: &str = "wiki-page.schema.json";
/// The contract of the content routes' error envelope.
const CONTENT_CONTRACT: &str = "content-upload.schema.json";

/// A signed-in caller: the Discord id the API stamps and the bearer token it presents.
pub(crate) struct Actor {
    pub id: String,
    pub token: String,
}

/// One case's router, its administrator and its enlisted member.
pub(crate) struct WikiSuite {
    pub state: AppState,
    pub app: Router,
    pub admin: Actor,
    pub member: Actor,
    suite: String,
}

/// Everything PostgreSQL holds for one slug: the page row, its revision rows (oldest first) and
/// its wiki audit rows (oldest first), each as the JSON Postgres renders the row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredWikiState {
    pub page: Option<Value>,
    pub revisions: Vec<Value>,
    pub audits: Vec<Value>,
}

impl WikiSuite {
    pub(crate) async fn new(suite: &str) -> Self {
        let url = common::require_test_database_url().expect("the wiki suite requires PostgreSQL");
        let pool = api_database::connect(&url)
            .await
            .expect("the test database accepts a connection");
        api_database::migrate(&pool)
            .await
            .expect("the migrations apply to the test database");
        let state = api_server::composition::application_state(
            pool,
            Config::for_tests(url, "wiki-features"),
        );
        let app = router(state.clone());
        let mut fixture = Self {
            state,
            app,
            admin: Actor {
                id: String::new(),
                token: String::new(),
            },
            member: Actor {
                id: String::new(),
                token: String::new(),
            },
            suite: suite.to_owned(),
        };
        fixture.admin = fixture.account("admin", "admin").await;
        fixture.member = fixture.account("member", "enlisted").await;
        fixture
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.state.pool
    }

    /// A suite-owned account with a verified membership snapshot for `role`.
    pub(crate) async fn account(&self, label: &str, role: &str) -> Actor {
        let id = format!("{}-{label}-{}", self.suite, Uuid::new_v4());
        let token = common::access_token(&self.state, &self.suite, &id, role, true).await;
        Actor { id, token }
    }

    /// One JSON request; answers the status and the decoded body (`null` when not JSON).
    pub(crate) async fn call(
        &self,
        actor: Option<&Actor>,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let content_type = body.as_ref().map(|_| "application/json");
        self.send(
            actor,
            method,
            uri,
            body.map(|value| value.to_string()),
            content_type,
        )
        .await
    }

    /// One request with a raw body and an explicit content type.
    pub(crate) async fn send(
        &self,
        actor: Option<&Actor>,
        method: &str,
        uri: &str,
        body: Option<String>,
        content_type: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(actor) = actor {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", actor.token));
        }
        if let Some(content_type) = content_type {
            request = request.header(header::CONTENT_TYPE, content_type);
        }
        let request = request
            .body(body.map_or(Body::empty(), Body::from))
            .expect("the request builds");
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the response body reads to the end");
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// `GET uri` as the member.
    pub(crate) async fn read(&self, uri: &str) -> (StatusCode, Value) {
        self.call(Some(&self.member), "GET", uri, None).await
    }

    /// `PUT /api/v1/wiki/{slug}` with `body` as `actor`.
    pub(crate) async fn save_as(
        &self,
        actor: &Actor,
        slug: &str,
        body: Value,
    ) -> (StatusCode, Value) {
        self.call(Some(actor), "PUT", &page_uri(slug), Some(body))
            .await
    }

    /// `PUT /api/v1/wiki/{slug}` with `body` as the administrator.
    pub(crate) async fn save(&self, slug: &str, body: Value) -> (StatusCode, Value) {
        self.save_as(&self.admin, slug, body).await
    }

    /// Creates the page as the administrator; answers the article after checking it is 201 and
    /// matches the contract.
    pub(crate) async fn create(&self, slug: &str, title: &str, body_md: &str) -> Value {
        let (status, article) = self.save(slug, save_body(title, body_md, None)).await;
        assert_eq!(status, StatusCode::CREATED, "create {slug}: {article}");
        contract_support::assert_valid(WIKI_CONTRACT, Some("WikiArticle"), &article);
        article
    }

    /// Saves the page's next revision from `base` as the administrator; answers the article after
    /// checking it is 200 and matches the contract.
    pub(crate) async fn update(&self, slug: &str, base: i64, title: &str, body_md: &str) -> Value {
        let (status, article) = self.save(slug, save_body(title, body_md, Some(base))).await;
        assert_eq!(status, StatusCode::OK, "save {slug} from {base}: {article}");
        contract_support::assert_valid(WIKI_CONTRACT, Some("WikiArticle"), &article);
        article
    }

    /// The page row stored under `slug`, as Postgres renders it to JSON.
    pub(crate) async fn page_row(&self, slug: &str) -> Option<Value> {
        let text: Option<String> =
            sqlx::query_scalar("SELECT row_to_json(p)::text FROM wiki_pages p WHERE p.slug = $1")
                .bind(slug)
                .fetch_optional(self.pool())
                .await
                .expect("the read of wiki_pages runs");
        text.map(|text| serde_json::from_str(&text).expect("the text decodes as JSON"))
    }

    /// Every revision row saved under `slug` or held by the page under `slug`, oldest first.
    pub(crate) async fn revision_rows(&self, slug: &str) -> Vec<Value> {
        self.json_rows(
            "SELECT COALESCE(json_agg(r ORDER BY r.revision), '[]'::json)::text \
             FROM wiki_page_revisions r \
             WHERE r.slug = $1 OR r.page_id IN (SELECT id FROM wiki_pages WHERE slug = $1)",
            slug,
        )
        .await
    }

    /// Every wiki audit row naming `slug`, oldest first.
    pub(crate) async fn audit_rows(&self, slug: &str) -> Vec<Value> {
        self.json_rows(
            "SELECT COALESCE(json_agg(json_build_object('action', action, 'actor_id', actor_id, \
             'target_type', target_type, 'target_id', target_id, 'message', message) \
             ORDER BY id), '[]'::json)::text \
             FROM audit_logs WHERE target_type = 'wiki_page' AND target_id = $1",
            slug,
        )
        .await
    }

    /// The page, revision and audit rows of `slug`.
    pub(crate) async fn stored(&self, slug: &str) -> StoredWikiState {
        StoredWikiState {
            page: self.page_row(slug).await,
            revisions: self.revision_rows(slug).await,
            audits: self.audit_rows(slug).await,
        }
    }

    /// The wiki audit rows naming `actor`, whatever page they target.
    pub(crate) async fn wiki_audit_count_by(&self, actor: &Actor) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND target_type = 'wiki_page'",
        )
        .bind(&actor.id)
        .fetch_one(self.pool())
        .await
        .expect("the read of audit_logs returns a row")
    }

    async fn json_rows(&self, sql: &'static str, slug: &str) -> Vec<Value> {
        let text: String = sqlx::query_scalar(sql)
            .bind(slug)
            .fetch_one(self.pool())
            .await
            .expect("the JSON rows query returns a row");
        match serde_json::from_str(&text).expect("the text decodes as JSON") {
            Value::Array(rows) => rows,
            other => panic!("json_agg answered {other}"),
        }
    }
}

/// A slug no other case or run holds: `wf-<label>-<ten hex digits>`.
pub(crate) fn unique_slug(label: &str) -> String {
    format!("wf-{label}-{}", &Uuid::new_v4().simple().to_string()[..10])
}

pub(crate) fn page_uri(slug: &str) -> String {
    format!("/api/v1/wiki/{slug}")
}

pub(crate) fn revisions_uri(slug: &str) -> String {
    format!("/api/v1/wiki/{slug}/revisions")
}

/// A complete save body: category `SOP`, no icon, navigation position 0.
pub(crate) fn save_body(title: &str, body_md: &str, base_revision: Option<i64>) -> Value {
    json!({
        "category": "SOP",
        "title": title,
        "icon": "",
        "nav_order": 0,
        "body_md": body_md,
        "base_revision": base_revision,
    })
}

/// Asserts a refusal: its status, the `{error, details?}` envelope of the content contract, a
/// wiki refusal's `details` against `WikiSaveRefusal`, and no `details.code` other than `code`.
pub(crate) fn assert_refusal(
    status: StatusCode,
    body: &Value,
    expected: StatusCode,
    code: Option<&str>,
) {
    assert_eq!(status, expected, "{body}");
    contract_support::assert_valid(CONTENT_CONTRACT, Some("ContentError"), body);
    assert_eq!(body["details"]["code"].as_str(), code, "{body}");
    if code.is_some_and(|code| code.starts_with("wiki_")) {
        contract_support::assert_valid(WIKI_CONTRACT, Some("WikiSaveRefusal"), &body["details"]);
    }
}
