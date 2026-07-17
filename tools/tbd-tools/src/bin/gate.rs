//! T-165.5 — `gate`: the Rust CDP gate harness CLI (replaces the Node driver entrypoints).
//!
//!   gate v-suite <freeze|verify|accept> [--oracle-dir d] [--leptos-dir d] [--only slug] [--note why]
//!   gate s-routes
//!   gate serve --dir <dist> [--port 5198] [--api-proxy http://127.0.0.1:8080] [--map-assets dir]
//!
//! Exit codes mirror the Node harness: 0 green · 1 gate fail · 2 usage · 3 driver error.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tbd_tools::{serve, sroutes, vsuite};

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
        /// freeze | verify | accept
        mode: String,
        #[arg(long, default_value = "apps/website/frontend/dist")]
        oracle_dir: PathBuf,
        #[arg(long, default_value = "apps/website-leptos/dist")]
        leptos_dir: PathBuf,
        #[arg(long, default_value = "")]
        only: String,
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Route-table drift gate (extract-leptos-routes.mjs port)
    #[command(name = "s-routes")]
    SRoutes,
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
                oracle_dir,
                leptos_dir,
                only,
                note,
            } => {
                vsuite::run(&vsuite::VSuiteArgs {
                    mode,
                    oracle_dir,
                    leptos_dir,
                    only,
                    note,
                })
                .await
            }
            Cmd::SRoutes => sroutes::run(),
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
