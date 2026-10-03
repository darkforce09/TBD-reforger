//! What a world boot reads from the machine.
//!
//! **Role:** reads the checkout's addon GUID and scenario id, detects a container and its host
//! bridge (`distrobox-host-exec`, then `host-spawn`), wraps a host command in that bridge, and
//! makes scratch folders and numeric settings from the environment.
//! **Position:** under [`crate::world_boot`]; the boot sequence and the compiled lane call it.
//! **Signals & state:** none; reads files, the environment and `PATH`.
//! **Invariants:** inside a container a host command always runs through the bridge; a container
//! without a bridge cannot reach the real machine and `require_host` says so; an unset or
//! unparsable numeric setting takes its default.

use super::*;

pub(super) fn read_addon_guid(gproj: &Path) -> Option<String> {
    let text = fs::read_to_string(gproj).ok()?;
    Regex::new(r#"(?m)^\s*GUID\s+"([0-9A-Fa-f]+)""#)
        .ok()?
        .captures(&text)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

pub(super) fn read_scenario_id(config: &Path) -> Option<String> {
    let text = fs::read_to_string(config).ok()?;
    let chunk = Regex::new(r#""scenarioId"[^,]*"#)
        .ok()?
        .find(&text)?
        .as_str();
    Regex::new(r#"\{[^}]+\}[^"]*"#)
        .ok()?
        .find(chunk)
        .map(|m| m.as_str().to_string())
}

#[rustfmt::skip]
pub(super) fn in_container() -> bool {
    Path::new("/run/.containerenv").is_file() || Path::new("/.dockerenv").is_file()
}

#[rustfmt::skip]
pub(super) fn host_bridge() -> Option<&'static str> {
    for b in ["distrobox-host-exec", "host-spawn"] {
        let ok = Run::new("sh").args(["-c", &format!("command -v {b} >/dev/null 2>&1")])
            .status().is_ok_and(|code| code == 0);
        if ok { return Some(b); }
    }
    None
}

#[rustfmt::skip]
pub(super) fn require_host() -> bool {
    if !in_container() { return true; }
    if host_bridge().is_none() {
        eprintln!("require_host: no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine.");
        return false;
    }
    true
}

pub(super) fn host_command(program: &str) -> Run {
    if in_container()
        && let Some(b) = host_bridge()
    {
        return Run::new(b).arg(program);
    }
    Run::new(program)
}

#[rustfmt::skip]
pub(super) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

#[rustfmt::skip]
pub(super) fn tempfile_dir(prefix: &str) -> Result<PathBuf> {
    let ns = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let path = PathBuf::from(std::env::var_os("TMPDIR").unwrap_or_else(|| "/tmp".into())).join(format!("{prefix}.{}.{}", std::process::id(), ns));
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[rustfmt::skip]
pub(super) fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}
