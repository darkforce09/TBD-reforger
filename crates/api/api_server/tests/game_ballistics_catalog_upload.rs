//! The game ballistics catalog routes through the real HTTP router: the administrator upload
//! (`POST /api/v1/ballistics-catalogs`) of the committed vanilla pair and of every refused
//! calibration variant, its tier, body limits and envelopes, and the public reads of the list
//! and of one stored version with its ETag.
//!
//! Every case needs `TEST_DATABASE_URL`; without it the suite fails with that cause. The
//! vanilla catalog version can be stored once per database, so one case owns the whole accepted
//! path (upload, audit line, duplicate, reads, immutability); the refused variants store nothing
//! and each run in their own case. Answers are checked against `ballistics-catalog.schema.json`.

mod common;
mod content_support;
mod contract_support;

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use api_operations::handlers::ballistics_catalogs::upload::{
    MAX_CALIBRATION_PART_BYTES, MAX_CATALOG_UPLOAD_BODY_BYTES,
};
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

fn negative_calibration(name: &str) -> Vec<u8> {
    repository_file(&format!(
        "contracts/fixtures/ballistics/vanilla_mortars.v1/negative/{name}.calibration.json"
    ))
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

fn failure_ids(report: &Value) -> Vec<String> {
    report["failures"]
        .as_array()
        .unwrap_or_else(|| panic!("report lists failures: {report}"))
        .iter()
        .map(|failure| {
            failure["case_id"]
                .as_str()
                .expect("the `case_id` field is a string")
                .to_owned()
        })
        .collect()
}

/// Uploads the vanilla catalog with one refused bundle; answers the 422 report.
async fn refused_report(suite: &ContentSuite, variant: &str) -> Value {
    let (status, answer) = upload_pair(
        suite,
        Some(&suite.admin),
        &vanilla_catalog(),
        &negative_calibration(variant),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{variant}: {answer}"
    );
    let report = &answer["details"];
    assert_valid(CATALOG_CONTRACT, Some("CatalogUploadReport"), report);
    assert_eq!(report["accepted"], json!(false), "{variant}");
    assert_eq!(
        suite.content_audits_by(&suite.admin).await,
        0,
        "{variant}: a refused upload writes no audit line"
    );
    report.clone()
}

#[tokio::test]
async fn game_ballistics_vanilla_upload_is_accepted_audited_served_and_immutable() {
    let suite = ContentSuite::new(SUITE).await;
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
async fn game_ballistics_skewed_native_row_is_refused_by_that_row() {
    let suite = ContentSuite::new(SUITE).await;
    let report = refused_report(&suite, "skewed_native_row").await;
    let ids = failure_ids(&report);
    assert_eq!(ids.len(), 1, "exactly the skewed row fails: {report}");
    assert!(ids[0].starts_with("native/m821/2.977/"), "{ids:?}");
}

#[tokio::test]
async fn game_ballistics_wrong_game_build_is_refused_by_provenance() {
    let suite = ContentSuite::new(SUITE).await;
    let report = refused_report(&suite, "wrong_game_build").await;
    let ids = failure_ids(&report);
    assert_eq!(ids.len(), 1, "{report}");
    assert!(ids[0].starts_with("provenance/"), "{ids:?}");
    assert!(
        report["failures"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("0.0.0.0"),
        "{report}"
    );
}

#[tokio::test]
async fn game_ballistics_stale_catalog_sha_is_refused_by_provenance() {
    let suite = ContentSuite::new(SUITE).await;
    let report = refused_report(&suite, "stale_catalog_sha").await;
    let ids = failure_ids(&report);
    assert_eq!(ids.len(), 1, "{report}");
    assert!(ids[0].starts_with("provenance/"), "{ids:?}");
    assert!(
        report["failures"][0]["reason"]
            .as_str()
            .unwrap()
            .contains(VANILLA_CATALOG_SHA256),
        "{report}"
    );
}

#[tokio::test]
async fn game_ballistics_missing_shell_is_refused_by_coverage_of_every_charge() {
    let suite = ContentSuite::new(SUITE).await;
    let report = refused_report(&suite, "missing_shell").await;
    let catalog: Value = serde_json::from_slice(&vanilla_catalog()).unwrap();
    let charges: Vec<u64> = catalog["shells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|shell| shell["shell_id"] == json!("s832s"))
        .unwrap()["charges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|charge| charge["rings"].as_u64().unwrap())
        .collect();
    let expected: Vec<String> = charges
        .iter()
        .flat_map(|rings| {
            [
                format!("coverage/s832s/{rings}"),
                format!("coverage/s832s/{rings}"),
            ]
        })
        .collect();
    assert_eq!(failure_ids(&report), expected, "{report}");
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

#[tokio::test]
async fn game_ballistics_malformed_uploads_answer_400_and_415() {
    let suite = ContentSuite::new(SUITE).await;
    let admin = Some(&suite.admin);
    let (catalog, calibration) = (vanilla_catalog(), vanilla_calibration());

    let (status, answer) = upload(&suite, admin, &[Part::json("catalog", &catalog)]).await;
    assert_code(status, &answer, StatusCode::BAD_REQUEST, "missing_part");
    let (status, answer) = upload(
        &suite,
        admin,
        &[
            Part::json("catalog", &catalog),
            Part::json("catalog", &catalog),
            Part::json("calibration", &calibration),
        ],
    )
    .await;
    assert_code(status, &answer, StatusCode::BAD_REQUEST, "repeated_part");

    let (status, answer) =
        upload_pair(&suite, admin, b"{\"schema_version\": 1}", &calibration).await;
    assert_code(
        status,
        &answer,
        StatusCode::BAD_REQUEST,
        "catalog_undecodable",
    );
    let (status, answer) = upload_pair(&suite, admin, &catalog, b"[]").await;
    assert_code(
        status,
        &answer,
        StatusCode::BAD_REQUEST,
        "calibration_undecodable",
    );

    let mut renamed: Value = serde_json::from_slice(&catalog).unwrap();
    renamed["catalog_id"] = json!("Vanilla Mortars");
    let renamed = serde_json::to_vec(&renamed).unwrap();
    let (status, answer) = upload_pair(&suite, admin, &renamed, &calibration).await;
    assert_code(
        status,
        &answer,
        StatusCode::BAD_REQUEST,
        "invalid_catalog_field",
    );
    assert!(
        answer["error"].as_str().unwrap().contains("catalog_id"),
        "{answer}"
    );

    let (status, answer) = upload(
        &suite,
        admin,
        &[
            Part::json("catalog", &catalog),
            Part {
                name: "calibration",
                content_type: Some("text/csv"),
                bytes: &calibration,
            },
        ],
    )
    .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE, "{answer}");
    let (status, answer) = suite
        .send(
            admin,
            "POST",
            UPLOAD_URI,
            Some("application/json"),
            catalog.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE, "{answer}");
    assert_eq!(suite.content_audits_by(&suite.admin).await, 0);
}

#[tokio::test]
async fn game_ballistics_body_and_part_limits_answer_413_just_above_them() {
    let suite = ContentSuite::new(SUITE).await;
    let admin = Some(&suite.admin);
    let catalog = vanilla_catalog();
    let calibration = vanilla_calibration();
    assert!(
        calibration.len() * 2 <= MAX_CALIBRATION_PART_BYTES,
        "the calibration cap keeps twice the committed bundle"
    );

    // A body of exactly the route limit passes the limit: its calibration part is at its cap
    // and not JSON, and a skipped padding part fills the rest.
    let at_cap = vec![b' '; MAX_CALIBRATION_PART_BYTES];
    let body_of = |padding: usize| {
        multipart(&[
            Part::json("catalog", &catalog),
            Part::json("calibration", &at_cap),
            Part {
                name: "padding",
                content_type: None,
                bytes: &vec![b'x'; padding],
            },
        ])
    };
    let (_, framed) = body_of(0);
    let padding = MAX_CATALOG_UPLOAD_BODY_BYTES - framed.len();
    let (content_type, at_limit) = body_of(padding);
    assert_eq!(at_limit.len(), MAX_CATALOG_UPLOAD_BODY_BYTES);
    let (status, answer) = suite
        .send(admin, "POST", UPLOAD_URI, Some(&content_type), at_limit)
        .await;
    assert_code(
        status,
        &answer,
        StatusCode::BAD_REQUEST,
        "calibration_undecodable",
    );

    let (content_type, over_limit) = body_of(padding + 1);
    assert_eq!(over_limit.len(), MAX_CATALOG_UPLOAD_BODY_BYTES + 1);
    let (status, answer) = suite
        .send(admin, "POST", UPLOAD_URI, Some(&content_type), over_limit)
        .await;
    assert_code(
        status,
        &answer,
        StatusCode::PAYLOAD_TOO_LARGE,
        "request_too_large",
    );

    let over_cap = vec![b' '; MAX_CALIBRATION_PART_BYTES + 1];
    let (status, answer) = upload_pair(&suite, admin, &catalog, &over_cap).await;
    assert_code(
        status,
        &answer,
        StatusCode::PAYLOAD_TOO_LARGE,
        "request_too_large",
    );
    assert_eq!(suite.content_audits_by(&suite.admin).await, 0);
}
