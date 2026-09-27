//! The doctrine wiki through the real HTTP router: the markup service's typed blocks, the
//! save-time refusals and the render-time degradation, the revision history and its paging, the
//! optimistic `base_revision` rules, the request limits, the tiers, the transactional revision and
//! audit writes, and the page-to-revision invariant.
//!
//! Each case owns its administrator, its member and its slugs, so every row it counts is its own,
//! and requires the isolated PostgreSQL harness. Live answers are checked against
//! `wiki-page.schema.json` and refusals against the content error envelope of
//! `content-upload.schema.json`.
//!
//! ## What makes this fail (non-vacuity)
//! Skip the `page_store::record_current_revision` call in `save_wiki_page`
//! (`src/community_content/handlers/wiki_knowledgebase/save.rs`): the saves still answer 201 and
//! 200, but the history stays empty and `wiki_features_each_save_appends_a_revision_newest_first`
//! goes red.

use axum::http::StatusCode;
use serde_json::{Value, json};

mod common;
mod contract_support;
mod wiki_support;

use contract_support::{assert_invalid, assert_valid};
use wiki_support::{
    MAX_BODY_BYTES, WIKI_CONTRACT, WikiSuite, assert_refusal, callout, clear_failure, findings,
    heading, inject_failure, link, markdown, nested_quotes, objects, page_uri, paragraph,
    revisions_uri, save_body, text, unique_slug,
};

const SUITE: &str = "wiki_features";
/// Every key a page summary may carry.
const SUMMARY_KEYS: [&str; 7] = [
    "slug",
    "category",
    "title",
    "icon",
    "nav_order",
    "revision",
    "updated_at",
];

/// Saves `body_md` as a new page and answers its blocks after checking that the save, the article
/// read and the revision read all carry the same blocks and echo the source unchanged.
async fn round_trip(suite: &WikiSuite, label: &str, body_md: &str) -> Value {
    let slug = unique_slug(label);
    let saved = suite.create(&slug, "Round trip", body_md).await;
    assert_eq!(saved["body_md"], body_md);
    let (status, article) = suite.read(&page_uri(&slug)).await;
    assert_eq!(status, StatusCode::OK, "{article}");
    assert_valid(WIKI_CONTRACT, Some("WikiArticle"), &article);
    assert_eq!(article["blocks"], saved["blocks"]);
    assert_eq!(article["body_md"], body_md);
    let (status, revision) = suite.read(&format!("{}/1", revisions_uri(&slug))).await;
    assert_eq!(status, StatusCode::OK, "{revision}");
    assert_valid(WIKI_CONTRACT, Some("WikiRevision"), &revision);
    assert_eq!(revision["blocks"], saved["blocks"]);
    saved["blocks"].clone()
}

/// Asserts a save of `body_md` is refused with `422 wiki_markup_refused` and stores nothing;
/// answers the `(line, code)` of its findings.
async fn refused_findings(suite: &WikiSuite, label: &str, body_md: &str) -> Vec<(i64, String)> {
    let slug = unique_slug(label);
    let (status, body) = suite.save(&slug, save_body("Refused", body_md, None)).await;
    assert_refusal(
        status,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        Some("wiki_markup_refused"),
    );
    let stored = suite.stored(&slug).await;
    assert!(
        stored.page.is_none() && stored.revisions.is_empty() && stored.audits.is_empty(),
        "a refused save stores nothing: {stored:?}"
    );
    findings(&body)
}

