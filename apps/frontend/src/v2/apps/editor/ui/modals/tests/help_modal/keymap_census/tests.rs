use super::*;

// ── THE COLLISION TEST ────────────────────────────────────────────────────────────────────

/// **The ticket.** Two window-level listeners must not both answer one keypress.
///
/// This is the case that has bitten twice already (Backspace: delete-the-selection vs
/// hide-the-interface; Space: flyTo vs cycle-the-widget). Nothing orders two separate `keydown`
/// closures — the browser runs both — so an overlap here is not a precedence question, it is two
/// actions firing on one key.
#[test]
fn no_two_listeners_claim_the_same_chord() {
    let all = all_bindings();
    let mut clashes: Vec<String> = Vec::new();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            if a.code != b.code
                || (a.file == b.file && a.listener == b.listener)
                || SHARED_CHANNELS.contains(&a.code.as_str())
                || !a.mods.overlaps(b.mods)
            {
                continue;
            }
            clashes.push(format!(
                "`{}` is claimed by {} {} AND by {} {}",
                a.code,
                a.site(),
                a.mods.describe(),
                b.site(),
                b.mods.describe()
            ));
        }
    }
    assert!(
        clashes.is_empty(),
        "T-703: KEYBINDING COLLISION — two window-level editor listeners answer the same \
         keypress, so BOTH fire and the operator gets two actions from one key:\n  {}\n\
         Fix it by re-keying one of them, by narrowing a modifier guard so the two predicates \
         no longer overlap, or — if the pile-up is deliberate and every claimant state-gates \
         itself — by adding the code to `SHARED_CHANNELS` with the reason written down.",
        clashes.join("\n  ")
    );
}

/// Within ONE listener, `match` order resolves an overlap deterministically — that is why
/// the undo/redo listener can put `"KeyZ" if ev.shift_key()` in front of a bare `"KeyZ"` and mean it.
/// What is never intentional is an arm whose every keypress was already taken by an arm above
/// it: that binding can never fire, and `rustc` will not warn because guards make arm
/// reachability undecidable for it.
#[test]
fn no_arm_is_shadowed_within_its_own_listener() {
    let mut dead: Vec<String> = Vec::new();
    for l in listeners() {
        for (i, later) in l.bindings.iter().enumerate() {
            for earlier in &l.bindings[..i] {
                if earlier.code == later.code && later.mods.covered_by(earlier.mods) {
                    dead.push(format!(
                        "{} arm #{} `{}` {} is entirely covered by arm #{} {}",
                        l.file,
                        later.order,
                        later.code,
                        later.mods.describe(),
                        earlier.order,
                        earlier.mods.describe()
                    ));
                }
            }
        }
    }
    assert!(
        dead.is_empty(),
        "T-703: DEAD BINDING — an arm is fully shadowed by an earlier arm on the same key in \
         the same listener, so it can never run:\n  {}",
        dead.join("\n  ")
    );
}

/// The exemption list cannot rot into a dumping ground: a code sits in [`SHARED_CHANNELS`] only
/// while it really is claimed by two or more listeners. Exempt a key that is singly bound and
/// this is red — which means the list can only ever describe a pile-up that exists.
#[test]
fn every_shared_channel_is_really_shared() {
    let all = all_bindings();
    for code in SHARED_CHANNELS {
        let sites: BTreeSet<(&str, usize)> = all
            .iter()
            .filter(|b| b.code == code)
            .map(|b| (b.file, b.listener))
            .collect();
        assert!(
            sites.len() >= 2,
            "T-703: `{code}` is exempted from the collision rule but only {} listener(s) claim \
             it. An exemption for a key that is not actually shared is a hole waiting for the \
             next real collision to fall into — delete the entry.",
            sites.len()
        );
    }
}

/// T-776 — the source region that answers ONE shared-channel claim, not the whole listener.
/// An unrelated `get_untracked()` in another arm (cursor, snap, chrome, …) must not satisfy
/// the live-state pin: that was the hollow shape NIT-1 named.
fn key_equals_claim_site(src: &str, code: &str) -> Option<String> {
    let needle = format!("ev.key() == \"{code}\"");
    let at = src.find(&needle)?;
    let before = &src[..at];
    let if_at = before.rfind("if ").unwrap_or(0);
    let after_if = &src[if_at..];
    let rel_open = after_if.find('{')?;
    let open = if_at + rel_open;
    let end = balanced(src, open)?;
    Some(src[if_at..=end].to_string())
}

