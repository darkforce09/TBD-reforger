//! Role: the physical key a stored draft record lives under, and how to read one back.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none; pure string arithmetic over an owner token and a logical key.
//! Invariants: the mapping `(owner, logical) -> key` is INJECTIVE for arbitrary owner bytes, so
//! one account's drafts can never be read as another's. Every key this module writes carries an
//! owner; a key that parses to no owner is a record nothing can attribute, and a caller is told so
//! rather than handed it, because handing an unattributable draft to whoever opens the mission
//! next is exactly the loss the scoping exists to prevent.

/// The owner token a record is filed under when nobody is signed in.
///
/// Signed-out editing is a real state, not a defect: the editor route carries no auth guard and
/// the editor smoke suite drives it logged out. Those drafts get their own namespace rather than a
/// bare unowned key, so the store holds exactly one key shape.
pub const ANONYMOUS_OWNER: &str = "anon";

/// The owner token to file a record under, given whatever account is signed in right now.
///
/// The host resolves the account — it is the host that holds a session. What belongs here is the
/// fallback, because [`ANONYMOUS_OWNER`] is a namespace of this module's key space and not a fact
/// about anybody's session.
#[must_use]
pub fn owner_token_or_anonymous(signed_in_account: Option<&str>) -> String {
    signed_in_account.map_or_else(|| ANONYMOUS_OWNER.to_string(), str::to_string)
}

/// The physical key prefix owning `owner`. Every key under it belongs to that account and to no
/// other, which is what makes purging one account a prefix scan rather than a guess.
///
/// The length prefix is not decoration: it makes the mapping `(owner, logical) -> key`
/// **injective** for arbitrary owner bytes. A plain `{owner}|{logical}` join collides the moment
/// an id contains the separator, and an account id is whatever the identity provider sends, not a
/// shape this module gets to assume. With the length in front, the owner segment is read by count
/// and can hold anything.
#[must_use]
pub fn owner_prefix(owner: &str) -> String {
    format!("u{}:{owner}|", owner.len())
}

/// The physical key for one caller-facing (logical) key under one account.
///
/// Callers pass the logical key they already have — the mission id for the live draft, the mission
/// id plus a snapshot suffix for the pre-adopt / pre-restore pair — and the account is applied
/// here. That is why every record kind is scoped by this one function and none of them has to know
/// the others exist.
#[must_use]
pub fn scoped_key(owner: &str, logical: &str) -> String {
    format!("{}{logical}", owner_prefix(owner))
}

/// Parse a physical key back into `(owner, logical)`, or `None` when it carries no owner at all.
///
/// That `None` is the only test for an unattributable record, so it is exact rather than a prefix
/// guess: read `u`, the decimal length, `:`, exactly that many bytes of owner, then `|`. `str::get`
/// returns `None` on a non-boundary index, so a multi-byte owner can never be sliced apart.
#[must_use]
pub fn split_scoped_key(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix('u')?;
    let (len_digits, rest) = rest.split_once(':')?;
    if len_digits.is_empty() || !len_digits.bytes().all(|b| b.is_ascii_digit()) {
        return None; // `+3` / `` / `0x2` are not the canonical form this module writes
    }
    let len: usize = len_digits.parse().ok()?;
    let owner = rest.get(..len)?;
    let logical = rest.get(len..)?.strip_prefix('|')?;
    Some((owner, logical))
}

#[cfg(test)]
#[path = "tests/record_key.rs"]
mod tests;
