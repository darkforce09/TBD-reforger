//! Git-dirty chip model (T-915.3 §UI shape) — pure, unit-tested, no egui types.
//!
//! The app never commits — the operator commits — so pending registry state must
//! be visible: after every reload and every check run, the app runs
//! `git status --porcelain -- .ai/tickets docs CLAUDE.md` (through the same
//! subproc helper, explicit `current_dir` at the repo root) and summarizes the
//! result as a chip. Git absent, or not a repo? The chip says "git unavailable"
//! — never a crash, never a fake "clean".

/// argv after `git`: `status --porcelain -- <registry surface>`.
pub const GIT_ARGS: [&str; 6] = [
    "status",
    "--porcelain",
    "--",
    ".ai/tickets",
    "docs",
    "CLAUDE.md",
];

/// Chip state. `Dirty` keeps the porcelain entries VERBATIM (`XY path`) — the
/// status columns are signal, not noise.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GitChip {
    /// Not yet run.
    #[default]
    Unknown,
    Clean,
    Dirty(Vec<String>),
    /// Spawn failed or nonzero exit — the verbatim reason for the tooltip.
    Unavailable(String),
}

impl GitChip {
    /// The banner chip text.
    pub fn label(&self) -> String {
        match self {
            GitChip::Unknown => "git …".to_owned(),
            GitChip::Clean => "clean".to_owned(),
            GitChip::Dirty(files) => {
                format!("{} uncommitted registry file(s)", files.len())
            }
            GitChip::Unavailable(_) => "git unavailable".to_owned(),
        }
    }
}

/// Build the chip from a finished `git status` run: exit 0 parses the porcelain
/// lines; anything else (including a killed process) is `Unavailable` with the
/// honest reason — nonzero git output is never parsed into a count.
pub fn chip_from_exit<'a>(code: Option<i32>, lines: impl IntoIterator<Item = &'a str>) -> GitChip {
    match code {
        Some(0) => {
            let entries = parse_porcelain(lines);
            if entries.is_empty() {
                GitChip::Clean
            } else {
                GitChip::Dirty(entries)
            }
        }
        Some(code) => GitChip::Unavailable(format!("git exited {code}")),
        None => GitChip::Unavailable("git killed by signal".to_owned()),
    }
}

/// Porcelain-v1 entries out of a MERGED stdout+stderr stream: two status
/// columns, a space, then the path (`XY path`; renames `XY old -> new`). Stray
/// non-porcelain lines (warnings, advice) are dropped by shape — defensive
/// because the subproc stream merges stderr in.
pub fn parse_porcelain<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| is_porcelain_entry(line))
        .map(str::to_owned)
        .collect()
}

/// `XY path` shape: both status columns from the porcelain alphabet, then a
/// space, then a non-empty path.
fn is_porcelain_entry(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 4 && is_status_byte(bytes[0]) && is_status_byte(bytes[1]) && bytes[2] == b' '
}

/// Porcelain status alphabet (`git-status(1)`): ` MTADRCU?!`.
fn is_status_byte(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'M' | b'T' | b'A' | b'D' | b'R' | b'C' | b'U' | b'?' | b'!'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture mirroring a real dirty registry: modified, staged, untracked, a
    /// rename — plus merged-stream pollution that must not count.
    const FIXTURE: &str = "\
warning: unable to access '/home/x/.gitconfig': Permission denied
 M .ai/tickets/T-915.3.toml
M  docs/TICKET_LEAD.md
?? .ai/tickets/T-999.toml
R  docs/old.md -> docs/TICKET_NEW.md
A  CLAUDE.md

hint: use git add to stage
";

    #[test]
    fn porcelain_parse_counts_and_lists_entries_only() {
        let entries = parse_porcelain(FIXTURE.lines());
        assert_eq!(entries.len(), 5);
        assert_eq!(
            entries,
            vec![
                " M .ai/tickets/T-915.3.toml",
                "M  docs/TICKET_LEAD.md",
                "?? .ai/tickets/T-999.toml",
                "R  docs/old.md -> docs/TICKET_NEW.md",
                "A  CLAUDE.md",
            ]
        );
    }

    #[test]
    fn chip_states_from_exit() {
        let dirty = chip_from_exit(Some(0), FIXTURE.lines());
        match &dirty {
            GitChip::Dirty(files) => assert_eq!(files.len(), 5),
            other => panic!("expected Dirty, got {other:?}"),
        }
        assert_eq!(dirty.label(), "5 uncommitted registry file(s)");

        let clean = chip_from_exit(Some(0), std::iter::empty());
        assert_eq!(clean, GitChip::Clean);
        assert_eq!(clean.label(), "clean");

        // Nonzero exit (e.g. "fatal: not a git repository" on stderr): the
        // stream is NOT parsed into a count.
        let unavailable = chip_from_exit(Some(128), ["fatal: not a git repository"]);
        assert_eq!(
            unavailable,
            GitChip::Unavailable("git exited 128".to_owned())
        );
        assert_eq!(unavailable.label(), "git unavailable");

        let killed = chip_from_exit(None, std::iter::empty());
        assert_eq!(killed.label(), "git unavailable");
        assert_eq!(GitChip::Unknown.label(), "git …");
    }

    #[test]
    fn shape_filter_edges() {
        // Too short / wrong separator / non-status alphabet.
        assert!(parse_porcelain(["?? "].into_iter()).is_empty());
        assert!(parse_porcelain(["MM"].into_iter()).is_empty());
        assert!(parse_porcelain(["fatal: nope"].into_iter()).is_empty());
        assert!(parse_porcelain(["-- x"].into_iter()).is_empty());
        // Minimal legal entry.
        assert_eq!(parse_porcelain(["?? x"].into_iter()), vec!["?? x"]);
    }
}
