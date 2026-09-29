//! The member a Discord read targets: its guild, its id, the API base the request goes to, and the
//! bot credential and HTTP client it goes with.
//!
//! **Role:** takes `--discord-id`, `--guild main|partner`, `--partner-guild-id` and
//! `--discord-api-base` from the command line, and resolves them against the API env file into a
//! [`ResolvedTarget`] a read can send.
//!
//! **Position:** both subcommands of `discord_member_probe` take a [`DiscordTargetOptions`] while
//! parsing and resolve it once the guards have passed; `member_read` sends through the
//! [`ResolvedTarget`].
//!
//! **Signals & state:** none; the resolved target owns its HTTP client for one run.
//!
//! **Invariants:** the guild and member ids are decimal Discord ids, so the request path holds no
//! segment a flag or env value chose; the request goes to Discord's API
//! ([`DEFAULT_DISCORD_API`]) unless `--discord-api-base` names an http(s) URL on a loopback IP
//! address, the test-only seam for a fake Discord on the same host, so the bot token never leaves
//! the host except to Discord; the `Authorization` value is marked sensitive and appears in no
//! message and no `Debug` rendering; redirects are never followed.

use std::net::IpAddr;
use std::time::Duration;

use reqwest::header::HeaderValue;
use url::{Host, Url};
use website_api::identity_and_access::services::discord_client::DEFAULT_DISCORD_API;

use crate::argument_list::ArgumentList;
use crate::guarded_context::GuardedContext;
use crate::tool_failure::ToolFailure;

/// The longest a read waits for Discord, as long as the API's own client waits.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// A Discord id is a 64-bit number: at most 20 decimal digits.
const MAXIMUM_DISCORD_ID_DIGITS: usize = 20;

/// The guild a read names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GuildChoice {
    /// The `DISCORD_GUILD_ID` guild of the API env file.
    Main,
    /// A partner guild, by the id `--partner-guild-id` gives.
    Partner(String),
}

/// Where the request goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ApiBase {
    /// Discord's API.
    Discord,
    /// A fake Discord on this host, at this base URL without a trailing slash.
    Loopback(String),
}

/// The target flags of a Discord read, as parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscordTargetOptions {
    pub(super) guild: GuildChoice,
    pub(super) member: String,
    pub(super) api_base: ApiBase,
}

/// A target whose guards passed: what one read sends, and the client it sends with.
pub(super) struct ResolvedTarget {
    /// `main` or `partner`.
    pub(super) guild_label: &'static str,
    /// The guild's Discord id.
    pub(super) guild_id: String,
    /// The member's Discord id.
    pub(super) member: String,
    /// `<base>/guilds/<guild id>/members/<member id>`.
    pub(super) member_url: String,
    /// `Bot <token>`, marked sensitive.
    pub(super) authorization: HeaderValue,
    /// The run's HTTP client.
    pub(super) client: reqwest::Client,
}

impl DiscordTargetOptions {
    /// Take `--discord-id <id> --guild main|partner [--partner-guild-id <id>]
    /// [--discord-api-base <loopback url>]`.
    pub(super) fn take(arguments: &mut ArgumentList) -> Result<Self, ToolFailure> {
        let member = discord_id("--discord-id", arguments.required("--discord-id")?)?;
        let guild_flag = arguments.required("--guild")?;
        let partner = arguments.optional("--partner-guild-id")?;
        let guild = match (guild_flag.as_str(), partner) {
            ("main", None) => GuildChoice::Main,
            ("partner", Some(id)) => GuildChoice::Partner(discord_id("--partner-guild-id", id)?),
            ("main", Some(_)) => {
                return Err(ToolFailure::refused(
                    "--partner-guild-id goes with --guild partner only",
                ));
            }
            ("partner", None) => {
                return Err(ToolFailure::refused(
                    "--guild partner needs --partner-guild-id <discord id>",
                ));
            }
            (other, _) => {
                return Err(ToolFailure::refused(format!(
                    "--guild takes main or partner, not `{other}`"
                )));
            }
        };
        let api_base = match arguments.optional("--discord-api-base")? {
            None => ApiBase::Discord,
            Some(value) => ApiBase::Loopback(loopback_api_base(&value)?),
        };
        Ok(Self {
            guild,
            member,
            api_base,
        })
    }

