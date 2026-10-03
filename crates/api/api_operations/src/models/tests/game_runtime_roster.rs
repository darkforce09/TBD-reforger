//! The roster's `missionId` serialises byte-for-byte as the plain string it was: `""` when the
//! server runs no mission of the event, the bare hyphenated UUID when it runs one.

use super::*;
use uuid::Uuid;

/// The roster wire as a plain-string `missionId`: the shape the game runtime's
/// `JsonLoadContext` binds, against which the typed field is compared.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlainTextRoster {
    version: u32,
    event_id: Uuid,
    mission_id: String,
    assignments: Vec<RosterAssignment>,
    slots: Vec<RosterSlot>,
}

fn seated_roster(event: Uuid, mission_id: Option<MissionId>) -> EventRoster {
    let event_mission = EventMissionId::from(Uuid::from_u128(3));
    let orbat_slot = OrbatSlotId::from(Uuid::from_u128(4));
    EventRoster {
        version: 2,
        event_id: EventId::from(event),
        mission_id,
        assignments: vec![RosterAssignment {
            arma_id: ArmaPlayerId::new("arma-1"),
            slot_uid: MissionSlotUid::new("slot-a"),
            orbat_slot_id: orbat_slot,
            event_mission_id: event_mission,
        }],
        slots: vec![RosterSlot {
            event_mission_id: event_mission,
            slot_uid: MissionSlotUid::new("slot-a"),
            orbat_slot_id: orbat_slot,
        }],
    }
}

fn plain_text_roster(event: Uuid, mission_id: String) -> PlainTextRoster {
    let typed = seated_roster(event, None);
    PlainTextRoster {
        version: typed.version,
        event_id: event,
        mission_id,
        assignments: typed.assignments,
        slots: typed.slots,
    }
}

#[test]
fn event_roster_without_a_mission_writes_the_empty_string() {
    let event = Uuid::from_u128(1);
    let typed = serde_json::to_string(&seated_roster(event, None)).unwrap();
    let plain = serde_json::to_string(&plain_text_roster(event, String::new())).unwrap();
    assert_eq!(typed, plain);
    assert!(typed.contains(r#""missionId":"""#), "{typed}");
}

#[test]
fn event_roster_with_a_mission_writes_the_bare_uuid() {
    let (event, mission) = (Uuid::from_u128(1), Uuid::new_v4());
    let typed =
        serde_json::to_string(&seated_roster(event, Some(MissionId::from(mission)))).unwrap();
    let plain = serde_json::to_string(&plain_text_roster(event, mission.to_string())).unwrap();
    assert_eq!(typed, plain);
    assert!(
        typed.contains(&format!(r#""missionId":"{mission}""#)),
        "{typed}"
    );
}
