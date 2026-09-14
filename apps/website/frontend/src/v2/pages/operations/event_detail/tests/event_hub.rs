//! The guards on the operation dossier: the briefing empty rule and where it is bound, the meta
//! badges, the modpack chip's fetch choice, the tier checks behind the slotting affordances, and
//! the avatar sink in the member picker.

use super::*;
use crate::v2::core::test_support::fixtures::golden;

/// The event hub as the dev stack actually served it (the same capture the DTO golden
/// round-trips). **Its one mission carries no `briefing` key at all** — the backend omits the
/// field when the column is empty — so the single recorded real response is itself a live
/// instance of the defect, not a hypothetical one.
const EVENT_HUB_GOLDEN: &str = golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json");

fn golden_dossier() -> EventMissionDossier {
    let hub: EventHub = serde_json::from_str(EVENT_HUB_GOLDEN).expect("golden parses");
    hub.missions
        .into_iter()
        .next()
        .expect("golden has a mission")
}

/// The state that could not be reached before: an author clears the box, `PATCH` stores
/// `''`, the wire omits the key, and this arrives as `None`.
#[test]
fn a_cleared_briefing_renders_the_empty_state_and_never_the_invented_lore() {
    for cleared in [None, Some(""), Some("   \n\n  ")] {
        let out = briefing_text(cleared);
        assert_eq!(
            out, "No briefing provided.",
            "an authored-empty briefing must render the affordance, not prose ({cleared:?})"
        );
        // The specific claim: not one word of the removed lore, under any name.
        assert!(
            !out.contains("Hostile mechanized"),
            "the placeholder lore came back for {cleared:?}: {out}"
        );
        assert!(
            !out.contains("winter storm") && !out.contains("contested airspace"),
            "the placeholder lore came back for {cleared:?}: {out}"
        );
    }
}

/// The golden is the empty case — proof this is the live wire shape, not a synthetic one.
#[test]
fn the_recorded_wire_response_hits_the_empty_case() {
    let m = golden_dossier();
    assert!(
        m.briefing.is_none(),
        "fixture drifted: this test is only meaningful while the golden omits `briefing`"
    );
    assert_eq!(
        briefing_text(m.briefing.as_deref()),
        "No briefing provided."
    );
}

/// The operation-level briefing must use the same empty/trim rule on the hub hero that mission
/// dossiers already use. The golden omits the event key too.
#[test]
fn operation_level_briefing_uses_the_same_empty_rule() {
    let hub: EventHub = serde_json::from_str(EVENT_HUB_GOLDEN).expect("golden parses");
    assert!(
        hub.briefing.is_none(),
        "fixture drifted: event-level briefing must be absent on the recorded wire"
    );
    assert_eq!(
        briefing_text(hub.briefing.as_deref()),
        "No briefing provided."
    );
    assert_eq!(
        briefing_text(Some("Hold the LZ until QRF.")),
        "Hold the LZ until QRF."
    );
    assert_eq!(
        briefing_text(Some("\n\n  ")),
        "No briefing provided.",
        "whitespace-only operation briefing is not authored content"
    );
}

/// The shipped half of this page, scrubbed by
/// [`crate::v2::core::test_support::class_r_scrub`].
fn live() -> String {
    crate::v2::core::test_support::class_r_scrub::live_code(
        &crate::v2::core::test_support::pins::event_hub_source(),
    )
}

/// The hub hero must actually *read* the operation briefing. A pure call above is not enough if
/// the view never binds it; this pins the source seam.
///
/// # Why this is a source pin rather than a behavioural one
///
/// The half of this invariant that has a runtime signature — "blank means blank, authored
/// text is rendered verbatim" — is [`briefing_text`], and it is already pinned by value in
/// [`operation_level_briefing_uses_the_same_empty_rule`] and
/// [`an_authored_briefing_is_rendered_verbatim`].
/// What is left is exactly the part that has **no** signature: whether the hero *view* binds
/// the helper's output into the DOM. That lives inside a `view!` macro in an `impl IntoView`,
/// so there is nothing to call and nothing to observe without mounting a reactive runtime.
/// Source shape is the honest instrument here; the job is to make sure the source it reads is
/// source that ships.
///
/// The needles run on the **scrubbed** page (comments and literals blanked, `#[cfg(<false>)]`
/// items and constant-false blocks removed) rather than on a raw source read, and they are
/// **scoped to [`event_hub_view`]**, the one function that draws the hero, instead of matching
/// anywhere across the page.
#[test]
fn hub_hero_reads_event_briefing() {
    let prod = live();
    let hero = crate::v2::core::test_support::class_r_scrub::only_body(&prod, "fn event_hub_view(");
    assert!(
        hero.contains("briefing_text(ev.briefing.as_deref())"),
        "event_hub hero must render EventHub.briefing via briefing_text — \
         schedule detail routes through this same helper (T-407/T-494)"
    );
    // Computing it and dropping it on the floor is the failure this guards against, so the
    // binding must also reach the view. Two mentions: the `let`, and the interpolation.
    assert!(
        hero.matches("operation_briefing").count() >= 2,
        "the operation_briefing binding must be bound AND rendered on the hub hero — one \
         mention means it is computed and discarded, which is the T-407 defect exactly"
    );
}

