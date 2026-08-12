//! T-863 — port of `scripts/mod/tbd-dev-bootstrap.sh` → `cargo xtask mod dev-bootstrap`.
//!
//! Path pins mirror `scripts/mod/lib/paths.sh` (do **not** delete paths.sh — T-879):
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`, `MOD_SCRIPTS=scripts/mod`, `WEB=apps/website/api`.
//!
//! Still shells out to `mcp-daemon.sh` (bash; later slice). MCP game root is in-process
//! (`gate_setup_mcp_game_root`, T-876).
//! `xtask mcp call` goes through `scripts/mod/lib/xtask-run.sh` like bash.
//!
//! Fail-opens closed or pinned:
//! - `steam -applaunch … 2>/dev/null || true` — preserved (launch attempt never fails the gate).
//! - `npm ci || echo warn` — preserved non-fatal offline path.
//! - `mcp-daemon.sh start || echo warn` — preserved.
//! - `mod_validate … || true` — preserved (validate soft).
//! - `podman start … || true` / `setup server-profile … || true` on `--api`/`--server`.
//!
//! Preserved oddities:
//! - Hardcoded npx-cache `HANDLERS_SRC` under `/home/Samuel/.npm/_npx/…` (former script).
//! - ACTION REQUIRED re-run line still names `bash scripts/mod/tbd-dev-bootstrap.sh`
//!   (historical `$0` parity; docs/callers use `cargo xtask mod dev-bootstrap`).
//! - `port_open`: `ss` then `netstat` fallback, each with bash's `2>/dev/null` collapse.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use tbd_gate::proc::Run;

use crate::root::find_repo_root;

/// Historical bash re-run string (byte parity with former script line 53).
const RERUN_HISTORICAL: &str = "bash scripts/mod/tbd-dev-bootstrap.sh";

/// Hardcoded enfusion-mcp handlers source from the former script (npx cache path).
const HANDLERS_SRC: &str = "/home/Samuel/.npm/_npx/be402e1c82700767/node_modules/enfusion-mcp/mod/Scripts/WorkbenchGame/EnfusionMCP";

struct Paths {
    mono_root: PathBuf,
    mod_root: PathBuf,
    mod_scripts: PathBuf,
    web: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mono_root: root.to_path_buf(),
            mod_root: root.join("apps/mod"),
            mod_scripts: root.join("scripts/mod"),
            web: root.join("apps/website/api"),
        }
    }
}

/// Entry for `xtask mod dev-bootstrap [--api] [--server]`.
pub fn run(args: &[String]) -> Result<u8> {
    // TBD_DEV_BOOTSTRAP_ROOT: throwaway fixture roots for T-853 bash-vs-port arms.
    let root = match std::env::var_os("TBD_DEV_BOOTSTRAP_ROOT") {
        Some(p) => PathBuf::from(p),
        None => find_repo_root()?,
    };
    run_with_root(&root, args)
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(root: &Path, args: &[String]) -> Result<u8> {
    // Bash `cd … && pwd` is logical (-L): on ostree hosts getcwd is `/var/home/…` while
    // `$PWD` / bash pwd stay `/home/…`. Prefer the `/home` form so gproj paths match bash.
    let root = bash_logical_path(root);
    let p = Paths::from_root(&root);
    let mod_dir = p.mod_root.join("tbd-framework");
    let gproj = mod_dir.join("addon.gproj");
    let handlers_dst = mod_dir.join("Scripts/WorkbenchGame/EnfusionMCP");

    let wb_port = std::env::var("ENFUSION_WORKBENCH_PORT").unwrap_or_else(|_| "5775".into());
    let wait_sec: u64 = std::env::var("TBD_WB_WAIT_SEC")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);

    apply_default_env();

    out_line("== TBD dev bootstrap ==")?;

    // Former bash: `bash "$MOD_SCRIPTS/setup-mcp-game-root.sh"` under set -e.
    // T-876: in-process `cargo xtask setup mcp-game-root` (same defaults).
    match crate::gate_setup_mcp_game_root::run(None, None) {
        Ok(0) => {}
        Ok(code) => return Ok(code),
        Err(e) => return Err(e),
    }

    // Pin enfusion-mcp for the warm MCP daemon (non-fatal offline).
    let pkg = p.mod_scripts.join("package.json");
    let nm = p.mod_scripts.join("node_modules/enfusion-mcp");
    if pkg.is_file() && !nm.is_dir() {
        match Run::new("npm")
            .arg("ci")
            .arg("--silent")
            .cwd(&p.mod_scripts)
            .merged_output()
        {
            Ok(m) if m.code == 0 => {}
            _ => {
                out_line(&format!(
                    "warn: npm ci in {} failed (offline?) — using npx-cache fallback",
                    p.mod_scripts.display()
                ))?;
            }
        }
    }

    let handlers_src = Path::new(HANDLERS_SRC);
    if handlers_src.is_dir() && !handlers_dst.is_dir() {
        if let Some(parent) = handlers_dst.parent() {
            fs::create_dir_all(parent).with_context(|| format!("mkdir -p {}", parent.display()))?;
        }
        // bash: `cp -a "$HANDLERS_SRC" "$HANDLERS_DST"`
        match Run::new("cp")
            .arg("-a")
            .arg(handlers_src)
            .arg(&handlers_dst)
            .merged_output()
        {
            Ok(m) if m.code == 0 => {
                out_line(&format!(
                    "Installed EMCP handlers to {}",
                    handlers_dst.display()
                ))?;
            }
            Ok(m) => {
                eprint!("{}", m.text);
                return Ok(code_u8(m.code));
            }
            Err(_) => return Ok(1),
        }
    }

    if !port_open(&wb_port) {
        out_line(&format!(
            "Workbench Net API not on :{wb_port} — trying steam -applaunch 1874910 ..."
        ))?;
        // Preserved fail-open: `steam -applaunch 1874910 2>/dev/null || true`
        let _ = Run::new("steam")
            .arg("-applaunch")
            .arg("1874910")
            .merged_output();
        let mut elapsed: u64 = 0;
        while !port_open(&wb_port) && elapsed < wait_sec {
            thread::sleep(Duration::from_secs(3));
            elapsed = elapsed.saturating_add(3);
        }
    }

    if !port_open(&wb_port) {
        out_line("")?;
        out_line(&format!(
            "ACTION REQUIRED: Launch Arma Reforger Tools from Steam, open {}, enable Net API (File > Options > General).",
            gproj.display()
        ))?;
        out_line(&format!("Then re-run: {RERUN_HISTORICAL}"))?;
        return Ok(1);
    }

    out_line(&format!("Port {wb_port} is listening."))?;

    out_line("Pre-warming MCP daemon...")?;
    let daemon = p.mod_scripts.join("mcp-daemon.sh");
    match Run::new("bash").arg(&daemon).arg("start").merged_output() {
        Ok(m) => {
            print!("{}", m.text);
            let _ = io::stdout().flush();
            if m.code != 0 {
                out_line(
                    "warn: daemon pre-warm failed — xtask mcp call will use one-shot fallback",
                )?;
            }
        }
        Err(_) => {
            out_line("warn: daemon pre-warm failed — xtask mcp call will use one-shot fallback")?;
        }
    }

    let xtask_run = p.mod_scripts.join("lib/xtask-run.sh");
    match Run::new(&xtask_run)
        .arg("mcp")
        .arg("call")
        .arg("wb_connect")
        .arg("{}")
        .merged_output()
    {
        Ok(m) => {
            print!("{}", m.text);
            let _ = io::stdout().flush();
            if m.code != 0 {
                out_line(
                    "wb_connect failed — reload tbd-framework addon in Workbench Resource Browser and retry.",
                )?;
                return Ok(1);
            }
        }
        Err(_) => {
            out_line(
                "wb_connect failed — reload tbd-framework addon in Workbench Resource Browser and retry.",
            )?;
            return Ok(1);
        }
    }

    // Preserved fail-open: mod_validate || true
    let mod_json = format!("{{\"modPath\":\"{}\"}}", mod_dir.display());
    if let Ok(m) = Run::new(&xtask_run)
        .arg("mcp")
        .arg("call")
        .arg("mod_validate")
        .arg(&mod_json)
        .merged_output()
    {
        print!("{}", m.text);
        let _ = io::stdout().flush();
    }

    for arg in args {
        match arg.as_str() {
            "--api" => {
                let _ = Run::new("podman")
                    .arg("start")
                    .arg("tbdevent-postgres")
                    .merged_output();
                // bash: `(cd "$WEB" && npm run dev) &`
                let _ = Command::new("npm")
                    .arg("run")
                    .arg("dev")
                    .current_dir(&p.web)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                out_line("API dev server starting on :8080")?;
            }
            "--server" => {
                // bash: `(cd "$MONO_ROOT" && cargo run -q -p xtask -- setup server-profile) 2>/dev/null || true`
                let _ = Run::new("cargo")
                    .arg("run")
                    .arg("-q")
                    .arg("-p")
                    .arg("xtask")
                    .arg("--")
                    .arg("setup")
                    .arg("server-profile")
                    .cwd(&p.mono_root)
                    .merged_output();
                // T-871: run-dev-server.sh → `cargo xtask mod dev-server` (still no args —
                // same as the former bash spawn; the shim exits 2 with usage).
                let _ = Command::new("cargo")
                    .args(["run", "-q", "-p", "xtask", "--", "mod", "dev-server"])
                    .current_dir(&p.mono_root)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                out_line("Dedicated server starting...")?;
            }
            _ => {}
        }
    }

    out_line("Bootstrap complete.")?;
    Ok(0)
}

fn bash_logical_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("/var/home/") {
        let alt = PathBuf::from(format!("/home/{rest}"));
        if alt.exists() {
            return alt;
        }
    }
    path.to_path_buf()
}

fn apply_default_env() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/Samuel".into());
    set_default(
        "ENFUSION_GAME_PATH",
        &format!("{home}/.cache/enfusion-mcp-root"),
    );
    set_default(
        "ENFUSION_WORKBENCH_PATH",
        &format!("{home}/.local/share/Steam/steamapps/common/Arma Reforger Tools"),
    );
    set_default(
        "ENFUSION_PROJECT_PATH",
        &format!("{home}/Documents/Games/ArmaReforgerWorkbench/addons"),
    );
}

fn set_default(key: &str, val: &str) {
    if std::env::var_os(key).is_none() {
        // Intentionally mutates process env so child helpers (mcp daemon / xtask-run) see the
        // same defaults the former script `export`ed.
        unsafe { std::env::set_var(key, val) };
    }
}

/// bash: `ss -tln 2>/dev/null | grep -q ":${WB_PORT} " || netstat -tln 2>/dev/null | grep -q …`
fn port_open(port: &str) -> bool {
    let needle = format!(":{port} ");
    if let Ok(o) = Run::new("ss").arg("-tln").output() {
        if o.code == 0 && o.stdout.lines().any(|l| l.contains(&needle)) {
            return true;
        }
    }
    if let Ok(o) = Run::new("netstat").arg("-tln").output() {
        if o.code == 0 && o.stdout.lines().any(|l| l.contains(&needle)) {
            return true;
        }
    }
    false
}

fn code_u8(code: i32) -> u8 {
    if (0..=255).contains(&code) {
        code as u8
    } else {
        1
    }
}

fn out_line(s: &str) -> Result<()> {
    println!("{s}");
    let _ = io::stdout().flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // T-876: setup-mcp-game-root is in-process (no shell script to stub). Failure arms for
    // the port live in `gate_setup_mcp_game_root` tests. Bootstrap still covers port_open.

    #[test]
    fn port_open_rejects_non_numeric_needle() {
        // Needle ":not-a-port " cannot appear in ss/netstat listen tables.
        assert!(!port_open("not-a-port"));
    }
}
