//! Fingerprints bind evidence to source and effective configuration without exposing secrets.

use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io::Read,
    path::{Component, Path},
};
use verification_core::proc::Run;

const INPUT_ROOTS: &[&str] = &[
    "apps/",
    "tools_v2/",
    "contracts_v2/",
    crate::core::repository_layout::documentation::API_READINESS_EVIDENCE_PREFIX,
    ".cargo/",
    ".github/",
];
const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "c", "h", "cpp", "wgsl", "glsl", "toml", "lock", "json", "jsonc", "jsonl", "yaml", "yml",
    "sql", "md", "mdc", "txt", "csv", "tsv", "html", "css", "scss", "js", "mjs", "ts", "tsx",
    "xml", "ron", "cfg", "conf", "ini", "env", "example", "layout", "gproj", "et", "ent", "ct",
    "mission", "rules", "service", "timer", "socket",
];
const ROOT_INPUTS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "AGENTS.md",
    "rustfmt.toml",
    ".rustfmt.toml",
    "clippy.toml",
    ".clippy.toml",
    ".editorconfig",
    ".gitignore",
];
const CONFIGURATION_FILES: &[&str] = &[
    ".env",
    "apps/website/.env",
    "apps/website/api_v2/.env",
    crate::core::repository_layout::DEPLOY_ENV,
];
const CONFIGURATION_ENVIRONMENT: &[&str] = &[
    "APP_ENV",
    "PORT",
    "FRONTEND_URL",
    "TRUSTED_PROXIES",
    "ALLOWED_ORIGINS",
    "SPA_DIST_DIR",
    "MAP_ASSETS_DIR",
    "GLYPH_ASSETS_DIR",
    "UPLOAD_DIR",
    "DATABASE_URL",
    "MISSION_VERSION_MAX_BODY_BYTES",
    "JWT_SECRET",
    "JWT_ACCESS_TTL_MIN",
    "DISCORD_CLIENT_ID",
    "DISCORD_CLIENT_SECRET",
    "DISCORD_REDIRECT_URL",
    "DISCORD_GUILD_ID",
    "DISCORD_BOT_TOKEN",
    "DISCORD_WEBHOOK_URL",
    "SERVICE_TOKEN",
    "TBD_DB_POOL_MAX_CONNECTIONS",
    "TBD_DB_POOL_IDLE_TIMEOUT_SECS",
    "TBD_DB_POOL_MAX_LIFETIME_SECS",
    "TBD_DB_POOL_ACQUIRE_TIMEOUT_SECS",
    "LEADERBOARD_REFRESH_INTERVAL_SECS",
    "SERVER_STATUS_PUBLISH_INTERVAL_SECS",
    "ROLE_RESYNC_INTERVAL_SECS",
    "TEST_DATABASE_URL",
    "TBD_API_VERIFICATION",
    "CI",
    "TBD_IT_BASE_DB",
    "TBD_DB_CONTAINER",
    "TBD_DB_USER",
    "TBD_CONTAINER_RUNTIME",
    "TBD_MK_WEB",
    "TBD_MK_TRACE",
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_ENCODED_RUSTDOCFLAGS",
    "RUSTUP_TOOLCHAIN",
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET",
    "CARGO_HOME",
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO",
    "PATH",
    "LD_LIBRARY_PATH",
    "RUST_TEST_THREADS",
    "TZ",
];

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn framed(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

/// Binary terrain, texture, model and media assets require separate fixture identities.
pub(super) fn source_input(path: &str) -> bool {
    if ROOT_INPUTS.contains(&path) {
        return true;
    }
    if !INPUT_ROOTS.iter().any(|prefix| path.starts_with(prefix)) {
        return false;
    }
    let path = Path::new(path);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    matches!(
        filename,
        "Dockerfile" | "Containerfile" | "Caddyfile" | ".gitignore" | ".editorconfig"
    ) || filename.starts_with("Dockerfile.")
        || filename.starts_with("Containerfile.")
        || filename.starts_with("Caddyfile.")
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension))
}

/// Reject symlinks at every input component; absent optional files remain distinguishable.
fn input_exists(root: &Path, relative: &str) -> Result<bool> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        let Component::Normal(component) = component else {
            anyhow::bail!("unsafe fingerprint input path");
        };
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "symlink fingerprint input: {relative}"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
    ensure!(
        path.is_file(),
        "fingerprint input is not a regular file: {relative}"
    );
    Ok(true)
}

