use super::*;

pub(super) fn resolve_service_token(root: &Path) -> Option<String> {
    if let Ok(t) = std::env::var("TBD_SERVICE_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    let main_root = git_main_root(root).unwrap_or_else(|| root.to_path_buf());
    for f in [
        root.join("apps/website/api_v2/.env"),
        main_root.join("apps/website/api_v2/.env"),
    ] {
        if let Some(tok) = token_from_env_file(&f) {
            return Some(tok);
        }
    }
    None
}

pub(super) fn token_from_env_file(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("SERVICE_TOKEN=") else {
            continue;
        };
        let mut tok = rest.trim_end_matches('\r').to_string();
        if (tok.starts_with('"') && tok.ends_with('"'))
            || (tok.starts_with('\'') && tok.ends_with('\''))
        {
            tok = tok[1..tok.len() - 1].to_string();
        }
        if !tok.is_empty() {
            return Some(tok);
        }
    }
    None
}

pub(super) fn git_main_root(root: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args([
            "-C",
            root.to_str()?,
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let common = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Path::new(&common).parent().map(|p| p.to_path_buf())
}

pub(super) fn dev_login_token(api_base: &str) -> Option<String> {
    let out = Command::new("curl")
        .args([
            "-sS",
            "-o",
            "/dev/null",
            "-D",
            "-",
            "-m",
            "10",
            &format!("{api_base}/api/v1/auth/dev-login?role=mission_maker"),
        ])
        .output()
        .ok()?;
    let headers = String::from_utf8_lossy(&out.stdout).replace('\r', "");
    Regex::new(r"[#&]access_token=([^&]*)")
        .ok()?
        .captures(&headers)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

pub(super) fn curl_http(args: &[&str], err_path: &Path) -> (i32, u16) {
    let mut cmd = Command::new("curl");
    cmd.args(args);
    if let Ok(f) = fs::File::create(err_path) {
        cmd.stderr(Stdio::from(f));
    }
    match cmd.output() {
        Ok(o) => {
            let code = String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse()
                .unwrap_or(0);
            (o.status.code().unwrap_or(1), code)
        }
        Err(_) => (127, 0),
    }
}

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
        let ok = Command::new("sh").args(["-c", &format!("command -v {b} >/dev/null 2>&1")])
            .status().map(|s| s.success()).unwrap_or(false);
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

pub(super) fn host_command(program: &str) -> Command {
    if in_container() {
        if let Some(b) = host_bridge() {
            let mut c = Command::new(b);
            c.arg(program);
            return c;
        }
    }
    Command::new(program)
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
