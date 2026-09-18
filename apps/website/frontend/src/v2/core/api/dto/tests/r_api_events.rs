//! Captured-response round trips for operations, orders of battle and armouries.

use super::*;

#[test]
fn deployments() {
    // Both lists are `Vec<Value>` — the service-record rows and the upcoming ops are not
    // ported types yet, so nothing below them is asserted.
    assert_golden::<Deployments>(
        golden!("GET__me__deployments.json"),
        &["service_history/*", "upcoming/*"],
    );
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

/// A `date` column crosses the wire as full midnight UTC (`go_date`), not a bare `YYYY-MM-DD`.
/// The DTO carries both spellings as `String`, so only an assertion catches the wrong one.
#[test]
fn leave_dates_are_the_backend_midnight_utc_spelling() {
    let queue: Paginated<LeaveRequest> =
        serde_json::from_str(golden!("GET__admin__leave-requests.json")).unwrap();
    for row in &queue.data {
        for (field, value) in [("starts_on", &row.starts_on), ("ends_on", &row.ends_on)] {
            assert!(
                value.ends_with("T00:00:00Z") && value.len() == 20,
                "{field} must be go_date-spelled, got {value}"
            );
        }
    }
}
