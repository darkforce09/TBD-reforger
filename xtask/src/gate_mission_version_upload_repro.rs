//! T-867 — port of `scripts/website/mission-version-upload-repro.sh`
//! → `cargo xtask repro mission-upload`.
//!
//! Orchestrates curl + the existing `repro mission-id` / `repro mission-version-body`
//! helpers (kept as separate subcommands; called in-process here so cold-cargo noise
//! cannot leak into the repro transcript).
//!
//! Prereqs (same as bash): live API on `$API` (default `http://localhost:8080/api/v1`)
//! with `APP_ENV=development` / dev-login. Offline / no-API: fails at the first curl
//! with curl's raw exit (typically 7). Happy-path wall-clock (`%{time_total}`) is
//! non-reproducible across runs — live recipe only; parity arms use throwaway stubs
//! that stop before the upload write-out, or normalise `time=…s`.
//!
//! Fail-opens closed vs bash: none — curl failures on the upload loop still soft-echo
//! (`curl exit N  (connection reset = server-side)`) exactly as the `|| echo` did.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use regex::Regex;
use tbd_gate::proc::Run;
use tbd_gate::verdict::NotRun;

use crate::repro;

/// Entry for `xtask repro mission-upload`.
pub fn run() -> Result<u8> {
    let api = env::var("API").unwrap_or_else(|_| "http://localhost:8080/api/v1".to_string());
    let role = env::var("ROLE").unwrap_or_else(|_| "mission_maker".to_string());
    let sizes_mb = env::var("SIZES_MB").unwrap_or_else(|_| "2 10 140".to_string());

    let tmp = make_tmp()?;
    let _guard = TmpGuard(tmp.clone());

    println!("==> dev-login ({role})");
    let login_url = format!("{api}/auth/dev-login?role={role}");
    let loc = match curl_output(&["-s", "-o", "/dev/null", "-w", "%{redirect_url}", &login_url]) {
        CurlOut::Code(code) => return Ok(code),
        CurlOut::Stdout(s) => s,
    };

    let token = extract_token(&loc);
    if token.is_empty() {
        println!("no token (is APP_ENV=development and the API up?)");
        return Ok(1);
    }

    println!("==> create mission");
    let create_body = r#"{"title":"version-upload-repro","terrain":"everon","game_mode":"pve_coop","max_players":8}"#;
    let create_url = format!("{api}/missions");
    let auth = format!("Authorization: Bearer {token}");
    let create_json = match curl_output(&[
        "-s",
        "-X",
        "POST",
        &create_url,
        "-H",
        &auth,
        "-H",
        "Content-Type: application/json",
        "-d",
        create_body,
    ]) {
        CurlOut::Code(code) => return Ok(code),
        CurlOut::Stdout(s) => s,
    };

    // bash: `… | cargo run -q -p xtask -- repro mission-id` → main prints `xtask: {e:#}`
    let mid = repro::mission_id_from_json(&create_json)?;
    println!("    mission={mid}");

    let mut i: u64 = 0;
    for mb_str in sizes_mb.split_whitespace() {
        i += 1;
        let mb: u64 = mb_str
            .parse()
            .with_context(|| format!("SIZES_MB entry not an integer: {mb_str}"))?;
        let f = tmp.join(format!("body_{mb}mb.json"));
        let semver = format!("1.{i}.0");
        repro::cmd_mission_version_body(&f, mb, &semver)?;
        // bash: printf '==> POST %4s MB  ' "$MB"
        print!("==> POST {mb_str:>4} MB  ");
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let resp = tmp.join("resp.txt");
        let version_url = format!("{api}/missions/{mid}/versions");
        let write_out = "HTTP %{http_code}  uploaded=%{size_upload}B  time=%{time_total}s\n";
        // Soft-fail like bash `curl … || echo "curl exit $?  (connection reset = server-side)"`
        match curl_upload(&version_url, &auth, &f, &resp, write_out) {
            Ok(text) => print!("{text}"),
            Err(code) => {
                println!("curl exit {code}  (connection reset = server-side)");
            }
        }
    }

    println!(
        "==> done. Watch the `make api` terminal for: CreateVersion: mission={mid} content_length=… status=…"
    );
    Ok(0)
}

enum CurlOut {
    Stdout(String),
    /// Non-zero curl exit (or tool-absent → 127). Propagate like `set -e` on the first curls.
    Code(u8),
}

fn curl_output(args: &[&str]) -> CurlOut {
    let mut run = Run::new("curl");
    for a in args {
        run = run.arg(*a);
    }
    match run.output() {
        Ok(out) if out.code == 0 => CurlOut::Stdout(out.stdout),
        Ok(out) => CurlOut::Code(clamp_code(out.code)),
        Err(NotRun::ToolAbsent(_)) => CurlOut::Code(127),
        Err(_) => CurlOut::Code(1),
    }
}

fn curl_upload(
    url: &str,
    auth: &str,
    body: &Path,
    resp: &Path,
    write_out: &str,
) -> Result<String, u8> {
    let data_arg = format!("@{}", body.display());
    let run = Run::new("curl")
        .arg("-s")
        .arg("-o")
        .arg(resp)
        .arg("-w")
        .arg(write_out)
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg(auth)
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("--data-binary")
        .arg(&data_arg);
    match run.output() {
        Ok(out) if out.code == 0 => Ok(out.stdout),
        Ok(out) => Err(clamp_code(out.code)),
        Err(NotRun::ToolAbsent(_)) => Err(127),
        Err(_) => Err(1),
    }
}

fn extract_token(loc: &str) -> String {
    // bash: sed -n 's/.*access_token=\([^&]*\).*/\1/p'
    let re = Regex::new(r".*access_token=([^&]*).*").expect("token regex");
    re.captures(loc)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_default()
}

fn clamp_code(code: i32) -> u8 {
    if (0..=255).contains(&code) {
        code as u8
    } else {
        1
    }
}

fn make_tmp() -> Result<PathBuf> {
    // bash: TMP="$(mktemp -d)"
    let base = env::temp_dir();
    let dir = base.join(format!(
        "t867-mission-upload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir(&dir).with_context(|| format!("mktemp -d → {}", dir.display()))?;
    Ok(dir)
}

struct TmpGuard(PathBuf);
impl Drop for TmpGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_token_matches_sed() {
        assert_eq!(
            extract_token("http://localhost:3000/auth/callback#access_token=abc123&x=1"),
            "abc123"
        );
        assert_eq!(extract_token("http://example/?nope=1"), "");
        assert_eq!(
            extract_token("https://x/#foo=1&access_token=tok%2Fval&refresh=r"),
            "tok%2Fval"
        );
    }
}
