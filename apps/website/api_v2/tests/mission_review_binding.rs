//! Immutable mission artifacts and the reviews that decide them, through the real HTTP routes:
//! every compilation input and the exact bytes are pinned, a decision names exactly the artifact
//! under review, rejection feedback and conditions commit with the decision, the review thread
//! carries author, version and artifact context, and the review workspace opens exactly the
//! reviewed version.

mod common;
mod mission_artifact_support;

use axum::http::{StatusCode, header};
use serde_json::{Value, json};
use uuid::Uuid;

use mission_artifact_support::{
    MissionFixture, clear_failure, inject_failure, payload_with_role, refusal_code, sha256_hex,
    uuid_of,
};

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
async fn immutable_artifacts_identical_inputs_share_one_artifact_and_each_input_change_compiles_another()
 {
    let f = MissionFixture::new(SUITE).await;
    let (mission, version) = f.compilable_mission("Input changes").await;
    let first = f.submit(mission).await;
    let (status, _) = f.reject(mission, first, "Rework the insertion").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        f.submit(mission).await,
        first,
        "identical inputs reuse the artifact"
    );
    let (_, original) = f.artifact(&f.author, mission, first).await;

    // Metadata the compiler reads.
    f.reject(mission, first, "Retitle it").await;
    let (status, body) = f
        .call(
            Some(&f.author),
            "PATCH",
            &format!("/api/v1/missions/{mission}"),
            Some(json!({ "title": "Input changes, retitled" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let retitled = f.submit(mission).await;
    assert_ne!(retitled, first);
    let (_, retitled_artifact) = f.artifact(&f.author, mission, retitled).await;
    assert_ne!(
        retitled_artifact["metadata_sha256"],
        original["metadata_sha256"]
    );
    assert_eq!(
        retitled_artifact["version_payload_sha256"],
        original["version_payload_sha256"]
    );
    assert_eq!(uuid_of(&retitled_artifact["mission_version_id"]), version);

    // The version payload.
    f.reject(mission, retitled, "Change the lead role").await;
    let second_version = f.save(mission, "0.3.0", &payload_with_role("TL")).await;
    let revised = f.submit(mission).await;
    let (_, revised_artifact) = f.artifact(&f.author, mission, revised).await;
    assert_eq!(
        uuid_of(&revised_artifact["mission_version_id"]),
        second_version
    );
    assert_ne!(
        revised_artifact["version_payload_sha256"],
        retitled_artifact["version_payload_sha256"]
    );
    assert_ne!(
        revised_artifact["document_sha256"],
        retitled_artifact["document_sha256"]
    );

    // The current modpack's catalog.
    f.reject(mission, revised, "Recompile against the new modpack")
        .await;
    let mut transaction = f.pool().begin().await.unwrap();
    sqlx::query("UPDATE modpacks SET is_current = false WHERE is_current")
        .execute(&mut *transaction)
        .await
        .unwrap();
    let modpack: Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, is_current, created_at)
         VALUES ('Review catalog', '4.2.0', 0, true, now()) RETURNING id",
    )
    .fetch_one(&mut *transaction)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO registry_items (modpack_id, resource_name, display_name, category, kind, created_at, updated_at)
         VALUES ($1, '{0000000000000001}Prefabs/Review/Crate.et', 'Review crate', 'supply', 'item', now(), now())",
    )
    .bind(modpack)
    .execute(&mut *transaction)
    .await
    .unwrap();
    transaction.commit().await.unwrap();
    let recatalogued = f.submit(mission).await;
    let (_, recatalogued_artifact) = f.artifact(&f.author, mission, recatalogued).await;
    assert_ne!(recatalogued, revised);
    assert_ne!(
        recatalogued_artifact["catalog_sha256"],
        revised_artifact["catalog_sha256"]
    );
    assert_eq!(uuid_of(&recatalogued_artifact["modpack_id"]), modpack);
    assert_eq!(recatalogued_artifact["modpack_version"], "4.2.0");
    assert_eq!(
        recatalogued_artifact["document_sha256"], revised_artifact["document_sha256"],
        "the catalog is an input even when the bytes it produces are unchanged"
    );
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_artifacts WHERE mission_id = $1",
            mission
        )
        .await,
        4
    );
}

