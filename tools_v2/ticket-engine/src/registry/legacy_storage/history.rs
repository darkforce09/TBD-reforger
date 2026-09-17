//! History.

use super::*;

/// Status map at a git rev: JSON monolith if that blob exists, else every `T-*.toml`.
/// Returns `None` when neither form exists (refuse — never an empty "all unshipped" map).
pub fn status_map_at_rev(
    repo: &Path,
    rev: &str,
) -> Option<std::collections::HashMap<String, String>> {
    use std::process::Command;
    let spec = format!("{rev}:.ai/tickets/registry.json");
    let json_exists = Command::new("git")
        .args(["cat-file", "-e", &spec])
        .current_dir(repo)
        .status()
        .ok()?
        .success();
    if json_exists {
        let blob = Command::new("git")
            .args(["show", &spec])
            .current_dir(repo)
            .output()
            .ok()?;
        if !blob.status.success() {
            return None;
        }
        let v: Value = serde_json::from_str(&String::from_utf8(blob.stdout).ok()?).ok()?;
        let mut by = std::collections::HashMap::new();
        if let Some(arr) = v.get("tickets").and_then(Value::as_array) {
            for t in arr {
                let id = t.get("id")?.as_str()?.to_string();
                let st = t
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                by.insert(id, st);
            }
        }
        return Some(by);
    }
    let listing = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", rev, "--", ".ai/tickets/"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !listing.status.success() {
        return None;
    }
    let mut by = std::collections::HashMap::new();
    let mut any = false;
    for line in String::from_utf8_lossy(&listing.stdout).lines() {
        let path = line.trim();
        if !path.starts_with(".ai/tickets/T-") || !path.ends_with(".toml") {
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
        let st = v
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        by.insert(id, st);
        any = true;
    }
    if !any {
        return None;
    }
    Some(by)
}
