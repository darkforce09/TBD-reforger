//! Runner.

use super::*;

/// T-383 — shared refuse-empty-write guard (lives under owns so cmds/schema_gates can call it).
/// A success path must not overwrite committed content with structurally empty / vacuous output.
pub fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}

pub fn cmd_sync(root: &Path, registry: &Value) -> Result<()> {
    use crate::repository::documentation as docs;

    fs::create_dir_all(root.join(docs::TREE_DIR))?;

    fs::write(
        root.join(docs::TICKET_REGISTRY_VIEW),
        generate_ticket_registry_md(registry),
    )?;
    fs::write(
        root.join(docs::TICKET_LEAD_VIEW),
        generate_ticket_lead_md(registry),
    )?;
    fs::write(
        root.join(docs::TICKET_DEV_QUEUE_VIEW),
        generate_ticket_dev_queue_md(registry),
    )?;
    fs::write(
        root.join(docs::TICKET_BRAINSTORM_VIEW),
        generate_ticket_brainstorm_md(registry),
    )?;
    fs::write(
        root.join(docs::TICKET_MOD_QUEUE_VIEW),
        generate_ticket_mod_queue_md(registry),
    )?;
    fs::write(
        root.join(docs::MILESTONES),
        generate_milestones_md(registry),
    )?;

    let queue = generate_queue_json(registry);
    write_json_ascii(&root.join(crate::repository::QUEUE_JSON), &queue)?;

    let roadmap = root.join(docs::ROADMAP);
    if roadmap.is_file() {
        let text = fs::read_to_string(&roadmap)?;
        if text.contains(NEXT_MARKER_START) {
            inject_next_block(root, registry)?;
        }
    }

    if root.join(docs::GAP_ANALYSIS).is_file() {
        sync_gap_analysis_ticket_column(root, registry)?;
    }

    println!("sync complete");
    Ok(())
}
