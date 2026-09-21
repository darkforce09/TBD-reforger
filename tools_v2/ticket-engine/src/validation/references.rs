//! References between tickets, and between tickets and the documents they cite.

use super::*;

use crate::repository::documentation::{
    ARCHIVED_WAVE_PLAN_READERS, SCAN_EXEMPT_PREFIXES, STALE_TICKET_ID_SCAN_ROOTS,
};

/// T-916.2 — parent↔child referential integrity over EVERY `.ai/tickets/T-*.toml` (the typed
/// corpus; parents-only walks cannot see either half of the relation). Two rules, both naming
/// the pair:
///
/// - every `children[]` entry must have an on-disk `T-<child>.toml` — with `save_tree`'s
///   delete pass gone (T-916.2 demoted it to migration/test duty) a mangled `children[]` can
///   no longer mass-delete files, but a listing without a file was previously INVISIBLE:
///   nothing checked parent↔child at all;
/// - every child file's `parent` must exist on disk — a removed parent would otherwise strand
///   its children as permanently unreachable rows.
///
/// Measured against the live tree 2026-08-14: ZERO violations, so no allowlist. The one
/// pre-known oddity — T-111 (frozen-unmappable parking) cross-listing T-067.1 — satisfies both
/// rules because the file and its parent both exist; only a dotted-extension SHAPE rule would
/// red it, and that rule deliberately lives in the ops post-image gate (changed programs only),
/// not here, exactly so frozen history stays green.
///
/// Fail-closed: a corpus that cannot load reports the load error — a guard that cannot scan
/// must not report clean (the fossil-guard precedent).
pub(super) fn check_children_integrity(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for (id, ticket) in &corpus.tickets {
        match ticket {
            crate::Ticket::Program(p) => {
                for child in &p.children {
                    if !corpus.tickets.contains_key(child) {
                        errors.push(format!(
                            "{id}: children[] names {child}, which has no .ai/tickets/{child}.toml on disk"
                        ));
                    }
                }
            }
            crate::Ticket::Work(w) => {
                if let Some(parent) = &w.parent
                    && !corpus.tickets.contains_key(parent)
                {
                    errors.push(format!(
                        "{id}: parent {parent} has no .ai/tickets/{parent}.toml on disk"
                    ));
                }
            }
        }
    }
    errors
}

/// T-912.2 fossil-path guard: the wave-plan TSVs and their env knobs are dead, and any LIVE
/// mention of them is a regression vector — a reader quietly retargeted at a file that no longer
/// exists is exactly the false-green class this program killed. Greps the tracked tree (working
/// contents, so an uncommitted plant is caught) minus a tight historical allowlist.
///
/// Needles are assembled at runtime, same trick as the T-912.1 `const DEPS` tripwire, so this
/// file's own source cannot satisfy the scan it performs.
pub(super) fn fossil_needles() -> [String; 3] {
    [
        format!("wave_plan{}", ".tsv"),
        format!("TBD_WAVE{}", "_PLAN"),
        format!("TBD_WAVE_GENERATION{}", "_FLOOR"),
    ]
}

pub(super) fn fossil_paths_check(root: &Path) -> Vec<String> {
    let needles = fossil_needles();
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(["grep", "-l", "-I", "--fixed-strings"]);
    for n in &needles {
        cmd.args(["-e", n]);
    }
    cmd.args(["--", "."]);
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => return vec![format!("fossil-path guard could not run git grep: {e}")],
    };
    // git grep: 0 = matches, 1 = no matches, anything else = failure. Fail closed — a guard
    // that cannot scan must not report clean.
    match out.status.code() {
        Some(0) | Some(1) => {}
        other => {
            return vec![format!(
                "fossil-path guard: git grep failed (rc {other:?}): {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )];
        }
    }
    let mut errors = Vec::new();
    for path in String::from_utf8_lossy(&out.stdout).lines() {
        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        if ARCHIVED_WAVE_PLAN_READERS
            .iter()
            .any(|(p, _)| path.starts_with(p))
        {
            continue;
        }
        errors.push(format!(
            "archived wave-plan reference in {path} — the wave plan is {}; a mention that is \
             genuinely about the past belongs on ARCHIVED_WAVE_PLAN_READERS in \
             tools_v2/ticket-engine/src/repository.rs, with its reason",
            crate::repository::WAVE_LOCK
        ));
    }
    errors
}

pub(super) fn scan_legacy_ids(root: &Path) -> HashMap<String, Vec<String>> {
    let mut hits: HashMap<String, Vec<String>> = HashMap::new();
    let scan_roots: Vec<PathBuf> = STALE_TICKET_ID_SCAN_ROOTS
        .iter()
        .map(|rel| root.join(rel))
        .collect();
    for base in scan_roots {
        let files: Vec<PathBuf> = if base.is_file() {
            vec![base]
        } else if base.is_dir() {
            WalkDir::new(&base)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            continue;
        };
        for f in files {
            let rel = match f.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if SCAN_EXEMPT_PREFIXES
                .iter()
                .any(|p| rel.starts_with(p) || rel.contains(p))
            {
                continue;
            }
            if rel.ends_with("REORG_CHANGELOG.md") {
                continue;
            }
            let text = match fs::read_to_string(&f) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let matches: Vec<String> = STRICT_RE
                .find_iter(&text)
                .map(|m| m.as_str().to_string())
                .collect();
            if !matches.is_empty() {
                hits.insert(rel, matches);
            }
        }
    }
    hits
}
