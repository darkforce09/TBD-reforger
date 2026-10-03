use super::*;

use crate::Status;

fn worktree_root() -> std::path::PathBuf {
    repository_layout::find_repository_root().expect("repository root")
}

fn parse_file(root: &Path, id: &str) -> Ticket {
    let text =
        std::fs::read_to_string(root.join(format!("{}/{id}.toml", repository_layout::TICKETS_DIR)))
            .unwrap();
    parse_ticket_toml(&text).unwrap_or_else(|e| panic!("{id}: {e}"))
}

mod typed_projection_tests;
