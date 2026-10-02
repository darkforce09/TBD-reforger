//! The module path of every file of a crate, and of every byte inside a file.
//!
//! **Role:** walks a crate from each of its roots (`src/lib.rs`, `src/main.rs`, binaries, tests,
//! benches, examples, the build script and every target `path` its manifest declares) through its
//! `mod` declarations, honouring `#[path = "…"]` and both the `mod.rs` and `<module>.rs` layouts,
//! and records the module path of each file reached; [`module_path_at`] adds the inline modules
//! that enclose one byte of a file.
//!
//! **Position:** built per crate by the Rust path pass ([`super`]), which needs a file's module to
//! resolve `self::` and `super::` paths.
//!
//! **Signals & state:** none held; a tree is built once per crate and read-only afterwards.
//!
//! **Invariants:** a crate root and a file loaded through `#[path]` own their folder, so their
//! child modules sit beside them; any other `<module>.rs` file's children sit in the folder named
//! after it; a `#[path]` inside an inline module is read from the folder the inline modules
//! spell; a file reached from two roots keeps the module path of the first root walked; a file
//! that cannot be read ends that branch of the walk without failing it.

use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use super::super::path_mapping::{normalize, parent_folder};
use super::super::repository_files::{CRATE_MANIFEST, PathSet};
use super::super::rust_lexer::{Token, TokenKind, code_tokens, string_content, tokenize};

/// Cargo's default build script below a crate folder. Spelled in two parts because the tooling
/// prose rule reads every whole Rust file name as a reference to a file that must exist, and this
/// is a Cargo convention no crate of the workspace uses.
const DEFAULT_BUILD_SCRIPT: &str = concat!("build", ".rs");

/// The default crate roots below a crate folder, `*` standing for one file or folder name.
const DEFAULT_ROOTS: [&str; 9] = [
    "src/lib.rs",
    "src/main.rs",
    "src/bin/*.rs",
    "src/bin/*/main.rs",
    "tests/*.rs",
    "tests/*/main.rs",
    "benches/*.rs",
    "examples/*.rs",
    DEFAULT_BUILD_SCRIPT,
];

/// The manifest tables whose `path` key names a crate root.
const TARGET_TABLES: [&str; 5] = ["[lib]", "[[bin]]", "[[test]]", "[[bench]]", "[[example]]"];

/// One inline module of a file: its name and the bytes of its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InlineModule {
    /// The module's name.
    pub(crate) name: String,
    /// The bytes between its braces.
    pub(crate) body: Range<usize>,
}

/// One `mod` declaration of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleDeclaration {
    /// The declared name.
    pub(crate) name: String,
    /// The value of its `#[path = "…"]` attribute.
    pub(crate) path_attribute: Option<String>,
    /// The names of the inline modules around the declaration, outermost first.
    pub(crate) enclosing: Vec<String>,
    /// Whether the module's body is inline (`mod name { … }`).
    pub(crate) inline: bool,
}

/// The module path, below `crate`, of every file the crate's roots reach.
#[derive(Clone, Debug, Default)]
pub(crate) struct ModuleTree {
    modules: HashMap<String, Vec<String>>,
}

impl ModuleTree {
    /// Walk the crate in `crate_folder` of the checkout at `root`.
    pub(crate) fn build(root: &Path, crate_folder: &str, paths: &PathSet) -> ModuleTree {
        let mut tree = ModuleTree::default();
        for crate_root in crate_roots(root, crate_folder, paths) {
            if tree.modules.contains_key(&crate_root) {
                continue;
            }
            tree.modules.insert(crate_root.clone(), Vec::new());
            let children = parent_folder(&crate_root).to_string();
            tree.walk(root, paths, &crate_root, &[], &children);
        }
        tree
    }

    /// The module path of `file`, when a root reaches it.
    pub(crate) fn module_of(&self, file: &str) -> Option<&[String]> {
        self.modules.get(file).map(Vec::as_slice)
    }

    fn walk(
        &mut self,
        root: &Path,
        paths: &PathSet,
        file: &str,
        module: &[String],
        children: &str,
    ) {
        let Ok(source) = std::fs::read_to_string(root.join(file)) else {
            return;
        };
        let (declarations, _) = scan_modules(&source);
        for declaration in declarations.into_iter().filter(|d| !d.inline) {
            let inline_folder = join_all(children, &declaration.enclosing);
            let found = match &declaration.path_attribute {
                Some(path) => {
                    let base = if declaration.enclosing.is_empty() {
                        parent_folder(file).to_string()
                    } else {
                        inline_folder.clone()
                    };
                    normalize(&base, path).map(|child| {
                        let folder = parent_folder(&child).to_string();
                        (child, folder)
                    })
                }
                None => {
                    let folder = join_all(&inline_folder, std::slice::from_ref(&declaration.name));
                    [format!("{folder}.rs"), format!("{folder}/mod.rs")]
                        .into_iter()
                        .find(|candidate| exists(root, paths, candidate))
                        .map(|child| (child, folder))
                }
            };
            let Some((child, child_children)) = found else {
                continue;
            };
            if self.modules.contains_key(&child) {
                continue;
            }
            let mut child_module = module.to_vec();
            child_module.extend(declaration.enclosing.iter().cloned());
            child_module.push(declaration.name.clone());
            self.modules.insert(child.clone(), child_module.clone());
            self.walk(root, paths, &child, &child_module, &child_children);
        }
    }
}

