//! Owned fixtures for the community content suites (`vehicle_mutations`, `content_storage`):
//! suite-owned administrators and members, a router whose upload directory is private to the
//! case, raw JSON and multipart requests sent from synthetic peers, image payload builders, the
//! error envelope check against `content-upload.schema.json`, and raising triggers that inject a
//! storage failure into one actor's or one row's transaction only.
//!
//! Compiled into each suite that writes `mod content_support;` (next to `mod common;` and
//! `mod contract_support;`); it adds no test binary.

#![allow(dead_code)]

use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use api::router::router;
use api_configuration::configuration::Config;
use api_state::AppState;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{common, contract_support};

/// The JSON request body limit of every content route except the upload (1 MiB).
pub const JSON_BODY_LIMIT: usize = 1 << 20;
/// The contract every content refusal is checked against.
const CONTENT_CONTRACT: &str = "content-upload.schema.json";

/// A signed-in caller: the Discord id the API stamps and the bearer token it presents.
pub struct Actor {
    pub id: String,
    pub token: String,
}

/// One case's router, its administrator and enlisted member, and its private upload directory.
pub struct ContentSuite {
    pub state: AppState,
    pub app: Router,
    pub admin: Actor,
    pub member: Actor,
    /// The directory the router's upload handler writes into and `/uploads` serves.
    pub upload_dir: PathBuf,
    /// The case's scratch directory, removed when the suite value drops.
    scratch: PathBuf,
    suite: String,
}

impl ContentSuite {
    /// A router whose upload directory is `<scratch>/uploads`, which the router creates at boot.
    pub async fn new(suite: &str) -> Self {
        let scratch = fresh_scratch_dir(suite);
        let upload_dir = scratch.join("uploads");
        Self::boot(suite, scratch, upload_dir).await
    }

    /// A router whose upload directory names a regular file, so every upload write fails.
    pub async fn with_upload_dir_as_regular_file(suite: &str) -> Self {
        let scratch = fresh_scratch_dir(suite);
        let upload_dir = scratch.join("uploads-is-a-regular-file");
        std::fs::write(&upload_dir, b"not a directory").unwrap();
        Self::boot(suite, scratch, upload_dir).await
    }