/// Match-arm claim: `(prelude before the match, arm body including the literal head)`.
fn match_arm_claim(src: &str, code: &str) -> Option<(String, String)> {
    let lit = format!("\"{code}\"");
    let mut from = 0usize;
    while let Some(i) = src[from..].find(&lit) {
        let at = from + i;
        let trimmed = src[at + lit.len()..].trim_start();
        if trimmed.starts_with("=>") || trimmed.starts_with("if ") || trimmed.starts_with('|') {
            let arrow_rel = src[at..].find("=>")?;
            let after_arrow = &src[at + arrow_rel + 2..];
            let body_start_rel = after_arrow.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            let abs = at + arrow_rel + 2 + body_start_rel;
            let end = if src.as_bytes().get(abs) == Some(&b'{') {
                balanced(src, abs)?
            } else {
                abs + after_arrow[body_start_rel..]
                    .find('\n')
                    .unwrap_or(after_arrow[body_start_rel..].len())
            };
            let match_at = src[..at].rfind("match ev.").unwrap_or(0);
            return Some((src[..match_at].to_string(), src[at..=end].to_string()));
        }
        from = at + lit.len();
    }
    None
}

/// Open/closed latch in the listener prelude: a `get_untracked()` whose nearby window
/// actually `return`s. An unrelated untracked read (cursor position, …) does not count.
fn early_return_live_gate(prelude: &str) -> bool {
    let mut from = 0usize;
    while let Some(i) = prelude[from..].find("get_untracked()") {
        let at = from + i;
        let window = &prelude[at..prelude.len().min(at + 80)];
        if window.contains("return") {
            return true;
        }
        from = at + 1;
    }
    false
}

/// True when `get_untracked()` / `.escape()` appears in an `if` PREDICATE (the text between
/// `if` / `if let` and its opening `{`), not merely somewhere in the claim body.
///
/// Wave-134 F1: a decoy `let _ = open.get_untracked()` inside an ungated Escape body used to
/// green the pin while the act stayed unconditional — the exact shared-channel collision the
/// exemption narrates.
fn live_state_in_if_predicate(site: &str) -> bool {
    let mut from = 0usize;
    while let Some(rel) = site[from..].find("if ") {
        let if_at = from + rel;
        if if_at > 0 {
            let prev = site.as_bytes()[if_at - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                from = if_at + 2;
                continue;
            }
        }
        let after = if_at + 3;
        let Some(open_rel) = site[after..].find('{') else {
            from = after;
            continue;
        };
        let cond = &site[after..after + open_rel];
        if cond.contains("get_untracked()") || cond.contains(".escape()") {
            return true;
        }
        from = after + open_rel + 1;
    }
    false
}

/// Does this listener's claim path for `code` read live state before acting?
fn shared_channel_claim_gated(src: &str, code: &str) -> bool {
    if let Some(site) = key_equals_claim_site(src, code) {
        // Predicate that guards the act — not a body-side decoy read (wave-134 F1).
        return live_state_in_if_predicate(&site);
    }
    if let Some((prelude, arm)) = match_arm_claim(src, code) {
        if live_state_in_if_predicate(&arm) {
            return true;
        }
        // `.escape()` as the act's deciding call (editor measure-tool OR-chain).
        if arm.contains(".escape()") {
            return true;
        }
        return early_return_live_gate(&prelude);
    }
    panic!(
        "T-776: could not locate the `{code}` claim site in a shared-channel claimant — the              census saw the binding but the live-state pin cannot find the path that answers it"
    );
}

