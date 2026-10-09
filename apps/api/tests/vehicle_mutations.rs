//! The administrator's vehicle database writes through the real HTTP router:
//! `POST /api/v1/vehicle-database` and `PUT`, `PATCH`, `DELETE /api/v1/vehicle-database/{id}`.
//!
//! Each case owns its administrator and member, so every count it reads (rows created, audit
//! lines written) is scoped to its own actors, and requires the isolated PostgreSQL harness.
//! Live answers are checked against `vehicle-database.schema.json` and refusals against the
//! content error envelope of `content-upload.schema.json`.
//!
//! ## What makes this fail (non-vacuity)
//! Drop `#[serde(deny_unknown_fields)]` from `VehiclePatchBody` in
//! `crates/api/api_community_content/src/handlers/vehicle_database/validation.rs`: a PATCH naming
//! an unknown key
//! becomes an empty edit answered 200, and `vehicle_mutations_unknown_field_is_refused_by_every_write`
//! goes red.

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

mod common;
mod content_support;
mod contract_support;

use content_support::{
    ContentSuite, JSON_BODY_LIMIT, assert_refusal, clear_failure, inject_failure, oversized_json,
    vehicle_body,
};
use contract_support::{assert_invalid, assert_valid};

const SUITE: &str = "vehicle_mutations";
const CONTRACT: &str = "vehicle-database.schema.json";
const LIST: &str = "/api/v1/vehicle-database";

/// A short tag unique to one case, carried by the names it writes.
fn tag() -> String {
    format!("vm-{}", &Uuid::new_v4().simple().to_string()[..10])
}

fn row_uri(id: &str) -> String {
    format!("{LIST}/{id}")
}

/// POST `body` as the administrator; answers the created row after checking it is 201 and
/// matches the contract.
async fn create(suite: &ContentSuite, body: Value) -> Value {
    let (status, row) = suite
        .call(Some(&suite.admin), "POST", LIST, Some(body))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{row}");
    assert_valid(CONTRACT, Some("Vehicle"), &row);
    row
}

/// GET one row as the member.
async fn read(suite: &ContentSuite, id: &str) -> (StatusCode, Value) {
    suite
        .call(Some(&suite.member), "GET", &row_uri(id), None)
        .await
}

/// The live list as the member, checked against the contract.
async fn list(suite: &ContentSuite) -> Vec<Value> {
    let (status, body) = suite.call(Some(&suite.member), "GET", LIST, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_valid(CONTRACT, Some("VehicleList"), &body);
    body["data"]
        .as_array()
        .expect("the `data` field is an array")
        .clone()
}

/// The lifecycle stamps of a stored row: `(created_by, updated_by, deleted_by, deleted)`.
async fn stamps(
    suite: &ContentSuite,
    id: &str,
) -> (Option<String>, Option<String>, Option<String>, bool) {
    sqlx::query_as(
        "SELECT created_by, updated_by, deleted_by, deleted_at IS NOT NULL \
         FROM vehicle_databases WHERE id = $1::uuid",
    )
    .bind(id)
    .fetch_one(suite.pool())
    .await
    .expect("the read of vehicle_databases returns a row")
}

fn id_of(row: &Value) -> String {
    row["id"]
        .as_str()
        .expect("the `id` field is a string")
        .to_owned()
}

#[tokio::test]
async fn vehicle_mutations_create_answers_201_with_the_trimmed_row() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let body = json!({
        "name": format!("  {tag} Marder  "),
        "faction": " BLUFOR ",
        "armor_type": "IFV",
        "amphibious": "   ",
        "primary_threat": "RPG-7",
        "profile_image_url": "/uploads/marder.png",
    });
    assert_valid(CONTRACT, Some("VehicleWrite"), &body);

    let row = create(&suite, body).await;
    assert_eq!(row["name"], format!("{tag} Marder"));
    assert_eq!(row["faction"], "BLUFOR");
    assert_eq!(row["armor_type"], "IFV");
    assert!(
        row.get("amphibious").is_none(),
        "a blank optional is none: {row}"
    );
    assert_eq!(row["primary_threat"], "RPG-7");
    assert_eq!(row["profile_image_url"], "/uploads/marder.png");

    let id = id_of(&row);
    let (status, stored) = read(&suite, &id).await;
    assert_eq!(status, StatusCode::OK, "{stored}");
    assert_eq!(stored, row, "GET answers the row POST answered");
    assert!(list(&suite).await.contains(&row));
    let (created_by, updated_by, deleted_by, deleted) = stamps(&suite, &id).await;
    assert_eq!(created_by.as_deref(), Some(suite.admin.id.as_str()));
    assert_eq!(updated_by.as_deref(), Some(suite.admin.id.as_str()));
    assert_eq!((deleted_by, deleted), (None, false));
}

