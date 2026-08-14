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
//! the app itself writes no ticket bytes; its only direct file write is the
//! picked-repo preference in eframe Storage in the user config dir. T-915.5 adds
//! the metrics dashboard over the `.ai/tickets/metrics/` run receipts: explicit
//! no-receipts state, per-ticket / per-agent token + elapsed aggregations, and
//! named error rows for malformed files — never zeros for missing data. Design
//! authority: `docs/platform/t915_ticketboard_design.md`.

mod app;
mod board;
mod corpus;
mod discovery;
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
