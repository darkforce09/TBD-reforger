//! Role: Module boundary for editor/state/operations/context.
//! Position: `editor/state/operations/context` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

thread_local! {
    pub(crate) static OPS_CTX: RefCell<Option<OpsCtx>> = const { RefCell::new(None) };

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
use crate::editor::arsenal::asset_catalog::PlacePayload;
use crate::editor::mission_editor::AssetPickerState;
use crate::editor::panels::outliner::build_outliner_with_comments;
use crate::editor::panels::outliner::OutlinerNode;
use crate::editor::state::doc_host::DocHandle;
use crate::editor::state::history as mission_history;
use selection::SelectionHandle;
use website_map_engine::data::store::operations::entity::{comment_rows, connection_id_in_doc};
use website_map_engine::editing::tools::selection;
use website_map_engine::frame::EngineHandle;

/// Expose crate :: v2 :: core :: api :: dto ::  mission env at this domain boundary.
pub use crate::v2::core::api::dto::MissionEnv;
use leptos::prelude::{GetUntracked, RwSignal, Set};

use std::cell::{Cell, RefCell};

/// Expose website mission core :: doc :: operations :: projections :: faction rows at this domain boundary.
pub(super) use website_map_engine::data::store::operations::projections::faction_rows;

/// Expose website mission core :: doc :: operations :: projections :: layer rows at this domain boundary.
pub(super) use website_map_engine::data::store::operations::projections::layer_rows;

/// Expose website mission core :: doc :: operations :: projections :: slot rows at this domain boundary.
pub(super) use website_map_engine::data::store::operations::projections::slot_rows;

/// Expose website mission core :: doc :: operations :: projections :: squad rows at this domain boundary.
pub(super) use website_map_engine::data::store::operations::projections::squad_rows;

mod registration;

/// Expose registration :: { cancel armed composition , close asset picker , close comment editor , open asset picker , open comment editor , place with crew , set asset picker signal , set comment editor signal , set ctx , set place with crew ,  zone draft , } at this domain boundary.
pub use registration::{
    cancel_armed_composition, close_asset_picker, close_comment_editor, open_asset_picker,
    open_comment_editor, place_with_crew, set_asset_picker_signal, set_comment_editor_signal,
    set_ctx, set_place_with_crew, ZoneDraft,
};

/// Expose registration :: {  ops ctx ,  pending } at this domain boundary.
pub(crate) use registration::{OpsCtx, Pending};
mod environment;

/// Expose environment :: { read env , read env value , read title , set title , slots json , update environment , } at this domain boundary.
pub use environment::{
    read_env, read_env_value, read_title, set_title, slots_json, update_environment,
};
mod attributes;

/// Expose attributes :: { close attributes , open arsenal , open attributes } at this domain boundary.
pub use attributes::{close_attributes, open_arsenal, open_attributes};
mod refresh;

/// Expose refresh :: bump doc tick at this domain boundary.
pub(crate) use refresh::bump_doc_tick;

/// Expose refresh :: { close connections panel , open connections panel , refresh docks , refresh selection mirrors , seed new mission template , set connection selection signal , set connections panel signal , } at this domain boundary.
pub use refresh::{
    close_connections_panel, open_connections_panel, refresh_docks, refresh_selection_mirrors,
    seed_new_mission_template, set_connection_selection_signal, set_connections_panel_signal,
};
