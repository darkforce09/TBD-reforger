//! Ticketboard — native egui viewer over the `.ai/tickets/` registry (T-915.1).
//!
//! Read-only projection: every `T-*.toml` (parents AND children) is parsed through
//! `tbd-tickets` and rendered as a status board with a full-field detail panel,
//! plus (T-915.2) verbatim wave lanes off `wave.lock`, a program tree, composable
//! filters, and the owns-collision explainer. The app writes nothing under the
//! repo; the only preference store is eframe Storage in the user config dir.
//! Design authority: `docs/platform/t915_ticketboard_design.md`.

mod app;
mod board;
mod corpus;
mod discovery;
mod filters;
#[cfg(test)]
mod testutil;
mod tree;
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