#[tokio::test]
async fn wiki_features_block_types_round_trip_from_body_md() {
    let suite = WikiSuite::new(SUITE).await;
    let body_md = markdown(&[
        "# Field Manual",
        "## Radio Nets",
        "### Radio Nets",
        "#### Fire *Support*",
        "##### Casualty Care",
        "###### Radio Nets",
        "",
        "Plain paragraph.",
        "",
        "---",
        "",
        "```sqf linenos",
        "hint \"ready\";",
        "```",
        "",
        "> Brevity beats politeness.",
        "",
        "| Role | Kit | Count | Notes |",
        "| :--- | :-: | ---: | --- |",
        "| Rifleman | M16A2 | 4 | spare |",
        "",
        "- [x] radios checked",
        "- [ ] batteries packed",
        "",
        "3. gamma",
        "4. delta",
    ]);
    let blocks = round_trip(&suite, "blocks", &body_md).await;
    let cell = |run: &str| json!([text(run)]);
    let expected = json!([
        heading(1, "field-manual", json!([text("Field Manual")])),
        heading(2, "radio-nets", json!([text("Radio Nets")])),
        heading(3, "radio-nets-2", json!([text("Radio Nets")])),
        heading(
            4,
            "fire-support",
            json!([text("Fire "), { "type": "emphasis", "children": [text("Support")] }])
        ),
        heading(5, "casualty-care", json!([text("Casualty Care")])),
        heading(6, "radio-nets-3", json!([text("Radio Nets")])),
        paragraph("Plain paragraph."),
        { "type": "rule" },
        { "type": "code", "language": "sqf linenos", "text": "hint \"ready\";\n" },
        { "type": "quote", "blocks": [paragraph("Brevity beats politeness.")] },
        {
            "type": "table",
            "alignments": ["left", "center", "right", "none"],
            "header": [cell("Role"), cell("Kit"), cell("Count"), cell("Notes")],
            "rows": [[cell("Rifleman"), cell("M16A2"), cell("4"), cell("spare")]],
        },
        {
            "type": "list",
            "ordered": false,
            "items": [
                { "checked": true, "blocks": [paragraph("radios checked")] },
                { "checked": false, "blocks": [paragraph("batteries packed")] },
            ],
        },
        {
            "type": "list",
            "ordered": true,
            "start": 3,
            "items": [{ "blocks": [paragraph("gamma")] }, { "blocks": [paragraph("delta")] }],
        },
    ]);
    assert_eq!(blocks, expected);
}

#[tokio::test]
async fn wiki_features_inline_types_round_trip_from_body_md() {
    let suite = WikiSuite::new(SUITE).await;
    let body_md = markdown(&[
        "Read the [field manual](/wiki/field-manual), the [Reforger site](https://reforger.armaplatform.com), \
         the [plain site](http://example.com/plain), the [contents](#contents) or [mail staff](mailto:staff@example.com).",
        "",
        "**strong** *emphasis* ~~struck~~ `inline code`",
        "",
        "![Map of Everon](https://example.com/everon.png \"Everon\") ![Local badge](/uploads/badge.png)",
        "",
        "First line  ",
        "second line",
        "third line",
        "",
        "See <https://example.com/auto> or <ops@example.com>, then go [home](/).",
    ]);
    let blocks = round_trip(&suite, "inlines", &body_md).await;
    let expected = json!([
        {
            "type": "paragraph",
            "inlines": [
                text("Read the "),
                link("/wiki/field-manual", false, "field manual"),
                text(", the "),
                link("https://reforger.armaplatform.com", true, "Reforger site"),
                text(", the "),
                link("http://example.com/plain", true, "plain site"),
                text(", the "),
                link("#contents", false, "contents"),
                text(" or "),
                link("mailto:staff@example.com", true, "mail staff"),
                text("."),
            ],
        },
        {
            "type": "paragraph",
            "inlines": [
                { "type": "strong", "children": [text("strong")] },
                text(" "),
                { "type": "emphasis", "children": [text("emphasis")] },
                text(" "),
                { "type": "strikethrough", "children": [text("struck")] },
                text(" "),
                { "type": "code", "text": "inline code" },
            ],
        },
        {
            "type": "paragraph",
            "inlines": [
                {
                    "type": "image",
                    "src": "https://example.com/everon.png",
                    "alt": "Map of Everon",
                    "title": "Everon",
                },
                text(" "),
                { "type": "image", "src": "/uploads/badge.png", "alt": "Local badge" },
            ],
        },
        {
            "type": "paragraph",
            "inlines": [
                text("First line"),
                { "type": "line_break" },
                text("second line\nthird line"),
            ],
        },
        {
            "type": "paragraph",
            "inlines": [
                text("See "),
                link("https://example.com/auto", true, "https://example.com/auto"),
                text(" or "),
                link("mailto:ops@example.com", true, "ops@example.com"),
                text(", then go "),
                link("/", false, "home"),
                text("."),
            ],
        },
    ]);
    assert_eq!(blocks, expected);
}

