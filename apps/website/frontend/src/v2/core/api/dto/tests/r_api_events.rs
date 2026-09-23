//! Captured-response round trips for operations, orders of battle and armouries.

use super::*;

fn reservation_dossier_wire() -> Value {
    serde_json::json!({
        "armory_by_faction": [],
        "event_mission_id": "dca49443-49eb-4ac2-8dc9-4f67a302d813",
        "factions": ["blue"],
        "filled": 0,
        "game_mode": "pve_coop",
        "mission_id": "26e65c20-305d-4467-af3c-4a643b8ac540",
        "start_time": "2026-09-22T18:00:00Z",
        "terrain": "everon",
        "title": "Reservation and attendance",
        "total": 1,
        "viewer_eligible": true,
        "my_state": "attended"
    })
}

#[test]
fn event_dossier_absent_reservation_and_attendance_fields_round_trip() {
    let wire = reservation_dossier_wire();
    let parsed: EventMissionDossier = serde_json::from_value(wire.clone()).unwrap();
    assert!(parsed.my_reservation_state.is_none());
    assert!(parsed.my_attendance_state.is_none());
    assert_eq!(parsed.my_state.as_deref(), Some("attended"));
    assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
}

#[test]
fn event_dossier_null_state_fields_normalize_to_absent_without_affecting_each_other() {
    for (reservation, attendance) in [
        (Value::Null, Value::Null),
        (Value::Null, serde_json::json!("attended")),
        (serde_json::json!("withdrawn"), Value::Null),
    ] {
        let mut wire = reservation_dossier_wire();
        wire["my_reservation_state"] = reservation.clone();
        wire["my_attendance_state"] = attendance.clone();
        let parsed: EventMissionDossier = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed.my_reservation_state.as_deref(), reservation.as_str());
        assert_eq!(parsed.my_attendance_state.as_deref(), attendance.as_str());
        let canonical = serde_json::to_value(&parsed).unwrap();
        for name in ["my_reservation_state", "my_attendance_state"] {
            if wire[name].is_null() {
                wire.as_object_mut().unwrap().remove(name);
            }
        }
        assert_eq!(canonical, wire);
        let reparsed: EventMissionDossier = serde_json::from_value(canonical).unwrap();
        assert!(reparsed == parsed);
    }
}

#[test]
fn event_dossier_reservation_and_attendance_states_round_trip_independently() {
    for reservation in ["registered", "waitlisted", "withdrawn", "legacy_unknown"] {
        for attendance in ["attended", "no_show"] {
            let mut wire = reservation_dossier_wire();
            wire["my_reservation_state"] = serde_json::json!(reservation);
            wire["my_attendance_state"] = serde_json::json!(attendance);
            let parsed: EventMissionDossier = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(parsed.my_reservation_state.as_deref(), Some(reservation));
            assert_eq!(parsed.my_attendance_state.as_deref(), Some(attendance));
            assert_eq!(parsed.my_state.as_deref(), Some("attended"));
            assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
            assert!(unclaimed_keys::<EventMissionDossier>(&wire.to_string()).is_empty());
        }
    }
}

#[test]
fn event_dossier_unknown_state_strings_preserve_exact_wire_values() {
    for (reservation, attendance) in [
        ("future_reservation", "future_attendance"),
        ("", ""),
        ("Registered", "NoShow"),
    ] {
        let mut wire = reservation_dossier_wire();
        wire["my_reservation_state"] = serde_json::json!(reservation);
        wire["my_attendance_state"] = serde_json::json!(attendance);
        let parsed: EventMissionDossier = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(parsed.my_reservation_state.as_deref(), Some(reservation));
        assert_eq!(parsed.my_attendance_state.as_deref(), Some(attendance));
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    }
}

#[test]
fn event_dossier_state_fields_reject_non_string_non_null_values() {
    for field in ["my_reservation_state", "my_attendance_state"] {
        for invalid in [
            serde_json::json!(0),
            serde_json::json!(false),
            serde_json::json!([]),
            serde_json::json!({}),
        ] {
            let mut wire = reservation_dossier_wire();
            wire[field] = invalid;
            assert!(serde_json::from_value::<EventMissionDossier>(wire).is_err());
        }
    }
}

#[test]
fn deployments() {
    // Upcoming reservations are typed; only the separate service-history rows remain opaque.
    assert_golden::<Deployments>(golden!("GET__me__deployments.json"), &["service_history/*"]);
}

