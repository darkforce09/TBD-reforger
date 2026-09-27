//! Content storage and request limits through the real HTTP router: the image upload
//! (`POST /api/v1/cms/uploads`), the JSON body limit and content type of the vehicle, wiki and
//! announcement writes, the announcement page order and page parameters, author stamping, and
//! the error envelope.
//!
//! Each case owns its administrator, member and upload directory, and requires the isolated
//! PostgreSQL harness. Upload answers are checked against `content-upload.schema.json`, and
//! every refusal against its `ContentError` envelope.
//!
//! The upload cases read the upload directory itself: an accepted image must sit there under its
//! public name, byte for byte, beside no staging file, and a refused one must leave the directory
//! as it was.

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;
use website_api::community_content::handlers::media_upload::MAX_UPLOAD_BYTES;

mod common;
mod content_support;
mod contract_support;

use content_support::{
    Actor, ContentSuite, JSON_BODY_LIMIT, MultipartPart, assert_envelope, assert_refusal,
    directory_entries, jpeg_bytes, oversized_json, png_bytes, vehicle_body, webp_bytes,
};
use contract_support::{assert_invalid, assert_valid};

const SUITE: &str = "content_storage";
const UPLOAD_CONTRACT: &str = "content-upload.schema.json";
const WIKI_CONTRACT: &str = "wiki-page.schema.json";
/// The multipart body limit of the upload route (6 MiB).
const UPLOAD_ROUTE_LIMIT: usize = 6 << 20;

fn wiki_slug() -> String {
    format!("cs-{}", &Uuid::new_v4().simple().to_string()[..12])
}

fn wiki_body(title: &str, base_revision: Option<i64>) -> Value {
    json!({
        "category": "general",
        "title": title,
        "icon": "",
        "nav_order": 3,
        "body_md": "Stay on the **net**.",
        "base_revision": base_revision,
    })
}

