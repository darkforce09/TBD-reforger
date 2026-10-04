//! Source pins: the production text of the mission library, the mission overview and the new-
//! mission dialog, for the guard tests that read it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the mission hub pages' guard
//! tests.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::production_source;

/// The mission library, as one text: the route component, the header and its controls, the hero
/// and the card grid, the dossier sheet and its sections, and the payload comparison behind them.
pub(crate) fn mission_library_source() -> String {
    production_source(&[
        include_str!("../library/mod.rs"),
        include_str!("../library/card_grid.rs"),
        include_str!("../library/dossier_body.rs"),
        include_str!("../library/dossier_collaboration.rs"),
        include_str!("../library/dossier_lifecycle.rs"),
        include_str!("../library/dossier_sheet.rs"),
        include_str!("../library/dossier_upload.rs"),
        include_str!("../library/dossier_upload_panel.rs"),
        include_str!("../library/dossier_versions.rs"),
        include_str!("../library/featured_hero.rs"),
        include_str!("../library/filter_bar.rs"),
        include_str!("../library/header.rs"),
        include_str!("../library/mission_diff.rs"),
        include_str!("../library/page.rs"),
        include_str!("../library/search_bar.rs"),
    ])
}

/// The mission overview, as one text: the route component, the dossier header, the shared body
/// and its briefing, and the armory editor's state and dialog.
pub(crate) fn mission_overview_source() -> String {
    production_source(&[
        include_str!("../overview/mod.rs"),
        include_str!("../overview/armory_dialog.rs"),
        include_str!("../overview/armory_editor.rs"),
        include_str!("../overview/dossier_body.rs"),
        include_str!("../overview/header.rs"),
        include_str!("../overview/intel_briefing.rs"),
        include_str!("../overview/page.rs"),
    ])
}

/// The new-mission dialog, as one text.
pub(crate) fn create_dialog_source() -> String {
    production_source(&[
        include_str!("../create_dialog/mod.rs"),
        include_str!("../create_dialog/dialog.rs"),
    ])
}