#[tokio::test]
async fn immutable_artifacts_document_over_the_8_mib_limit_is_refused_before_storage() {
    let f = MissionFixture::new(SUITE).await;
    let mission = f.create_mission("Oversized").await;
    let role = "R".repeat(8 * 1024 * 1024 + 1);
    let (status, body) = f
        .save_version(&f.author, mission, "0.2.0", &payload_with_role(&role))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let (status, body) = f.submit_as(&f.author, mission).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(refusal_code(&body), "DOCUMENT_CONTRACT_VIOLATION");
    assert!(
        body["details"]["findings"][0]
            .as_str()
            .is_some_and(|finding| finding.contains("MISSION_FILE_MAX_BYTES")),
        "{body}"
    );
    assert_eq!(f.mission(&f.author, mission).await["status"], "draft");
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_artifacts WHERE mission_id = $1",
            mission
        )
        .await,
        0
    );
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_reviews WHERE mission_id = $1",
            mission
        )
        .await,
        0
    );
}

#[tokio::test]
async fn immutable_artifacts_submission_failure_leaves_no_artifact_or_review_and_retries_once() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Submission rollback").await;
    let failure = inject_failure(
        f.pool(),
        "audit_logs",
        "INSERT",
        "target_id",
        &mission.to_string(),
    )
    .await;
    let (status, body) = f.submit_as(&f.author, mission).await;
    clear_failure(f.pool(), &failure, "audit_logs").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(f.mission(&f.author, mission).await["status"], "draft");
    for table in ["mission_artifacts", "mission_reviews"] {
        assert_eq!(
            f.count(
                &format!("SELECT count(*) FROM {table} WHERE mission_id = $1"),
                mission
            )
            .await,
            0,
            "{table} rolls back with the audit record"
        );
    }
    f.submit(mission).await;
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_artifacts WHERE mission_id = $1",
            mission
        )
        .await,
        1
    );
    assert_eq!(f.audits("mission.submit", mission).await, 1);
}

#[tokio::test]
async fn immutable_artifacts_concurrent_submissions_open_one_review() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Concurrent submit").await;
    let (first, second) = tokio::join!(
        f.submit_as(&f.author, mission),
        f.submit_as(&f.admin, mission)
    );
    let mut statuses = [first.0, second.0];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CONFLICT],
        "{first:?} {second:?}"
    );
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_reviews WHERE mission_id = $1",
            mission
        )
        .await,
        1
    );
    assert_eq!(f.audits("mission.submit", mission).await, 1);
}