#[tokio::test]
async fn vehicle_mutations_put_replaces_every_field() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);
    let editor = suite.account("editor", "admin").await;

    let replacement = json!({
        "name": format!("{tag} Leopard 2A6"),
        "faction": "NATO",
        "armor_type": "MBT",
        "primary_threat": " Javelin ",
    });
    assert_valid(CONTRACT, Some("VehicleWrite"), &replacement);
    let (status, replaced) = suite
        .call(Some(&editor), "PUT", &row_uri(&id), Some(replacement))
        .await;
    assert_eq!(status, StatusCode::OK, "{replaced}");
    assert_valid(CONTRACT, Some("Vehicle"), &replaced);
    assert_eq!(
        replaced,
        json!({
            "id": id,
            "name": format!("{tag} Leopard 2A6"),
            "faction": "NATO",
            "armor_type": "MBT",
            "primary_threat": "Javelin",
        }),
        "PUT stores every field and clears the optional ones it leaves out"
    );
    assert_eq!(read(&suite, &id).await.1, replaced);
    let (created_by, updated_by, _, deleted) = stamps(&suite, &id).await;
    assert_eq!(created_by.as_deref(), Some(suite.admin.id.as_str()));
    assert_eq!(updated_by.as_deref(), Some(editor.id.as_str()));
    assert!(!deleted);
}

#[tokio::test]
async fn vehicle_mutations_patch_changes_only_the_named_fields() {
    let suite = ContentSuite::new(SUITE).await;
    let row = create(&suite, vehicle_body(&tag())).await;
    let id = id_of(&row);

    let patch = json!({ "faction": " OPFOR ", "primary_threat": "Carl Gustaf" });
    assert_valid(CONTRACT, Some("VehiclePatch"), &patch);
    let (status, patched) = suite
        .call(Some(&suite.admin), "PATCH", &row_uri(&id), Some(patch))
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_valid(CONTRACT, Some("Vehicle"), &patched);
    let mut expected = row.clone();
    expected["faction"] = json!("OPFOR");
    expected["primary_threat"] = json!("Carl Gustaf");
    assert_eq!(
        patched, expected,
        "every key the PATCH leaves out keeps its value"
    );
    assert_eq!(read(&suite, &id).await.1, expected);
}

#[tokio::test]
async fn vehicle_mutations_patch_null_clears_an_optional_field() {
    let suite = ContentSuite::new(SUITE).await;
    let row = create(&suite, vehicle_body(&tag())).await;
    let id = id_of(&row);

    let patch = json!({ "amphibious": null, "profile_image_url": "" });
    assert_valid(CONTRACT, Some("VehiclePatch"), &patch);
    let (status, patched) = suite
        .call(Some(&suite.admin), "PATCH", &row_uri(&id), Some(patch))
        .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_valid(CONTRACT, Some("Vehicle"), &patched);
    let mut expected = row.clone();
    expected.as_object_mut().unwrap().remove("amphibious");
    expected
        .as_object_mut()
        .unwrap()
        .remove("profile_image_url");
    assert_eq!(patched, expected, "null and empty clear an optional field");
    assert_eq!(read(&suite, &id).await.1, expected);
    let (amphibious, image): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT amphibious, profile_image_url FROM vehicle_databases WHERE id = $1::uuid",
    )
    .bind(&id)
    .fetch_one(suite.pool())
    .await
    .unwrap();
    assert_eq!(
        (amphibious, image),
        (None, None),
        "a cleared field is stored as null"
    );
}

