use super::*;

use crate::registry::format_json_unicode_preserve;
use crate::registry::ticket_status_history::historical_registry_json;

use std::collections::BTreeSet;

fn repo_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

/// The whole registry as one JSON document, read from the revision that last carried it, and
/// the unicode-preserving emit of it. The on-disk bytes there still carry `\u` escapes, so the
/// comparison target is parse-then-emit rather than the raw file.
fn canonical_registry_document(root: &Path) -> (Value, String) {
    let document = historical_registry_json();
    let deleting_commit = std::process::Command::new("git")
        .args([
            "log",
            "-1",
            "--diff-filter=D",
            "--pretty=%H",
            "--",
            &document,
        ])
        .current_dir(root)
        .output()
        .expect("git log delete");
    let sha = String::from_utf8_lossy(&deleting_commit.stdout)
        .trim()
        .to_string();
    assert!(!sha.is_empty(), "no commit deletes {document}");
    let shown = std::process::Command::new("git")
        .args(["show", &format!("{sha}^:{document}")])
        .current_dir(root)
        .output()
        .expect("git show the document");
    assert!(shown.status.success(), "git show {sha}^:{document} failed");
    let original = String::from_utf8(shown.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&original).expect("parse the document");
    let gold = format_json_unicode_preserve(&parsed).expect("emit gold");
    (parsed, gold)
}

mod ticket_file_storage_tests;
