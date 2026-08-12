//! T-874 — port of `scripts/mod/test-phase1-api.sh`
//! → `cargo xtask mod test-phase1-api`.
//!
//! Path pins mirror `scripts/mod/lib/paths.sh` (do **not** delete paths.sh — T-879):
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`, `SCHEMA=packages/tbd-schema`, `WEB=apps/website/api`.
//! Bash sources paths.sh then `WEB="$WEB"` (no-op); `$WEB` is unused by the smoke itself.
//!
//! Smoke: curl Phase-1 game-server routes (link / roster / compiled mission) with
//! `API_BASE` (default `http://127.0.0.1:8080`), `GAME_SERVER_TOKEN`, `EVENT_ID`.
//!
//! Fail-opens closed vs bash: none — curl failures still hard-exit under former `set -e`
//! / `pipefail` (first failing curl's raw status). Tool-absent curl → 127.
//!
//! Preserved oddities:
//! - `curl -sS` + `tee /tmp/tbd-link.json` / `tee /tmp/tbd-roster.json` (body bytes to
//!   stdout exactly as curl emitted them, then a bare `echo` newline).
//! - Mission arm: `curl -sS -o /tmp/tbd-mission.json -w '%{http_code}'` then
//!   `head -c 120` + `echo "..."`.
//! - Auth always sends both Bearer + `Content-Type: application/json` (GETs too).

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use tbd_gate::proc::Run;
use tbd_gate::verdict::NotRun;

/// Entry for `xtask mod test-phase1-api`.
pub fn run(repo_root: &Path) -> Result<u8> {
    let _paths = Paths::from_root(repo_root);
    // Mirror bash `source …/paths.sh` + `WEB="$WEB"` — WEB unused by the curls.
    let _ = &_paths.web;

    let api = env::var("API_BASE").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let token = env::var("GAME_SERVER_TOKEN")
        .unwrap_or_else(|_| "dev-server-token-change-in-prod".to_string());
    let event_id =
        env::var("EVENT_ID").unwrap_or_else(|_| "b0000000-0000-4000-8000-000000000001".to_string());

    let auth_bearer = format!("Authorization: Bearer {token}");
    let auth_ct = "Content-Type: application/json";

    println!("== POST /api/link ==");
    let link_url = format!("{api}/api/link");
    let link_body = r#"{"code":"123456","identityId":"test-identity-abc","platform":"pc"}"#;
    match curl_stdout(&[
        "-sS",
        "-X",
        "POST",
        &link_url,
        "-H",
        &auth_bearer,
        "-H",
        auth_ct,
        "-d",
        link_body,
    ]) {
        CurlOut::Stdout(body) => {
            tee_stdout(&body, Path::new("/tmp/tbd-link.json"))?;
            println!(); // bare `echo` after tee
        }
        CurlOut::Code(code) => return Ok(code),
    }

    println!("== GET /api/game/events/{event_id}/roster (empty) ==");
    let roster_url = format!("{api}/api/game/events/{event_id}/roster");
    match curl_stdout(&["-sS", "-H", &auth_bearer, "-H", auth_ct, &roster_url]) {
        CurlOut::Stdout(body) => {
            tee_stdout(&body, Path::new("/tmp/tbd-roster.json"))?;
            println!();
        }
        CurlOut::Code(code) => return Ok(code),
    }

    println!("== GET /api/missions/msn_8f3a2c/compiled ==");
    let mission_url = format!("{api}/api/missions/msn_8f3a2c/compiled");
    let mission_path = Path::new("/tmp/tbd-mission.json");
    let code = match curl_stdout(&[
        "-sS",
        "-o",
        mission_path.to_str().unwrap_or("/tmp/tbd-mission.json"),
        "-w",
        "%{http_code}",
        "-H",
        &auth_bearer,
        "-H",
        auth_ct,
        &mission_url,
    ]) {
        CurlOut::Stdout(http_code) => http_code,
        CurlOut::Code(c) => return Ok(c),
    };
    println!("HTTP {code}");
    head_c_echo(mission_path, 120)?;

    println!();
    println!("Done. Link without login requires POST /api/me/link (needs Discord session).");
    Ok(0)
}

struct Paths {
    #[allow(dead_code)]
    mono_root: PathBuf,
    #[allow(dead_code)]
    mod_root: PathBuf,
    #[allow(dead_code)]
    schema: PathBuf,
    web: PathBuf,
}

impl Paths {
    /// Reproduce `scripts/mod/lib/paths.sh` against an already-resolved monorepo root.
    fn from_root(root: &Path) -> Self {
        Self {
            mono_root: root.to_path_buf(),
            mod_root: root.join("apps/mod"),
            schema: root.join("packages/tbd-schema"),
            web: root.join("apps/website/api"),
        }
    }
}

enum CurlOut {
    Stdout(String),
    /// Non-zero curl exit (or tool-absent → 127). Propagate like `set -e` / `pipefail`.
    Code(u8),
}

fn curl_stdout(args: &[&str]) -> CurlOut {
    let mut run = Run::new("curl");
    for a in args {
        run = run.arg(*a);
    }
    match run.output() {
        Ok(out) if out.code == 0 => {
            // curl -sS: errors on stderr; body / -w text on stdout.
            CurlOut::Stdout(out.stdout)
        }
        Ok(out) => {
            // Mirror bash: curl's stderr reaches the terminal; no post-curl `echo`.
            let _ = io::stderr().write_all(out.stderr.as_bytes());
            let _ = io::stderr().flush();
            CurlOut::Code(clamp_code(out.code))
        }
        Err(NotRun::ToolAbsent(_)) => CurlOut::Code(127),
        Err(_) => CurlOut::Code(1),
    }
}

fn tee_stdout(body: &str, path: &Path) -> Result<()> {
    fs::write(path, body.as_bytes())?;
    let mut out = io::stdout();
    out.write_all(body.as_bytes())?;
    out.flush()?;
    Ok(())
}

/// bash: `head -c N file; echo "..."`
fn head_c_echo(path: &Path, n: usize) -> Result<()> {
    let bytes = fs::read(path).unwrap_or_default();
    let take = n.min(bytes.len());
    let mut out = io::stdout();
    out.write_all(&bytes[..take])?;
    out.write_all(b"...\n")?;
    out.flush()?;
    Ok(())
}

fn clamp_code(code: i32) -> u8 {
    if (0..=255).contains(&code) {
        code as u8
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_mirror_paths_sh() {
        let root = Path::new("/repo");
        let p = Paths::from_root(root);
        assert_eq!(p.web, PathBuf::from("/repo/apps/website/api"));
        assert_eq!(p.mod_root, PathBuf::from("/repo/apps/mod"));
        assert_eq!(p.schema, PathBuf::from("/repo/packages/tbd-schema"));
    }

    #[test]
    fn clamp_code_passthrough() {
        assert_eq!(clamp_code(7), 7);
        assert_eq!(clamp_code(52), 52);
        assert_eq!(clamp_code(127), 127);
        assert_eq!(clamp_code(-1), 1);
    }
}
