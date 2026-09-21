//! `cargo xtask mod dev-bootstrap` — bring a workstation to the state mod work needs.
//!
//! It installs the pinned `enfusion-mcp` package, warms the MCP daemon, sets up the MCP game
//! root, launches Workbench with `-gproj apps/mod/tbd-export/addon.gproj` (which skips the
//! project picker and loads tbd-framework and tbd-emcp through the dependency), and optionally
//! starts the API database and the dedicated-server profile.
//!
//! The enfusion-mcp Workbench handlers are committed in `apps/mod/tbd-emcp`, so nothing is ever
//! copied into an addon: a second handler set beside tbd-emcp's would shadow it and break the
//! bridge. A checkout missing those handlers stops the command.
//!
//! Every external step is non-fatal, because this command prepares a machine rather than
//! verifying one: a failed `npm ci` falls back to whatever npm has already downloaded, and a
//! Steam launch, a daemon start, a mod validate, a database start and a server-profile setup
//! each report and continue. The one thing it refuses to do is claim success while Workbench's
//! Net API is unreachable: that prints what to do by hand and exits 1.
//!
//! `port_open` asks `ss` and falls back to `netstat`; neither being installed reads as closed.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use verification_core::proc::Run;

use crate::core::repository_root::find_repo_root;

/// What to run again once the operator has done the manual step this command cannot do.
const RERUN_COMMAND: &str = "cargo xtask mod dev-bootstrap";

struct Paths {
    mono_root: PathBuf,
    mod_root: PathBuf,
    enfusion_mcp_node_package: PathBuf,
    web: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mono_root: root.to_path_buf(),
            mod_root: root.join("apps/mod"),
            enfusion_mcp_node_package:
                developer_tools::repository_layout::enfusion_mcp_node_package_dir(root),
            web: root.join("apps/website/api_v2"),
        }
    }
}

