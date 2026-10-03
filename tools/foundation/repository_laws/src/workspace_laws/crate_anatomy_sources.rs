//! The source half of the crate-anatomy law.
//!
//! **Role:** reads a library crate's production sources and finds: a `lib.rs` over 80 lines or
//! holding anything but doc comments, attributes, `mod` lines and `pub use` lines; a missing
//! `pub mod prelude`; a fallible public API (`pub fn … -> Result`) without an `error.rs` holding a
//! `thiserror` `pub enum Error` and a `pub type Result`; a primitive-typed public `id` / `*_id`
//! field or `pub fn` parameter; and a `pub use` of another workspace crate outside the crate's
//! prelude module.
//! **Position:** private to [`super`], called by [`super::crate_anatomy`] per library crate.
//! **Signals & state:** none; reads files.
//! **Invariants:** test files ([`crate::source_roots::is_test_file`]) are never
//! judged; files under a `generated` folder and `#[wasm_bindgen]` items are exempt from the typed
//! id rule only. A missing `lib.rs` or `src` folder is [`NotRun::TargetMissing`].

use std::path::Path;

use regex::Regex;

use super::strangler::reexported_crate;
use crate::source_roots::{is_test_file, repository_relative};
use crate::workspace_members::WorkspaceMember;
use verification_core::scan;
use verification_core::verdict::NotRun;

/// The most lines a `lib.rs` holds.
pub(crate) const LIB_RS_MAX_LINES: usize = 80;

/// A public function, up to its name.
const PUBLIC_FN: &str = r"\bpub\s+(?:const\s+|async\s+|unsafe\s+)*fn\s+\w+";
/// A primitive type, optionally borrowed or optional, right after a `:`.
const PRIMITIVE_TYPE: &str =
    r"\s*:\s*(?:Option<\s*)?(?:&\s*(?:'[a-z_]+\s+)?)?(?:String|str|[ui](?:8|16|32|64|128|size))\b";

/// Every source finding of the library crate `member`.
pub(super) fn source_findings(
    repo_root: &Path,
    member: &WorkspaceMember,
    members: &[WorkspaceMember],
) -> Result<Vec<String>, NotRun> {
    let crate_root = repo_root.join(&member.path);
    let lib_rel = member
        .manifest
        .library
        .as_ref()
        .and_then(|lib| lib.path.clone())
        .unwrap_or_else(|| "src/lib.rs".to_string());
    let lib_path = crate_root.join(&lib_rel);
    let lib_text = read(&lib_path)?;
    let lib_label = format!("{}/{lib_rel}", member.path);
    let mut findings = lib_rs_findings(&lib_label, &lib_text);
    let source_root = crate_root.join("src");
    let files = scan::walk_files(&[source_root.as_path()], scan::with_extension(&["rs"]))?;
    let other_crates: Vec<String> = members
        .iter()
        .filter(|other| other.package_name != member.package_name)
        .map(WorkspaceMember::crate_identifier)
        .collect();
    let public_fn = Regex::new(PUBLIC_FN).expect("the public-fn pattern compiles");
    let fallible = Regex::new(&format!(r"{PUBLIC_FN}[^{{;]*?->\s*(?:[\w:]+::)?Result\b"))
        .expect("the fallible pattern compiles");
    let mut fallible_at: Option<String> = None;
    for file in &files {
        let rel = repository_relative(repo_root, file);
        if is_test_file(&rel) {
            continue;
        }
        let text = read(file)?;
        if fallible_at.is_none()
            && let Some(found) = fallible.find(&text)
        {
            fallible_at = Some(format!("{rel}:{}", line_of(&text, found.start())));
        }
        if !rel.split('/').any(|part| part == "generated") {
            findings.extend(primitive_id_findings(&rel, &text, &public_fn));
        }
        if !is_prelude_module(&rel) {
            for (index, line) in text.lines().enumerate() {
                if let Some(name) = reexported_crate(line)
                    && other_crates.iter().any(|other| other == name)
                {
                    findings.push(format!(
                        "{rel}:{}: `pub use {name}::…` outside the prelude module re-exports \
                         another workspace crate",
                        index + 1
                    ));
                }
            }
        }
    }
    if let Some(at) = fallible_at {
        findings.extend(error_module_findings(&crate_root, &member.path, &at)?);
    }
    Ok(findings)
}

/// The `lib.rs` rules over its text.
fn lib_rs_findings(label: &str, text: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let count = text.lines().count();
    if count > LIB_RS_MAX_LINES {
        findings.push(format!(
            "{label}: {count} lines; lib.rs holds at most {LIB_RS_MAX_LINES}"
        ));
    }
    let module_line = Regex::new(r"^(?:pub(?:\([a-z: ]+\))?\s+)?mod\s+\w+\s*;$")
        .expect("the mod-line pattern compiles");
    // A multi-line attribute ends on a line ending in `]`; a multi-line `pub use` on its `;`.
    let mut continuation: Option<char> = None;
    let mut has_prelude = false;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if let Some(terminator) = continuation {
            let ends = match terminator {
                ';' => line.contains(';'),
                _ => line.ends_with(terminator),
            };
            if ends {
                continuation = None;
            }
            continue;
        }
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if line.starts_with("#[") || line.starts_with("#![") {
            if line.matches('[').count() > line.matches(']').count() {
                continuation = Some(']');
            }
        } else if line.starts_with("pub use ") {
            if !line.ends_with(';') {
                continuation = Some(';');
            }
        } else if module_line.is_match(line) {
            has_prelude |= line == "pub mod prelude;";
        } else {
            findings.push(format!(
                "{label}:{}: lib.rs holds only doc comments, attributes, mod and pub use lines: \
                 `{line}`",
                index + 1
            ));
        }
    }
    if !has_prelude {
        findings.push(format!("{label}: no `pub mod prelude;`"));
    }
    findings
}

