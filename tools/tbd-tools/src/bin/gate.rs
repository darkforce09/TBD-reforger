//! T-165.5 — `gate`: the Rust CDP gate harness CLI (replaces the Node driver entrypoints).
//!
//!   gate v-suite <verify|accept> [--leptos-dir d] [--only slug] [--note why]
//!   gate s-routes
//!   gate serve --dir <dist> [--port 5198] [--api-proxy http://127.0.0.1:8080] [--map-assets dir]
//!
//! Exit codes mirror the Node harness: 0 green · 1 gate fail · 2 usage · 3 driver error.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tbd_tools::{doctor, serve, smokes, sroutes, vsuite};

#[derive(Parser)]
#[command(name = "gate", about = "T-159/T-165 CDP gate harness")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// V-suite frozen-oracle DOM gate (gate_v_suite.mjs port)
    #[command(name = "v-suite")]
    VSuite {
        /// verify | accept  (freeze retired at T-171 — the React oracle is non-regenerable)
        mode: String,
        #[arg(long, default_value = "apps/website/frontend/dist")]
        leptos_dir: PathBuf,
        #[arg(long, default_value = "")]
        only: String,
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Route-table drift gate (extract-leptos-routes.mjs port)
    #[command(name = "s-routes")]
    SRoutes,
    /// One editor/live smoke by name (smoke_*_editor.mjs ports; see EDITOR_SUITE)
    Smoke {
        /// selfcheck|arsenal|attributes|cur|doc|editor|fullmap|hillshade|hydrate|keyboard-settings|
        /// marquee-drag|outliner-palette|pan|persist|save-export|select|undo|mutations
        name: String,
        #[arg(long)]
        dist: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// All 17 editor smokes in the Makefile glob order (first failure stops)
    #[command(name = "editor-suite")]
    EditorSuite {
        #[arg(long)]
        dist: Option<String>,
    },
    /// T-177 — fail-fast editor-gate preflight: pins + RAM/orphans + a ~15 s liveness probe.
    /// A prerequisite of `make leptos-gates`; a wedge fails here with a diagnosis, not a 130 s hang.
    Doctor {
        #[arg(long)]
        dist: Option<String>,
        /// Promote pin/env drift warnings to failures (versions must match `gate-env.json`).
        #[arg(long)]
        strict: bool,
    },
    /// R-auth single-flight refresh gate (gate_r_auth.mjs port; LEPTOS_DIST env respected)
    #[command(name = "r-auth")]
    RAuth {
        #[arg(long)]
        dist: Option<String>,
    },
    /// Generic SPA render liveness check (render-check.mjs port)
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
    },
    /// Static SPA server with COOP/COEP (serve.mjs CLI port)
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

fn main() -> ExitCode {
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
            Cmd::Smoke { name, dist, path } => smokes::run_smoke(&name, dist, path).await,
            Cmd::EditorSuite { dist } => smokes::editor_suite(dist).await,
            Cmd::Doctor { dist, strict } => doctor::run(dist, strict).await,
            Cmd::RAuth { dist } => smokes::r_auth(dist).await,
            Cmd::RenderCheck {
                dir,
                path,
                expect,
                assert_js,
                seed_auth,
                port,
                debug_port,
            } => {
                smokes::render_check(&smokes::RenderCheckArgs {
                    dir,
                    path,
                    expect,
                    assert_js,
                    seed_auth,
                    port,
                    debug_port,
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
                        map_assets_dir: map_assets,
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
