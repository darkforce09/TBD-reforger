//! The workspace crates a Rust source file re-exports publicly.
//!
//! **Role:** reads every public re-export statement of a source text — `pub use` and
//! `pub extern crate` — and names the crate each root of its use tree starts from, whatever the
//! statement's shape: `pub use <crate>::…`, `pub use <crate>;`, an alias (`pub use <crate> as x;`),
//! a leading `::`, a group at the top or nested (`pub use {<crate> as x};`,
//! `pub use {{<crate>, x}};`, `pub use <crate>::{self as x};`), a statement spread over lines
//! (`pub use` alone on its line included) and `pub extern crate <crate> as x;`.
//! **Position:** private to [`super`]; the crate-anatomy law reads re-exports outside a crate's
//! prelude module through [`reexported_crates`].
//! **Signals & state:** none; pure functions over source text.
//! **Invariants:** a statement is read from its first line to its `;` with line comments dropped,
//! and every finding cites that first line; restricted visibility (`pub(crate)`, `pub(super)`,
//! `pub(in …)`), a private `use` and a line comment re-export nothing; `crate`, `self`, `super`
//! and `Self` lead no crate.

/// Every crate a public re-export statement of `text` names, with the 1-based first line of its
/// statement: each root of its use tree, whether the statement sits on one line or spreads over
/// several.
pub(super) fn reexported_crates(text: &str) -> Vec<(usize, String)> {
    reexport_statements(text)
        .into_iter()
        .flat_map(|(line_no, statement)| {
            statement_crates(&statement)
                .into_iter()
                .map(|name| (line_no, name.to_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every public re-export statement of `text` with its 1-based first line, its lines joined by
/// spaces and their `//` comments dropped, up to its `;` (or the end of the text).
fn reexport_statements(text: &str) -> Vec<(usize, String)> {
    let mut statements = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (index, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        match open.as_mut() {
            Some((_, statement)) => {
                statement.push(' ');
                statement.push_str(code);
            }
            None if public_reexport_tree(code).is_some() => {
                open = Some((index + 1, code.to_string()));
            }
            None => continue,
        }
        if code.contains(';') {
            statements.extend(open.take());
        }
    }
    statements.extend(open);
    statements
}

/// Every crate one public re-export statement names as the first segment of a use path: the one
/// path of `pub use <crate>::…;`, `pub use <crate>;`, `pub use <crate> as x;` and
/// `pub use ::<crate> as x;`, each item of a group at the top or nested
/// (`pub use {<crate> as x, {y, z}};`), and the crate of `pub extern crate <crate> as x;`. Any
/// other statement, restricted visibility included, names none.
fn statement_crates(statement: &str) -> Vec<&str> {
    let Some(tree) = public_reexport_tree(statement) else {
        return Vec::new();
    };
    tree_crates(tree.split(';').next().unwrap_or(""))
}

/// The crates one use tree starts from: one for a path, one per item of a group, nested groups
/// read item by item.
fn tree_crates(tree: &str) -> Vec<&str> {
    let tree = tree.trim_start();
    let tree = tree.strip_prefix("::").unwrap_or(tree).trim_start();
    match tree.strip_prefix('{') {
        Some(group) => top_level_items(group)
            .into_iter()
            .flat_map(tree_crates)
            .collect(),
        None => leading_crate(tree).into_iter().collect(),
    }
}

/// The use tree after `pub use` or `pub extern crate`; `None` for any other text, a restricted
/// visibility such as `pub(crate)` included.
fn public_reexport_tree(text: &str) -> Option<&str> {
    let rest = text.trim_start().strip_prefix("pub")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    if let Some(tree) = after_keyword(rest, "use") {
        return Some(tree);
    }
    after_keyword(after_keyword(rest, "extern")?, "crate")
}

/// The text after `keyword` when it stands as a whole word at the start of `text`, the end of
/// the line included (a statement whose tree starts on the next line).
fn after_keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    (rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == '{' || c == ':'))
        .then(|| rest.trim_start())
}

/// The comma-separated items of a group whose `{` is already consumed, up to its closing `}`;
/// nested groups stay inside their item.
fn top_level_items(group: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in group.char_indices() {
        match c {
            '{' => depth += 1,
            '}' if depth == 0 => {
                items.push(&group[start..index]);
                return items;
            }
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(&group[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    items.push(&group[start..]);
    items
}

/// The first path segment of one use-tree item when it can name a crate.
fn leading_crate(item: &str) -> Option<&str> {
    let item = item.trim_start();
    let item = item.strip_prefix("::").unwrap_or(item).trim_start();
    let end = item
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(item.len());
    let name = &item[..end];
    let rest = item[end..].trim_start();
    let starts_like_a_name = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let ends_a_segment = rest.is_empty()
        || rest.starts_with("::")
        || rest.starts_with([';', ',', '}'])
        || rest
            .strip_prefix("as")
            .is_some_and(|after| after.starts_with(char::is_whitespace));
    (starts_like_a_name && ends_a_segment && !["crate", "self", "super", "Self"].contains(&name))
        .then_some(name)
}
