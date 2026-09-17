use super::*;

use crate::Status;

fn repo_root() -> std::path::PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

fn parse_file(root: &Path, id: &str) -> Ticket {
    let text = std::fs::read_to_string(root.join(format!(".ai/tickets/{id}.toml"))).unwrap();
    parse_ticket_toml(&text).unwrap_or_else(|e| panic!("{id}: {e}"))
}

mod typed_projection_tests;