/// The stored file name of an accepted upload's `url`.
fn stored_name(answer: &Value) -> String {
    answer["url"]
        .as_str()
        .unwrap()
        .strip_prefix("/uploads/")
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn content_storage_upload_stores_the_file_whole_with_no_staging_leftovers() {
    let suite = ContentSuite::new(SUITE).await;
    let uploads = [
        ("map-overlay.png", png_bytes(2 << 20), "png"),
        ("PHOTO.JPEG", jpeg_bytes(4096), "jpeg"),
        ("snapshot.jpg", jpeg_bytes(64), "jpg"),
        ("banner.webp", webp_bytes(512), "webp"),
    ];
    let mut stored = Vec::new();
    for (file_name, bytes, extension) in uploads {
        let (status, answer) = suite
            .upload_file(Some(&suite.admin), file_name, &bytes)
            .await;
        assert_eq!(status, StatusCode::CREATED, "{file_name}: {answer}");
        assert_valid(UPLOAD_CONTRACT, Some("UploadResponse"), &answer);
        let name = stored_name(&answer);
        assert!(name.ends_with(&format!(".{extension}")), "{name}");

        let on_disk = std::fs::read(suite.upload_dir.join(&name)).unwrap();
        assert!(on_disk == bytes, "{file_name} is stored whole");
        let (status, served) = suite
            .send_bytes(
                Some(&suite.member),
                "GET",
                answer["url"].as_str().unwrap(),
                None,
                Vec::new(),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(served == bytes, "{file_name} is served whole");
        stored.push(name);
    }
    stored.sort();
    assert_eq!(
        directory_entries(&suite.upload_dir),
        stored,
        "the directory holds the stored files and no staging file"
    );
}

#[tokio::test]
async fn content_storage_file_over_5_mib_answers_413() {
    let suite = ContentSuite::new(SUITE).await;
    assert_eq!(MAX_UPLOAD_BYTES, 5 << 20);

    let (status, answer) = suite
        .upload_file(
            Some(&suite.admin),
            "at-limit.png",
            &png_bytes(MAX_UPLOAD_BYTES),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{answer}");
    let kept = stored_name(&answer);

    let (status, answer) = suite
        .upload_file(
            Some(&suite.admin),
            "over-limit.png",
            &png_bytes(MAX_UPLOAD_BYTES + 1),
        )
        .await;
    assert_refusal(
        status,
        &answer,
        StatusCode::PAYLOAD_TOO_LARGE,
        Some("request_too_large"),
    );
    assert_eq!(directory_entries(&suite.upload_dir), vec![kept]);
}

#[tokio::test]
async fn content_storage_body_over_the_route_limit_answers_413_not_400() {
    let suite = ContentSuite::new(SUITE).await;
    let padding = vec![b'p'; UPLOAD_ROUTE_LIMIT + (512 << 10)];
    let image = png_bytes(128);
    let (status, answer) = suite
        .upload(
            Some(&suite.admin),
            &[
                MultipartPart {
                    name: "note",
                    file_name: None,
                    bytes: &padding,
                },
                MultipartPart {
                    name: "file",
                    file_name: Some("small.png"),
                    bytes: &image,
                },
            ],
        )
        .await;
    assert_refusal(
        status,
        &answer,
        StatusCode::PAYLOAD_TOO_LARGE,
        Some("request_too_large"),
    );
    assert!(directory_entries(&suite.upload_dir).is_empty());
}

#[tokio::test]
async fn content_storage_wrong_extension_answers_415() {
    let suite = ContentSuite::new(SUITE).await;
    for file_name in [
        "animation.gif",
        "vector.svg",
        "no-extension",
        "image.png.html",
        "trailing-dot.",
    ] {
        let (status, answer) = suite
            .upload_file(Some(&suite.admin), file_name, &png_bytes(64))
            .await;
        assert_refusal(status, &answer, StatusCode::UNSUPPORTED_MEDIA_TYPE, None);
    }
    assert!(directory_entries(&suite.upload_dir).is_empty());
}

#[tokio::test]
async fn content_storage_mismatched_magic_bytes_answer_415() {
    let suite = ContentSuite::new(SUITE).await;
    let mut riff_not_webp = webp_bytes(64);
    riff_not_webp[8..12].copy_from_slice(b"AVI ");
    let cases: [(&str, Vec<u8>); 6] = [
        ("png-named.jpg", png_bytes(64)),
        ("jpeg-named.png", jpeg_bytes(64)),
        ("text.png", b"<svg onload=alert(1)>".to_vec()),
        ("empty.webp", Vec::new()),
        ("riff.webp", riff_not_webp),
        ("webp-named.jpeg", webp_bytes(64)),
    ];
    for (file_name, bytes) in cases {
        let (status, answer) = suite
            .upload_file(Some(&suite.admin), file_name, &bytes)
            .await;
        assert_refusal(status, &answer, StatusCode::UNSUPPORTED_MEDIA_TYPE, None);
    }
    assert!(directory_entries(&suite.upload_dir).is_empty());
}

#[tokio::test]
async fn content_storage_unwritable_upload_directory_answers_503() {
    let suite = ContentSuite::with_upload_dir_as_regular_file(SUITE).await;
    let before = std::fs::read(&suite.upload_dir).unwrap();

    let (status, answer) = suite
        .upload_file(Some(&suite.admin), "orders.png", &png_bytes(1024))
        .await;
    assert_refusal(
        status,
        &answer,
        StatusCode::SERVICE_UNAVAILABLE,
        Some("storage_unavailable"),
    );
    assert!(
        !answer["error"].as_str().unwrap().contains("os error"),
        "the io error stays in the log: {answer}"
    );
    assert_eq!(
        std::fs::read(&suite.upload_dir).unwrap(),
        before,
        "the regular file is untouched"
    );
    let upload_dir_name = suite
        .upload_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        directory_entries(suite.scratch()),
        vec![upload_dir_name],
        "no staging or partial file is left anywhere"
    );
}

#[tokio::test]
async fn content_storage_member_upload_answers_403_and_anonymous_401() {
    let suite = ContentSuite::new(SUITE).await;
    let bytes = png_bytes(256);
    let (status, answer) = suite
        .upload_file(Some(&suite.member), "member.png", &bytes)
        .await;
    assert_refusal(status, &answer, StatusCode::FORBIDDEN, None);
    let (status, answer) = suite.upload_file(None, "anonymous.png", &bytes).await;
    assert_refusal(status, &answer, StatusCode::UNAUTHORIZED, None);
    assert!(directory_entries(&suite.upload_dir).is_empty());
}

#[tokio::test]
async fn content_storage_missing_file_field_and_non_multipart_body_answer_400() {
    let suite = ContentSuite::new(SUITE).await;
    let bytes = png_bytes(64);
    let (status, answer) = suite
        .upload(
            Some(&suite.admin),
            &[MultipartPart {
                name: "image",
                file_name: Some("wrong-field.png"),
                bytes: &bytes,
            }],
        )
        .await;
    assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);

    let (status, answer) = suite
        .send(
            Some(&suite.admin),
            "POST",
            "/api/v1/cms/uploads",
            Some("application/json"),
            br#"{"file":"map.png"}"#.to_vec(),
        )
        .await;
    assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
    assert!(directory_entries(&suite.upload_dir).is_empty());
}

/// The JSON writes of the content routes, each as `(method, uri, accepted body)`.
fn json_writes(slug: &str) -> [(&'static str, String, Value); 3] {
    [
        (
            "PUT",
            format!("/api/v1/wiki/{slug}"),
            wiki_body("Comms", None),
        ),
        (
            "POST",
            "/api/v1/vehicle-database".to_owned(),
            vehicle_body("content-storage"),
        ),
        (
            "POST",
            "/api/v1/cms/announcements".to_owned(),
            json!({ "title": "Limits", "body": "Body" }),
        ),
    ]
}

/// Nothing the refused JSON writes could have created exists.
async fn assert_nothing_written(suite: &ContentSuite, slug: &str) {
    let (status, body) = suite
        .call(
            Some(&suite.member),
            "GET",
            &format!("/api/v1/wiki/{slug}"),
            None,
        )
        .await;
    assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    assert_eq!(suite.vehicles_created_by(&suite.admin).await, 0);
    let announcements: i64 =
        sqlx::query_scalar("SELECT count(*) FROM announcements WHERE author_id = $1")
            .bind(&suite.admin.id)
            .fetch_one(suite.pool())
            .await
            .unwrap();
    assert_eq!(announcements, 0);
    assert_eq!(suite.content_audits_by(&suite.admin).await, 0);
}

#[tokio::test]
async fn content_storage_json_bodies_over_the_limit_answer_413() {
    let suite = ContentSuite::new(SUITE).await;
    let slug = wiki_slug();
    for (method, uri, body) in json_writes(&slug) {
        let oversized = oversized_json(body, JSON_BODY_LIMIT);
        let (status, answer) = suite
            .send(
                Some(&suite.admin),
                method,
                &uri,
                Some("application/json"),
                oversized,
            )
            .await;
        assert_refusal(
            status,
            &answer,
            StatusCode::PAYLOAD_TOO_LARGE,
            Some("request_too_large"),
        );
    }
    assert_nothing_written(&suite, &slug).await;
}

#[tokio::test]
async fn content_storage_json_bodies_without_a_json_content_type_answer_415() {
    let suite = ContentSuite::new(SUITE).await;
    let slug = wiki_slug();
    for (method, uri, body) in json_writes(&slug) {
        for content_type in [None, Some("text/plain"), Some("multipart/form-data")] {
            let (status, answer) = suite
                .send(
                    Some(&suite.admin),
                    method,
                    &uri,
                    content_type,
                    body.to_string().into_bytes(),
                )
                .await;
            assert_refusal(status, &answer, StatusCode::UNSUPPORTED_MEDIA_TYPE, None);
        }
    }
    assert_nothing_written(&suite, &slug).await;
}

/// Seven pinned, published announcements sharing one `published_at` and `updated_at`; answers
/// their ids in the order every page must list them (`id DESC`).
async fn tied_announcements(suite: &ContentSuite) -> Vec<String> {
    let mut ids = Vec::new();
    for n in 0..7 {
        let (status, row) = suite
            .call(
                Some(&suite.admin),
                "POST",
                "/api/v1/cms/announcements",
                Some(json!({
                    "title": format!("Tied {n}"),
                    "body": "Same instant.",
                    "status": "published",
                    "is_pinned": true,
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{row}");
        ids.push(row["id"].as_str().unwrap().parse::<Uuid>().unwrap());
    }
    sqlx::query(
        "UPDATE announcements SET published_at = '2999-01-01T00:00:00Z', \
         updated_at = '2999-01-01T00:00:00Z' WHERE id = ANY($1)",
    )
    .bind(ids.as_slice())
    .execute(suite.pool())
    .await
    .unwrap();
    let mut ids: Vec<String> = ids.iter().map(Uuid::to_string).collect();
    ids.sort_unstable_by(|a, b| b.cmp(a));
    ids
}

/// The ids of the first `pages` pages of `path` at `limit` rows per page.
async fn walk_pages(
    suite: &ContentSuite,
    path: &str,
    limit: usize,
    pages: usize,
) -> (Vec<String>, i64) {
    let mut ids = Vec::new();
    let mut totals = Vec::new();
    for page in 0..pages {
        let offset = page * limit;
        let (status, body) = suite
            .call(
                Some(&suite.admin),
                "GET",
                &format!("{path}?limit={limit}&offset={offset}"),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["limit"], limit, "{body}");
        assert_eq!(body["offset"], offset, "{body}");
        let rows = body["data"].as_array().unwrap();
        assert!(rows.len() <= limit);
        ids.extend(
            rows.iter()
                .map(|row| row["id"].as_str().unwrap().to_owned()),
        );
        totals.push(body["total"].as_i64().unwrap());
    }
    assert!(
        totals.windows(2).all(|pair| pair[0] == pair[1]),
        "{totals:?}"
    );
    (ids, totals[0])
}

#[tokio::test]
async fn content_storage_announcement_pages_keep_a_stable_complete_order() {
    let suite = ContentSuite::new(SUITE).await;
    let expected = tied_announcements(&suite).await;

    for path in ["/api/v1/announcements", "/api/v1/cms/announcements"] {
        let (first_walk, total) = walk_pages(&suite, path, 3, 3).await;
        assert!(total >= 7, "{path}: {total}");
        assert_eq!(
            first_walk[..7],
            expected[..],
            "{path}: tied rows page by id DESC, each exactly once"
        );
        let (second_walk, _) = walk_pages(&suite, path, 3, 3).await;
        assert_eq!(first_walk, second_walk, "{path}: the order is stable");
        let (one_page, _) = walk_pages(&suite, path, 7, 1).await;
        assert_eq!(one_page, expected, "{path}: pages agree with one page");
    }
}

#[tokio::test]
async fn content_storage_announcement_page_limit_is_bounded() {
    let suite = ContentSuite::new(SUITE).await;
    for path in ["/api/v1/announcements", "/api/v1/cms/announcements"] {
        for (query, limit, offset) in [
            ("limit=100", 100, 0),
            ("limit=101", 20, 0),
            ("limit=100000", 20, 0),
            ("limit=0", 20, 0),
            ("limit=-5&offset=-3", 20, 0),
            ("", 20, 0),
            ("limit=1&offset=2", 1, 2),
        ] {
            let (status, body) = suite
                .call(Some(&suite.admin), "GET", &format!("{path}?{query}"), None)
                .await;
            assert_eq!(status, StatusCode::OK, "{path}?{query}: {body}");
            assert_eq!(body["limit"], limit, "{path}?{query}: {body}");
            assert_eq!(body["offset"], offset, "{path}?{query}: {body}");
            assert!(body["data"].as_array().unwrap().len() <= limit);
        }
    }
}

/// A `limit` or `offset` that is not an integer answers 400 in the `{error, details?}` envelope
/// on both announcement pages, and the message names the refused parameter.
#[tokio::test]
async fn content_storage_non_numeric_announcement_page_values_answer_the_error_envelope() {
    let suite = ContentSuite::new(SUITE).await;
    for path in ["/api/v1/announcements", "/api/v1/cms/announcements"] {
        for (query, parameter) in [
            ("limit=abc", "limit"),
            ("limit=1.5", "limit"),
            ("limit=", "limit"),
            ("offset=ten", "offset"),
            ("limit=5&offset=-x", "offset"),
        ] {
            let (status, body) = suite
                .call(Some(&suite.admin), "GET", &format!("{path}?{query}"), None)
                .await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{path}?{query}: {body}");
            assert_envelope(&body);
            assert!(
                body["error"]
                    .as_str()
                    .is_some_and(|message| message.contains(parameter)),
                "{path}?{query}: the message names `{parameter}`: {body}"
            );
        }
    }
}

#[tokio::test]
async fn content_storage_author_is_stamped_from_the_caller() {
    let suite = ContentSuite::new(SUITE).await;
    let editor = suite.account("editor", "admin").await;

    let (status, announcement) = suite
        .call(
            Some(&suite.admin),
            "POST",
            "/api/v1/cms/announcements",
            Some(json!({ "title": "Orders", "body": "Muster", "author_id": editor.id })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{announcement}");
    assert_eq!(announcement["author_id"], suite.admin.id.as_str());
    let id = announcement["id"].as_str().unwrap();
    let (status, edited) = suite
        .call(
            Some(&editor),
            "PATCH",
            &format!("/api/v1/cms/announcements/{id}"),
            Some(json!({ "title": "Orders, revised", "author_id": editor.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(edited["title"], "Orders, revised");
    assert_eq!(edited["author_id"], suite.admin.id.as_str());

    let slug = wiki_slug();
    let page_uri = format!("/api/v1/wiki/{slug}");
    let mut claimed = wiki_body("Radio", None);
    claimed["updated_by"] = json!(editor.id);
    assert_invalid(WIKI_CONTRACT, Some("WikiSaveRequest"), &claimed);
    let (status, answer) = suite
        .call(Some(&suite.admin), "PUT", &page_uri, Some(claimed))
        .await;
    assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
    let (status, created) = suite
        .call(
            Some(&suite.admin),
            "PUT",
            &page_uri,
            Some(wiki_body("Radio", None)),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_valid(WIKI_CONTRACT, Some("WikiArticle"), &created);
    assert_eq!(created["updated_by"], suite.admin.id.as_str());
    let (status, saved) = suite
        .call(
            Some(&editor),
            "PUT",
            &page_uri,
            Some(wiki_body("Radio", Some(1))),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["updated_by"], editor.id.as_str());
    for (revision, author) in [(1, &suite.admin), (2, &editor)] {
        let (status, stored) = suite
            .call(
                Some(&suite.member),
                "GET",
                &format!("{page_uri}/revisions/{revision}"),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{stored}");
        assert_valid(WIKI_CONTRACT, Some("WikiRevision"), &stored);
        assert_eq!(stored["author_id"], author.id.as_str());
    }

    let mut claimed = vehicle_body("stamped");
    claimed["created_by"] = json!(editor.id);
    let (status, answer) = suite
        .call(
            Some(&suite.admin),
            "POST",
            "/api/v1/vehicle-database",
            Some(claimed),
        )
        .await;
    assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
    let (status, vehicle) = suite
        .call(
            Some(&suite.admin),
            "POST",
            "/api/v1/vehicle-database",
            Some(vehicle_body("stamped")),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{vehicle}");
    assert_eq!(suite.vehicles_created_by(&suite.admin).await, 1);
    assert_eq!(suite.vehicles_created_by(&editor).await, 0);
}

/// One refused JSON request: `(label, caller, method, uri, body, expected status)`.
type JsonRefusal<'a> = (
    &'static str,
    Option<&'a Actor>,
    &'static str,
    String,
    Option<Value>,
    StatusCode,
);

#[tokio::test]
async fn content_storage_every_refusal_uses_the_error_envelope() {
    let suite = ContentSuite::new(SUITE).await;
    let slug = wiki_slug();
    let (status, page) = suite
        .call(
            Some(&suite.admin),
            "PUT",
            &format!("/api/v1/wiki/{slug}"),
            Some(wiki_body("Envelope", None)),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{page}");
    let missing = Uuid::new_v4();
    let admin = Some(&suite.admin);
    let member = Some(&suite.member);

    let json_refusals: Vec<JsonRefusal> = vec![
        (
            "wiki anonymous",
            None,
            "PUT",
            format!("/api/v1/wiki/{slug}"),
            Some(wiki_body("x", Some(1))),
            StatusCode::UNAUTHORIZED,
        ),
        (
            "wiki member",
            member,
            "PUT",
            format!("/api/v1/wiki/{slug}"),
            Some(wiki_body("x", Some(1))),
            StatusCode::FORBIDDEN,
        ),
        (
            "wiki bad slug",
            admin,
            "PUT",
            "/api/v1/wiki/Bad_Slug".to_owned(),
            Some(wiki_body("x", None)),
            StatusCode::BAD_REQUEST,
        ),
        (
            "wiki stale base",
            admin,
            "PUT",
            format!("/api/v1/wiki/{slug}"),
            Some(wiki_body("x", Some(7))),
            StatusCode::CONFLICT,
        ),
        (
            "wiki unknown page",
            member,
            "GET",
            format!("/api/v1/wiki/{}", wiki_slug()),
            None,
            StatusCode::NOT_FOUND,
        ),
        (
            "vehicle unknown",
            member,
            "GET",
            format!("/api/v1/vehicle-database/{missing}"),
            None,
            StatusCode::NOT_FOUND,
        ),
        (
            "vehicle bad id",
            admin,
            "DELETE",
            "/api/v1/vehicle-database/x".to_owned(),
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            "vehicle blank",
            admin,
            "POST",
            "/api/v1/vehicle-database".to_owned(),
            Some(json!({ "name": " ", "faction": "a", "armor_type": "b" })),
            StatusCode::BAD_REQUEST,
        ),
        (
            "announcement blank",
            admin,
            "POST",
            "/api/v1/cms/announcements".to_owned(),
            Some(json!({ "title": " ", "body": "b" })),
            StatusCode::BAD_REQUEST,
        ),
        (
            "announcement member",
            member,
            "POST",
            "/api/v1/cms/announcements".to_owned(),
            Some(json!({ "title": "t", "body": "b" })),
            StatusCode::FORBIDDEN,
        ),
        (
            "announcement anonymous",
            None,
            "GET",
            "/api/v1/announcements".to_owned(),
            None,
            StatusCode::UNAUTHORIZED,
        ),
        (
            "announcement bad id",
            member,
            "GET",
            "/api/v1/announcements/x".to_owned(),
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            "announcement unknown",
            admin,
            "PATCH",
            format!("/api/v1/cms/announcements/{missing}"),
            Some(json!({ "title": "t" })),
            StatusCode::NOT_FOUND,
        ),
        (
            "announcement bad status",
            admin,
            "POST",
            "/api/v1/cms/announcements".to_owned(),
            Some(json!({ "title": "t", "body": "b", "status": "live" })),
            StatusCode::BAD_REQUEST,
        ),
    ];
    for (label, actor, method, uri, body, expected) in json_refusals {
        let (status, answer) = suite.call(actor, method, &uri, body).await;
        assert_eq!(status, expected, "{label}: {answer}");
        assert_envelope(&answer);
    }

    for (label, content_type, bytes, expected) in [
        (
            "malformed wiki JSON",
            Some("application/json"),
            b"{\"title\":".to_vec(),
            StatusCode::BAD_REQUEST,
        ),
        (
            "wiki without content type",
            None,
            wiki_body("x", Some(1)).to_string().into_bytes(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ] {
        let (status, answer) = suite
            .send(
                admin,
                "PUT",
                &format!("/api/v1/wiki/{slug}"),
                content_type,
                bytes,
            )
            .await;
        assert_eq!(status, expected, "{label}: {answer}");
        assert_envelope(&answer);
    }

    let bytes = png_bytes(64);
    for (label, actor, file_name, expected) in [
        ("upload anonymous", None, "a.png", StatusCode::UNAUTHORIZED),
        ("upload member", member, "a.png", StatusCode::FORBIDDEN),
        (
            "upload extension",
            admin,
            "a.bmp",
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            "upload signature",
            admin,
            "a.webp",
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ] {
        let (status, answer) = suite.upload_file(actor, file_name, &bytes).await;
        assert_eq!(status, expected, "{label}: {answer}");
        assert_envelope(&answer);
    }
    assert!(directory_entries(&suite.upload_dir).is_empty());
}
