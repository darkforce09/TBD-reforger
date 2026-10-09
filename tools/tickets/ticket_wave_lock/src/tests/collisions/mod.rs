use super::*;

fn worktree_root() -> PathBuf {
    repository_root::find_repository_root().expect("repository root")
}

mod collision_source_tests;