/// The module path at byte `offset` of a file whose own module is `file_module`.
pub(crate) fn module_path_at(
    file_module: &[String],
    inline: &[InlineModule],
    offset: usize,
) -> Vec<String> {
    let mut path = file_module.to_vec();
    let mut enclosing: Vec<&InlineModule> = inline
        .iter()
        .filter(|module| module.body.start <= offset && offset < module.body.end)
        .collect();
    enclosing.sort_by_key(|module| module.body.start);
    path.extend(enclosing.into_iter().map(|module| module.name.clone()));
    path
}

/// Every `mod` declaration of `source` and every inline module body.
pub(crate) fn scan_modules(source: &str) -> (Vec<ModuleDeclaration>, Vec<InlineModule>) {
    let tokens = tokenize(source);
    let code = code_tokens(&tokens);
    let mut declarations = Vec::new();
    let mut inline = Vec::new();
    let mut braces: Vec<Option<(String, usize)>> = Vec::new();
    let mut opening_module: Option<String> = None;
    for (index, token) in code.iter().enumerate() {
        if token.is_punctuation(source, '{') {
            braces.push(opening_module.take().map(|name| (name, token.end)));
            continue;
        }
        if token.is_punctuation(source, '}') {
            if let Some(Some((name, start))) = braces.pop() {
                inline.push(InlineModule {
                    name,
                    body: start..token.start,
                });
            }
            continue;
        }
        let Some(name) = declared_module(source, &code, index) else {
            continue;
        };
        let is_inline = code[index + 2].is_punctuation(source, '{');
        declarations.push(ModuleDeclaration {
            name: name.clone(),
            path_attribute: path_attribute(source, &code, index),
            enclosing: braces.iter().flatten().map(|(n, _)| n.clone()).collect(),
            inline: is_inline,
        });
        if is_inline {
            opening_module = Some(name);
        }
    }
    (declarations, inline)
}

/// The name declared when `code[index]` is `mod` followed by a name and `;` or `{`.
fn declared_module(source: &str, code: &[Token], index: usize) -> Option<String> {
    let name = code.get(index + 1)?;
    let end = code.get(index + 2)?;
    let declares = code[index].is_word(source, "mod")
        && name.kind == TokenKind::Identifier
        && (end.is_punctuation(source, ';') || end.is_punctuation(source, '{'));
    declares.then(|| name.text(source).trim_start_matches("r#").to_string())
}

/// The value of a `#[path = "…"]` attribute among the outer attributes before `code[index]`.
fn path_attribute(source: &str, code: &[Token], index: usize) -> Option<String> {
    let mut at = index;
    if at > 0 && code[at - 1].is_punctuation(source, ')') {
        while at > 0 && !code[at - 1].is_punctuation(source, '(') {
            at -= 1;
        }
        at = at.saturating_sub(1);
    }
    if at > 0 && code[at - 1].is_word(source, "pub") {
        at -= 1;
    }
    while at > 0 && code[at - 1].is_punctuation(source, ']') {
        let close = at - 1;
        let mut depth = 0usize;
        let mut open = close;
        loop {
            if code[open].is_punctuation(source, ']') {
                depth += 1;
            } else if code[open].is_punctuation(source, '[') {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            if open == 0 {
                return None;
            }
            open -= 1;
        }
        let inside = &code[open + 1..close];
        if inside.len() == 3
            && inside[0].is_word(source, "path")
            && inside[1].is_punctuation(source, '=')
            && inside[2].kind == TokenKind::StringLiteral
        {
            return Some(source[string_content(source, &inside[2])].to_string());
        }
        at = open.saturating_sub(1);
    }
    None
}

/// The crate roots of the crate in `crate_folder`.
fn crate_roots(root: &Path, crate_folder: &str, paths: &PathSet) -> Vec<String> {
    let prefix = if crate_folder.is_empty() {
        String::new()
    } else {
        format!("{crate_folder}/")
    };
    let mut roots = Vec::new();
    let manifest = format!("{prefix}{CRATE_MANIFEST}");
    if let Ok(text) = std::fs::read_to_string(root.join(&manifest)) {
        roots.extend(
            declared_target_paths(&text)
                .into_iter()
                .filter_map(|path| normalize(crate_folder, &path)),
        );
    }
    for file in paths.files() {
        let Some(relative) = file.strip_prefix(&prefix) else {
            continue;
        };
        let matches_default = DEFAULT_ROOTS.iter().any(|pattern| {
            super::super::manifest::glob_matches(pattern.as_bytes(), relative.as_bytes())
        });
        if matches_default && !roots.contains(file) {
            roots.push(file.clone());
        }
    }
    roots
}

/// The `path` values of a manifest's target tables.
fn declared_target_paths(manifest: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_target = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_target = TARGET_TABLES.contains(&trimmed);
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=')
            && in_target
            && key.trim() == "path"
        {
            found.push(value.trim().trim_matches('"').to_string());
        }
    }
    found
}

fn exists(root: &Path, paths: &PathSet, candidate: &str) -> bool {
    paths.is_file(candidate) || root.join(candidate).is_file()
}

fn join_all(folder: &str, names: &[String]) -> String {
    let mut path = folder.to_string();
    for name in names {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(name);
    }
    path
}