/// Source ratchet for [`briefing_text`]: ban non-trim emptiness on briefing paths (filter *and*
/// match-arm). Same failure mode as the mission overview's dossier ratchet.
///
/// Cure 2 for the same reason as above: the positive halves are wiring seams inside views, and
/// the negative halves ban a *shape*, which is not a value anything can return.
#[test]
fn briefing_text_source_ratchet_requires_trim() {
    let prod = live();
    let dossier =
        crate::v2::core::test_support::class_r_scrub::only_body(&prod, "fn mission_dossier(");
    assert!(
        dossier.contains("briefing_text(m.briefing.as_deref())"),
        "mission dossier must route briefing through briefing_text"
    );
    // concat! so this test body does not match itself. Belt and braces: the scrubber already
    // cuts everything from `#[cfg(test)]` on, so the test module is not in `prod` at all — but
    // the fragments cost nothing and survive a future re-scoping.
    let old_filter = concat!(".filter(|b| !b.", "is_empty())");
    assert!(
        !prod.contains(old_filter),
        "is_empty-only briefing filter must not return — whitespace-only briefings \
         would blank the dossier Briefing section again"
    );
    let old_arm = concat!("Some(b) if !b.", "is_empty()");
    assert!(
        !prod.contains(old_arm),
        "match-arm !b.is_empty() without trim must not return on briefing paths"
    );
    let helper =
        crate::v2::core::test_support::class_r_scrub::only_body(&prod, "fn briefing_text(");
    let trim_arm = concat!("Some(b) if !b.trim().", "is_empty()");
    assert!(
        helper.contains(trim_arm),
        "briefing_text must keep the trim-aware match arm — on its own live path, not in a \
         comment describing it"
    );
    // The shape the schedule page's own detail renderer used before it routed through here.
    let old_then = concat!("(!briefing.", "is_empty())");
    assert!(
        !prod.contains(old_then),
        "non-trim briefing.then emptiness guard must not return on hub briefing paths"
    );
}

/// **Calibration for the two briefing pins above.**
///
/// The full attack battery against the needle the hero pin cannot do without, plus the two
/// shadow-copy shapes (a second `fn hub_shell` in a `mod` or an `impl` compiles fine and is
/// exactly how a whole-file grep gets fed a pristine decoy).
#[test]
fn the_briefing_pins_reject_every_dead_code_wrapper() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let needle = "briefing_text(ev.briefing.as_deref())";
    let attacks: [(&str, String); 12] = [
        (
            "if true == false",
            format!("if true == false {{ let b = {needle}; }}"),
        ),
        (
            "loop { break; … }",
            format!("loop {{ break; let b = {needle}; }}"),
        ),
        (
            "#[cfg(any())]",
            format!("#[cfg(any())] fn d() {{ let b = {needle}; }}"),
        ),
        (
            "while false",
            format!("while false {{ let b = {needle}; }}"),
        ),
        ("if !true", format!("if !true {{ let b = {needle}; }}")),
        ("if 1 > 2", format!("if 1 > 2 {{ let b = {needle}; }}")),
        (
            "if std::hint::black_box(false)",
            format!("if std::hint::black_box(false) {{ let b = {needle}; }}"),
        ),
        (
            "const C: bool = false; if C",
            format!("const C: bool = false;\nfn d() {{ if C {{ let b = {needle}; }} }}"),
        ),
        (
            "return; above",
            format!("fn d() {{ return; let b = {needle}; }}"),
        ),
        (
            "#[cfg(any())] mod shadow",
            format!("#[cfg(any())] mod shadow {{ fn d() {{ let b = {needle}; }} }}"),
        ),
        (
            "match guard",
            format!("match () {{ _ if false => {{ let b = {needle}; }} _ => {{}} }}"),
        ),
        ("comment", format!("// {needle}")),
    ];
    for (label, body) in attacks {
        let forged = format!("fn event_hub_view() {{\n    {body}\n}}\n#[cfg(test)]\n");
        assert!(
            !live_code(&forged).contains(needle),
            "{label}: the hero needle survived scrubbing — this pin would report a rendered \
             operation briefing over code the build never runs"
        );
    }
    for (label, forged) in [
        (
            "shadow event_hub_view in a live mod, no cfg",
            "fn event_hub_view() { good(); }\n\
             mod real { pub fn event_hub_view() { bad(); } }\n#[cfg(test)]\n",
        ),
        (
            "shadow event_hub_view in an impl",
            "fn event_hub_view() { good(); }\n\
             impl T { fn event_hub_view() { bad(); } }\n#[cfg(test)]\n",
        ),
    ] {
        let scrubbed = live_code(forged);
        let caught =
            std::panic::catch_unwind(|| only_body(&scrubbed, "fn event_hub_view(")).is_err();
        assert!(caught, "{label}: ambiguity must be RED, not a coin flip");
    }
    let live = format!("fn event_hub_view() {{\n    let b = {needle};\n}}\n#[cfg(test)]\n");
    assert!(live_code(&live).contains(needle));
}

