use super::*;

/// The identity-link attendance backfill must scope through `(event_id, mission_id)`,
/// matching ingest. Playing one mission on a multi-mission event must NOT flip sibling
/// `event_mission` registrations.
///
/// RED (must FAIL):
///   (a) an `em.event_id IN (SELECT m.event_id …)` nest with no mission join
///   (b) join pin present + `UNION ALL` event-id-only second arm
///   (c) dead `(FALSE AND <join_pin> OR TRUE)` with an event-only JOIN
///   (d) `em.event_id = ANY (...)` + join pin only in a SQL `--` / `/* */` comment
/// GREEN: single live `FROM event_missions em INNER JOIN matches m ON` carrying both
/// equalities; no comment-only pins; no event-wide IN/ANY/UNION bypass.
#[test]
fn backfill_attendance_joins_event_id_and_mission_id() {
    // Assembled so a bait comment / this test's source cannot false-green the const.
    let join_pin_ab = format!(
        "{}{}",
        "m.event_id = em.event_id AND ", "m.mission_id = em.mission_id"
    );
    let join_pin_ba = format!(
        "{}{}",
        "m.mission_id = em.mission_id AND ", "m.event_id = em.event_id"
    );

    let sql = strip_sql_comments(BACKFILL_ATTENDANCE);
    let collapsed = collapse_ws(&sql);

    assert!(
        collapsed.contains("m.mission_id IS NOT NULL"),
        "BACKFILL_ATTENDANCE must refuse event-only matches (no mission to scope)"
    );

    assert!(
        live_event_mission_join_on(&collapsed, &join_pin_ab, &join_pin_ba),
        "BACKFILL_ATTENDANCE must have a single live \
         `FROM event_missions em INNER JOIN matches m ON` carrying both \
         `m.event_id = em.event_id` and `m.mission_id = em.mission_id` \
         (order-flexible; not comment-only; not `(FALSE AND … OR TRUE)`)"
    );

    // Event-wide bypass shapes (whitespace-collapsed, comments already stripped).
    assert!(
        !collapsed.contains("em.event_id IN ("),
        "BACKFILL_ATTENDANCE must not use `em.event_id IN (` — \
         that flips sibling missions on multi-mission events"
    );
    assert!(
        !collapsed.contains("em.event_id = ANY"),
        "BACKFILL_ATTENDANCE must not use `em.event_id = ANY` — \
         event-id-only ANY is the same event-wide bypass as IN"
    );
    assert!(
        !collapsed.contains(" UNION ") && !collapsed.contains(" UNION ALL "),
        "BACKFILL_ATTENDANCE must not UNION an event-id-only arm onto the \
         mission-scoped SELECT em.id subquery"
    );

    // Source pin (production only) — the same live join must appear on the const, not only
    // in docs / SQL comments inside the string. Drop Rust `//` / `///` lines first so a doc
    // mention of an event-only nest cannot false-red the ban.
    const SRC: &str = include_str!("../arma_link_confirmation.rs");
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("split always yields a first slice");
    let prod_sql = strip_sql_comments(&strip_rust_line_comments(production));
    // Rust string continuations leave `\` in the source; treat them as whitespace
    // so the live FROM/JOIN ON shape still matches the compiled const.
    let prod = collapse_ws(&prod_sql.replace('\\', " "));
    assert!(
        live_event_mission_join_on(&prod, &join_pin_ab, &join_pin_ba),
        "production BACKFILL_ATTENDANCE source must contain a live \
         `FROM event_missions em INNER JOIN matches m ON` with both equalities"
    );
    assert!(
        !prod.contains("em.event_id IN ("),
        "production must not keep an `em.event_id IN (` event-wide nest"
    );
    assert!(
        !prod.contains("em.event_id = ANY"),
        "production must not keep an `em.event_id = ANY` event-wide bypass"
    );
    assert!(
        !prod.contains(" UNION ") && !prod.contains(" UNION ALL "),
        "production must not UNION an event-id-only arm into BACKFILL_ATTENDANCE"
    );
    assert!(
        prod.contains("m.mission_id IS NOT NULL"),
        "production BACKFILL_ATTENDANCE must keep `m.mission_id IS NOT NULL`"
    );
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
