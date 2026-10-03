//! The identifiers of the match telemetry domain: matches and their per-player statistics.
//!
//! **Role:** declares one typed id per match telemetry table key, plus the game runtime's own
//! keys for a match, a player line and a telemetry event.
//! **Position:** declared here, below every API crate; match telemetry owns the tables, and the
//! server status, membership and leaderboard code that names a match takes the same type.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner UUID, text or
//! integer (transparent serde, `#[sqlx(transparent)]`), so the wire shapes and the SQL stay those
//! of the bare value.

newtype_ids::uuid_id! {
    sqlx,
    /// A match a game server played: the key of `matches`.
    pub struct MatchId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One player's statistics row of a match: the key of `match_player_stats`.
    pub struct MatchPlayerStatId;
}

newtype_ids::string_id! {
    sqlx,
    /// The game runtime's own key for a match (`matches.source_match_id`), which it registers
    /// the match under and names in every later report.
    pub struct SourceMatchId;
}

newtype_ids::string_id! {
    sqlx,
    /// The game runtime's idempotency key of one player line of a match result.
    pub struct SourceEventId;
}

newtype_ids::string_id! {
    sqlx,
    /// A telemetry event's key within its match (`match_events.event_id`), chosen by the game
    /// runtime: 1 to 64 of `A-Z a-z 0-9 . _ : -`.
    pub struct MatchEventId;
}
