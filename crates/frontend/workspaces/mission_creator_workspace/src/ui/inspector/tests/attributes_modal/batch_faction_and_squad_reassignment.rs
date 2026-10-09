//! Attributes modal batch faction and squad reassignment tests.

use mission_operations::reassign::plan_reassign;
use mission_operations::rows::FactionRow;
use mission_operations::rows::SquadRow;

fn rows() -> (Vec<FactionRow>, Vec<SquadRow>) {
    let squad = |id: &str, name: &str, faction: &str| SquadRow {
        id: id.into(),
        name: name.to_string(),
        faction_id: faction.into(),
        slot_ids: vec!["s1".to_string()],
        leader_slot_id: String::new().into(),
        vehicle_ids: Vec::new(),
    };
    (
        vec![
            FactionRow {
                id: "faction-BLUFOR".into(),
                key: "BLUFOR".to_string(),
                name: "US Army".to_string(),
                squad_ids: vec!["sq-a".to_string(), "sq-c".to_string()],
            },
            FactionRow {
                id: "faction-OPFOR".into(),
                key: "OPFOR".to_string(),
                name: "Soviet Army".to_string(),
                squad_ids: vec!["sq-b".to_string()],
            },
            FactionRow {
                id: "faction-EMPTY".into(),
                key: "INDFOR".to_string(),
                name: "Militia".to_string(),
                squad_ids: Vec::new(),
            },
        ],
        vec![
            squad("sq-a", "Alpha", "faction-BLUFOR"),
            squad("sq-b", "Bravo", "faction-OPFOR"),
            squad("sq-c", "Charlie", "faction-BLUFOR"),
        ],
    )
}

#[test]
fn a_squad_of_another_faction_is_refused_and_the_reason_names_squad_and_both_factions() {
    let (factions, squads) = rows();
    let why = plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-b")
        .expect_err("a squad under OPFOR must be refused for a BLUFOR pick");
    for needle in ["Bravo", "Soviet Army", "OPFOR", "US Army", "BLUFOR"] {
        assert!(
            why.contains(needle),
            "T-939.2: the refusal must name {needle}; got: {why}"
        );
    }
    assert!(plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-b").is_err());
}

#[test]
fn a_squad_of_the_picked_faction_resolves_to_itself() {
    let (factions, squads) = rows();
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-c"),
        Ok("sq-c".to_string())
    );
}

#[test]
fn picking_a_faction_alone_resolves_to_that_factions_first_squad_in_doc_order() {
    let (mut factions, squads) = rows();
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", ""),
        Ok("sq-a".to_string())
    );
    factions[0].squad_ids = vec!["sq-c".to_string(), "sq-a".to_string()];
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", ""),
        Ok("sq-c".to_string())
    );
    factions[0].squad_ids = vec!["sq-deleted".to_string(), "sq-a".to_string()];
    assert_eq!(
        plan_reassign(&factions, &squads, "faction-BLUFOR", ""),
        Ok("sq-a".to_string())
    );
}

#[test]
fn a_faction_with_no_squads_refuses_by_name_and_says_what_to_do() {
    let (factions, squads) = rows();
    let why = plan_reassign(&factions, &squads, "faction-EMPTY", "")
        .expect_err("a faction with no squads has no destination");
    assert!(
        why.contains("Militia") && why.contains("INDFOR"),
        "T-939.2: the refusal must name the faction; got: {why}"
    );
    assert!(
        why.to_lowercase().contains("orbat"),
        "T-939.2: the refusal must point at where squads are made; got: {why}"
    );
}

#[test]
fn a_destination_that_no_longer_exists_refuses_rather_than_guessing() {
    let (factions, squads) = rows();
    assert!(plan_reassign(&factions, &squads, "faction-GONE", "").is_err());
    assert!(plan_reassign(&factions, &squads, "faction-BLUFOR", "sq-gone").is_err());
}