    async fn boot(suite: &str, scratch: PathBuf, upload_dir: PathBuf) -> Self {
        let url = common::require_test_database_url().expect("content suites require PostgreSQL");
        let pool = api_database::connect(&url).await.unwrap();
        api_database::migrate(&pool).await.unwrap();
        let mut config = Config::for_tests(url, "content-suites");
        config.upload_dir = upload_dir.display().to_string();
        let state = api::composition::application_state(pool, config);
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
            upload_dir,
            scratch,
            suite: suite.to_owned(),
        };
        fixture.admin = fixture.account("admin", "admin").await;
        fixture.member = fixture.account("member", "enlisted").await;
        fixture
    }

    pub fn pool(&self) -> &PgPool {
        &self.state.pool
    }

    /// The case's scratch directory, which holds the upload directory.
    pub fn scratch(&self) -> &Path {
        &self.scratch
    }

    /// A suite-owned account with a verified membership snapshot for `role`.
    pub async fn account(&self, label: &str, role: &str) -> Actor {
        let id = format!("{}-{label}-{}", self.suite, Uuid::new_v4());
        let token = common::access_token(&self.state, &self.suite, &id, role, true).await;
        Actor { id, token }
    }

    /// One JSON request; a body is sent as `application/json`.
    pub async fn call(
        &self,
        actor: Option<&Actor>,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        match body {
            Some(body) => {
                self.send(
                    actor,
                    method,
                    uri,
                    Some("application/json"),
                    body.to_string().into_bytes(),
                )
                .await
            }
            None => self.send(actor, method, uri, None, Vec::new()).await,
        }
    }

    /// One request with raw body bytes and an optional content type; answers the status and the
    /// body as JSON (`null` when it is not JSON).
    pub async fn send(
        &self,
        actor: Option<&Actor>,
        method: &str,
        uri: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> (StatusCode, Value) {
        let (status, bytes) = self
            .send_bytes(actor, method, uri, content_type, body)
            .await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// One request from a fresh synthetic peer; answers the status and the raw body bytes.
    ///
    /// A `oneshot` request carries no `ConnectInfo`, so every request would share the limiter
    /// bucket of `0.0.0.0` and a case with many requests would end at 429. The limiter is not
    /// under test here, so each request gets its own peer, and a 429 fails loudly instead of
    /// reading as a handler answer.
    pub async fn send_bytes(
        &self,
        actor: Option<&Actor>,
        method: &str,
        uri: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> (StatusCode, Vec<u8>) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(actor) = actor {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", actor.token));
        }
        if let Some(content_type) = content_type {
            request = request.header(header::CONTENT_TYPE, content_type);
        }
        let mut request = request.body(Body::from(body)).unwrap();
        request.extensions_mut().insert(ConnectInfo(next_peer()));
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        assert_ne!(
            status,
            StatusCode::TOO_MANY_REQUESTS,
            "{method} {uri} was rate limited before its handler ran"
        );
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        (status, bytes)
    }

    /// `POST /api/v1/cms/uploads` with a multipart body made of `parts`.
    pub async fn upload(
        &self,
        actor: Option<&Actor>,
        parts: &[MultipartPart<'_>],
    ) -> (StatusCode, Value) {
        let (content_type, body) = multipart_body(parts);
        self.send(
            actor,
            "POST",
            "/api/v1/cms/uploads",
            Some(&content_type),
            body,
        )
        .await
    }

    /// `POST /api/v1/cms/uploads` with one `file` field named `file_name`.
    pub async fn upload_file(
        &self,
        actor: Option<&Actor>,
        file_name: &str,
        bytes: &[u8],
    ) -> (StatusCode, Value) {
        self.upload(
            actor,
            &[MultipartPart {
                name: "file",
                file_name: Some(file_name),
                bytes,
            }],
        )
        .await
    }

    /// The number of audit rows `actor` wrote, leaving out the `auth.*` lines of the sign-in that
    /// minted the actor's token.
    pub async fn content_audits_by(&self, actor: &Actor) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action NOT LIKE 'auth.%'",
        )
        .bind(&actor.id)
        .fetch_one(self.pool())
        .await
        .unwrap()
    }

    /// The number of vehicle rows `actor` created, deleted ones included.
    pub async fn vehicles_created_by(&self, actor: &Actor) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM vehicle_databases WHERE created_by = $1")
            .bind(&actor.id)
            .fetch_one(self.pool())
            .await
            .unwrap()
    }
}

impl Drop for ContentSuite {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

/// A new empty directory under the system temporary directory, unique to one case.
fn fresh_scratch_dir(suite: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{suite}-{}", Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

static PEER: AtomicU32 = AtomicU32::new(1);

/// A synthetic client address no other request of this process uses.
fn next_peer() -> SocketAddr {
    let n = PEER.fetch_add(1, Ordering::Relaxed);
    let [_, b, c, d] = n.to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 41000))
}

/// One field of a multipart body; `file_name` makes it a file field.
pub struct MultipartPart<'a> {
    pub name: &'a str,
    pub file_name: Option<&'a str>,
    pub bytes: &'a [u8],
}

