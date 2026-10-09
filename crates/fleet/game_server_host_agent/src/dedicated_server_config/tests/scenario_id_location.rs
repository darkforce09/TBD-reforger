use super::*;

fn located(document: &str) -> Result<&str, ScenarioIdAbsence> {
    scenario_id_span(document).map(|span| &document[span])
}

#[test]
fn the_scenario_id_string_is_located_with_its_quotes() {
    let document = r#"{"bindAddress": "", "game": {"name": "TBD", "scenarioId": "{ECC61978EDCC2B5A}Missions/23_Campaign.conf", "maxPlayers": 64}}"#;
    assert_eq!(
        located(document),
        Ok(r#""{ECC61978EDCC2B5A}Missions/23_Campaign.conf""#)
    );
}

#[test]
fn members_before_it_and_nested_values_are_stepped_over() {
    let document = r##"{
  "a2s": {"address": "0.0.0.0", "port": 17777},
  "rcon": {"blacklist": ["#shutdown", "scenarioId"], "nested": [[{}], {"x": [1, 2.5e3, true, null]}]},
  "note": "a \"game\" inside a string: {\"scenarioId\": 1}",
  "game": {
    "mods": [{"modId": "59727DAE364DEADB", "name": "Weapon \\ Switching"}],
    "gameProperties": {"scenarioId": "not this one"},
    "scenarioId" : "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
  }
}"##;
    assert_eq!(
        located(document),
        Ok(r#""{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf""#)
    );
}

#[test]
fn escaped_member_names_are_decoded_before_they_are_compared() {
    let document = r#"{"game": {"scenarioId": "{0000000000000000}A.conf"}}"#;
    assert_eq!(located(document), Ok(r#""{0000000000000000}A.conf""#));
}

#[test]
fn escapes_inside_the_value_belong_to_its_token() {
    let document = r#"{"game": {"scenarioId": "{0000000000000000}Missions\/A \"x\".conf"}}"#;
    assert_eq!(
        located(document),
        Ok(r#""{0000000000000000}Missions\/A \"x\".conf""#)
    );
}

#[test]
fn documents_without_exactly_one_scenario_id_string_are_refused() {
    for (document, absence) in [
        (
            r#"[{"game": {"scenarioId": "x"}}]"#,
            ScenarioIdAbsence::RootNotAnObject,
        ),
        ("{}", ScenarioIdAbsence::NoGame),
        (r#"{"gameHostBindAddress": ""}"#, ScenarioIdAbsence::NoGame),
        (
            r#"{"game": {"scenarioId": "a"}, "game": {"scenarioId": "b"}}"#,
            ScenarioIdAbsence::DuplicateGame,
        ),
        (
            r#"{"game": "scenarioId"}"#,
            ScenarioIdAbsence::GameNotAnObject,
        ),
        (r#"{"game": {}}"#, ScenarioIdAbsence::NoScenarioId),
        (
            r#"{"game": {"gameProperties": {"scenarioId": "x"}}}"#,
            ScenarioIdAbsence::NoScenarioId,
        ),
        (
            r#"{"game": {"scenarioId": "a", "scenarioId": "b"}}"#,
            ScenarioIdAbsence::DuplicateScenarioId,
        ),
        (
            r#"{"game": {"scenarioId": 42}}"#,
            ScenarioIdAbsence::ScenarioIdNotAString,
        ),
        (
            r#"{"game": {"scenarioId": null}}"#,
            ScenarioIdAbsence::ScenarioIdNotAString,
        ),
    ] {
        assert_eq!(scenario_id_span(document), Err(absence), "{document}");
        assert!(!absence.describe().is_empty());
    }
}
