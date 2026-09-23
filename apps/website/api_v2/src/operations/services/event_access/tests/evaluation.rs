//! Policy algebra and mandatory constraints hold for generated subjects and grant combinations.

use super::*;
use crate::operations::models::event_access_policy::EventAccessGrant;
use proptest::prelude::*;

fn allowed_constraints() -> MandatoryAccessConstraints {
    MandatoryAccessConstraints::SATISFIED
}

fn quota_gate(open: bool) -> QuotaOpeningGate {
    if open {
        QuotaOpeningGate::Open
    } else {
        QuotaOpeningGate::NotYetOpen {
            quota_kind: ReservationQuotaKind::Guest,
            opens_at: DateTime::from_timestamp(1_900_000_000, 0).unwrap(),
        }
    }
}

fn condition(index: u8) -> EventAccessCondition {
    match index {
        0 => EventAccessCondition::Authenticated {},
        1 => EventAccessCondition::TbdMember {},
        2 => EventAccessCondition::NamedAccount {
            discord_id: "player".into(),
        },
        3 => EventAccessCondition::NamedAccount {
            discord_id: "other".into(),
        },
        4 => EventAccessCondition::DiscordRole {
            guild_id: "tbd".into(),
            role_id: "role".into(),
        },
        5 => EventAccessCondition::DiscordRole {
            guild_id: "partner".into(),
            role_id: "role".into(),
        },
        6 => EventAccessCondition::DiscordRole {
            guild_id: "tbd".into(),
            role_id: "other".into(),
        },
        7 => EventAccessCondition::EventGroup {
            group_id: Uuid::from_u128(1),
        },
        8 => EventAccessCondition::EventGroup {
            group_id: Uuid::from_u128(2),
        },
        _ => unreachable!("strategy restricts condition IDs"),
    }
}

fn policy(grants: &[Vec<u8>]) -> EventAccessPolicy {
    EventAccessPolicy {
        grants: grants
            .iter()
            .map(|grant| EventAccessGrant {
                conditions: grant.iter().map(|index| condition(*index)).collect(),
            })
            .collect(),
    }
}

fn subject(flags: [bool; 6]) -> EventAccessSubject {
    let mut value = EventAccessSubject {
        discord_id: "player".into(),
        tbd_member: flags[0],
        ..Default::default()
    };
    for (present, guild, role) in [
        (flags[1], "tbd", "role"),
        (flags[2], "partner", "role"),
        (flags[3], "tbd", "other"),
    ] {
        if present {
            value
                .guild_roles
                .entry(guild.into())
                .or_default()
                .insert(role.into());
        }
    }
    for (present, id) in [(flags[4], 1), (flags[5], 2)] {
        if present {
            value.event_groups.insert(Uuid::from_u128(id));
        }
    }
    value
}

fn condition_truth(flags: [bool; 6]) -> [bool; 9] {
    [
        true, flags[0], true, false, flags[1], flags[2], flags[3], flags[4], flags[5],
    ]
}

fn grants_strategy() -> impl Strategy<Value = Vec<Vec<u8>>> {
    prop::collection::vec(prop::collection::vec(0u8..9, 1..=8), 0..=12)
}

fn expected_grant(grants: &[Vec<u8>], flags: [bool; 6]) -> bool {
    let truth = condition_truth(flags);
    // Count satisfied conjunctions using an independent truth-table representation.
    grants
        .iter()
        .filter(|grant| {
            grant
                .iter()
                .filter(|index| !truth[**index as usize])
                .count()
                == 0
        })
        .count()
        > 0
}

#[test]
fn explicit_closed_policies_replace_ancestors_and_absent_policies_inherit() {
    let open = policy(&[vec![0]]);
    let closed = policy(&[]);
    let player = subject([false; 6]);
    for (event, squad, slot, source, denial) in [
        (&open, None, None, PolicySource::Event, None),
        (
            &open,
            Some(&closed),
            None,
            PolicySource::Squad,
            Some(AccessDenial::Policy),
        ),
        (
            &open,
            Some(&open),
            Some(&closed),
            PolicySource::Slot,
            Some(AccessDenial::Policy),
        ),
        (&closed, Some(&open), None, PolicySource::Squad, None),
        (
            &closed,
            Some(&closed),
            Some(&open),
            PolicySource::Slot,
            None,
        ),
        (&closed, None, Some(&open), PolicySource::Slot, None),
    ] {
        assert_eq!(
            evaluate_access(event, squad, slot, &player, allowed_constraints()).unwrap(),
            EventAccessDecision {
                policy_source: source,
                denial
            }
        );
    }
}

