//! Registering an account from its Discord profile: the insert that creates the account at its
//! first sign-in, and the refresh of its profile at every later one.
//!
//! **Role:** the one write of an account's Discord profile columns (`username`, `discord_handle`,
//! `avatar_url`) and of its `last_login_at`.
//!
//! **Position:** the OAuth callback
//! ([`discord_callback`](crate::identity_and_access::handlers::discord_oauth::discord_callback))
//! passes the profile Discord returned, before the membership observation and the session; the
//! `staging-fixtures seed-load-population` host tool passes the synthetic profiles of its load
//! population, before it records their membership and issues their refresh sessions. The role,
//! the membership snapshot and the sessions are written elsewhere, never here.
//!
//! **Signals & state:** none; [`register_account`] runs one statement on the executor its caller
//! passes.
//!
//! **Invariants:** a new account starts unbanned, with no Arma character and the column's default
//! role; registering an existing account changes only its profile columns, `last_login_at` and
//! `updated_at`, so its role, ban, Arma link and statistics stay as they are; `avatar_url` holds an
//! http(s) URL or the empty "no avatar" value, never anything else.
//!
//! **The write boundary for `users.avatar_url`.** The column is public tier (anyone who can sign in
//! writes it) and reaches an `<img src>` on four SPA surfaces: the leaderboards, the layout chrome,
//! the settings page and the event hub. [`DiscordUser::avatar_url`] already refuses to build a URL
//! from an `id` or `avatar` that is not a bare path segment; [`stored_avatar_url`] checks the other
//! guarantee, that whatever reaches the column is an http(s) URL. The two fail independently: a
//! configured CDN base or another identity provider would move the first without touching the
//! second. A failing URL is stored as `""` instead of failing the registration, because a sign-in
//! must not fail over a cosmetic field and every reader already handles `""`.
//!
//! [`DiscordUser::avatar_url`]: crate::identity_and_access::services::discord_user_profile::DiscordUser::avatar_url

use sqlx::PgExecutor;

use crate::core::text::http_url_guard::is_http_url;

/// The Discord profile an account is registered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountProfile<'a> {
    /// The account's Discord id, the key of its `users` row.
    pub discord_id: &'a str,
    /// The name the platform displays: Discord's global name, else the username.
    pub username: &'a str,
    /// The Discord handle: the username, or the classic `name#1234`.
    pub discord_handle: &'a str,
    /// The avatar URL, or `""` for none.
    pub avatar_url: &'a str,
}

/// Create the account `profile` names, or refresh the profile of the existing one; either way
/// `last_login_at` becomes the statement's time.
pub async fn register_account<'e>(
    executor: impl PgExecutor<'e>,
    profile: &AccountProfile<'_>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO users \
         (discord_id, username, discord_handle, avatar_url, arma_character, is_banned, ban_reason, \
          last_login_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, '', false, '', now(), now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET \
          username = EXCLUDED.username, discord_handle = EXCLUDED.discord_handle, \
          avatar_url = EXCLUDED.avatar_url, last_login_at = EXCLUDED.last_login_at, updated_at = now()",
    )
    .bind(profile.discord_id)
    .bind(profile.username)
    .bind(profile.discord_handle)
    .bind(stored_avatar_url(profile))
    .execute(executor)
    .await?;
    Ok(())
}

/// The value `users.avatar_url` receives: the profile's URL when it is an http(s) URL, otherwise
/// `""`, with a warning naming the account when a non-empty URL is discarded.
fn stored_avatar_url<'a>(profile: &AccountProfile<'a>) -> &'a str {
    if is_http_url(profile.avatar_url) {
        return profile.avatar_url;
    }
    if !profile.avatar_url.is_empty() {
        tracing::warn!(
            discord_id = %profile.discord_id,
            "discarded a non-http(s) avatar URL from an account profile"
        );
    }
    ""
}

#[cfg(test)]
#[path = "tests/account_registration.rs"]
mod tests;
