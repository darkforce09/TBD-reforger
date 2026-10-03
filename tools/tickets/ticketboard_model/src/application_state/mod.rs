//! The egui-free half of the ticketboard application: its tabs and actions, the loaded board,
//! reloads that keep selections, the right-hand column layout and the combined background load.
//!
//! **Role:** [`events::Action`] and [`events::Tab`], with a conversion from each feature's events;
//! [`workspace_state::State`] and [`workspace_state::WorkspaceState`], the loaded board with every
//! projection, filter and sort; [`WorkspaceState::reload`](workspace_state::WorkspaceState::reload),
//! which keeps selections by id; [`window_layout::right_pane`]; and
//! [`background_loading::spawn_load`], which reads everything a board needs on one worker thread.
//! **Position:** over every feature module; `apps/ticketboard`'s `TicketboardApp` owns a
//! [`workspace_state::State`], paints it and applies the [`events::Action`]s its views emit.
//! **Signals & state:** [`workspace_state::WorkspaceState`] is the mutable board session;
//! [`background_loading::spawn_load`] runs one worker thread per load and reports over a channel.
//! **Invariants:** no feature imports this module; filters and sorts survive a reload while raw
//! indices are resolved again by ticket id; a malformed ticket refuses the corpus, while lock,
//! receipt, estimate and vocabulary failures stay local to their displays.

pub mod background_loading;
pub mod events;
pub mod preferences;
pub mod window_layout;
mod workspace_reload;
pub mod workspace_state;

use std::path::PathBuf;

use ticket_model::ScopeVocab;

use crate::execution_metrics::estimated::{
    self as estimates, EstimatedSortPair, EstimatedTableKind, EstimatesState,
};
use crate::execution_metrics::measured::{self as metrics, MetricsState, SortPair, TableKind};
use crate::ticket_actions::models::Dialog;
use crate::ticket_actions::services::commands as verbs;
use crate::ticket_browser::models::program_tree::{self as tree, TreeModel};
use crate::ticket_browser::models::status_board::BoardModel;
use crate::ticket_browser::services::filtering::{FilterIndex, Filters};
use crate::ticket_browser::services::scope_facets::{self as facets, FacetOptions};
use crate::ticket_registry::models::corpus::{Corpus, LoadError};
use crate::ticket_registry::models::projection as board;
use crate::wave_plan::models::wave_projection::WavesModel;
use crate::wave_plan::services::lock_file::LockState;
use background_loading::LoadBundle;
use workspace_state::{ReloadPreferences, WorkspaceState};