#[tokio::test]
async fn wiki_features_callout_kinds_cover_alerts_and_bracket_markers() {
    let suite = WikiSuite::new(SUITE).await;
    let body_md = markdown(&[
        "> [!NOTE]",
        "> Check the frequencies.",
        "",
        "> [!TIP]",
        "> Carry spare batteries.",
        "",
        "> [!IMPORTANT]",
        "> Report contact first.",
        "",
        "> [!WARNING]",
        "> Mines on the ridge.",
        "",
        "> [!CAUTION]",
        "> Friendly armour nearby.",
        "",
        "> [!CRITICAL] Never cross the river at night.",
        "",
        "> [!INFO]",
        "> Nets open at H-30.",
        "",
        "> [!TIP] Legacy tip on the marker line.",
        "",
        "> [!SECRET] Not a callout.",
    ]);
    let blocks = round_trip(&suite, "callouts", &body_md).await;
    let expected = json!([
        callout("note", "Check the frequencies."),
        callout("tip", "Carry spare batteries."),
        callout("important", "Report contact first."),
        callout("warning", "Mines on the ridge."),
        callout("caution", "Friendly armour nearby."),
        callout("critical", "Never cross the river at night."),
        callout("info", "Nets open at H-30."),
        callout("tip", "Legacy tip on the marker line."),
        { "type": "quote", "blocks": [paragraph("[!SECRET] Not a callout.")] },
    ]);
    assert_eq!(blocks, expected);
}

#[tokio::test]
async fn wiki_features_unsafe_link_urls_are_refused_with_their_lines() {
    let suite = WikiSuite::new(SUITE).await;
    let body_md = markdown(&[
        "Safe [start](/wiki/start) line.",
        "[a](javascript:alert(1))",
        "[b](data:text/html;base64,PHNjcmlwdD4=)",
        "[c](vbscript:msgbox(1))",
        "[d](//evil.example.com/path)",
        "[e](JAVASCRIPT:alert(1))",
        "[f](<java script:alert(1)>)",
        "[g](java&#09;script:alert(1))",
        "[h](&#x20;javascript:alert(1))",
        "[i](https://example.com/\u{a0}x)",
        "[j](<https://exa\u{1}mple.com>)",
        "[k](/\\evil.example.com)",
        "[l](file:///etc/passwd)",
        "[m](\u{feff}javascript:alert(1))",
        "[n](https:evil.example.com)",
    ]);
    let found = refused_findings(&suite, "links", &body_md).await;
    let expected: Vec<(i64, String)> = (2..=15)
        .map(|line| (line, "unsafe_link_url".to_owned()))
        .collect();
    assert_eq!(found, expected);
}

#[tokio::test]
async fn wiki_features_unsafe_images_raw_html_and_deep_nesting_are_refused() {
    let suite = WikiSuite::new(SUITE).await;
    let images = markdown(&[
        "![a](http://example.com/a.png)",
        "",
        "![b](data:image/png;base64,iVBORw0KGgo=)",
        "",
        "![c](//cdn.example.com/c.png)",
        "",
        "![d](javascript:alert(1))",
        "",
        "![e](/)",
    ]);
    let found = refused_findings(&suite, "images", &images).await;
    let expected: Vec<(i64, String)> = [1, 3, 5, 7, 9]
        .into_iter()
        .map(|line| (line, "unsafe_image_url".to_owned()))
        .collect();
    assert_eq!(found, expected);

    let html = markdown(&[
        "Intro paragraph.",
        "",
        "<div>block html</div>",
        "",
        "Text with <b>inline</b> html.",
    ]);
    let found = refused_findings(&suite, "html", &html).await;
    assert!(
        found.iter().all(|(_, code)| code == "raw_html"),
        "{found:?}"
    );
    let mut lines: Vec<i64> = found.iter().map(|(line, _)| *line).collect();
    lines.dedup();
    assert_eq!(lines, [3, 5]);

    let deep = markdown(&[
        "Deep below.",
        "",
        &format!("{} seventeen levels", ">".repeat(17)),
    ]);
    let found = refused_findings(&suite, "deep", &deep).await;
    assert_eq!(found, [(3, "nesting_too_deep".to_owned())]);

    // Sixteen levels is the limit, not past it.
    let limit = format!("{} sixteen levels", ">".repeat(16));
    let blocks = round_trip(&suite, "limit", &limit).await;
    assert_eq!(
        blocks,
        json!([nested_quotes(16, paragraph("sixteen levels"))])
    );

    // A body breaking every rule lists one finding per construct, in line order.
    let mixed = markdown(&[
        "[x](javascript:void(0))",
        "",
        "![y](http://example.com/y.png)",
        "",
        "<span>html</span>",
        "",
        &format!("{} deep", ">".repeat(17)),
    ]);
    let found = refused_findings(&suite, "mixed", &mixed).await;
    let mut distinct = found.clone();
    distinct.dedup();
    assert_eq!(
        distinct,
        [
            (1, "unsafe_link_url".to_owned()),
            (3, "unsafe_image_url".to_owned()),
            (5, "raw_html".to_owned()),
            (7, "nesting_too_deep".to_owned()),
        ]
    );
}

