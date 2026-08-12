//! T-861 — port of `scripts/mod/setup-server-profile.sh` → `cargo xtask setup server-profile`.
//!
//! Path pins mirror `scripts/mod/lib/paths.sh` (do **not** delete paths.sh — T-879):
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`, `SCHEMA=packages/tbd-schema`, `WEB=apps/website/api`.
//!
//! Builds a dedicated-server profile tree (`profile/TBD_BackendConfig.json`, mission fallback,
//! optional registry). Acceptance is bash/port stdout+stderr+rc (+ tree modes/bytes) on a clean
//! tree and ≥2 broken arms — not a green run alone (T-556 / T-853).
//!
//! Fail-opens closed vs bash:
//! - Missing `backend.example.json` / golden still hard-fail (bash `set -e` on `cp`, explicit
//!   golden check). Registry copy keeps bash's `cp … 2>/dev/null || true`.
//! - `.env` SERVICE_TOKEN parse matches `world-boot.sh` / former script (strip CR, one quote layer,
//!   `sed -n 's/^SERVICE_TOKEN=//p' | head -1`); absent line leaves the placeholder.
//!
//! Preserved oddities:
//! - Success banner still lists the Workbench checklist verbatim.
//! - Missing-backend stderr uses the GNU `cp: cannot stat '…': No such file or directory` shape
//!   so broken-arm diffs stay byte-aligned with bash on Linux.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::root::find_repo_root;

const MISSION_ID: &str = "msn_8f3a2c";
const PLACEHOLDER: &str = "replace-with-SERVICE_TOKEN-value";
const GOLDEN_REL: &str = "packages/tbd-schema/golden-missions/bridgehead-at-levie.json";
const BACKEND_EXAMPLE_REL: &str = "apps/mod/tbd-framework/Data/backend.example.json";
const REGISTRY_REL: &str = "apps/mod/tbd-framework/Data/registry.json";

/// Paths mirroring `scripts/mod/lib/paths.sh` for an already-resolved monorepo root.
struct Paths {
    mod_root: PathBuf,
    web: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mod_root: root.join("apps/mod"),
            web: root.join("apps/website/api"),
        }
    }
}

/// Entry for `xtask setup server-profile [PROFILE_DIR]`.
pub fn run(profile_arg: Option<&Path>) -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root, profile_arg)
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(root: &Path, profile_arg: Option<&Path>) -> Result<u8> {
    let paths = Paths::from_root(root);
    let profile = resolve_profile(profile_arg, &paths.mod_root);
    let profile_root = profile.join("profile");
    let missions = profile_root.join("missions");

    fs::create_dir_all(&missions).with_context(|| format!("mkdir -p {}", missions.display()))?;

    // 700 BEFORE anything is written — see former script comments (SERVICE_TOKEN file).
    set_mode(&profile_root, 0o700)?;

    let backend_src = root.join(BACKEND_EXAMPLE_REL);
    let backend_dst = profile_root.join("TBD_BackendConfig.json");
    if let Err(e) = fs::copy(&backend_src, &backend_dst) {
        if e.kind() == io::ErrorKind::NotFound {
            // GNU cp shape on Linux — byte-parity for the missing-backend broken arm.
            eprintln!(
                "cp: cannot stat '{}': No such file or directory",
                backend_src.display()
            );
            return Ok(1);
        }
        return Err(e)
            .with_context(|| format!("cp {} -> {}", backend_src.display(), backend_dst.display()));
    }
    set_mode(&backend_dst, 0o600)?;

    if let Some(token) = resolve_service_token(&paths.web) {
        substitute_token(&backend_dst, &token)?;
    }

    let golden = root.join(GOLDEN_REL);
    if !golden.is_file() {
        eprintln!("ERROR: golden mission not found: {}", golden.display());
        eprintln!(
            "       This script seeds the {MISSION_ID} disk fallback from that file. If the golden"
        );
        eprintln!("       was renamed, point GOLDEN at the one whose meta.id is {MISSION_ID}.");
        return Ok(1);
    }
    let mission_dst = missions.join(format!("{MISSION_ID}.json"));
    fs::copy(&golden, &mission_dst)
        .with_context(|| format!("cp {} -> {}", golden.display(), mission_dst.display()))?;

    // Optional registry override — bash `cp … 2>/dev/null || true`.
    let registry_src = root.join(REGISTRY_REL);
    let registry_dst = profile_root.join("TBD_Registry.json");
    let _ = fs::copy(&registry_src, &registry_dst);

    println!(
        "Profile ready at: {} (game data under {})",
        profile.display(),
        profile_root.display()
    );
    println!("  profile/TBD_BackendConfig.json");
    println!("  profile/missions/{MISSION_ID}.json");
    println!();
    println!("Workbench checklist:");
    println!("  1. Open tbd-framework/addon.gproj");
    println!("  2. Load mission Missions/TBD_Dev_POC.conf (or your scenario)");
    println!("  3. Add TBD_FrameworkManager + TBD_RegistryPocComponent to GameMode entity");
    println!(
        "  4. Host dedicated server with -profile pointing at: {}",
        profile.display()
    );

    Ok(0)
}

fn resolve_profile(arg: Option<&Path>, mod_root: &Path) -> PathBuf {
    if let Some(p) = arg {
        return p.to_path_buf();
    }
    if let Ok(p) = std::env::var("TBD_PROFILE") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    mod_root.join(".local-test-profile")
}

/// `SERVICE_TOKEN` env wins; else first `SERVICE_TOKEN=` line in `apps/website/api/.env`.
fn resolve_service_token(web: &Path) -> Option<String> {
    if let Ok(t) = std::env::var("SERVICE_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    let env_file = web.join(".env");
    if env_file.is_file() {
        token_from_env_file(&env_file)
    } else {
        None
    }
}

/// Character-for-character the reader in world-boot.sh / former setup-server-profile.sh.
fn token_from_env_file(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("SERVICE_TOKEN=") else {
            continue;
        };
        let mut v: String = rest.chars().filter(|c| *c != '\r').collect();
        // sed 's/^["'\'']//;s/["'\'']$//' — one surrounding quote layer.
        let bytes = v.as_bytes();
        if bytes.first().is_some_and(|c| *c == b'"' || *c == b'\'') {
            v.remove(0);
        }
        if v.as_bytes()
            .last()
            .is_some_and(|c| *c == b'"' || *c == b'\'')
        {
            v.pop();
        }
        if v.is_empty() {
            return None;
        }
        return Some(v);
    }
    None
}

fn substitute_token(config_path: &Path, token: &str) -> Result<()> {
    let body = fs::read_to_string(config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    // Literal replace of the placeholder — equivalent to bash sed with escaped `&`/`|`/`\`
    // because we are not going through sed's replacement grammar.
    let new_body = body.replace(PLACEHOLDER, token);
    // Keep mode 600: rewrite in place without changing permissions.
    let mut f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(config_path)
        .with_context(|| format!("open {}", config_path.display()))?;
    f.write_all(new_body.as_bytes())
        .with_context(|| format!("write {}", config_path.display()))?;
    Ok(())
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    let mut perms = fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)
        .with_context(|| format!("chmod {mode:o} {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn token_from_env_strips_quotes_and_cr() {
        let dir = tempfile_dir("t861-env");
        let env = dir.join(".env");
        fs::write(&env, "SERVICE_TOKEN=\"abc/def\"\r\nOTHER=1\n").unwrap();
        assert_eq!(token_from_env_file(&env).as_deref(), Some("abc/def"));
    }

    #[test]
    fn token_from_env_first_line_wins() {
        let dir = tempfile_dir("t861-env2");
        let env = dir.join(".env");
        fs::write(&env, "SERVICE_TOKEN=first\nSERVICE_TOKEN=second\n").unwrap();
        assert_eq!(token_from_env_file(&env).as_deref(), Some("first"));
    }

    #[test]
    fn substitute_preserves_ampersand_and_pipe() {
        let dir = tempfile_dir("t861-sub");
        let cfg = dir.join("cfg.json");
        fs::write(&cfg, format!("{{\"serverToken\": \"{PLACEHOLDER}\"}}")).unwrap();
        substitute_token(&cfg, "tok&pipe|slash/x").unwrap();
        let body = fs::read_to_string(&cfg).unwrap();
        assert_eq!(body, "{\"serverToken\": \"tok&pipe|slash/x\"}");
    }

    #[test]
    fn missing_golden_exits_1_with_bash_stderr() {
        let root = throwaway_root("no-golden", true, false);
        let prof = root.join("out-profile");
        let code = run_with_root(&root, Some(&prof)).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn missing_backend_exits_1() {
        let root = throwaway_root("no-backend", false, true);
        let prof = root.join("out-profile");
        let code = run_with_root(&root, Some(&prof)).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn clean_tree_writes_modes_and_mission_id_name() {
        let root = throwaway_root("clean", true, true);
        let prof = root.join("out-profile");
        let code = run_with_root(&root, Some(&prof)).unwrap();
        assert_eq!(code, 0);
        let profile_root = prof.join("profile");
        assert_eq!(mode_of(&profile_root), 0o700);
        assert_eq!(mode_of(&profile_root.join("TBD_BackendConfig.json")), 0o600);
        assert!(
            profile_root
                .join(format!("missions/{MISSION_ID}.json"))
                .is_file()
        );
    }

    fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn tempfile_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("t861-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn throwaway_root(tag: &str, with_backend: bool, with_golden: bool) -> PathBuf {
        let root = tempfile_dir(tag);
        fs::create_dir_all(root.join(".ai/tickets")).unwrap();
        fs::write(root.join(".ai/tickets/registry.json"), "{}").unwrap();
        fs::create_dir_all(root.join("apps/mod/tbd-framework/Data")).unwrap();
        fs::create_dir_all(root.join("packages/tbd-schema/golden-missions")).unwrap();
        fs::create_dir_all(root.join("apps/website/api")).unwrap();
        if with_backend {
            fs::write(
                root.join(BACKEND_EXAMPLE_REL),
                format!(
                    "{{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"serverToken\": \"{PLACEHOLDER}\",\n  \"missionId\": \"{MISSION_ID}\",\n  \"eventId\": \"b0000000-0000-4000-8000-000000000001\"\n}}\n"
                ),
            )
            .unwrap();
        }
        if with_golden {
            fs::write(
                root.join(GOLDEN_REL),
                format!("{{\"meta\":{{\"id\":\"{MISSION_ID}\"}}}}\n"),
            )
            .unwrap();
        }
        fs::write(root.join(REGISTRY_REL), "{\"ok\":true}\n").unwrap();
        root
    }
}
