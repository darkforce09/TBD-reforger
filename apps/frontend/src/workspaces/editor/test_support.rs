//! Test-only support for the editor's source pins.
//!
//! **Role:** hands a source pin the production half of one editor source file, so a pin that
//! reads raw text (comments and literals kept) never searches its own test module.
//! **Position:** compiled only into the test build; called from the editor's test files, which
//! pass it the text of an `include_str!`.
//! **Signals & state:** none; pure functions over `&str`.
//! **Invariants:** the production half ends at the file's test-module declaration — a
//! `#[cfg(test)]` whose item, after any further attributes and an optional visibility, is a
//! `mod`. A `#[cfg(test)]` on any other item is a test-only helper inside the production half and
//! never ends it. A file without a test-module declaration is returned whole.

/// The production half of `src`: everything before its test-module declaration.
///
/// The declaration is `#[cfg(test)]` followed (on the same line or the next ones) by further
/// attributes and then `mod`, as in `#[cfg(test)] #[path = "tests/…"] mod tests;`. A test-gated
/// `use`, `const` or `fn` above the declaration stays in the returned text.
pub(crate) fn production_half(src: &str) -> &str {
    let mut offset = 0;
    for line in src.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(after) = trimmed.strip_prefix("#[cfg(test)]") {
            let rest = &src[offset + line.len()..];
            if item_is_module(after, rest) {
                return &src[..offset];
            }
        }
        offset += line.len();
    }
    src
}

/// Whether the item that follows an attribute is a module, skipping further attributes and
/// comment lines. `same_line` is the text after the attribute on its own line and `following` the
/// text of the lines after it.
fn item_is_module(same_line: &str, following: &str) -> bool {
    std::iter::once(same_line)
        .chain(following.lines())
        .map(strip_leading_attributes)
        .find(|piece| !piece.is_empty() && !piece.starts_with("//"))
        .is_some_and(|item| {
            let item = item.strip_prefix("pub").map_or(item, |rest| {
                let rest = rest.trim_start();
                rest.strip_prefix('(')
                    .and_then(|scoped| scoped.split_once(')'))
                    .map_or(rest, |(_, after)| after)
            });
            item.trim_start().starts_with("mod ")
        })
}

/// One line with its leading `#[…]` attributes removed, so an attribute and the item it annotates
/// can share a line.
fn strip_leading_attributes(line: &str) -> &str {
    let mut rest = line.trim();
    while rest.starts_with("#[") {
        let Some(close) = rest.find(']') else {
            break;
        };
        rest = rest[close + 1..].trim();
    }
    rest
}

#[cfg(test)]
#[path = "tests/test_support.rs"]
mod tests;
