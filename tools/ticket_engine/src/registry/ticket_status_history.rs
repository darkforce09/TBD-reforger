//! Ticket statuses as they stood at a past revision.
//!
//! The wave gate corroborates a derived wave-close boundary against the registry at that
//! boundary's parent commit, so the answer must come from git rather than from the checkout.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::registry::ticket_file_storage::ticket_from_toml_str;
use crate::repository::TICKETS_DIR;

/// The single JSON file the registry lived in before it became one file per ticket.
const HISTORICAL_REGISTRY_FILE_NAME: &str = "registry.json";

/// That file's path inside the ticket directory. Revisions older than the split into one file
/// per ticket still carry it, and reading them is the only reason this path exists: nothing in a
/// working tree has held it since.
pub(crate) fn historical_registry_json() -> String {
    format!("{TICKETS_DIR}/{HISTORICAL_REGISTRY_FILE_NAME}")
}

/// Ticket id → status at `rev`, read from the ticket files there, or from the single JSON file
/// at revisions old enough to predate them.
///
/// `None` when neither form exists at that revision. An empty map would read as "every ticket
/// is unshipped", which is a wrong answer rather than a missing one.
pub fn status_map_at_rev(repo: &Path, rev: &str) -> Option<HashMap<String, String>> {
    let spec = format!("{rev}:{}", historical_registry_json());
    let json_exists = Command::new("git")
        .args(["cat-file", "-e", &spec])
        .current_dir(repo)
        .status()
        .ok()?
        .success();
    if json_exists {
        return statuses_from_json_blob(repo, &spec);
    }
    statuses_from_ticket_files(repo, rev)
}

fn statuses_from_json_blob(repo: &Path, spec: &str) -> Option<HashMap<String, String>> {
    let blob = Command::new("git")
        .args(["show", spec])
        .current_dir(repo)
        .output()
        .ok()?;
    if !blob.status.success() {
        return None;
    }
    let v: Value = serde_json::from_str(&String::from_utf8(blob.stdout).ok()?).ok()?;
    let mut by = HashMap::new();
    if let Some(arr) = v.get("tickets").and_then(Value::as_array) {
        for t in arr {
            let id = t.get("id")?.as_str()?.to_string();
            by.insert(id, status_of(t));
        }
    }
    Some(by)
}

fn statuses_from_ticket_files(repo: &Path, rev: &str) -> Option<HashMap<String, String>> {
    let tickets_prefix = format!("{TICKETS_DIR}/");
    let listing = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", rev, "--", &tickets_prefix])
        .current_dir(repo)
        .output()
        .ok()?;
    if !listing.status.success() {
        return None;
    }
    let ticket_file_prefix = format!("{tickets_prefix}T-");
    let mut by = HashMap::new();
    for line in String::from_utf8_lossy(&listing.stdout).lines() {
        let path = line.trim();
        if !path.starts_with(&ticket_file_prefix) || !path.ends_with(".toml") {
            continue;
        }
        let blob = Command::new("git")
            .args(["show", &format!("{rev}:{path}")])
            .current_dir(repo)
            .output()
            .ok()?;
        if !blob.status.success() {
            continue;
        }
        let text = String::from_utf8(blob.stdout).ok()?;
        let (_, v) = ticket_from_toml_str(&text).ok()?;
        let id = v.get("id")?.as_str()?.to_string();
        by.insert(id, status_of(&v));
    }
    if by.is_empty() {
        return None;
    }
    Some(by)
}

fn status_of(ticket: &Value) -> String {
    ticket
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
#[path = "tests/ticket_status_history_tests.rs"]
mod tests;
