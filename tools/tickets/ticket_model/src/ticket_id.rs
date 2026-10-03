//! The ticket identifier and its grammar.
//!
//! **Role:** [`TicketId`], the one type every ticket id, slice id and parent id is held in, with
//! the questions its spelling answers (parent or child, the parent numeral).
//! **Position:** leaf of `ticket_model`; the model, the encoding and the store hold ids as
//! [`TicketId`], and every crate above names a ticket through it.
//! **Signals & state:** none; a plain value type.
//! **Invariants:** an id serialises as its bare string, so every ticket file, `queue.json`,
//! `wave.lock` and receipt carries the id's plain text; it compares, orders and hashes as its
//! string; a parent id is `T-` and ASCII digits only.

newtype_ids::string_id! {
    /// A ticket's identifier: `T-` and a numeral for a parent ticket (`T-<n>`), the parent's id
    /// and a dotted numeral for a child or slice (`T-<n>.<m>`). The file `.ai/tickets/<id>.toml`
    /// holds the ticket; in TOML and JSON the id is its bare string.
    pub struct TicketId;
}

impl TicketId {
    /// Whether this id names a parent ticket: `T-` followed by ASCII digits only. A dotted child
    /// id never does.
    #[must_use]
    pub fn is_parent(&self) -> bool {
        parent_numeral_text(self.as_str()).is_some()
    }

    /// The numeral of a parent id (`T-<n>` → `<n>`); `None` for a child id or any other text.
    #[must_use]
    pub fn parent_number(&self) -> Option<u64> {
        parent_numeral_text(self.as_str())?.parse().ok()
    }
}

/// The digits after `T-` when `text` spells a parent id, else `None`.
pub(crate) fn parent_numeral_text(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("T-")?;
    (!rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())).then_some(rest)
}