/// What makes the Escape pile-up sound rather than a collision: every claimant reads its own
/// live state before it acts, so at most one surface is ever dismissed. Pin that, or the
/// exemption is a wish rather than an argument.
#[test]
fn every_shared_channel_claimant_reads_live_state() {
    // T-776 — per CLAIM, not per listener. A substring over `l.src` was satisfied by any
    // unrelated `get_untracked()` in the same closure (cursor, snap, chrome toggles, …) while
    // the Escape path itself stayed unconditional. Wave-134 F1: a decoy untracked read *inside*
    // an ungated Escape body is the same hollow — the latch must sit in the predicate that
    // guards the act. The exemption this pin guards is what keeps a many-claimant Escape
    // channel legal — the last place a weak guard should sit.
    for l in listeners() {
        for b in &l.bindings {
            if !SHARED_CHANNELS.contains(&b.code.as_str()) {
                continue;
            }
            assert!(
                shared_channel_claim_gated(&l.src, &b.code),
                "T-703/T-776: {}#{} claims shared channel `{}` but its claim path never gates the act on                      live state (a `get_untracked()` / `.escape()` in the `if` predicate, `.escape()` as the act, or an early-return latch) — it will                      fire alongside every other claimant on one keypress, which is a collision and                      not a shared channel",
                l.file,
                l.index,
                b.code
            );
        }
    }
    // The editor keydown's own Escape arm is the one claimant with no open/closed latch: it is
    // gated on the measure tools having something to dismiss. `.escape()` returns false when a
    // tool is empty and the arm returns the OR, so an Escape with nothing placed falls through
    // untouched instead of swallowing the key from the dialogs above.
    let arms = keydown_arms(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/input/window_keydown.rs"
    )));
    let esc = arms
        .find(&format!("\"{}\" if !modk", "Escape"))
        .expect("the editor keydown's Escape arm");
    // Just this arm: from its head to the head of the next one. Every arm in that match is
    // guarded, so `" if ` is where the next one starts.
    let rest = &arms[esc..];
    let body = &rest[..rest[16..].find("\" if ").map_or(rest.len(), |i| i + 16)];
    assert!(
        body.contains(".escape()") && body.contains("||"),
        "T-703: the editor keydown's Escape arm must ACT only when a measurement was really \
         dismissed (the OR of the tools' `.escape()` results) — an arm that returns a bare \
         `true` would swallow Escape from every dialog that shares the channel"
    );
}

/// The census must see every listener there is. This is T-738's finding as a standing pin: the
/// old extractor read two listeners out of the thirteen the editor runs and reported total
/// coverage, and nothing was red. A new window-level keydown anywhere in the editor surface now
/// fails here until someone bumps the count — which is the moment to check whether it collides.
///
/// **This pin only sees what [`editor_surface`] hands it, and T-774 is the proof.** The tripwire
/// was green while `faction_manager` and `orbat_manager` each ran an uncensused window-level
/// keydown, because neither file was in the list — a growth tripwire over an incomplete input
/// reports on its own input, not on the editor. Adding a MODULE to the surface is therefore the
/// one move this pin cannot prompt you to make; the scope note above [`editor_surface`] is what
/// bounds it, and it is exhaustive on purpose.
#[test]
fn every_editor_surface_listener_is_censused() {
    let mut total = 0usize;
    for (file, raw, expected) in editor_surface() {
        let found = listener_bodies(raw).len();
        assert_eq!(
            found, expected,
            "T-703: `{file}` installs {found} window-level keydown listener(s), the census \
             expects {expected}. If a listener was ADDED, bump the count here — and read \
             `no_two_listeners_claim_the_same_chord` before you do, because a new listener is \
             exactly how the Backspace and Space collisions got in. If one was REMOVED, drop \
             the count."
        );
        total += found;
    }
    // T-946.86 — 15: include the private-gesture Escape listener, measured by the census.
    assert_eq!(
        total, 15,
        "T-703: the editor surface should carry 15 window-level keydown listeners, found \
         {total}"
    );
    // A listener that yields no binding means the slicer lost the closure body (an unbalanced
    // brace in a literal, a reworded registration) — that is a silently EMPTY census, the
    // exact shape of a pin that passes forever while the UI rots.
    for l in listeners() {
        assert!(
            !l.bindings.is_empty(),
            "T-703: {}#{} was discovered but yielded no binding — the extractor lost its body",
            l.file,
            l.index
        );
    }
}

