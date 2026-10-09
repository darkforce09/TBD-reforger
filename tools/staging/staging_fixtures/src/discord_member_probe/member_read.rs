//! One bot-authenticated read of a guild member: the request, what Discord's answer means, the
//! rate-limit headers that came with it, and the `discord-member-read` line that reports it; and
//! `observe-discord-member`, which makes one such read.
//!
//! **Role:** sends `GET /guilds/{guild}/members/{member}` through a [`ResolvedTarget`] and turns the
//! answer into a [`MemberRead`]: member with its role ids, nonmember (Discord's Unknown Member,
//! code 10007), rate limited with its retry delay, or unavailable with the reason.
//!
//! **Position:** `observe-discord-member` reads once; `bucket_spend` reads until the bucket is
//! spent. The `staging discord` harness parses the `discord-member-read` lines both print.
//!
//! **Signals & state:** none; each read is independent.
//!
//! **Invariants:** a read's outcome follows the API's own member lookup
//! (`DiscordService::fetch_member_with_bot`): only a 200 with a `roles` list of strings is a
//! member, only a 404 carrying code 10007 is a nonmember, a 429 is rate limited, and everything
//! else, a transport failure included, is unavailable; header values outside their valid range
//! are dropped; the printed line carries ids, statuses, times and header values, never the token
//! or the member's profile.

use reqwest::header::{AUTHORIZATION, HeaderMap, USER_AGENT};
use serde_json::{Value, json};

use super::discord_target::{DiscordTargetOptions, ResolvedTarget};
use crate::argument_list::ArgumentList;
use crate::guarded_context::{GuardedContext, ParsedSubcommand};
use crate::tool_failure::ToolFailure;

/// The prefix of every printed read.
pub(super) const MEMBER_READ_LINE_PREFIX: &str = "discord-member-read";
/// Discord's error code for a user who is not a member of the guild.
const UNKNOWN_MEMBER_CODE: i64 = 10007;
/// The longest delay a rate-limit value may name, a week, as the API bounds it.
const MAXIMUM_DELAY_SECONDS: f64 = 604_800.0;
/// The longest bucket name kept.
const MAXIMUM_BUCKET_NAME_LENGTH: usize = 128;
/// The User-Agent every read sends.
const PROBE_USER_AGENT: &str = "TBD-Reforger/2 (staging-fixtures member probe)";

/// What Discord's answer means for the member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MemberOutcome {
    /// A member, with its role ids.
    Member { roles: Vec<String> },
    /// Discord's Unknown Member: not in the guild.
    Nonmember,
    /// A 429: the bucket, or the global limit, is spent.
    RateLimited {
        /// How long Discord asks the caller to wait, when it says.
        retry_after_ms: Option<u64>,
        /// Whether the global limit, rather than the route's bucket, is spent.
        global: bool,
    },
    /// No membership could be read.
    Unavailable { reason: &'static str },
}

impl MemberOutcome {
    /// Whether the answer said whether the account is a member.
    pub(super) fn observed_membership(&self) -> bool {
        matches!(self, Self::Member { .. } | Self::Nonmember)
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Member { .. } => "member",
            Self::Nonmember => "nonmember",
            Self::RateLimited { .. } => "rate_limited",
            Self::Unavailable { .. } => "unavailable",
        }
    }
}

/// The rate-limit headers of one answer; an absent or out-of-range header is `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct RateLimitHeaders {
    /// `X-RateLimit-Bucket`: the bucket's opaque name.
    pub(super) bucket: Option<String>,
    /// `X-RateLimit-Limit`: requests per window.
    pub(super) limit: Option<u32>,
    /// `X-RateLimit-Remaining`: requests left in the window.
    pub(super) remaining: Option<u32>,
    /// `X-RateLimit-Reset-After`, in milliseconds: until the window resets.
    pub(super) reset_after_ms: Option<u64>,
    /// `X-RateLimit-Scope`: `user`, `global` or `shared`, on a 429.
    pub(super) scope: Option<String>,
}

impl RateLimitHeaders {
    /// Read the rate-limit headers of `headers`.
    pub(super) fn from_headers(headers: &HeaderMap) -> Self {
        let text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
        let short_text = |name: &str| {
            text(name)
                .filter(|value| value.len() <= MAXIMUM_BUCKET_NAME_LENGTH)
                .map(str::to_owned)
        };
        Self {
            bucket: short_text("x-ratelimit-bucket"),
            limit: text("x-ratelimit-limit").and_then(|value| value.parse().ok()),
            remaining: text("x-ratelimit-remaining").and_then(|value| value.parse().ok()),
            reset_after_ms: text("x-ratelimit-reset-after").and_then(seconds_text_to_ms),
            scope: short_text("x-ratelimit-scope"),
        }
    }
}

/// One read and what it observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MemberRead {
    /// When the request left, in Unix milliseconds.
    pub(super) sent_at_unix_ms: u64,
    /// When the answer, or the transport failure, arrived.
    pub(super) answered_at_unix_ms: u64,
    /// The HTTP status, when an answer arrived.
    pub(super) http_status: Option<u16>,
    /// What the answer means.
    pub(super) outcome: MemberOutcome,
    /// The answer's rate-limit headers.
    pub(super) rate_limit: RateLimitHeaders,
}