fn hash_file(hash: &mut Sha256, path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let length = file.metadata()?.len();
    hash.update(length.to_le_bytes());
    let mut read_length = 0_u64;
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        read_length += count as u64;
        hash.update(&buffer[..count]);
    }
    ensure!(
        read_length == length,
        "fingerprint input length changed while reading"
    );
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct SourceInventory {
    paths: BTreeSet<String>,
    deleted: BTreeSet<String>,
}

fn git_source_paths(root: &Path, arguments: &[&str]) -> Result<BTreeSet<String>> {
    let output = Run::new("git")
        .args(arguments)
        .cwd(root)
        .output()
        .map_err(|error| anyhow::anyhow!("source inventory failed: {error:?}"))?;
    ensure!(output.code == 0, "git source inventory failed");
    Ok(output
        .stdout
        .split('\0')
        .filter(|path| !path.is_empty() && source_input(path))
        .map(str::to_owned)
        .collect())
}

fn source_inventory(root: &Path) -> Result<SourceInventory> {
    let paths = git_source_paths(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    ensure!(!paths.is_empty(), "empty source input inventory");
    let deleted = git_source_paths(root, &["ls-files", "-z", "--deleted"])?;
    ensure!(
        deleted.is_subset(&paths),
        "source inventory changed while discovering tracked deletions"
    );
    Ok(SourceInventory { paths, deleted })
}

/// Only Git-observed tracked deletions may lack bytes. Required implementation paths are
/// validated independently by the register; a tombstone never establishes their availability.
fn hash_source_inventory(root: &Path, inventory: &SourceInventory) -> Result<String> {
    let mut hash = Sha256::new();
    framed(&mut hash, b"api-readiness-source-v3");
    for path in &inventory.paths {
        let exists = input_exists(root, path)?;
        ensure!(
            exists != inventory.deleted.contains(path),
            "fingerprint source input presence changed after inventory: {path}"
        );
        framed(&mut hash, path.as_bytes());
        if exists {
            framed(&mut hash, b"present");
            hash_file(&mut hash, &root.join(path))?;
        } else {
            framed(&mut hash, b"deleted");
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn source(root: &Path) -> Result<String> {
    let inventory = source_inventory(root)?;
    let digest = hash_source_inventory(root, &inventory)?;
    ensure!(
        inventory == source_inventory(root)?,
        "source inventory changed while fingerprinting"
    );
    Ok(digest)
}

/// Property settings sort names and frame raw OS bytes, including unknown future options.
fn hash_property_environment(
    hash: &mut Sha256,
    environment: impl IntoIterator<Item = (OsString, OsString)>,
) {
    let property_environment: BTreeMap<_, _> = environment
        .into_iter()
        .filter(|(name, _)| name.as_encoded_bytes().starts_with(b"PROPTEST_"))
        .collect();
    framed(hash, b"property-environment");
    hash.update((property_environment.len() as u64).to_le_bytes());
    for (name, value) in property_environment {
        framed(hash, name.as_encoded_bytes());
        framed(hash, value.as_encoded_bytes());
    }
}

/// Hash raw configuration inputs and environment overrides conservatively: even a
/// changed overridden file invalidates evidence. Values never appear in diagnostics.
pub(super) fn configuration(root: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    framed(&mut hash, b"api-readiness-configuration-v2");
    for relative in CONFIGURATION_FILES {
        framed(&mut hash, relative.as_bytes());
        let exists = input_exists(root, relative)?;
        hash.update([u8::from(exists)]);
        if exists {
            hash_file(&mut hash, &root.join(relative))?;
        }
    }
    hash_property_environment(&mut hash, std::env::vars_os());
    for name in CONFIGURATION_ENVIRONMENT {
        framed(&mut hash, name.as_bytes());
        let value = std::env::var_os(name);
        hash.update([u8::from(value.is_some())]);
        if let Some(value) = value {
            framed(&mut hash, value.as_encoded_bytes());
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
#[path = "tests/property_fingerprint.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/source_fingerprint.rs"]
mod source_tests;
