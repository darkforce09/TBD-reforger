//! Captured-response round trips for access administration, and the shapes of its change requests.

use super::*;

const ACCESS: &str = golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access.json");
const PARTICIPANTS: &str =
    golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access__participants.json");
const POLICY_CHANGE: &str = golden!("PUT__events__access-policy.json");

/// The whole access view is claimed: policies, groups, pools and usage are all named fields.
#[test]
fn event_access_administration() {
    assert_golden::<EventAccessAdministration>(ACCESS, &[]);
}

/// The capture exercises every tagged shape the panel edits: three condition kinds, both group
/// sources, a capped, an uncapped and a closed pool, and one squad and one slot override.
#[test]
fn event_access_capture_exercises_every_shape_the_panel_edits() {
    let access: EventAccessAdministration = serde_json::from_str(ACCESS).unwrap();
    let conditions: Vec<&EventAccessCondition> = std::iter::once(&access.event_policy)
        .chain(access.squad_policies.iter().map(|squad| &squad.policy))
        .chain(access.slot_policies.iter().map(|slot| &slot.policy))
        .flat_map(|policy| &policy.grants)
        .flat_map(|grant| &grant.conditions)
        .collect();
    assert!(conditions.contains(&&EventAccessCondition::TbdMember {}));
    assert!(conditions
        .iter()
        .any(|c| matches!(c, EventAccessCondition::EventGroup { .. })));
    assert!(conditions
        .iter()
        .any(|c| matches!(c, EventAccessCondition::NamedAccount { .. })));
    assert!(access
        .groups
        .iter()
        .any(|g| g.source == EventGroupSource::ManagedRoster {} && !g.roster.is_empty()));
    assert!(access.groups.iter().any(|g| matches!(
        &g.source,
        EventGroupSource::PartnerGuild { required_role_ids, .. } if !required_role_ids.is_empty()
    ) && g.roster.is_empty()));
    let pools = &access.reservation_quotas;
    assert_eq!(
        (pools.member.seats, pools.guest.seats, pools.open.seats),
        (None, Some(2), Some(0))
    );
    assert_eq!(access.squad_policies.len(), 1);
    assert_eq!(access.slot_policies.len(), 1);
    assert_eq!(access.quota_usage.total, 5);
}

/// A condition or group source whose kind this build does not know fails to load, rather than
/// being read as something else and rewritten by the next save.
#[test]
fn an_unknown_condition_kind_or_field_is_refused() {
    for unknown in [
        serde_json::json!({"kind": "rank_at_least", "rank": 3}),
        serde_json::json!({"kind": "tbd_member", "since": "2026-01-01T00:00:00Z"}),
        serde_json::json!({"kind": "discord_role", "guild_id": "1"}),
    ] {
        assert!(
            serde_json::from_value::<EventAccessCondition>(unknown.clone()).is_err(),
            "{unknown} must not deserialise"
        );
    }
    assert!(serde_json::from_value::<EventGroupSource>(
        serde_json::json!({"kind": "managed_roster", "guild_id": "1"})
    )
    .is_err());
}

/// Each condition kind serialises to exactly the tagged object the backend's enum reads.
#[test]
fn conditions_serialise_to_the_backend_tagged_shape() {
    let cases = [
        (
            EventAccessCondition::Authenticated {},
            serde_json::json!({"kind": "authenticated"}),
        ),
        (
            EventAccessCondition::TbdMember {},
            serde_json::json!({"kind": "tbd_member"}),
        ),
        (
            EventAccessCondition::DiscordRole {
                guild_id: "100000000000000777".into(),
                role_id: "200000000000000888".into(),
            },
            serde_json::json!({
                "kind": "discord_role",
                "guild_id": "100000000000000777",
                "role_id": "200000000000000888"
            }),
        ),
        (
            EventAccessCondition::EventGroup {
                group_id: "00000000-0000-4000-b100-000000000001".into(),
            },
            serde_json::json!({
                "kind": "event_group",
                "group_id": "00000000-0000-4000-b100-000000000001"
            }),
        ),
        (
            EventAccessCondition::NamedAccount {
                discord_id: "000000000000000006".into(),
            },
            serde_json::json!({"kind": "named_account", "discord_id": "000000000000000006"}),
        ),
    ];
    for (condition, wire) in cases {
        assert_eq!(serde_json::to_value(&condition).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<EventAccessCondition>(wire).unwrap(),
            condition
        );
    }
}

/// Every participant's evidence is claimed, including the explicit nulls of a waiting participant
/// who holds no place.
#[test]
fn access_participants() {
    assert_golden::<Vec<ParticipantAccessExplanation>>(PARTICIPANTS, &[]);
    let participants: Vec<ParticipantAccessExplanation> =
        serde_json::from_str(PARTICIPANTS).unwrap();
    assert!(participants.iter().any(|p| p.allocation.is_none()));
    assert!(participants.iter().any(|p| p
        .registrations
        .iter()
        .any(|r| r.reservation_state == "waitlisted" && r.slot_id.is_none())));
    assert!(participants
        .iter()
        .all(|p| !p.guilds.is_empty() && !p.roster_groups.is_empty()));
}

