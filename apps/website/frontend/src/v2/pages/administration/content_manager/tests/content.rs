//! Guards on the content manager: the routes, the boot path, the list failure and the hero upload.

use super::{
    announcement_create_path, announcement_id_path, announcement_list_path, announcement_push_path,
    apply_md_tool, category_tag, date_ymd, doc_from_announcement, is_server_id, tag_category,
};
use serde_json::json;

/// Every category the editor offers, standing orders included, must resolve to a stored tag.
#[test]
fn category_tag_covers_all_ui_categories_including_sop() {
    assert_eq!(category_tag("announcement"), Some("update"));
    assert_eq!(
        category_tag("sop"),
        Some("update"),
        "SOP has no enum variant — closest tag is update (perturbation: return None for sop)"
    );
    assert_eq!(category_tag("event"), Some("event"));
    assert_eq!(category_tag("modpack"), Some("modpack_update"));
    assert_eq!(category_tag("important"), Some("important"));
    assert_eq!(category_tag("nope"), None);
}

#[test]
fn server_id_detects_uuid_not_local_mock() {
    assert!(is_server_id("44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"));
    assert!(!is_server_id("d1"));
    assert!(!is_server_id("new-1710000000000"));
    assert!(!is_server_id(""));
}

#[test]
fn tag_category_round_trips_live_tags() {
    assert_eq!(tag_category("update"), "announcement");
    assert_eq!(tag_category("event"), "event");
    assert_eq!(tag_category("modpack_update"), "modpack");
    assert_eq!(tag_category("important"), "important");
}

#[test]
fn doc_from_announcement_maps_status_and_dates() {
    let row = json!({
        "id": "44fa4c17-5bd5-4c6b-b02d-4ccd52af6910",
        "title": "Live row",
        "body": "hello",
        "tag": "modpack_update",
        "status": "published",
        "published_at": "2026-07-27T12:00:00Z",
        "created_at": "2026-07-01T00:00:00Z",
        "updated_at": "2026-07-27T12:00:00Z",
    });
    let doc = doc_from_announcement(&row).expect("row maps");
    assert_eq!(doc.id, "44fa4c17-5bd5-4c6b-b02d-4ccd52af6910");
    assert_eq!(doc.category, "modpack");
    assert!(doc.published);
    assert_eq!(doc.date, "2026-07-27");
    assert_eq!(date_ymd("2026-06-18T00:00:00Z"), "2026-06-18");
}

