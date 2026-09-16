//! Role: the editor context installed once at load — the handles every panel reaches the open
//! mission through (the document, the render engine, the selection), the Leptos signals that mirror
//! that document into the docks, and the handful of side signals a panel opens or closes.
//! Position: `editor/state` in the frontend editor shell.
//! Signals & state: one thread-local holding the installed context, plus one thread-local per side
//! signal (asset picker, comment editor, Connections panel, connection selection) that a component
//! registers when it mounts.
//! Invariants: the handles are `!Send` `Rc`s, so everything here is wasm-only and reached through
//! thread-locals rather than passed down the component tree. Every entry point opens exactly one
//! borrow of the context, and any document borrow it takes is scoped to drop before the post-edit
//! tail opens its own read borrows. Nothing here is document state: an unset signal is silence, not
//! an error, and no undo step is ever minted.
#![cfg(target_arch = "wasm32")]

thread_local! {
    /// The one installed editor context, or `None` before the page installs it and after it tears
    /// it down. Every entry point in this module opens exactly one borrow of it.
    pub(crate) static EDITOR_CONTEXT: RefCell<Option<EditorContext>> = const { RefCell::new(None) };

    static PLACE_WITH_CREW: Cell<bool> = const { Cell::new(true) };
}
thread_local! {

    static ASSET_PICKER: RefCell<Option<RwSignal<Option<AssetPickerState>>>> =
        const { RefCell::new(None) };
}
thread_local! {

    static COMMENT_EDITOR: RefCell<Option<RwSignal<Option<String>>>> = const { RefCell::new(None) };
}
thread_local! {

    static CONNECTIONS_PANEL: RefCell<Option<RwSignal<bool>>> = const { RefCell::new(None) };
}
thread_local! {

    static CONNECTION_SELECTION: RefCell<Option<RwSignal<Option<String>>>> =
        const { RefCell::new(None) };
}
use crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload;
use crate::v2::apps::editor::mission_editor::AssetPickerState;
use crate::v2::apps::editor::panels::outliner::build_outliner_with_comments;
use crate::v2::apps::editor::panels::outliner::OutlinerNode;
use crate::v2::apps::editor::state::doc_host::DocHandle;
use crate::v2::apps::editor::state::history as mission_history;
use selection::SelectionHandle;
use website_map_engine::data::store::operations::entity::{comment_rows, connection_id_in_doc};
use website_map_engine::editing::tools::selection;
use website_map_engine::frame::EngineHandle;

use crate::v2::core::api::dto::MissionEnv;
use leptos::prelude::{GetUntracked, RwSignal, Set};

use std::cell::{Cell, RefCell};

use website_map_engine::data::store::operations::projections::faction_rows;
use website_map_engine::data::store::operations::projections::layer_rows;
use website_map_engine::data::store::operations::projections::slot_rows;
use website_map_engine::data::store::operations::projections::squad_rows;

mod installation;

/// Expose installation :: { cancel armed composition , close asset picker , close comment editor , install , open asset picker , open comment editor , place with crew , set asset picker signal , set comment editor signal , set place with crew ,  zone draft , } at this domain boundary.
pub use installation::{
    cancel_armed_composition, close_asset_picker, close_comment_editor, install, open_asset_picker,
    open_comment_editor, place_with_crew, set_asset_picker_signal, set_comment_editor_signal,
    set_place_with_crew, ZoneDraft,
};

/// Expose installation :: {  editor context ,  pending } at this domain boundary.
pub(crate) use installation::{EditorContext, Pending};
mod document_fields;

/// Expose document fields :: { read env , read env value , read title , set title , slots json , update environment , } at this domain boundary.
pub use document_fields::{
    read_env, read_env_value, read_title, set_title, slots_json, update_environment,
};
mod attributes_modal;

/// Expose attributes modal :: { close attributes , open arsenal , open attributes } at this domain boundary.
pub use attributes_modal::{close_attributes, open_arsenal, open_attributes};
mod dock_mirrors;

/// Expose dock mirrors :: bump doc tick at this domain boundary.
pub(crate) use dock_mirrors::bump_doc_tick;

/// Expose dock mirrors :: { close connections panel , open connections panel , refresh docks , refresh selection mirrors , seed new mission template , set connection selection signal , set connections panel signal , } at this domain boundary.
pub use dock_mirrors::{
    close_connections_panel, open_connections_panel, refresh_docks, refresh_selection_mirrors,
    seed_new_mission_template, set_connection_selection_signal, set_connections_panel_signal,
};
