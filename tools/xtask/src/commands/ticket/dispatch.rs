use super::cli::TicketCmd;
use super::execution::{cmd_clean, cmd_done, cmd_run};
use anyhow::Result;
use repository_layout::prelude::find_repository_root;
use ticket_metrics::cmd_metrics;
use ticket_registry::load_registry;
use ticket_registry::sync::cmd_sync;
use ticket_registry::validation::cmd_check;
use ticket_registry::verbs::{
    cmd_add, cmd_add_child, cmd_advance_slice, cmd_brief, cmd_config, cmd_gap_round_trip, cmd_get,
    cmd_list, cmd_mark_ready, cmd_next, cmd_plan_batch, cmd_prompt, cmd_ready_ids, cmd_remove,
    cmd_reorder, cmd_scope_histogram, cmd_set_status, cmd_ship_opt, cmd_show, cmd_sparse_paths,
    cmd_stamp_sha,
};

/// Runs one `ticket` verb. A [`ticket_registry::Error::Refused`] prints its message bare on
/// stderr and exits 1, with no `xtask:` prefix; every other failure goes to the bin's handler.
pub(crate) fn run(cmd: TicketCmd) -> Result<u8> {
    match run_verb(cmd) {
        Err(error) => match error.downcast_ref::<ticket_registry::Error>() {
            Some(ticket_registry::Error::Refused { message }) => {
                eprintln!("{message}");
                Ok(1)
            }
            _ => Err(error),
        },
        answer => answer,
    }
}

fn run_verb(cmd: TicketCmd) -> Result<u8> {
    {
        let root = find_repository_root()?;
        match cmd {
            TicketCmd::Sync => {
                let reg = load_registry(&root)?;
                cmd_sync(&root, &reg)?;
            }
            TicketCmd::Check { strict } => {
                let reg = load_registry(&root)?;
                cmd_check(&root, &reg, strict)?;
            }
            TicketCmd::Brief { id } => {
                let reg = load_registry(&root)?;
                cmd_brief(&root, &reg, &id)?;
            }
            TicketCmd::Prompt { id, slice, header } => {
                let reg = load_registry(&root)?;
                let slice = if slice.is_empty() {
                    None
                } else {
                    Some(slice.as_str())
                };
                cmd_prompt(&root, &reg, &id, slice, header)?;
            }
            TicketCmd::Show { id } => {
                let reg = load_registry(&root)?;
                cmd_show(&reg, &id)?;
            }
            TicketCmd::Next => {
                let reg = load_registry(&root)?;
                cmd_next(&reg)?;
            }
            TicketCmd::List => {
                let reg = load_registry(&root)?;
                cmd_list(&root, &reg)?;
            }
            TicketCmd::PlanBatch => {
                let reg = load_registry(&root)?;
                cmd_plan_batch(&reg)?;
            }
            TicketCmd::SparsePaths { id } => {
                let reg = load_registry(&root)?;
                cmd_sparse_paths(&reg, &id)?;
            }
            TicketCmd::GapRoundTrip => {
                cmd_gap_round_trip(&root)?;
            }
            TicketCmd::Add {
                title,
                program,
                surfaces,
                impact,
                summary,
            } => {
                let mut reg = load_registry(&root)?;
                cmd_add(
                    &root, &mut reg, &title, &program, &surfaces, &impact, &summary,
                )?;
            }
            TicketCmd::AddChild {
                parent,
                title,
                summary,
                promote,
            } => {
                let mut reg = load_registry(&root)?;
                cmd_add_child(&root, &mut reg, &parent, &title, &summary, promote)?;
            }
            TicketCmd::Remove { id, force } => {
                let mut reg = load_registry(&root)?;
                cmd_remove(&root, &mut reg, &id, force)?;
            }
            TicketCmd::Reorder { id, after } => {
                let mut reg = load_registry(&root)?;
                cmd_reorder(&root, &mut reg, &id, &after)?;
            }
            TicketCmd::Ship { id, no_repack } => {
                let mut reg = load_registry(&root)?;
                cmd_ship_opt(&root, &mut reg, &id, !no_repack)?;
            }
            // No registry pre-load and no check preflight: stamp-sha is the verb
            // that moves the transiently gate-red ship→commit window back to
            // green — a full-check preflight would deadlock it (cmd doc).
            TicketCmd::StampSha { id, sha } => {
                cmd_stamp_sha(&root, &id, &sha)?;
            }
            TicketCmd::MarkReady { id, spec, plan } => {
                let mut reg = load_registry(&root)?;
                cmd_mark_ready(&root, &mut reg, &id, spec.as_deref(), plan.as_deref())?;
            }
            TicketCmd::AdvanceSlice { id } => {
                let mut reg = load_registry(&root)?;
                cmd_advance_slice(&root, &mut reg, &id)?;
            }
            TicketCmd::ReadyIds { limit, stream } => {
                let reg = load_registry(&root)?;
                let stream = if stream.is_empty() {
                    None
                } else {
                    Some(stream.as_str())
                };
                cmd_ready_ids(&root, &reg, limit, stream)?;
            }
            TicketCmd::SetStatus { id, status } => {
                let mut reg = load_registry(&root)?;
                cmd_set_status(&root, &mut reg, &id, &status)?;
            }
            TicketCmd::Get { id, field } => {
                let reg = load_registry(&root)?;
                cmd_get(&reg, &id, field.as_deref())?;
            }
            TicketCmd::Config { key } => {
                let reg = load_registry(&root)?;
                cmd_config(&root, &reg, &key)?;
            }
            TicketCmd::Run { dry_run, stream } => {
                let reg = load_registry(&root)?;
                cmd_run(&root, &reg, dry_run, stream.as_deref())?;
            }
            TicketCmd::Done { id } => {
                let mut reg = load_registry(&root)?;
                cmd_done(&root, &mut reg, &id)?;
            }
            TicketCmd::Clean { id } => {
                let reg = load_registry(&root)?;
                cmd_clean(&root, &reg, &id)?;
            }
            TicketCmd::Metrics { by } => {
                cmd_metrics(&root, by.as_deref())?;
            }
            // No registry pre-load: the histogram reads the typed corpus directly.
            TicketCmd::ScopeHistogram => {
                cmd_scope_histogram(&root)?;
            }
        }
        Ok(0)
    }
}