#[tokio::test]
async fn approval_binding_stale_decision_is_refused_with_the_artifact_under_review() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Stale decision").await;
    let first = f.submit(mission).await;
    f.reject(mission, first, "Move the spawn").await;
    f.save(mission, "0.3.0", &payload_with_role("TL")).await;
    let second = f.submit(mission).await;
    assert_ne!(first, second);

    let (status, body) = f.approve(mission, first, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal_code(&body), "REVIEWED_ARTIFACT_CHANGED");
    assert_eq!(uuid_of(&body["details"]["artifact_id"]), second);
    let pending = f.mission(&f.author, mission).await;
    assert_eq!(pending["status"], "pending_approval");
    assert!(pending.get("approved_artifact_id").is_none(), "{pending}");

    let approve = format!("/api/v1/approvals/{mission}/approve");
    for (body, why) in [
        (None, "no body"),
        (Some(json!({})), "no artifact"),
        (
            Some(json!({ "artifact_id": "not-a-uuid" })),
            "malformed artifact",
        ),
        (
            Some(json!({ "artifact_id": second, "extra": true })),
            "unknown field",
        ),
    ] {
        let (status, response) = f.call(Some(&f.admin), "POST", &approve, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{why}: {response}");
    }
    let (status, body) = f
        .call(
            Some(&f.author),
            "POST",
            &approve,
            Some(json!({ "artifact_id": second })),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "authors cannot decide: {body}"
    );

    let (status, decided) = f.approve(mission, second, None).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_eq!(decided["status"], "live");
    assert_eq!(uuid_of(&decided["approved_artifact_id"]), second);
    let (_, history) = f.reviews(&f.author, mission).await;
    assert_eq!(history["reviews"][0]["state"], "approved");
    assert_eq!(uuid_of(&history["reviews"][0]["artifact_id"]), second);
    assert_eq!(history["reviews"][0]["decided_by"], f.admin.id.as_str());
    assert_eq!(history["reviews"][1]["state"], "rejected");
    let (status, body) = f.approve(mission, second, None).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a decided review is final: {body}"
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
async fn approval_binding_conditional_approval_commits_conditions_with_the_decision() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, version) = f.compilable_mission("Conditional").await;
    let artifact = f.submit(mission).await;
    let (status, decided) = f
        .approve(
            mission,
            artifact,
            Some("  Night rotation only; cap at 40 players  "),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_eq!(decided["status"], "live");
    let (status, history) = f.reviews(&f.author, mission).await;
    assert_eq!(status, StatusCode::OK, "{history}");
    let review = &history["reviews"][0];
    assert_eq!(review["state"], "approved_with_conditions");
    let conditions = &history["comments"][0];
    assert_eq!(conditions["kind"], "approval_conditions");
    assert_eq!(conditions["body"], "Night rotation only; cap at 40 players");
    assert_eq!(conditions["review_id"], review["id"]);
    assert_eq!(uuid_of(&conditions["artifact_id"]), artifact);
    assert_eq!(uuid_of(&conditions["mission_version_id"]), version);
    assert_eq!(conditions["author_id"], f.admin.id.as_str());
    assert_eq!(f.audits("mission.approve", mission).await, 1);
}

#[tokio::test]
async fn approval_binding_legacy_pending_mission_is_decided_only_after_resubmission() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Legacy pending").await;
    sqlx::query("UPDATE missions SET status = 'pending_approval' WHERE id = $1")
        .bind(mission)
        .execute(f.pool())
        .await
        .unwrap();
    let row = f
        .queue_row(mission)
        .await
        .expect("legacy submissions stay queued");
    assert!(row.get("artifact_id").is_none(), "{row}");
    let (status, body) = f.approve(mission, Uuid::new_v4(), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal_code(&body), "NO_PENDING_REVIEW");

    let artifact = f.submit(mission).await;
    let (status, body) = f.submit_as(&f.author, mission).await;
    assert_eq!(status, StatusCode::CONFLICT, "one review at a time: {body}");
    let (status, decided) = f.approve(mission, artifact, None).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_eq!(uuid_of(&decided["approved_artifact_id"]), artifact);
}

#[tokio::test]
async fn approval_binding_decision_failure_rolls_back_and_retries_once() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Decision rollback").await;
    let artifact = f.submit(mission).await;
    let failure = inject_failure(
        f.pool(),
        "audit_logs",
        "INSERT",
        "target_id",
        &mission.to_string(),
    )
    .await;
    let (status, body) = f.approve(mission, artifact, Some("Daylight only")).await;
    clear_failure(f.pool(), &failure, "audit_logs").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    let pending = f.mission(&f.author, mission).await;
    assert_eq!(pending["status"], "pending_approval");
    assert!(pending.get("approved_artifact_id").is_none(), "{pending}");
    assert_eq!(f.pending_artifact(mission).await, Some(artifact));
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_review_comments WHERE mission_id = $1",
            mission
        )
        .await,
        0
    );

    let (status, decided) = f.approve(mission, artifact, Some("Daylight only")).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_eq!(f.audits("mission.approve", mission).await, 1);
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_review_comments WHERE mission_id = $1",
            mission
        )
        .await,
        1
    );
}

