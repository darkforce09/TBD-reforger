//! The `gate` command line: every headless browser gate of the single-page app behind one binary.
//!
//! ```text
//!   gate serve --dir <dist> [--port 5198] [--api-proxy http://127.0.0.1:8080] [--map-assets dir]
//!   gate mortar-offline [--dist d] [--port 5402] [--debug-port 9402] [--api-corpus dir]
//!   gate ballistics-agreement [--dist d] [--seed 1] [--count 32] [--catalog f] [--api-goldens d] [--timeout-s 600]
//! ```
//!
//! **Role:** parses the `gate` arguments with clap and runs the subcommand's gate of this crate.
//! **Position:** called by `tools/developer_tools/src/bin/gate.rs`; `cargo xtask mk leptos-gates`,
//! `gate-doctor`, `mortar-offline-gate` and `ballistics-wasm-agreement` run that binary.
//! **Signals & state:** one tokio runtime for the length of the run; the gate font cache is pinned
//! in the process environment before the runtime starts.
//! **Invariants:** exit codes are 0 green, 1 gate fail, 2 usage (clap's own exit, or a gate's
//! scenario error), 3 driver error, printed as `gate: driver error: ` and the error with every
//! cause.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::diagnostics as doctor;
use crate::editor_smoke_tests;
use crate::gate_layout::{FRONTEND_APPLICATION_DIST, MapAssetMounts};
use crate::server as serve;
use clap::{Parser, Subcommand};

use crate::{Error, Result};

#[derive(Parser)]
#[command(name = "gate", about = "CDP gate harness")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Inspect the real equipment viewer through the normally running website.
    EquipmentDataViewer {
        #[arg(long, default_value = "http://localhost:3000")]
        website: String,
        #[arg(
            long,
            default_value = "assets/scratch/equipment-data-viewer-verification"
        )]
        output: PathBuf,
    },
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
    /// The editor suite in EDITOR_SUITE order: selfcheck, editor, save-export, undo (first failure stops)
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
        /// Inject the admin localStorage seed before boot (auth-gated pages).
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
    /// The mortar calculator offline: pack download, service-worker reload, native solution
    #[command(name = "mortar-offline")]
    MortarOffline {
        #[arg(long, default_value = FRONTEND_APPLICATION_DIST)]
        dist: PathBuf,
        #[arg(long, default_value_t = 5402)]
        port: u16,
        #[arg(long, default_value_t = 9402)]
        debug_port: u16,
        /// The recorded API corpus served under `/api/`.
        #[arg(long, default_value = crate::mortar_offline::API_CORPUS_DIR)]
        api_corpus: PathBuf,
    },
    /// The fire-mission solver's wasm build against its native build, case by case
    #[command(name = "ballistics-agreement")]
    BallisticsAgreement {
        #[arg(long, default_value = FRONTEND_APPLICATION_DIST)]
        dist: PathBuf,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 32)]
        count: usize,
        #[arg(
            long,
            default_value = crate::ballistics_agreement::COMMITTED_CATALOG
        )]
        catalog: PathBuf,
        #[arg(
            long,
            default_value = crate::ballistics_agreement::API_GOLDENS
        )]
        api_goldens: PathBuf,
        #[arg(long, default_value_t = 5407)]
        port: u16,
        #[arg(long, default_value_t = 9407)]
        debug_port: u16,
        #[arg(long, default_value_t = 600)]
        timeout_s: u64,
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

/// Parses the process arguments, runs the subcommand and returns its exit code.
#[must_use]
pub fn run() -> ExitCode {
    // Pin the gate font cache in the single-threaded prologue, before any
    // tokio task exists. `cdp::launch` also sets `XDG_CACHE_HOME` on the chromium child
    // this covers doctor inherit-path probes (`check_fonts`) that deliberately do not
    // force their own env.
    doctor::ensure_gate_font_cache();
    let cli = Cli::parse();
    let result = tokio::runtime::Runtime::new()
        .map_err(Error::from)
        .and_then(|runtime| runtime.block_on(run_command(cli.cmd)));
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("gate: driver error: {}", error.with_causes());
            ExitCode::from(3)
        }
    }
}

/// Runs one subcommand and returns its exit code.
async fn run_command(command: Cmd) -> Result<u8> {
    match command {
        Cmd::EquipmentDataViewer { website, output } => {
            Ok(crate::equipment_data_viewer::run(&website, &output).await?)
        }
        Cmd::Smoke { name, dist, path } => {
            Ok(editor_smoke_tests::run_smoke(&name, dist, path).await?)
        }
        Cmd::EditorSuite { dist } => Ok(editor_smoke_tests::editor_suite(dist).await?),
        Cmd::Doctor { dist, strict } => Ok(doctor::run(dist, strict).await?),
        Cmd::RAuth { dist } => Ok(editor_smoke_tests::r_auth(dist).await?),
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
        } => Ok(
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
            .await?,
        ),
        Cmd::MortarOffline {
            dist,
            port,
            debug_port,
            api_corpus,
        } => {
            crate::mortar_offline::run(&crate::mortar_offline::MortarOfflineArgs {
                dist,
                port,
                debug_port,
                api_corpus,
            })
            .await
        }
        Cmd::BallisticsAgreement {
            dist,
            seed,
            count,
            catalog,
            api_goldens,
            port,
            debug_port,
            timeout_s,
        } => {
            crate::ballistics_agreement::run(&crate::ballistics_agreement::AgreementArgs {
                dist,
                seed,
                count,
                catalog,
                api_goldens,
                port,
                debug_port,
                timeout_s,
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
                    api_fixture_corpus: None,
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
}
