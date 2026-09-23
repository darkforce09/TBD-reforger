//! Seat binding against a template and a compiled document: a one-to-one correspondence binds
//! every seat; every other shape is refused with each unpaired seat and slot named.

use serde_json::json;
use uuid::Uuid;

use super::{BindingMismatch, CompiledSlot, OrbatSeat, bind_seats, compiled_slots};
use crate::operations::services::{OrbatSlotTemplate, OrbatSquadTemplate};

fn squad(faction: &str, name: &str, roles: &[&str]) -> OrbatSquadTemplate {
    OrbatSquadTemplate {
        faction: faction.into(),
        callsign: String::new(),
        squad: name.into(),
        slots: roles
            .iter()
            .map(|role| OrbatSlotTemplate {
                role: (*role).into(),
                loadout: String::new(),
                tag: String::new(),
            })
            .collect(),
    }
}

fn slot(uid: &str, role: &str) -> CompiledSlot {
    CompiledSlot {
        uid: uid.into(),
        role: role.into(),
    }
}

fn seat(faction: &str, squad: &str, index: i64, role: &str) -> OrbatSeat {
    OrbatSeat {
        id: Uuid::new_v4(),
        faction: faction.into(),
        squad: squad.into(),
        slot_index: index,
        role: role.into(),
    }
}

fn two_squads() -> (Vec<OrbatSquadTemplate>, Vec<CompiledSlot>) {
    (
        vec![
            squad("BLUFOR", "Alpha", &["SL", "Rifleman"]),
            squad("OPFOR", "Alpha", &["SL"]),
        ],
        vec![slot("s1", "SL"), slot("s2", "Rifleman"), slot("s3", "SL")],
    )
}

#[test]
fn every_seat_binds_to_the_slot_at_its_faction_squad_and_position() {
    let (template, compiled) = two_squads();
    let seats = vec![
        seat("OPFOR", "Alpha", 0, "SL"),
        seat("BLUFOR", "Alpha", 1, "Rifleman"),
        seat("BLUFOR", "Alpha", 0, "SL"),
    ];
    let bindings = bind_seats(&template, &compiled, &seats).unwrap();
    let uids: Vec<(Uuid, &str)> = bindings
        .iter()
        .map(|(id, uid)| (*id, uid.as_str()))
        .collect();
    assert_eq!(
        uids,
        vec![
            (seats[0].id, "s3"),
            (seats[1].id, "s2"),
            (seats[2].id, "s1")
        ],
        "the same squad name in two factions binds by faction"
    );
}

#[test]
fn an_authored_empty_role_binds_as_the_compiler_writes_it() {
    let template = vec![squad("BLUFOR", "Alpha", &[""])];
    let compiled = vec![slot("s1", "unassigned")];
    let seats = vec![seat("BLUFOR", "Alpha", 0, "")];
    assert_eq!(bind_seats(&template, &compiled, &seats).unwrap()[0].1, "s1");
}

#[test]
fn a_seat_with_another_role_or_position_is_unbound_and_its_slot_unseated() {
    let (template, compiled) = two_squads();
    let seats = vec![
        seat("BLUFOR", "Alpha", 0, "SL"),
        seat("BLUFOR", "Alpha", 1, "Medic"),
        seat("OPFOR", "Alpha", 3, "SL"),
    ];
    let mismatch = bind_seats(&template, &compiled, &seats).unwrap_err();
    assert_eq!(
        mismatch.unbound_seats,
        vec![
            "BLUFOR Alpha position 1 (Medic)",
            "OPFOR Alpha position 3 (SL)"
        ]
    );
    assert_eq!(mismatch.unseated_slots, vec!["s2 (Rifleman)", "s3 (SL)"]);
    assert!(mismatch.document_disagrees.is_none());
}

#[test]
fn fewer_seats_than_slots_leave_slots_unseated() {
    let (template, compiled) = two_squads();
    let seats = vec![seat("BLUFOR", "Alpha", 0, "SL")];
    let mismatch = bind_seats(&template, &compiled, &seats).unwrap_err();
    assert!(mismatch.unbound_seats.is_empty());
    assert_eq!(mismatch.unseated_slots.len(), 2);
}

#[test]
fn two_seats_at_one_position_bind_once() {
    let template = vec![squad("BLUFOR", "Alpha", &["SL"])];
    let compiled = vec![slot("s1", "SL")];
    let seats = vec![
        seat("BLUFOR", "Alpha", 0, "SL"),
        seat("BLUFOR", "Alpha", 0, "SL"),
    ];
    let mismatch = bind_seats(&template, &compiled, &seats).unwrap_err();
    assert_eq!(mismatch.unbound_seats, vec!["BLUFOR Alpha position 0 (SL)"]);
}

#[test]
fn a_template_that_disagrees_with_its_document_is_refused_whole() {
    let (template, mut compiled) = two_squads();
    compiled.pop();
    let mismatch = bind_seats(&template, &compiled, &[]).unwrap_err();
    assert!(mismatch.document_disagrees.unwrap().contains("3 slots"));
    let (template, mut compiled) = two_squads();
    compiled[1].role = "Medic".into();
    let mismatch = bind_seats(&template, &compiled, &[]).unwrap_err();
    assert_eq!(
        mismatch,
        BindingMismatch {
            document_disagrees: Some(
                "Alpha position 1 is Rifleman in the ORBAT and Medic in the document".into()
            ),
            ..BindingMismatch::default()
        }
    );
}

#[test]
fn compiled_slots_read_uid_and_role_in_document_order() {
    let document = json!({ "slots": [{ "uid": "a", "role": "SL" }, { "uid": "b", "role": "TL" }] });
    assert_eq!(
        compiled_slots(&document),
        vec![slot("a", "SL"), slot("b", "TL")]
    );
    assert!(compiled_slots(&json!({})).is_empty());
}
