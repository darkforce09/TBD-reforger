//! **Role:** whether a mission id names a row the server can hold.
//! **Position:** `mission_id` in `mission_persistence`; the Mission Creator's draft shell calls it.
//! **Signals & state:** none; a pure shape test over the id's bytes.
//! **Invariants:** only the canonical 8-4-4-4-12 hexadecimal form is a server id. Every other
//! spelling — a fixture name, an ad-hoc local id — names a document that exists nowhere but this
//! machine, and the whole server reconciliation is SKIPPED for it rather than attempted against an
//! endpoint that has no such row. That skip is what keeps a local-only document off a network path
//! whose only possible answer is 404.

use mission_model::ids::MissionId;

/// Is `mission_id` a mission id the API can have a row for?
///
/// The test is positional rather than a parse: exactly 36 bytes, hyphens at 8/13/18/23, and every
/// other byte an ASCII hex digit. Nothing here interprets the version or variant nibbles, because
/// the question is "would the server recognise this as an id", not "which flavour of UUID is it".
#[must_use]
pub fn is_uuid(mission_id: impl Into<MissionId>) -> bool {
    let mission_id = mission_id.into();
    let b = mission_id.as_str().as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

#[cfg(test)]
#[path = "tests/mission_id.rs"]
mod tests;