#[test]
fn default_members_only_does_not_confuse_authenticated_guests_with_members() {
    let default = EventAccessPolicy::default();
    let guest = subject([false; 6]);
    let member = subject([true, false, false, false, false, false]);
    assert_eq!(
        evaluate_access(&default, None, None, &guest, allowed_constraints())
            .unwrap()
            .denial,
        Some(AccessDenial::Policy)
    );
    assert_eq!(
        evaluate_access(&default, None, None, &member, allowed_constraints())
            .unwrap()
            .denial,
        None
    );
    let anonymous = EventAccessSubject::default();
    assert_eq!(
        evaluate_access(
            &policy(&[vec![0]]),
            None,
            None,
            &anonymous,
            allowed_constraints()
        )
        .unwrap()
        .denial,
        Some(AccessDenial::InvalidSession)
    );
}

#[test]
fn invalid_effective_policy_fails_closed_and_cannot_fall_back_to_open_ancestor() {
    let open = policy(&[vec![0]]);
    let player = subject([true; 6]);
    let invalid = [
        policy(&[vec![]]),
        policy(&vec![vec![0]; 33]),
        policy(&[vec![0; 17]]),
        EventAccessPolicy {
            grants: vec![EventAccessGrant {
                conditions: vec![EventAccessCondition::NamedAccount {
                    discord_id: "".into(),
                }],
            }],
        },
        EventAccessPolicy {
            grants: vec![EventAccessGrant {
                conditions: vec![EventAccessCondition::EventGroup {
                    group_id: Uuid::nil(),
                }],
            }],
        },
    ];
    for invalid in invalid {
        for constraints in [
            allowed_constraints(),
            MandatoryAccessConstraints {
                session_valid: false,
                account_available: false,
                registration_open: false,
                quota_opening: quota_gate(false),
                deployment_eligible: false,
                capacity_available: false,
            },
        ] {
            assert!(evaluate_access(&invalid, None, None, &player, constraints).is_err());
            assert!(evaluate_access(&open, Some(&invalid), None, &player, constraints).is_err());
            assert!(
                evaluate_access(&open, Some(&open), Some(&invalid), &player, constraints).is_err()
            );
        }
        assert_eq!(
            evaluate_access(&invalid, Some(&open), None, &player, allowed_constraints())
                .unwrap()
                .denial,
            None
        );
    }
}

#[test]
fn alternative_grants_are_independent_and_conditions_cannot_mix_between_grants() {
    let policy = policy(&[vec![4, 7], vec![5, 8]]);
    for (flags, permitted) in [
        ([false, true, false, false, false, true], false),
        ([false, false, true, false, true, false], false),
        ([false, true, false, false, true, false], true),
        ([false, false, true, false, false, true], true),
        ([false; 6], false),
    ] {
        assert_eq!(
            evaluate_access(&policy, None, None, &subject(flags), allowed_constraints())
                .unwrap()
                .denial
                .is_none(),
            permitted
        );
    }
}

#[test]
fn event_access_is_exactly_or_of_and_grants() {
    crate::property_evidence::run_property(
        "event_access_is_exactly_or_of_and_grants",
        512,
        &(grants_strategy(), any::<[bool; 6]>()),
        |(grants, flags)| {
            let decision = evaluate_access(
                &policy(&grants),
                None,
                None,
                &subject(flags),
                allowed_constraints(),
            )
            .unwrap();
            prop_assert_eq!(decision.denial.is_none(), expected_grant(&grants, flags));
            prop_assert_eq!(decision.policy_source, PolicySource::Event);
            Ok(())
        },
    );
}