/// Entry for `xtask mod dev-bootstrap [--api] [--server]`.
pub fn run(args: &[String]) -> Result<u8> {
    // TBD_DEV_BOOTSTRAP_ROOT lets a test point this command at a throwaway tree.
    let root = match std::env::var_os("TBD_DEV_BOOTSTRAP_ROOT") {
        Some(p) => PathBuf::from(p),
        None => find_repo_root()?,
    };
    run_with_root(&root, args)
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(root: &Path, args: &[String]) -> Result<u8> {
    // On an ostree host `getcwd` answers `/var/home/…` while the login shell's own view of the
    // same directory is `/home/…`. Workbench is handed a `-gproj` path, and the Proton prefix
    // maps the shell's view, so the `/home` form is the one it can open.
    let root = symlinked_home_path(root);
    let p = Paths::from_root(&root);
    let mod_dir = p.mod_root.join("tbd-framework");
    let export_dir = p.mod_root.join("tbd-export");
    // The session project: tbd-export depends on tbd-framework AND tbd-emcp, so opening it loads
    // all three and the Net API handlers with them.
    let gproj = export_dir.join("addon.gproj");
    let emcp_ping = p
        .mod_root
        .join("tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_Ping.c");

    let wb_port = std::env::var("ENFUSION_WORKBENCH_PORT").unwrap_or_else(|_| "5775".into());
    let wait_sec: u64 = std::env::var("TBD_WB_WAIT_SEC")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);

    apply_default_env();

    out_line("== TBD dev bootstrap ==")?;

    // The same work `cargo xtask setup mcp-game-root` does, with the same defaults.
    match crate::commands::setup::mcp_game_root::run(None, None) {
        Ok(0) => {}
        Ok(code) => return Ok(code),
        Err(e) => return Err(e),
    }

    // Install the pinned enfusion-mcp package so the daemon starts from disk. Non-fatal: the
    // entrypoint resolver falls back to npm's download cache and then to a download.
    let pkg = p.enfusion_mcp_node_package.join("package.json");
    let installed = developer_tools::repository_layout::enfusion_mcp_entrypoint(&p.mono_root);
    if pkg.is_file() && !installed.is_file() {
        match Run::new("npm")
            .arg("ci")
            .arg("--silent")
            .cwd(&p.enfusion_mcp_node_package)
            .merged_output()
        {
            Ok(m) if m.code == 0 => {}
            _ => {
                out_line(&format!(
                    "warn: npm ci in {} failed (offline?) — falling back to npm's download cache",
                    p.enfusion_mcp_node_package.display()
                ))?;
            }
        }
    }

    if !emcp_ping.is_file() {
        out_line(&format!(
            "checkout incomplete: {} missing — the enfusion-mcp handlers live in apps/mod/tbd-emcp and are never copied into another addon",
            emcp_ping.display()
        ))?;
        return Ok(1);
    }

    if !port_open(&wb_port) {
        out_line(&format!(
            "Workbench Net API not on :{wb_port} — trying steam -applaunch 1874910 ..."
        ))?;
        // A failed launch is reported by the port poll below, not here. `-gproj` skips the
        // project picker (a picker-stuck launch never opens the Net API) and opens tbd-export,
        // which pulls in tbd-framework and tbd-emcp. Proton maps `/` to `Z:`.
        let _ = Run::new("steam")
            .arg("-applaunch")
            .arg("1874910")
            .arg("-gproj")
            .arg(format!("Z:{}", gproj.display()))
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
        out_line(&format!("Then re-run: {RERUN_COMMAND}"))?;
        return Ok(1);
    }

    out_line(&format!("Port {wb_port} is listening."))?;

    out_line("Pre-warming MCP daemon...")?;
    // Non-quiet, so the daemon's own start messages stream to this command's stdout.
    let code = crate::commands::mcp::daemon::start_at(
        &crate::commands::mcp::daemon::resolve_sock(),
        false,
    );
    if code != 0 {
        out_line("warn: daemon pre-warm failed — xtask mcp call will use one-shot fallback")?;
    }

    // Former lib/xtask-run.sh → cargo run -q -p xtask -- (mono root).
    match Run::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("mcp")
        .arg("call")
        .arg("wb_connect")
        .arg("{}")
        .cwd(&p.mono_root)
        .merged_output()
    {
        Ok(m) => {
            print!("{}", m.text);
            let _ = io::stdout().flush();
            if m.code != 0 {
                out_line(
                    "wb_connect failed — Workbench must have apps/mod/tbd-export/addon.gproj open (it loads tbd-emcp, which carries the Net API handlers); open it and retry.",
                )?;
                return Ok(1);
            }
        }
        Err(_) => {
            out_line(
                "wb_connect failed — Workbench must have apps/mod/tbd-export/addon.gproj open (it loads tbd-emcp, which carries the Net API handlers); open it and retry.",
            )?;
            return Ok(1);
        }
    }

    // Preserved fail-open: mod_validate || true — the shipping mod and the export tooling addon.
    for dir in [&mod_dir, &export_dir] {
        let mod_json = format!("{{\"modPath\":\"{}\"}}", dir.display());
        if let Ok(m) = Run::new("cargo")
            .arg("run")
            .arg("-q")
            .arg("-p")
            .arg("xtask")
            .arg("--")
            .arg("mcp")
            .arg("call")
            .arg("mod_validate")
            .arg(&mod_json)
            .cwd(&p.mono_root)
            .merged_output()
        {
            print!("{}", m.text);
            let _ = io::stdout().flush();
        }
    }

    for arg in args {
        match arg.as_str() {
            "--api" => {
                let _ = Run::new("podman")
                    .arg("start")
                    .arg("tbdevent-postgres")
                    .merged_output();
                // Detached: the dev server runs until the operator stops it.
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
                // Optional: a host without a server directory is a valid workstation.
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
                // No arguments: `mod dev-server` with none prints its usage and exits 2,
                // which is the intended outcome of this optional step.
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

/// The `/home/…` spelling of a path an ostree host reports as `/var/home/…`.
fn symlinked_home_path(path: &Path) -> PathBuf {
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
        // Intentionally mutates process env so child helpers (mcp daemon / cargo xtask) see the
        // same defaults the former script `export`ed.
        unsafe { std::env::set_var(key, val) };
    }
}

/// Whether anything is listening on `port`: `ss` first, `netstat` as the fallback. Neither
/// being installed reads as closed, which is the safe answer for a port poll.
fn port_open(port: &str) -> bool {
    let needle = format!(":{port} ");
    if let Ok(o) = Run::new("ss").arg("-tln").output()
        && o.code == 0
        && o.stdout.lines().any(|l| l.contains(&needle))
    {
        return true;
    }
    if let Ok(o) = Run::new("netstat").arg("-tln").output()
        && o.code == 0
        && o.stdout.lines().any(|l| l.contains(&needle))
    {
        return true;
    }
    false
}

fn out_line(s: &str) -> Result<()> {
    println!("{s}");
    let _ = io::stdout().flush();
    Ok(())
}

#[cfg(test)]
#[path = "tests/development_bootstrap/tests.rs"]
mod tests;
