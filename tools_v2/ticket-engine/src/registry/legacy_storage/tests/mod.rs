use super::*;

use crate::registry::{format_json_unicode_preserve, load_json_monolith};

use std::collections::BTreeSet;

fn repo_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

fn load_json_or_toml(root: &Path) -> Result<Value> {
    let json = tickets_dir(root).join("registry.json");
    if json.is_file() {
        load_json_monolith(&json)
    } else {
        load_toml_tree(root)
    }
}

/// Gold JSON is the unicode-preserving emit of the parsed monolith.
/// The on-disk file still contains `\\u` escapes; parse+emit is the cmp target.
fn canonical_monolith(root: &Path) -> (Value, String) {
    let json_path = tickets_dir(root).join("registry.json");
    let original = if json_path.is_file() {
        fs::read_to_string(&json_path).expect("read monolith")
    } else {
        // After the cutover commit deletes the blob, gold is the parent of that delete.
        let del = std::process::Command::new("git")
            .args([
                "log",
                "-1",
                "--diff-filter=D",
                "--pretty=%H",
                "--",
                ".ai/tickets/registry.json",
            ])
            .current_dir(root)
            .output()
            .expect("git log delete");
        let sha = String::from_utf8_lossy(&del.stdout).trim().to_string();
        assert!(
            !sha.is_empty(),
            "no deleting commit for .ai/tickets/registry.json"
        );
        let shown = std::process::Command::new("git")
            .args(["show", &format!("{sha}^:.ai/tickets/registry.json")])
            .current_dir(root)
            .output()
            .expect("git show cutover monolith");
        assert!(
            shown.status.success(),
            "git show {sha}^:.ai/tickets/registry.json failed"
        );
        String::from_utf8(shown.stdout).unwrap()
    };
    let parsed: Value = serde_json::from_str(&original).expect("parse monolith");
    let gold = format_json_unicode_preserve(&parsed).expect("emit gold");
    (parsed, gold)
}

mod legacy_storage_tests;
