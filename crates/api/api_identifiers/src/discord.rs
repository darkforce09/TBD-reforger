//! The identifiers Discord assigns: users, guilds, roles, messages and the OAuth2 application.
//!
//! **Role:** declares one typed id per Discord snowflake the API stores, sends or reads, so a
//! guild id and a user id are distinct types although both are snowflake text.
//! **Position:** declared here, below every API crate; identity and access stores the users and
//! their roles, the configuration names the guild and the application, and every domain that
//! names a member takes [`DiscordUserId`].
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as the snowflake text it
//! wraps (transparent serde, `#[sqlx(transparent)]`); parsing never fails, since Discord, not the
//! API, decides what a snowflake is.

newtype_ids::string_id! {
    sqlx,
    /// A Discord user's snowflake, as text: the key of `users` (`users.discord_id`). The
    /// default is the empty text, which names no member.
    #[derive(Default)]
    pub struct DiscordUserId;
}

newtype_ids::string_id! {
    sqlx,
    /// A Discord guild's (server's) snowflake, as text: the community guild the configuration
    /// names, and the partner guilds an event admits members of.
    pub struct DiscordGuildId;
}

newtype_ids::string_id! {
    sqlx,
    /// A Discord role's snowflake, as text: the key of the role-to-platform-role mapping.
    pub struct DiscordRoleId;
}

newtype_ids::string_id! {
    sqlx,
    /// A Discord message's snowflake, as text: the channel post an announcement was published as.
    #[derive(Default)]
    pub struct DiscordMessageId;
}

newtype_ids::string_id! {
    /// The OAuth2 client id of the Discord application the platform signs members in through.
    pub struct DiscordClientId;
}

impl DiscordUserId {
    /// True when the record names no member (the stored text is empty).
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

impl DiscordMessageId {
    /// True when the announcement was never published to Discord (the stored text is empty).
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}