    /// Resolve the guild id and the bot token from the API env file and build the run's client.
    pub(super) fn resolve(&self, context: &GuardedContext) -> Result<ResolvedTarget, ToolFailure> {
        let environment = &context.api_environment;
        let file = environment.path().display();
        let (guild_label, guild_id) = match &self.guild {
            GuildChoice::Main => {
                let id = environment.non_empty("DISCORD_GUILD_ID").ok_or_else(|| {
                    ToolFailure::refused(format!("DISCORD_GUILD_ID is not set in {file}"))
                })?;
                if !is_discord_id(id) {
                    return Err(ToolFailure::refused(format!(
                        "DISCORD_GUILD_ID in {file} is not a decimal Discord id"
                    )));
                }
                ("main", id.to_owned())
            }
            GuildChoice::Partner(id) => ("partner", id.clone()),
        };
        let token = environment.non_empty("DISCORD_BOT_TOKEN").ok_or_else(|| {
            ToolFailure::refused(format!(
                "DISCORD_BOT_TOKEN is not set in {file}; a member read authenticates as the bot"
            ))
        })?;
        if token.contains(char::is_whitespace) {
            return Err(ToolFailure::refused(format!(
                "DISCORD_BOT_TOKEN in {file} contains whitespace"
            )));
        }
        let mut authorization = HeaderValue::from_str(&format!("Bot {token}")).map_err(|_| {
            ToolFailure::refused(format!(
                "DISCORD_BOT_TOKEN in {file} is not a valid header value"
            ))
        })?;
        authorization.set_sensitive(true);
        let (base, loopback) = match &self.api_base {
            ApiBase::Discord => (DEFAULT_DISCORD_API, false),
            ApiBase::Loopback(base) => (base.as_str(), true),
        };
        Ok(ResolvedTarget {
            guild_label,
            member_url: format!("{base}/guilds/{guild_id}/members/{}", self.member),
            guild_id,
            member: self.member.clone(),
            authorization,
            client: http_client(loopback)?,
        })
    }
}

impl ResolvedTarget {
    /// The read in words, for a plan line: guild, ids and the endpoint's host, never the token.
    pub(super) fn describe(&self) -> String {
        let host = Url::parse(&self.member_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_default();
        format!(
            "GET /guilds/{}/members/{} on {host} (guild={}) with the bot token of the API env file",
            self.guild_id, self.member, self.guild_label
        )
    }
}

/// Whether `value` is a decimal Discord id: 1 to 20 ASCII digits.
pub(super) fn is_discord_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAXIMUM_DISCORD_ID_DIGITS
        && value.bytes().all(|byte| byte.is_ascii_digit())
}

/// The value of `flag` when it is a decimal Discord id; the refusal quotes it, since no flag holds
/// a secret.
fn discord_id(flag: &str, value: String) -> Result<String, ToolFailure> {
    if is_discord_id(&value) {
        Ok(value)
    } else {
        Err(ToolFailure::refused(format!(
            "{flag} takes a decimal Discord id, not `{value}`"
        )))
    }
}

/// The base `--discord-api-base` names, accepted only as an http(s) URL on a loopback IP address
/// with no credentials, query or fragment, and returned without its trailing slash.
pub(super) fn loopback_api_base(value: &str) -> Result<String, ToolFailure> {
    // The refusal does not quote the value: a URL can carry credentials.
    let refusal = || {
        ToolFailure::refused(
            "--discord-api-base takes an http(s) URL on a loopback IP address with no credentials, \
             query or fragment: a fake Discord on this host, since the bot token goes nowhere else \
             but Discord",
        )
    };
    let url = Url::parse(value).map_err(|_| refusal())?;
    let loopback = match url.host() {
        Some(Host::Ipv4(address)) => IpAddr::V4(address).is_loopback(),
        Some(Host::Ipv6(address)) => IpAddr::V6(address).is_loopback(),
        Some(Host::Domain(_)) | None => false,
    };
    let plain = url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none();
    if matches!(url.scheme(), "http" | "https") && loopback && plain {
        Ok(url.as_str().trim_end_matches('/').to_owned())
    } else {
        Err(refusal())
    }
}

/// The run's client: Discord's timeout, no redirects, and no proxy for a loopback base.
fn http_client(loopback: bool) -> Result<reqwest::Client, ToolFailure> {
    // reqwest carries no TLS provider of its own; the API installs ring the same way.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut builder = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none());
    if loopback {
        builder = builder.no_proxy();
    }
    builder
        .build()
        .map_err(|error| ToolFailure::failed(format!("cannot build the HTTP client: {error}")))
}

#[cfg(test)]
#[path = "tests/discord_target.rs"]
mod tests;