#[tokio::test]
async fn vehicle_mutations_patch_null_or_blank_on_a_required_field_is_refused() {
    let suite = ContentSuite::new(SUITE).await;
    let row = create(&suite, vehicle_body(&tag())).await;
    let id = id_of(&row);
    let audits = suite.content_audits_by(&suite.admin).await;

    for key in ["name", "faction", "armor_type"] {
        for value in [Value::Null, json!(""), json!("  \t ")] {
            let patch = json!({ key: value.clone(), "primary_threat": "changed alongside" });
            assert_invalid(CONTRACT, Some("VehiclePatch"), &patch);
            let (status, body) = suite
                .call(Some(&suite.admin), "PATCH", &row_uri(&id), Some(patch))
                .await;
            assert_refusal(status, &body, StatusCode::BAD_REQUEST, None);
            assert!(
                body["error"].as_str().unwrap().contains(key),
                "{key} = {value}: {body}"
            );
        }
    }
    assert_eq!(
        read(&suite, &id).await.1,
        row,
        "a refused PATCH changes nothing"
    );
    assert_eq!(suite.content_audits_by(&suite.admin).await, audits);
}

#[tokio::test]
async fn vehicle_mutations_unknown_field_is_refused_by_every_write() {
    let suite = ContentSuite::new(SUITE).await;
    let row = create(&suite, vehicle_body(&tag())).await;
    let id = id_of(&row);
    let audits = suite.content_audits_by(&suite.admin).await;

    let mut whole = vehicle_body(&tag());
    whole["colour"] = json!("olive");
    assert_invalid(CONTRACT, Some("VehicleWrite"), &whole);
    let partial = json!({ "colour": "olive" });
    assert_invalid(CONTRACT, Some("VehiclePatch"), &partial);
    let stamped = json!({ "name": "Stamped", "created_by": "someone-else" });
    assert_invalid(CONTRACT, Some("VehiclePatch"), &stamped);

    let attempts = [
        ("POST", LIST.to_owned(), whole.clone(), "colour"),
        ("PUT", row_uri(&id), whole, "colour"),
        ("PATCH", row_uri(&id), partial, "colour"),
        ("PATCH", row_uri(&id), stamped, "created_by"),
    ];
    for (method, uri, body, key) in attempts {
        let (status, answer) = suite
            .call(Some(&suite.admin), method, &uri, Some(body))
            .await;
        assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
        assert!(
            answer["error"]
                .as_str()
                .unwrap()
                .contains(&format!("unknown field `{key}`")),
            "{method} names the unknown key: {answer}"
        );
    }
    assert_eq!(read(&suite, &id).await.1, row);
    assert_eq!(suite.vehicles_created_by(&suite.admin).await, 1);
    assert_eq!(suite.content_audits_by(&suite.admin).await, audits);
}

