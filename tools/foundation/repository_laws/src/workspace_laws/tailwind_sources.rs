//! The Tailwind-sources law: the app stylesheet names every leptos crate's sources, one line each.
//!
//! **Role:** finds every workspace member that depends on `leptos` (outside dev-dependencies) and
//! checks that exactly one `@source` line of the app stylesheet, resolved relative to the
//! stylesheet's folder, names that member's `src/**/*.rs`; a member no line names ships classes
//! Tailwind never generates. A line that names no leptos member's sources is stale.
//! **Position:** `cargo xtask verify tailwind-sources` prints [`check_tailwind_sources`]; xtask
//! passes the stylesheet path.
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** a line names a member when, `..` and `.` folded, its glob is exactly
//! `<member>/src/**/*.rs`: an ancestor folder or a wildcard folder names no member. A member
//! named by no line or by several lines is a finding, and so is every line naming no leptos
//! member (a glob climbing above the repository root included); `@source not` lines are never
//! read. A missing stylesheet is [`NotRun::TargetMissing`].

use std::path::Path;

use super::{LawOutcome, WorkspaceLawReport};
use crate::workspace_members::read_workspace_members;
use verification_core::verdict::NotRun;

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
    let globs: Vec<(String, Option<String>)> = source_globs(&text)
        .into_iter()
        .map(|glob| {
            let resolved = resolved(stylesheet_folder, &glob);
            (glob, resolved)
        })
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
    let mut findings = Vec::new();
    for member in &leptos_members {
        let source = member_source_glob(member);
        let naming = globs
            .iter()
            .filter(|(_, resolved)| resolved.as_deref() == Some(source.as_str()))
            .count();
        match naming {
            0 => findings.push(format!(
                "{member} depends on leptos but no @source line of {stylesheet} names \
                 {source} (one line per crate, its own src folder)"
            )),
            1 => {}
            lines => findings.push(format!(
                "{lines} @source lines of {stylesheet} name {source}; keep one"
            )),
        }
    }
    findings.extend(
        globs
            .iter()
            .filter(|(_, resolved)| {
                !leptos_members
                    .iter()
                    .any(|member| resolved.as_deref() == Some(member_source_glob(member).as_str()))
            })
            .map(|(glob, _)| {
                format!(
                    "@source \"{glob}\" in {stylesheet} names no leptos member's \
                     src/**/*.rs: a stale line"
                )
            }),
    );
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

/// The one glob that names the sources of the member at `member`: `<member>/src/**/*.rs`.
fn member_source_glob(member: &str) -> String {
    format!("{member}/src/**/*.rs")
}

#[cfg(test)]
#[path = "tests/tailwind_sources.rs"]
mod tests;
