//! Ticket title lookup for platform displays.

use std::path::Path;

/// Missing or malformed tickets have an empty display title.
pub fn read_ticket_title(root: &Path, s: &str) -> String {
    let dir = crate::registry::ticket_file_storage::tickets_dir(root).join(format!("{s}.toml"));
    let Ok(text) = std::fs::read_to_string(dir) else {
        return String::new();
    };
    match crate::parse_ticket_toml(&text) {
        Ok(crate::Ticket::Work(w)) => w.title,
        Ok(crate::Ticket::Program(p)) => p.title,
        Err(_) => String::new(),
    }
}
