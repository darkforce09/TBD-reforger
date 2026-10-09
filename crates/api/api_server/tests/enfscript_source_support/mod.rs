//! Reads the tbd-framework mod scripts as the outline JSON wire parity needs: classes with their
//! banners, fields and JSON key bindings, methods with their bodies, tagged constant groups, and
//! the classes `JsonLoadContext`/`JsonSaveContext` read or write.
//!
//! **Role:** turns `mod/tbd-framework/Scripts/**/*.c` into [`ScriptFile`] outlines, so the
//! contract parity suite judges the mod's wire classes against `contracts/definitions` without
//! an EnfScript compiler.
//!
//! **Position:** compiled into each suite that writes `mod enfscript_source_support;`; read by
//! `tests/contract_parity_mod_wire.rs`. It reads the scripts at test runtime, so a script edit
//! is judged without rebuilding the crate.
//!
//! **Signals & state:** none; pure functions over the files read at call time.
//!
//! **Invariants:** comments and string contents never count as code (braces in a string or a
//! comment never shift the nesting depth); a banner is the run of `//!` lines directly above an
//! item, and a blank line ends it; a class member line the reader does not recognise panics, so
//! an unread field never passes silently; braces balance in every file.

#![allow(dead_code)]

pub(crate) mod contract_tag;

use std::path::{Path, PathBuf};

/// The pinned tbd-framework Scripts root.
pub(crate) fn scripts_root() -> PathBuf {
    repository_root::find_repository_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the repository root above the API package")
        .join("mod/tbd-framework/Scripts")
}

/// One script file's outline.
pub(crate) struct ScriptFile {
    /// Path relative to [`scripts_root`], `/`-separated.
    pub path: String,
    /// The file's full text.
    pub text: String,
    /// Code of the whole file: comments removed, string contents emptied, lines kept.
    pub code: String,
    /// The classes, in file order.
    pub classes: Vec<ScriptClass>,
    /// Line numbers of every `//! @contract` banner line in the file.
    pub contract_tag_lines: Vec<usize>,
}

/// One class declaration.
pub(crate) struct ScriptClass {
    pub line: usize,
    pub name: String,
    pub modded: bool,
    /// The `//!` banner above the class, without the `//!` markers.
    pub banner: Vec<String>,
    pub fields: Vec<ScriptField>,
    pub methods: Vec<ScriptMethod>,
    pub constant_groups: Vec<ConstantGroup>,
}

/// One instance field.
pub(crate) struct ScriptField {
    pub line: usize,
    pub name: String,
    /// The declared type as written, for example `ref array<ref TBD_RosterSlotStruct>`.
    pub type_text: String,
    /// The JSON key the trailing `//!<` doc binds, when it opens with ``JSON key `k` `` or
    /// ``JSON `k` ``. `JsonLoadContext` itself reads the field name.
    pub binding: Option<String>,
}

/// One method with its body.
pub(crate) struct ScriptMethod {
    pub line: usize,
    pub name: String,
    pub banner: Vec<String>,
    /// Raw text from the signature to the closing brace.
    pub body: String,
}

/// A run of constants whose first member carries a banner.
pub(crate) struct ConstantGroup {
    pub line: usize,
    pub banner: Vec<String>,
    pub constants: Vec<ScriptConstant>,
}

/// One `const` member.
pub(crate) struct ScriptConstant {
    pub line: usize,
    pub name: String,
    /// The literal of a string constant.
    pub string_value: Option<String>,
}

/// A `ReadValue`/`WriteValue` call and the class of the value it reads or writes.
pub(crate) struct JsonContextCall {
    pub path: String,
    pub line: usize,
    /// The declared type of the value argument.
    pub value_type: String,
}

/// Every `.c` file under [`scripts_root`], outlined, in path order.
pub(crate) fn read_scripts() -> Vec<ScriptFile> {
    let root = scripts_root();
    assert!(
        root.is_dir(),
        "the mod Scripts root {} is missing",
        root.display()
    );
    let mut paths = Vec::new();
    collect_scripts(&root, &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .c file under {}", root.display());
    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            let relative = path
                .strip_prefix(&root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/");
            outline(&relative, &text)
        })
        .collect()
}

