//! Ticketboard — a native egui projection of the `.ai/tickets/` registry.
//!
//! Every `T-*.toml`, parents AND children, is parsed through `ticket-engine` and rendered as a
//! status board with a full-field detail panel, verbatim wave lanes off `wave.lock`, a program
//! tree, composable filters and the owns-collision explainer.
//!
//! The trust banner runs `cargo xtask ticket check --strict` as a streamed subprocess and pairs it
//! with a notify file watch (debounced auto-reload) and a git-dirty chip. Every mutation shells
//! `cargo xtask ticket <verb>` as a subprocess behind a single-flight queue and a compare-and-set
//! guard, surfacing refusals verbatim and never repacking on its own: the app writes no ticket
//! bytes itself. Its only direct file writes are the preferences — picked repository root and
//! viewer-column width — in eframe Storage under the user config directory.
//!
//! The metrics dashboard reads the `.ai/tickets/metrics/` run receipts and reports per-ticket and
//! per-agent token and elapsed aggregations, an explicit no-receipts state, and named error rows
//! for malformed files; missing data never renders as a zero. Provenance stays separated:
//! measured and estimated values are NEVER summed, stamp rows carry the `~` glyph with the
//! verbatim estimate note as a tooltip, the detail panel shows a "tokens (estimated)" row off
//! `.ai/tickets/estimates/<id>.json`, and the Metrics tab keeps a structurally separate
//! "Estimated (historical)" panel broken down per class and per domain (estimates have no agent).
//!
//! Clicking a spec, a plan or a `.md` citation opens the document in the in-app markdown viewer: a
//! read-only `egui_commonmark` view fenced to the repository root, read on a worker thread, with a
//! raw-text fallback carrying a naming note and external-open as the secondary action. The right
//! pane holds two surfaces: the detail panel renders `main_goal` (else `summary`) label-free
//! directly under the title with the body sections starting at context, and the viewer is a third
//! COLUMN beside it — both visible at once, Back collapses only the column, its width is
//! drag-resizable and persisted in eframe Storage, and narrow windows degrade to the viewer alone.
//! Cards carry a `main_goal` hover tooltip.

mod application;
mod core;
mod document_viewer;
mod execution_metrics;
mod repository_status;
#[cfg(test)]
#[path = "tests/support/mod.rs"]
mod test_support;
mod ticket_actions;
mod ticket_browser;
mod ticket_registry;
mod wave_plan;

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
    let arg_root = ticket_registry::services::discovery::positional_arg(args);
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
        Box::new(move |cc| {
            Ok(Box::new(application::TicketboardApp::new(
                cc, arg_root, cwd,
            )))
        }),
    )
}

/// wgpu is the default backend; a `--features glow` build selects the glow
/// fallback instead (driver quirks — ).
#[cfg(feature = "glow")]
fn renderer() -> eframe::Renderer {
    eframe::Renderer::Glow
}

#[cfg(not(feature = "glow"))]
fn renderer() -> eframe::Renderer {
    eframe::Renderer::Wgpu
}

#[cfg(test)]
#[path = "tests/architecture_rules.rs"]
mod architecture_rules;