/// A `multipart/form-data` body of `parts`; answers its content type (with the boundary) and
/// bytes.
pub fn multipart_body(parts: &[MultipartPart<'_>]) -> (String, Vec<u8>) {
    let boundary = format!("content-suite-{}", Uuid::new_v4().simple());
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        let disposition = match part.file_name {
            Some(file_name) => format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{file_name}\"\r\n\
                 Content-Type: application/octet-stream\r\n\r\n",
                part.name
            ),
            None => format!(
                "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                part.name
            ),
        };
        body.extend_from_slice(disposition.as_bytes());
        body.extend_from_slice(part.bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// `len` bytes that open with `signature`, zero-padded.
fn signed_bytes(signature: &[u8], len: usize) -> Vec<u8> {
    assert!(len >= signature.len());
    let mut bytes = signature.to_vec();
    bytes.resize(len, 0);
    bytes
}

/// `len` bytes a PNG signature opens.
pub fn png_bytes(len: usize) -> Vec<u8> {
    signed_bytes(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A], len)
}

/// `len` bytes a JPEG start-of-image marker opens.
pub fn jpeg_bytes(len: usize) -> Vec<u8> {
    signed_bytes(&[0xFF, 0xD8, 0xFF, 0xE0], len)
}

/// `len` bytes a WebP RIFF header opens.
pub fn webp_bytes(len: usize) -> Vec<u8> {
    signed_bytes(b"RIFF\x24\x00\x00\x00WEBPVP8 ", len)
}

/// The sorted file names in `dir`; empty when it does not exist.
pub fn directory_entries(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Assert `body` is the error envelope `{error, details?}` of the content contract, and that a
/// content refusal code in `details` is one the contract names.
pub fn assert_envelope(body: &Value) {
    contract_support::assert_valid(CONTENT_CONTRACT, Some("ContentError"), body);
    if let Some(code) = body["details"]["code"].as_str()
        && matches!(code, "request_too_large" | "storage_unavailable")
    {
        contract_support::assert_valid(CONTENT_CONTRACT, Some("ContentRefusal"), &body["details"]);
    }
}

/// Assert a refusal: its status, the envelope, and no `details.code` other than `code`.
pub fn assert_refusal(status: StatusCode, body: &Value, expected: StatusCode, code: Option<&str>) {
    assert_eq!(status, expected, "{body}");
    assert_envelope(body);
    assert_eq!(body["details"]["code"].as_str(), code, "{body}");
}

/// A complete vehicle write body whose values carry `tag`.
pub fn vehicle_body(tag: &str) -> Value {
    json!({
        "name": format!("{tag} Leopard"),
        "faction": "BLUFOR",
        "armor_type": "Tank",
        "amphibious": "No",
        "primary_threat": "ATGM",
        "profile_image_url": "https://example.com/leopard.png",
    })
}

/// A JSON body of `len` bytes or more: `value` with a `padding` string that fills it.
pub fn oversized_json(mut value: Value, len: usize) -> Vec<u8> {
    value["padding"] = json!("a".repeat(len));
    value.to_string().into_bytes()
}

/// A uniquely named trigger that raises on `operation` of `table` for rows whose `column` equals
/// `value`, injecting a real storage failure into one actor's or one row's transaction only.
pub async fn inject_failure(
    pool: &PgPool,
    table: &str,
    operation: &str,
    column: &str,
    value: &str,
) -> String {
    let name = format!("content_failure_{}", Uuid::new_v4().simple());
    assert!(
        matches!(
            (table, column),
            ("audit_logs", "actor_id")
                | ("vehicle_databases", "id")
                | ("vehicle_databases", "created_by")
        ),
        "no failure injection is defined for {table}.{column}"
    );
    assert!(matches!(operation, "INSERT" | "UPDATE"));
    assert!(
        value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "the injected value is interpolated into SQL and must stay a bare token: {value}"
    );
    let sql = format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            IF NEW.{column}::text = '{value}' THEN RAISE EXCEPTION 'injected content failure'; END IF;
            RETURN NEW; END; $$;
         CREATE TRIGGER {name} BEFORE {operation} ON {table} FOR EACH ROW EXECUTE FUNCTION {name}();"
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(pool)
        .await
        .unwrap();
    name
}

/// Drops a trigger [`inject_failure`] installed on `table`.
pub async fn clear_failure(pool: &PgPool, name: &str, table: &str) {
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {name} ON {table}; DROP FUNCTION {name}();"
    )))
    .execute(pool)
    .await
    .unwrap();
}
