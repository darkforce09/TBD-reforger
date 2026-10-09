use serde_json::{Value, json};

use super::*;

const IDENTITY: &str = "3d8f0e52-6c5a-4e0b-9a55-0c2b1d7e4f10";
const DEV_POC: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

#[test]
fn identifiers_session_player_id_round_trips_as_its_bare_number() {
    for number in [0, 12, u32::MAX] {
        let id = SessionPlayerId::new(number);
        let wire = serde_json::to_value(id).unwrap();
        assert_eq!(
            wire,
            json!(number),
            "the outcome's player_id is the bare number"
        );
        assert_eq!(serde_json::from_value::<SessionPlayerId>(wire).unwrap(), id);
        assert_eq!(id.to_string().parse::<SessionPlayerId>().unwrap(), id);
        assert_eq!(id.get(), number);
    }
}

#[test]
fn identifiers_session_player_id_reads_only_what_a_u32_reads() {
    for text in ["-1", "x", "", "4294967296", " 4"] {
        assert_eq!(
            text.parse::<SessionPlayerId>().ok(),
            text.parse::<u32>().ok().map(SessionPlayerId::new),
            "{text:?}"
        );
    }
}

#[test]
fn identifiers_arma_player_id_round_trips_as_its_bare_text() {
    let id = ArmaPlayerId::new(IDENTITY);
    let wire = serde_json::to_value(&id).unwrap();
    assert_eq!(
        wire,
        Value::from(IDENTITY),
        "the outcome's arma_id is the bare text"
    );
    assert_eq!(serde_json::from_value::<ArmaPlayerId>(wire).unwrap(), id);
    assert_eq!(id.as_str(), IDENTITY);
    assert_eq!(id.to_string(), IDENTITY);
}

#[test]
fn identifiers_scenario_id_round_trips_through_its_text() {
    let id = ScenarioId::parse(DEV_POC).expect("the Dev POC header is a scenario id");
    assert_eq!(id.as_str(), DEV_POC, "the text is kept byte for byte");
    assert_eq!(ScenarioId::parse(id.as_str()), Some(id.clone()));
    assert_eq!(
        Value::from(id.as_str()).to_string(),
        format!("\"{DEV_POC}\""),
        "the server config receives the same JSON string the raw text gave"
    );
}