#[tokio::test]
async fn vehicle_mutations_every_write_shares_one_validation_message() {
    let suite = ContentSuite::new(SUITE).await;
    let row = create(&suite, vehicle_body(&tag())).await;
    let id = id_of(&row);
    let audits = suite.content_audits_by(&suite.admin).await;

    let refused = [
        ("name", json!("   ")),
        ("faction", json!("")),
        ("armor_type", json!("\t")),
        ("name", json!("Ж".repeat(121))),
        ("faction", json!("x".repeat(61))),
        ("armor_type", json!("x".repeat(61))),
        ("amphibious", json!("x".repeat(61))),
        ("primary_threat", json!("x".repeat(121))),
        ("profile_image_url", json!("javascript:alert(1)")),
        ("profile_image_url", json!("//evil.example/x.png")),
        ("profile_image_url", json!("http://example.com/x.png")),
        ("profile_image_url", json!("data:image/png;base64,AAAA")),
    ];
    for (key, value) in refused {
        let mut whole = vehicle_body(&tag());
        whole[key] = value.clone();
        let partial = json!({ key: value.clone() });
        assert_invalid(CONTRACT, Some("VehicleWrite"), &whole);
        assert_invalid(CONTRACT, Some("VehiclePatch"), &partial);

        let mut messages = Vec::new();
        for (method, uri, body) in [
            ("POST", LIST.to_owned(), whole.clone()),
            ("PUT", row_uri(&id), whole),
            ("PATCH", row_uri(&id), partial),
        ] {
            let (status, answer) = suite
                .call(Some(&suite.admin), method, &uri, Some(body))
                .await;
            assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
            messages.push(answer["error"].as_str().unwrap().to_owned());
        }
        assert!(messages[0].contains(key), "{key}: {messages:?}");
        assert!(
            messages.iter().all(|message| *message == messages[0]),
            "{key} = {value}: POST, PUT and PATCH answer one message: {messages:?}"
        );
    }
    assert_eq!(
        read(&suite, &id).await.1,
        row,
        "a refused write changes nothing"
    );
    assert_eq!(suite.vehicles_created_by(&suite.admin).await, 1);
    assert_eq!(suite.content_audits_by(&suite.admin).await, audits);

    // The limits count characters, as the schema's maxLength does: 120 two-byte letters fit.
    let at_limit = "Ж".repeat(120);
    let mut whole = vehicle_body(&tag());
    whole["name"] = json!(at_limit);
    assert_valid(CONTRACT, Some("VehicleWrite"), &whole);
    let created = create(&suite, whole.clone()).await;
    assert_eq!(created["name"], at_limit);
    let (status, body) = suite
        .call(Some(&suite.admin), "PUT", &row_uri(&id), Some(whole))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = suite
        .call(
            Some(&suite.admin),
            "PATCH",
            &row_uri(&id),
            Some(json!({ "primary_threat": "Ж".repeat(120) })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn vehicle_mutations_delete_is_soft_and_hides_the_row() {
    let suite = ContentSuite::new(SUITE).await;
    let row = create(&suite, vehicle_body(&tag())).await;
    let id = id_of(&row);

    let (status, deleted) = suite
        .call(Some(&suite.admin), "DELETE", &row_uri(&id), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{deleted}");
    assert_valid(CONTRACT, Some("Vehicle"), &deleted);
    assert_eq!(deleted, row, "DELETE answers the row as stored");

    let (_, _, deleted_by, is_deleted) = stamps(&suite, &id).await;
    assert!(is_deleted, "the row is kept with deleted_at set");
    assert_eq!(deleted_by.as_deref(), Some(suite.admin.id.as_str()));
    assert!(
        !list(&suite).await.iter().any(|item| item["id"] == id),
        "a deleted row leaves the list"
    );
    let (status, body) = read(&suite, &id).await;
    assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    let (status, body) = suite
        .call(Some(&suite.admin), "DELETE", &row_uri(&id), None)
        .await;
    assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    assert_eq!(
        suite.content_audits_by(&suite.admin).await,
        2,
        "the create and the one accepted DELETE"
    );
}

#[tokio::test]
async fn vehicle_mutations_put_and_patch_on_a_deleted_row_answer_404() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);
    let (status, body) = suite
        .call(Some(&suite.admin), "DELETE", &row_uri(&id), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let audits = suite.content_audits_by(&suite.admin).await;

    let unknown = Uuid::new_v4().to_string();
    for target in [&id, &unknown] {
        let (status, body) = suite
            .call(
                Some(&suite.admin),
                "PUT",
                &row_uri(target),
                Some(vehicle_body(&tag)),
            )
            .await;
        assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
        let (status, body) = suite
            .call(
                Some(&suite.admin),
                "PATCH",
                &row_uri(target),
                Some(json!({ "name": "Revived" })),
            )
            .await;
        assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    }
    let (name, updated_by): (String, Option<String>) =
        sqlx::query_as("SELECT name, updated_by FROM vehicle_databases WHERE id = $1::uuid")
            .bind(&id)
            .fetch_one(suite.pool())
            .await
            .unwrap();
    assert_eq!(
        name,
        row["name"].as_str().unwrap(),
        "the deleted row is untouched"
    );
    assert_eq!(updated_by.as_deref(), Some(suite.admin.id.as_str()));
    assert!(stamps(&suite, &id).await.3, "the row stays deleted");
    assert_eq!(suite.content_audits_by(&suite.admin).await, audits);
}

/// Every write route with a body that would be accepted from an administrator.
fn write_attempts(tag: &str, id: &str) -> [(&'static str, String, Option<Value>); 4] {
    [
        ("POST", LIST.to_owned(), Some(vehicle_body(tag))),
        ("PUT", row_uri(id), Some(vehicle_body(tag))),
        ("PATCH", row_uri(id), Some(json!({ "name": "Taken over" }))),
        ("DELETE", row_uri(id), None),
    ]
}

#[tokio::test]
async fn vehicle_mutations_member_writes_answer_403() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);

    for (method, uri, body) in write_attempts(&tag, &id) {
        let (status, answer) = suite.call(Some(&suite.member), method, &uri, body).await;
        assert_refusal(status, &answer, StatusCode::FORBIDDEN, None);
    }
    assert_eq!(
        read(&suite, &id).await,
        (StatusCode::OK, row),
        "a member reads"
    );
    assert!(!list(&suite).await.is_empty());
    assert_eq!(suite.vehicles_created_by(&suite.member).await, 0);
    assert_eq!(suite.content_audits_by(&suite.member).await, 0);
    assert_eq!(suite.content_audits_by(&suite.admin).await, 1);
}

#[tokio::test]
async fn vehicle_mutations_anonymous_writes_answer_401() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);

    for (method, uri, body) in write_attempts(&tag, &id) {
        let (status, answer) = suite.call(None, method, &uri, body).await;
        assert_refusal(status, &answer, StatusCode::UNAUTHORIZED, None);
    }
    for uri in [LIST.to_owned(), row_uri(&id)] {
        let (status, answer) = suite.call(None, "GET", &uri, None).await;
        assert_refusal(status, &answer, StatusCode::UNAUTHORIZED, None);
    }
    assert_eq!(read(&suite, &id).await, (StatusCode::OK, row));
    assert_eq!(suite.content_audits_by(&suite.admin).await, 1);
}

