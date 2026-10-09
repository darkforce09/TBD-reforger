//! The operation's game server: the values the create and edit forms send.

use super::*;

/// An edit sends `server_id` only when the choice differs from the operation's server: the id to
/// set one, `null` to clear the one it had, and nothing when they agree.
#[test]
fn an_edit_sends_the_server_only_when_it_changes() {
    let primary = "00000000-0000-4000-d000-000000000001";
    let secondary = "00000000-0000-4000-d000-000000000002";
    assert_eq!(server_id_change(None, ""), None);
    assert_eq!(server_id_change(Some(primary), primary), None);
    assert_eq!(
        server_id_change(None, primary),
        Some(Value::String(primary.to_string()))
    );
    assert_eq!(
        server_id_change(Some(primary), secondary),
        Some(Value::String(secondary.to_string()))
    );
    assert_eq!(server_id_change(Some(primary), ""), Some(Value::Null));
    assert_eq!(server_id_change(Some(primary), "  "), Some(Value::Null));
}

/// A new operation names a server only when one is chosen.
#[test]
fn a_new_operation_names_a_server_only_when_one_is_chosen() {
    assert_eq!(chosen_server_id(""), None);
    assert_eq!(
        chosen_server_id("00000000-0000-4000-d000-000000000001"),
        Some("00000000-0000-4000-d000-000000000001".to_string())
    );
}
