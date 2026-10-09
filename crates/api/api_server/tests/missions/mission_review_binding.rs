//! Immutable mission artifacts and the reviews that decide them, through the real HTTP routes:
//! every compilation input and the exact bytes are pinned, a review decides the submitted version
//! rather than a later draft, and the review workspace is limited to the author and
//! administrators.

use crate::mission_artifact_support;

use axum::http::{StatusCode, header};
use serde_json::Value;
use uuid::Uuid;

use mission_artifact_support::{MissionFixture, payload_with_role, sha256_hex, uuid_of};

const SUITE: &str = "mission_review_binding";

/// The canonical digest input, keys in sorted order, exactly as the artifact store builds it.
fn expected_digest(artifact: &Value) -> String {
    let canonical = format!(
        r#"{{"catalog_sha256":{},"compiler_version":{},"document_sha256":{},"metadata_sha256":{},"mission_version_id":{},"modpack_id":{},"modpack_version":{},"schema_version":{},"version_payload_sha256":{}}}"#,
        artifact["catalog_sha256"],
        artifact["compiler_version"],
        artifact["document_sha256"],
        artifact["metadata_sha256"],
        artifact["mission_version_id"],
        artifact.get("modpack_id").unwrap_or(&Value::Null),
        artifact.get("modpack_version").unwrap_or(&Value::Null),
        artifact["schema_version"],
        artifact["version_payload_sha256"],
    );
    sha256_hex(canonical.as_bytes())
}

fn is_sha256(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|text| text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[tokio::test]
async fn immutable_artifacts_pin_every_compilation_input_and_the_exact_bytes() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, version) = f.compilable_mission("Pinned inputs").await;
    let artifact_id = f.submit(mission).await;

    let (status, artifact) = f.artifact(&f.author, mission, artifact_id).await;
    assert_eq!(status, StatusCode::OK, "{artifact}");
    assert_eq!(uuid_of(&artifact["mission_version_id"]), version);
    let payload_digest: String = sqlx::query_scalar(
        "SELECT encode(sha256(convert_to(json_payload::text, 'UTF8')), 'hex') FROM mission_versions WHERE id = $1",
    )
    .bind(version)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(artifact["version_payload_sha256"], payload_digest.as_str());
    assert_eq!(artifact["metadata"]["title"], "Pinned inputs");
    assert_eq!(artifact["metadata"]["terrain"], "everon");
    for digest in [
        "metadata_sha256",
        "catalog_sha256",
        "document_sha256",
        "artifact_digest",
    ] {
        assert!(is_sha256(&artifact[digest]), "{digest}: {artifact}");
    }
    assert!(
        artifact["compiler_version"]
            .as_str()
            .is_some_and(|v| v.starts_with("website-map-engine ")),
        "{artifact}"
    );
    assert!(
        artifact["schema_version"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    assert_eq!(artifact["terrain"], "everon");
    assert_eq!(artifact["created_by"], f.author.id.as_str());
    assert!(artifact["diagnostics"].is_array());
    assert_eq!(
        artifact["artifact_digest"].as_str().unwrap(),
        expected_digest(&artifact),
        "the digest covers version, metadata, catalog, modpack, compiler, schema and bytes"
    );

    let (status, headers, bytes) = f
        .send(
            Some(&f.admin),
            "GET",
            &format!("/api/v1/missions/{mission}/artifacts/{artifact_id}/document"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    let document_digest = sha256_hex(&bytes);
    assert_eq!(artifact["document_sha256"], document_digest.as_str());
    assert_eq!(
        headers[header::ETAG].to_str().unwrap(),
        format!("\"{document_digest}\"")
    );
    assert_eq!(
        artifact["document_bytes"].as_u64().unwrap() as usize,
        bytes.len()
    );
    assert!(bytes.len() <= 8 * 1024 * 1024);
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(document["slots"][0]["uid"], "s1");
    assert_eq!(document["slots"][0]["role"], "SL");

    for statement in [
        "UPDATE mission_artifacts SET terrain = 'arland' WHERE id = $1",
        "DELETE FROM mission_artifacts WHERE id = $1",
    ] {
        let refused = sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(artifact_id)
            .execute(f.pool())
            .await
            .unwrap_err();
        assert!(
            refused
                .to_string()
                .contains("mission artifacts are immutable"),
            "{statement}: {refused}"
        );
    }
    let refused =
        sqlx::query("UPDATE mission_versions SET editor_notes = 'rewritten' WHERE id = $1")
            .bind(version)
            .execute(f.pool())
            .await
            .unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("mission versions are immutable"),
        "{refused}"
    );
}

#[tokio::test]
async fn approval_binding_review_decides_the_submitted_version_not_a_later_draft() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, submitted_version) = f.compilable_mission("Later draft").await;
    let artifact = f.submit(mission).await;
    let later = f.save(mission, "0.3.0", &payload_with_role("TL")).await;
    let (status, decided) = f.approve(mission, artifact, None).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_eq!(uuid_of(&decided["approved_artifact_id"]), artifact);
    assert_eq!(uuid_of(&decided["current_version_id"]), later);
    let (_, approved) = f.artifact(&f.admin, mission, artifact).await;
    assert_eq!(uuid_of(&approved["mission_version_id"]), submitted_version);
    let (_, _, bytes) = f
        .send(
            Some(&f.admin),
            "GET",
            &format!("/api/v1/missions/{mission}/artifacts/{artifact}/document"),
            None,
        )
        .await;
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        document["slots"][0]["role"], "SL",
        "the approved bytes predate the draft"
    );
}

#[tokio::test]
async fn review_workspace_is_limited_to_the_author_and_administrators() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Workspace access").await;
    let artifact = f.submit(mission).await;
    let (other_mission, _) = f.compilable_mission("Workspace other").await;
    let foreign = f.submit(other_mission).await;
    let outsider = f.account("outsider", "mission_maker").await;
    let enlisted = f.account("enlisted", "enlisted").await;
    let workspace =
        |artifact: &str| format!("/api/v1/missions/{mission}/artifacts/{artifact}/workspace");

    for reader in [&outsider, &enlisted] {
        let (status, _) = f
            .call(Some(reader), "GET", &workspace(&artifact.to_string()), None)
            .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "a pending mission is invisible to non-authors"
        );
    }
    let (status, _) = f
        .call(None, "GET", &workspace(&artifact.to_string()), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = f
        .call(
            Some(&f.admin),
            "GET",
            &workspace(&Uuid::new_v4().to_string()),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = f
        .call(
            Some(&f.admin),
            "GET",
            &workspace(&foreign.to_string()),
            None,
        )
        .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "an artifact belongs to its own mission"
    );
    let (status, _) = f
        .call(Some(&f.admin), "GET", &workspace("not-a-uuid"), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, decided) = f.approve(mission, artifact, None).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    for reader in [&outsider, &enlisted] {
        for route in ["", "/document", "/workspace"] {
            let (status, _) = f
                .call(
                    Some(reader),
                    "GET",
                    &format!("/api/v1/missions/{mission}/artifacts/{artifact}{route}"),
                    None,
                )
                .await;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "a live mission's artifacts stay with its author and reviewers: artifact{route}"
            );
        }
    }
}