#[tokio::test]
async fn wiki_features_legacy_unsafe_markup_reads_back_degraded() {
    let suite = WikiSuite::new(SUITE).await;
    let slug = unique_slug("legacy");
    let body_md = markdown(&[
        "Intro [bad link](javascript:alert(1)) end.",
        "",
        "![bad image](data:image/png;base64,AAAA)",
        "",
        "<script>alert(1)</script>",
        "",
        "Inline <b>bold</b> html.",
        "",
        "[**styled** link](vbscript:x) and ![](http://example.com/no-alt.png) after.",
        "",
        &format!("{} seventeen levels", ">".repeat(17)),
    ]);
    let page_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO wiki_pages (slug, category, title, icon, body_md, nav_order, updated_by, \
         updated_at) VALUES ($1, 'Legacy', 'Legacy page', NULL, $2, 0, NULL, now()) RETURNING id",
    )
    .bind(&slug)
    .bind(&body_md)
    .fetch_one(suite.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO wiki_page_revisions (page_id, revision, slug, category, title, icon, \
         nav_order, body_md, author_id, created_at) \
         SELECT id, 1, slug, category, title, icon, nav_order, body_md, updated_by, updated_at \
         FROM wiki_pages WHERE id = $1",
    )
    .bind(page_id)
    .execute(suite.pool())
    .await
    .unwrap();

    let expected = json!([
        paragraph("Intro bad link end."),
        paragraph("bad image"),
        paragraph("<script>alert(1)</script>"),
        paragraph("Inline <b>bold</b> html."),
        {
            "type": "paragraph",
            "inlines": [
                { "type": "strong", "children": [text("styled")] },
                text(" link and  after."),
            ],
        },
        nested_quotes(16, paragraph("seventeen levels")),
    ]);
    let (status, article) = suite.read(&page_uri(&slug)).await;
    assert_eq!(status, StatusCode::OK, "{article}");
    assert_valid(WIKI_CONTRACT, Some("WikiArticle"), &article);
    assert_eq!(
        article["body_md"], body_md,
        "the stored source is served as stored"
    );
    assert_eq!(article["revision"], 1);
    assert!(article.get("updated_by").is_none(), "{article}");
    assert_eq!(article["blocks"], expected);
    let unsafe_nodes: Vec<&Value> = objects(&article["blocks"])
        .into_iter()
        .filter(|node| node["type"] == "link" || node["type"] == "image")
        .collect();
    assert!(unsafe_nodes.is_empty(), "{unsafe_nodes:?}");

    let (status, revision) = suite.read(&format!("{}/1", revisions_uri(&slug))).await;
    assert_eq!(status, StatusCode::OK, "{revision}");
    assert_valid(WIKI_CONTRACT, Some("WikiRevision"), &revision);
    assert_eq!(revision["blocks"], expected);
    assert!(revision.get("author_id").is_none(), "{revision}");
}

#[tokio::test]
async fn wiki_features_each_save_appends_a_revision_newest_first() {
    let suite = WikiSuite::new(SUITE).await;
    let slug = unique_slug("history");
    let created = suite.create(&slug, "One", "# One\n\nfirst").await;
    assert_eq!(created["revision"], 1);
    assert_eq!(created["updated_by"], suite.admin.id.as_str());
    assert_eq!(
        suite.update(&slug, 1, "Two", "# Two\n\nsecond").await["revision"],
        2
    );
    assert_eq!(
        suite.update(&slug, 2, "Three", "# Three\n\nthird").await["revision"],
        3
    );

    let (status, history) = suite.read(&revisions_uri(&slug)).await;
    assert_eq!(status, StatusCode::OK, "{history}");
    assert_valid(WIKI_CONTRACT, Some("WikiRevisionPage"), &history);
    assert_eq!(
        (&history["page"], &history["per_page"], &history["total"]),
        (&json!(1), &json!(20), &json!(3))
    );
    let items = history["items"].as_array().unwrap();
    let listed: Vec<(i64, &str, &str)> = items
        .iter()
        .map(|item| {
            (
                item["revision"].as_i64().unwrap(),
                item["title"].as_str().unwrap(),
                item["author_id"].as_str().unwrap(),
            )
        })
        .collect();
    let admin = suite.admin.id.as_str();
    assert_eq!(
        listed,
        [(3, "Three", admin), (2, "Two", admin), (1, "One", admin)]
    );

    let (status, second) = suite.read(&format!("{}/2", revisions_uri(&slug))).await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_valid(WIKI_CONTRACT, Some("WikiRevision"), &second);
    assert_eq!(second["slug"], slug.as_str());
    assert_eq!(second["revision"], 2);
    assert_eq!(second["title"], "Two");
    assert_eq!(second["category"], "SOP");
    assert_eq!(second["body_md"], "# Two\n\nsecond");
    assert_eq!(second["author_id"], admin);
    assert_eq!(
        second["blocks"],
        json!([heading(1, "two", json!([text("Two")])), paragraph("second")])
    );
    let (_, first) = suite.read(&format!("{}/1", revisions_uri(&slug))).await;
    assert_eq!(first["body_md"], "# One\n\nfirst");

    let (status, article) = suite.read(&page_uri(&slug)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (&article["revision"], &article["title"]),
        (&json!(3), &json!("Three"))
    );

    for missing in ["4", "0", "99999999999"] {
        let (status, body) = suite
            .read(&format!("{}/{missing}", revisions_uri(&slug)))
            .await;
        assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    }
    let (status, body) = suite.read(&format!("{}/abc", revisions_uri(&slug))).await;
    assert_refusal(status, &body, StatusCode::BAD_REQUEST, None);
    let unknown = unique_slug("unknown");
    let (status, body) = suite.read(&revisions_uri(&unknown)).await;
    assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    let (status, body) = suite.read(&format!("{}/1", revisions_uri(&unknown))).await;
    assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
}

