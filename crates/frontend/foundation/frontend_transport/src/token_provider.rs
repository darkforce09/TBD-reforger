//! What an authenticated request needs from whoever holds the session's tokens.
//!
//! **Role:** declares [`TokenProvider`], the one seam between the transport and the session: the
//! bearer token to send, the session generation a request belongs to, the cold-start restore to
//! wait for, the per-tab refresh cell, the refresh itself, and where a rotated pair is stored.
//! **Position:** the transport declares it and is generic over it: the request verbs, the typed
//! endpoint calls, the status stream and the audit stream take any `impl TokenProvider`. The
//! session store (`AuthStore`, in the session layer above) implements it, so the transport never
//! names the store.
//! **Signals & state:** none here. An implementation reads its own state untracked, so a request
//! never subscribes the calling view to the session.
//! **Invariants:** a provider is `Copy + 'static`, because the retry closures capture it by move
//! and may outlive the call that built them. Every refresh of one provider runs through the cell
//! [`TokenProvider::refresh_flight`] returns, which is the same cell for every call in one browsing
//! context, so concurrent `401`s share one spend of the single-use refresh token.

use futures::future::LocalBoxFuture;

use super::client::SingleFlight;
use frontend_api_dtos::RefreshResponse;

/// The session as the transport sees it: a source of bearer tokens that can renew them.
///
/// Implemented by the session store. Every method answers for the session the
/// provider holds at the moment of the call; the generation methods are what let a request that
/// outlived its session discard its answer.
pub trait TokenProvider: Copy + 'static {
    /// Resolves once the provider knows which session requests are sent under: restored from
    /// storage, found absent, signed in or ended.
    fn session_restored(&self) -> LocalBoxFuture<'static, ()>;

    /// The current session generation, captured before a request starts.
    fn current_generation(&self) -> u64;

    /// Whether `generation` is still the current session generation.
    fn is_current_generation(&self, generation: u64) -> bool;

    /// The bearer token to send, or `None` when no access token is held.
    fn access_token(&self) -> Option<String>;

    /// Adopt a rotated pair after a refresh.
    fn set_tokens(&self, tokens: RefreshResponse);

    /// The per-tab single-flight cell every refresh of this session runs through.
    fn refresh_flight(&self) -> SingleFlight<Option<RefreshResponse>>;

    /// Spend the refresh token once and resolve to the rotated pair, or `None` when the session
    /// could not be renewed.
    fn refresh(&self) -> LocalBoxFuture<'static, Option<RefreshResponse>>;
}
