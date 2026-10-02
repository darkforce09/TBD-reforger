use super::*;

fn worktree_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

mod collision_source_tests;
