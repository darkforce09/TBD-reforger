//! `cargo xtask setup server-profile`: write the dedicated-server profile this checkout boots with.
//!
//! Path pins:
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`, `WEB=apps/website/api_v2`.
//!
//! Builds a dedicated-server profile tree: `profile/TBD_BackendConfig.json` from the committed
//! example, with the service token and the game runtime's machine credential filled in when they
//! are known, and the optional registry. The mission a server runs is not part of the profile:
//! the mod reads the server's deployment with its machine credential (or boots the last verified
//! artifact it cached), so a profile without a credential boots no mission.
//!
//! - `SERVICE_TOKEN` (environment, else the first `SERVICE_TOKEN=` line of
//!   `apps/website/api_v2/.env`) replaces the example's `serverToken` placeholder.
//! - `TBD_MACHINE_CREDENTIAL` (environment) becomes `machineCredential`; without it the example's
//!   placeholder stays, which the mod reports as unset.
//! - A missing `backend.example.json` fails with the GNU `cp: cannot stat` shape; the registry
//!   copy is best-effort.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::repository_root::find_repo_root;

const PLACEHOLDER: &str = "replace-with-SERVICE_TOKEN-value";
/// Where the game runtime's machine credential comes from.
const MACHINE_CREDENTIAL_VARIABLE: &str = "TBD_MACHINE_CREDENTIAL";
const BACKEND_EXAMPLE_REL: &str = "apps/mod/tbd-framework/Data/backend.example.json";
const REGISTRY_REL: &str = "apps/mod/tbd-framework/Data/registry.json";

/// The path pins, for an already-resolved monorepo root.
struct Paths {
    mod_root: PathBuf,
    web: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mod_root: root.join("apps/mod"),
            web: root.join("apps/website/api_v2"),
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

    fs::create_dir_all(&profile_root)
        .with_context(|| format!("mkdir -p {}", profile_root.display()))?;

    // 700 before anything is written: the backend config carries the service token and the
    // machine credential.
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

    let credential = std::env::var(MACHINE_CREDENTIAL_VARIABLE).unwrap_or_default();
    if !credential.is_empty() {
        set_machine_credential(&backend_dst, &credential)?;
    }

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
    if credential.is_empty() {
        println!(
            "  machineCredential: not set — set {MACHINE_CREDENTIAL_VARIABLE} to a mod_runtime credential of"
        );
        println!(
            "  this server; until then it boots no mission (or the last verified artifact it cached)"
        );
    } else {
        println!("  machineCredential: set from {MACHINE_CREDENTIAL_VARIABLE}");
    }
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
    if let Ok(p) = std::env::var("TBD_PROFILE")
        && !p.is_empty()
    {
        return PathBuf::from(p);
    }
    mod_root.join(".local-test-profile")
}

/// `SERVICE_TOKEN` env wins; else first `SERVICE_TOKEN=` line in `apps/website/api_v2/.env`.
fn resolve_service_token(web: &Path) -> Option<String> {
    if let Ok(t) = std::env::var("SERVICE_TOKEN")
        && !t.is_empty()
    {
        return Some(t);
    }
    let env_file = web.join(".env");
    if env_file.is_file() {
        token_from_env_file(&env_file)
    } else {
        None
    }
}

/// Character-for-character the reader the world-boot gate uses.
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

/// Write `machineCredential` into the backend config, keeping its other keys and their order.
fn set_machine_credential(config_path: &Path, credential: &str) -> Result<()> {
    let body = fs::read_to_string(config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let mut config: serde_json::Value =
        serde_json::from_str(&body).with_context(|| format!("parse {}", config_path.display()))?;
    let object = config
        .as_object_mut()
        .with_context(|| format!("{} is not a JSON object", config_path.display()))?;
    object.insert(
        "machineCredential".to_string(),
        serde_json::Value::String(credential.to_string()),
    );
    let mut f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(config_path)
        .with_context(|| format!("open {}", config_path.display()))?;
    f.write_all(format!("{}\n", serde_json::to_string_pretty(&config)?).as_bytes())
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
#[path = "tests/server_profile/tests.rs"]
mod tests;