/// A change answers the access view after it, and what it released and promoted.
#[test]
fn access_policy_change_outcome() {
    assert_golden::<AccessChangeOutcome>(POLICY_CHANGE, &[]);
    let outcome: AccessChangeOutcome = serde_json::from_str(POLICY_CHANGE).unwrap();
    let before: EventAccessAdministration = serde_json::from_str(ACCESS).unwrap();
    assert_eq!(
        outcome.access.access_revision,
        before.access_revision + 1,
        "an accepted change advances the revision by one"
    );
}

/// Released and promoted registrations are strings the structural gate reads, not opaque values:
/// the capture's lists are empty, so they are exercised here populated.
#[test]
fn change_outcome_registration_lists_are_named_fields() {
    let mut wire: Value = serde_json::from_str(POLICY_CHANGE).unwrap();
    wire["released_registrations"] = serde_json::json!(["00000000-0000-4000-a100-000000000004"]);
    wire["promoted_registrations"] = serde_json::json!(["00000000-0000-4000-a100-000000000006"]);
    let text = wire.to_string();
    assert_golden::<AccessChangeOutcome>(&text, &[]);
    let outcome: AccessChangeOutcome = serde_json::from_str(&text).unwrap();
    assert_eq!(outcome.released_registrations.len(), 1);
    assert_eq!(outcome.promoted_registrations.len(), 1);
}

/// The request bodies carry exactly the keys the backend's refusing-unknown-fields models read.
#[test]
fn change_request_bodies_serialise_to_the_backend_shape() {
    let policy = EventAccessPolicy { grants: vec![] };
    assert_eq!(
        serde_json::to_value(AccessPolicyChange {
            expected_access_revision: 4,
            policy,
        })
        .unwrap(),
        serde_json::json!({"expected_access_revision": 4, "policy": {"grants": []}})
    );
    assert_eq!(
        serde_json::to_value(EventGroupCreation {
            expected_access_revision: 4,
            name: "Allies".into(),
            source: EventGroupSource::PartnerGuild {
                guild_id: "7".into(),
                required_role_ids: vec![],
            },
        })
        .unwrap(),
        serde_json::json!({
            "expected_access_revision": 4,
            "name": "Allies",
            "source": {"kind": "partner_guild", "guild_id": "7", "required_role_ids": []}
        })
    );
    // A rename leaves the source out entirely rather than sending a null the backend would refuse.
    assert_eq!(
        serde_json::to_value(EventGroupChange {
            expected_access_revision: 5,
            name: Some("Renamed".into()),
            source: None,
        })
        .unwrap(),
        serde_json::json!({"expected_access_revision": 5, "name": "Renamed"})
    );
    assert_eq!(
        serde_json::to_value(AccessRevisionPrecondition {
            expected_access_revision: 6
        })
        .unwrap(),
        serde_json::json!({"expected_access_revision": 6})
    );
}

/// An uncapped pool is sent as an explicit null: the backend reads an absent limit as a mistake,
/// never as "uncapped".
#[test]
fn quota_change_sends_an_uncapped_pool_as_an_explicit_null() {
    let access: EventAccessAdministration = serde_json::from_str(ACCESS).unwrap();
    let body = serde_json::to_value(ReservationQuotaChange {
        expected_access_revision: access.access_revision,
        reservation_quotas: access.reservation_quotas,
    })
    .unwrap();
    assert_eq!(body["reservation_quotas"]["member"]["seats"], Value::Null);
    assert!(body["reservation_quotas"]["member"]
        .as_object()
        .unwrap()
        .contains_key("seats"));
    assert_eq!(body["reservation_quotas"]["guest"]["seats"], 2);
}

/// The access writes of the corpus, in the order the capture sends them after the policy change.
/// Each names the revision the write before it left, so together they walk the revision forward.
const ACCESS_WRITES: [(&str, &str); 10] = [
    (
        "reservation quotas",
        golden!("PUT__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__reservation-quotas.json"),
    ),
    (
        "group rename",
        golden!("PATCH__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__groups__00000000-0000-4000-b100-000000000001.json"),
    ),
    (
        "roster member kept",
        golden!("PUT__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__groups__00000000-0000-4000-b100-000000000001__members__000000000000000002.json"),
    ),
    (
        "roster member removed",
        golden!("DELETE__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__groups__00000000-0000-4000-b100-000000000001__members__000000000000000005.json"),
    ),
    (
        "slot policy set",
        golden!("PUT__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__slots__00000000-0000-4000-5000-000000000010__access-policy.json"),
    ),
    (
        "slot policy removed",
        golden!("DELETE__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__slots__00000000-0000-4000-5000-000000000013__access-policy.json"),
    ),
    (
        "squad policy set",
        golden!("PUT__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__squads__BLUFOR__Bravo__access-policy.json"),
    ),
    (
        "squad policy removed",
        golden!("DELETE__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__squads__OPFOR__Recon__access-policy.json"),
    ),
    (
        "group removed",
        golden!("DELETE__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__groups__00000000-0000-4000-b100-000000000002.json"),
    ),
    (
        "group created",
        golden!("POST__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__groups.json"),
    ),
];

