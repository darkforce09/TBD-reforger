//! The access panel's forms: policy drafts, group forms and pool forms, read back into what the
//! backend takes.

use super::groups::group_form::{parse_role_ids, source_line, GroupForm, GroupKind};
use super::policy_draft::*;
use super::quota_editor::{usage_line, QuotaFields};
use crate::v2::core::api::dto::{
    EventAccessAdministration, EventAccessCondition, EventGroupSource, QuotaUsageView,
};
use crate::v2::core::test_support::fixtures::golden;

const ACCESS: &str = golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access.json");

fn access() -> EventAccessAdministration {
    serde_json::from_str(ACCESS).unwrap()
}

/* ───────────────────────── policy drafts ───────────────────────── */

/// Every captured policy survives a trip through the editor unchanged.
#[test]
fn every_captured_policy_round_trips_through_the_draft() {
    let access = access();
    let policies = std::iter::once(&access.event_policy)
        .chain(access.squad_policies.iter().map(|p| &p.policy))
        .chain(access.slot_policies.iter().map(|p| &p.policy));
    for policy in policies {
        let mut ids = 0;
        let draft = draft_from_policy(policy, &mut ids);
        assert_eq!(&policy_from_draft(&draft).unwrap(), policy);
    }
}

/// Every row of a draft has its own id, so a keyed list never confuses two of them.
#[test]
fn draft_rows_carry_unique_ids() {
    let access = access();
    let mut ids = 0;
    let mut draft = draft_from_policy(&access.event_policy, &mut ids);
    add_grant(&mut draft, &mut ids);
    let first = draft[0].id;
    add_condition(&mut draft, first, &mut ids);
    let mut seen: Vec<u64> = draft
        .iter()
        .flat_map(|g| std::iter::once(g.id).chain(g.conditions.iter().map(|c| c.id)))
        .collect();
    let count = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), count);
}

/// A kind change keeps what was typed for every kind, and only the chosen kind's fields are read.
#[test]
fn a_kind_change_keeps_typed_text_and_reads_only_the_chosen_fields() {
    let mut ids = 0;
    let mut draft = Vec::new();
    add_grant(&mut draft, &mut ids);
    let condition = draft[0].conditions[0].id;
    set_kind(&mut draft, condition, ConditionKind::NamedAccount);
    set_field(
        &mut draft,
        condition,
        ConditionIdentifier::Account,
        " 000000000000000006 ".into(),
    );
    set_kind(&mut draft, condition, ConditionKind::DiscordRole);
    set_field(
        &mut draft,
        condition,
        ConditionIdentifier::Guild,
        "100000000000000777".into(),
    );
    set_field(
        &mut draft,
        condition,
        ConditionIdentifier::Role,
        "200000000000000888".into(),
    );
    assert_eq!(
        policy_from_draft(&draft).unwrap().grants[0].conditions,
        vec![EventAccessCondition::DiscordRole {
            guild_id: "100000000000000777".into(),
            role_id: "200000000000000888".into(),
        }]
    );
    set_kind(&mut draft, condition, ConditionKind::NamedAccount);
    assert_eq!(
        policy_from_draft(&draft).unwrap().grants[0].conditions,
        vec![EventAccessCondition::NamedAccount {
            discord_id: "000000000000000006".into()
        }],
        "the account typed earlier is still there, trimmed"
    );
}

/// Removing a grant's last condition removes the grant, because a grant needs one.
#[test]
fn removing_the_last_condition_removes_its_grant() {
    let mut ids = 0;
    let mut draft = Vec::new();
    add_grant(&mut draft, &mut ids);
    add_grant(&mut draft, &mut ids);
    let (grant, condition) = (draft[0].id, draft[0].conditions[0].id);
    remove_condition(&mut draft, grant, condition);
    assert_eq!(draft.len(), 1);
    let remaining = draft[0].id;
    remove_grant(&mut draft, remaining);
    assert!(draft.is_empty());
}

/// An empty draft is a valid policy that admits nobody — not an error, and not inheritance.
#[test]
fn an_empty_draft_is_a_policy_that_admits_nobody() {
    let policy = policy_from_draft(&[]).unwrap();
    assert!(policy.grants.is_empty());
    assert_eq!(policy_summary(&policy, &[]), "Admits nobody");
}

/// The backend's bounds are enforced before sending: missing identifiers, control characters and
/// too many grants or conditions are refused with the grant they are in.
#[test]
fn the_backend_bounds_are_enforced_before_sending() {
    let mut ids = 0;
    let mut draft = Vec::new();
    add_grant(&mut draft, &mut ids);
    let condition = draft[0].conditions[0].id;
    set_kind(&mut draft, condition, ConditionKind::EventGroup);
    assert_eq!(
        policy_from_draft(&draft).unwrap_err(),
        "Grant 1: the group is required"
    );
    set_kind(&mut draft, condition, ConditionKind::NamedAccount);
    set_field(
        &mut draft,
        condition,
        ConditionIdentifier::Account,
        "a\u{7}b".into(),
    );
    assert!(policy_from_draft(&draft)
        .unwrap_err()
        .contains("control characters"));
    set_field(
        &mut draft,
        condition,
        ConditionIdentifier::Account,
        "x".repeat(129),
    );
    assert!(policy_from_draft(&draft).is_err());

    let mut many = Vec::new();
    for _ in 0..=MAX_GRANTS {
        add_grant(&mut many, &mut ids);
    }
    assert!(policy_from_draft(&many).is_err());
    let mut crowded = Vec::new();
    add_grant(&mut crowded, &mut ids);
    let grant = crowded[0].id;
    for _ in 0..MAX_CONDITIONS {
        add_condition(&mut crowded, grant, &mut ids);
    }
    assert_eq!(
        policy_from_draft(&crowded).unwrap_err(),
        format!("Grant 1 needs 1 to {MAX_CONDITIONS} conditions")
    );
}