/// The other half of the contract: a real briefing is still rendered verbatim, newlines and
/// all — the `<p>` is `whitespace-pre-line`, and the document core keeps paragraph breaks
/// intact specifically so they survive to a reader.
#[test]
fn an_authored_briefing_is_rendered_verbatim() {
    let authored = "Hold the ridge.\n\nSecond wave at H+20.";
    assert_eq!(briefing_text(Some(authored)), authored);
    // Leading/trailing space is not evidence of emptiness — only all-whitespace is.
    assert_eq!(briefing_text(Some(" Hold. ")), " Hold. ");
}

/// Every badge on the dossier header must come out of the dossier. This is the assertion the
/// fabricated `Maker`/`Duration` chips could not have survived: it pins the whole list, so
/// re-adding a hardcoded chip fails here rather than shipping.
#[test]
fn meta_badges_are_all_dossier_derived() {
    let m = golden_dossier();
    assert_eq!(meta_badges(&m), vec![("Terrain", "Everon".to_string())]);

    // Change the dossier, and every badge changes with it — nothing is pinned to a literal.
    let mut other = golden_dossier();
    other.terrain = "arland".into();
    assert_eq!(meta_badges(&other), vec![("Terrain", "Arland".to_string())]);
}

/// The guard for the sections this ticket deleted outright — the objective panels and the
/// per-faction vehicle list. They returned no value, so there is no seam to call; the honest
/// assertion is over the module source itself: nothing here may name the old constants or
/// carry their copy, whatever a future slice decides to call it.
///
/// Each needle is assembled with `concat!` so this test does not match itself: only the
/// fragments appear in the source, never the joined string. That constraint binds the prose
/// too — quoting the deleted copy in a comment anywhere in this module fails this test, which
/// is the point. If you are here because it went red on a comment you just wrote, paraphrase.
#[test]
fn no_fabricated_mission_intel_survives_in_this_module() {
    let src = crate::v2::core::test_support::pins::event_hub_source();
    let banned = [
        concat!("PLACEHOLDER_", "MAKER"),
        concat!("PLACEHOLDER_", "DURATION"),
        concat!("PLACEHOLDER_", "BLUFOR"),
        concat!("PLACEHOLDER_", "OPFOR"),
        concat!("PLACEHOLDER_", "LORE"),
        concat!("PLACEHOLDER_", "VEHICLES"),
        // The copy itself, in case it returns under a different name.
        concat!("Hostile mechanized", " elements"),
        concat!("Protect and secure", " the nuke"),
        concat!("Find and detonate", " the nuke"),
        concat!("Escort the VIP", " convoy"),
        concat!("BTR-70", " APC"),
        concat!("Mi-8", " Hip"),
        concat!("90", " MIN"),
    ];
    for needle in banned {
        assert!(
            !src.contains(needle),
            "fabricated mission intel is back in event_hub.rs: {needle:?}. \
             The Event Hub may only render what the dossier serves — see T-392."
        );
    }
}

