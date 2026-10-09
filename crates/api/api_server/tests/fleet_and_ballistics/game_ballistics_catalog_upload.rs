//! The game ballistics catalog upload (`POST /api/v1/ballistics-catalogs`) through the real HTTP
//! router: the committed vanilla pair is accepted, audited, served and immutable, and the upload
//! is refused to everyone but an administrator.
//!
//! The vanilla catalog version can be stored once per database, so one case owns the whole
//! accepted path (upload, audit line, duplicate, reads, immutability) on an isolated database. Answers are checked against
//! `ballistics-catalog.schema.json`.

use crate::{content_support, contract_support};

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use content_support::{Actor, ContentSuite};
use contract_support::assert_valid;

const SUITE: &str = "game_ballistics_catalog_upload";
const CATALOG_CONTRACT: &str = "ballistics-catalog.schema.json";
const UPLOAD_URI: &str = "/api/v1/ballistics-catalogs";
const VANILLA_DETAIL_URI: &str = "/api/v1/ballistics-catalogs/vanilla_mortars/versions/1";
/// SHA-256 of the committed catalog bytes, as the bundle's provenance records it.
const VANILLA_CATALOG_SHA256: &str =
    "24a68cc5e22b5d3dc62ec80d82d0e004916e1d300ac0a964d1660847e8f9fbde";
/// SHA-256 of the committed calibration bundle.
const VANILLA_CALIBRATION_SHA256: &str =
    "12be201bbdd22f1bfa72b3aead176cc53969abeca532e33ee8e7216b2fc2d3d0";
/// Forward-angle samples of the committed bundle that sit between native rows.
const VANILLA_FORWARD_SAMPLES_NOT_JUDGED: u64 = 15_427;

fn repository_file(relative: &str) -> Vec<u8> {
    let path = repository_root::find_repository_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("the repository root above the API package")
    .join(relative);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("read committed fixture {}: {error}", path.display()))
}

fn vanilla_catalog() -> Vec<u8> {
    repository_file("contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json")
}

fn vanilla_calibration() -> Vec<u8> {
    repository_file("contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json")
}

/// One multipart part: its name, declared content type, and bytes.
struct Part<'a> {
    name: &'a str,
    content_type: Option<&'a str>,
    bytes: &'a [u8],
}

impl<'a> Part<'a> {
    fn json(name: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            name,
            content_type: Some("application/json"),
            bytes,
        }
    }
}

/// A `multipart/form-data` body of `parts` with a fixed boundary.
fn multipart(parts: &[Part<'_>]) -> (String, Vec<u8>) {
    let boundary = "game-ballistics-catalog-upload-boundary";
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}.json\"\r\n",
                part.name, part.name
            )
            .as_bytes(),
        );
        if let Some(content_type) = part.content_type {
            body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
        }
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(part.bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn upload(
    suite: &ContentSuite,
    actor: Option<&Actor>,
    parts: &[Part<'_>],
) -> (StatusCode, Value) {
    let (content_type, body) = multipart(parts);
    suite
        .send(actor, "POST", UPLOAD_URI, Some(&content_type), body)
        .await
}

async fn upload_pair(
    suite: &ContentSuite,
    actor: Option<&Actor>,
    catalog: &[u8],
    calibration: &[u8],
) -> (StatusCode, Value) {
    upload(
        suite,
        actor,
        &[
            Part::json("catalog", catalog),
            Part::json("calibration", calibration),
        ],
    )
    .await
}

/// A response with its headers, for the reads whose caching headers are under test.
struct RawResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl RawResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }
}

static PEER: AtomicU32 = AtomicU32::new(1);

/// An anonymous `GET uri` from a peer no other request of this binary uses, with an optional
/// `If-None-Match`.
async fn anonymous_get(
    suite: &ContentSuite,
    uri: &str,
    if_none_match: Option<&str>,
) -> RawResponse {
    let mut request = Request::builder().method("GET").uri(uri);
    if let Some(etag) = if_none_match {
        request = request.header(header::IF_NONE_MATCH, etag);
    }
    let mut request = request.body(Body::empty()).expect("the request builds");
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((
            IpAddr::from([10, 200 ^ b, c, d]),
            42000,
        ))));
    let response = suite.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end")
        .to_vec();
    RawResponse {
        status,
        headers,
        body,
    }
}

fn assert_code(status: StatusCode, answer: &Value, expected: StatusCode, code: &str) {
    assert_eq!(status, expected, "{answer}");
    assert!(
        answer["error"].is_string(),
        "envelope has an error: {answer}"
    );
    assert_eq!(answer["details"]["code"], json!(code), "{answer}");
}

