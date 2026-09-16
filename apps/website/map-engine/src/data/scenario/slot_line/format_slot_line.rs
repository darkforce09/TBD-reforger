//! Role: format slot line.
//! Position: `slot_line` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Format a 1-based ORBAT slot line for the Stitch manager tree.
#[must_use]
pub fn format_slot_line(
    index_1based: u32,
    role: &str,
    summary: Option<&str>,
    primary: Option<&str>,
    launcher: Option<&str>,
    tag: Option<&str>,
    is_leader: bool,
) -> String {
    let (prim, launch) = resolve_weapons(summary, primary, launcher);
    let mut out = format!("{index_1based}: {role}");
    if let Some(p) = prim {
        if let Some(l) = launch {
            out.push_str(&format!(" ({p} + {l})"));
        } else {
            out.push_str(&format!(" ({p})"));
        }
    }
    if let Some(t) = tag.map(str::trim).filter(|s| !s.is_empty()) {
        out.push_str(" | ");
        out.push_str(t);
    }
    if is_leader {
        out.push_str(" | SL");
    }
    out
}

/// Resolve weapons using the supplied domain data.
pub(super) fn resolve_weapons<'a>(
    summary: Option<&'a str>,
    primary: Option<&'a str>,
    launcher: Option<&'a str>,
) -> (Option<&'a str>, Option<&'a str>) {
    let prim = primary.map(str::trim).filter(|s| !s.is_empty());
    let launch = launcher.map(str::trim).filter(|s| !s.is_empty());
    if prim.is_some() || launch.is_some() {
        return (prim, launch);
    }
    let Some(sum) = summary.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None);
    };
    if let Some((a, b)) = sum.split_once(" · ") {
        let a = a.trim();
        let b = b.trim();
        let prim = (!a.is_empty()).then_some(a);
        let launch = (!b.is_empty()).then_some(b);
        (prim, launch)
    } else {
        (Some(sum), None)
    }
}
