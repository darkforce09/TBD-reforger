//! `seed-load-fixture-events` and `clean-load-fixture-events`: the events the load population
//! registers on during a staging load run, and their removal.
//!
//! **Role:** parses the two subcommands' flags and hands each run its context: the seeding
//! creates ten `[Load fixture]` events with a 2 × 8 × 8 ORBAT of the mission `--mission` names,
//! and the cleaning deletes them with everything registered on them.
//!
//! **Position:** two rows of the subcommand table in `main.rs`. The shape of the fixture lives in
//! `fixture_plan.rs`, the writes in `fixture_seeding.rs` and `fixture_cleaning.rs`; the seeding
//! writes through the event authoring services of `website_api::operations::services`, the same
//! code the administrator routes run.
//!
//! **Signals & state:** none of its own; each run is one database transaction.
//!
//! **Invariants:** flags parse and the fixture ORBAT passes the attachment checks before any
//! guard connects; a run without `--apply` writes nothing; the seeding needs the load population,
//! whose first account authors the events, and the cleaning deletes only `[Load fixture]` events
//! of reserved authors.

mod fixture_cleaning;
mod fixture_plan;
mod fixture_seeding;

use uuid::Uuid;

use crate::argument_list::ArgumentList;
use crate::guarded_context::ParsedSubcommand;
use crate::tool_failure::ToolFailure;

/// Parse `--mission <uuid>` and check the fixture ORBAT before any guard runs.
pub(crate) fn parse_seed(arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    let raw = arguments.required("--mission")?;
    let mission = Uuid::parse_str(&raw).map_err(|_| {
        ToolFailure::refused(format!(
            "--mission takes a mission id (a UUID), not `{raw}`"
        ))
    })?;
    fixture_plan::fixture_template()?;
    Ok(Box::new(move |context| {
        Box::pin(fixture_seeding::seed(context, mission))
    }))
}

/// The cleaning takes no flags of its own.
pub(crate) fn parse_clean(_arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    Ok(Box::new(|context| {
        Box::pin(fixture_cleaning::clean(context))
    }))
}
