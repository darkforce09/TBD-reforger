//! Ticket title lookup for platform displays.

use std::path::Path;

/// The title of ticket `s` read from its file under `root`; empty when the file is missing or
/// does not parse.
pub fn read_ticket_title(root: &Path, s: &str) -> String {
    let dir = crate::registry::ticket_file_storage::tickets_dir(root).join(format!("{s}.toml"));
    let Ok(text) = std::fs::read_to_string(dir) else {
        return String::new();
    };
    match ticket_model::parse_ticket_toml(&text) {
        Ok(ticket_model::Ticket::Work(w)) => w.title,
        Ok(ticket_model::Ticket::Program(p)) => p.title,
        Err(_) => String::new(),
    }
}
