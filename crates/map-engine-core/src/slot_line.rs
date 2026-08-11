//! T-180.7 — ORBAT Manager slot line formatter (`N: Role (weapons) | TAG`).
//!
//! Pure string helper (always available — no `doc` / `mission` feature gate) so
//! `cargo test -p map-engine-core format_slot_line` matches without feature flags.
//!
//! T-801 — map **tether** (leader→member hairline) drag-preview packing is NOT here; it lives in
//! [`crate::squad_links::pack_squad_link_drag_preview`]. This module remains the ORBAT text line.

/// Format a 1-based ORBAT slot line for the Stitch manager tree.
///
/// ```text
/// "{index}: {role}{weapons?}{tag?}{sl?}"
/// weapons = "(" + primary + optional " + " + launcher + ")"
/// ```
///
/// Weapon display prefers explicit `primary` / `launcher`. When those are absent and `summary`
/// contains `" · "`, the first segment is primary and the second (if present) is launcher.
/// `is_leader` appends `" | SL"` without mutating the tag field.
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

fn resolve_weapons<'a>(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// G3 — primary + launcher → `(Primary + Launcher)`.
    #[test]
    fn format_slot_line_primary_and_launcher() {
        let s = format_slot_line(
            1,
            "Squad Leader",
            None,
            Some("L85A3"),
            Some("GL"),
            None,
            false,
        );
        assert_eq!(s, "1: Squad Leader (L85A3 + GL)");
    }

    /// G3 — tag MED preserved; not overwritten by SL.
    #[test]
    fn format_slot_line_tag_med() {
        let s = format_slot_line(2, "Medic", None, Some("L85A3"), None, Some("MED"), false);
        assert_eq!(s, "2: Medic (L85A3) | MED");
    }

    /// G3 — is_leader appends `| SL` without clearing tag.
    #[test]
    fn format_slot_line_is_leader() {
        let s = format_slot_line(
            1,
            "Squad Leader",
            Some("L85A3 · GL"),
            None,
            None,
            Some("MED"),
            true,
        );
        assert_eq!(s, "1: Squad Leader (L85A3 + GL) | MED | SL");
    }

    #[test]
    fn format_slot_line_summary_dot_split() {
        let s = format_slot_line(
            3,
            "Rifleman (AT)",
            Some("L85A3 · NLAW"),
            None,
            None,
            None,
            false,
        );
        assert_eq!(s, "3: Rifleman (AT) (L85A3 + NLAW)");
    }

    #[test]
    fn format_slot_line_no_weapons() {
        let s = format_slot_line(1, "Rifleman", None, None, None, None, false);
        assert_eq!(s, "1: Rifleman");
    }
}
