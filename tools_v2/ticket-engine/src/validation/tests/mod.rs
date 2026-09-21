use super::*;

use serde_json::json;

use std::path::PathBuf;

fn worktree_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

/// Scratch tickets dir carrying the minimal vocabulary the fail-closed corpus load
/// requires (Corpus::load resolves scope legality on every load).
fn scratch_tickets_dir(tag: &str) -> (PathBuf, PathBuf) {
    let tmp = std::env::temp_dir().join(format!("{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let dir = tmp.join(crate::repository::TICKETS_DIR);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
            dir.join("scope-vocab.toml"),
            "[mod.scripts]\nbackend = []\n\n[repo.docs]\n\n[website.frontend]\nmission_creator = [\"map_canvas\"]\n",
        )
        .unwrap();
    (tmp, dir)
}

mod schema_and_integrity_tests;

mod readiness_and_accounting_tests;