#[test]
fn cms_paths_match_axum_routes() {
    assert_eq!(announcement_list_path(), "/cms/announcements?limit=100");
    assert_eq!(announcement_create_path(), "/cms/announcements");
    assert_eq!(
        announcement_id_path("44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"),
        "/cms/announcements/44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"
    );
    assert_eq!(
        announcement_push_path("44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"),
        "/cms/announcements/44fa4c17-5bd5-4c6b-b02d-4ccd52af6910/push-discord"
    );
    let src = crate::v2::core::test_support::fixtures::api_route_source();
    let src: &str = &src;
    assert!(
        src.contains(r#""/cms/announcements""#),
        "the api_v2 domain route tables must register /cms/announcements"
    );
    // The listing must share the route the create uses; a create-only registration refuses it.
    assert!(
        src.contains("get(handlers::announcements_admin::list_cms_announcements)"),
        "the api_v2 community_content route table must register GET on /cms/announcements \
         (perturbation: post-only)"
    );
    assert!(
        src.contains(".post(handlers::announcements_admin::create_announcement)"),
        "the api_v2 community_content route table must register POST on /cms/announcements \
         (perturbation: get-only)"
    );
    assert!(
        src.contains(r#""/cms/announcements/{id}""#),
        "the api_v2 domain route tables must register PATCH|DELETE /cms/announcements/{{id}}"
    );
    assert!(
        src.contains(r#""/cms/announcements/{id}/push-discord""#),
        "the api_v2 domain route tables must register POST …/push-discord"
    );
    assert!(
        src.contains(r#""/cms/uploads""#),
        "the api_v2 domain route tables must still register POST /cms/uploads"
    );
    assert_eq!(super::cms_uploads_path(), "/cms/uploads");
}

/// The screen must fetch the catalogue and hydrate the working set from what comes back.
///
/// What is pinned here is a wiring seam, not a value: a resource inside a component, and an effect
/// that reads its answer and writes a signal. There is no function to call — the seam only exists
/// once a reactive runtime has mounted the component — so this reads the source instead. The half
/// that does have a signature, the row mapping, has its own value tests below; this covers only
/// that the boot path reaches it.
///
/// The source is scrubbed first: comments and string literals blanked, unreachable items removed.
/// Without that, every positive needle here could be satisfied by a commented-out line, and the
/// listing route in particular is discussed in prose a few lines above its own definition.
#[test]
fn content_boots_from_cms_list_not_mock_docs() {
    use crate::v2::core::test_support::class_r_scrub::live_code;
    let prod = live_code(&crate::v2::core::test_support::pins::content_source());
    assert!(
        !prod.contains("fn mock_docs()"),
        "mock_docs must be gone from production (perturbation: restore mock seed helper)"
    );
    assert!(
        !prod.contains("RwSignal::new(mock_docs())"),
        "must not boot the master list from mock_docs alone"
    );
    assert!(
        prod.contains("announcement_list_path"),
        "list path helper must exist"
    );
    assert!(
        prod.contains("LocalResource::new")
            && prod.contains("api_get::<Paginated<Value>>(")
            && prod.contains("announcement_list_path()"),
        "boot must LocalResource api_get the CMS list (perturbation: drop Resource)"
    );
    // B2 — Effect must map `page.data` and write it into `docs`. Needles assembled so
    // this test's source / a free `doc_from_announcement` mention cannot false-green.
    let map_page = format!(
        "{}{}",
        "page.data.iter().filter_map(", "doc_from_announcement)"
    );
    let set_docs = format!("{}{}", "docs.set(", "mapped)");
    assert!(
        prod.contains(&map_page),
        "hydrate Effect must `{map_page}` (perturbation: ignore opt / skip mapping)"
    );
    assert!(
        prod.contains(&set_docs),
        "hydrate Effect must `{set_docs}` (perturbation: hardcoded docs.set / drop apply)"
    );
}

/// Calibration for the boot pin above.
///
/// Every dead-code wrapper in the battery is applied to the two needles that pin cannot do
/// without, and each must stop satisfying it. Without this, an edit that weakened the scrubber
/// would leave the pin green over a catalogue the screen never fetches.
#[test]
fn the_boot_pin_rejects_every_dead_code_wrapper() {
    use crate::v2::core::test_support::class_r_scrub::live_code;
    let needle = "LocalResource::new";
    let attacks: [(&str, String); 12] = [
        (
            "if true == false",
            format!("if true == false {{ {needle}(f); }}"),
        ),
        (
            "loop { break; … }",
            format!("loop {{ break; {needle}(f); }}"),
        ),
        (
            "#[cfg(any())]",
            format!("#[cfg(any())] fn d() {{ {needle}(f); }}"),
        ),
        ("while false", format!("while false {{ {needle}(f); }}")),
        ("if !true", format!("if !true {{ {needle}(f); }}")),
        ("if 1 > 2", format!("if 1 > 2 {{ {needle}(f); }}")),
        (
            "if std::hint::black_box(false)",
            format!("if std::hint::black_box(false) {{ {needle}(f); }}"),
        ),
        (
            "const C: bool = false; if C",
            format!("const C: bool = false;\nfn d() {{ if C {{ {needle}(f); }} }}"),
        ),
        (
            "return; above",
            format!("fn d() {{ return; {needle}(f); }}"),
        ),
        (
            "#[cfg(any())] mod shadow",
            format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}(f); }} }}"),
        ),
        (
            "match guard",
            format!("match () {{ _ if false => {{ {needle}(f); }} _ => {{}} }}"),
        ),
        ("comment", format!("// {needle}(f)")),
    ];
    for (label, body) in attacks {
        let forged = format!("fn content_page() {{\n    {body}\n}}\n#[cfg(test)]\n");
        assert!(
            !live_code(&forged).contains(needle),
            "{label}: the boot needle survived scrubbing — this pin would report a live CMS \
             fetch over code the build never runs"
        );
    }
    // Two attacks the handed-down list does not contain (see arsenal.rs for the full write-up).
    assert!(
        !live_code(&format!(
            "const NEVER: bool = 1 > 2;\nfn d() {{ if NEVER {{ {needle}(f); }} }}\n#[cfg(test)]\n"
        ))
        .contains(needle),
        "A2: a constant folded through a comparison never spells `false` — grepping for \
         `= false` would have shipped this hole"
    );
    assert!(
        !live_code(&format!(
            "#[cfg(all(any(), unix))] fn d() {{ {needle}(f); }}\n#[cfg(test)]\n"
        ))
        .contains(needle),
        "a composite never-true cfg is not the literal `#[cfg(any())]`"
    );
    let live = format!("fn content_page() {{\n    {needle}(f);\n}}\n#[cfg(test)]\n");
    assert!(live_code(&live).contains(needle));
}

