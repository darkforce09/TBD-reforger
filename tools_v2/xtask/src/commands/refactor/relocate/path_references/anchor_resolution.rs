//! Resolving a relative path literal against its anchors, and re-relativising it after the moves.
//!
//! **Role:** answers, for one relative literal in one file, which tracked path it names and from
//! which anchor (the file's own folder, the owning crate's manifest folder, or the repository
//! root), then computes the literal that names the same path, from the same anchor, once the
//! manifest's moves are made.
//!
//! **Position:** called by [`super`] for every candidate [`super::relative_references`] finds;
//! reads the tracked paths before and after the moves and the [`PathMapping`].
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** a literal resolves fully when the whole literal names a tracked path, partly
//! when only a leading part of its named segments does (a gitignored or planned tail); a full
//! resolution outranks every partial one; a literal that resolves under two anchors to different
//! places is ambiguous, and an error only when the two readings rewrite it differently; the new
//! literal keeps the old one's `./` lead, trailing `/` and leading `/`; a literal whose meaning
//! the moves do not change is never rewritten.

use super::super::path_mapping::{PathMapping, normalize, parent_folder, relative_path};
use super::super::repository_files::PathSet;

/// A folder a relative literal may be resolved against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnchorKind {
    /// The folder holding the file.
    FileFolder,
    /// The manifest folder of the crate holding the file (`CARGO_MANIFEST_DIR`, and the working
    /// folder a crate's binaries and tests run in).
    CrateFolder,
    /// The repository root.
    RepositoryRoot,
    /// Every crate folder of the checkout, read only when no anchor before it names a tracked
    /// path, and kept only where it names a path the moves move: prose that names a path from
    /// another crate's working folder (`../../../data`, said of a server that runs in its crate
    /// folder).
    EveryCrateFolder,
}

impl AnchorKind {
    /// The anchor's name in a report.
    pub(crate) fn label(self) -> &'static str {
        match self {
            AnchorKind::FileFolder => "the file's folder",
            AnchorKind::CrateFolder => "the crate folder",
            AnchorKind::RepositoryRoot => "the repository root",
            AnchorKind::EveryCrateFolder => "a crate folder",
        }
    }
}

/// The trees and mapping one file's literals are judged against.
pub(crate) struct ResolutionContext<'a> {
    /// The file holding the literals, before the moves.
    pub(crate) file: &'a str,
    /// The tracked paths before the moves.
    pub(crate) before: &'a PathSet,
    /// The tracked paths after the moves.
    pub(crate) after: &'a PathSet,
    /// The manifest's moves.
    pub(crate) mapping: &'a PathMapping,
    /// Every crate folder before the moves, for [`AnchorKind::EveryCrateFolder`].
    pub(crate) crate_folders: &'a [String],
}

/// What the moves do to one literal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReferenceOutcome {
    /// The literal names no tracked path under any of its anchors.
    NotAReference,
    /// The literal still names the same path after the moves.
    Unchanged,
    /// The literal must become `replacement`; `row_line` names the row that moved its target or
    /// its anchor.
    Rewritten {
        replacement: String,
        row_line: usize,
    },
    /// The literal resolved before the moves and has no single rewrite.
    Unresolvable(String),
}

/// One reading of a literal.
#[derive(Clone, Debug)]
struct Resolution {
    anchor: AnchorKind,
    anchor_folder: String,
    /// The tracked path the literal names, or the tracked part of it.
    target: String,
    /// The untracked rest of the literal below `target`, `""` when it resolved fully.
    tail: String,
    /// How many named segments resolved; full resolutions resolve all of them.
    depth: usize,
    full: bool,
}

/// The outcome of the moves for `literal`, read under `anchors` in order.
pub(crate) fn resolve_and_rewrite(
    literal: &str,
    leading_slash: bool,
    anchors: &[AnchorKind],
    context: &ResolutionContext<'_>,
) -> ReferenceOutcome {
    let readings = readings(literal, anchors, context);
    if readings.is_empty() {
        return ReferenceOutcome::NotAReference;
    }
    let mut outcomes: Vec<(Resolution, ReferenceOutcome)> = readings
        .into_iter()
        .map(|reading| {
            let outcome = rewrite_reading(literal, leading_slash, &reading, context);
            (reading, outcome)
        })
        .collect();
    let first = outcomes[0].1.clone();
    if outcomes
        .iter()
        .all(|(_, outcome)| same_effect(outcome, &first))
    {
        return first;
    }
    outcomes.sort_by_key(|(reading, _)| reading.anchor_folder.clone());
    let places: Vec<String> = outcomes
        .iter()
        .map(|(reading, _)| format!("{} under {}", reading.target, reading.anchor.label()))
        .collect();
    ReferenceOutcome::Unresolvable(format!(
        "`{literal}` resolves to {} and the moves rewrite those readings differently",
        places.join(" and ")
    ))
}

