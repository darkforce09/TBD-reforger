use super::*;

use std::process::Command;

/// Whether rsync, walking the checkout, excludes `tracked_file` under the anchored `pattern`.
///
/// rsync matches an anchored pattern against the whole path from the transfer root, tests every
/// folder before it descends into it, and reads `*` as any run of characters within one component.
/// A pattern of `k` components therefore meets the file's first `k` components: a folder when the
/// file lies deeper, the file itself when its path has exactly `k`, which a pattern ending in `/`
/// never matches.
fn rsync_excludes(pattern: &str, tracked_file: &str) -> bool {
    let anchored = pattern.strip_prefix('/').expect("an anchored pattern");
    let (body, folders_only) = match anchored.strip_suffix('/') {
        Some(body) => (body, true),
        None => (anchored, false),
    };
    let pattern_components: Vec<&str> = body.split('/').collect();
    let path_components: Vec<&str> = tracked_file.split('/').collect();
    let reaches = if folders_only {
        path_components.len() > pattern_components.len()
    } else {
        path_components.len() >= pattern_components.len()
    };
    reaches
        && pattern_components
            .iter()
            .zip(&path_components)
            .all(|(pattern, component)| wildcard_matches(pattern, component))
}

/// `*` matches any run of characters; every other character matches itself.
fn wildcard_matches(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, rest)) => text.strip_prefix(head).is_some_and(|tail| {
            (0..=tail.len())
                .filter(|&at| tail.is_char_boundary(at))
                .any(|at| wildcard_matches(rest, &tail[at..]))
        }),
    }
}

/// Every tracked path of the checkout, as `git ls-files -z` lists it.
fn tracked_files() -> Vec<String> {
    let root = tool_test_support::test_repo_root();
    let output = Command::new("git")
        .current_dir(&root)
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files");
    assert!(output.status.success(), "git ls-files failed");
    let files: Vec<String> = String::from_utf8(output.stdout)
        .expect("git ls-files output is UTF-8")
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect();
    assert!(
        files.len() > 1000,
        "the checkout tracks {} files, which is too few",
        files.len()
    );
    files
}

/// The tracked-file check below reads each pattern as rsync does only while every pattern is
/// anchored and uses no wildcard but `*`.
#[test]
fn every_development_machine_only_path_is_anchored_and_uses_only_the_star_wildcard() {
    for pattern in DEVELOPMENT_MACHINE_ONLY_PATHS {
        assert!(
            pattern.starts_with('/'),
            "{pattern} is not anchored at the checkout root"
        );
        assert!(
            !pattern.contains("**") && !pattern.contains(['?', '[', '\\']),
            "{pattern} uses a wildcard the tracked-file check does not read"
        );
    }
}

/// The matcher is the tracked-file check's control: it excludes what rsync excludes and keeps
/// what rsync keeps.
#[test]
fn the_matcher_reads_folders_files_and_anchoring_as_rsync_does() {
    assert!(rsync_excludes("/target-*/", "target-container/debug/xtask"));
    assert!(!rsync_excludes("/target-*/", "target-notes.md"));
    assert!(!rsync_excludes(
        "/target-*/",
        "apps/frontend/target-picker/mod.rs"
    ));
    assert!(rsync_excludes("/.mcp.json", ".mcp.json"));
    assert!(!rsync_excludes("/.mcp.json", "apps/.mcp.json"));
    assert!(rsync_excludes(
        "/.claude/worktrees/",
        ".claude/worktrees/slice/Cargo.toml"
    ));
    assert!(rsync_excludes(
        "/.claude/agent-memory-local",
        ".claude/agent-memory-local/reviewer/MEMORY.md"
    ));
    assert!(!rsync_excludes(
        "/.claude/settings.local.json",
        ".claude/settings.json"
    ));
}

/// The host receives the whole tracked tree: no pattern reaches a tracked file, not even
/// `.claude/settings.json`, which sits beside the excluded Claude Code files.
#[test]
fn no_tracked_file_matches_a_development_machine_only_path() {
    let offences: Vec<String> = tracked_files()
        .iter()
        .flat_map(|file| {
            DEVELOPMENT_MACHINE_ONLY_PATHS
                .iter()
                .filter(|pattern| rsync_excludes(pattern, file))
                .map(move |pattern| format!("{pattern} excludes the tracked {file}"))
        })
        .collect();
    assert!(offences.is_empty(), "{offences:#?}");
}

/// The root cargo target folders and the local tool state stay on the list.
#[test]
fn the_list_names_the_root_target_folders_and_the_local_tool_state() {
    for needed in [
        "/target-*/",
        "/.ai/artifacts/worktrees/T-*/",
        "/.claude/settings.local.json",
        "/.claude/worktrees/",
        "/.codex/",
        "/.mcp.json",
        "/.compile-vanilla-baseline",
    ] {
        assert!(
            DEVELOPMENT_MACHINE_ONLY_PATHS.contains(&needed),
            "missing {needed}"
        );
    }
}

#[test]
fn the_exclude_arguments_follow_the_list() {
    let arguments: Vec<String> = exclude_arguments().collect();
    assert_eq!(arguments.len(), DEVELOPMENT_MACHINE_ONLY_PATHS.len());
    for (argument, pattern) in arguments.iter().zip(DEVELOPMENT_MACHINE_ONLY_PATHS) {
        assert_eq!(argument, &format!("--exclude={pattern}"));
    }
}