/// The newest audit line of the vehicle `id` and whether it was written at the instant the
/// row's `updated_at` (or `deleted_at`) names: `now()` is fixed per transaction, so equal
/// instants mean the row write and the audit line shared one.
async fn latest_audit(suite: &ContentSuite, id: &str) -> (String, String, String, String, bool) {
    sqlx::query_as(
        "SELECT a.action, a.target_type, a.actor_id, a.message, \
         a.created_at = COALESCE(v.deleted_at, v.updated_at) \
         FROM audit_logs a JOIN vehicle_databases v ON a.target_id = v.id::text \
         WHERE v.id = $1::uuid ORDER BY a.id DESC LIMIT 1",
    )
    .bind(id)
    .fetch_one(suite.pool())
    .await
    .expect("the read of audit_logs returns a row")
}

async fn audits_of(suite: &ContentSuite, id: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE target_id = $1")
        .bind(id)
        .fetch_one(suite.pool())
        .await
        .expect("the read of audit_logs returns a row")
}

#[tokio::test]
async fn vehicle_mutations_every_write_appends_one_audit_row_in_its_transaction() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);
    let created_same_instant: bool = sqlx::query_scalar(
        "SELECT v.created_at = v.updated_at FROM vehicle_databases v WHERE v.id = $1::uuid",
    )
    .bind(&id)
    .fetch_one(suite.pool())
    .await
    .unwrap();
    assert!(created_same_instant);

    let steps: [(&str, Option<Value>, &str, &str); 5] = [
        (
            "POST",
            None,
            "vehicle.created",
            "Created vehicle database entry",
        ),
        (
            "PUT",
            Some(vehicle_body(&tag)),
            "vehicle.replaced",
            "Replaced",
        ),
        (
            "PATCH",
            Some(json!({ "primary_threat": "Mines" })),
            "vehicle.updated",
            "(primary_threat)",
        ),
        ("PATCH", Some(json!({})), "vehicle.updated", "(no fields)"),
        ("DELETE", None, "vehicle.deleted", "Deleted"),
    ];
    for (count, (method, body, action, message)) in steps.into_iter().enumerate() {
        if method != "POST" {
            let (status, answer) = suite
                .call(Some(&suite.admin), method, &row_uri(&id), body)
                .await;
            assert_eq!(status, StatusCode::OK, "{method}: {answer}");
        }
        assert_eq!(
            audits_of(&suite, &id).await,
            count as i64 + 1,
            "{method} appends exactly one audit row"
        );
        let (logged_action, target_type, actor, logged_message, same_transaction) =
            latest_audit(&suite, &id).await;
        assert_eq!(logged_action, action);
        assert_eq!(target_type, "vehicle");
        assert_eq!(actor, suite.admin.id);
        assert!(logged_message.contains(message), "{logged_message}");
        assert!(
            same_transaction,
            "{method}: the audit row carries its row write's transaction instant"
        );
    }
    let (_, updated_by, deleted_by, deleted) = stamps(&suite, &id).await;
    assert_eq!(updated_by.as_deref(), Some(suite.admin.id.as_str()));
    assert_eq!(deleted_by.as_deref(), Some(suite.admin.id.as_str()));
    assert!(deleted);
    assert_eq!(suite.content_audits_by(&suite.admin).await, 5);
}

