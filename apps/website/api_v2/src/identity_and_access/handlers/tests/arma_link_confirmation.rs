use crate::identity_and_access::services::identity_linking::BACKFILL_ATTENDANCE;

/// Attendance attribution selects the event mission that matches both recorded keys.
#[test]
fn backfill_attendance_joins_event_id_and_mission_id() {
    assert_eq!(validate_attendance_scope(BACKFILL_ATTENDANCE), Ok(()));
    const SOURCE: &str = include_str!("../../services/identity_linking.rs");
    assert!(
        invokes_attendance_query(SOURCE),
        "production must execute the checked BACKFILL_ATTENDANCE constant"
    );
}

/// Inspect the compiled SQL independently of other queries in the Rust module.
fn validate_attendance_scope(sql: &str) -> Result<(), &'static str> {
    let collapsed = collapse_ws(&strip_sql_comments(sql));
    let join_pin_ab = "m.event_id = em.event_id AND m.mission_id = em.mission_id";
    let join_pin_ba = "m.mission_id = em.mission_id AND m.event_id = em.event_id";
    if !collapsed.contains("m.mission_id IS NOT NULL") {
        return Err("attendance requires a match mission identity");
    }
    if !live_event_mission_join_on(&collapsed, join_pin_ab, join_pin_ba) {
        return Err("attendance requires a live event-and-mission join");
    }
    if collapsed.contains("em.event_id IN (") || collapsed.contains("em.event_id = ANY") {
        return Err("event-only IN/ANY attribution admits sibling missions");
    }
    if collapsed.contains(" UNION ") {
        return Err("attendance cannot append an event-only UNION arm");
    }
    Ok(())
}

fn invokes_attendance_query(source: &str) -> bool {
    collapse_ws(&strip_rust_line_comments(source)).contains("sqlx::query(BACKFILL_ATTENDANCE)")
}

#[test]
fn attendance_scope_rejects_removed_mission_join_and_missing_mission_identity() {
    for invalid in [
        BACKFILL_ATTENDANCE.replace(" AND m.mission_id = em.mission_id", ""),
        BACKFILL_ATTENDANCE.replace(" AND m.mission_id IS NOT NULL", ""),
    ] {
        assert!(validate_attendance_scope(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn attendance_scope_rejects_union_event_only_arm() {
    for operator in ["UNION", "UNION ALL"] {
        let invalid = format!(
            "{} {operator} SELECT em.id FROM event_missions em INNER JOIN matches m \
             ON m.event_id = em.event_id WHERE m.event_id IS NOT NULL)",
            BACKFILL_ATTENDANCE.trim_end_matches(')')
        );
        assert!(validate_attendance_scope(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn attendance_scope_rejects_dead_join_pin_or_true_bypass() {
    let pin = "m.event_id = em.event_id AND m.mission_id = em.mission_id";
    for condition in [
        format!("(FALSE AND {pin} OR TRUE)"),
        format!("{pin} OR TRUE"),
    ] {
        let invalid = BACKFILL_ATTENDANCE.replace(pin, &condition);
        assert!(validate_attendance_scope(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn attendance_scope_rejects_join_pin_present_only_in_sql_comments() {
    let pin = "m.event_id = em.event_id AND m.mission_id = em.mission_id";
    let event_only = BACKFILL_ATTENDANCE.replace(pin, "m.event_id = em.event_id");
    for invalid in [
        format!("-- {pin}\n{event_only}"),
        format!("/* {pin} */ {event_only}"),
        BACKFILL_ATTENDANCE.replace(
            " AND m.mission_id = em.mission_id",
            " /* AND m.mission_id = em.mission_id */",
        ),
    ] {
        assert!(validate_attendance_scope(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn attendance_scope_rejects_event_only_in_and_any_bypasses() {
    let without_mission_join = BACKFILL_ATTENDANCE.replace(" AND m.mission_id = em.mission_id", "");
    for predicate in [
        "em.event_id IN (SELECT m.event_id FROM matches m)",
        "em.event_id = ANY (SELECT m.event_id FROM matches m)",
    ] {
        // A remaining valid join cannot legitimize a separate event-wide branch.
        for sql in [BACKFILL_ATTENDANCE, without_mission_join.as_str()] {
            let invalid = sql.replace(
                "WHERE s.arma_id = $2",
                &format!("WHERE {predicate} OR s.arma_id = $2"),
            );
            assert!(validate_attendance_scope(&invalid).is_err(), "{invalid}");
        }
    }
}

#[test]
fn attendance_scope_accepts_reordered_live_join_and_sql_comments() {
    let reversed = BACKFILL_ATTENDANCE.replace(
        "m.event_id = em.event_id AND m.mission_id = em.mission_id",
        "m.mission_id = em.mission_id AND m.event_id = em.event_id",
    );
    assert_eq!(validate_attendance_scope(&reversed), Ok(()));
    let documented = format!(
        "-- em.event_id IN (SELECT event_id FROM matches)\n\
         /* UNION ALL SELECT em.id FROM event_missions em */ {BACKFILL_ATTENDANCE}"
    );
    assert_eq!(validate_attendance_scope(&documented), Ok(()));
}

#[test]
fn attendance_query_invocation_ignores_comments_and_unrelated_module_sql() {
    assert!(!invokes_attendance_query(
        "// sqlx::query(BACKFILL_ATTENDANCE)\n/// sqlx::query(BACKFILL_ATTENDANCE)"
    ));
    assert!(invokes_attendance_query(
        "let affected = sqlx::query(\"SELECT discord_id FROM match_player_stats UNION SELECT discord_id FROM users\");\n\
         let attended = sqlx::query(BACKFILL_ATTENDANCE);"
    ));
}

/// Drop whole-line Rust `//` / `///` comments from a production source slice.
fn strip_rust_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Strip SQL `-- …` (to EOL) and `/* … */` before the pins so a join equality that lives
/// only inside a comment cannot false-green the gate.
fn strip_sql_comments(sql: &str) -> String {
    let chars: Vec<char> = sql.chars().collect();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Require one contiguous `FROM event_missions em INNER JOIN matches m ON <body>`
/// whose ON body carries both equalities (AND, either order) and is not killed by
/// a dead `(FALSE AND … OR TRUE)` wrapper.
fn live_event_mission_join_on(collapsed: &str, pin_ab: &str, pin_ba: &str) -> bool {
    let marker = "FROM event_missions em INNER JOIN matches m ON ";
    let Some(rest) = collapsed.split(marker).nth(1) else {
        return false;
    };
    let on_end = rest
        .find(" INNER JOIN ")
        .or_else(|| rest.find(" LEFT JOIN "))
        .or_else(|| rest.find(" WHERE "))
        .unwrap_or(rest.len());
    let on_body = &rest[..on_end];
    if on_body.contains("FALSE") || on_body.contains("OR TRUE") {
        return false;
    }
    on_body.contains(pin_ab) || on_body.contains(pin_ba)
}
