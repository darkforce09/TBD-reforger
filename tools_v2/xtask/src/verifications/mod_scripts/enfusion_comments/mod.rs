//! `cargo xtask verify enfusion-comments`: the in-code documentation card over Enfusion scripts.
//!
//! **Role:** walks the `.c` scripts under the pinned mod roots, or under `--path`, runs the nine
//! card rules ECM-1 to ECM-9 on each, prints one `<rule> <path>:<line> <message>` line per finding
//! and the per-rule counts, and returns the exit code.
//!
//! **Position:** called by `tools_v2/xtask/src/commands/verify/dispatch.rs`; reads scripts under
//! `apps/mod/`; each rule lives in its own sibling module and sees a [`CheckedScript`].
//!
//! **Signals & state:** none; reads files, writes stdout.
//!
//! **Invariants:**
//! - Fail closed: no roots, a root outside `apps/mod`, a missing root, an unreadable file or a walk
//!   that finds no `.c` file is did-not-run (exit 2), never a clean pass.
//! - Exit 0 clean, 1 findings, 2 did not run.
//! - Output order is deterministic: files sorted by path, findings by line, then rule.

mod ascii_rule;
mod attribute_description_rule;
mod boundary_tag_rule;
mod checked_script;
mod context_free_prose_rule;
mod declaration_banner_rule;
mod file_header_rule;
mod findings;
mod network_authority_rule;
mod primary_type_rule;
mod script_outline;
mod trailing_member_doc_rule;

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use verification_core::NotRun;
use verification_core::scan::{walk_files, with_extension};

use checked_script::CheckedScript;
use findings::{Finding, RuleId};

/// Repository-relative roots judged when no `--path` is given; each entry is a folder or file
/// under [`MOD_TREE`] whose scripts meet the card.
pub(crate) const PINNED_ROOTS: &[&str] = &[];

/// The only tree `--path` may point into.
const MOD_TREE: &str = "apps/mod";

/// Runs every rule over the scripts under `paths` (repository-relative or absolute, each under
/// `apps/mod`), or under [`PINNED_ROOTS`] when `paths` is empty.
///
/// Returns 0 when every script is clean, 1 when a rule found a violation, 2 when the check could
/// not run; prints the report to stdout either way.
pub(crate) fn verify_enfusion_comments(repo: &Path, paths: &[String]) -> u8 {
    let scripts = match collect_scripts(repo, paths) {
        Ok(scripts) => scripts,
        Err(reason) => {
            println!("enfusion-comments: did not run: {reason}");
            return 2;
        }
    };
    let mut counts: BTreeMap<RuleId, usize> = RuleId::ALL.iter().map(|rule| (*rule, 0)).collect();
    let mut offending_files = 0;
    for path in &scripts {
        let source = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => {
                println!(
                    "enfusion-comments: did not run: unreadable {}: {error}",
                    path.display()
                );
                return 2;
            }
        };
        let display = path
            .strip_prefix(repo)
            .unwrap_or(path)
            .display()
            .to_string();
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let found = check_script(&file_name, &source);
        if !found.is_empty() {
            offending_files += 1;
        }
        for finding in found {
            println!(
                "{} {display}:{} {}",
                finding.rule, finding.line, finding.message
            );
            *counts.entry(finding.rule).or_default() += 1;
        }
    }
    let total: usize = counts.values().sum();
    println!(
        "enfusion-comments: {total} finding(s) in {offending_files} of {} script(s)",
        scripts.len()
    );
    for (rule, count) in &counts {
        println!("  {rule} {count}");
    }
    u8::from(total > 0)
}

/// Runs every rule over one script called `file_name` whose text is `source`.
///
/// Returns the findings sorted by line, then rule; never fails.
pub(crate) fn check_script(file_name: &str, source: &str) -> Vec<Finding> {
    let script = CheckedScript::new(file_name, source);
    let rules: [fn(&CheckedScript) -> Vec<Finding>; 9] = [
        ascii_rule::check,
        file_header_rule::check,
        declaration_banner_rule::check,
        trailing_member_doc_rule::check,
        network_authority_rule::check,
        boundary_tag_rule::check,
        attribute_description_rule::check,
        context_free_prose_rule::check,
        primary_type_rule::check,
    ];
    let mut found: Vec<Finding> = rules.iter().flat_map(|rule| rule(&script)).collect();
    found.sort();
    found
}

/// The sorted `.c` files under the requested roots, or why the check cannot run.
fn collect_scripts(repo: &Path, paths: &[String]) -> Result<Vec<PathBuf>, String> {
    let requested: Vec<String> = if paths.is_empty() {
        PINNED_ROOTS
            .iter()
            .map(|root| (*root).to_string())
            .collect()
    } else {
        paths.to_vec()
    };
    if requested.is_empty() {
        return Err("no roots: the pinned root list is empty and no --path was given".to_string());
    }
    let mod_tree = repo.join(MOD_TREE);
    let mut roots = Vec::new();
    for raw in &requested {
        let candidate = Path::new(raw);
        let root = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            repo.join(candidate)
        };
        let escapes = root.components().any(|part| part == Component::ParentDir);
        if escapes || !root.starts_with(&mod_tree) {
            return Err(format!("`{raw}` is not under {MOD_TREE}"));
        }
        roots.push(root);
    }
    let root_refs: Vec<&Path> = roots.iter().map(PathBuf::as_path).collect();
    let scripts =
        walk_files(&root_refs, with_extension(&["c"])).map_err(|reason| match reason {
            NotRun::TargetMissing(path) => format!("missing root {}", path.display()),
            NotRun::Unreadable { path, source } => {
                format!("unreadable {}: {source}", path.display())
            }
            other => format!("{other:?}"),
        })?;
    if scripts.is_empty() {
        return Err(format!(
            "the walk found no .c file under {}",
            requested.join(", ")
        ));
    }
    Ok(scripts)
}

#[cfg(test)]
#[path = "../tests/enfusion_comments_tests.rs"]
mod tests;