#[tokio::test]
async fn wiki_features_revision_history_pages_and_clamps() {
    let suite = WikiSuite::new(SUITE).await;
    let slug = unique_slug("paging");
    suite.create(&slug, "Revision 1", "body 1").await;
    for base in 1..=4 {
        let title = format!("Revision {}", base + 1);
        suite
            .update(&slug, base, &title, &format!("body {}", base + 1))
            .await;
    }
    let page_of = |query: &str| format!("{}?{query}", revisions_uri(&slug));
    for (query, page, per_page, revisions) in [
        ("per_page=2", 1, 2, vec![5, 4]),
        ("page=2&per_page=2", 2, 2, vec![3, 2]),
        ("page=3&per_page=2", 3, 2, vec![1]),
        ("page=4&per_page=2", 4, 2, vec![]),
        ("per_page=500", 1, 100, vec![5, 4, 3, 2, 1]),
        ("per_page=100", 1, 100, vec![5, 4, 3, 2, 1]),
        ("page=1", 1, 20, vec![5, 4, 3, 2, 1]),
    ] {
        let (status, body) = suite.read(&page_of(query)).await;
        assert_eq!(status, StatusCode::OK, "{query}: {body}");
        assert_valid(WIKI_CONTRACT, Some("WikiRevisionPage"), &body);
        let served: Vec<i64> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["revision"].as_i64().unwrap())
            .collect();
        assert_eq!(served, revisions, "{query}");
        assert_eq!(
            (&body["page"], &body["per_page"], &body["total"]),
            (&json!(page), &json!(per_page), &json!(5)),
            "{query}"
        );
    }
    for query in [
        "page=0",
        "page=-1",
        "per_page=0",
        "per_page=-5",
        "per_page=abc",
        "page=1.5",
    ] {
        let (status, body) = suite.read(&page_of(query)).await;
        assert_refusal(status, &body, StatusCode::BAD_REQUEST, None);
    }
}

#[tokio::test]
async fn wiki_features_base_revision_decides_create_update_or_conflict() {
    let suite = WikiSuite::new(SUITE).await;
    let slug = unique_slug("base");

    // A numeric base names a revision of a page that does not exist.
    let (status, body) = suite.save(&slug, save_body("Base", "body", Some(1))).await;
    assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    assert!(suite.page_row(&slug).await.is_none());

    // null on a new page creates it.
    let created = suite.create(&slug, "Base", "first body").await;
    assert_eq!(created["revision"], 1);

    // null on an existing page is a conflict with its current revision.
    let (status, body) = suite.save(&slug, save_body("Base", "again", None)).await;
    assert_refusal(
        status,
        &body,
        StatusCode::CONFLICT,
        Some("wiki_revision_conflict"),
    );
    assert_eq!(body["details"]["current_revision"], 1);

    let saved = suite.update(&slug, 1, "Base", "second body").await;
    assert_eq!(saved["revision"], 2);
    let before = suite.stored(&slug).await;

    // A stale base and a base from the future both name the current revision.
    for base in [1, 3] {
        let (status, body) = suite
            .save(&slug, save_body("Overwrite", "lost edit", Some(base)))
            .await;
        assert_refusal(
            status,
            &body,
            StatusCode::CONFLICT,
            Some("wiki_revision_conflict"),
        );
        assert_eq!(body["details"]["current_revision"], 2, "base {base}");
    }
    for base in [0, -1] {
        let (status, body) = suite
            .save(&slug, save_body("Overwrite", "lost edit", Some(base)))
            .await;
        assert_refusal(status, &body, StatusCode::BAD_REQUEST, None);
    }
    assert_eq!(suite.stored(&slug).await, before, "refusals change nothing");
    let (_, article) = suite.read(&page_uri(&slug)).await;
    assert_eq!(
        (&article["revision"], &article["body_md"]),
        (&json!(2), &json!("second body"))
    );
}