#[test]
fn slot_then_squad_then_event_selects_exactly_one_policy() {
    crate::property_evidence::run_property(
        "slot_then_squad_then_event_selects_exactly_one_policy",
        512,
        &(
            grants_strategy(),
            prop::option::of(grants_strategy()),
            prop::option::of(grants_strategy()),
            any::<[bool; 6]>(),
        ),
        |(event, squad, slot, flags)| {
            let event_policy = policy(&event);
            let squad_policy = squad.as_ref().map(|grants| policy(grants));
            let slot_policy = slot.as_ref().map(|grants| policy(grants));
            let (expected, expected_policy, expected_source) = if let Some(slot) = &slot {
                (slot, slot_policy.as_ref().unwrap(), PolicySource::Slot)
            } else if let Some(squad) = &squad {
                (squad, squad_policy.as_ref().unwrap(), PolicySource::Squad)
            } else {
                (&event, &event_policy, PolicySource::Event)
            };
            let (selected, source) =
                effective_policy(&event_policy, squad_policy.as_ref(), slot_policy.as_ref());
            prop_assert!(std::ptr::eq(selected, expected_policy));
            prop_assert_eq!(source, expected_source);
            let decision = evaluate_access(
                &event_policy,
                squad_policy.as_ref(),
                slot_policy.as_ref(),
                &subject(flags),
                allowed_constraints(),
            )
            .unwrap();
            prop_assert_eq!(decision.policy_source, expected_source);
            prop_assert_eq!(decision.denial.is_none(), expected_grant(expected, flags));
            Ok(())
        },
    );
}

#[test]
fn grant_and_condition_permutations_preserve_authority() {
    crate::property_evidence::run_property(
        "grant_and_condition_permutations_preserve_authority",
        512,
        &(
            grants_strategy(),
            any::<[bool; 6]>(),
            prop::collection::vec((any::<usize>(), any::<usize>()), 0..=40),
        ),
        |(grants, flags, swaps)| {
            let original = evaluate_access(
                &policy(&grants),
                None,
                None,
                &subject(flags),
                allowed_constraints(),
            )
            .unwrap();
            let mut permuted = grants;
            permuted.reverse();
            for (left, right) in swaps {
                let count = permuted.len();
                if count > 0 {
                    permuted.swap(left % count, right % count);
                }
                for grant in &mut permuted {
                    let count = grant.len();
                    grant.swap(left % count, right % count);
                }
            }
            let rearranged = evaluate_access(
                &policy(&permuted),
                None,
                None,
                &subject(flags),
                allowed_constraints(),
            )
            .unwrap();
            prop_assert_eq!(original, rearranged);
            Ok(())
        },
    );
}

#[test]
fn every_mandatory_gate_blocks_every_satisfied_grant() {
    crate::property_evidence::run_property(
        "every_mandatory_gate_blocks_every_satisfied_grant",
        512,
        &(
            any::<[bool; 6]>(),
            prop::sample::select(vec![0u8, 1, 2, 4, 5, 7]),
            0u8..3,
        ),
        |(flags, kind, source)| {
            let constraints = MandatoryAccessConstraints {
                session_valid: flags[0],
                account_available: flags[1],
                registration_open: flags[2],
                quota_opening: quota_gate(flags[3]),
                deployment_eligible: flags[4],
                capacity_available: flags[5],
            };
            let QuotaOpeningGate::NotYetOpen {
                quota_kind,
                opens_at,
            } = quota_gate(false)
            else {
                unreachable!("a closed gate carries its opening");
            };
            let expected = flags.iter().position(|allowed| !allowed).map(|index| {
                [
                    AccessDenial::InvalidSession,
                    AccessDenial::AccountUnavailable,
                    AccessDenial::RegistrationClosed,
                    AccessDenial::QuotaNotOpen {
                        quota_kind,
                        opens_at,
                    },
                    AccessDenial::DeploymentRequirements,
                    AccessDenial::Capacity,
                ][index]
            });
            let grant = policy(&[vec![kind]]);
            let closed = policy(&[]);
            let decision = match source {
                0 => evaluate_access(&grant, None, None, &subject([true; 6]), constraints),
                1 => evaluate_access(
                    &closed,
                    Some(&grant),
                    None,
                    &subject([true; 6]),
                    constraints,
                ),
                _ => evaluate_access(
                    &closed,
                    Some(&closed),
                    Some(&grant),
                    &subject([true; 6]),
                    constraints,
                ),
            }
            .unwrap();
            prop_assert_eq!(decision.denial, expected);
            Ok(())
        },
    );
}