/// The armoury list used to be empty in the only captured operation, so the armoury types had no
/// structural coverage at all: skipping every named field still left this test green. The golden
/// now carries real rows, in the shape the detail route actually emits.
///
/// Named fields claim `faction`/`items`/`id`/`item_name`/`quantity`. The rest of each
/// armory row rides `ArmoryItem::extra` — that inventory is what stands between the
/// catch-all and a silent field drop on the named three.
const EVENT_HUB_ARMORY_EXTRA: &[&str] = &[
    "missions/*/armory_by_faction/*/items/*/category",
    "missions/*/armory_by_faction/*/items/*/faction",
    "missions/*/armory_by_faction/*/items/*/icon",
    "missions/*/armory_by_faction/*/items/*/mission_id",
    "missions/*/armory_by_faction/*/items/*/sort_order",
];

#[test]
fn event_hub() {
    const G: &str = golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json");
    assert_golden::<EventHub>(G, EVENT_HUB_ARMORY_EXTRA);
    // Anti-vacuous: the corpus must actually exercise the armory DTOs.
    let hub: EventHub = serde_json::from_str(G).unwrap();
    assert!(
        !hub.missions.is_empty() && !hub.missions[0].armory_by_faction.is_empty(),
        "event-hub golden must carry non-empty armory_by_faction"
    );
    assert!(
        hub.missions[0]
            .armory_by_faction
            .iter()
            .any(|f| !f.items.is_empty()),
        "at least one faction must carry non-empty items"
    );
    let item = &hub.missions[0].armory_by_faction[0].items[0];
    assert!(
        !item.id.is_empty() && !item.item_name.is_empty(),
        "ArmoryItem named fields must round-trip populated values"
    );
    assert!(
        item.quantity.is_some(),
        "quantity must be present on a golden row"
    );
}

/// The viewer half of the captured dossier is typed, populated and in the documented order: the
/// structural gate above only proves the keys are read, not that the capture exercises them.
#[test]
fn event_hub_viewer_access_and_pools_are_populated() {
    let hub: EventHub = serde_json::from_str(golden!(
        "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
    ))
    .unwrap();
    assert_eq!(hub.viewer_access.visibility, "full");
    assert_eq!(hub.viewer_access.quota_class, "guest");
    assert!(!hub.viewer_access.membership_verification_pending);
    let kinds: Vec<&str> = hub
        .reservation_quotas
        .iter()
        .map(|pool| pool.quota_kind.as_str())
        .collect();
    assert_eq!(
        kinds,
        ["member", "guest", "open"],
        "pools arrive in a fixed order"
    );
    // One uncapped pool, one capped with places left, one closed with none: every branch the
    // availability panel renders is in the capture.
    let [member, guest, open] = [0, 1, 2].map(|i| &hub.reservation_quotas[i]);
    assert_eq!((member.seat_limit, member.remaining), (None, None));
    assert_eq!((guest.seat_limit, guest.remaining), (Some(2), Some(1)));
    assert_eq!(open.closed_reason.as_deref(), Some("no_places"));
    assert!(member.open && guest.open && !open.open);
    assert_eq!(
        hub.remaining_event_places, None,
        "the captured operation is uncapped"
    );
    assert!(hub.missions[0].viewer_eligible);
}

/// An uncapped operation's remaining places cross the wire as an explicit null, and a capped one's
/// as a number; both round-trip exactly, so a skipped key would show up here as drift.
#[test]
fn remaining_event_places_keeps_its_explicit_null() {
    let mut wire: Value = serde_json::from_str(golden!(
        "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
    ))
    .unwrap();
    for places in [Value::Null, serde_json::json!(0), serde_json::json!(12)] {
        wire["remaining_event_places"] = places.clone();
        let hub: EventHub = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(hub.remaining_event_places, places.as_i64());
        assert_eq!(serde_json::to_value(&hub).unwrap(), wire);
    }
}

/// The released-signup tombstone and the waiting position are absent from the capture, so they
/// are exercised on the capture's own mission with each one set: every one is a named field, and
/// each round-trips exactly.
#[test]
fn event_dossier_release_and_waiting_fields_are_named_and_round_trip() {
    let mut wire = reservation_dossier_wire();
    wire["my_reservation_state"] = serde_json::json!("withdrawn");
    wire["my_release_reason"] = serde_json::json!("access_policy_changed");
    wire["my_withdrawn_at"] = serde_json::json!("2026-07-20T18:30:00Z");
    let released: EventMissionDossier = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        released.my_release_reason.as_deref(),
        Some("access_policy_changed")
    );
    assert_eq!(
        released.my_withdrawn_at.as_deref(),
        Some("2026-07-20T18:30:00Z")
    );
    assert_eq!(serde_json::to_value(&released).unwrap(), wire);
    assert!(unclaimed_keys::<EventMissionDossier>(&wire.to_string()).is_empty());

    let mut waiting = reservation_dossier_wire();
    waiting["my_reservation_state"] = serde_json::json!("waitlisted");
    waiting["my_waiting_position"] = serde_json::json!(3);
    let queued: EventMissionDossier = serde_json::from_value(waiting.clone()).unwrap();
    assert_eq!(queued.my_waiting_position, Some(3));
    assert_eq!(serde_json::to_value(&queued).unwrap(), waiting);
    assert!(unclaimed_keys::<EventMissionDossier>(&waiting.to_string()).is_empty());
}

