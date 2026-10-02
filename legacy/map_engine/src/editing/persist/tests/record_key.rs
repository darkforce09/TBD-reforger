//! Role: the record key's shape, its injectivity, and what an unowned key parses to.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: explicit strings built in the test body.
//! Invariants: every owner/logical pair round-trips, and no two distinct pairs share a key — the
//! property that keeps one account's drafts out of another's reach.

use super::*;

#[test]
fn a_signed_out_author_gets_the_anonymous_namespace() {
    assert_eq!(owner_token_or_anonymous(None), ANONYMOUS_OWNER);
    assert_eq!(owner_token_or_anonymous(Some("1234")), "1234");
}

#[test]
fn the_key_carries_the_owner_length_in_front_of_the_owner() {
    assert_eq!(scoped_key("1234", "mission-9"), "u4:1234|mission-9");
    assert_eq!(owner_prefix("1234"), "u4:1234|");
    assert!(scoped_key("1234", "mission-9").starts_with(&owner_prefix("1234")));
}

#[test]
fn every_owner_and_logical_pair_round_trips() {
    for (owner, logical) in [
        ("1234", "mission-9"),
        ("1234", "mission-9::pre-adopt"),
        ("1234", "mission-9::pre-restore"),
        // Separators inside either half are what a naive join cannot survive.
        ("a|b", "mission-9"),
        ("a:b", "u4:xxxx|decoy"),
        ("1234", "|leading-pipe"),
        ("", "mission-9"),
        // A multi-byte owner is sliced by byte count, so the boundary has to be respected.
        ("ölken", "mission-9"),
    ] {
        let key = scoped_key(owner, logical);
        assert_eq!(
            split_scoped_key(&key),
            Some((owner, logical)),
            "round trip failed for owner={owner:?} logical={logical:?} key={key}"
        );
    }
}

/// The reason the length prefix exists: a plain `{owner}|{logical}` join maps these two distinct
/// pairs onto one key, which is one account reading another's draft.
#[test]
fn a_separator_in_the_owner_cannot_collide_two_accounts() {
    let one = scoped_key("a|b", "mission-9");
    let two = scoped_key("a", "b|mission-9");
    assert_ne!(one, two);
    assert_eq!(split_scoped_key(&one), Some(("a|b", "mission-9")));
    assert_eq!(split_scoped_key(&two), Some(("a", "b|mission-9")));
}

#[test]
fn a_key_that_carries_no_owner_parses_to_nothing() {
    for unowned in [
        "mission-9",               // a bare logical key
        "u:1234|mission-9",        // no length
        "u+3:abc|mission-9",       // not a decimal length
        "u0x2:ab|mission-9",       // not a decimal length
        "u9:abc|mission-9",        // a length past the end of the owner segment
        "u4:1234mission-9",        // no separator after the owner
        "u1:\u{f6}lken|mission-9", // a length that would split a multi-byte owner
    ] {
        assert_eq!(
            split_scoped_key(unowned),
            None,
            "{unowned} must read as unattributable, not as somebody's record"
        );
    }
}

/// A zero-length owner is still an owner: it is a key this module writes, so it must parse.
#[test]
fn a_zero_length_owner_is_a_parsed_owner_and_not_an_orphan() {
    assert_eq!(split_scoped_key("u0:|mission-9"), Some(("", "mission-9")));
}

/// A snapshot is a logical key like any other: the account scoping is applied on top of it, and it
/// round-trips through the same parse.
#[test]
fn a_snapshot_key_is_a_logical_key_the_owner_scoping_wraps() {
    let id = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";
    let logical = snapshot_key(id, "::pre-adopt");
    assert_eq!(logical, "3f2504e0-4f89-11d3-9a0c-0305e82c3301::pre-adopt");
    let physical = scoped_key("1234", &logical);
    assert_eq!(
        split_scoped_key(&physical),
        Some(("1234", logical.as_str()))
    );
    assert!(physical.starts_with(&owner_prefix("1234")));
}

/// Distinct suffixes give distinct records, and neither is the live draft's own key — which is what
/// keeps the debounced draft write, which writes the bare id, off both of them.
#[test]
fn distinct_suffixes_never_collide_with_each_other_or_with_the_live_draft() {
    let id = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";
    let pre_adopt = snapshot_key(id, "::pre-adopt");
    let pre_restore = snapshot_key(id, "::pre-restore");
    assert_ne!(pre_adopt, pre_restore);
    assert_ne!(pre_adopt, id);
    assert_ne!(pre_restore, id);
    assert_eq!(
        snapshot_key(id, ""),
        id,
        "no suffix is the live draft itself"
    );
}
