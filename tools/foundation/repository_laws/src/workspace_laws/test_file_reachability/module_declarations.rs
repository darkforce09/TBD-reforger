//! The source files one Rust file's out-of-line module declarations load.
//!
//! **Role:** reads every `mod <name>;` of a source file, with the `#[path = "…"]` attribute
//! (plain or inside `cfg_attr`) that applies to it, and names the files the compiler would load
//! for it, following the module path rules of the Rust reference.
//! **Position:** private to [`super`], which walks a crate's module tree from its targets' root
//! files through [`declared_module_files`].
//! **Signals & state:** none; pure functions over source text and paths.
//! **Invariants:** comments and string literals are blanked before the scan
//! (`rust_module_references::blank_comments_and_strings`), so a declaration quoted in a string or
//! a comment never counts, while the `path` value is read from the original text at the same
//! position. Path rules: a declaration in a file that owns its folder (a crate root, a `mod.rs`
//! or a file loaded through `#[path]`) resolves beside that file, one in any other file
//! `<folder>/<stem>.rs` resolves under `<folder>/<stem>/`; inline `mod <name> { … }` blocks add
//! their name (or their own `path` value) as a folder; a top-level `#[path]` resolves beside the
//! declaring file. A declaration counts whatever `cfg` guards it, since the law asks whether any
//! build reaches a file.

use std::path::{Component, Path, PathBuf};

use regex::Regex;

use super::super::rust_module_references::blank_comments_and_strings;

/// One file a module declaration may load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModuleFile {
    /// The file's path, lexically normalised (no `.` or `..` segment that can be folded).
    pub(super) path: PathBuf,
    /// True when the file owns its folder: its own declarations resolve beside it.
    pub(super) owns_folder: bool,
}

/// The candidate files of every out-of-line module `text` declares; `file` is the declaring
/// file, `owns_folder` whether it owns its folder. A declaration without `#[path]` has two
/// candidates (`<name>.rs` and `<name>/mod.rs`), one with `#[path]` has one.
pub(super) fn declared_module_files(file: &Path, owns_folder: bool, text: &str) -> Vec<ModuleFile> {
    let folder = file.parent().unwrap_or(Path::new(""));
    let own_base = if owns_folder {
        folder.to_path_buf()
    } else {
        let stem = file.file_stem().unwrap_or_default();
        folder.join(stem)
    };
    let path_value = Regex::new(r#"\bpath\s*=\s*"([^"]*)""#).expect("the path pattern compiles");
    let original: Vec<char> = text.chars().collect();
    let code: Vec<char> = blank_comments_and_strings(text).chars().collect();
    let mut files = Vec::new();
    let mut pending_path: Option<String> = None;
    let mut depth = 0usize;
    let mut inline_modules: Vec<(usize, String)> = Vec::new();
    let mut index = 0;
    while index < code.len() {
        let c = code[index];
        if c == '#' {
            if let Some(end) = attribute_end(&code, index) {
                let attribute: String = original[index..=end].iter().collect();
                let inner = code.get(index + 1) == Some(&'!');
                if !inner && let Some(found) = path_value.captures(&attribute) {
                    pending_path = Some(found[1].to_string());
                }
                index = end + 1;
                continue;
            }
        } else if c == '{' {
            depth += 1;
            pending_path = None;
        } else if c == '}' {
            depth = depth.saturating_sub(1);
            while inline_modules.last().is_some_and(|(at, _)| *at == depth) {
                inline_modules.pop();
            }
            pending_path = None;
        } else if c == ';' {
            pending_path = None;
        } else if is_word_start(&code, index) {
            let word = word_at(&code, index);
            if word == "mod"
                && let Some((name, after)) = module_name(&code, index + word.len())
            {
                let folders: PathBuf = inline_modules.iter().map(|(_, s)| s.as_str()).collect();
                match code.get(after) {
                    Some(';') => {
                        let path = pending_path.take();
                        files.extend(candidates(
                            folder,
                            &own_base,
                            &folders,
                            &name,
                            path.as_deref(),
                        ));
                    }
                    Some('{') => {
                        let segment = pending_path.take().unwrap_or(name);
                        inline_modules.push((depth, segment));
                        depth += 1;
                    }
                    _ => {}
                }
                index = after + 1;
                continue;
            }
            index += word.chars().count();
            continue;
        }
        index += 1;
    }
    files
}

/// The files `mod <name>;` may load: `path` resolved beside the declaring file at the top level
/// and under the inline folders otherwise, or `<name>.rs` and `<name>/mod.rs` under the base.
fn candidates(
    folder: &Path,
    own_base: &Path,
    inline_folders: &Path,
    name: &str,
    path: Option<&str>,
) -> Vec<ModuleFile> {
    match path {
        Some(path) if inline_folders.as_os_str().is_empty() => vec![ModuleFile {
            path: normalised(&folder.join(path)),
            owns_folder: true,
        }],
        Some(path) => vec![ModuleFile {
            path: normalised(&own_base.join(inline_folders).join(path)),
            owns_folder: true,
        }],
        None => {
            let base = own_base.join(inline_folders);
            vec![
                ModuleFile {
                    path: normalised(&base.join(format!("{name}.rs"))),
                    owns_folder: false,
                },
                ModuleFile {
                    path: normalised(&base.join(name).join("mod.rs")),
                    owns_folder: true,
                },
            ]
        }
    }
}

/// `path` with every `.` dropped and every `..` folded into the segment before it.
pub(super) fn normalised(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir if out.file_name().is_some() => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The index of the `]` closing the attribute whose `#` sits at `start`, when `#` opens one
/// (`#[` or `#![`).
fn attribute_end(code: &[char], start: usize) -> Option<usize> {
    let mut index = start + 1;
    if code.get(index) == Some(&'!') {
        index += 1;
    }
    while code.get(index).is_some_and(|c| c.is_whitespace()) {
        index += 1;
    }
    if code.get(index) != Some(&'[') {
        return None;
    }
    let mut depth = 0usize;
    for (at, c) in code.iter().enumerate().skip(index) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// True when an identifier starts at `index`: an identifier character not preceded by one (nor
/// by `$`, a macro variable, nor by `'`, a lifetime or label).
fn is_word_start(code: &[char], index: usize) -> bool {
    let starts = code[index].is_ascii_alphabetic() || code[index] == '_';
    let before = index
        .checked_sub(1)
        .map(|at| code[at])
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '\'');
    starts && !before
}

/// The identifier starting at `index`.
fn word_at(code: &[char], index: usize) -> String {
    code[index..]
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == '_')
        .collect()
}

/// The module name after the `mod` keyword ending at `from`, and the index of the first
/// non-blank character after it; `None` when no identifier follows.
fn module_name(code: &[char], from: usize) -> Option<(String, usize)> {
    let mut index = from;
    if !code.get(index).is_some_and(|c| c.is_whitespace()) {
        return None;
    }
    while code.get(index).is_some_and(|c| c.is_whitespace()) {
        index += 1;
    }
    if code[index..].starts_with(&['r', '#']) {
        index += 2;
    }
    if !code
        .get(index)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
    {
        return None;
    }
    let name = word_at(code, index);
    index += name.len();
    while code.get(index).is_some_and(|c| c.is_whitespace()) {
        index += 1;
    }
    Some((name, index))
}

#[cfg(test)]
#[path = "../tests/test_file_reachability_module_declarations.rs"]
mod tests;
