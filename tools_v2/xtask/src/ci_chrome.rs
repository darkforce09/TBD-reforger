//! T-901 — install the pinned Chrome-for-Testing build used by editor-gates.yml.
//!
//! The workflow used to `set -euo pipefail`, `apt-get`, `curl | unzip`, and append `GITHUB_ENV`
//! in a multi-line `run:` block. That is exactly the logic this ticket pulls out of YAML: no
//! type checker sees it, and `|| true` was one edit away. One `cargo xtask ci ci-chrome` line
//! replaces it.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

const GATE_ENV: &str = "tools/tbd-tools/gate-env.json";
const ZIP_NAME: &str = "chrome-linux64.zip";
const REL_BIN: &str = "chrome-linux64/chrome";

const APT_PKGS: &[&str] = &[
    "libnss3",
    "libatk1.0-0",
    "libatk-bridge2.0-0",
    "libcups2",
    "libdrm2",
    "libxkbcommon0",
    "libxcomposite1",
    "libxdamage1",
    "libxfixes3",
    "libxrandr2",
    "libgbm1",
    "libasound2t64",
    "libpango-1.0-0",
    "libcairo2",
    "libgtk-3-0",
    "fonts-liberation",
    "fonts-noto-core",
    "fonts-noto-color-emoji",
];

#[derive(Deserialize)]
struct GateEnv {
    chromium: Chromium,
}

#[derive(Deserialize)]
struct Chromium {
    version: String,
}

pub fn run() -> i32 {
    match run_inner() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("ci-chrome: {e}");
            1
        }
    }
}

fn run_inner() -> Result<(), String> {
    let root = crate::root::find_repo_root().map_err(|e| e.to_string())?;
    let version = chrome_version(&root)?;
    let home = std::env::var("HOME").map_err(|_| "HOME is unset".to_string())?;
    let dest = PathBuf::from(&home).join("cft");
    fs::create_dir_all(&dest).map_err(io_err("mkdir cft"))?;

    apt_install()?;

    let url = format!(
        "https://storage.googleapis.com/chrome-for-testing-public/{version}/linux64/{ZIP_NAME}"
    );
    let zip = dest.join(ZIP_NAME);
    run_cmd(
        Command::new("curl").args(["-fsSL", &url, "-o"]).arg(&zip),
        "curl chrome zip",
    )?;
    run_cmd(
        Command::new("unzip")
            .args(["-qo"])
            .arg(&zip)
            .arg("-d")
            .arg(&dest),
        "unzip chrome",
    )?;

    let chrome = dest.join(REL_BIN);
    if !chrome.is_file() {
        return Err(format!("chrome binary missing at {}", chrome.display()));
    }
    let chrome_s = chrome.display().to_string();
    println!("CHROME_HEADLESS_SHELL={chrome_s}");
    write_github_env(&chrome_s)?;
    Ok(())
}

fn chrome_version(root: &Path) -> Result<String, String> {
    if let Ok(v) = std::env::var("CHROME_VERSION")
        && !v.is_empty()
    {
        return Ok(v);
    }
    let text = fs::read_to_string(root.join(GATE_ENV)).map_err(io_err(GATE_ENV))?;
    let env: GateEnv = serde_json::from_str(&text).map_err(|e| format!("{GATE_ENV}: {e}"))?;
    Ok(env.chromium.version)
}

fn apt_install() -> Result<(), String> {
    run_cmd(
        Command::new("sudo").args(["apt-get", "update", "-qq"]),
        "apt-get update",
    )?;
    let mut cmd = Command::new("sudo");
    cmd.args(["apt-get", "install", "-y", "--no-install-recommends"]);
    cmd.args(APT_PKGS);
    run_cmd(&mut cmd, "apt-get install chrome deps")
}

fn write_github_env(chrome: &str) -> Result<(), String> {
    let Ok(path) = std::env::var("GITHUB_ENV") else {
        return Ok(());
    };
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(io_err("GITHUB_ENV"))?;
    writeln!(f, "CHROME_HEADLESS_SHELL={chrome}").map_err(io_err("GITHUB_ENV write"))?;
    Ok(())
}

fn run_cmd(cmd: &mut Command, what: &str) -> Result<(), String> {
    let st = cmd.status().map_err(|e| format!("{what}: spawn: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("{what}: exited {}", st.code().unwrap_or(1)))
    }
}

fn io_err(ctx: &'static str) -> impl Fn(io::Error) -> String {
    move |e| format!("{ctx}: {e}")
}
