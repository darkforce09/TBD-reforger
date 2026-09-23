use serde_json::json;

use super::*;

const DEPLOYMENT_ID: &str = "5f1c9a52-2d64-4f4e-9d1e-3b7b0a6f9c21";
const ARTIFACT_ID: &str = "0b9f2c7e-8a41-4c3d-b6f5-1e2d3c4b5a69";
const ARTIFACT_SHA256: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
const SCENARIO_ID: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

fn arguments() -> Value {
    json!({
        "deployment_id": DEPLOYMENT_ID,
        "artifact_id": ARTIFACT_ID,
        "artifact_sha256": ARTIFACT_SHA256,
        "scenario_id": SCENARIO_ID,
    })
}

fn with(key: &str, value: Value) -> Value {
    let mut arguments = arguments();
    arguments[key] = value;
    arguments
}

fn refusal(key: &'static str, expected: &'static str) -> CommandRefusal {
    CommandRefusal::InvalidArgument {
        action: "restart_with_mission",
        key,
        expected,
    }
}

#[test]
fn the_deployment_the_api_builds_is_accepted() {
    let deployment = MissionDeployment::from_arguments(&arguments()).unwrap();
    assert_eq!(deployment.deployment_id.to_string(), DEPLOYMENT_ID);
    assert_eq!(deployment.artifact_id.to_string(), ARTIFACT_ID);
    assert_eq!(deployment.artifact_sha256, ARTIFACT_SHA256);
    assert_eq!(deployment.scenario_id.as_str(), SCENARIO_ID);
}

#[test]
fn unknown_and_missing_keys_are_refused() {
    assert_eq!(
        MissionDeployment::from_arguments(&with("unit", json!("other.service"))),
        Err(CommandRefusal::UnexpectedArgument {
            action: "restart_with_mission",
            key: "unit".to_owned(),
        })
    );
    for key in ARGUMENT_KEYS {
        let mut arguments = arguments();
        arguments.as_object_mut().unwrap().remove(key);
        assert!(
            matches!(
                MissionDeployment::from_arguments(&arguments),
                Err(CommandRefusal::InvalidArgument { key: refused, .. }) if refused == key
            ),
            "{key}"
        );
    }
    assert_eq!(
        MissionDeployment::from_arguments(&json!([])),
        Err(CommandRefusal::ArgumentsNotAnObject {
            action: "restart_with_mission",
        })
    );
}

#[test]
fn ids_must_be_hyphenated_uuids() {
    assert!(
        MissionDeployment::from_arguments(&with("artifact_id", json!(ARTIFACT_ID.to_uppercase())))
            .is_ok()
    );
    for invalid in [
        json!("0b9f2c7e8a414c3db6f51e2d3c4b5a69"),
        json!("{0b9f2c7e-8a41-4c3d-b6f5-1e2d3c4b5a69}"),
        json!("urn:uuid:0b9f2c7e-8a41-4c3d-b6f5-1e2d3c4b5a69"),
        json!("0b9f2c7e-8a41-4c3d-b6f5-1e2d3c4b5a6g"),
        json!(""),
        json!(7),
    ] {
        for key in ["deployment_id", "artifact_id"] {
            assert_eq!(
                MissionDeployment::from_arguments(&with(key, invalid.clone())),
                Err(refusal(key, UUID_EXPECTATION)),
                "{key} = {invalid}"
            );
        }
    }
}

#[test]
fn the_digest_must_be_64_lowercase_hex_digits() {
    for invalid in [
        json!(ARTIFACT_SHA256.to_uppercase()),
        json!(&ARTIFACT_SHA256[1..]),
        json!(format!("{ARTIFACT_SHA256}0")),
        json!(ARTIFACT_SHA256.replacen('9', "g", 1)),
        json!(null),
    ] {
        assert_eq!(
            MissionDeployment::from_arguments(&with("artifact_sha256", invalid.clone())),
            Err(refusal("artifact_sha256", SHA256_EXPECTATION)),
            "{invalid}"
        );
    }
}

#[test]
fn the_scenario_must_be_a_scenario_header_resource() {
    for valid in [
        SCENARIO_ID,
        "{ECC61978EDCC2B5A}Missions/23_Campaign.conf",
        "{0123456789ABCDEF}a.conf",
        "{0123456789ABCDEF}Missions/Sub-Dir/v1.2/Mission_A.conf",
    ] {
        assert_eq!(
            ScenarioId::parse(valid).map(|scenario| scenario.as_str().to_owned()),
            Some(valid.to_owned())
        );
    }
    for invalid in [
        "Missions/TBD_Dev_POC.conf",
        "{69a85365fc09e2ca}Missions/TBD_Dev_POC.conf",
        "{69A85365FC09E2C}Missions/TBD_Dev_POC.conf",
        "{69A85365FC09E2CA0}Missions/TBD_Dev_POC.conf",
        "{69A85365FC09E2CA}.conf",
        "{69A85365FC09E2CA}Missions/TBD_Dev_POC",
        "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf.bak",
        "{69A85365FC09E2CA}Missions/TBD Dev POC.conf",
        "{69A85365FC09E2CA}Missions/TBD}Dev.conf",
        "{69A85365FC09E2CA}Missions\\TBD.conf",
        "{69A85365FC09E2CA}Missions/TBD\n.conf",
        " {69A85365FC09E2CA}Missions/TBD_Dev_POC.conf",
        "",
    ] {
        assert_eq!(ScenarioId::parse(invalid), None, "{invalid:?}");
        assert_eq!(
            MissionDeployment::from_arguments(&with("scenario_id", json!(invalid))),
            Err(refusal("scenario_id", SCENARIO_EXPECTATION)),
            "{invalid:?}"
        );
    }
}