#[tokio::test]
async fn wiki_features_request_shape_refusals_store_nothing() {
    let suite = WikiSuite::new(SUITE).await;
    let tag = unique_slug("shape");

    let too_long = format!("{tag}-{}", "a".repeat(64 - tag.len()));
    assert_eq!(too_long.len(), 65);
    for slug in [
        "Upper-Case".to_owned(),
        "under_score".to_owned(),
        "dot.slug".to_owned(),
        "%20padded".to_owned(),
        "caf%C3%A9".to_owned(),
        too_long,
    ] {
        let (status, body) = suite.save(&slug, save_body("Slug", "body", None)).await;
        assert_refusal(status, &body, StatusCode::BAD_REQUEST, None);
    }
    let longest = format!("{tag}-{}", "a".repeat(63 - tag.len()));
    assert_eq!(longest.len(), 64);
    suite.create(&longest, "Longest slug", "body").await;

    // The body limit counts bytes: 262 144 fit, one more does not, and a two-byte character
    // counts twice.
    let at_limit = unique_slug("at-limit");
    suite
        .create(&at_limit, "At limit", &"a".repeat(MAX_BODY_BYTES))
        .await;
    let over_limit = unique_slug("over-limit");
    for body_md in [
        "a".repeat(MAX_BODY_BYTES + 1),
        "é".repeat(MAX_BODY_BYTES / 2 + 1),
    ] {
        let (status, body) = suite
            .save(&over_limit, save_body("Over limit", &body_md, None))
            .await;
        assert_refusal(
            status,
            &body,
            StatusCode::BAD_REQUEST,
            Some("wiki_body_too_large"),
        );
    }

    // Unknown fields (the editor among them) and empty or mistyped required fields.
    let refused = unique_slug("refused");
    for (field, value) in [
        ("updated_by", json!("someone-else")),
        ("revision", json!(7)),
        ("author_id", json!("someone-else")),
        ("title", json!("")),
        ("category", json!("")),
        ("body_md", json!("")),
        ("nav_order", json!("3")),
    ] {
        let mut body = save_body("Bad field", "body", None);
        body[field] = value;
        let (status, answer) = suite.save(&refused, body.clone()).await;
        assert_refusal(status, &answer, StatusCode::BAD_REQUEST, None);
        assert_invalid(WIKI_CONTRACT, Some("WikiSaveRequest"), &body);
    }
    let (status, answer) = suite
        .send(
            Some(&suite.admin),
            "PUT",
            &page_uri(&refused),
            Some(save_body("Plain text", "body", None).to_string()),
            Some("text/plain"),
        )
        .await;
    assert_refusal(status, &answer, StatusCode::UNSUPPORTED_MEDIA_TYPE, None);

    for slug in [&over_limit, &refused] {
        let stored = suite.stored(slug).await;
        assert!(
            stored.page.is_none() && stored.revisions.is_empty() && stored.audits.is_empty(),
            "{slug}: {stored:?}"
        );
    }
}

