//! Production source, addressed by the file it used to be, for the crate's guard tests.
//!
//! **Role:** each function returns the full text of one logical source file. Where that file has
//! been split into shards, the function concatenates them in declaration order, so a guard that
//! used to `include_str!` the single file still sees every definition exactly once.
//! **Position:** test-only support, called from the guard tests of the modules it names.
//! **Signals & state:** none — every include is resolved at compile time.
//! **Invariants:** concatenation preserves the "exactly one definition" property the
//! `only_item` and `only_body` helpers depend on; a shard must therefore never be listed twice.

/// One shard's text with its test-module declaration removed.
///
/// A production file declares its tests as `#[cfg(test)] #[path = "tests/…"] mod …;`. Those lines
/// carry no behaviour, and leaving them in would make a scrubber cut every shard concatenated after
/// the first one, hiding most of the file from the guard that reads it.
fn production(shard: &str) -> String {
    let mut out = String::with_capacity(shard.len());
    let mut lines = shard.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            for skipped in lines.by_ref() {
                if skipped.trim_end().ends_with(';') {
                    break;
                }
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The HTTP client, as one text: the failure types, the refusal reader, the refresh policy and the
/// request verbs.
pub(crate) fn client_source() -> String {
    [
        include_str!("../api/client/mod.rs"),
        include_str!("../api/client/errors.rs"),
        include_str!("../api/client/fetched.rs"),
        include_str!("../api/client/refusals.rs"),
        include_str!("../api/client/refresh.rs"),
        include_str!("../api/client/requests.rs"),
    ]
    .map(production)
    .concat()
}

/// The telemetry stream consumer.
pub(crate) fn sse_source() -> String {
    production(include_str!("../api/sse.rs"))
}

/// The shared visual primitives, as one text: the small components, the three form controls, the
/// two overlay surfaces, and the registry they share.
pub(crate) fn ui_source() -> String {
    [
        include_str!("../ui/mod.rs"),
        include_str!("../ui/badge.rs"),
        include_str!("../ui/dialog.rs"),
        include_str!("../ui/gates.rs"),
        include_str!("../ui/icons.rs"),
        include_str!("../ui/modal_stack.rs"),
        include_str!("../ui/page_header.rs"),
        include_str!("../ui/search_box.rs"),
        include_str!("../ui/select.rs"),
        include_str!("../ui/sheet.rs"),
        include_str!("../ui/slider.rs"),
    ]
    .map(production)
    .concat()
}

/// The session store and everything it is built from.
pub(crate) fn auth_source() -> String {
    [
        include_str!("../auth/mod.rs"),
        include_str!("../auth/route_guard.rs"),
        include_str!("../auth/session.rs"),
        include_str!("../auth/single_flight.rs"),
        include_str!("../auth/store.rs"),
    ]
    .map(production)
    .concat()
}

/// The live server panel, as one text: the route component, the connect header, the telemetry
/// grid, and the picker and shell that compose them.
pub(crate) fn server_intel_source() -> String {
    [
        include_str!("../../pages/command_center/server_intel/mod.rs"),
        include_str!("../../pages/command_center/server_intel/page.rs"),
        include_str!("../../pages/command_center/server_intel/direct_connect.rs"),
        include_str!("../../pages/command_center/server_intel/player_census.rs"),
        include_str!("../../pages/command_center/server_intel/server_list.rs"),
    ]
    .map(production)
    .concat()
}

/// The doctrine wiki, as one text: the route component, the index, the article surface and the
/// Markdown renderer.
pub(crate) fn wiki_source() -> String {
    [
        include_str!("../../pages/doctrine_and_info/wiki/mod.rs"),
        include_str!("../../pages/doctrine_and_info/wiki/category_nav.rs"),
        include_str!("../../pages/doctrine_and_info/wiki/helpers.rs"),
        include_str!("../../pages/doctrine_and_info/wiki/markdown.rs"),
        include_str!("../../pages/doctrine_and_info/wiki/markdown_article.rs"),
        include_str!("../../pages/doctrine_and_info/wiki/page.rs"),
    ]
    .map(production)
    .concat()
}

/// The modpacks page, as one text: the route component, the list, the manifest, the edit form,
/// the mode switch and the draft type.
pub(crate) fn modpacks_source() -> String {
    [
        include_str!("../../pages/doctrine_and_info/modpacks/mod.rs"),
        include_str!("../../pages/doctrine_and_info/modpacks/mod_table.rs"),
        include_str!("../../pages/doctrine_and_info/modpacks/mode_toggle.rs"),
        include_str!("../../pages/doctrine_and_info/modpacks/pack_edit.rs"),
        include_str!("../../pages/doctrine_and_info/modpacks/pack_editor.rs"),
        include_str!("../../pages/doctrine_and_info/modpacks/page.rs"),
        include_str!("../../pages/doctrine_and_info/modpacks/preset_list.rs"),
    ]
    .map(production)
    .concat()
}

/// The mission library, as one text: the route component, the header and its controls, the hero
/// and the card grid, the dossier sheet and its sections, and the payload comparison behind them.
pub(crate) fn mission_library_source() -> String {
    [
        include_str!("../../pages/mission_hub/library/mod.rs"),
        include_str!("../../pages/mission_hub/library/card_grid.rs"),
        include_str!("../../pages/mission_hub/library/dossier_body.rs"),
        include_str!("../../pages/mission_hub/library/dossier_collaboration.rs"),
        include_str!("../../pages/mission_hub/library/dossier_lifecycle.rs"),
        include_str!("../../pages/mission_hub/library/dossier_sheet.rs"),
        include_str!("../../pages/mission_hub/library/dossier_upload.rs"),
        include_str!("../../pages/mission_hub/library/dossier_upload_panel.rs"),
        include_str!("../../pages/mission_hub/library/dossier_versions.rs"),
        include_str!("../../pages/mission_hub/library/featured_hero.rs"),
        include_str!("../../pages/mission_hub/library/filter_bar.rs"),
        include_str!("../../pages/mission_hub/library/header.rs"),
        include_str!("../../pages/mission_hub/library/mission_diff.rs"),
        include_str!("../../pages/mission_hub/library/page.rs"),
        include_str!("../../pages/mission_hub/library/search_bar.rs"),
    ]
    .map(production)
    .concat()
}

/// The mission overview, as one text: the route component, the dossier header, the shared body
/// and its briefing, and the armory editor's state and dialog.
pub(crate) fn mission_overview_source() -> String {
    [
        include_str!("../../pages/mission_hub/overview/mod.rs"),
        include_str!("../../pages/mission_hub/overview/armory_dialog.rs"),
        include_str!("../../pages/mission_hub/overview/armory_editor.rs"),
        include_str!("../../pages/mission_hub/overview/dossier_body.rs"),
        include_str!("../../pages/mission_hub/overview/header.rs"),
        include_str!("../../pages/mission_hub/overview/intel_briefing.rs"),
        include_str!("../../pages/mission_hub/overview/page.rs"),
    ]
    .map(production)
    .concat()
}

/// The new-mission dialog, as one text.
pub(crate) fn create_dialog_source() -> String {
    [
        include_str!("../../pages/mission_hub/create_dialog/mod.rs"),
        include_str!("../../pages/mission_hub/create_dialog/dialog.rs"),
    ]
    .map(production)
    .concat()
}

/// The operations schedule, as one text: the route component with its split pane, and the
/// operation card the master column repeats.
pub(crate) fn event_schedule_source() -> String {
    [
        include_str!("../../pages/operations/schedule/mod.rs"),
        include_str!("../../pages/operations/schedule/page.rs"),
        include_str!("../../pages/operations/schedule/upcoming_ops.rs"),
    ]
    .map(production)
    .concat()
}

/// The operation dossier, as one text: the route component, the hub body, the mission dossier
/// card, the faction cards, the slotting selector with its squad pane, seat rows, footer actions
/// and assign picker, and the viewer's registration access.
pub(crate) fn event_hub_source() -> String {
    [
        include_str!("../../pages/operations/event_detail/mod.rs"),
        include_str!("../../pages/operations/event_detail/page.rs"),
        include_str!("../../pages/operations/event_detail/hero_countdown.rs"),
        include_str!("../../pages/operations/event_detail/mission_dossier.rs"),
        include_str!("../../pages/operations/event_detail/faction_armory.rs"),
        include_str!("../../pages/operations/event_detail/slotting_selector.rs"),
        include_str!("../../pages/operations/event_detail/squad_pane.rs"),
        include_str!("../../pages/operations/event_detail/seat_row.rs"),
        include_str!("../../pages/operations/event_detail/reservation_actions.rs"),
        include_str!("../../pages/operations/event_detail/assign_picker.rs"),
        include_str!("../../pages/operations/event_detail/registration_access/mod.rs"),
        include_str!("../../pages/operations/event_detail/registration_access/mission_standing.rs"),
        include_str!("../../pages/operations/event_detail/registration_access/place_outlook.rs"),
        include_str!("../../pages/operations/event_detail/registration_access/places_panel.rs"),
        include_str!("../../pages/operations/event_detail/registration_access/refusal_notices.rs"),
        include_str!("../../pages/operations/event_detail/registration_access/seat_eligibility.rs"),
        include_str!(
            "../../pages/operations/event_detail/registration_access/waitlist_promotion.rs"
        ),
    ]
    .map(production)
    .concat()
}

/// The service record, as one text: the route component, the active-orders banner, the combat
/// history table, both leave panels, and the column heading they share.
pub(crate) fn deployments_source() -> String {
    [
        include_str!("../../pages/operations/deployments/mod.rs"),
        include_str!("../../pages/operations/deployments/page.rs"),
        include_str!("../../pages/operations/deployments/active_orders.rs"),
        include_str!("../../pages/operations/deployments/service_record.rs"),
        include_str!("../../pages/operations/deployments/leave_of_absence.rs"),
        include_str!("../../pages/operations/deployments/leave_review_queue.rs"),
        include_str!("../../pages/operations/deployments/table_head.rs"),
    ]
    .map(production)
    .concat()
}

/// The operations calendar, as one text: the route component, the state, the calendar body, the
/// two forms, the mission pickers, the destructive confirmations, and the access panel.
pub(crate) fn event_manager_source() -> String {
    [
        include_str!("../../pages/administration/event_manager/mod.rs"),
        include_str!("../../pages/administration/event_manager/confirm_dialogs.rs"),
        include_str!("../../pages/administration/event_manager/dates.rs"),
        include_str!("../../pages/administration/event_manager/edit_dialog.rs"),
        include_str!("../../pages/administration/event_manager/event_table.rs"),
        include_str!("../../pages/administration/event_manager/lifecycle.rs"),
        include_str!("../../pages/administration/event_manager/mission_picker.rs"),
        include_str!("../../pages/administration/event_manager/page.rs"),
        include_str!("../../pages/administration/event_manager/schedule_dialog.rs"),
        include_str!("../../pages/administration/event_manager/state.rs"),
        include_str!("../../pages/administration/event_manager/access/mod.rs"),
        include_str!("../../pages/administration/event_manager/access/change_report.rs"),
        include_str!("../../pages/administration/event_manager/access/groups/mod.rs"),
        include_str!("../../pages/administration/event_manager/access/groups/group_card.rs"),
        include_str!("../../pages/administration/event_manager/access/groups/group_fields.rs"),
        include_str!("../../pages/administration/event_manager/access/groups/group_form.rs"),
        include_str!("../../pages/administration/event_manager/access/member_search.rs"),
        include_str!("../../pages/administration/event_manager/access/panel.rs"),
        include_str!("../../pages/administration/event_manager/access/participants_table.rs"),
        include_str!("../../pages/administration/event_manager/access/policy_draft.rs"),
        include_str!("../../pages/administration/event_manager/access/policy_editor.rs"),
        include_str!("../../pages/administration/event_manager/access/policy_inheritance.rs"),
        include_str!("../../pages/administration/event_manager/access/policy_lists.rs"),
        include_str!("../../pages/administration/event_manager/access/quota_editor.rs"),
        include_str!("../../pages/administration/event_manager/access/state.rs"),
        include_str!("../../pages/administration/event_manager/access/waitlist_promotion.rs"),
    ]
    .map(production)
    .concat()
}

/// Server control, as one text: the route component, the picker and card, the fleet command
/// console, the deployments panel, the fleet scenario sheet, and the machine-credential sheet.
pub(crate) fn server_control_source() -> String {
    [
        include_str!("../../pages/administration/server_control/mod.rs"),
        include_str!("../../pages/administration/server_control/page.rs"),
        include_str!("../../pages/administration/server_control/server_cards.rs"),
        include_str!("../../pages/administration/server_control/fleet_commands/mod.rs"),
        include_str!("../../pages/administration/server_control/fleet_commands/command_history.rs"),
        include_str!(
            "../../pages/administration/server_control/fleet_commands/command_requests.rs"
        ),
        include_str!("../../pages/administration/server_control/fleet_commands/command_wording.rs"),
        include_str!("../../pages/administration/server_control/mission_deployments/mod.rs"),
        include_str!(
            "../../pages/administration/server_control/mission_deployments/deployment_list.rs"
        ),
        include_str!(
            "../../pages/administration/server_control/mission_deployments/deployment_refusal.rs"
        ),
        include_str!(
            "../../pages/administration/server_control/mission_deployments/deployment_request.rs"
        ),
        include_str!(
            "../../pages/administration/server_control/mission_deployments/deployment_wording.rs"
        ),
        include_str!("../../pages/administration/server_control/fleet_scenarios/mod.rs"),
        include_str!("../../pages/administration/server_control/fleet_scenarios/scenario_sheet.rs"),
        include_str!(
            "../../pages/administration/server_control/fleet_scenarios/scenario_wording.rs"
        ),
        include_str!("../../pages/administration/server_control/machine_credentials/mod.rs"),
        include_str!(
            "../../pages/administration/server_control/machine_credentials/credential_sheet.rs"
        ),
        include_str!(
            "../../pages/administration/server_control/machine_credentials/credential_text.rs"
        ),
    ]
    .map(production)
    .concat()
}

/// The personnel roster, as one text: the route component, the roster table, the dossier and the
/// role and sanction controls.
pub(crate) fn personnel_source() -> String {
    [
        include_str!("../../pages/administration/personnel/mod.rs"),
        include_str!("../../pages/administration/personnel/dossier.rs"),
        include_str!("../../pages/administration/personnel/member_roster.rs"),
        include_str!("../../pages/administration/personnel/page.rs"),
        include_str!("../../pages/administration/personnel/role_dialog.rs"),
    ]
    .map(production)
    .concat()
}

/// The content manager, as one text: the route component first, so the guards that read the boot
/// path, the hydrate and the two panes see them in the order the file declares them, then the post
/// shape, the list row, the editor form and the hero upload.
pub(crate) fn content_source() -> String {
    [
        include_str!("../../pages/administration/content_manager/page.rs"),
        include_str!("../../pages/administration/content_manager/mod.rs"),
        include_str!("../../pages/administration/content_manager/article_table.rs"),
        include_str!("../../pages/administration/content_manager/doc.rs"),
        include_str!("../../pages/administration/content_manager/editor_form.rs"),
        include_str!("../../pages/administration/content_manager/hero_upload.rs"),
    ]
    .map(production)
    .concat()
}

/// The audit trail, as one text: the route component, the filter box and the trail itself.
pub(crate) fn audit_source() -> String {
    [
        include_str!("../../pages/administration/audit_logs/mod.rs"),
        include_str!("../../pages/administration/audit_logs/filter_bar.rs"),
        include_str!("../../pages/administration/audit_logs/log_table.rs"),
        include_str!("../../pages/administration/audit_logs/page.rs"),
    ]
    .map(production)
    .concat()
}
