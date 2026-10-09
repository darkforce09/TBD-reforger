//! The administrator's vehicle database writes through the real HTTP router: a create answers 201
//! with the trimmed row, and a member's writes are refused with 403.
//!
//! Each case owns its administrator and member. Live answers are checked against
//! `vehicle-database.schema.json` and refusals against the content error envelope of
//! `content-upload.schema.json`.

use crate::{content_support, contract_support};

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use content_support::{ContentSuite, assert_refusal, vehicle_body};
use contract_support::assert_valid;

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
