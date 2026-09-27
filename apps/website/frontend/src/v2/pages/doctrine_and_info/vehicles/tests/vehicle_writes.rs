//! The vehicle write requests: their methods, paths and bodies, held to the backend's routes and the
//! contract's write body.

use super::VehicleRequest;
use crate::v2::core::api::dto::vehicles::VehicleWrite;
use serde_json::json;

/// The vehicle database contract, whose `VehicleWrite` names every key a write body may carry.
const VEHICLE_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts_v2/definitions/vehicle-database.schema.json"
));

const STORED_ID: &str = "00000000-0000-4000-3000-000000000002";

fn body() -> VehicleWrite {
    VehicleWrite {
        name: "M113A3".into(),
        faction: "US Army".into(),
        armor_type: "Light Armour".into(),
        amphibious: "No".into(),
        primary_threat: String::new(),
        profile_image_url: "/uploads/m113a3.png".into(),
    }
}

fn body_json(request: &VehicleRequest) -> serde_json::Value {
    serde_json::to_value(request.body().expect("the request carries a body"))
        .expect("a write body serialises")
}

#[test]
fn create_posts_every_field_to_the_collection() {
    let request = VehicleRequest::create(body());
    assert_eq!(request.method(), "POST");
    assert_eq!(request.path(), "/vehicle-database");
    assert_eq!(
        body_json(&request),
        json!({
            "name": "M113A3",
            "faction": "US Army",
            "armor_type": "Light Armour",
            "amphibious": "No",
            "primary_threat": "",
            "profile_image_url": "/uploads/m113a3.png"
        })
    );
}

#[test]
fn replace_puts_every_field_to_the_row() {
    let request = VehicleRequest::replace(STORED_ID, body());
    assert_eq!(request.method(), "PUT");
    assert_eq!(
        request.path(),
        "/vehicle-database/00000000-0000-4000-3000-000000000002"
    );
    assert_eq!(request.body(), Some(&body()));
    assert_eq!(body_json(&request)["name"], "M113A3");
}

#[test]
fn delete_sends_no_body_to_the_row() {
    let request = VehicleRequest::delete(STORED_ID);
    assert_eq!(request.method(), "DELETE");
    assert_eq!(
        request.path(),
        "/vehicle-database/00000000-0000-4000-3000-000000000002"
    );
    assert_eq!(request.body(), None);
}

#[test]
fn the_id_travels_as_one_percent_encoded_segment() {
    let id = "a/b c?d#e";
    let expected = "/vehicle-database/a%2Fb%20c%3Fd%23e";
    assert_eq!(VehicleRequest::delete(id).path(), expected);
    assert_eq!(VehicleRequest::replace(id, body()).path(), expected);
    assert_eq!(
        VehicleRequest::delete("../admin/users").path(),
        "/vehicle-database/..%2Fadmin%2Fusers"
    );
}

#[test]
fn the_write_body_carries_exactly_the_keys_the_contract_names() {
    let contract: serde_json::Value =
        serde_json::from_str(VEHICLE_CONTRACT).expect("the contract is JSON");
    let mut named: Vec<String> = contract["definitions"]["VehicleWrite"]["properties"]
        .as_object()
        .expect("VehicleWrite names its properties")
        .keys()
        .cloned()
        .collect();
    named.sort();
    let sent = body_json(&VehicleRequest::create(body()));
    let mut keys: Vec<String> = sent
        .as_object()
        .expect("the body is an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    assert_eq!(keys, named);
}