/// The outcome of the write named `name` in [`ACCESS_WRITES`].
fn access_write(name: &str) -> AccessChangeOutcome {
    let (_, golden) = ACCESS_WRITES
        .iter()
        .find(|(write, _)| *write == name)
        .unwrap_or_else(|| panic!("no access write named {name}"));
    serde_json::from_str(golden).unwrap()
}

/// Every access write answers the whole view with every key claimed, and each accepted write
/// advances the revision by exactly one from the revision the policy change left.
#[test]
fn every_access_write_outcome_is_claimed_and_advances_the_revision() {
    let policy_change: AccessChangeOutcome = serde_json::from_str(POLICY_CHANGE).unwrap();
    let mut revision = policy_change.access.access_revision;
    for (name, golden) in ACCESS_WRITES {
        assert_golden::<AccessChangeOutcome>(golden, &[]);
        let outcome: AccessChangeOutcome = serde_json::from_str(golden).unwrap();
        revision += 1;
        assert_eq!(outcome.access.access_revision, revision, "{name}");
        assert!(outcome.promoted_registrations.is_empty(), "{name}");
    }
}

/// The pool write stores the three pools as sent: a capped guest pool, an uncapped member pool and
/// an open pool of one seat.
#[test]
fn reservation_quota_write_answers_the_stored_pools() {
    let pools = access_write("reservation quotas").access.reservation_quotas;
    assert_eq!(
        (pools.member.seats, pools.guest.seats, pools.open.seats),
        (None, Some(3), Some(1))
    );
}

/// Renaming a group keeps its roster; re-adding a member already on the roster changes nothing
/// but the revision; removing a member drops them from the roster and releases their seat.
#[test]
fn group_and_roster_writes_answer_the_changed_groups() {
    let roster = |outcome: &AccessChangeOutcome| -> Vec<String> {
        let group = outcome
            .access
            .groups
            .iter()
            .find(|g| g.id == "00000000-0000-4000-b100-000000000001")
            .unwrap();
        group.roster.iter().map(|r| r.discord_id.clone()).collect()
    };
    let renamed = access_write("group rename");
    assert!(renamed
        .access
        .groups
        .iter()
        .any(|g| g.name == "Byte Parity main roster"));
    let kept = access_write("roster member kept");
    assert_eq!(roster(&kept), roster(&renamed));
    let removed = access_write("roster member removed");
    assert!(!roster(&removed).contains(&"000000000000000005".to_string()));
    assert_eq!(roster(&removed).len() + 1, roster(&kept).len());
    assert_eq!(
        removed.released_registrations,
        ["00000000-0000-4000-a100-000000000005"]
    );
    assert_eq!(
        removed.access.quota_usage.total + 1,
        kept.access.quota_usage.total
    );
    let without_partner = access_write("group removed");
    assert!(without_partner
        .access
        .groups
        .iter()
        .all(|g| g.id != "00000000-0000-4000-b100-000000000002"));
}

/// Setting a slot or squad policy adds its override to the view; removing one drops it.
#[test]
fn slot_and_squad_policy_writes_answer_the_changed_overrides() {
    let slots = |name: &str| -> Vec<String> {
        let outcome = access_write(name);
        outcome
            .access
            .slot_policies
            .into_iter()
            .map(|p| p.slot_id)
            .collect()
    };
    assert!(slots("slot policy set").contains(&"00000000-0000-4000-5000-000000000010".to_string()));
    assert!(
        !slots("slot policy removed").contains(&"00000000-0000-4000-5000-000000000013".to_string())
    );
    let squads = |name: &str| -> Vec<(String, String)> {
        let outcome = access_write(name);
        outcome
            .access
            .squad_policies
            .into_iter()
            .map(|p| (p.faction, p.squad))
            .collect()
    };
    let set = squads("squad policy set");
    assert!(set.contains(&("BLUFOR".to_string(), "Bravo".to_string())));
    let removed = squads("squad policy removed");
    assert_eq!(removed, [("BLUFOR".to_string(), "Bravo".to_string())]);
}

/// A created group joins the view after the groups created before it, as an empty managed roster.
/// Its id and creation time are generated by the server, so the golden holds the normalisation
/// placeholders there.
#[test]
fn group_creation_appends_an_empty_managed_roster() {
    let groups = access_write("group created").access.groups;
    let created = groups.last().expect("the created group is listed");
    assert_eq!(created.name, "Allied partner roster");
    assert!(created.roster.is_empty());
    assert_eq!(created.id, "99999999-9999-4999-9999-999999999999");
}