#[tokio::test]
async fn wiki_features_list_serves_summaries_in_navigation_order() {
    let suite = WikiSuite::new(SUITE).await;
    let tag = unique_slug("list");
    let page = |suffix: &str| format!("{tag}-{suffix}");
    for (suffix, nav_order, title, icon) in [
        ("p1", 5, "Bravo", "book"),
        ("p2", 5, "Alpha", ""),
        ("p3", 1, "Zulu", "map"),
        ("p5", 7, "Same", ""),
        ("p4", 7, "Same", ""),
    ] {
        let mut body = save_body(title, "# Body\n\ntext", None);
        body["nav_order"] = json!(nav_order);
        body["icon"] = json!(icon);
        let (status, article) = suite.save(&page(suffix), body).await;
        assert_eq!(status, StatusCode::CREATED, "{article}");
    }
    let listed_order = |list: &Value| -> Vec<String> {
        list["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|row| row["slug"].as_str())
            .filter(|slug| slug.starts_with(&tag))
            .map(str::to_owned)
            .collect()
    };

    let (status, list) = suite.read("/api/v1/wiki").await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_valid(WIKI_CONTRACT, Some("WikiPageList"), &list);
    assert_eq!(
        listed_order(&list),
        ["p3", "p2", "p1", "p4", "p5"].map(page)
    );
    for row in list["data"].as_array().unwrap() {
        assert!(
            row.as_object()
                .unwrap()
                .keys()
                .all(|key| SUMMARY_KEYS.contains(&key.as_str())),
            "a summary carries no body: {row}"
        );
    }
    let summary = |slug: &str| -> Value {
        list["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["slug"] == slug)
            .unwrap()
            .clone()
    };
    assert_eq!(summary(&page("p1"))["icon"], "book");
    assert!(summary(&page("p2")).get("icon").is_none());

    // A summary carrying a body is not a summary; the contract sees the difference.
    let mut with_body = list.clone();
    with_body["data"][0]["body_md"] = json!("# leaked");
    assert_invalid(WIKI_CONTRACT, Some("WikiPageList"), &with_body);

    // A save moves the page to its new navigation position and revision.
    let mut body = save_body("Bravo", "# Body\n\nmoved", Some(1));
    body["nav_order"] = json!(0);
    let (status, moved) = suite.save(&page("p1"), body).await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    let (_, list) = suite.read("/api/v1/wiki").await;
    assert_valid(WIKI_CONTRACT, Some("WikiPageList"), &list);
    assert_eq!(
        listed_order(&list),
        ["p1", "p3", "p2", "p4", "p5"].map(page)
    );
    let row = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["slug"] == page("p1").as_str())
        .unwrap();
    assert_eq!(row["revision"], 2);

    let (status, article) = suite.read(&page_uri(&page("p1"))).await;
    assert_eq!(status, StatusCode::OK, "{article}");
    assert_valid(WIKI_CONTRACT, Some("WikiArticle"), &article);
    let mut unsafe_article = article.clone();
    unsafe_article["blocks"] = json!([{
        "type": "paragraph",
        "inlines": [link("javascript:alert(1)", false, "x")],
    }]);
    assert_invalid(WIKI_CONTRACT, Some("WikiArticle"), &unsafe_article);
}

#[tokio::test]
async fn wiki_features_members_read_but_only_administrators_save() {
    let suite = WikiSuite::new(SUITE).await;
    let slug = unique_slug("tiers");
    suite.create(&slug, "Tiers", "admin body").await;
    let before = suite.stored(&slug).await;

    for uri in [
        "/api/v1/wiki".to_owned(),
        page_uri(&slug),
        revisions_uri(&slug),
        format!("{}/1", revisions_uri(&slug)),
    ] {
        let (status, body) = suite.read(&uri).await;
        assert_eq!(status, StatusCode::OK, "member reads {uri}: {body}");
        let (status, body) = suite.call(None, "GET", &uri, None).await;
        assert_refusal(status, &body, StatusCode::UNAUTHORIZED, None);
    }

    let fresh = unique_slug("tiers-fresh");
    for (target, base) in [(&slug, Some(1)), (&fresh, None)] {
        let body = save_body("Member edit", "member body", base);
        let (status, answer) = suite.save_as(&suite.member, target, body.clone()).await;
        assert_refusal(status, &answer, StatusCode::FORBIDDEN, None);
        let (status, answer) = suite.call(None, "PUT", &page_uri(target), Some(body)).await;
        assert_refusal(status, &answer, StatusCode::UNAUTHORIZED, None);
    }
    assert_eq!(suite.stored(&slug).await, before);
    assert!(suite.page_row(&fresh).await.is_none());
    assert_eq!(suite.wiki_audit_count_by(&suite.member).await, 0);
}

#[tokio::test]
async fn wiki_features_storage_failure_leaves_page_revisions_and_audit_unchanged() {
    let suite = WikiSuite::new(SUITE).await;
    let pool = suite.pool();
    let slug = unique_slug("failure");
    suite.create(&slug, "Failure", "stored body").await;
    let before = suite.stored(&slug).await;

    for (table, operation, column) in [
        ("wiki_page_revisions", "INSERT", "slug"),
        ("audit_logs", "INSERT", "target_id"),
        ("wiki_pages", "UPDATE", "slug"),
    ] {
        let trigger = inject_failure(pool, table, operation, column, &slug).await;
        let (status, body) = suite
            .save(&slug, save_body("Failure", "lost body", Some(1)))
            .await;
        clear_failure(pool, &trigger, table).await;
        assert_refusal(status, &body, StatusCode::INTERNAL_SERVER_ERROR, None);
        assert_eq!(suite.stored(&slug).await, before, "{table} {operation}");
    }

    // A create whose revision insert fails leaves no page behind.
    let fresh = unique_slug("failure-fresh");
    let trigger = inject_failure(pool, "wiki_page_revisions", "INSERT", "slug", &fresh).await;
    let (status, body) = suite
        .save(&fresh, save_body("Fresh", "fresh body", None))
        .await;
    clear_failure(pool, &trigger, "wiki_page_revisions").await;
    assert_refusal(status, &body, StatusCode::INTERNAL_SERVER_ERROR, None);
    let stored = suite.stored(&fresh).await;
    assert!(
        stored.page.is_none() && stored.revisions.is_empty() && stored.audits.is_empty(),
        "{stored:?}"
    );

    // With the failure gone the same save lands as the next revision.
    let saved = suite.update(&slug, 1, "Failure", "saved body").await;
    assert_eq!(saved["revision"], 2);
    assert_eq!(suite.stored(&slug).await.revisions.len(), 2);
}

#[tokio::test]
async fn wiki_features_every_accepted_save_writes_one_audit_row_naming_the_actor() {
    let suite = WikiSuite::new(SUITE).await;
    let editor = suite.account("editor", "admin").await;
    let slug = unique_slug("audit");
    suite.create(&slug, "Audit", "first").await;
    let (status, saved) = suite
        .save_as(&editor, &slug, save_body("Audit", "second", Some(1)))
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["updated_by"], editor.id.as_str());

    // Refused saves write no audit row.
    for body in [
        save_body("Audit", "stale", Some(1)),
        save_body("Audit", "[x](javascript:alert(1))", Some(2)),
        save_body("Audit", &"a".repeat(MAX_BODY_BYTES + 1), Some(2)),
    ] {
        let (status, answer) = suite.save(&slug, body).await;
        assert!(status.is_client_error(), "{status}: {answer}");
    }
    let (status, _) = suite
        .save_as(&suite.member, &slug, save_body("Audit", "member", Some(2)))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let audits = suite.audit_rows(&slug).await;
    let actions: Vec<(&str, &str)> = audits
        .iter()
        .map(|row| {
            (
                row["action"].as_str().unwrap(),
                row["actor_id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        actions,
        [
            ("wiki_page.created", suite.admin.id.as_str()),
            ("wiki_page.updated", editor.id.as_str()),
        ],
        "{audits:?}"
    );
    let authors: Vec<Value> = suite
        .revision_rows(&slug)
        .await
        .iter()
        .map(|row| row["author_id"].clone())
        .collect();
    assert_eq!(
        authors,
        [json!(suite.admin.id.as_str()), json!(editor.id.as_str())]
    );
}

#[tokio::test]
async fn wiki_features_page_revision_equals_its_highest_revision_row() {
    let suite = WikiSuite::new(SUITE).await;
    for (label, saves) in [("invariant-one", 0), ("invariant-many", 3)] {
        let slug = unique_slug(label);
        suite.create(&slug, "Invariant 1", "body 1").await;
        for base in 1..=saves {
            suite
                .update(&slug, base, &format!("Invariant {}", base + 1), "body")
                .await;
        }
        // A refused save leaves the invariant standing.
        let (status, _) = suite
            .save(&slug, save_body("Stale", "stale", Some(saves + 2)))
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{slug}");

        let stored = suite.stored(&slug).await;
        let page = stored.page.expect("the page is stored");
        let numbers: Vec<i64> = stored
            .revisions
            .iter()
            .map(|row| row["revision"].as_i64().unwrap())
            .collect();
        let current = page["revision"].as_i64().unwrap();
        assert_eq!(numbers, (1..=current).collect::<Vec<_>>(), "{slug}");
        assert_eq!(current, saves + 1, "{slug}");
        let newest = stored.revisions.last().unwrap();
        for field in ["slug", "category", "title", "icon", "nav_order", "body_md"] {
            assert_eq!(newest[field], page[field], "{slug} {field}");
        }
        assert_eq!(newest["page_id"], page["id"]);
        assert_eq!(newest["author_id"], page["updated_by"]);
        assert_eq!(newest["created_at"], page["updated_at"]);
    }
}
