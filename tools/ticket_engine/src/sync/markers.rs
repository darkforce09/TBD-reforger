//! Markers.

use super::*;

pub(super) fn inject_marker_block(path: &Path, start: &str, end: &str, inner: &str) -> Result<()> {
    let text = fs::read_to_string(path)?;
    if !text.contains(start) || !text.contains(end) {
        bail!("Missing markers in {}: {} / {}", path.display(), start, end);
    }
    let inner_r = inner.trim_end();
    // Never collapse a marker to whitespace / bare heading with no body lines.
    refuse_empty_write(
        &format!("marker {}", path.display()),
        marker_inner_is_vacuous(inner_r),
        "inner block is structurally empty (would collapse committed marker content)",
    )?;
    let (before, rest) = text.split_once(start).unwrap();
    let (_, after) = rest.split_once(end).unwrap();
    let new_text = format!("{before}{start}\n{inner_r}\n{end}{after}");
    fs::write(path, new_text)?;
    Ok(())
}

/// Vacuous = blank, or only a markdown heading + blank lines (no bullet / bold body).
pub(super) fn marker_inner_is_vacuous(inner: &str) -> bool {
    let substantive: Vec<&str> = inner
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        // De Morgan of `!(starts_with('#') && !contains("**"))`: keep a line unless it is a bare
        // heading. A heading carrying bold body text is substantive and still counts.
        .filter(|l| !l.starts_with('#') || l.contains("**"))
        .collect();
    substantive.is_empty()
}

pub(super) fn inject_next_block(root: &Path, registry: &Value) -> Result<()> {
    let all = tickets(registry);
    refuse_empty_write(
        "ROADMAP next block",
        all.is_empty(),
        "registry.tickets missing or empty — would collapse ROADMAP marker to bare heading",
    )?;
    let mut open_t: Vec<&Value> = all
        .iter()
        .filter(|t| {
            matches!(
                opt_str(t, "status"),
                Some("ready" | "queued" | "running" | "review")
            ) && order_truthy(t)
        })
        .collect();
    open_t.sort_by_key(|t| {
        (
            order_or(t, 9999),
            crate::store::ticket_id_order_key(str_field(t, "id")),
        )
    });
    // Bare "### Recommended next work" with zero bullets is a vacuous overwrite.
    refuse_empty_write(
        "ROADMAP next block",
        open_t.is_empty(),
        "no ready/queued/running/review tickets — refusing bare-heading collapse",
    )?;
    let mut lines = vec![
        "### Recommended next work (auto-generated)".into(),
        "".into(),
    ];
    for t in open_t.into_iter().take(10) {
        lines.push(format!(
            "- **{}** — {} ({})",
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or(""),
            opt_str(t, "status").unwrap_or(""),
        ));
    }
    inject_marker_block(
        &root.join(crate::repository::documentation::ROADMAP),
        NEXT_MARKER_START,
        NEXT_MARKER_END,
        &lines.join("\n"),
    )
}
