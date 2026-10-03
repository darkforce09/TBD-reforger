//! Whether an account counts as linked to an Arma identity.
//!
//! **Role:** the one reading of `users.arma_id` that every session claim and profile shares.
//! **Position:** called by session issuance, rotation and authorization and by the profile and
//! link status handlers.
//! **Signals & state:** none; a pure function.
//! **Invariants:** a missing, empty or whitespace-only id is not linked; the value is read, never
//! written back, so the trim cannot change which rows an identity join matches.

use api_identifiers::ArmaPlayerId;

/// True when `arma_id` is present **and** non-whitespace after trim.
///
/// Read-side only — does not mutate storage. A row can hold `Some("   ")`, and
/// `Option::is_some()` alone reports that as linked: the JWT claim and the SPA gates then
/// say LINKED while every resolve path, which binds the trimmed id, finds nothing.
///
/// Trimming is safe *here*: the flag is derived for the claim, never written back, and it
/// takes no part in the byte-identity joins (`orbat_reservations.squad` / `or_fallback`)
/// where a one-sided trim would change which rows match.
pub fn arma_id_is_linked(arma_id: &Option<ArmaPlayerId>) -> bool {
    arma_id
        .as_ref()
        .is_some_and(|id| !id.as_str().trim().is_empty())
}

#[cfg(test)]
#[path = "tests/arma_identity_link.rs"]
mod tests;