#[tokio::test]
async fn game_ballistics_vanilla_upload_is_accepted_audited_served_and_immutable() {
    // The vanilla catalog version is stored once per database, and the fire-mission module stores
    // it too, so this case runs on an isolated database.
    let suite = ContentSuite::isolated(SUITE, "vanilla_catalog_upload").await;
    let (catalog, calibration) = (vanilla_catalog(), vanilla_calibration());

    let (status, report) = upload_pair(&suite, Some(&suite.admin), &catalog, &calibration).await;
    assert_eq!(status, StatusCode::CREATED, "{report}");
    assert_valid(CATALOG_CONTRACT, Some("CatalogUploadReport"), &report);
    assert_eq!(report["accepted"], json!(true));
    assert_eq!(report["failures"], json!([]));
    assert!(report["cases"].as_u64().unwrap() > 0, "{report}");
    assert_eq!(
        report["forward_samples_not_judged"],
        json!(VANILLA_FORWARD_SAMPLES_NOT_JUDGED)
    );

    let (uploaded_by, calibration_sha, stored_report): (String, String, Value) = sqlx::query_as(
        "SELECT uploaded_by, calibration_sha256, validation_report FROM ballistics_catalogs \
         WHERE catalog_id = 'vanilla_mortars' AND catalog_version = 1",
    )
    .fetch_one(suite.pool())
    .await
    .unwrap();
    assert_eq!(uploaded_by, suite.admin.id);
    assert_eq!(calibration_sha, VANILLA_CALIBRATION_SHA256);
    assert_eq!(
        stored_report, report,
        "the stored report is the answered one"
    );

    let audits: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT action, target_type, target_id FROM audit_logs \
         WHERE actor_id = $1 AND action NOT LIKE 'auth.%'",
    )
    .bind(&suite.admin.id)
    .fetch_all(suite.pool())
    .await
    .unwrap();
    assert_eq!(
        audits,
        [(
            "ballistics_catalog.uploaded".to_owned(),
            "ballistics_catalog".to_owned(),
            "vanilla_mortars/1".to_owned()
        )]
    );

    let (status, answer) = upload_pair(&suite, Some(&suite.admin), &catalog, &calibration).await;
    assert_code(
        status,
        &answer,
        StatusCode::CONFLICT,
        "catalog_version_exists",
    );
    assert_eq!(
        suite.content_audits_by(&suite.admin).await,
        1,
        "a duplicate is not audited"
    );

    let (status, list) = suite.call(None, "GET", UPLOAD_URI, None).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_valid(CATALOG_CONTRACT, Some("BallisticsCatalogList"), &list);
    let summary = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["catalog_id"] == json!("vanilla_mortars"))
        .unwrap_or_else(|| panic!("the list carries the stored version: {list}"));
    assert_eq!(summary["catalog_version"], json!(1));
    assert_eq!(summary["catalog_sha256"], json!(VANILLA_CATALOG_SHA256));
    assert_eq!(summary["game_build"], json!("1.8.0.13"));

    assert_detail_is_immutably_cached(&suite, &catalog).await;
    assert_stored_version_is_immutable(&suite).await;
}

async fn assert_detail_is_immutably_cached(suite: &ContentSuite, catalog: &[u8]) {
    let etag = format!("\"{VANILLA_CATALOG_SHA256}\"");
    let response = anonymous_get(suite, VANILLA_DETAIL_URI, None).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.header("etag"), Some(etag.as_str()));
    assert_eq!(
        response.header("cache-control"),
        Some("public, max-age=31536000, immutable")
    );
    assert_eq!(response.header("content-type"), Some("application/json"));
    let served: Value = serde_json::from_slice(&response.body).expect("the body decodes as JSON");
    assert_valid(CATALOG_CONTRACT, None, &served);
    let committed: Value = serde_json::from_slice(catalog).expect("the body decodes as JSON");
    assert_eq!(served, committed, "the served document is the uploaded one");

    let revalidated = anonymous_get(suite, VANILLA_DETAIL_URI, Some(&etag)).await;
    assert_eq!(revalidated.status, StatusCode::NOT_MODIFIED);
    assert_eq!(revalidated.header("etag"), Some(etag.as_str()));
    assert!(revalidated.body.is_empty());
    let stale = anonymous_get(suite, VANILLA_DETAIL_URI, Some("\"other\"")).await;
    assert_eq!(stale.status, StatusCode::OK);

    for (uri, expected) in [
        (
            "/api/v1/ballistics-catalogs/vanilla_mortars/versions/2",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/ballistics-catalogs/unknown/versions/1",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/ballistics-catalogs/vanilla_mortars/versions/one",
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (status, answer) = suite.call(None, "GET", uri, None).await;
        assert_eq!(status, expected, "{uri}: {answer}");
        assert!(answer["error"].is_string(), "{uri}: {answer}");
    }
}

async fn assert_stored_version_is_immutable(suite: &ContentSuite) {
    for statement in [
        "UPDATE ballistics_catalogs SET title = 'Changed' WHERE catalog_id = 'vanilla_mortars'",
        "DELETE FROM ballistics_catalogs WHERE catalog_id = 'vanilla_mortars'",
    ] {
        let error = sqlx::query(statement)
            .execute(suite.pool())
            .await
            .expect_err(statement);
        assert!(
            error
                .to_string()
                .contains("ballistics catalogs are immutable"),
            "{statement}: {error}"
        );
    }
    for method in ["PUT", "PATCH", "DELETE"] {
        let (status, _) = suite
            .send(
                Some(&suite.admin),
                method,
                VANILLA_DETAIL_URI,
                None,
                Vec::new(),
            )
            .await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method}");
    }
}

#[tokio::test]
async fn game_ballistics_upload_is_administrator_only() {
    let suite = ContentSuite::new(SUITE).await;
    let (catalog, calibration) = (vanilla_catalog(), vanilla_calibration());
    let (status, answer) = upload_pair(&suite, Some(&suite.member), &catalog, &calibration).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    let (status, answer) = upload_pair(&suite, None, &catalog, &calibration).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{answer}");
    assert_eq!(suite.content_audits_by(&suite.member).await, 0);
}
