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

/// The HTTP client, as one text: the failure types, the refresh policy and the request verbs.
pub(crate) fn client_source() -> String {
    [
        include_str!("../api/client/mod.rs"),
        include_str!("../api/client/errors.rs"),
        include_str!("../api/client/fetched.rs"),
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