/// An `error.rs` beside `lib.rs` with a `thiserror` `pub enum Error` and a `pub type Result`.
fn error_module_findings(
    crate_root: &Path,
    crate_rel: &str,
    fallible_at: &str,
) -> Result<Vec<String>, NotRun> {
    let error_path = crate_root.join("src/error.rs");
    let why = format!("the public API is fallible ({fallible_at})");
    if !error_path.is_file() {
        return Ok(vec![format!("{crate_rel}/src/error.rs is missing; {why}")]);
    }
    let text = read(&error_path)?;
    let mut findings = Vec::new();
    let needs = [
        ("thiserror", "derives its Error with thiserror"),
        ("pub enum Error", "declares `pub enum Error`"),
        ("pub type Result", "declares `pub type Result`"),
    ];
    for (needle, rule) in needs {
        if !text.contains(needle) {
            findings.push(format!("{crate_rel}/src/error.rs never {rule}; {why}"));
        }
    }
    Ok(findings)
}

/// Every primitive-typed public `id` / `*_id` field and `pub fn` parameter in `text`, outside
/// `#[wasm_bindgen]` items.
fn primitive_id_findings(rel: &str, text: &str, public_fn: &Regex) -> Vec<String> {
    let field = Regex::new(&format!(r"\bpub\s+(?:id|[a-z0-9_]+_id){PRIMITIVE_TYPE}"))
        .expect("the id-field pattern compiles");
    let parameter = Regex::new(&format!(r"\b(?:id|[a-z0-9_]+_id){PRIMITIVE_TYPE}"))
        .expect("the id-parameter pattern compiles");
    let lines: Vec<&str> = text.lines().collect();
    let mut findings = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("//") || in_wasm_bindgen_item(&lines, index) {
            continue;
        }
        if field.is_match(line) {
            findings.push(format!(
                "{rel}:{}: a public id field is a primitive; give it a newtype id",
                index + 1
            ));
        }
    }
    for found in public_fn.find_iter(text) {
        let line_index = line_of(text, found.start()) - 1;
        if in_wasm_bindgen_item(&lines, line_index) {
            continue;
        }
        let signature = parameter_list(&text[found.end()..]);
        if parameter.is_match(signature) {
            findings.push(format!(
                "{rel}:{}: a pub fn takes a primitive id parameter; give it a newtype id",
                line_index + 1
            ));
        }
    }
    findings
}

/// The text between the first `(` of `rest` and its matching `)`.
fn parameter_list(rest: &str) -> &str {
    let Some(open) = rest.find('(') else {
        return "";
    };
    let mut depth = 0;
    for (offset, character) in rest[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[open + 1..open + offset];
                }
            }
            _ => {}
        }
    }
    &rest[open..]
}

/// True when the item at `index`, or the `impl` / `struct` that encloses it, carries a
/// `#[wasm_bindgen]` attribute.
fn in_wasm_bindgen_item(lines: &[&str], index: usize) -> bool {
    let indent = |line: &str| line.len() - line.trim_start().len();
    if attributes_above(lines, index).any(|line| line.contains("wasm_bindgen")) {
        return true;
    }
    let own = indent(lines[index]);
    for at in (0..index).rev() {
        let line = lines[at];
        let code = line.trim_start();
        if indent(line) < own && (code.starts_with("impl") || code.contains("struct ")) {
            return code.contains("wasm_bindgen")
                || attributes_above(lines, at).any(|above| above.contains("wasm_bindgen"));
        }
    }
    false
}

/// The attribute, doc-comment and comment lines directly above `index`, nearest first.
fn attributes_above<'a>(lines: &'a [&'a str], index: usize) -> impl Iterator<Item = &'a str> + 'a {
    lines[..index]
        .iter()
        .rev()
        .map(|line| line.trim_start())
        .take_while(|line| line.starts_with("#[") || line.starts_with("//"))
}

/// True when the source file `rel` is a crate's prelude module: a file whose stem is `prelude`.
fn is_prelude_module(rel: &str) -> bool {
    rel.strip_suffix(".rs")
        .is_some_and(|stem| stem.ends_with("/prelude"))
}

/// The 1-based line of byte `offset` in `text`.
fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
}

/// The text of `path`; missing is [`NotRun::TargetMissing`], unreadable [`NotRun::Unreadable`].
fn read(path: &Path) -> Result<String, NotRun> {
    if !path.is_file() {
        return Err(NotRun::TargetMissing(path.to_path_buf()));
    }
    std::fs::read_to_string(path).map_err(|source| NotRun::Unreadable {
        path: path.to_path_buf(),
        source,
    })
}