#[tokio::test]
async fn approval_binding_concurrent_decisions_decide_once() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, _) = f.compilable_mission("Concurrent decisions").await;
    let artifact = f.submit(mission).await;
    let (approved, rejected) = tokio::join!(
        f.approve(mission, artifact, None),
        f.reject(mission, artifact, "Needs another pass")
    );
    let mut statuses = [approved.0, rejected.0];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CONFLICT],
        "{approved:?} {rejected:?}"
    );
    let decided = f
        .count(
            "SELECT count(*) FROM mission_reviews WHERE mission_id = $1 AND state <> 'pending'",
            mission,
        )
        .await;
    assert_eq!(decided, 1);
    assert_eq!(
        f.audits("mission.approve", mission).await + f.audits("mission.reject", mission).await,
        1
    );
}

#[tokio::test]
async fn review_comments_rejection_reason_and_state_change_commit_together() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, version) = f.compilable_mission("Rejection feedback").await;
    let artifact = f.submit(mission).await;
    let failure = inject_failure(
        f.pool(),
        "mission_review_comments",
        "INSERT",
        "mission_id",
        &mission.to_string(),
    )
    .await;
    let (status, body) = f
        .reject(mission, artifact, "Objective markers overlap the spawn")
        .await;
    clear_failure(f.pool(), &failure, "mission_review_comments").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    let pending = f.mission(&f.author, mission).await;
    assert_eq!(pending["status"], "pending_approval");
    assert!(pending.get("rejection_reason").is_none(), "{pending}");
    assert_eq!(f.pending_artifact(mission).await, Some(artifact));

    let (status, rejected) = f
        .reject(mission, artifact, "  Objective markers overlap the spawn ")
        .await;
    assert_eq!(status, StatusCode::OK, "{rejected}");
    assert_eq!(rejected["status"], "rejected");
    assert_eq!(
        rejected["rejection_reason"],
        "Objective markers overlap the spawn"
    );
    assert_eq!(rejected["reviewed_by"], f.admin.id.as_str());
    let (_, history) = f.reviews(&f.author, mission).await;
    let review = &history["reviews"][0];
    assert_eq!(review["state"], "rejected");
    let feedback = &history["comments"][0];
    assert_eq!(feedback["kind"], "rejection");
    assert_eq!(feedback["body"], "Objective markers overlap the spawn");
    assert_eq!(feedback["review_id"], review["id"]);
    assert_eq!(uuid_of(&feedback["artifact_id"]), artifact);
    assert_eq!(uuid_of(&feedback["mission_version_id"]), version);
    assert_eq!(feedback["author_id"], f.admin.id.as_str());
    assert_eq!(f.audits("mission.reject", mission).await, 1);

    let (status, body) = f.reject(mission, artifact, "Again").await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let resubmitted = f.submit(mission).await;
    let (status, body) = f.reject(mission, resubmitted, "   ").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a blank reason says nothing: {body}"
    );
    let (status, body) = f.reject(mission, resubmitted, &"x".repeat(8001)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(f.pending_artifact(mission).await, Some(resubmitted));
}

