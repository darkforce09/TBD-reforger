//! Wire round trips and fail-closed decoding keep malformed policies from becoming access grants.

use super::*;
use serde_json::{Value, json};
use uuid::Uuid;

fn policy_with(condition: Value) -> Value {
    json!({"grants": [{"conditions": [condition]}]})
}

fn validated(value: Value) -> Result<EventAccessPolicy, String> {
    let policy: EventAccessPolicy = serde_json::from_value(value).map_err(|e| e.to_string())?;
    policy.validate().map_err(str::to_owned)?;
    Ok(policy)
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