/// The captured operation policy reads as its two alternatives, with the group named.
#[test]
fn a_policy_summary_names_alternatives_and_groups() {
    let access = access();
    assert_eq!(
        policy_summary(&access.event_policy, &access.groups),
        "Admits: Verified TBD member; or Member of Byte Parity roster"
    );
    for kind in ConditionKind::ALL {
        assert_eq!(ConditionKind::from_wire(kind.wire()), Some(kind));
    }
}

/* ───────────────────────── group forms ───────────────────────── */

#[test]
fn role_ids_split_on_commas_and_whitespace_without_duplicates() {
    assert_eq!(
        parse_role_ids(" 1, 2\n3  1 ,,"),
        vec!["1".to_string(), "2".to_string(), "3".to_string()]
    );
    assert!(parse_role_ids("  ").is_empty());
}

/// A captured group's form reads back into exactly its source, and an untouched edit sends nothing.
#[test]
fn an_untouched_group_edit_sends_nothing() {
    let access = access();
    for group in &access.groups {
        let form = GroupForm::of(&group.name, &group.source);
        assert_eq!(form.validated_source().unwrap(), group.source);
        assert_eq!(
            form.changes_from(&group.name, &group.source).unwrap(),
            (None, None)
        );
    }
}

/// A rename sends only the name; a source change sends only the source.
#[test]
fn a_group_edit_sends_only_what_changed() {
    let access = access();
    let partner = &access.groups[1];
    let mut form = GroupForm::of(&partner.name, &partner.source);
    form.name = "  Allied scouts ".into();
    assert_eq!(
        form.changes_from(&partner.name, &partner.source).unwrap(),
        (Some("Allied scouts".to_string()), None)
    );
    let mut form = GroupForm::of(&partner.name, &partner.source);
    form.role_ids = "200000000000000888 200000000000000889".into();
    let (name, source) = form.changes_from(&partner.name, &partner.source).unwrap();
    assert_eq!(name, None);
    assert_eq!(
        source,
        Some(EventGroupSource::PartnerGuild {
            guild_id: "100000000000000777".into(),
            required_role_ids: vec!["200000000000000888".into(), "200000000000000889".into()],
        })
    );
}

/// A partner group needs its guild, and a name needs text.
#[test]
fn a_group_form_refuses_what_the_backend_would() {
    let mut form = GroupForm::blank();
    assert!(form.validated_name().is_err());
    form.name = "Allies".into();
    form.kind = GroupKind::PartnerGuild;
    assert!(form.validated_source().is_err());
    form.guild_id = "7".into();
    form.role_ids = (0..33).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    assert!(form.validated_source().is_err(), "at most 32 roles");
    assert_eq!(
        GroupKind::from_wire(GroupKind::PartnerGuild.wire()),
        Some(GroupKind::PartnerGuild)
    );
}

#[test]
fn a_source_reads_as_where_members_come_from() {
    let access = access();
    assert!(source_line(&access.groups[0].source).starts_with("Managed roster"));
    let partner = source_line(&access.groups[1].source);
    assert!(partner.contains("100000000000000777") && partner.contains("200000000000000888"));
}

/* ───────────────────────── pool forms ───────────────────────── */

/// The captured pools survive the form unchanged, sub-second opening times included.
#[test]
fn untouched_pools_are_sent_back_exactly() {
    let access = access();
    let form = QuotaFields::of(&access.reservation_quotas);
    assert!(form.member.uncapped && !form.guest.uncapped);
    assert_eq!(form.guest.seats, "2");
    assert_eq!(form.member.opens_at, "2026-07-15T14:05");
    assert_eq!(form.validated().unwrap(), access.reservation_quotas);
}

/// An edited opening is sent as the UTC instant the field shows; a limit must be a whole number,
/// and zero — closing a pool — is kept apart from uncapped.
#[test]
fn edited_pools_are_read_as_utc_and_whole_numbers() {
    let access = access();
    let mut form = QuotaFields::of(&access.reservation_quotas);
    form.open.opens_at = "2026-08-01T12:30".into();
    form.open.seats = "0".into();
    form.member.uncapped = false;
    form.member.seats = "40".into();
    let pools = form.validated().unwrap();
    assert_eq!(pools.open.opens_at, "2026-08-01T12:30:00Z");
    assert_eq!(pools.open.seats, Some(0));
    assert_eq!(pools.member.seats, Some(40));
    form.guest.seats = "-1".into();
    assert!(form.validated().is_err());
    form.guest.seats = "2".into();
    form.guest.opens_at = String::new();
    assert!(form.validated().is_err());
}

#[test]
fn pool_usage_reads_against_the_operation_limit() {
    let usage = QuotaUsageView {
        member: 4,
        guest: 1,
        open: 0,
        legacy_unclassified: 2,
        total: 7,
    };
    assert_eq!(
        usage_line(&usage, 0),
        "7 places held; no operation-wide limit — member 4 · guest 1 · open 0 · 2 from before the pools"
    );
    assert!(usage_line(&usage, 40).starts_with("7 of 40 operation places held"));
}
