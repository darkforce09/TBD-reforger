//! `spend-discord-member-bucket`: spends the Get Guild Member rate-limit bucket at a chosen instant
//! and keeps it spent for a hold, so the API's own member refresh meets a 429.
//!
//! **Role:** waits for `--start-at-unix-ms`, reads the member until Discord reports the bucket
//! spent (`X-RateLimit-Remaining: 0` or a 429), and until the hold ends reads again only when the
//! bucket resets, so it is spent again at once; it prints a `discord-member-read` line per request
//! and a `discord-bucket-spend` summary.
//!
//! **Position:** a row of the subcommand table in `main.rs`. The `staging discord` harness starts
//! it 0.3 s before the operator's `next_refresh_at`, on the host, with the bot token the API holds,
//! so both spend the same bucket; the harness then observes the API's `Discord rate limited`
//! outcome and its recovery.
//!
//! **Signals & state:** the run's request count and the first instant the bucket was seen spent.
//!
//! **Invariants:** at most `--max-requests` requests, never more than [`MAXIMUM_BUCKET_REQUESTS`];
//! no request while the last answer said the bucket is spent and its reset has not come; no request
//! after the hold ends; a read Discord answers with neither a membership nor a 429 (a refused
//! token, a transport failure) ends the run at once; a start already more than
//! [`LATE_START_TOLERANCE_MS`] past, or further ahead than [`MAXIMUM_START_DELAY_MS`], is refused
//! before any request. The run succeeds only when it saw the bucket spent and ended at the hold or
//! the request cap.

use std::time::Duration;

use serde_json::json;

use super::discord_target::DiscordTargetOptions;
use super::member_read::{MemberOutcome, RateLimitHeaders, read_member, unix_now_ms};
use crate::argument_list::ArgumentList;
use crate::guarded_context::{GuardedContext, ParsedSubcommand};
use crate::tool_failure::ToolFailure;

/// The most requests one spend may send.
pub(crate) const MAXIMUM_BUCKET_REQUESTS: u32 = 50;
/// The longest hold, in seconds.
pub(crate) const MAXIMUM_HOLD_SECONDS: u32 = 10;
/// How late a start may be and still spend the bucket before a refresh 0.3 s after it.
pub(crate) const LATE_START_TOLERANCE_MS: u64 = 250;
/// How far ahead a start may lie: a mistyped time must not park the tool for hours.
pub(crate) const MAXIMUM_START_DELAY_MS: u64 = 15 * 60 * 1000;
/// Added to a reset delay so the next request lands after the reset, not on its edge.
const RESET_MARGIN_MS: u64 = 25;
/// The prefix of the printed summary.
const SPEND_LINE_PREFIX: &str = "discord-bucket-spend";

/// What a spend does and when.
struct SpendPlan {
    target: DiscordTargetOptions,
    start_at_unix_ms: u64,
    hold_seconds: u32,
    max_requests: u32,
}

/// What the spend does after a read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NextStep {
    /// Read again at once: the bucket still has room.
    SendNow,
    /// Read again at this Unix millisecond, when the spent bucket resets.
    SendAt(u64),
    /// End the run.
    Stop(SpendStop),
}

/// Why a spend ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpendStop {
    /// The hold is over, or the next read would fall after it.
    HoldEnded,
    /// The run sent `--max-requests` requests.
    RequestCap,
    /// Discord answered with neither a membership nor a 429.
    DiscordUnavailable,
}

impl SpendStop {
    fn label(self) -> &'static str {
        match self {
            Self::HoldEnded => "hold_ended",
            Self::RequestCap => "request_cap",
            Self::DiscordUnavailable => "discord_unavailable",
        }
    }
}

/// Parse the target flags and `--start-at-unix-ms <ms> --hold-seconds <s> --max-requests <n>`.
pub(crate) fn parse(arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    let target = DiscordTargetOptions::take(arguments)?;
    let start_at_unix_ms =
        arguments.required_parsed::<u64>("--start-at-unix-ms", "a Unix time in milliseconds")?;
    let hold_seconds =
        arguments.required_parsed::<u32>("--hold-seconds", "a whole number of seconds")?;
    if !(1..=MAXIMUM_HOLD_SECONDS).contains(&hold_seconds) {
        return Err(ToolFailure::refused(format!(
            "--hold-seconds takes 1 to {MAXIMUM_HOLD_SECONDS}, not {hold_seconds}"
        )));
    }
    let max_requests = arguments.required_parsed::<u32>("--max-requests", "a whole number")?;
    if !(1..=MAXIMUM_BUCKET_REQUESTS).contains(&max_requests) {
        return Err(ToolFailure::refused(format!(
            "--max-requests takes 1 to {MAXIMUM_BUCKET_REQUESTS}, not {max_requests}"
        )));
    }
    let plan = SpendPlan {
        target,
        start_at_unix_ms,
        hold_seconds,
        max_requests,
    };
    Ok(Box::new(move |context| Box::pin(spend(context, plan))))
}