/// Strip `//` and `/* */` so bans cannot false-red on a comment.
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

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The hub chip must prefer the operation's own `modpack_id` when it is set; an unconditional
/// `GET /modpacks/current` as the sole fetch is the defect.
#[test]
fn hub_chip_prefers_event_modpack_id() {
    assert_eq!(
        hub_modpack_fetch(Some("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee")),
        HubModpackFetch::ById("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee".into())
    );
    assert_eq!(hub_modpack_fetch(None), HubModpackFetch::Current);
    assert_eq!(hub_modpack_fetch(Some("")), HubModpackFetch::Current);
    assert_eq!(hub_modpack_fetch(Some("  \t")), HubModpackFetch::Current);

    let src = crate::v2::core::test_support::pins::event_hub_source();
    let production = src
        .split("mod tests {")
        .next()
        .expect("tests module marker");
    let code = collapse_ws(&strip_rust_comments(production));
    assert!(
        code.contains("hub_modpack_fetch(event_modpack_id.as_deref())"),
        "Hub chip must route the event's modpack_id through hub_modpack_fetch"
    );
    assert!(
        code.contains("HubModpackFetch::ById") && code.contains("HubModpackFetch::Current"),
        "both ById (event modpack) and Current (fallback) arms must exist in production"
    );
    // Ban the defect where the only modpack URL in the chip path is /modpacks/current. The
    // by-id arm uses the list envelope, which is not a substring of /modpacks/current.
    assert!(
        code.contains("api_get::<DataEnvelope<ModpackDto>>(store, \"/modpacks\")"),
        "event-bound path must hit GET /modpacks (list equivalent of /modpacks/:id)"
    );
    assert!(
        code.contains("\"/modpacks/current\""),
        "null/absent modpack_id must still fall back to /modpacks/current"
    );
    // The list fetch alone is false-green if the find ignores the id (`.next()`, or an
    // always-true predicate), so pin the live id-match predicate. format! keeps the pin
    // bindable without a single literal that a bulk edit of the production line could rewrite
    // in lockstep.
    let by_id_find = format!("into_iter().find(|{}| {}.modpack.id == id)", "mp", "mp");
    assert!(
        code.contains(&by_id_find),
        "ById path must select via into_iter().find(|mp| mp.modpack.id == id) \
         (list+first / always-true find is a fail)"
    );
}

/// Reserve, release and assign must not be driven by the browse-mode tier check, which answers
/// "yes" for a session with no user. Binds to the live `is_leader` and `is_admin` memo
/// assignments, and bans the unauthenticated form in production.
#[test]
fn orbat_affordances_use_authed_reactive_role() {
    let src = crate::v2::core::test_support::pins::event_hub_source();
    let production = src
        .split("mod tests {")
        .next()
        .expect("tests module marker");
    let code = collapse_ws(&strip_rust_comments(production));
    assert!(
        code.contains(
            "let is_leader = Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Leader))"
        ),
        "is_leader must be the Memo that re-reads AuthStore.user via has_min_role_authed \
         (dead Memo + browse-mode has_min_role is a fail)"
    );
    assert!(
        code.contains(
            "let is_admin = Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin))"
        ),
        "is_admin must be the Memo that re-reads AuthStore.user via has_min_role_authed \
         (dead Memo + browse-mode has_min_role is a fail)"
    );
    let masked = code.replace("has_min_role_authed", "HAS_MIN_ROLE_AUTHED");
    assert!(
        !masked.contains("has_min_role("),
        "production must not call browse-mode has_min_role( — use has_min_role_authed only"
    );
    let one_shot_leader = format!("store.has_min_role({}::Leader)", "Role");
    let one_shot_admin = format!("store.has_min_role({}::Admin)", "Role");
    assert!(
        !code.contains(&one_shot_leader) && !code.contains(&one_shot_admin),
        "one-shot store.has_min_role freezes pre-bootstrap None as leader/admin"
    );
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../shared/is_http_url_cases.rs"
));

#[test]
fn member_picker_avatar_src_only_keeps_http_urls() {
    let mut wrong = Vec::new();
    for (input, ok) in IS_HTTP_URL_CASES {
        let got = crate::v2::core::utils::safe_avatar_url(input);
        if *ok {
            if got != *input {
                wrong.push(format!("  dropped a legitimate avatar {input:?}"));
            }
        } else if got != DEFAULT_AVATAR {
            wrong.push(format!("  kept a non-http avatar {input:?} (got {got:?})"));
        }
    }
    assert!(
        wrong.is_empty(),
        "event_hub avatar sink wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
    assert_eq!(crate::v2::core::utils::safe_avatar_url(""), DEFAULT_AVATAR);
}
