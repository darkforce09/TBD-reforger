//! Runner.

use super::*;

/// Shared refuse-empty-write guard: every generated-document writer goes through it.
/// A success path must not overwrite committed content with structurally empty / vacuous output.
pub fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}

/// Regenerate the three sync outputs from `registry`, in this order: the dispatch queue
/// ([`crate::repository::QUEUE_JSON`]), the roadmap's recommended-next-work block between its
/// markers, and the gap-analysis ticket column.
///
/// The two document targets are optional: a checkout without the roadmap or the gap-analysis
/// file, or a roadmap without the start marker, is skipped and never created. The roadmap block
/// refuses a structurally empty write through [`refuse_empty_write`], and the gap-analysis column
/// refuses a table it cannot round-trip byte for byte.
pub fn cmd_sync(root: &Path, registry: &Value) -> Result<()> {
    use crate::repository::documentation as docs;

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
