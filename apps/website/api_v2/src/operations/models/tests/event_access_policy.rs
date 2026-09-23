//! Wire-shape and semantic bounds prevent malformed policies from becoming access grants.

use super::*;
use proptest::prelude::*;
use serde_json::{Value, json};

fn policy_with(condition: Value) -> Value {
    json!({"grants": [{"conditions": [condition]}]})
}

fn validated(value: Value) -> Result<EventAccessPolicy, String> {
    let policy: EventAccessPolicy = serde_json::from_value(value).map_err(|e| e.to_string())?;
    policy.validate().map_err(str::to_owned)?;
    Ok(policy)
}

#[test]
fn default_policy_requires_tbd_membership_and_closed_policy_is_explicit() {
    let default = EventAccessPolicy::default();
    assert_eq!(
        serde_json::to_value(&default).unwrap(),
        policy_with(json!({"kind": "tbd_member"}))
    );
    assert_eq!(default.validate(), Ok(()));
    assert_eq!(validated(json!({"grants": []})).unwrap().grants, vec![]);
    assert!(serde_json::from_value::<EventAccessPolicy>(json!({})).is_err());
    assert!(serde_json::from_value::<EventAccessPolicy>(Value::Null).is_err());
}

#[test]
fn all_conditions_round_trip_with_exact_snake_case_contracts() {
    for condition in [
        json!({"kind": "authenticated"}),
        json!({"kind": "tbd_member"}),
        json!({"kind": "discord_role", "guild_id": "guild", "role_id": "role"}),
        json!({"kind": "event_group", "group_id": Uuid::from_u128(1)}),
        json!({"kind": "named_account", "discord_id": "account"}),
    ] {
        let wire = policy_with(condition);
        let parsed = validated(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
        assert_eq!(
            serde_json::from_str::<EventAccessPolicy>(&serde_json::to_string(&parsed).unwrap())
                .unwrap(),
            parsed
        );
    }
}

#[test]
fn unknown_fields_kinds_and_wrong_shapes_are_rejected_by_deserialization() {
    for wire in [
        json!({"grants": [], "allow": true}),
        json!({"grants": [{"conditions": [{"kind": "authenticated"}], "allow": true}]}),
        json!({"grants": null}),
        json!({"grants": false}),
        json!({"grants": 0}),
        json!({"grants": {"conditions": []}}),
        json!({"grants": [null]}),
        json!({"grants": [{}]}),
        json!({"grants": [{"conditions": null}]}),
        json!({"grants": [{"conditions": false}]}),
        json!({"grants": [{"conditions": {"kind": "authenticated"}}]}),
        policy_with(Value::Null),
        policy_with(json!({})),
        policy_with(json!("authenticated")),
        policy_with(json!({"kind": "unknown"})),
        policy_with(json!({"kind": "TbdMember"})),
        policy_with(json!({"kind": null})),
        policy_with(json!({"kind": false})),
        policy_with(json!({"kind": "authenticated", "allow": true})),
        policy_with(json!({"kind": "tbd_member", "guild_id": "other"})),
        policy_with(json!({"kind": "discord_role", "guild_id": "guild"})),
        policy_with(json!({"kind": "discord_role", "guild_id": null, "role_id": "role"})),
        policy_with(json!({"kind": "discord_role", "guild_id": "guild", "role_id": false})),
        policy_with(
            json!({"kind": "discord_role", "guild_id": "guild", "role_id": "role", "extra": 0}),
        ),
        policy_with(json!({"kind": "event_group", "group_id": "not-a-uuid"})),
        policy_with(json!({"kind": "event_group", "group_id": null})),
        policy_with(json!({"kind": "event_group"})),
        policy_with(json!({"kind": "named_account", "discord_id": 0})),
        policy_with(json!({"kind": "named_account", "discord_id": null})),
        policy_with(json!({"kind": "named_account"})),
    ] {
        assert!(
            serde_json::from_value::<EventAccessPolicy>(wire.clone()).is_err(),
            "unexpectedly accepted malformed wire policy: {wire}"
        );
    }
}

#[test]
fn empty_grants_and_invalid_identity_conditions_fail_semantic_validation() {
    assert!(validated(json!({"grants": [{"conditions": []}]})).is_err());
    assert!(
        validated(policy_with(
            json!({"kind": "event_group", "group_id": Uuid::nil()})
        ))
        .is_err()
    );
    for identity in [
        "".to_owned(),
        " ".to_owned(),
        " account".to_owned(),
        "account ".to_owned(),
        "\u{00a0}account".to_owned(),
        "account\n".to_owned(),
        "ac\0count".to_owned(),
        "ac\tcount".to_owned(),
        "ac\u{007f}count".to_owned(),
        "x".repeat(129),
        "é".repeat(65),
    ] {
        for condition in [
            json!({"kind": "named_account", "discord_id": identity}),
            json!({"kind": "discord_role", "guild_id": identity, "role_id": "role"}),
            json!({"kind": "discord_role", "guild_id": "guild", "role_id": identity}),
        ] {
            assert!(
                validated(policy_with(condition.clone())).is_err(),
                "{condition}"
            );
        }
    }
    for identity in ["x".to_owned(), "x".repeat(128), "é".repeat(64)] {
        assert!(
            validated(policy_with(
                json!({"kind": "named_account", "discord_id": identity})
            ))
            .is_ok()
        );
        assert!(
            validated(policy_with(
                json!({"kind": "discord_role", "guild_id": identity, "role_id": identity})
            ))
            .is_ok()
        );
    }
}

#[test]
fn largest_policy_and_each_adjacent_limit_have_explicit_acceptance_cases() {
    for (grants, conditions, accepted) in [
        (0, 0, true),
        (1, 0, false),
        (1, 1, true),
        (32, 16, true),
        (33, 16, false),
        (32, 17, false),
    ] {
        let wire = json!({
            "grants": vec![json!({
                "conditions": vec![json!({"kind": "authenticated"}); conditions]
            }); grants]
        });
        assert_eq!(
            validated(wire).is_ok(),
            accepted,
            "{grants} by {conditions}"
        );
    }
}

#[test]
fn grant_and_condition_bounds_are_exact() {
    crate::property_evidence::run_property(
        "grant_and_condition_bounds_are_exact",
        256,
        &(0usize..=35, 0usize..=19),
        |(grants, conditions)| {
            let policy = EventAccessPolicy {
                grants: vec![
                    EventAccessGrant {
                        conditions: vec![EventAccessCondition::Authenticated {}; conditions],
                    };
                    grants
                ],
            };
            let expected = grants <= 32 && (grants == 0 || (1..=16).contains(&conditions));
            prop_assert_eq!(policy.validate().is_ok(), expected);
            prop_assert_eq!(
                validated(serde_json::to_value(&policy).unwrap()).is_ok(),
                expected
            );
            Ok(())
        },
    );
}

#[test]
fn identity_bounds_count_utf8_bytes_and_reject_padding() {
    crate::property_evidence::run_property(
        "identity_bounds_count_utf8_bytes_and_reject_padding",
        256,
        &("[a-zA-Z0-9_]{1,128}", 1usize..=70),
        |(ascii, unicode_count)| {
            let padded = format!(" {ascii}");
            let newline = format!("{ascii}\n");
            prop_assert!(valid_identity(&ascii));
            prop_assert!(!valid_identity(&padded));
            prop_assert!(!valid_identity(&newline));
            prop_assert_eq!(
                valid_identity(&"é".repeat(unicode_count)),
                unicode_count <= 64
            );
            Ok(())
        },
    );
}
