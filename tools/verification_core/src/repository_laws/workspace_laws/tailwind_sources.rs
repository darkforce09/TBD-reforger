//! The Tailwind-sources law: every leptos crate's sources are scanned by the app stylesheet.
//!
//! **Role:** finds every workspace member that depends on `leptos` and checks that an `@source`
//! line of the app stylesheet, resolved relative to the stylesheet's folder, covers that member's
//! `src/**/*.rs`; a member no line covers ships classes Tailwind never generates.
//! **Position:** `cargo xtask verify tailwind-sources` prints [`check_tailwind_sources`]; xtask
//! passes the stylesheet path.
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** a glob covers a member when it ends in `/**/*.rs` and its folder part (whose
//! segments may hold `*`) is the member's `src` folder or an ancestor of it; `@source not` lines
//! never cover. A missing stylesheet is [`NotRun::TargetMissing`].

use std::path::Path;

use super::{LawOutcome, WorkspaceLawReport};
use crate::repository_laws::workspace_members::{read_workspace_members, wildcard_matches};
use crate::verdict::NotRun;

/// The package whose dependents the stylesheet must scan.
pub const LEPTOS_PACKAGE: &str = "leptos";

/// The Tailwind-sources report for the stylesheet at the repository-relative `stylesheet`.
pub fn check_tailwind_sources(repo_root: &Path, stylesheet: &str) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome(
        "tailwind-sources",
        tailwind_sources_outcome(repo_root, stylesheet),
    )
}

/// The findings of the Tailwind-sources law; [`NotRun`] when an input could not be read.
pub fn tailwind_sources_outcome(repo_root: &Path, stylesheet: &str) -> Result<LawOutcome, NotRun> {
    let members = read_workspace_members(repo_root)?;
    let stylesheet_path = repo_root.join(stylesheet);
    if !stylesheet_path.is_file() {
        return Err(NotRun::TargetMissing(stylesheet_path));
    }
    let text = std::fs::read_to_string(&stylesheet_path).map_err(|source| NotRun::Unreadable {
        path: stylesheet_path.clone(),
        source,
    })?;
    let stylesheet_folder = stylesheet.rsplit_once('/').map_or("", |(folder, _)| folder);
    let globs: Vec<String> = source_globs(&text)
        .iter()
        .filter_map(|glob| resolved(stylesheet_folder, glob))
        .collect();
    let leptos_members: Vec<&str> = members
        .iter()
        .filter(|member| {
            member
                .manifest
                .dependencies
                .iter()
                .any(|edge| edge.package == LEPTOS_PACKAGE && !edge.is_dev_dependency())
        })
        .map(|member| member.path.as_str())
        .collect();
    let findings = leptos_members
        .iter()
        .filter(|member| !globs.iter().any(|glob| covers(glob, member)))
        .map(|member| {
            format!(
                "{member} depends on leptos but no @source line of {stylesheet} covers \
                 {member}/src/**/*.rs"
            )
        })
        .collect();
    Ok(LawOutcome {
        summary: format!(
            "{} leptos member(s), {} @source glob(s) in {stylesheet}",
            leptos_members.len(),
            globs.len()
        ),
        findings,
        notes: Vec::new(),
    })
}

/// The quoted glob of every `@source "<glob>";` line, `@source not` lines left out.
pub fn source_globs(stylesheet: &str) -> Vec<String> {
    stylesheet
        .lines()
        .filter_map(|line| line.trim().strip_prefix("@source"))
        .filter(|rest| rest.starts_with([' ', '\t']) && !rest.trim_start().starts_with("not "))
        .filter_map(|rest| {
            let quote = rest.trim_start().chars().next()?;
            matches!(quote, '"' | '\'').then(|| rest.trim_start()[1..].split(quote).next())?
        })
        .map(str::to_string)
        .collect()
}

/// `glob` resolved against the repository-relative `folder`, `..` and `.` folded; `None` when
/// it climbs above the repository root.
fn resolved(folder: &str, glob: &str) -> Option<String> {
    let mut parts: Vec<&str> = folder.split('/').filter(|part| !part.is_empty()).collect();
    for part in glob.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// True when the resolved `glob` covers `member/src/**/*.rs`.
fn covers(glob: &str, member: &str) -> bool {
    let Some(folder) = glob.strip_suffix("/**/*.rs") else {
        return false;
    };
    let source = format!("{member}/src");
    let glob_parts: Vec<&str> = folder.split('/').collect();
    let source_parts: Vec<&str> = source.split('/').collect();
    glob_parts.len() <= source_parts.len()
        && glob_parts
            .iter()
            .zip(&source_parts)
            .all(|(pattern, name)| wildcard_matches(pattern, name))
}

#[cfg(test)]
#[path = "tests/tailwind_sources.rs"]
mod tests;