fn collect_scripts(directory: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_scripts(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "c") {
            out.push(path);
        }
    }
}

/// One source line: its code (comments removed, string contents emptied) and its doc comment.
struct SourceLine {
    number: usize,
    raw: String,
    code: String,
    banner: Option<String>,
    trailing: Option<String>,
}

fn split_lines(text: &str) -> Vec<SourceLine> {
    let mut in_block_comment = false;
    let mut lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let chars: Vec<char> = raw.chars().collect();
        let (mut code, mut banner, mut trailing) = (String::new(), None, None);
        let (mut at, mut in_string) = (0, false);
        while at < chars.len() {
            let c = chars[at];
            let next = chars.get(at + 1).copied();
            if in_block_comment {
                if c == '*' && next == Some('/') {
                    in_block_comment = false;
                    at += 1;
                }
            } else if in_string {
                if c == '\\' {
                    at += 1;
                } else if c == '"' {
                    in_string = false;
                    code.push('"');
                }
            } else if c == '"' {
                in_string = true;
                code.push('"');
            } else if c == '/' && next == Some('*') {
                in_block_comment = true;
                at += 1;
            } else if c == '/' && next == Some('/') {
                let rest: String = chars[at..].iter().collect();
                if let Some(text) = rest.strip_prefix("//!<") {
                    trailing = Some(text.trim().to_owned());
                } else if let Some(text) = rest.strip_prefix("//!") {
                    banner = Some(text.trim().to_owned());
                }
                break;
            } else {
                code.push(c);
            }
            at += 1;
        }
        lines.push(SourceLine {
            number: index + 1,
            raw: raw.to_owned(),
            code,
            banner,
            trailing,
        });
    }
    lines
}

