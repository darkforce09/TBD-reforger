//! `capture` — the editor-capture harness CLI (T-661 port of `tools/editor-capture/`).
//!
//! Replaces `run_shot_gpu.sh` + `cdp2.mjs` (→ `shot`), `zoomsweep.mjs` (→ `zoomsweep`) and
//! `crop.sh` (→ `crop`). Drives the LIVE Mission Creator over CDP on ANGLE/Vulkan; the stack must be
//! up (`make db-up && make api && make leptos` / `leptos-debug`). See `tools/editor-capture/README.md`.
//!
//!   capture shot <out.png> <url> <waitMs> [url waitMs ...] [--canvas] [--hide-overlay]
//!   capture zoomsweep <out-prefix> <mission-id> <zoom,zoom,...>
//!   capture crop <img> <x> <y> <w> <h> [scale] [out]
//!
//! Exit codes: 0 = capture written · 1 = capture produced nothing · 2 = usage.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tbd_tools::capture::{self, ShotOptions, Step};

#[derive(Parser)]
#[command(name = "capture", about = "T-661 editor-capture harness (Rust)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// cdp2.mjs port: navigate the steps, poll the boot overlay out, capture chrome (+ map).
    ///
    /// ARGS are the cdp2.mjs positional pairs: `<out.png> <url> <waitMs> [url waitMs ...]`.
    Shot {
        /// `<out.png> <url> <waitMs> [url waitMs ...]` — first is the output PNG, then url/waitMs pairs.
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
        /// CANVAS_CAPTURE=1 — also write `<out>_canvas.png` via toDataURL (required to see the map).
        #[arg(long)]
        canvas: bool,
        /// FORCE_HIDE_OVERLAY=1 — remove the boot overlay from the DOM before capturing.
        #[arg(long = "hide-overlay")]
        hide_overlay: bool,
    },
    /// zoomsweep.mjs port: boot the editor, then per zoom set the camera and read the canvas.
    Zoomsweep {
        /// Output filename prefix (`<prefix>_z<z>.png` per zoom).
        out_prefix: String,
        /// Mission id to open at `/missions/<id>/edit`.
        mission_id: String,
        /// Comma-separated zoom levels, e.g. `-2,-1,0,1`.
        zooms: String,
    },
    /// crop.sh port: crop (and optionally nearest-neighbour upscale) a region of a screenshot.
    Crop {
        /// Source image.
        img: PathBuf,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        /// Integer upscale factor (nearest-neighbour). Default 1.
        #[arg(default_value_t = 1)]
        scale: u32,
        /// Output path. Default `<cwd>/crop.png`.
        #[arg(default_value = "crop.png")]
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result: anyhow::Result<u8> = match cli.cmd {
        Cmd::Shot {
            args,
            canvas,
            hide_overlay,
        } => {
            // cdp2.mjs: `out = a.shift(); for (i=0; i<a.length; i+=2) steps.push([a[i], Number(a[i+1]||3000)])`.
            let mut it = args.into_iter();
            let out = PathBuf::from(it.next().expect("clap required=true guarantees >=1 arg"));
            let rest: Vec<String> = it.collect();
            let mut steps = Vec::new();
            let mut i = 0;
            while i < rest.len() {
                let url = rest[i].clone();
                // Default wait 3000ms when the pair is missing its second element (cdp2.mjs `|| 3000`).
                let wait_ms = rest
                    .get(i + 1)
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(3000);
                steps.push(Step { url, wait_ms });
                i += 2;
            }
            let opts = ShotOptions {
                canvas_capture: canvas,
                force_hide_overlay: hide_overlay,
            };
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(capture::shot(&out, &steps, opts))
        }
        Cmd::Zoomsweep {
            out_prefix,
            mission_id,
            zooms,
        } => {
            let parsed: Vec<f64> = zooms
                .split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();
            if parsed.is_empty() {
                eprintln!("capture zoomsweep: no valid zoom values in '{zooms}'");
                return ExitCode::from(2);
            }
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(capture::zoomsweep(&out_prefix, &mission_id, &parsed))
        }
        Cmd::Crop {
            img,
            x,
            y,
            w,
            h,
            scale,
            out,
        } => capture::crop(&img, x, y, w, h, scale, &out),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("capture: {e:#}");
            ExitCode::from(1)
        }
    }
}
