//! The identifiers of the identity and access domain: authentication sessions, refresh tokens
//! and the Arma player identity a member links.
//!
//! **Role:** declares one typed id per identity table key, and the Arma player id the account
//! link handshake stores and the game runtime reports players by.
//! **Position:** declared here, below every API crate; identity and access owns the tables, and
//! the operations, missions and telemetry code that names a player in game takes
//! [`ArmaPlayerId`].
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner value
//! (transparent serde, `#[sqlx(transparent)]`): a UUID for the session keys, the game's identity
//! text for a player.

newtype_ids::uuid_id! {
    sqlx,
    /// A signed-in session: the key of `authentication_sessions`.
    pub struct AuthenticationSessionId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A rotating refresh credential: the key of `refresh_tokens`.
    pub struct RefreshTokenId;
}

newtype_ids::string_id! {
    sqlx,
    /// A player's Arma Reforger identity, as the game reports it (`users.arma_id`): the key the
    /// game runtime names players by in rosters, occupancies and telemetry.
    pub struct ArmaPlayerId;
}