fn brace_delta(code: &str) -> i32 {
    code.chars().fold(0, |delta, c| match c {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
}

fn is_identifier(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !word.starts_with(|c: char| c.is_ascii_digit())
}

/// `class Name`, `modded class Name`, `sealed class Name`, with or without a base and a brace.
fn class_header(code: &str) -> Option<(String, bool)> {
    let mut words = code.split_whitespace().peekable();
    let mut modded = false;
    while let Some(word) = words.peek() {
        match *word {
            "modded" => modded = true,
            "sealed" => {}
            _ => break,
        }
        words.next();
    }
    if words.next() != Some("class") {
        return None;
    }
    let name: String = words
        .next()?
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    is_identifier(&name).then_some((name, modded))
}

/// A `const` member: `[protected|private] [static] const TYPE NAME = VALUE;`.
fn constant_line(line: &SourceLine) -> Option<ScriptConstant> {
    let code = line.code.trim();
    let words: Vec<&str> = code.split_whitespace().collect();
    let const_at = words.iter().position(|word| *word == "const")?;
    let modifiers_only = words[..const_at]
        .iter()
        .all(|word| matches!(*word, "static" | "protected" | "private"));
    if !modifiers_only || !code.ends_with(';') {
        return None;
    }
    let name = words.get(const_at + 2)?.trim_end_matches(';').to_owned();
    let string_value = line.raw.split_once('=').and_then(|(_, value)| {
        let value = value.trim_start();
        let literal = value.strip_prefix('"')?;
        literal.find('"').map(|end| literal[..end].to_owned())
    });
    Some(ScriptConstant {
        line: line.number,
        name,
        string_value,
    })
}

/// The name of the method a signature line opens, or `None` for anything else.
fn method_name(code: &str) -> Option<String> {
    if code.ends_with(';') || code.starts_with('[') {
        return None;
    }
    let open = code.find('(')?;
    let name: String = code[..open]
        .trim_end()
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    (is_identifier(&name) && !matches!(name.as_str(), "if" | "for" | "while" | "switch"))
        .then_some(name)
}

/// The instance fields a declaration line declares; `None` when the line is not one.
fn field_line(code: &str, trailing: Option<&str>, number: usize) -> Option<Vec<ScriptField>> {
    let declaration = code.strip_suffix(';')?.trim();
    let declaration = declaration
        .split_once('=')
        .map_or(declaration, |(left, _)| left)
        .trim();
    let mut rest = declaration;
    loop {
        let (word, tail) = rest.split_once(char::is_whitespace)?;
        match word {
            "protected" | "private" | "ref" | "autoptr" | "owned" => rest = tail.trim_start(),
            "static" | "const" | "return" => return None,
            _ => break,
        }
    }
    let mut depth = 0;
    let split = rest.char_indices().find(|(_, c)| {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            c if c.is_whitespace() && depth == 0 => return true,
            _ => {}
        }
        false
    })?;
    let (type_text, names) = (rest[..split.0].trim(), rest[split.0..].trim());
    let names: Vec<&str> = names.split(',').map(str::trim).collect();
    if !names.iter().all(|name| is_identifier(name)) {
        return None;
    }
    let binding = trailing.and_then(json_key_binding);
    Some(
        names
            .iter()
            .map(|name| ScriptField {
                line: number,
                name: (*name).to_owned(),
                type_text: type_text.to_owned(),
                binding: binding.clone().filter(|_| names.len() == 1),
            })
            .collect(),
    )
}

/// The key of a trailing doc that opens with ``JSON key `k` `` or ``JSON `k` ``.
fn json_key_binding(trailing: &str) -> Option<String> {
    let rest = trailing.strip_prefix("JSON")?.trim_start();
    let rest = rest.strip_prefix("key").map_or(rest, str::trim_start);
    let quoted = rest.strip_prefix('`')?;
    quoted.find('`').map(|end| quoted[..end].to_owned())
}

/// What the depth-0 item above the current line is.
#[derive(PartialEq)]
enum TopLevel {
    Class,
    Other,
}

/// The outline of one file; `path` is only used in messages.
pub(crate) fn outline(path: &str, text: &str) -> ScriptFile {
    let lines = split_lines(text);
    let mut classes: Vec<ScriptClass> = Vec::new();
    let mut contract_tag_lines = Vec::new();
    let (mut depth, mut top_level) = (0i32, TopLevel::Other);
    let mut banner: Vec<String> = Vec::new();
    let mut method: Option<ScriptMethod> = None;
    let (mut method_opened, mut group_open) = (false, false);
    for line in &lines {
        if line
            .banner
            .as_deref()
            .is_some_and(|text| text.starts_with("@contract"))
        {
            contract_tag_lines.push(line.number);
        }
        let code = line.code.trim();
        if let Some(open) = method.as_mut() {
            open.body.push_str(&line.raw);
            open.body.push('\n');
            depth += brace_delta(&line.code);
            method_opened |= line.code.contains('{');
            if method_opened && depth <= 1 {
                let finished = method.take().expect("open method");
                classes
                    .last_mut()
                    .expect("method inside a class")
                    .methods
                    .push(finished);
            }
            continue;
        }
        if code.is_empty() {
            if let Some(text) = &line.banner {
                banner.push(text.clone());
            } else if line.raw.trim().is_empty() {
                banner.clear();
                group_open = false;
            }
            continue;
        }
        let taken = std::mem::take(&mut banner);
        if depth == 0 && !code.starts_with('{') {
            top_level = match class_header(code) {
                Some((name, modded)) => {
                    classes.push(ScriptClass {
                        line: line.number,
                        name,
                        modded,
                        banner: taken,
                        fields: Vec::new(),
                        methods: Vec::new(),
                        constant_groups: Vec::new(),
                    });
                    TopLevel::Class
                }
                None => TopLevel::Other,
            };
        } else if depth == 1 && top_level == TopLevel::Class && !matches!(code, "{" | "}" | "};") {
            let class = classes.last_mut().expect("a class is open");
            if let Some(constant) = constant_line(line) {
                if !taken.is_empty() {
                    class.constant_groups.push(ConstantGroup {
                        line: line.number,
                        banner: taken,
                        constants: vec![constant],
                    });
                    group_open = true;
                } else if group_open {
                    let group = class.constant_groups.last_mut().expect("open group");
                    group.constants.push(constant);
                }
            } else {
                group_open = false;
                if let Some(name) = method_name(code) {
                    method = Some(ScriptMethod {
                        line: line.number,
                        name,
                        banner: taken,
                        body: format!("{}\n", line.raw),
                    });
                    depth += brace_delta(&line.code);
                    method_opened = line.code.contains('{');
                    if method_opened && depth <= 1 {
                        class.methods.push(method.take().expect("open method"));
                    }
                    continue;
                }
                let skipped = code.starts_with('[')
                    || code.split_whitespace().any(|word| word == "static")
                    || (code.contains('(') && code.ends_with(';'));
                match field_line(code, line.trailing.as_deref(), line.number) {
                    Some(fields) => class.fields.extend(fields),
                    None if skipped => {}
                    None => panic!(
                        "{path}:{}: unrecognised member of class `{}`: `{code}`",
                        line.number, class.name
                    ),
                }
            }
        }
        depth += brace_delta(&line.code);
    }
    assert!(
        depth == 0 && method.is_none(),
        "{path}: braces do not balance (depth {depth} at the end)"
    );
    let code = lines
        .iter()
        .map(|line| line.code.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    ScriptFile {
        path: path.to_owned(),
        text: text.to_owned(),
        code,
        classes,
        contract_tag_lines,
    }
}

/// Every `ReadValue(key, value)` and `WriteValue(key, value)` call in `file`, with the declared
/// type of `value`: the nearest declaration above the call, else the first one in the file.
///
/// Panics when a value argument has no declaration in the file, so no call escapes judgement.
pub(crate) fn json_context_calls(file: &ScriptFile) -> Vec<JsonContextCall> {
    let mut calls = Vec::new();
    for pattern in ["ReadValue(", "WriteValue("] {
        for (offset, _) in file.code.match_indices(pattern) {
            let arguments = &file.code[offset + pattern.len()..];
            let close = arguments.find(')').expect("closed call");
            let Some((_, value)) = arguments[..close].split_once(',') else {
                continue;
            };
            let value = value.trim();
            assert!(
                is_identifier(value),
                "{}: ReadValue into `{value}`",
                file.path
            );
            let line = file.code[..offset].matches('\n').count() + 1;
            let value_type = declared_type(&file.code, value, offset).unwrap_or_else(|| {
                panic!(
                    "{}:{line}: no declaration of `{value}` in the file",
                    file.path
                )
            });
            calls.push(JsonContextCall {
                path: file.path.clone(),
                line,
                value_type,
            });
        }
    }
    calls
}

/// The type in the nearest `TYPE name` declaration above `before`, else the first below it.
fn declared_type(code: &str, name: &str, before: usize) -> Option<String> {
    let mut above = None;
    let mut below = None;
    for (offset, _) in code.match_indices(name) {
        let preceding = code[..offset].chars().next_back();
        let following = code[offset + name.len()..].trim_start().chars().next();
        let whole_word = !preceding.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if !whole_word || !matches!(following, Some('=' | ';' | ',' | ')')) {
            continue;
        }
        let head = code[..offset].trim_end();
        let type_name: String = head
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if !is_identifier(&type_name) || matches!(type_name.as_str(), "return" | "new" | "ref") {
            continue;
        }
        if offset < before {
            above = Some(type_name);
        } else if below.is_none() {
            below = Some(type_name);
        }
    }
    above.or(below)
}

/// The JSON object keys a method writes by hand: every `\"key\":` inside its string literals.
pub(crate) fn hand_built_json_keys(method: &ScriptMethod) -> Vec<String> {
    let mut keys = Vec::new();
    for (offset, _) in method.body.match_indices("\\\"") {
        let rest = &method.body[offset + 2..];
        let key: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if is_identifier(&key) && rest[key.len()..].starts_with("\\\":") && !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

/// The script classes named in a field's declared type.
pub(crate) fn type_class_names(type_text: &str) -> Vec<String> {
    type_text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| is_identifier(word))
        .map(str::to_owned)
        .collect()
}
