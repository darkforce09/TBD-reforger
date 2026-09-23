use super::*;
/// argv prefix spawned before the verb tail: the `.cargo/config.toml` alias
/// expansion of `cargo xtask ticket` (`xtask = "run --package xtask --"`) —
/// byte-equivalent without depending on alias resolution, mirroring
/// `trust::CHECK_ARGS`.
pub const TICKET_PREFIX: [&str; 5] = ["run", "--package", "xtask", "--", "ticket"];

// ---- requests ----

/// One verb invocation, fully built: the argv to hand to the subproc helper and
/// the literal command line the operator confirms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketCommand {
    /// argv after `cargo` — [`TICKET_PREFIX`] + verb tail.
    pub args: Vec<String>,
    /// `cargo xtask ticket ship T-915.4` — shown verbatim in every confirm
    /// dialog and in the drawer header.
    pub display: String,
    /// Compare-and-swap target; `None` only for `add` (no pre-existing file).
    pub guard: Option<FileChangeGuard>,
}

impl TicketCommand {
    pub fn with_guard(mut self, guard: FileChangeGuard) -> Self {
        self.guard = Some(guard);
        self
    }
}

/// Single-quote an argument for DISPLAY when it contains anything beyond the
/// plain filename alphabet. The spawned argv is the exact string — no shell ever
/// parses it; quoting exists so the confirm dialog shows a paste-able line.
fn sh_quote(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@' | '+' | '=')
        });
    if plain {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

fn request(tail: Vec<String>) -> TicketCommand {
    let mut args: Vec<String> = TICKET_PREFIX.iter().map(|s| (*s).to_owned()).collect();
    args.extend(tail.iter().cloned());
    let display = std::iter::once("cargo xtask ticket".to_owned())
        .chain(tail.iter().map(|a| sh_quote(a)))
        .collect::<Vec<_>>()
        .join(" ");
    TicketCommand {
        args,
        display,
        guard: None,
    }
}

// ---- builders (one per CLI verb — arg shapes mirror tools_v2/xtask/src/main.rs TicketCmd) ----

/// `ticket ship <id>` — status→shipped, stamps completed_at, clears active.
pub fn ship(id: &str) -> TicketCommand {
    request(vec!["ship".into(), id.into()])
}

/// `ticket set-status <id> <status>` — the raw 8-value enum gate.
pub fn set_status(id: &str, status: StatusName) -> TicketCommand {
    request(vec!["set-status".into(), id.into(), status.as_str().into()])
}

/// `ticket mark-ready <id> [spec]` — the verb takes ONLY id + spec; main_goal /
/// acceptance backfill is the verb's own behavior, never a UI field.
pub fn mark_ready(id: &str, spec: Option<&str>) -> TicketCommand {
    let mut tail = vec!["mark-ready".to_owned(), id.to_owned()];
    if let Some(spec) = spec {
        tail.push(spec.to_owned());
    }
    request(tail)
}

/// `ticket reorder <id> <after>` — order = anchor + 1; flips idea→queued
/// server-side.
pub fn reorder(id: &str, after: &str) -> TicketCommand {
    request(vec!["reorder".into(), id.into(), after.into()])
}

/// `ticket add <title> [--summary <s>]` — id minted server-side (max parent
/// numeric + 1), kind work, status idea.
pub fn add(title: &str, summary: &str) -> TicketCommand {
    let mut tail = vec!["add".to_owned(), title.to_owned()];
    if !summary.trim().is_empty() {
        tail.push("--summary".to_owned());
        tail.push(summary.to_owned());
    }
    request(tail)
}

/// `ticket add-child <parent> <title> [--summary <s>] [--promote]` — a work
/// parent refuses without `--promote` (the atomic work→program rewrite).
pub fn add_child(parent: &str, title: &str, summary: &str, promote: bool) -> TicketCommand {
    let mut tail = vec!["add-child".to_owned(), parent.to_owned(), title.to_owned()];
    if !summary.trim().is_empty() {
        tail.push("--summary".to_owned());
        tail.push(summary.to_owned());
    }
    if promote {
        tail.push("--promote".to_owned());
    }
    request(tail)
}

/// `ticket remove <id> [--force]` — a program refuses without `--force`
/// (cascade-deletes every descendant ticket file).
pub fn remove(id: &str, force: bool) -> TicketCommand {
    let mut tail = vec!["remove".to_owned(), id.to_owned()];
    if force {
        tail.push("--force".to_owned());
    }
    request(tail)
}

/// `ticket advance-slice <id>` — programs only; walks typed children.
pub fn advance_slice(id: &str) -> TicketCommand {
    request(vec!["advance-slice".into(), id.into()])
}
