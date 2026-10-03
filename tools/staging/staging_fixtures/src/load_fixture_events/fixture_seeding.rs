//! `seed-load-fixture-events`: creates the load fixture's events through the event authoring
//! services the administrator routes write through.
//!
//! **Role:** checks the seeding preconditions, prints the plan, and with `--apply` creates each
//! fixture event, confirms it admits verified TBD members, and attaches `--mission` with the
//! fixture ORBAT.
//!
//! **Position:** the parser in `mod.rs` hands it the mission id; it writes through
//! `event_creation::create_event` and `mission_attachment::attach_mission`, which also write the
//! `event.created` and `event.mission_attached` audit rows; `fixture_cleaning.rs` removes what it
//! wrote.
//!
//! **Signals & state:** one database transaction.
//!
//! **Invariants:** a dry run and a refusal write nothing, and a dry run runs every read-only check
//! an apply runs. It refuses while the API env file sets `DISCORD_BOT_TOKEN`, while no account of
//! the reserved range exists, while a fixture event of a reserved author exists, and when the
//! mission is missing, deleted or archived. The author is the lowest reserved account present, the
//! load population's first account. An apply commits every fixture event with its attachment,
//! slots and audit rows, or nothing: an event created without the member-admitting access policy
//! and an open, uncapped member pool fails the run.

use api_identifiers::{DiscordUserId, EventId, MissionId};
use api_operations::services::event_authoring::event_creation::create_event;
use api_operations::services::event_authoring::mission_attachment::{
    AttachmentAuthority, attach_mission,
};

use super::fixture_plan::{
    FIXTURE_TITLE_PREFIX, POPULATION_SHARE_PER_EVENT, fixture_event_plans, fixture_template,
};
use crate::guarded_context::GuardedContext;
use crate::reserved_accounts::RESERVED_DISCORD_ID_PATTERN;
use crate::tool_failure::ToolFailure;

/// The access policy the `events` table gives a new event: one grant, to verified TBD members.
const MEMBER_ADMITTING_POLICY: &str = r#"{"grants":[{"conditions":[{"kind":"tbd_member"}]}]}"#;

/// Run the seeding for `mission`: every check, the plan, and the writes when the run applies.
pub(super) async fn seed(context: GuardedContext, mission: MissionId) -> Result<(), ToolFailure> {
    if context
        .api_environment
        .non_empty("DISCORD_BOT_TOKEN")
        .is_some()
    {
        return Err(ToolFailure::refused(format!(
            "DISCORD_BOT_TOKEN is set in {}; the load fixtures are seeded only while the Discord \
             reconciler is off",
            context.api_environment.path().display()
        )));
    }
    let template = fixture_template()?;
    let mut transaction = context.pool.begin().await?;
    let author = fixture_author(&mut transaction).await?;
    refuse_existing_fixture(&mut transaction).await?;
    require_attachable_mission(&mut transaction, mission).await?;
    let now = sqlx::query_scalar("SELECT now()")
        .fetch_one(&mut *transaction)
        .await?;
    let plans = fixture_event_plans(now)?;
    for plan in &plans {
        println!(
            "plan fixture={:02} title=\"{}\" start={} mission={mission} slots={} \
             population_share={POPULATION_SHARE_PER_EVENT}",
            plan.number,
            plan.creation.name_override(),
            plan.creation.start_time().to_rfc3339(),
            template.slot_count(),
        );
    }
    println!("plan author={author}");
    if !context.mode.writes() {
        return Ok(());
    }

    let authority = AttachmentAuthority::HostToolAccount {
        discord_id: &author,
    };
    let mut created = Vec::new();
    for plan in &plans {
        let event = create_event(&mut transaction, &plan.creation, &author).await?;
        require_member_admitting(&mut transaction, event.id).await?;
        let attachment = attach_mission(
            &mut transaction,
            event.id,
            mission,
            plan.creation.start_time(),
            &template,
            &authority,
        )
        .await?;
        created.push((plan.number, event.id, attachment.id));
    }
    transaction.commit().await?;
    for (number, event, attachment) in &created {
        println!(
            "fixture={number:02} event={event} event_mission={attachment} slots={}",
            template.slot_count()
        );
    }
    println!(
        "seeded {} fixture events with {} slots each, authored by {author}",
        created.len(),
        template.slot_count()
    );
    Ok(())
}

/// The lowest account of the reserved range: the load population's first account.
async fn fixture_author(connection: &mut sqlx::PgConnection) -> Result<DiscordUserId, ToolFailure> {
    let author: Option<DiscordUserId> = sqlx::query_scalar(
        "SELECT discord_id FROM users WHERE discord_id ~ $1 ORDER BY discord_id LIMIT 1",
    )
    .bind(RESERVED_DISCORD_ID_PATTERN)
    .fetch_optional(connection)
    .await?;
    author.ok_or_else(|| {
        ToolFailure::refused(
            "no account of the reserved range exists; run seed-load-population first, whose first \
             account authors the fixture events",
        )
    })
}

/// Refuse while any fixture event of a reserved author exists: a seeding always starts clean.
async fn refuse_existing_fixture(connection: &mut sqlx::PgConnection) -> Result<(), ToolFailure> {
    let existing: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events WHERE starts_with(name_override, $1) AND created_by ~ $2",
    )
    .bind(FIXTURE_TITLE_PREFIX)
    .bind(RESERVED_DISCORD_ID_PATTERN)
    .fetch_one(connection)
    .await?;
    if existing == 0 {
        Ok(())
    } else {
        Err(ToolFailure::refused(format!(
            "{existing} events titled \"{FIXTURE_TITLE_PREFIX} ...\" of reserved authors already \
             exist; run clean-load-fixture-events first"
        )))
    }
}

/// The mission exists, is not deleted and is not archived: the checks the attachment repeats
/// under its lock, run here so a dry run reports them.
async fn require_attachable_mission(
    connection: &mut sqlx::PgConnection,
    mission: MissionId,
) -> Result<(), ToolFailure> {
    let state: Option<(bool, String)> =
        sqlx::query_as("SELECT deleted_at IS NOT NULL, status::text FROM missions WHERE id = $1")
            .bind(mission)
            .fetch_optional(connection)
            .await?;
    match state {
        None | Some((true, _)) => Err(ToolFailure::refused(format!(
            "--mission {mission} names no mission"
        ))),
        Some((false, status)) if status == "archived" => Err(ToolFailure::refused(format!(
            "--mission {mission} is archived and cannot be attached"
        ))),
        Some(_) => Ok(()),
    }
}

/// The event carries the table's member-admitting access policy and an open, uncapped member
/// pool, so every verified member of the load population may register.
async fn require_member_admitting(
    connection: &mut sqlx::PgConnection,
    event: EventId,
) -> Result<(), ToolFailure> {
    let admits: bool = sqlx::query_scalar(
        "SELECT e.access_policy = $2::jsonb AND EXISTS (
             SELECT 1 FROM event_reservation_quota_pools pool
             WHERE pool.event_id = e.id AND pool.quota_kind = 'member'
               AND pool.seat_limit IS NULL AND pool.opens_at <= now())
         FROM events e WHERE e.id = $1",
    )
    .bind(event)
    .bind(MEMBER_ADMITTING_POLICY)
    .fetch_one(connection)
    .await?;
    if admits {
        Ok(())
    } else {
        Err(ToolFailure::failed(format!(
            "event {event} was not created with the member-admitting access policy and an open, \
             uncapped member pool; the run is rolled back"
        )))
    }
}
