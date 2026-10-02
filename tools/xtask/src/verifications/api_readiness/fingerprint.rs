//! Fingerprints bind evidence to one source tree and one effective configuration without
//! exposing secrets.
//!
//! **Role:** Computes the source digest (every Git-visible source input, tracked deletions and
//! tracked symlinks included) and the configuration digest (the `.env` files, `deploy.env`, every
//! `PROPTEST_*` variable and a fixed list of build and API variables) that every readiness receipt
//! records.
//!
//! **Position:** `verify` in the parent module calls [`source`] and [`configuration`] before
//! judging and again at the end; `--execute` writes both digests into each receipt and
//! `evidence.rs` compares them with the current tree.
//!
//! **Signals & state:** none; reads the Git index, the worktree and the process environment.
//!
//! **Invariants:** a symlink is hashed only when Git tracks it with mode 120000, as its link text
//! under a `symlink` tag that no regular file's `present` tag can collide with, and only when it
//! resolves to an existing entry inside the repository. Every other symlink on an input path (one
//! Git does not track as a symlink, a symlinked ancestor directory, a configuration file) fails the
//! run. Configuration values never appear in diagnostics.

use anyhow::{Result, ensure};
use content_digest::Sha256Hasher;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::Metadata,
    path::{Component, Path, PathBuf},
};
use verification_core::proc::Run;

/// The Git index mode of a symbolic link; its blob holds the link text.
const GIT_SYMLINK_MODE: &str = "120000";

const INPUT_ROOTS: &[&str] = &[
    "apps/",
    "tools/",
    "contracts/",
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
    "apps/api/.env",
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
    "OBSERVABILITY_TOKEN",
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

/// What a fingerprint input path holds in the worktree.
#[derive(Debug)]
enum InputEntry {
    /// No entry: a Git-observed tracked deletion, or an optional configuration file.
    Absent,
    /// A regular file, hashed by its bytes.
    RegularFile,
    /// A symlink Git tracks with mode 120000 that resolves inside the repository; holds the link
    /// text, the bytes of its Git blob.
    TrackedSymlink(PathBuf),
}

/// Walks `relative` below `root` without following links: an absent component yields `None`, a
/// symlinked ancestor directory fails, and the final entry's own metadata is returned.
fn final_entry(root: &Path, relative: &str) -> Result<Option<(PathBuf, Metadata)>> {
    let mut path = root.to_path_buf();
    let mut entry: Option<Metadata> = None;
    for component in Path::new(relative).components() {
        let Component::Normal(component) = component else {
            anyhow::bail!("unsafe fingerprint input path");
        };
        if let Some(ancestor) = &entry {
            ensure!(
                !ancestor.file_type().is_symlink(),
                "symlink fingerprint input directory: {relative}"
            );
        }
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => entry = Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        }
    }
    let entry = entry.ok_or_else(|| anyhow::anyhow!("unsafe fingerprint input path"))?;
    Ok(Some((path, entry)))
}

/// Classifies one input. A final symlink is accepted only when `tracked_as_symlink` (Git index
/// mode 120000) holds and it resolves inside the repository; every other symlink fails the run.
fn input_entry(root: &Path, relative: &str, tracked_as_symlink: bool) -> Result<InputEntry> {
    let Some((path, metadata)) = final_entry(root, relative)? else {
        return Ok(InputEntry::Absent);
    };
    if metadata.file_type().is_symlink() {
        ensure!(
            tracked_as_symlink,
            "symlink fingerprint input is not tracked as a symlink: {relative}"
        );
        return tracked_symlink_text(root, &path, relative).map(InputEntry::TrackedSymlink);
    }
    ensure!(
        metadata.is_file(),
        "fingerprint input is not a regular file: {relative}"
    );
    Ok(InputEntry::RegularFile)
}

/// Resolves the whole link chain: the final target must exist inside the canonical repository
/// root, and the link text must be unchanged after resolution.
fn tracked_symlink_text(root: &Path, link: &Path, relative: &str) -> Result<PathBuf> {
    let text = std::fs::read_link(link)?;
    let repository = std::fs::canonicalize(root)?;
    let target = std::fs::canonicalize(link).map_err(|error| {
        anyhow::anyhow!("symlink fingerprint input does not resolve: {relative}: {error}")
    })?;
    ensure!(
        target.starts_with(&repository),
        "symlink fingerprint input resolves outside the repository: {relative}"
    );
    ensure!(
        std::fs::read_link(link)? == text,
        "symlink fingerprint input changed while resolving: {relative}"
    );
    Ok(text)
}

/// Configuration files are never tracked, so any symlink on their path fails the run; absent
/// optional files remain distinguishable.
fn input_exists(root: &Path, relative: &str) -> Result<bool> {
    Ok(!matches!(
        input_entry(root, relative, false)?,
        InputEntry::Absent
    ))
}

