//! `gate` — the CDP gate harness CLI.
//!
//!   `gate v-suite <verify|accept> [--leptos-dir d] [--only slug] [--note why]`
//!   `gate s-routes`
//!   `gate serve --dir <dist> [--port 5198] [--api-proxy http://127.0.0.1:8080] [--map-assets dir]`
//!
//! Exit codes: 0 green · 1 gate fail · 2 usage · 3 driver error.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::browser_testing::diagnostics as doctor;
use crate::browser_testing::dom_oracle as vsuite;
use crate::browser_testing::editor_smoke_tests;
use crate::browser_testing::route_drift as sroutes;
use crate::browser_testing::server as serve;
use crate::repository_layout::MapAssetMounts;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gate", about = "CDP gate harness")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// V-suite frozen-oracle DOM gate
    #[command(name = "v-suite")]
    VSuite {
        /// verify | accept  (there is no freeze mode: the reference oracle is non-regenerable)
        mode: String,
        #[arg(long, default_value = "apps/website/frontend/dist")]
        leptos_dir: PathBuf,
        #[arg(long, default_value = "")]
        only: String,
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Route-table drift gate
    #[command(name = "s-routes")]
    SRoutes,
    /// One editor/live smoke by name (see EDITOR_SUITE)
    Smoke {
        /// selfcheck|arsenal|attributes|cur|doc|editor|fullmap|hillshade|hydrate|keyboard-settings|
        /// marquee-drag|outliner-palette|pan|persist|save-export|select|undo|mutations
        name: String,
        #[arg(long)]
        dist: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// The whole editor suite in EDITOR_SUITE order (first failure stops)
    #[command(name = "editor-suite")]
    EditorSuite {
        #[arg(long)]
        dist: Option<String>,
    },
    /// Fail-fast editor-gate preflight: pins + RAM/orphans + a ~15 s liveness probe.
    /// A prerequisite of `cargo xtask mk leptos-gates`; a wedge fails here with a diagnosis, not a 130 s hang.
    Doctor {
        #[arg(long)]
        dist: Option<String>,
        /// Promote pin/env drift warnings to failures (versions must match `gate-env.json`).
        #[arg(long)]
        strict: bool,
    },
    /// R-auth single-flight refresh gate (LEPTOS_DIST env respected)
    #[command(name = "r-auth")]
    RAuth {
        #[arg(long)]
        dist: Option<String>,
    },
    /// Generic SPA render liveness check
    #[command(name = "render-check")]
    RenderCheck {
        #[arg(long)]
        dir: String,
        #[arg(long, default_value = "/")]
        path: String,
        #[arg(long, default_value = "")]
        expect: String,
        #[arg(long)]
        assert_js: Option<String>,
        /// Inject the v-suite admin localStorage seed before boot (auth-gated pages).
        #[arg(long, default_value_t = false)]
        seed_auth: bool,
        #[arg(long, default_value_t = 5197)]
        port: u16,
        #[arg(long, default_value_t = 9337)]
        debug_port: u16,
        /// Proxy `/api` to a live backend. Default in `render_check` is
        /// `http://127.0.0.1:8080` when omitted — required for `--seed-auth` to fully hydrate.
        #[arg(long)]
        api_proxy: Option<String>,
        /// Write a full-viewport PNG of the page AFTER `--assert-js` settled (the
        /// live-check evidence rig: one URL + one probe script = one screenshot).
        #[arg(long)]
        shot: Option<PathBuf>,
        /// Serve `/map-assets/` from this terrain directory (the editor smokes' passthrough);
        /// without it the SPA fallback answers every asset fetch with index.html. The glyph atlas
        /// is taken from this directory's `glyphs` sibling, which is how the repository ships it.
        #[arg(long)]
        map_assets: Option<PathBuf>,
        /// A JS file evaluated on every new document BEFORE the SPA boots (peer of
        /// `--seed-auth` for a caller-built seed, e.g. a real dev-login session).
        #[arg(long)]
        inject_js: Option<PathBuf>,
        /// Skip the determinism freeze (fixed clock / seeded RNG) so a probe can
        /// measure real wall time (the frozen `performance.now` would collapse a budgeted rAF
        /// pass into one frame). Screenshots taken this way are NOT golden-comparable.
        #[arg(long, default_value_t = false)]
        no_freeze: bool,
    },
    /// Static SPA server with COOP/COEP
    Serve {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long, default_value_t = 5198)]
        port: u16,
        #[arg(long)]
        api_proxy: Option<String>,
        #[arg(long)]
        map_assets: Option<PathBuf>,
    },
}

pub fn run() -> ExitCode {
    // Pin the gate font cache in the single-threaded prologue, before any
    // tokio task exists. `cdp::launch` also sets `XDG_CACHE_HOME` on the chromium child
    // this covers doctor inherit-path probes (`check_fonts`) that deliberately do not
    // force their own env.
    doctor::ensure_gate_font_cache();
    let cli = Cli::parse();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result: anyhow::Result<u8> = rt.block_on(async {
        match cli.cmd {
            Cmd::VSuite {
                mode,
                leptos_dir,
                only,
                note,
            } => {
                vsuite::run(&vsuite::VSuiteArgs {
                    mode,
                    leptos_dir,
                    only,
                    note,
                })
                .await
            }
            Cmd::SRoutes => sroutes::run(),
            Cmd::Smoke { name, dist, path } => {
                editor_smoke_tests::run_smoke(&name, dist, path).await
            }
            Cmd::EditorSuite { dist } => editor_smoke_tests::editor_suite(dist).await,
            Cmd::Doctor { dist, strict } => doctor::run(dist, strict).await,
            Cmd::RAuth { dist } => editor_smoke_tests::r_auth(dist).await,
            Cmd::RenderCheck {
                dir,
                path,
                expect,
                assert_js,
                seed_auth,
                port,
                debug_port,
                api_proxy,
                shot,
                map_assets,
                inject_js,
                no_freeze,
            } => {
                editor_smoke_tests::render_check(&editor_smoke_tests::RenderCheckArgs {
                    dir,
                    path,
                    expect,
                    assert_js,
                    seed_auth,
                    port,
                    debug_port,
                    api_proxy,
                    shot,
                    map_assets,
                    inject_js,
                    no_freeze,
                })
                .await
            }
            Cmd::Serve {
                dir,
                port,
                api_proxy,
                map_assets,
            } => {
                let srv = serve::start_server(
                    serve::ServeConfig {
                        dir: dir.clone(),
                        api_proxy,
                        map_assets: map_assets.map(MapAssetMounts::beside_terrains),
                    },
                    port,
                )
                .await?;
                println!("serving {} on http://localhost:{}", dir.display(), srv.port);
                // Foreground until Ctrl-C (the Node CLI behaves the same).
                tokio::signal::ctrl_c().await.ok();
                srv.close().await;
                Ok(0)
            }
        }
    });
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("gate: driver error: {e:#}");
            ExitCode::from(3)
        }
    }
}
