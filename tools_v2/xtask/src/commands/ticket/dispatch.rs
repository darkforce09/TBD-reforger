use super::cli::TicketCmd;
use crate::commands::ticket::*;
use crate::core::repository_root::find_repo_root;
use anyhow::Result;

pub(crate) fn run(cmd: TicketCmd) -> Result<u8> {
    {
        let root = find_repo_root()?;
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
            TicketCmd::Milestone { milestone } => {
                let reg = load_registry(&root)?;
                cmd_milestone(&reg, &milestone)?;
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
            // No load_registry on either arm: migrate-v2 must run BEFORE the tree
            // parses as v2 (the registry loader would refuse the v1 files), and
            // scope-histogram reads the typed corpus directly.
            TicketCmd::MigrateV2 => {
                cmd_migrate_v2(&root)?;
            }
            TicketCmd::ScopeHistogram => {
                cmd_scope_histogram(&root)?;
            }
            // Like migrate-v2: no registry pre-load — the pass itself reloads and
            // regenerates the sync surface after the write.
            TicketCmd::QuarantineWalls => {
                cmd_quarantine_walls(&root)?;
            }
            // No registry pre-load either: the miner reads git metadata + the
            // typed corpus directly, and stamps feed no generated view.
            TicketCmd::BackfillStamps => {
                cmd_backfill_stamps(&root)?;
            }
            // Same shape as backfill-stamps: git metadata + typed corpus only;
            // estimates and markers feed no generated view.
            TicketCmd::EstimateTokens => {
                cmd_estimate_tokens(&root)?;
            }
            // T-920.1: typed corpus only; main_goal and the body lists feed no
            // generated view and no wave.lock input — no sync, no repack.
            TicketCmd::MigrateMainGoal => {
                cmd_migrate_main_goal(&root)?;
            }
        }
        Ok(0)
    }
}