/// Every declared precondition must still match live source. A needle that no longer appears is
/// a stale entry, and a stale entry means the census is narrowing arms on the strength of a
/// guard that has been deleted.
#[test]
fn every_declared_precondition_is_still_in_the_source() {
    let bodies: Vec<String> = listeners().into_iter().map(|l| l.src).collect();
    for (needle, _) in PRECONDITIONS {
        assert!(
            bodies.iter().any(|b| b.contains(needle)),
            "T-703: the listener precondition `{needle}` matches no listener any more. Either \
             the guard was reworded (update the needle — until you do, every arm under it is \
             censused as modifier-free) or it is gone (delete the entry)."
        );
    }
}

/// The T-669 lesson, stated as arithmetic: a census that compares bare CODES cannot see the
/// collisions this ticket is about. `Ctrl+V` and `Ctrl+Shift+V` share a code and do not
/// collide; `Ctrl+V` and "V with any modifiers" share a code and do. Any rule phrased on codes
/// alone gets both of those wrong, in opposite directions.
#[test]
fn overlap_is_modifier_aware_not_code_aware() {
    let ctrl_v = Mods {
        modk: Some(true),
        alt: Some(false),
        shift: Some(false),
    };
    let ctrl_shift_v = Mods {
        modk: Some(true),
        alt: Some(false),
        shift: Some(true),
    };
    assert!(
        !ctrl_v.overlaps(ctrl_shift_v),
        "Ctrl+V and Ctrl+Shift+V partition the key — a rule that called this a collision would \
         have blocked T-669"
    );
    assert!(
        ctrl_v.overlaps(Mods::ANY),
        "an unguarded `ev.key() == \"…\"` listener claims the key under EVERY modifier, so it \
         collides with a Ctrl-guarded arm on the same code"
    );
    assert!(
        ctrl_v.overlaps(Mods {
            modk: Some(true),
            alt: None,
            shift: None
        }),
        "a Ctrl-anything arm swallows Ctrl+V — this is the shape T-669's Ctrl+Shift+V pin \
         could not see, because both are `KeyV`"
    );
    // Coverage, the other relation, is strictly stronger than overlap.
    assert!(ctrl_v.covered_by(Mods::ANY));
    assert!(!Mods::ANY.covered_by(ctrl_v));
    assert!(!ctrl_v.covered_by(ctrl_shift_v));
}

/// T-738 banked this: consume the extractor, do not write a fifth copy. By wave 113 it existed
/// four times (`mission_editor` ×3, `eden_help` ×1) in two variants, and each copy was one more
/// place a census could drift from the code it censuses. Exactly one definition, forever.
#[test]
fn there_is_exactly_one_extractor() {
    // Assembled so the needle never appears verbatim in this test's own source.
    let needle = format!("fn keydown{}(", "_arms");
    // T-776 — scan the WHOLE crate, not `editor_surface()` + self. A fifth copy in
    // `eden_dock_left` / `editor_ops` / `ui` sat outside the old six-file list and would have
    // passed; this pin enforces T-738's banked "one extractor" instruction, so its input is
    // the crate.
    let src_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut copies: Vec<String> = Vec::new();
    fn walk(dir: &std::path::Path, needle: &str, copies: &mut Vec<String>) {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("T-776: cannot read {}: {e}", dir.display()));
        for ent in entries {
            let ent = ent.expect("read_dir entry");
            let path = ent.path();
            if path.is_dir() {
                walk(&path, needle, copies);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("T-776: cannot read {}: {e}", path.display()));
            let n = raw.matches(needle).count();
            if n > 0 {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                copies.push(format!("{name} ×{n}"));
            }
        }
    }
    walk(&src_root, &needle, &mut copies);
    copies.sort();
    assert_eq!(
        copies,
        vec!["mod.rs ×1".to_string()],
        "T-738/T-776: the keydown-arm extractor must be defined ONCE, in `keymap_census`.              Found: {copies:?}. Consume it (`use crate::v2::apps::editor::ui::modals::help_modal::keymap_census::…`) and widen              it there — a second copy is a second answer to the same question."
    );
}