#[tokio::test]
async fn review_comments_thread_records_author_version_and_artifact() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, version) = f.compilable_mission("Review thread").await;
    let artifact = f.submit(mission).await;
    let comments = format!("/api/v1/missions/{mission}/review-comments");

    let (status, first) = f
        .call(
            Some(&f.author),
            "POST",
            &comments,
            Some(json!({ "body": " Ready for a look " })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["kind"], "comment");
    assert_eq!(first["body"], "Ready for a look");
    assert_eq!(first["author_id"], f.author.id.as_str());
    assert_eq!(first["author_name"], "Integration Test");
    assert!(first.get("artifact_id").is_none() && first.get("mission_version_id").is_none());

    let (status, second) = f
        .call(
            Some(&f.admin),
            "POST",
            &comments,
            Some(json!({ "body": "Check the second squad's radios", "artifact_id": artifact })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    assert_eq!(uuid_of(&second["artifact_id"]), artifact);
    assert_eq!(uuid_of(&second["mission_version_id"]), version);

    let (status, history) = f.reviews(&f.author, mission).await;
    assert_eq!(status, StatusCode::OK, "{history}");
    let thread: Vec<&Value> = history["comments"].as_array().unwrap().iter().collect();
    assert_eq!(thread.len(), 2);
    assert_eq!(thread[0]["id"], first["id"]);
    assert_eq!(thread[1]["id"], second["id"]);
    assert_eq!(history["reviews"][0]["state"], "pending");
    assert_eq!(f.audits("mission.review_comment", mission).await, 2);

    let (other_mission, _) = f.compilable_mission("Another mission").await;
    let foreign_artifact = f.submit(other_mission).await;
    let refusals = [
        (json!({ "body": "   " }), "blank body"),
        (json!({ "body": "x".repeat(8001) }), "over 8000 bytes"),
        (
            json!({ "body": "wrong mission", "artifact_id": foreign_artifact }),
            "foreign artifact",
        ),
        (
            json!({ "body": "extra", "kind": "rejection" }),
            "unknown field",
        ),
    ];
    for (body, why) in refusals {
        let (status, response) = f.call(Some(&f.author), "POST", &comments, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{why}: {response}");
    }
    let outsider = f.account("outsider", "mission_maker").await;
    let enlisted = f.account("enlisted", "enlisted").await;
    let hello = || Some(json!({ "body": "hello" }));
    // A pending mission is invisible to everyone but its author and administrators.
    let (status, _) = f.call(Some(&outsider), "POST", &comments, hello()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for reader in [&outsider, &enlisted] {
        let (status, _) = f.reviews(reader, mission).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let (status, _) = f
        .call(
            None,
            "GET",
            &format!("/api/v1/missions/{mission}/reviews"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // A live mission is visible, but its review thread stays with its author and reviewers.
    let (status, decided) = f.approve(mission, artifact, None).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    let (status, _) = f.call(Some(&outsider), "POST", &comments, hello()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = f.call(Some(&enlisted), "POST", &comments, hello()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "members cannot comment");
    for reader in [&outsider, &enlisted] {
        let (status, _) = f.reviews(reader, mission).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
    let (status, _) = f.call(Some(&f.author), "POST", &comments, hello()).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "the thread outlives the decision"
    );
    assert_eq!(f.audits("mission.review_comment", mission).await, 3);
}

#[tokio::test]
async fn review_workspace_opens_exactly_the_reviewed_version() {
    let f = MissionFixture::new(SUITE).await;
    let (mission, reviewed_version) = f.compilable_mission("Workspace").await;
    let artifact = f.submit(mission).await;
    let later = f.save(mission, "0.3.0", &payload_with_role("TL")).await;
    let workspace = format!("/api/v1/missions/{mission}/artifacts/{artifact}/workspace");

    for reader in [&f.admin, &f.author] {
        let (status, opened) = f.call(Some(reader), "GET", &workspace, None).await;
        assert_eq!(status, StatusCode::OK, "{opened}");
        assert_eq!(uuid_of(&opened["artifact"]["id"]), artifact);
        assert_eq!(uuid_of(&opened["version"]["id"]), reviewed_version);
        assert_ne!(uuid_of(&opened["version"]["id"]), later);
        assert_eq!(opened["version"]["semver"], "0.2.0");
        let saved: Value = serde_json::from_str(common::COMPILABLE_EDITOR_PAYLOAD).unwrap();
        assert_eq!(opened["version"]["json_payload"], saved);
    }
    let (status, decided) = f.approve(mission, artifact, None).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    let (_, opened) = f.call(Some(&f.admin), "GET", &workspace, None).await;
    assert_eq!(uuid_of(&opened["version"]["id"]), reviewed_version);

    let refused =
        sqlx::query("UPDATE mission_versions SET json_payload = '{}'::jsonb WHERE id = $1")
            .bind(reviewed_version)
            .execute(f.pool())
            .await
            .unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("mission versions are immutable"),
        "{refused}"
    );
    let refused = sqlx::query("DELETE FROM mission_versions WHERE id = $1")
        .bind(reviewed_version)
        .execute(f.pool())
        .await;
    assert!(refused.is_err(), "an artifact's version cannot be deleted");
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
