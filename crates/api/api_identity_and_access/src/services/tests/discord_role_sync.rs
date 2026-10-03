use super::*;

#[test]
fn verified_member_with_no_mapped_roles_is_enlisted() {
    assert_eq!(
        role_for_membership("member", UserRole::Enlisted),
        UserRole::Enlisted
    );
}

#[test]
fn confirmed_nonmember_and_unknown_never_inherit_a_stored_admin_override() {
    for status in ["nonmember", "unknown", ""] {
        assert_eq!(
            role_for_membership(status, UserRole::Admin),
            UserRole::Guest
        );
    }
}

#[test]
fn verified_membership_retains_its_mapped_role() {
    assert_eq!(
        role_for_membership("member", UserRole::Admin),
        UserRole::Admin
    );
}
