//! Ticketboard — native egui projection of the `.ai/tickets/` registry (T-915).
//!
//! Every `T-*.toml` (parents AND children) is parsed through `tbd-tickets` and
//! rendered as a status board with a full-field detail panel, plus (T-915.2)
//! verbatim wave lanes off `wave.lock`, a program tree, composable filters, and
//! the owns-collision explainer, plus (T-915.3) the trust banner —
//! `cargo xtask ticket check --strict` as a streamed subprocess, a notify file
//! watch with debounced auto-reload, and the git-dirty chip. T-915.4 adds the
//! mutation UI: every write shells `cargo xtask ticket <verb>` as a subprocess
//! (single-flight queue, CAS guard, verbatim refusals, no auto-repack ever) —
//! the app itself writes no ticket bytes; its only direct file writes are the
//! preferences (picked repo root, T-920.2 viewer-column width) in eframe
//! Storage in the user config dir. T-915.5 adds
//! the metrics dashboard over the `.ai/tickets/metrics/` run receipts: explicit
//! no-receipts state, per-ticket / per-agent token + elapsed aggregations, and
//! named error rows for malformed files — never zeros for missing data. T-918.2
//! adds provenance rendering — measured vs estimated, NEVER summed: stamp rows
//! carry the `~` glyph + verbatim estimate_note tooltip, the detail panel gains
//! a "tokens (estimated)" row off `.ai/tickets/estimates/<id>.json`, and the
//! Metrics tab gains the structurally separate "Estimated (historical)" panel
//! (per-class / per-domain; estimates have no agent). T-918.4 adds the in-app
//! markdown viewer: spec/plan/`.md`-citation clicks render the document in a
//! right-pane egui_commonmark view — read-only, repo-root-fenced, worker-thread
//! reads, raw-text fallback with a naming note — with external-open kept as the
//! secondary action. T-920.2 reshapes both right-pane surfaces: main_goal (then
//! summary) renders label-free in the detail header directly under the title
//! and the body sections start at context; the viewer becomes a third COLUMN
//! beside the detail panel (both visible; Back collapses just the column; its
//! width drag-resizable and persisted in eframe Storage; narrow windows degrade
//! to the viewer alone); cards gain a main_goal hover tooltip.
//! Design authority: `docs/platform/t915_ticketboard_design.md` +
//! `docs/platform/t917_ticket_schema_v2.md` §Provenance + B.4 +
//! `docs/platform/t920_body_obligations.md` §Board changes.

mod app;
mod board;
mod corpus;
mod detail;
mod discovery;
mod estimates;
mod facets;
mod filters;
mod gitstatus;
mod metrics;
mod mutate;
mod subproc;
#[cfg(test)]
mod testutil;
mod tree;
mod trust;
mod verbs;
mod viewer;
mod watch;
mod wavelock;
mod waves;

use eframe::egui;

const USAGE: &str = "\
ticketboard [REPO_ROOT]

Native viewer for the .ai/tickets registry.

REPO_ROOT   repo root containing .ai/tickets/ (wins over discovery); when absent
            the app walks up from the current directory looking for .ai/tickets/.
";

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return Ok(());
    }
    let arg_root = discovery::positional_arg(args);
    let cwd = std::env::current_dir().ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Ticketboard")
            .with_app_id("ticketboard")
            .with_inner_size([1500.0, 950.0])
            .with_min_inner_size([720.0, 480.0]),
        renderer: renderer(),
        ..Default::default()
    };
    eframe::run_native(
        "ticketboard",
        options,
        Box::new(move |cc| Ok(Box::new(app::TicketboardApp::new(cc, arg_root, cwd)))),
    )
}

/// wgpu is the default backend; a `--features glow` build selects the glow
/// fallback instead (driver quirks — design §Framework).
#[cfg(feature = "glow")]
fn renderer() -> eframe::Renderer {
    eframe::Renderer::Glow
}

#[cfg(not(feature = "glow"))]
fn renderer() -> eframe::Renderer {
    eframe::Renderer::Wgpu
}
