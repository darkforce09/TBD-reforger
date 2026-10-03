//! `observe-discord-member` and `spend-discord-member-bucket`: bot-authenticated reads of one guild
//! member, the staging Discord procedure's witness of role changes and its rate-limit fault.
//!
//! **Role:** the two subcommands' parsers, over what they share: the member, guild and Discord
//! base a read targets (`discord_target`) and the read itself with its outcome and rate-limit
//! headers (`member_read`); `bucket_spend` drives repeated reads to spend the Get Guild Member
//! bucket.
//!
//! **Position:** rows of the subcommand table in `main.rs`; the `staging discord` harness runs both
//! on the host through its `discord_member_reader` observer and reads their `discord-member-read`
//! and `discord-bucket-spend` lines.
//!
//! **Signals & state:** none of its own; see the submodules.
//!
//! **Invariants:** nothing here writes to the database; the bot token comes from the API env file
//! and goes only into the `Authorization` header of a request to Discord's API or to a loopback
//! address, never into a message, a printed line or a `Debug` rendering; a dry run sends no
//! request.

mod bucket_spend;
mod discord_target;
mod member_read;

pub(crate) use bucket_spend::parse as parse_spend;
pub(crate) use member_read::parse_observe;
