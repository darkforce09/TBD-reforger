mod action_dispatch;
mod background_events;
mod background_loading;
mod command_execution;
mod events;
mod feature_views;
mod lifecycle;
mod preferences;
mod shell_screens;
mod ticket_command_views;
mod window;
mod window_layout;
mod workspace_state;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Button, Id, Layout, Panel, RichText, ScrollArea, Spinner, Ui};
use egui_commonmark::CommonMarkCache;
use ticket_engine::StatusName;

use crate::core::process::{self as subproc, BoundedLog, ProcessEvent, ProcessHandle};
use crate::document_viewer::services::document_loading::{self as viewer, ViewerState};
use crate::execution_metrics::estimated::{
    self as estimates, EstimatedSortPair, EstimatedTableKind, EstimatesState,
};
use crate::execution_metrics::measured::{self as metrics, MetricsState, SortPair, TableKind};
use crate::repository_status::models::check_status::{self as trust, CheckModel, Coalescer};
use crate::repository_status::models::git_status::{self as gitstatus, GitChip};
use crate::repository_status::services::file_watch::{self as watch, Debouncer};
use crate::ticket_actions::models::{
    CommandExecutionState, CommandOutcome, Dialog, MutationContext, Toast,
};
use crate::ticket_actions::services::commands as verbs;
use crate::ticket_browser::models::program_tree::{self as tree, TreeModel};
use crate::ticket_browser::models::status_board::BoardModel;
use crate::ticket_browser::services::filtering::{FilterIndex, Filters};
use crate::ticket_browser::services::scope_facets::{self as facets, FacetOptions, VocabTree};
use crate::ticket_registry::models::corpus::{Corpus, LoadError};
use crate::ticket_registry::models::projection as board;
use crate::ticket_registry::services::discovery;
use crate::wave_plan::models::wave_projection::WavesModel;
use crate::wave_plan::services::lock_file::LockState;
use background_loading::LoadBundle;
use ticket_command_views as mutate;

use crate::core::process::external_open::open_path;
use crate::core::time::epoch_secs;
use crate::ticket_browser::ui::filter_bar::filter_bar_ui;
use events::*;
use feature_views::*;
use preferences::*;
use shell_screens::*;
use window_layout::*;
use workspace_state::*;
pub struct TicketboardApp {
    repo_root: Option<PathBuf>,
    state: State,
    tab: Tab,
    load_rx: Option<Receiver<LoadBundle>>,
    pick_rx: Option<Receiver<Option<PathBuf>>>,
    // ---- trust banner + watch ----
    // Resolved once at startup ($CARGO → PATH → ~/.cargo/bin — GUI PATH is bare).
    cargo: PathBuf,
    check: CheckModel,
    check_log: BoundedLog,
    check_handle: Option<ProcessHandle>,
    /// Verbatim-output pane toggle (auto-opens when a check lands red).
    show_output: bool,
    git_chip: GitChip,
    git_lines: Vec<String>,
    git_handle: Option<ProcessHandle>,
    git_flight: Coalescer,
    git_expanded: bool,
    watch: Option<watch::WatchHandle>,
    watch_rx: Option<Receiver<()>>,
    /// The load-bearing `.ai/tickets` watch failed to arm — banner note.
    watch_error: Option<String>,
    debounce: Debouncer,
    /// Millisecond clock base for the debouncer (monotonic).
    epoch: Instant,
    // ---- mutation UI over subprocess xtask verbs ----
    // Single-flight verb queue + in-flight subprocess + verbatim log + drawer.
    verb: CommandExecutionState,
    /// The one open mutation dialog (confirm / form), CAS guard inside.
    dialog: Option<Dialog>,
    toasts: Vec<Toast>,
    /// Advanced raw set-status dropdown selection (detail panel).
    advanced_status: Option<StatusName>,
    // ---- in-app markdown viewer ----
    // Viewer pane state machine — lives on the app (not WorkspaceState) so a
    // watch-triggered corpus reload never closes an open document.
    viewer: ViewerState,
    /// In-flight worker read; replaced wholesale on every open (the old
    /// channel's late result is additionally dropped by `ViewerState::land`).
    viewer_rx: Option<Receiver<viewer::LoadedDocument>>,
    /// egui_commonmark render cache (render-side only — no document state).
    md_cache: CommonMarkCache,
    /// Viewer column width — loaded from eframe Storage at startup
    /// ([`parse_viewer_width`]: revalidated + clamped), tracked across drags
    /// while the column is on screen, persisted in `save`.
    viewer_w: f32,
}

mod workspace_reload;

#[cfg(test)]
#[path = "tests/rendering.rs"]
mod rendering_tests;
