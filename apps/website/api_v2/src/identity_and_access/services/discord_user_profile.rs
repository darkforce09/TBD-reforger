//! The Discord user profile (`/users/@me`) and the names and avatar URL derived from it.
//!
//! These three derived values are what the OAuth callback writes into `users.username`,
//! `users.discord_handle` and `users.avatar_url`, so this is the boundary where a hostile
//! or malformed upstream profile has to be stopped.

use serde::Deserialize;

/// The subset of `/users/@me` this crate uses.
///
/// **`username` is deliberately required — do not add `#[serde(default)]` to it.** A
/// malformed *upstream* response is not a client error, so there is no 400 to return and no
/// request to reject; the only lever here is whether the body decodes.
///
/// Discord's user object always carries `username` — it is required and non-nullable, and
/// there is no such thing as a Discord account without one. So a 200 body that lacks it is
/// not "this user has no username", it is **not a user object**: a gateway or CDN answering
/// 200 with something else, or an API shape change. Defaulted, that decodes cleanly into
/// `DiscordUser { username: "" }`, and because [`Self::display_name`] falls back to
/// `username` and [`Self::handle`] is built from it,
/// [`crate::identity_and_access::handlers::discord_oauth`] would bind two empty strings into
/// `users.username` and `users.discord_handle`.
///
/// **The right answer is to fail the login, not to patch the value.** Keeping the stored
/// name would need a `COALESCE` in the oauth upsert, which is the wrong place to encode
/// "the profile was junk" — and it would still let the junk profile mint a session. Failing
/// the decode routes the whole callback down its existing `Err` path
/// (`fetch_user` → `err("discord_unreachable")`), which writes nothing at all: no user row,
/// no session, no audit entry. The user retries; a transient blip costs one login.
///
/// **"It self-heals on the next login" is only half true, and the wrong half is the one that
/// lasts.** `users.username`/`discord_handle` do heal, via `ON CONFLICT (discord_id) DO
/// UPDATE SET username = EXCLUDED.username`. But the same callback then writes an
/// `auth.login` row with `actor_name = ''` and the message `" signed in via Discord"`, and
/// `audit_logs` is append-only — this crate contains zero `UPDATE audit_logs`. The user row
/// recovers; the audit trail keeps an anonymous login forever, and every action taken during
/// the blank window is logged under an empty actor.
///
/// Like `GuildMember::roles` this does **not** get `null_default` either — `"username": null`
/// is malformed for a user object, so failing closed is right. An explicit `""` still
/// decodes: that is a stated answer, not silence, and the same line `GuildMember` draws
/// between an absent `roles` and `[]`.
#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    #[serde(default, deserialize_with = "super::discord_client::null_default")]
    pub global_name: String,
    #[serde(default, deserialize_with = "super::discord_client::null_default")]
    pub discriminator: String,
    #[serde(default, deserialize_with = "super::discord_client::null_default")]
    pub avatar: String,
}

/// Is `s` safe to interpolate as **one path segment** of a CDN URL?
///
/// Non-empty and `[A-Za-z0-9_]` only. That excludes every character that could end the segment or
/// the path — `/`, `\`, `.`, `?`, `#`, `%`, `@`, `:` — and every control character and space along
/// with them, so the value cannot move the URL anywhere its author did not intend. See
/// [`DiscordUser::avatar_url`] for why the rule is a character class rather than Discord's exact
/// documented formats.
fn is_cdn_path_segment(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl DiscordUser {
    /// Prefer the new global display name, falling back to username.
    ///
    /// **The selection test is `trim().is_empty()`, not `is_empty()`.** This is a *choice
    /// between two fields*, not a guard that should trim the winner. A `global_name` of
    /// `"   "` is present and non-empty, so an `is_empty()` branch would store it verbatim
    /// into `users.username` on every oauth login upsert — a nameless account that still
    /// looks named. Whitespace-only `global_name` therefore falls through to `username`. A
    /// name that survives (`"  Dave  "`) is returned exactly as Discord sent it: padding is
    /// cosmetic; namelessness is not.
    pub fn display_name(&self) -> String {
        if self.global_name.trim().is_empty() {
            self.username.clone()
        } else {
            self.global_name.clone()
        }
    }

    /// Classic `name#1234`, or just the username for the new unique-username system.
    pub fn handle(&self) -> String {
        if self.discriminator.is_empty() || self.discriminator == "0" {
            self.username.clone()
        } else {
            format!("{}#{}", self.username, self.discriminator)
        }
    }

    /// CDN avatar URL, or `""` if the user has no custom avatar **or if Discord handed us an `id`
    /// or `avatar` that is not a bare path segment**.
    ///
    /// # The trust boundary, stated and enforced
    ///
    /// This `format!`s two strings straight out of an HTTP response into a URL path. The stored
    /// result is public-tier (the oauth callback writes it to `users.avatar_url` on every login)
    /// and reaches an `<img src>` on four SPA pages. Unchecked, an `avatar` of `../../evil` walks
    /// the URL out of `/avatars/` entirely, and one containing `?`, `#` or `@` re-points it by
    /// query, fragment or authority — while still *looking* like a `cdn.discordapp.com` link to
    /// anyone reading the database.
    ///
    /// The trust boundary here is "we trust Discord's API", and that is very probably fine. Left
    /// undocumented and unenforced it would be two different problems from "wrong": an assumption
    /// nobody wrote down cannot be reviewed, and one nothing checks is indistinguishable from an
    /// assumption that has quietly stopped holding — a compromised or spoofed token endpoint, a
    /// proxy, a future Discord format change, or a test double. Enforcing it costs one
    /// character-class check and converts a silent trust into a loud one.
    ///
    /// The rule is **`[A-Za-z0-9_]` only**, which is deliberately looser than Discord's documented
    /// shapes (snowflakes are decimal; avatar hashes are 32 hex characters, optionally `a_`-
    /// prefixed when animated). Pinning the exact shapes would be a stricter guard and a worse
    /// one: it buys nothing extra — every character that could escape the path segment is already
    /// excluded — and it would start silently blanking real avatars the day Discord widens its
    /// hash format. What is excluded is the part that matters: `/`, `.`, `?`, `#`, `%`, `@`, `:`,
    /// `\` and whitespace.
    ///
    /// Failing to `""` rather than panicking or erroring, because the only caller is an OAuth
    /// callback: a login must not fail over a cosmetic field. `""` is this column's existing
    /// "no avatar" value and every reader already handles it.
    pub fn avatar_url(&self) -> String {
        if self.avatar.is_empty()
            || !is_cdn_path_segment(&self.id)
            || !is_cdn_path_segment(&self.avatar)
        {
            String::new()
        } else {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                self.id, self.avatar
            )
        }
    }
}

#[cfg(test)]
#[path = "tests/discord_user_profile.rs"]
mod tests;
