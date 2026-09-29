//! Source pins for the PATCH clear contract, the duplicate-attach mapping, and the event authoring
//! services both routes write through.
//!
//! Empty-string clear for briefing/banner is a *live* PATCH writer shape, not a doc-comment
//! phrase, so every assertion below runs against source with `//` and `/* */` stripped — a
//! comment mentioning the behaviour cannot green a deleted code path.

const CREATE_UPDATE: &str = include_str!("../event_create_update.rs");
const ATTACHMENT: &str = include_str!("../event_mission_attachment.rs");
const EVENT_CREATION: &str = include_str!("../../services/event_authoring/event_creation.rs");
const MISSION_ATTACHMENT: &str =
    include_str!("../../services/event_authoring/mission_attachment.rs");

fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            } else {
                i = bytes.len();
            }
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn production_half(source: &str) -> &str {
    source
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests module")
}

#[test]
fn patch_empty_string_clear_writers_and_attach_maps_unique() {
    let production = production_half(CREATE_UPDATE);
    let code = collapse_ws(&strip_rust_comments(production));

    // Shape: key absent = leave alone; `""` = clear. That requires plain
    // `Option<String>` (not `present_option` / `Option<Option<_>>`) so JSON `""`
    // deserializes to `Some("")` and reaches the SQL writer.
    let patch = production
        .split("pub struct PatchEventInput")
        .nth(1)
        .expect("PatchEventInput")
        .split("pub async fn update_event")
        .next()
        .expect("struct body");
    let patch_code = collapse_ws(&strip_rust_comments(patch));
    assert!(
        patch_code.contains("briefing: Option<String>")
            && patch_code.contains("banner_image_url: Option<String>"),
        "PatchEventInput briefing/banner must be Option<String> so \"\" clears \
         (fails with: present_option / Option<Option<_>> / drop fields)"
    );
    assert!(
        !patch_code.contains("briefing: Option<Option")
            && !patch_code.contains("banner_image_url: Option<Option"),
        "briefing/banner must not use present_option Option<Option<_>> \
         (fails with: treat null as clear like server_id)"
    );

    // Live UPDATE writers — empty Some(\"\") must still push the column.
    assert!(
        code.contains("if let Some(b) = &input.briefing")
            && code.contains("qb.push(\", briefing = \").push_bind(b.clone())"),
        "update_event must WRITE briefing when the key is present (incl. \"\") \
         (fails with: drop the briefing qb.push arm)"
    );
    assert!(
        code.contains("qb.push(\", banner_image_url = \").push_bind(u.clone())"),
        "update_event must WRITE banner_image_url when the key is present (incl. \"\") \
         (fails with: drop the banner qb.push arm)"
    );
    // Banner clear path goes through the validator — empty must be Ok, not 400. The validator
    // lives in the event creation service, which both the POST and the PATCH routes apply.
    let creation_code = collapse_ws(&strip_rust_comments(production_half(EVENT_CREATION)));
    assert!(
        creation_code.contains("pub fn validated_banner_image_url")
            && creation_code.contains("trimmed.is_empty() || is_http_url(trimmed)"),
        "validated_banner_image_url must accept empty (clear) or http(s) \
         (fails with: refuse empty → \"\" clear 400s)"
    );
    assert!(
        code.contains(".map(validated_banner_image_url)"),
        "PATCH must route banner_image_url through validated_banner_image_url \
         (fails with: write raw / skip validator)"
    );

    let add = production_half(ATTACHMENT)
        .split("pub async fn add_event_mission")
        .nth(1)
        .expect("add_event_mission")
        .split("\npub async fn ")
        .next()
        .expect("handler body");
    let add_code = collapse_ws(&strip_rust_comments(add));
    assert!(
        add_code.contains("attach_mission(")
            && add_code.contains("AttachmentAuthority::AdministratorSession")
            && !add_code.contains("INSERT INTO"),
        "add_event_mission must write through the mission attachment service for the \
         administrator session (fails with: an attach writer inside the handler)"
    );
    let attach = production_half(MISSION_ATTACHMENT)
        .split("pub async fn attach_mission")
        .nth(1)
        .expect("attach_mission")
        .split("\n}\n")
        .next()
        .expect("service body");
    let attach_code = collapse_ws(&strip_rust_comments(attach));
    assert!(
        attach_code.contains("is_unique_violation")
            && attach_code.contains("already attached to this event"),
        "attach_mission must map idx_event_mission unique violations to a 409 \
         (fails with: drop is_unique_violation arm → 500 on duplicate attach)"
    );
}

/// `POST /api/v1/events` writes through the event creation service, the path the host tool's
/// fixture events take too, and inserts no event row itself.
#[test]
fn create_event_writes_through_the_event_creation_service() {
    let handler = production_half(CREATE_UPDATE)
        .split("pub async fn create_event")
        .nth(1)
        .expect("create_event")
        .split("\npub async fn ")
        .next()
        .expect("handler body");
    let handler_code = collapse_ws(&strip_rust_comments(handler));
    assert!(
        handler_code.contains("EventCreation::new(")
            && handler_code.contains("event_creation::create_event(&mut tx, &creation,"),
        "create_event must validate and write through event_creation"
    );
    assert!(
        !collapse_ws(&strip_rust_comments(production_half(CREATE_UPDATE)))
            .contains("INSERT INTO events"),
        "the handler module must not insert events itself"
    );
}