/// Refuse a start more than the tolerance past, or further ahead than the longest delay.
pub(super) fn check_start(start_at_unix_ms: u64, now_unix_ms: u64) -> Result<(), ToolFailure> {
    if now_unix_ms > start_at_unix_ms.saturating_add(LATE_START_TOLERANCE_MS) {
        return Err(ToolFailure::refused(format!(
            "--start-at-unix-ms {start_at_unix_ms} passed {} ms ago; the bucket must be spent \
             before the refresh it targets",
            now_unix_ms - start_at_unix_ms
        )));
    }
    if start_at_unix_ms > now_unix_ms.saturating_add(MAXIMUM_START_DELAY_MS) {
        return Err(ToolFailure::refused(format!(
            "--start-at-unix-ms {start_at_unix_ms} is more than {} minutes ahead",
            MAXIMUM_START_DELAY_MS / 60_000
        )));
    }
    Ok(())
}

/// The step after a read answered at `now_ms`, when `sent` requests of `max_requests` are spent.
pub(super) fn next_step(
    outcome: &MemberOutcome,
    rate_limit: &RateLimitHeaders,
    now_ms: u64,
    hold_until_ms: u64,
    sent: u32,
    max_requests: u32,
) -> NextStep {
    let after = |wait: Option<u64>| {
        wait.map_or(hold_until_ms, |wait| {
            now_ms.saturating_add(wait).saturating_add(RESET_MARGIN_MS)
        })
    };
    let resume_at = match outcome {
        MemberOutcome::Unavailable { .. } => return NextStep::Stop(SpendStop::DiscordUnavailable),
        MemberOutcome::RateLimited { retry_after_ms, .. } => after(*retry_after_ms),
        MemberOutcome::Member { .. } | MemberOutcome::Nonmember => match rate_limit.remaining {
            Some(0) => after(rate_limit.reset_after_ms),
            _ => now_ms,
        },
    };
    if sent >= max_requests {
        NextStep::Stop(SpendStop::RequestCap)
    } else if resume_at >= hold_until_ms {
        NextStep::Stop(SpendStop::HoldEnded)
    } else if resume_at <= now_ms {
        NextStep::SendNow
    } else {
        NextStep::SendAt(resume_at)
    }
}

/// Whether a read saw the bucket spent.
pub(super) fn saw_bucket_spent(outcome: &MemberOutcome, rate_limit: &RateLimitHeaders) -> bool {
    matches!(outcome, MemberOutcome::RateLimited { .. }) || rate_limit.remaining == Some(0)
}

async fn spend(context: GuardedContext, plan: SpendPlan) -> Result<(), ToolFailure> {
    let target = plan.target.resolve(&context)?;
    check_start(plan.start_at_unix_ms, unix_now_ms())?;
    let hold_until_ms = plan.start_at_unix_ms + u64::from(plan.hold_seconds) * 1000;
    println!(
        "spend plan: from {} for {} s, at most {} requests: {}",
        plan.start_at_unix_ms,
        plan.hold_seconds,
        plan.max_requests,
        target.describe()
    );
    if !context.mode.writes() {
        println!("dry run: no request sent to Discord");
        return Ok(());
    }
    sleep_until(plan.start_at_unix_ms).await;
    let (mut sent, mut rate_limited, mut first_spent_at) = (0_u32, 0_u32, None::<u64>);
    let stop = loop {
        let read = read_member(&target).await;
        sent += 1;
        if matches!(read.outcome, MemberOutcome::RateLimited { .. }) {
            rate_limited += 1;
        }
        if first_spent_at.is_none() && saw_bucket_spent(&read.outcome, &read.rate_limit) {
            first_spent_at = Some(read.answered_at_unix_ms);
        }
        println!("{}", read.report_line(sent, &target));
        let step = next_step(
            &read.outcome,
            &read.rate_limit,
            read.answered_at_unix_ms,
            hold_until_ms,
            sent,
            plan.max_requests,
        );
        match step {
            NextStep::SendNow => {}
            NextStep::SendAt(instant) => sleep_until(instant).await,
            NextStep::Stop(stop) => break stop,
        }
    };
    let summary = json!({
        "guild": target.guild_label,
        "guild_id": target.guild_id,
        "discord_id": target.member,
        "started_at_unix_ms": plan.start_at_unix_ms,
        "hold_until_unix_ms": hold_until_ms,
        "requests": sent,
        "rate_limited_responses": rate_limited,
        "bucket_spent": first_spent_at.is_some(),
        "first_spent_at_unix_ms": first_spent_at,
        "stopped_because": stop.label(),
    });
    println!("{SPEND_LINE_PREFIX} {summary}");
    match (first_spent_at, stop) {
        (_, SpendStop::DiscordUnavailable) => Err(ToolFailure::failed(
            "Discord answered with neither a membership nor a 429; the spend stopped",
        )),
        (None, _) => Err(ToolFailure::failed(format!(
            "the bucket was not seen spent after {sent} requests"
        ))),
        (Some(_), _) => Ok(()),
    }
}

/// Sleep until the host's clock reads `instant_unix_ms`.
async fn sleep_until(instant_unix_ms: u64) {
    let now = unix_now_ms();
    if instant_unix_ms > now {
        tokio::time::sleep(Duration::from_millis(instant_unix_ms - now)).await;
    }
}

#[cfg(test)]
#[path = "tests/bucket_spend.rs"]
mod tests;