#[test]
fn discord_roles_are_guild_scoped_and_independent_grants_survive_missing_guilds() {
    crate::property_evidence::run_property(
        "discord_roles_are_guild_scoped_and_independent_grants_survive_missing_guilds",
        512,
        &(
            "[a-z0-9]{1,20}",
            "[a-z0-9]{1,20}",
            any::<bool>(),
            any::<bool>(),
        ),
        |(suffix, role, first, second)| {
            let first_guild = format!("first-{suffix}");
            let second_guild = format!("second-{suffix}");
            let first_condition = EventAccessCondition::DiscordRole {
                guild_id: first_guild.clone(),
                role_id: role.clone(),
            };
            let second_condition = EventAccessCondition::DiscordRole {
                guild_id: second_guild.clone(),
                role_id: role.clone(),
            };
            let mut player = subject([false; 6]);
            if first {
                player
                    .guild_roles
                    .insert(first_guild, BTreeSet::from([role.clone()]));
            }
            if second {
                player
                    .guild_roles
                    .insert(second_guild, BTreeSet::from([role]));
            }
            let first_only = EventAccessPolicy {
                grants: vec![EventAccessGrant {
                    conditions: vec![first_condition.clone()],
                }],
            };
            prop_assert_eq!(
                evaluate_access(&first_only, None, None, &player, allowed_constraints())
                    .unwrap()
                    .denial
                    .is_none(),
                first
            );
            let either = EventAccessPolicy {
                grants: vec![
                    EventAccessGrant {
                        conditions: vec![first_condition.clone()],
                    },
                    EventAccessGrant {
                        conditions: vec![second_condition.clone()],
                    },
                ],
            };
            prop_assert_eq!(
                evaluate_access(&either, None, None, &player, allowed_constraints())
                    .unwrap()
                    .denial
                    .is_none(),
                first || second
            );
            let both = EventAccessPolicy {
                grants: vec![EventAccessGrant {
                    conditions: vec![first_condition, second_condition],
                }],
            };
            prop_assert_eq!(
                evaluate_access(&both, None, None, &player, allowed_constraints())
                    .unwrap()
                    .denial
                    .is_none(),
                first && second
            );
            Ok(())
        },
    );
}

#[test]
fn pending_membership_evidence_only_relabels_policy_denials() {
    crate::property_evidence::run_property(
        "pending_membership_evidence_only_relabels_policy_denials",
        512,
        &(
            grants_strategy(),
            any::<[bool; 6]>(),
            any::<[bool; 6]>(),
            any::<bool>(),
        ),
        |(grants, flags, gates, pending)| {
            let constraints = MandatoryAccessConstraints {
                session_valid: gates[0],
                account_available: gates[1],
                registration_open: gates[2],
                quota_opening: quota_gate(gates[3]),
                deployment_eligible: gates[4],
                capacity_available: gates[5],
            };
            let policy = policy(&grants);
            let strict =
                evaluate_access(&policy, None, None, &subject(flags), constraints).unwrap();
            let relabelled = evaluate_access_with_pending_evidence(
                &policy,
                None,
                None,
                &subject(flags),
                pending,
                constraints,
            )
            .unwrap();
            prop_assert_eq!(relabelled.policy_source, strict.policy_source);
            // Pending evidence never converts a denial into access, nor access into a denial.
            prop_assert_eq!(relabelled.denial.is_none(), strict.denial.is_none());
            let expected = if pending && strict.denial == Some(AccessDenial::Policy) {
                Some(AccessDenial::MembershipVerificationRequired)
            } else {
                strict.denial
            };
            prop_assert_eq!(relabelled.denial, expected);
            Ok(())
        },
    );
}
