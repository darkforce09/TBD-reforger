//! Source pins: the production text of the operations calendar, server control, the personnel
//! roster, the content manager and the audit trail, for the guard tests that read it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the administration pages' guard
//! tests.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::production_source;

/// The operations calendar, as one text: the route component, the state, the calendar body, the
/// two forms, the mission pickers, the game server choice, the destructive confirmations, and the
/// access panel.
pub(crate) fn event_manager_source() -> String {
    production_source(&[
        include_str!("../event_manager/mod.rs"),
        include_str!("../event_manager/confirm_dialogs.rs"),
        include_str!("../event_manager/dates.rs"),
        include_str!("../event_manager/edit_dialog.rs"),
        include_str!("../event_manager/event_table.rs"),
        include_str!("../event_manager/lifecycle.rs"),
        include_str!("../event_manager/mission_picker.rs"),
        include_str!("../event_manager/page.rs"),
        include_str!("../event_manager/schedule_dialog.rs"),
        include_str!("../event_manager/server_choice.rs"),
        include_str!("../event_manager/state.rs"),
        include_str!("../event_manager/access/mod.rs"),
        include_str!("../event_manager/access/change_report.rs"),
        include_str!("../event_manager/access/groups/mod.rs"),
        include_str!("../event_manager/access/groups/group_card.rs"),
        include_str!("../event_manager/access/groups/group_fields.rs"),
        include_str!("../event_manager/access/groups/group_form.rs"),
        include_str!("../event_manager/access/member_search.rs"),
        include_str!("../event_manager/access/panel.rs"),
        include_str!("../event_manager/access/participants_table.rs"),
        include_str!("../event_manager/access/policy_draft.rs"),
        include_str!("../event_manager/access/policy_editor.rs"),
        include_str!("../event_manager/access/policy_inheritance.rs"),
        include_str!("../event_manager/access/policy_lists.rs"),
        include_str!("../event_manager/access/quota_editor.rs"),
        include_str!("../event_manager/access/state.rs"),
        include_str!("../event_manager/access/waitlist_promotion.rs"),
    ])
}

/// Server control, as one text: the route component, the server registry and its registration
/// sheet, the picker and card, the fleet command console, the deployments panel, the fleet
/// scenario sheet, and the machine-credential sheet.
pub(crate) fn server_control_source() -> String {
    production_source(&[
        include_str!("../server_control/mod.rs"),
        include_str!("../server_control/page.rs"),
        include_str!("../server_control/server_cards.rs"),
        include_str!("../server_control/server_card_telemetry.rs"),
        include_str!("../server_control/server_registry/mod.rs"),
        include_str!("../server_control/server_registry/registration_sheet.rs"),
        include_str!("../server_control/server_registry/registration_wording.rs"),
        include_str!("../server_control/fleet_commands/mod.rs"),
        include_str!("../server_control/fleet_commands/command_history.rs"),
        include_str!("../server_control/fleet_commands/command_requests.rs"),
        include_str!("../server_control/fleet_commands/command_wording.rs"),
        include_str!("../server_control/fleet_commands/console_command_form.rs"),
        include_str!("../server_control/mission_deployments/mod.rs"),
        include_str!("../server_control/mission_deployments/deployment_list.rs"),
        include_str!("../server_control/mission_deployments/deployment_refusal.rs"),
        include_str!("../server_control/mission_deployments/deployment_request.rs"),
        include_str!("../server_control/mission_deployments/deployment_wording.rs"),
        include_str!("../server_control/fleet_scenarios/mod.rs"),
        include_str!("../server_control/fleet_scenarios/scenario_sheet.rs"),
        include_str!("../server_control/fleet_scenarios/scenario_wording.rs"),
        include_str!("../server_control/machine_credentials/mod.rs"),
        include_str!("../server_control/machine_credentials/credential_sheet.rs"),
        include_str!("../server_control/machine_credentials/credential_text.rs"),
    ])
}

/// The personnel roster, as one text: the route component, the roster table, the dossier and the
/// role and sanction controls.
pub(crate) fn personnel_source() -> String {
    production_source(&[
        include_str!("../personnel/mod.rs"),
        include_str!("../personnel/dossier.rs"),
        include_str!("../personnel/member_roster.rs"),
        include_str!("../personnel/page.rs"),
        include_str!("../personnel/role_dialog.rs"),
    ])
}

/// The content manager, as one text: the route component first, so the guards that read the boot
/// path, the hydrate and the two panes see them in the order the file declares them, then the post
/// shape, the list row, the editor form and the hero upload.
pub(crate) fn content_source() -> String {
    production_source(&[
        include_str!("../content_manager/page.rs"),
        include_str!("../content_manager/mod.rs"),
        include_str!("../content_manager/article_table.rs"),
        include_str!("../content_manager/doc.rs"),
        include_str!("../content_manager/editor_form.rs"),
        include_str!("../content_manager/hero_upload.rs"),
    ])
}

/// The audit trail, as one text: the route component, the filter box and the trail itself.
pub(crate) fn audit_source() -> String {
    production_source(&[
        include_str!("../audit_logs/mod.rs"),
        include_str!("../audit_logs/filter_bar.rs"),
        include_str!("../audit_logs/log_table.rs"),
        include_str!("../audit_logs/page.rs"),
    ])
}
