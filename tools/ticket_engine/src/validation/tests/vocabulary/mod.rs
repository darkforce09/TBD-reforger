use super::*;

fn worktree_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

/// Scratch tree carrying only the vocab file — the rule under test reads nothing else.
fn scratch(tag: &str, content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t917-vocab-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(crate::repository::TICKETS_DIR)).unwrap();
    fs::write(dir.join(SCOPE_VOCAB), content).unwrap();
    dir
}

/// A minimal green tree exercising both encodings a layer can take: components with
/// surfaces, empty component arrays, and a bare component-free layer header.
const GREEN: &str = "[mod.workbench]\n\n[website.frontend]\nmission_creator = [\"map_canvas\", \"toolbelt\"]\nsite_pages = []\n";

mod vocabulary_shape_tests;
