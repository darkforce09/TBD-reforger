//! The operation's game server: the values the two forms send, the choice's labels, and the
//! wiring that puts them in the create and edit bodies.

use super::*;
use frontend_api_dtos::{DataEnvelope, ServerRowDto};
use frontend_test_support::class_r_scrub::{live_code, live_source};
use frontend_test_support::fixtures::golden;

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

/// The choice names each captured server, and marks the deactivated ones.
#[test]
fn the_choice_names_each_server_and_marks_the_deactivated() {
    let servers: DataEnvelope<ServerRowDto> =
        serde_json::from_str(golden!("GET__servers.json")).unwrap();
    let labels: Vec<String> = servers.data.iter().map(server_choice_label).collect();
    assert_eq!(
        labels,
        [
            "TBD Primary — Everon",
            "TBD Secondary — Arland (deactivated)",
            "TBD Staging — Sandbox (deactivated)",
        ]
    );
}

/// The schedule form's create names the chosen server, the edit form's save diffs the choice
/// against the operation's server, and opening the edit form seeds the choice from the row.
#[test]
fn both_forms_send_the_chosen_server() {
    let compact = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let schedule = compact(&live_source(include_str!("../schedule_dialog.rs")));
    assert!(schedule.contains("chosen_server_id(&server_id.get_untracked())"));
    assert!(schedule.contains("body[\"server_id\"]=serde_json::Value::String(server)"));
    assert!(schedule.contains("server_picker(st.servers,server_id)"));
    let edit = compact(&live_source(include_str!("../edit_dialog.rs")));
    assert!(
        edit.contains(
            "server_id_change(orig.server_id.as_ref().map(|id|id.as_str()),&edit_server_id.get_untracked(),)"
        )
    );
    assert!(edit.contains("body.insert(\"server_id\".into(),server)"));
    assert!(edit.contains("server_picker(st.servers,edit_server_id)"));
    let table = compact(&live_code(include_str!("../event_table.rs")));
    assert!(table.contains(
        "edit_server_id.set(op.server_id.as_ref().map(ToString::to_string).unwrap_or_default(),)"
    ));
    let state = compact(&live_code(include_str!("../state.rs")));
    assert!(state.contains("letwanted=form_open.get()||edit_open.get();"));
    assert!(state.contains("server_registry::load_servers(store)"));
}