impl MemberRead {
    /// The `discord-member-read {json}` line of request number `request` of a run.
    pub(super) fn report_line(&self, request: u32, target: &ResolvedTarget) -> String {
        let mut record = json!({
            "request": request,
            "guild": target.guild_label,
            "guild_id": target.guild_id,
            "discord_id": target.member,
            "sent_at_unix_ms": self.sent_at_unix_ms,
            "answered_at_unix_ms": self.answered_at_unix_ms,
            "http_status": self.http_status,
            "outcome": self.outcome.label(),
            "roles": null,
            "retry_after_ms": null,
            "global": null,
            "reason": null,
            "rate_limit": {
                "bucket": self.rate_limit.bucket,
                "limit": self.rate_limit.limit,
                "remaining": self.rate_limit.remaining,
                "reset_after_ms": self.rate_limit.reset_after_ms,
                "scope": self.rate_limit.scope,
            },
        });
        match &self.outcome {
            MemberOutcome::Member { roles } => record["roles"] = json!(roles),
            MemberOutcome::Nonmember => {}
            MemberOutcome::RateLimited {
                retry_after_ms,
                global,
            } => {
                record["retry_after_ms"] = json!(retry_after_ms);
                record["global"] = json!(global);
            }
            MemberOutcome::Unavailable { reason } => record["reason"] = json!(reason),
        }
        format!("{MEMBER_READ_LINE_PREFIX} {record}")
    }
}

/// Send one member read through `target`.
pub(super) async fn read_member(target: &ResolvedTarget) -> MemberRead {
    let sent_at_unix_ms = unix_now_ms();
    let sent = target
        .client
        .get(&target.member_url)
        .header(AUTHORIZATION, target.authorization.clone())
        .header(USER_AGENT, PROBE_USER_AGENT)
        .send()
        .await;
    let response = match sent {
        Ok(response) => response,
        Err(error) => {
            let reason = if error.is_timeout() {
                "Discord did not answer within the timeout"
            } else if error.is_connect() {
                "cannot connect to Discord"
            } else {
                "Discord transport unavailable"
            };
            return MemberRead {
                sent_at_unix_ms,
                answered_at_unix_ms: unix_now_ms(),
                http_status: None,
                outcome: MemberOutcome::Unavailable { reason },
                rate_limit: RateLimitHeaders::default(),
            };
        }
    };
    let status = response.status().as_u16();
    let rate_limit = RateLimitHeaders::from_headers(response.headers());
    let retry_after_header_ms = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(seconds_text_to_ms);
    let body: Value = response.json().await.unwrap_or(Value::Null);
    MemberRead {
        sent_at_unix_ms,
        answered_at_unix_ms: unix_now_ms(),
        http_status: Some(status),
        outcome: classify_answer(status, &body, retry_after_header_ms),
        rate_limit,
    }
}

/// What status `status` with body `body` means, following the API's member lookup.
pub(super) fn classify_answer(
    status: u16,
    body: &Value,
    retry_after_header_ms: Option<u64>,
) -> MemberOutcome {
    match status {
        200 => match body.get("roles").and_then(Value::as_array) {
            Some(roles) if roles.iter().all(Value::is_string) => MemberOutcome::Member {
                roles: roles
                    .iter()
                    .filter_map(|role| role.as_str().map(str::to_owned))
                    .collect(),
            },
            _ => MemberOutcome::Unavailable {
                reason: "Discord returned malformed membership data",
            },
        },
        404 if body.get("code").and_then(Value::as_i64) == Some(UNKNOWN_MEMBER_CODE) => {
            MemberOutcome::Nonmember
        }
        429 => MemberOutcome::RateLimited {
            retry_after_ms: body
                .get("retry_after")
                .and_then(Value::as_f64)
                .and_then(seconds_to_ms)
                .or(retry_after_header_ms),
            global: body.get("global").and_then(Value::as_bool).unwrap_or(false),
        },
        401 => MemberOutcome::Unavailable {
            reason: "Discord refused the bot token",
        },
        403 => MemberOutcome::Unavailable {
            reason: "the bot may not read this guild's members",
        },
        _ => MemberOutcome::Unavailable {
            reason: "Discord answered with neither a member nor Unknown Member",
        },
    }
}

/// Parse a header's seconds, possibly fractional, into whole milliseconds rounded up.
fn seconds_text_to_ms(text: &str) -> Option<u64> {
    text.trim().parse::<f64>().ok().and_then(seconds_to_ms)
}

/// Whole milliseconds rounded up, for a finite delay of 0 to a week.
fn seconds_to_ms(seconds: f64) -> Option<u64> {
    (seconds.is_finite() && (0.0..=MAXIMUM_DELAY_SECONDS).contains(&seconds))
        .then(|| (seconds * 1000.0).ceil() as u64)
}

/// The host's clock in Unix milliseconds, read through the workspace's one clock
/// ([`time_source::SystemClock`]).
pub(super) fn unix_now_ms() -> u64 {
    time_source::Clock::now_unix_ms(&time_source::SystemClock)
}

/// Parse `observe-discord-member`: the target flags only.
pub(crate) fn parse_observe(arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    let options = DiscordTargetOptions::take(arguments)?;
    Ok(Box::new(move |context| Box::pin(observe(context, options))))
}

async fn observe(
    context: GuardedContext,
    options: DiscordTargetOptions,
) -> Result<(), ToolFailure> {
    let target = options.resolve(&context)?;
    println!("observe plan: one member read: {}", target.describe());
    if !context.mode.writes() {
        println!("dry run: no request sent to Discord");
        return Ok(());
    }
    let read = read_member(&target).await;
    println!("{}", read.report_line(1, &target));
    match &read.outcome {
        outcome if outcome.observed_membership() => Ok(()),
        MemberOutcome::Unavailable { reason } => Err(ToolFailure::failed(format!(
            "the member read observed no membership: {reason}"
        ))),
        _ => Err(ToolFailure::failed(
            "the member read observed no membership: Discord rate limited it",
        )),
    }
}