#[tokio::test]
async fn vehicle_mutations_audit_failure_rolls_back_every_write() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);

    let trigger = inject_failure(
        suite.pool(),
        "audit_logs",
        "INSERT",
        "actor_id",
        &suite.admin.id,
    )
    .await;
    for (method, uri, body) in write_attempts(&tag, &id) {
        let (status, answer) = suite.call(Some(&suite.admin), method, &uri, body).await;
        assert_refusal(status, &answer, StatusCode::INTERNAL_SERVER_ERROR, None);
    }
    clear_failure(suite.pool(), &trigger, "audit_logs").await;

    assert_eq!(
        suite.vehicles_created_by(&suite.admin).await,
        1,
        "the failed create left no row"
    );
    assert_eq!(
        read(&suite, &id).await.1,
        row,
        "the failed writes left the row"
    );
    let (_, _, deleted_by, deleted) = stamps(&suite, &id).await;
    assert_eq!((deleted_by, deleted), (None, false));
    let untouched: bool = sqlx::query_scalar(
        "SELECT created_at = updated_at FROM vehicle_databases WHERE id = $1::uuid",
    )
    .bind(&id)
    .fetch_one(suite.pool())
    .await
    .unwrap();
    assert!(untouched, "no failed write stamped updated_at");
    assert_eq!(suite.content_audits_by(&suite.admin).await, 1);

    let (status, body) = suite
        .call(
            Some(&suite.admin),
            "PATCH",
            &row_uri(&id),
            Some(json!({ "armor_type": "APC" })),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the write succeeds once storage recovers: {body}"
    );
    assert_eq!(suite.content_audits_by(&suite.admin).await, 2);
}

#[tokio::test]
async fn vehicle_mutations_row_failure_writes_no_audit_row() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();

    let trigger = inject_failure(
        suite.pool(),
        "vehicle_databases",
        "INSERT",
        "created_by",
        &suite.admin.id,
    )
    .await;
    let (status, body) = suite
        .call(Some(&suite.admin), "POST", LIST, Some(vehicle_body(&tag)))
        .await;
    assert_refusal(status, &body, StatusCode::INTERNAL_SERVER_ERROR, None);
    clear_failure(suite.pool(), &trigger, "vehicle_databases").await;
    assert_eq!(suite.vehicles_created_by(&suite.admin).await, 0);
    assert_eq!(suite.content_audits_by(&suite.admin).await, 0);

    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);
    let trigger = inject_failure(suite.pool(), "vehicle_databases", "UPDATE", "id", &id).await;
    for (method, uri, body) in write_attempts(&tag, &id).into_iter().skip(1) {
        let (status, answer) = suite.call(Some(&suite.admin), method, &uri, body).await;
        assert_refusal(status, &answer, StatusCode::INTERNAL_SERVER_ERROR, None);
    }
    clear_failure(suite.pool(), &trigger, "vehicle_databases").await;
    assert_eq!(read(&suite, &id).await.1, row);
    assert_eq!(
        suite.content_audits_by(&suite.admin).await,
        1,
        "only the accepted create is audited"
    );
}

