//! Mission row model for the mission settings interface.

use super::*;

/// Game mode values accepted by the mission row API.
pub(super) const GAME_MODES: [(&str, &str); 3] =
    [("pve_coop", "Co-op PvE"), ("pvp", "PvP"), ("zeus", "Zeus")];

/// Checks whether a game mode matches the server accepted values.
pub(super) fn is_known_game_mode(v: &str) -> bool {
    GAME_MODES.iter().any(|(k, _)| *k == v)
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// The row columns edited by the mission settings dialog.
pub(super) struct RowShape {
    pub(super) game_mode: String,
    pub(super) max_players: i64,
    pub(super) briefing: String,
    pub(super) thumbnail_url: String,
}

#[cfg(target_arch = "wasm32")]
impl From<crate::v2::apps::editor::shell::document_commands::HydratedRow> for RowShape {
    fn from(h: crate::v2::apps::editor::shell::document_commands::HydratedRow) -> Self {
        Self {
            game_mode: h.game_mode,
            max_players: h.max_players,
            briefing: h.briefing,
            thumbnail_url: h.thumbnail_url,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// The placed and declared player figures shown in mission settings.
pub(super) struct PlayerCount {
    pub(super) placed: usize,
    pub(super) declared: Option<i64>,
}

impl PlayerCount {
    /// Provides player figure for mission settings.
    pub(super) fn player_figure(self) -> usize {
        self.placed
    }

    /// Provides disagrees for mission settings.
    pub(super) fn disagrees(self) -> bool {
        matches!(self.declared, Some(d) if d != i64::try_from(self.placed).unwrap_or(i64::MAX))
    }
}

/// Explains the derived player count.
pub(super) const SLOTS_PLACED_NOTE: &str = "Counted from the slots placed in this mission — nobody types it. \
                                 A slot is a seat, not a player: this is what the mission contains, \
                                 not how many people your server will hold.";

/// Explains which player figure the editor displays.
pub(super) const PLAYER_COUNT_RULING_NOTE: &str =
    "These two do not match. The editor goes by the slots placed — that is what this mission \
     actually contains. Max players was chosen once, in the create dialog, and nothing has compared \
     it to the mission since. Neither is enforced here.";

/// Explains the declared capacity value.
pub(super) const MAX_PLAYERS_KEPT_NOTE: &str =
    "Declared once, when the mission was created. It is shown because the compiled mission and the \
     library card still carry this figure — not because anything here counts it.";

/// Explains why row controls are unavailable.
pub(super) const SHAPE_UNAVAILABLE_NOTE: &str =
    "The mission row has not loaded, so game mode cannot be changed here. A draft that has never \
     been saved to the library has no row yet.";

/// Formats a failed game mode update.
pub(super) fn game_mode_failure_message(err: &crate::v2::core::api::client::ApiErr) -> String {
    if err.0 == 403 {
        return "Game mode was not saved — you are not this mission's author. It has been put back \
                to the stored value."
            .to_string();
    }
    format!(
        "Could not save the game mode: {}. It has been put back to the stored value.",
        crate::v2::core::api::client::api_error_message(err, "the server did not respond")
    )
}