#[derive(Debug, PartialEq, Eq)]
struct SourceInventory {
    paths: BTreeSet<String>,
    deleted: BTreeSet<String>,
    /// Source inputs whose Git index entry has mode 120000.
    symlinks: BTreeSet<String>,
}

/// NUL-separated records of one `git ls-files -z` invocation.
fn git_records(root: &Path, arguments: &[&str]) -> Result<Vec<String>> {
    let output = Run::new("git")
        .args(arguments)
        .cwd(root)
        .output()
        .map_err(|error| anyhow::anyhow!("source inventory failed: {error:?}"))?;
    ensure!(output.code == 0, "git source inventory failed");
    Ok(output
        .stdout
        .split('\0')
        .filter(|record| !record.is_empty())
        .map(str::to_owned)
        .collect())
}

fn git_source_paths(root: &Path, arguments: &[&str]) -> Result<BTreeSet<String>> {
    Ok(git_records(root, arguments)?
        .into_iter()
        .filter(|path| source_input(path))
        .collect())
}

/// `git ls-files -z --stage` records read `<mode> <object> <stage>\t<path>`.
fn git_tracked_symlinks(root: &Path) -> Result<BTreeSet<String>> {
    let mut symlinks = BTreeSet::new();
    for record in git_records(root, &["ls-files", "-z", "--stage"])? {
        let (index_entry, path) = record
            .split_once('\t')
            .ok_or_else(|| anyhow::anyhow!("malformed git index record"))?;
        if index_entry.split(' ').next() == Some(GIT_SYMLINK_MODE) && source_input(path) {
            symlinks.insert(path.to_owned());
        }
    }
    Ok(symlinks)
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
    let symlinks = git_tracked_symlinks(root)?;
    ensure!(
        symlinks.is_subset(&paths),
        "source inventory changed while discovering tracked symlinks"
    );
    Ok(SourceInventory {
        paths,
        deleted,
        symlinks,
    })
}

/// Only Git-observed tracked deletions may lack bytes. Required implementation paths are
/// validated independently by the register; a tombstone never establishes their availability.
/// Each path frames one tag (`deleted`, `present` with the file bytes, or `symlink` with the link
/// text), so no two entry kinds share a hash input.
fn hash_source_inventory(root: &Path, inventory: &SourceInventory) -> Result<String> {
    let mut hash = Sha256Hasher::new();
    hash.update_length_framed(b"api-readiness-source-v4");
    for path in &inventory.paths {
        let entry = input_entry(root, path, inventory.symlinks.contains(path))?;
        ensure!(
            matches!(entry, InputEntry::Absent) == inventory.deleted.contains(path),
            "fingerprint source input presence changed after inventory: {path}"
        );
        hash.update_length_framed(path.as_bytes());
        match entry {
            InputEntry::Absent => hash.update_length_framed(b"deleted"),
            InputEntry::RegularFile => {
                hash.update_length_framed(b"present");
                hash.update_file_length_framed(&root.join(path))?;
            }
            InputEntry::TrackedSymlink(link_text) => {
                hash.update_length_framed(b"symlink");
                hash.update_length_framed(link_text.as_os_str().as_encoded_bytes());
            }
        }
    }
    Ok(hash.finalize_hex())
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
    hash: &mut Sha256Hasher,
    environment: impl IntoIterator<Item = (OsString, OsString)>,
) {
    let property_environment: BTreeMap<_, _> = environment
        .into_iter()
        .filter(|(name, _)| name.as_encoded_bytes().starts_with(b"PROPTEST_"))
        .collect();
    hash.update_length_framed(b"property-environment");
    hash.update((property_environment.len() as u64).to_le_bytes());
    for (name, value) in property_environment {
        hash.update_length_framed(name.as_encoded_bytes());
        hash.update_length_framed(value.as_encoded_bytes());
    }
}

/// Hash raw configuration inputs and environment overrides conservatively: even a
/// changed overridden file invalidates evidence. Values never appear in diagnostics.
pub(super) fn configuration(root: &Path) -> Result<String> {
    let mut hash = Sha256Hasher::new();
    hash.update_length_framed(b"api-readiness-configuration-v2");
    for relative in CONFIGURATION_FILES {
        hash.update_length_framed(relative.as_bytes());
        let exists = input_exists(root, relative)?;
        hash.update([u8::from(exists)]);
        if exists {
            hash.update_file_length_framed(&root.join(relative))?;
        }
    }
    hash_property_environment(&mut hash, std::env::vars_os());
    for name in CONFIGURATION_ENVIRONMENT {
        hash.update_length_framed(name.as_bytes());
        let value = std::env::var_os(name);
        hash.update([u8::from(value.is_some())]);
        if let Some(value) = value {
            hash.update_length_framed(value.as_encoded_bytes());
        }
    }
    Ok(hash.finalize_hex())
}

#[cfg(test)]
#[path = "tests/property_fingerprint.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/source_fingerprint.rs"]
mod source_tests;