/// T-740 — the module's prose census numbers must be DERIVED from this census, not retyped.
/// That sentence has gone stale twice; a number nobody can check is a comment pretending to be
/// a measurement.
///
/// T-774 widened it in both directions. The **input** grew (`faction_manager` and
/// `orbat_manager` were missing from `editor_surface`, so every count below was true of what the
/// census SCANNED and false of what the editor RAN), and the **prose under pin** grew: this used
/// to read the `//!` header alone, which left `keymap_census`'s own doc block free to state
/// numbers — "nine listeners claim Escape" — that no pin ever checked. Both regions are read
/// now.
#[test]
fn the_prose_census_numbers_are_derived() {
    let raw = include_str!("../../../help_modal.rs");
    // Doc prose is hard-wrapped at 100 columns, so a claim phrase routinely straddles a line
    // break and a naive `contains` then fails on prose that is perfectly correct. Flatten the
    // comment markers and collapse every whitespace run to one space, so what is matched is the
    // SENTENCE and the pin is indifferent to where the wrap lands.
    let flatten = |s: &str| {
        s.split_whitespace()
            .filter(|w| *w != "//!" && *w != "///")
            .collect::<Vec<_>>()
            .join(" ")
    };
    // Only the `//!` header, so this pin can never read its own assertion strings back as
    // evidence (the hollow-pin failure a bare whole-file `include_str!` walks into).
    let header = flatten(
        &raw.lines()
            .take_while(|l| l.starts_with("//!") || l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
    );
    // `keymap_census`'s own doc block: everything before the module opens, which is strictly
    // before this test's source and so is subject to the same no-self-reading guarantee.
    let opens = "pub(crate) mod keymap_census;";
    let census_doc = flatten(
        &raw[..raw.find(opens).expect("the census module")]
            .lines()
            .filter(|l| l.trim_start().starts_with("///"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let ls = listeners();
    let files: BTreeSet<&str> = ls.iter().map(|l| l.file).collect();
    let escape_claimants = ls
        .iter()
        .filter(|l| l.bindings.iter().any(|b| b.code == "Escape"))
        .count();
    let claims = [
        (
            all_bound_codes().len(),
            "distinct `KeyboardEvent` codes",
            &header,
        ),
        (ls.len(), "window-level keydown listeners", &header),
        (files.len(), "editor-surface modules", &header),
        // T-774 — the TOTAL, not just the distinct codes. The T-703 slice reported "39
        // bindings" in prose nobody could check and the wave-119 verifier's own parser said 32;
        // two unchecked numbers about the same census is exactly the defect this module exists
        // to kill, so the total now lives in the header and is derived like the other three.
        (all_bindings().len(), "bindings in total", &header),
        // The Escape pile-up is the census's own headline number and it was typed, not derived.
        (escape_claimants, "listeners claim Escape", &census_doc),
    ];
    for (n, what, prose) in claims {
        let phrase = format!("{} {what}", spell(n));
        assert!(
            prose.contains(&phrase),
            "T-740/T-774: the module prose must say `{phrase}` — the census counts {n}. Do not \
             retype the old number; this is the third time."
        );
    }
}

/// A per-listener breakdown, so a reader can see what the census actually holds rather than
/// trusting that it holds something. Also a floor: a census that collapsed to a handful of
/// bindings would make every coverage pin above pass vacuously.
#[test]
fn the_census_reports_what_it_found() {
    let ls = listeners();
    let mut by_file: BTreeMap<&str, usize> = BTreeMap::new();
    for l in &ls {
        *by_file.entry(l.file).or_default() += l.bindings.len();
    }
    let total: usize = by_file.values().sum();
    assert!(
        total >= 25,
        "T-703: the editor binds well over two dozen chords; a census finding {total} \
         ({by_file:?}) has broken, and a broken census makes every pin built on it vacuous"
    );
    // Both accessors must be represented, or the widening T-738 asked for has been undone.
    assert!(
        ls.iter()
            .flat_map(|l| &l.bindings)
            .any(|b| b.via == "ev.code()"),
        "the census must read the `match ev.code().as_str()` keydowns"
    );
    assert!(
        ls.iter()
            .flat_map(|l| &l.bindings)
            .any(|b| b.via == "ev.key()"),
        "T-738: the census must read the `ev.key()` listeners too — that widening IS this ticket"
    );
}