/// Every reading of `literal` worth keeping: the full ones when any exist, otherwise the deepest
/// partial ones.
fn readings(
    literal: &str,
    anchors: &[AnchorKind],
    context: &ResolutionContext<'_>,
) -> Vec<Resolution> {
    let mut seen_folders = Vec::new();
    let mut found = Vec::new();
    for anchor in anchors {
        let folders = match anchor {
            AnchorKind::EveryCrateFolder if found.is_empty() => context.crate_folders.to_vec(),
            AnchorKind::EveryCrateFolder => Vec::new(),
            _ => vec![anchor_before(*anchor, context)],
        };
        for folder in folders {
            if seen_folders.contains(&folder) {
                continue;
            }
            seen_folders.push(folder.clone());
            let reading = read_under(literal, *anchor, &folder, context.before);
            let fallback_reads_unmoved = *anchor == AnchorKind::EveryCrateFolder
                && reading
                    .as_ref()
                    .is_some_and(|r| context.mapping.move_of(&r.target).is_none());
            if let Some(reading) = reading.filter(|_| !fallback_reads_unmoved) {
                found.push(reading);
            }
        }
    }
    if found.iter().any(|reading| reading.full) {
        found.retain(|reading| reading.full);
    } else if let Some(deepest) = found.iter().map(|reading| reading.depth).max() {
        found.retain(|reading| reading.depth == deepest);
    }
    found
}

/// The literal's reading under one anchor folder, if it names a tracked path there.
fn read_under(
    literal: &str,
    anchor: AnchorKind,
    folder: &str,
    before: &PathSet,
) -> Option<Resolution> {
    let segments: Vec<&str> = literal.split('/').filter(|s| !s.is_empty()).collect();
    let lead = segments
        .iter()
        .take_while(|s| **s == "." || **s == "..")
        .count();
    let named = &segments[lead..];
    if named.contains(&"..") {
        let target = normalize(folder, literal)?;
        return (before.contains(&target) && !target.is_empty()).then(|| Resolution {
            anchor,
            anchor_folder: folder.to_string(),
            target,
            tail: String::new(),
            depth: named.len(),
            full: true,
        });
    }
    let base = normalize(folder, &segments[..lead].join("/"))?;
    if named.is_empty() {
        let only_climbs = anchor == AnchorKind::FileFolder && literal.contains('/');
        return (only_climbs && before.contains(&base)).then(|| Resolution {
            anchor,
            anchor_folder: folder.to_string(),
            target: base,
            tail: String::new(),
            depth: 0,
            full: true,
        });
    }
    (1..=named.len()).rev().find_map(|depth| {
        let candidate = join(&base, &named[..depth].join("/"));
        before.contains(&candidate).then(|| Resolution {
            anchor,
            anchor_folder: folder.to_string(),
            target: candidate,
            tail: named[depth..].join("/"),
            depth,
            full: depth == named.len(),
        })
    })
}

/// What the moves do to one reading of `literal`.
fn rewrite_reading(
    literal: &str,
    leading_slash: bool,
    reading: &Resolution,
    context: &ResolutionContext<'_>,
) -> ReferenceOutcome {
    let new_anchor = anchor_after(reading, context);
    let relocated_target = context.mapping.relocate(&reading.target).into_owned();
    let new_target = join(&relocated_target, &reading.tail);
    if normalize(&new_anchor, literal).as_deref() == Some(new_target.as_str()) {
        return ReferenceOutcome::Unchanged;
    }
    if reading.full && !context.after.contains(&relocated_target) {
        return ReferenceOutcome::Unresolvable(format!(
            "`{literal}` names {} before the moves and nothing after them",
            reading.target
        ));
    }
    let mut replacement = relative_path(&new_anchor, &new_target);
    if literal.starts_with("./") && !replacement.starts_with("..") && replacement != "." {
        replacement = format!("./{replacement}");
    }
    if literal.ends_with('/') && !replacement.ends_with('/') {
        replacement.push('/');
    }
    if leading_slash && replacement == "." {
        replacement.clear();
    }
    let row_line = context
        .mapping
        .move_of(&reading.target)
        .or_else(|| context.mapping.move_of(context.file))
        .map_or(0, |found| found.row_line);
    ReferenceOutcome::Rewritten {
        replacement,
        row_line,
    }
}

/// The anchor folder before the moves.
fn anchor_before(anchor: AnchorKind, context: &ResolutionContext<'_>) -> String {
    let folder = parent_folder(context.file);
    match anchor {
        AnchorKind::FileFolder => folder.to_string(),
        AnchorKind::CrateFolder => context.before.crate_folder_of(folder),
        AnchorKind::RepositoryRoot | AnchorKind::EveryCrateFolder => String::new(),
    }
}

/// The anchor folder after the moves: the moved file's folder, or the crate holding the moved
/// file.
fn anchor_after(reading: &Resolution, context: &ResolutionContext<'_>) -> String {
    let moved_file = context.mapping.relocate(context.file);
    let folder = parent_folder(&moved_file);
    match reading.anchor {
        AnchorKind::FileFolder => folder.to_string(),
        AnchorKind::CrateFolder => context.after.crate_folder_of(folder),
        AnchorKind::RepositoryRoot => String::new(),
        AnchorKind::EveryCrateFolder => context
            .mapping
            .relocate(&reading.anchor_folder)
            .into_owned(),
    }
}

/// Whether two outcomes leave the literal with the same text, whichever row they name.
fn same_effect(left: &ReferenceOutcome, right: &ReferenceOutcome) -> bool {
    match (left, right) {
        (
            ReferenceOutcome::Rewritten { replacement: a, .. },
            ReferenceOutcome::Rewritten { replacement: b, .. },
        ) => a == b,
        _ => left == right,
    }
}

fn join(folder: &str, rest: &str) -> String {
    match (folder.is_empty(), rest.is_empty()) {
        (true, _) => rest.to_string(),
        (false, true) => folder.to_string(),
        (false, false) => format!("{folder}/{rest}"),
    }
}