/// `viewer_eligible` is required: a dossier without it is not this contract's dossier.
#[test]
fn event_dossier_requires_viewer_eligibility() {
    let mut wire = reservation_dossier_wire();
    wire.as_object_mut().unwrap().remove("viewer_eligible");
    assert!(serde_json::from_value::<EventMissionDossier>(wire).is_err());
}

/// Frozen proof that skipping every named field on the armoury types is visible once the golden is
/// non-empty — the same claimed-versus-absorbed pair as the harness's own control, applied here.
#[test]
fn armory_skip_all_fields_is_visible_under_populated_golden() {
    const FACTION: &str = r#"{
        "faction":"BLUFOR",
        "items":[{
            "id":"a1000000-0000-4000-8000-000000000001",
            "mission_id":"512d8658-7025-4a70-94e9-a1b44a7aa155",
            "faction":"BLUFOR",
            "category":"rifle",
            "item_name":"M4A1",
            "quantity":24,
            "icon":"m4.png",
            "sort_order":0
        }]
    }"#;
    const ITEM: &str = r#"{
        "id":"a1000000-0000-4000-8000-000000000001",
        "mission_id":"512d8658-7025-4a70-94e9-a1b44a7aa155",
        "faction":"BLUFOR",
        "category":"rifle",
        "item_name":"M4A1",
        "quantity":24,
        "icon":"m4.png",
        "sort_order":0
    }"#;

    #[derive(Serialize, Deserialize)]
    struct AbsorbedArmoryItem {
        #[serde(skip)]
        #[allow(dead_code)]
        id: String,
        #[serde(skip)]
        #[allow(dead_code)]
        item_name: String,
        #[serde(skip)]
        #[allow(dead_code)]
        quantity: Option<i64>,
        #[serde(flatten)]
        extra: serde_json::Map<String, Value>,
    }

    #[derive(Serialize, Deserialize)]
    struct AbsorbedArmoryFaction {
        #[serde(skip)]
        #[allow(dead_code)]
        faction: String,
        #[serde(skip)]
        #[allow(dead_code)]
        items: Vec<AbsorbedArmoryItem>,
        #[serde(flatten)]
        extra: serde_json::Map<String, Value>,
    }

    // Live DTOs claim the named keys; extras ride the flatten.
    assert_eq!(
        unclaimed_keys::<ArmoryFaction>(FACTION),
        vec![
            "items/*/category".to_string(),
            "items/*/faction".to_string(),
            "items/*/icon".to_string(),
            "items/*/mission_id".to_string(),
            "items/*/sort_order".to_string(),
        ]
    );
    assert_eq!(
        unclaimed_keys::<ArmoryItem>(ITEM),
        vec![
            "category".to_string(),
            "faction".to_string(),
            "icon".to_string(),
            "mission_id".to_string(),
            "sort_order".to_string(),
        ]
    );

    // Total skip: every wire key is only swept into `extra`. Byte-equality stays green;
    // the structural half is what fails the Event Hub gate if this ever lands on the
    // real DTOs against the populated golden.
    assert_canonical_round_trip::<AbsorbedArmoryFaction>(FACTION);
    assert_canonical_round_trip::<AbsorbedArmoryItem>(ITEM);
    assert_eq!(
        unclaimed_keys::<AbsorbedArmoryFaction>(FACTION),
        vec!["faction".to_string(), "items".to_string()],
        "skip-all ArmoryFaction: faction+items must show as unclaimed"
    );
    assert_eq!(
        unclaimed_keys::<AbsorbedArmoryItem>(ITEM),
        vec![
            "category".to_string(),
            "faction".to_string(),
            "icon".to_string(),
            "id".to_string(),
            "item_name".to_string(),
            "mission_id".to_string(),
            "quantity".to_string(),
            "sort_order".to_string(),
        ],
        "skip-all ArmoryItem: every wire key must show as unclaimed"
    );
}

/// Typed as the order-of-battle selector reads it. Typing it immediately failed, and found the
/// seat's assignment field skipping a key the backend emits as an explicit null.
#[test]
fn orbat_envelope() {
    assert_golden::<DataEnvelope<OrbatSquad>>(
        golden!("GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json"),
        &[],
    );
}

