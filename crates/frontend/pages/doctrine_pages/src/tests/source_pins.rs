//! Source pins: the production text of the doctrine wiki and the modpacks page, for the guard tests
//! that read it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the doctrine pages' guard tests.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::production_source;

/// The doctrine wiki, as one text: the route component, the shared state, the index, the article
/// pane, the block renderer, the revision history and the save path.
pub(crate) fn wiki_source() -> String {
    production_source(&[
        include_str!("../wiki/mod.rs"),
        include_str!("../wiki/api_paths.rs"),
        include_str!("../wiki/category_nav.rs"),
        include_str!("../wiki/display_text.rs"),
        include_str!("../wiki/page.rs"),
        include_str!("../wiki/page_state.rs"),
        include_str!("../wiki/article/mod.rs"),
        include_str!("../wiki/article/article_body.rs"),
        include_str!("../wiki/article/article_header.rs"),
        include_str!("../wiki/article/article_pane.rs"),
        include_str!("../wiki/article/article_state.rs"),
        include_str!("../wiki/blocks/mod.rs"),
        include_str!("../wiki/blocks/block_mapping.rs"),
        include_str!("../wiki/blocks/callout_style.rs"),
        include_str!("../wiki/blocks/element_views.rs"),
        include_str!("../wiki/blocks/inline_mapping.rs"),
        include_str!("../wiki/blocks/render_tree.rs"),
        include_str!("../wiki/blocks/table_mapping.rs"),
        include_str!("../wiki/revisions/mod.rs"),
        include_str!("../wiki/revisions/revision_fetches.rs"),
        include_str!("../wiki/revisions/revision_list.rs"),
        include_str!("../wiki/revisions/revision_view.rs"),
        include_str!("../wiki/saving/mod.rs"),
        include_str!("../wiki/saving/save_problem_view.rs"),
        include_str!("../wiki/saving/save_refusal.rs"),
        include_str!("../wiki/saving/save_requests.rs"),
        include_str!("../wiki/saving/save_submission.rs"),
    ])
}

/// The modpacks page, as one text: the route component, the list, the manifest, the edit form,
/// the mode switch and the draft type.
pub(crate) fn modpacks_source() -> String {
    production_source(&[
        include_str!("../modpacks/mod.rs"),
        include_str!("../modpacks/mod_table.rs"),
        include_str!("../modpacks/mode_toggle.rs"),
        include_str!("../modpacks/pack_edit.rs"),
        include_str!("../modpacks/pack_editor.rs"),
        include_str!("../modpacks/page.rs"),
        include_str!("../modpacks/preset_list.rs"),
    ])
}
