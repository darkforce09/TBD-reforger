//! Why a side-level authoring command refuses.
//!
//! **Role:** the error of the crate's two side-level commands,
//! [`crate::apply_faction::apply_faction_library`] and
//! [`crate::place_orbat::place_character_under_side`].
//! **Position:** returned through [`Result`]; the entity commands and the map engine's hosted
//! commands show its `Display` text to the operator.
//! **Signals & state:** none.
//! **Invariants:** a refusal writes nothing to the document; the `Display` text is one readable
//! paragraph naming the side and, for a collapse, every authored squad that blocks it.

use crate::apply_faction::{AuthoredSquad, plural};
use mission_document::ids::SquadId;

/// Why a side-level authoring command refuses.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The side was not `BLUFOR`, `OPFOR` or `INDFOR`.
    #[error("invalid side {0:?}; expected BLUFOR|OPFOR|INDFOR")]
    InvalidSide(String),

    /// Applying a faction template would fold squads the operator authored into one squad.
    #[error("{}", collapse_text(.side, .squad_ids, .blocking, *.slots_at_risk))]
    WouldCollapseSquads {
        /// The side the template was applied onto.
        side: String,

        /// Every squad of the side, in authored order.
        squad_ids: Vec<SquadId>,

        /// The squads beyond the first whose ORBAT reads as authored.
        blocking: Vec<AuthoredSquad>,

        /// The slots the blocking squads hold.
        slots_at_risk: usize,
    },
}

/// The result of a side-level authoring command.
pub type Result<T> = std::result::Result<T, Error>;

/// The operator-facing refusal of a collapsing faction apply.
fn collapse_text(
    side: &str,
    squad_ids: &[SquadId],
    blocking: &[AuthoredSquad],
    slots_at_risk: usize,
) -> String {
    let named = blocking
        .iter()
        .map(|b| format!("\"{}\" ({})", b.name, b.why))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "refusing to apply a template onto {side}: it has {} squads, {} of them \
         holding ORBAT you authored — {named} — and a faction template is a flat role \
         list with no squad level. Applying folds the whole side into one squad, so \
         those squad boundaries, names, callsigns and vehicles would be gone for good: \
         Save-as-template never captured them, so nothing can put them back. Nothing \
         was changed — those squads, and the {} in them, are exactly as you left them, \
         leaders, callsigns, ranks and map positions included. Merge them into one \
         squad or delete them, then apply again. (Squads a map placement created and \
         you never edited are folded in automatically; these are not those.)",
        squad_ids.len(),
        blocking.len(),
        plural(slots_at_risk, "slot"),
    )
}