#[tokio::test]
async fn vehicle_mutations_body_limit_content_type_and_syntax_are_refused() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let row = create(&suite, vehicle_body(&tag)).await;
    let id = id_of(&row);
    let targets = [
        ("POST", LIST.to_owned(), vehicle_body(&tag)),
        ("PUT", row_uri(&id), vehicle_body(&tag)),
        ("PATCH", row_uri(&id), json!({ "name": "Renamed" })),
    ];

    for (method, uri, body) in targets {
        let oversized = oversized_json(body.clone(), JSON_BODY_LIMIT);
        assert!(oversized.len() > JSON_BODY_LIMIT);
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

        let bytes = body.to_string().into_bytes();
        for content_type in [None, Some("text/plain")] {
            let (status, answer) = suite
                .send(
                    Some(&suite.admin),
                    method,
                    &uri,
                    content_type,
                    bytes.clone(),
                )
                .await;
            assert_refusal(status, &answer, StatusCode::UNSUPPORTED_MEDIA_TYPE, None);
        }

        let (status, answer) = suite
            .send(
                Some(&suite.admin),
                method,
                &uri,
                Some("application/json"),
                br#"{"name": "Half"#.to_vec(),
            )
            .await;
        assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
    }
    assert_eq!(read(&suite, &id).await.1, row);
    assert_eq!(suite.vehicles_created_by(&suite.admin).await, 1);
    assert_eq!(suite.content_audits_by(&suite.admin).await, 1);
}

#[tokio::test]
async fn vehicle_mutations_malformed_id_is_refused_on_every_route() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    for (method, uri, body) in write_attempts(&tag, "not-a-uuid").into_iter().skip(1) {
        let (status, answer) = suite.call(Some(&suite.admin), method, &uri, body).await;
        assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
        assert_eq!(answer["error"], "invalid vehicle id");
    }
    let (status, answer) = read(&suite, "not-a-uuid").await;
    assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
    assert_eq!(suite.content_audits_by(&suite.admin).await, 0);
}

#[tokio::test]
async fn vehicle_mutations_list_orders_by_name_then_id() {
    let suite = ContentSuite::new(SUITE).await;
    let tag = tag();
    let named = |name: &str| {
        let mut body = vehicle_body(&tag);
        body["name"] = json!(format!("{tag} {name}"));
        body
    };
    let bravo = create(&suite, named("Bravo")).await;
    let alpha_one = create(&suite, named("Alpha")).await;
    let alpha_two = create(&suite, named("Alpha")).await;
    let gone = create(&suite, named("Alpha")).await;
    let (status, body) = suite
        .call(Some(&suite.admin), "DELETE", &row_uri(&id_of(&gone)), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let mut alphas = [alpha_one, alpha_two];
    alphas.sort_by_key(id_of);
    let expected: Vec<Value> = alphas.into_iter().chain([bravo]).collect();
    let listed: Vec<Value> = list(&suite)
        .await
        .into_iter()
        .filter(|item| item["name"].as_str().unwrap().starts_with(&tag))
        .collect();
    assert_eq!(
        listed, expected,
        "name ASC, then id ASC, without the deleted row"
    );
}

#[tokio::test]
async fn vehicle_mutations_missing_required_field_is_refused_with_400() {
    let suite = ContentSuite::new(SUITE).await;
    let row = create(&suite, vehicle_body(&tag())).await;
    let id = id_of(&row);
    let partial = json!({ "name": "Only a name" });
    assert_invalid(CONTRACT, Some("VehicleWrite"), &partial);

    for (method, uri) in [("POST", LIST.to_owned()), ("PUT", row_uri(&id))] {
        let (status, body) = suite
            .call(Some(&suite.admin), method, &uri, Some(partial.clone()))
            .await;
        assert_refusal(status, &body, StatusCode::BAD_REQUEST, None);
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("missing field `faction`"),
            "{method}: {body}"
        );
    }
    assert_eq!(read(&suite, &id).await.1, row);
    assert_eq!(suite.vehicles_created_by(&suite.admin).await, 1);
    assert_eq!(suite.content_audits_by(&suite.admin).await, 1);
}