/// Strip all whitespace so `set( true )` and `set(true)` compare equal.
fn strip_ws(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Strip TOML `#` comments and `//` line comments, respecting strings, so a commented-out
/// feature line cannot satisfy the manifest pins below.
fn strip_toml_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == '/' && matches!(chars.peek(), Some('/')) {
            chars.next();
            while let Some(n) = chars.next() {
                if n == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if c == '#' {
            while let Some(n) = chars.next() {
                if n == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Strip `//` and `/* */` so comment-only needles cannot satisfy live-call pins.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '*' && matches!(chars.peek(), Some('/')) {
                            chars.next();
                            break;
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

/// Count every `list_seeded.set(` call — any arg form, whitespace-insensitive.
fn count_list_seeded_sets(s: &str) -> usize {
    strip_ws(s).matches("list_seeded.set(").count()
}

/// True-ish seed args after whitespace strip: `true`, `(true)`, `true as bool`, `{ true }`.
fn has_true_list_seed(s: &str) -> bool {
    let t = strip_ws(s);
    t.contains("list_seeded.set(true)")
        || t.contains("list_seeded.set((true))")
        || t.contains("list_seeded.set(trueasbool)")
        || t.contains("list_seeded.set({true})")
}

/// The list fetch must keep its result, seed only on success, and surface a reachable retry.
///
/// Three shapes that an exact-string pin used to miss, and that must each fail here:
/// 1. an early seed outside the success arm while the success arm still seeds;
/// 2. a seed in the failure arm written in any indirect form;
/// 3. the retry control parked inside an unreachable branch.
#[test]
fn content_list_error_does_not_seed_as_empty_success() {
    let prod = crate::v2::core::test_support::pins::content_source();
    let prod: &str = &prod;

    // List Resource must not collapse Err→None (perturbation: restore `.await.ok()`).
    let list_get = "api_get::<Paginated<Value>>(";
    let list_region = prod
        .split("LocalResource::new")
        .nth(1)
        .and_then(|s| s.split("Effect::new").next())
        .expect("LocalResource block before Effect");
    assert!(
        list_region.contains(list_get) && list_region.contains("announcement_list_path()"),
        "list LocalResource must call {list_get} on announcement_list_path()"
    );
    assert!(
        !list_region.contains(".ok()"),
        "list LocalResource must not .ok() the GET (perturbation: Err→None empty success)"
    );

    // Hydrate Effect window (one-shot seed + Err surface) — before retry_list.
    let effect = prod
        .split("Effect::new")
        .nth(1)
        .and_then(|s| s.split("let retry_list").next())
        .expect("hydrate Effect must sit before retry_list");

    // (1) Order pin — any list_seeded.set(...) before Ok(page) must FAIL (incl. spaced).
    let before_ok = effect
        .split("Ok(page)")
        .next()
        .expect("hydrate Effect must match on Ok(page)");
    assert_eq!(
        count_list_seeded_sets(before_ok),
        0,
        "list_seeded.set(...) must not run before Ok(page) \
         (perturbation: spaced/early seed before match / outside Ok)"
    );

    // Ok arm window: Ok(page) … Err(e) — exactly one true-ish seed.
    let ok_arm = effect
        .split("Ok(page)")
        .nth(1)
        .and_then(|s| s.split("Err(e)").next())
        .expect("Ok(page) arm must precede Err(e)");
    assert!(
        has_true_list_seed(ok_arm),
        "Ok arm must set list_seeded to true (perturbation: drop success seed)"
    );
    assert_eq!(
        count_list_seeded_sets(ok_arm),
        1,
        "Ok arm must call list_seeded.set exactly once"
    );
    let set_docs = format!("{}{}", "docs.set(", "mapped)");
    assert!(
        ok_arm.contains(&set_docs),
        "Ok hydrate must `{set_docs}` inside the Ok arm"
    );

    // (2) Err arm — any list_seeded.set form must FAIL (spaced / (true) / as bool / {{}} / seed).
    let err_arm = effect
        .split("Err(e)")
        .nth(1)
        .expect("hydrate Effect must have Err(e) arm");
    assert_eq!(
        count_list_seeded_sets(err_arm),
        0,
        "Err arm must not call list_seeded.set(...) \
         (perturbation: spaced/indirect seed in Err)"
    );
    assert!(
        err_arm.contains("Failed to load announcements") && err_arm.contains("list_error.set"),
        "Err arm must write list_error with the actionable message"
    );

    // Exactly one set in the whole hydrate Effect — and it is inside Ok (above).
    assert_eq!(
        count_list_seeded_sets(effect),
        1,
        "hydrate Effect must call list_seeded.set exactly once (inside Ok only)"
    );
    assert!(
        !strip_ws(prod).contains(&strip_ws(
            "list_seeded.set(true);\n        let Some(page) = opt else"
        )),
        "must not seed then early-return on None (perturbation: restore seed-on-None)"
    );

    // (3) Reachable error UI — dead `.filter(|_| false)` OR `if false { … }` must FAIL.
    let master = prod
        .split("master=view!")
        .nth(1)
        .and_then(|s| s.split("detail=view!").next())
        .expect("master pane must sit before detail");
    let master_ws = strip_ws(master);
    assert!(
        !master_ws.contains("filter(|_|false)"),
        "master pane must not gate UI behind .filter(|_| false) \
         (perturbation: unreachable error UI)"
    );
    assert!(
        !master_ws.contains("iffalse"),
        "master pane must not wrap UI in `if false` / `if false {{ … }}` \
         (perturbation: unreachable Retry without .filter)"
    );
    let err_bind = "if let Some(err) = list_error.get()";
    let err_start = master
        .find(err_bind)
        .unwrap_or_else(|| panic!("master must bind list_error via `{err_bind}`"));
    let empty_i = master
        .find("\"No announcements yet.\"")
        .expect("empty success copy must remain for true zero-row Ok responses");
    assert!(
        err_start < empty_i,
        "reachable list_error if-let must precede empty success copy"
    );
    let err_window = &master[err_start..empty_i];
    assert!(
        err_window.contains("content-list-error")
            && err_window.contains("content-list-retry")
            && err_window.contains("\"Retry\"")
            && err_window.contains("on:click=retry_list"),
        "Retry/error UI must sit inside the reachable list_error if-let branch \
         (perturbation: needles present but unreachable)"
    );
    assert!(
        !strip_ws(err_window).contains("iffalse"),
        "list_error if-let body must not nest `if false` around Retry"
    );
    assert!(
        prod.contains("list_res.refetch()"),
        "Retry handler must refetch the list Resource"
    );
}

/// Source guards: publishing must keep the returned identifier, standing orders must not report a
/// local success, and the markdown controls must change the text rather than report one.
///
/// Reads the production shards only, so this test's own assertion strings cannot satisfy it.
#[test]
fn publish_edit_delete_push_are_wired_no_fake_toasts() {
    let prod = crate::v2::core::test_support::pins::content_source();
    let prod: &str = &prod;
    assert!(
        prod.contains("api_patch::<serde_json::Value>"),
        "re-Publish of a server id must PATCH (perturbation: remove api_patch)"
    );
    assert!(
        prod.contains("api_delete(") && prod.contains("&announcement_id_path("),
        "Delete must call api_delete on /cms/announcements/{{id}}"
    );
    assert!(
        prod.contains("announcement_push_path") && prod.contains("api_post_ok"),
        "re-push must hit …/push-discord via api_post_ok"
    );
    assert!(
        prod.contains(".get(\"id\")"),
        "POST create must read the returned id (perturbation: discard Ok(_) body)"
    );
    assert!(
        prod.contains("is_server_id"),
        "Publish must branch POST vs PATCH on server id"
    );
    assert!(
        !prod.contains("success(\"SOP published\")"),
        "SOP must not toast local-only success (perturbation: restore fake SOP toast)"
    );
    assert!(
        prod.contains("\"announcement\" | \"sop\""),
        "SOP must share the update tag arm with announcement"
    );
    assert!(
        !prod.contains("(mock)"),
        "markdown tools must not toast mock success"
    );
    assert!(
        prod.contains("apply_md_tool"),
        "markdown toolbar must mutate body via apply_md_tool"
    );
    assert!(
        !prod.contains("Hero image upload unavailable"),
        "hero stub toast must be gone (perturbation: restore honest-refusal stub)"
    );
    assert!(
        !prod.contains("Hero image upload coming soon"),
        "old stub success toast must be gone"
    );
    assert!(
        prod.contains("api_upload_file")
            && prod.contains("cms_uploads_path()")
            && prod.contains("thumbnail_url.set"),
        "hero button must POST multipart via api_upload_file and set draft thumbnail_url \
         (perturbation: restore stub toast / drop upload call)"
    );
    assert!(
        prod.contains("\"thumbnail_url\": thumbnail_url.get_untracked()"),
        "Publish payload must carry thumbnail_url (perturbation: omit from json!)"
    );
}

/// The hero upload must be wired end to end: the manifest features, the client helper and this
/// screen's own call. Dropping any one of them fails here even if this screen still looks honest.
///
/// The manifest pins run on comment-stripped text, so commenting a feature out fails. The success
/// path must live-call the absolute-address helper, because the publish endpoint refuses a
/// relative one.
#[test]
fn hero_multipart_upload_is_wired_not_stubbed() {
    let cargo = crate::v2::core::test_support::fixtures::crate_cargo_toml();
    let client = crate::v2::core::test_support::pins::client_source();
    let client: &str = &client;
    let prod = crate::v2::core::test_support::pins::content_source();
    let prod: &str = &prod;
    let prod_code = strip_rust_comments(prod);
    let cargo_live = strip_toml_comments(cargo);
    let client_prod = client;

    assert!(
        cargo_live.contains("\"FormData\"") && cargo_live.contains("\"File\""),
        "Cargo.toml must enable web-sys FormData + File (perturbation: comment-out / drop features)"
    );
    assert!(
        cargo_live.contains("\"FileList\""),
        "Cargo.toml must enable FileList for HtmlInputElement::files() \
         (perturbation: comment-out / drop FileList)"
    );
    assert!(
        client_prod.contains("pub async fn api_upload_file<")
            && client_prod.contains("FormData::new()")
            && client_prod.contains("append_with_blob_and_filename(\"file\""),
        "client must expose api_upload_file with FormData field \"file\" \
         (perturbation: delete helper / rename field)"
    );
    assert!(
        !prod.contains("Hero image upload unavailable — multipart client not wired"),
        "content must not keep the T-267 honest-refusal stub"
    );
    assert!(
        prod.contains("api_upload_file::<serde_json::Value>")
            && prod.contains("success(\"Hero image uploaded\")"),
        "handle_hero must upload and toast success on Ok"
    );
    assert!(
        prod.contains("api_error_message") && prod.contains("Hero upload failed"),
        "handle_hero must toast the real API error on Err"
    );
    // The definition needle and the call needle are deliberately different: matching the bare
    // name would be satisfied by the signature alone.
    assert!(
        prod_code.contains("fn absolute_cms_upload_url"),
        "prod must define absolute_cms_upload_url (perturbation: comment-only / delete helper)"
    );
    assert!(
        prod_code.contains("absolute_cms_upload_url(&raw)"),
        "hero Ok path must live-call absolute_cms_upload_url(&raw) \
         (perturbation: let url = raw; / comment-only call)"
    );
}

#[test]
fn apply_md_tool_inserts_real_markers() {
    assert_eq!(apply_md_tool("", "Bold"), "**bold**");
    assert_eq!(apply_md_tool("hi", "Italic"), "hi *italic*");
    assert!(apply_md_tool("x", "Link").contains("](https://)"));
    assert!(apply_md_tool("x", "List").contains("- item"));
    assert!(apply_md_tool("", "Image").starts_with("![alt]"));
}

/// Calibration: mapping a standing order to no tag again would send publish down the branch that
/// reports a local success. This pins the difference.
#[test]
fn sop_none_mapping_is_detectably_wrong() {
    let live = category_tag("sop");
    let discarded: Option<&str> = None; // the earlier defect: no tag, so a local-only report
    assert_eq!(live, Some("update"));
    assert_ne!(
        live, discarded,
        "mapping SOP to None reintroduces the fake local-only publish path"
    );
}
