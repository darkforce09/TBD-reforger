//! The typed identifiers the agent reads from the game server and writes into its config.
//!
//! **Role:** [`SessionPlayerId`] and [`ArmaPlayerId`], the two ids of a `#players` row, and
//! [`ScenarioId`], the mission header resource a `restart_with_mission` command switches the
//! server config to.
//! **Position:** [`crate::rcon::reforger_commands`] reads the player ids out of an RCON
//! response; [`crate::command_execution`] validates a scenario id out of a claimed command's
//! arguments and [`crate::dedicated_server_config`] writes it into the server config.
//! **Signals & state:** none; plain values.
//! **Invariants:** the player ids serialise exactly as their inner number and text, so the
//! `list_players` outcome is the JSON the ledger has always received; a [`ScenarioId`] exists
//! only for text matching `^\{[0-9A-F]{16}\}[A-Za-z0-9_./-]+\.conf$`, and its text is the exact
//! value the server config stores.

/// Hex digits of the resource GUID between the braces of a scenario id.
const SCENARIO_GUID_HEX_DIGITS: usize = 16;
/// The file suffix of a mission header resource.
const SCENARIO_HEADER_SUFFIX: &str = ".conf";

newtype_ids::integer_id! {
    /// The transient number of a player in the current game session: the first column of a
    /// `#players` row, and the `player_id` of the `list_players` outcome.
    pub struct SessionPlayerId(u32);
}

newtype_ids::string_id! {
    /// A player's Bohemia identity UID: the second column of a `#players` row, and the
    /// `arma_id` of the `list_players` outcome.
    pub struct ArmaPlayerId;
}

/// A mission header resource matching `^\{[0-9A-F]{16}\}[A-Za-z0-9_./-]+\.conf$`, for example
/// `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`: the value of the server config's
/// `game.scenarioId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioId(String);

impl ScenarioId {
    /// The header resource in `raw`, or `None` when `raw` does not match the pattern.
    pub fn parse(raw: &str) -> Option<Self> {
        let (guid, path) = raw.strip_prefix('{')?.split_once('}')?;
        let valid = guid.len() == SCENARIO_GUID_HEX_DIGITS
            && guid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
            && path.len() > SCENARIO_HEADER_SUFFIX.len()
            && path.ends_with(SCENARIO_HEADER_SUFFIX)
            && path.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'/' | b'-')
            });
        valid.then(|| Self(raw.to_owned()))
    }

    /// The header resource as the server config stores it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