/// The captured order of battle carries eligible and restricted seats, and seats decided by each of
/// the three policy sources, so the seat rendering is exercised on every branch it has.
#[test]
fn orbat_seats_carry_viewer_eligibility_from_every_policy_source() {
    let orbat: DataEnvelope<OrbatSquad> = serde_json::from_str(golden!(
        "GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json"
    ))
    .unwrap();
    let seats: Vec<&OrbatSlot> = orbat.data.iter().flat_map(|squad| &squad.slots).collect();
    for access in ["eligible", "restricted"] {
        assert!(
            seats.iter().any(|seat| seat.viewer_access == access),
            "the capture must carry a {access} seat"
        );
    }
    for source in ["event", "squad", "slot"] {
        assert!(
            seats.iter().any(|seat| seat.policy_source == source),
            "the capture must carry a seat decided by the {source} policy"
        );
    }
}

/// One promotion answer as the backend served it: the promoted waiter, the seat they were given.
#[test]
fn waitlist_promotion() {
    const G: &str = golden!("POST__event-missions__waitlist__promote.json");
    assert_golden::<WaitlistPromotion>(G, &[]);
    let promotion: WaitlistPromotion = serde_json::from_str(G).unwrap();
    assert_eq!(promotion.promoted.len(), 1);
    assert!(!promotion.promoted[0].slot_id.is_empty());
}

// ── paginated `{data,total,limit,offset}` envelopes (item type ported per page) ──
/// Typed as the event manager reads it. Typing it pinned the filled percentage as the whole number
/// the backend actually sends, which a float field re-serialised with a decimal point.
#[test]
fn events_envelope() {
    // The three row keys `EventListItem` does not name; they ride the `extra` catch-all.
    assert_golden::<Paginated<EventListItem>>(
        golden!("GET__events.json"),
        &[
            "data/*/created_at",
            "data/*/created_by",
            "data/*/updated_at",
        ],
    );
}

/// The caller's own leave panel. A bare `{data: […]}` envelope, not the paginated one the admin
/// queue uses — `list_my_leave` and `list_all_leave` really do differ there.
#[test]
fn my_leave_requests_envelope() {
    assert_golden::<DataEnvelope<LeaveRequest>>(golden!("GET__me__leave-requests.json"), &[]);
}

/// The rows belong to the seeded operator. A corpus keyed to anyone else would render another
/// member's leave on the caller's own panel and still look perfectly healthy.
#[test]
fn my_leave_requests_belong_to_the_seeded_operator() {
    let me: Value = serde_json::from_str(golden!("GET__me.json")).unwrap();
    let seeded = me["user"]["discord_id"]
        .as_str()
        .expect("seeded discord_id");
    let mine: DataEnvelope<LeaveRequest> =
        serde_json::from_str(golden!("GET__me__leave-requests.json")).unwrap();
    assert!(!mine.data.is_empty(), "the personal panel must have rows");
    for row in &mine.data {
        assert_eq!(row.discord_id, seeded);
    }
}

#[test]
fn admin_leave_queue_envelope() {
    assert_golden::<Paginated<LeaveRequest>>(golden!("GET__admin__leave-requests.json"), &[]);
}

/// The queue renders a chip per status and approve/deny controls only on `pending`. A corpus with
/// one status leaves the other branches unrendered and therefore unguarded.
#[test]
fn admin_leave_queue_exercises_every_status_branch() {
    let queue: Paginated<LeaveRequest> =
        serde_json::from_str(golden!("GET__admin__leave-requests.json")).unwrap();
    let statuses: Vec<&str> = queue.data.iter().map(|r| r.status.as_str()).collect();
    for expected in ["pending", "approved", "denied"] {
        assert!(
            statuses.contains(&expected),
            "the queue corpus must carry a {expected} row, got {statuses:?}"
        );
    }
    assert_eq!(
        queue.total,
        queue.data.len() as i64,
        "the envelope total must describe the rows it carries"
    );
    // A reviewed row records who reviewed it; a pending one cannot have.
    for row in &queue.data {
        assert_eq!(
            row.status == "pending",
            row.reviewed_by.is_none(),
            "row {} has status {} and reviewed_by {:?}",
            row.id,
            row.status,
            row.reviewed_by
        );
    }
}

/// A `date` column crosses the wire as a full midnight-UTC timestamp (the backend's
/// `rfc3339_utc_date` spelling), not a bare `YYYY-MM-DD`.
/// The DTO carries both spellings as `String`, so only an assertion catches the wrong one.
#[test]
fn leave_dates_are_the_backend_midnight_utc_spelling() {
    let queue: Paginated<LeaveRequest> =
        serde_json::from_str(golden!("GET__admin__leave-requests.json")).unwrap();
    for row in &queue.data {
        for (field, value) in [("starts_on", &row.starts_on), ("ends_on", &row.ends_on)] {
            assert!(
                value.ends_with("T00:00:00Z") && value.len() == 20,
                "{field} must be the midnight-UTC timestamp spelling, got {value}"
            );
        }
    }
}

#[test]
fn reservation_response() {
    assert_golden::<ReservationResponse>(golden!("POST__event-missions__register.json"), &[]);
}
