//! The Arma Reforger RCON commands this agent sends, and the reading of their responses.
//!
//! Arma Reforger's own RCON server (the `rcon` block of the server config, UDP port 19999 by
//! default) speaks the BattlEye RCon protocol but executes the game's server commands, not
//! BattlEye's Arma 3 command set. The commands used here, and the sources behind them:
//!
//! - `#players` lists the session's players with their playerId. It is a standard Arma
//!   Reforger server command (Bohemia Interactive wiki, "Arma Reforger:Server Management",
//!   Commands, players) and one RCON monitor clients may run, since it changes nothing. Each
//!   player row reads `<playerId> ; <identity UID> ; <name>` under a `Players on server:`
//!   header, the format community RCON tools parse; any other line is kept verbatim in the
//!   outcome, so an unexpected format is visible instead of silently dropped.
//! - `@logout` is the custom RCON command that de-authenticates the client at once and frees
//!   its slot (same wiki page, Custom RCON Commands); without it the server drops the client
//!   after its 45 second timeout.
//!
//! Both are reads or session housekeeping, so sending one again after a lost answer is
//! harmless. Reforger's RCON has no broadcast command, so broadcasts run in the game runtime.

use serde_json::{Map, Value, json};

/// Lists the session's players and their playerId.
pub const PLAYERS_COMMAND: &str = "#players";

/// De-authenticates this RCON client at once and frees its slot on the server.
pub const SESSION_LOGOUT_COMMAND: &str = "@logout";

/// Longest identity accepted in a player row: a Bohemia identity UID is a 36-character UUID.
const IDENTITY_MAX_BYTES: usize = 64;

/// One row of the `#players` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedPlayer {
    /// The transient number of the player in this session.
    pub player_id: u32,
    /// The player's Bohemia identity UID.
    pub arma_id: String,
    pub name: String,
}

/// The `#players` response read into rows. `raw_lines` holds every non-empty line of the
/// response when some line was neither a player row nor the listing's header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerListing {
    pub players: Vec<ListedPlayer>,
    pub raw_lines: Option<Vec<String>>,
}

impl PlayerListing {
    /// The list_players outcome of the ledger: `{"players": [{player_id, arma_id, name}]}`,
    /// with `raw_lines` when the response was only partly understood.
    pub fn into_outcome(self) -> Map<String, Value> {
        let players = self
            .players
            .into_iter()
            .map(|player| {
                json!({
                    "player_id": player.player_id,
                    "arma_id": player.arma_id,
                    "name": player.name,
                })
            })
            .collect();
        let mut outcome = Map::new();
        outcome.insert("players".to_owned(), Value::Array(players));
        if let Some(raw_lines) = self.raw_lines {
            outcome.insert("raw_lines".to_owned(), Value::from(raw_lines));
        }
        outcome
    }
}

pub fn parse_player_listing(response: &str) -> PlayerListing {
    let lines: Vec<&str> = response
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut players = Vec::new();
    let mut fully_understood = true;
    for line in &lines {
        match player_row(line) {
            Some(player) => players.push(player),
            None => fully_understood &= is_listing_header(line),
        }
    }
    PlayerListing {
        players,
        raw_lines: (!fully_understood)
            .then(|| lines.iter().map(|line| (*line).to_owned()).collect()),
    }
}

/// `<playerId> ; <identity> ; <name>`. A line with more separators is not a row: its fields
/// cannot be told apart.
fn player_row(line: &str) -> Option<ListedPlayer> {
    let mut fields = line.split(';').map(str::trim);
    let player_id = fields.next()?.parse::<u32>().ok()?;
    let arma_id = fields.next()?;
    let name = fields.next()?;
    let identity_like = !arma_id.is_empty()
        && arma_id.len() <= IDENTITY_MAX_BYTES
        && arma_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    (fields.next().is_none() && identity_like && !name.is_empty()).then(|| ListedPlayer {
        player_id,
        arma_id: arma_id.to_owned(),
        name: name.to_owned(),
    })
}

/// The `Players on server:` title, the bracketed column legend, or a dashed rule.
fn is_listing_header(line: &str) -> bool {
    line.starts_with("Players on server")
        || line.bytes().all(|byte| byte == b'-')
        || line
            .split(';')
            .map(str::trim)
            .all(|field| field.len() > 2 && field.starts_with('[') && field.ends_with(']'))
}

#[cfg(test)]
#[path = "tests/reforger_commands.rs"]
mod tests;
