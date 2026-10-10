//! How the tools name a ticket: its readable slug, its legacy number, and the shapes a command
//! line may pass.
//!
//! **Role:** the serde-transparent [`TicketSlug`], [`LegacyTicketNumber`] and [`ReceiptName`];
//! [`is_ticket_reference`] says whether a string is safe to treat as a ticket reference (a slug or
//! a legacy `T-<n>[.<n>…]` number), [`is_legacy_ticket_number`] whether it is the legacy shape, and
//! [`parent_slice`] which worktree a nested slice shares.
//! **Position:** the documents of [`crate::ticket_documents`] and [`crate::wave_documents`] carry
//! the newtypes; the wave drivers check argv strings with the predicates before they build a path,
//! a branch name or a `ttm` argument from them.
//! **Signals & state:** none; pure functions over strings.
//! **Invariants:** a string [`is_ticket_reference`] accepts contains no `/`, no whitespace, no
//! empty dot segment and no leading or trailing separator, so it can never leave the folder it is
//! joined onto; the predicates accept a superset of what `ttm` itself mints, never a subset, so a
//! valid ticket is never refused here.

newtype_ids::string_id! {
    /// A ticket's readable slug, such as `slot-identity.flatten-emit`: lowercase words joined by
    /// `-`, a child slice after its parent's slug and a `.`.
    pub struct TicketSlug;
}

newtype_ids::string_id! {
    /// A ticket's number from the file-based registry, such as `T-674.1`; `ttm` resolves it to the
    /// ticket it was imported as.
    pub struct LegacyTicketNumber;
}

newtype_ids::string_id! {
    /// The name `ttm record-run` gives a run receipt, `run-<row>`.
    pub struct ReceiptName;
}

/// Whether `text` is the legacy ticket number shape: `T-` (or `t-`) and dot-separated digit
/// groups, such as `T-674` or `T-674.1.2`.
pub fn is_legacy_ticket_number(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("T-").or_else(|| text.strip_prefix("t-")) else {
        return false;
    };
    !rest.is_empty()
        && rest
            .split('.')
            .all(|group| !group.is_empty() && group.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether `text` names a ticket in a shape the tools accept on a command line: a legacy number
/// ([`is_legacy_ticket_number`]) or a slug of lowercase ASCII letters, digits, `-` and `.`, whose
/// dot segments are non-empty and neither start nor end with `-`.
pub fn is_ticket_reference(text: &str) -> bool {
    if is_legacy_ticket_number(text) {
        return true;
    }
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && text.split('.').all(|segment| {
            !segment.is_empty() && !segment.starts_with('-') && !segment.ends_with('-')
        })
}

/// The slice whose worktree and branch `reference` uses: a reference of three or more dot
/// segments (a sub-slice, `programme.slice.part`) resolves to its first two (`programme.slice`);
/// one of one or two segments is its own slice.
pub fn parent_slice(reference: &str) -> &str {
    match reference.match_indices('.').nth(1) {
        Some((second_dot, _)) => &reference[..second_dot],
        None => reference,
    }
}

#[cfg(test)]
#[path = "tests/ticket_references_tests.rs"]
mod tests;
