//! Resolution of the command that starts an `enfusion-mcp` server.
//!
//! Three callers need the same answer — the one-shot runner behind `cargo xtask mcp call`, the
//! daemon launcher behind `cargo xtask mcp daemon`, and the `mcpd` broker that owns the server
//! child. Resolving it in one place keeps them from drifting into three different servers, and
//! keeps the installed module's path spelled exactly once, in
//! [`crate::repository_layout::ENFUSION_MCP_ENTRYPOINT`].
//!
//! The tiers are ordered by how much the caller controls the answer:
//!
//! 1. `ENFUSION_MCP_BIN` — an explicit override, used by tests and by the daemon to hand its
//!    child the entry it already resolved. Honoured only when it names an existing file, so a
//!    stale value degrades to the next tier instead of failing the call.
//! 2. The module installed by `npm ci` in this repository's npm package directory. This is the
//!    normal answer: the version is pinned by the committed lockfile and starting it costs no
//!    network round trip.
//! 3. A copy npm already downloaded under `~/.npm/_npx`. Scanned four directories deep, sorted so
//!    that two machines with the same cache pick the same file; an unreadable directory is
//!    skipped rather than raised, because a missing cache is not an error.
//! 4. `npx -y enfusion-mcp`, which downloads on demand.

use std::path::{Path, PathBuf};

use crate::repository_layout::{ENFUSION_MCP_ENTRYPOINT, enfusion_mcp_entrypoint};

/// Where the resolved command came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnfusionMcpSource {
    /// `ENFUSION_MCP_BIN` named an existing file.
    EnvironmentOverride,
    /// The module installed by `npm ci` in this repository.
    PinnedPackage,
    /// A copy npm had already downloaded under `~/.npm/_npx`.
    NpxCache,
    /// Nothing was on disk; `npx` downloads the server on demand.
    NpxDownload,
}

impl EnfusionMcpSource {
    /// Short label for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            Self::EnvironmentOverride => "environment-override",
            Self::PinnedPackage => "pinned-package",
            Self::NpxCache => "npx-cache",
            Self::NpxDownload => "npx-download",
        }
    }
}

/// The program and arguments that start an `enfusion-mcp` server process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnfusionMcpCommand {
    /// Executable to spawn: `node`, `npx`, or a native runner named by the override.
    pub program: String,
    /// Arguments to pass it.
    pub args: Vec<String>,
    /// The entry module or native runner on disk, when the answer names one. `None` for the
    /// download tier, which has no file until `npx` has fetched it.
    pub entry_path: Option<String>,
    /// Which tier produced this command.
    pub source: EnfusionMcpSource,
}

impl EnfusionMcpCommand {
    /// The command as one argv vector: program first, then its arguments.
    pub fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len() + 1);
        argv.push(self.program.clone());
        argv.extend(self.args.iter().cloned());
        argv
    }
}

/// Resolve the server command for a checkout.
pub fn resolve(repo_root: &Path) -> EnfusionMcpCommand {
    if let Some(command) = from_environment_override() {
        return command;
    }
    let pinned = enfusion_mcp_entrypoint(repo_root);
    if pinned.is_file() {
        return node_command(
            pinned.to_string_lossy().into_owned(),
            EnfusionMcpSource::PinnedPackage,
        );
    }
    if let Some(cached) = first_npx_cache_copy() {
        return node_command(cached, EnfusionMcpSource::NpxCache);
    }
    EnfusionMcpCommand {
        program: "npx".into(),
        args: vec!["-y".into(), "enfusion-mcp".into()],
        entry_path: None,
        source: EnfusionMcpSource::NpxDownload,
    }
}

/// The regular expression that matches a running pinned server by its argv.
///
/// `pkill -f` matches against the whole command line, so the pattern is the installed module's
/// path below its package directory: it identifies this repository's server wherever the checkout
/// sits, and matches the same module started from an npm cache copy. Derived from
/// [`ENFUSION_MCP_ENTRYPOINT`] so the two can never name different files.
pub fn process_pattern() -> String {
    let suffix = ENFUSION_MCP_ENTRYPOINT
        .rsplit_once("/node_modules/")
        .map(|(_, tail)| format!("node_modules/{tail}"))
        .unwrap_or_else(|| ENFUSION_MCP_ENTRYPOINT.to_string());
    escape_regex_metacharacters(&suffix)
}

/// An `ENFUSION_MCP_BIN` that names an existing file.
///
/// A JavaScript module runs under `node`; anything else is a native runner and is executed
/// directly, which is what lets the test stub stand in for the real server.
fn from_environment_override() -> Option<EnfusionMcpCommand> {
    let bin = std::env::var("ENFUSION_MCP_BIN").ok()?;
    if bin.is_empty() || !Path::new(&bin).is_file() {
        return None;
    }
    if bin.ends_with(".js") || bin.ends_with(".mjs") {
        return Some(node_command(bin, EnfusionMcpSource::EnvironmentOverride));
    }
    Some(EnfusionMcpCommand {
        program: bin.clone(),
        args: Vec::new(),
        entry_path: Some(bin),
        source: EnfusionMcpSource::EnvironmentOverride,
    })
}

fn node_command(entry: String, source: EnfusionMcpSource) -> EnfusionMcpCommand {
    EnfusionMcpCommand {
        program: "node".into(),
        args: vec![entry.clone()],
        entry_path: Some(entry),
        source,
    }
}

/// The lexically first `enfusion-mcp` entry module in npm's download cache.
fn first_npx_cache_copy() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let cache = PathBuf::from(home).join(".npm/_npx");
    if !cache.is_dir() {
        return None;
    }
    let mut hits = Vec::new();
    collect_entry_modules(&cache, 0, 4, &mut hits);
    hits.sort();
    hits.into_iter().next()
}

/// Depth-limited walk collecting every `enfusion-mcp` entry module under `dir`.
///
/// A directory that cannot be read is skipped: an npm cache is other software's private state,
/// and a permission error inside it says nothing about whether a usable copy exists elsewhere.
fn collect_entry_modules(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<String>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_entry_modules(&path, depth + 1, max_depth, out);
        } else if path.is_file() {
            let shown = path.to_string_lossy();
            if shown.contains("enfusion-mcp/dist/index.js") {
                out.push(shown.into_owned());
            }
        }
    }
}

/// Escape the characters a POSIX extended regular expression reads as syntax.
fn escape_regex_metacharacters(literal: &str) -> String {
    let mut escaped = String::with_capacity(literal.len());
    for ch in literal.chars() {
        if r".^$*+?()[]{}|\".contains(ch) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

#[cfg(test)]
#[path = "tests/enfusion_mcp_entrypoint_tests.rs"]
mod tests;
