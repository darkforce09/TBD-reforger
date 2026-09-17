use super::serialization_and_export_tests::{attachment_feed, names, picks};
use super::*;

mod t699 {
    use super::*;

    fn buf(source: &str, json: Option<&str>) -> BufferedLoadout {
        BufferedLoadout {
            source_id: source.to_string(),
            loadout_json: json.map(str::to_string),
        }
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// A `SlotLoadoutV2` document as the Arsenal persists one, distinguishable by its primary.
    fn kit_doc(primary: &str) -> String {
        picks_to_loadout(&picks(&[("primary", primary)]), &names(), None)
            .expect("a picked primary is not an empty loadout")
    }

    /// **What "random" means here**, asserted rather than described.
    ///
    /// Four properties, and every one of them is load-bearing: uniform (no source is
    /// systematically favoured), independent per entity (N entities get N draws, not one draw
    /// N times), reproducible from `(seed, ordinal, len)` (so a bug report replays and this very
    /// test can exist), and degenerate at `len == 1` (a single-source Copy→Apply must be plain
    /// deterministic behaviour).
    #[test]
    fn the_draw_is_uniform_independent_and_reproducible() {
        const N: u64 = 30_000;
        let len = 3usize;
        let mut hits = [0usize; 3];
        for ordinal in 0..N {
            hits[buffer_draw(0xA5A5_A5A5, ordinal, len)] += 1;
        }
        // Uniform: a fair 3-way split of 30k is 10k each; ±5% is far outside anything a
        // correct mix produces by chance and far inside anything a biased one does.
        for (i, h) in hits.iter().enumerate() {
            assert!(
                (9_500..=10_500).contains(h),
                "index {i} came up {h} times in {N} draws — not a uniform draw: {hits:?}"
            );
        }
        // Independent per entity: consecutive ordinals must not walk the buffer in lockstep.
        let walk: Vec<usize> = (0..12).map(|o| buffer_draw(7, o, len)).collect();
        let cyclic: Vec<usize> = (0..12).map(|o| (o as usize) % len).collect();
        assert_ne!(walk, cyclic, "the draw is a counter, not a die: {walk:?}");

        // Reproducible: same seed → same assignment, every time.
        for ordinal in 0..50 {
            assert_eq!(
                buffer_draw(1234, ordinal, len),
                buffer_draw(1234, ordinal, len)
            );
        }
        // …and a different seed genuinely re-rolls.
        let a: Vec<usize> = (0..40).map(|o| buffer_draw(1, o, len)).collect();
        let b: Vec<usize> = (0..40).map(|o| buffer_draw(2, o, len)).collect();
        assert_ne!(a, b, "advancing the seed must change the assignment");

        // Degenerate at one: randomness must not be able to surprise the single-source case.
        for ordinal in 0..100 {
            assert_eq!(buffer_draw(ordinal * 7919, ordinal, 1), 0);
        }
        // …and an empty buffer never indexes anything (plan_apply refuses to call it, but the
        // function must not be a landmine for the next caller either).
        assert_eq!(buffer_draw(9, 9, 0), 0);
    }

    /// Apply writes ONE buffered loadout per entity, drawn from the buffer, and every write is
    /// a full document — never a merge of two sources, which is the shape that would quietly
    /// invent a soldier nobody authored.
    #[test]
    fn apply_gives_every_entity_exactly_one_buffered_loadout() {
        let sources = [
            buf("s1", Some(&kit_doc("res://rifle_m16"))),
            buf("s2", Some(&kit_doc("res://rifle_ak"))),
            buf("s3", None), // a bare soldier is a legitimate thing to copy and to apply
        ];
        let targets = ids(&["t1", "t2", "t3", "t4", "t5", "t6"]);
        let writes = plan_apply(&targets, &sources, 42, &[], &CompatFeed::default())
            .expect("a clean buffer applies");

        assert_eq!(writes.len(), targets.len(), "one write per selected entity");
        for (w, t) in writes.iter().zip(&targets) {
            assert_eq!(
                &w.target_id, t,
                "writes stay index-aligned with the selection"
            );
            let src = sources
                .iter()
                .find(|s| Some(&s.source_id) == w.source_id.as_ref())
                .expect("every write names a buffered source");
            assert_eq!(
                w.loadout_json, src.loadout_json,
                "a write is one source's document verbatim, never a blend"
            );
        }
        // Over six entities and three sources the draw must actually vary — a plan that gave
        // everyone source #1 would satisfy every assertion above and be the bug.
        let drawn: HashSet<Option<String>> = writes.iter().map(|w| w.source_id.clone()).collect();
        assert!(drawn.len() > 1, "the draw did not vary: {drawn:?}");

        // THE ANTI-INHERITANCE PROPERTY (T-687 was cancelled): the plan carries BYTES, so it is
        // complete without the sources. Drop them and every write still describes its loadout.
        drop(sources);
        assert!(writes.iter().all(|w| w.target_id.starts_with('t')));
    }

    /// One buffered loadout ⇒ everybody gets it, with no draw involved.
    #[test]
    fn a_single_buffered_loadout_needs_no_die() {
        let only = kit_doc("res://rifle_m16");
        let writes = plan_apply(
            &ids(&["a", "b", "c"]),
            &[buf("s", Some(&only))],
            0xDEAD_BEEF,
            &[],
            &CompatFeed::default(),
        )
        .expect("a clean buffer applies");
        assert_eq!(writes.len(), 3);
        assert!(writes
            .iter()
            .all(|w| w.loadout_json.as_deref() == Some(only.as_str())));
    }

    /// Nothing selected, or nothing buffered, is **not** a refusal — there is simply no work.
    #[test]
    fn an_empty_selection_or_buffer_plans_nothing_and_refuses_nothing() {
        let full = [buf("s", Some(&kit_doc("res://rifle_m16")))];
        assert!(plan_apply(&[], &full, 1, &[], &CompatFeed::default())
            .expect("no targets is not a fault")
            .is_empty());
        assert!(
            plan_apply(&ids(&["t"]), &[], 1, &[], &CompatFeed::default())
                .expect("no buffer is not a fault")
                .is_empty()
        );
        assert!(plan_remove(&[]).is_empty());
    }

    /// **The gate is the one T-686 built, not a second one that merely resembles it.** The same
    /// bytes refused on the way IN through `try_import` are refused on the way ACROSS through
    /// `plan_apply`, reason for reason — because both call `loadout_rule_refusals`.
    ///
    /// RED (a second gate): drop the `cargo_capacity_errors` line from `loadout_rule_refusals`
    /// and both sides go quiet together, which is what makes this an equivalence and not a
    /// transcription.
    #[test]
    fn the_apply_gate_is_the_import_gate() {
        let items = capacity_catalog();
        // 4 × 60 cm³ of magazine into a 200 cm³ chest rig — a schema-valid document describing
        // kit the game would silently drop.
        let raw = picks_to_export(
            &picks(&[("vest", "res://chest_rig")]),
            &[row("vest", "res://mag_stanag", 4)],
            "mp",
        );
        let on_the_way_in = try_import(&raw, &items, &CompatFeed::default())
            .expect_err("over-capacity cargo must not be importable");
        let across = plan_apply(
            &ids(&["t1"]),
            &[buf("s1", Some(&raw))],
            7,
            &items,
            &CompatFeed::default(),
        )
        .expect_err("…nor applicable");

        assert_eq!(
            on_the_way_in.len(),
            across.len(),
            "the two doors must find the same faults: {on_the_way_in:?} vs {across:?}"
        );
        for (i, a) in on_the_way_in.iter().zip(&across) {
            assert_eq!(i.key, a.key, "same row blamed");
            assert!(
                a.message.ends_with(&i.message),
                "same reason, differing only by which buffered source it names: {a:?}"
            );
            assert!(
                a.message.starts_with("Buffered loadout from s1"),
                "a refusal must say WHICH copied loadout is unusable: {a:?}"
            );
        }
    }

    /// **The verdict must not depend on the die.** A buffer holding one unusable loadout is
    /// refused for every seed — including the seeds on which the bad entry would never have been
    /// drawn. Validating only what came up would leave a broken loadout lurking in the buffer to
    /// ambush the author on some later press.
    ///
    /// RED: move the gate below the draw loop and validate `src` instead of the buffer → seeds
    /// on which the good source wins go green and this test names them.
    #[test]
    fn a_bad_entry_refuses_the_apply_whatever_the_die_says() {
        let items = capacity_catalog();
        let good = picks_to_export(
            &picks(&[("vest", "res://chest_rig")]),
            &[row("vest", "res://mag_stanag", 1)],
            "mp",
        );
        let bad = picks_to_export(
            &picks(&[("vest", "res://chest_rig")]),
            &[row("vest", "res://mag_stanag", 4)],
            "mp",
        );
        let buffer = [buf("good", Some(&good)), buf("bad", Some(&bad))];
        for seed in 0..64u64 {
            let out = plan_apply(&ids(&["t"]), &buffer, seed, &items, &CompatFeed::default());
            let refusals = out.expect_err(&format!("seed {seed} must refuse the whole apply"));
            assert!(
                refusals.iter().all(|r| r.message.contains("from bad")),
                "seed {seed}: {refusals:?}"
            );
        }
        // The same buffer without the bad entry applies cleanly — so the refusal is about the
        // loadout, not about the shape of the test.
        assert!(plan_apply(
            &ids(&["t"]),
            &buffer[..1],
            0,
            &items,
            &CompatFeed::default()
        )
        .is_ok());
    }

    /// T-504, matched deliberately to T-686's choice: cargo authored against a container the
    /// loadout wears nothing in is a **warning on the entity**, never a refusal at the door.
    /// Apply has a second reason on top of T-686's — the fault is a property of the target's
    /// character rather than of the bytes, so wiring it in would make the gate's answer depend
    /// on which entity the die picked.
    #[test]
    fn undeliverable_cargo_warns_but_never_blocks_an_apply() {
        let items = capacity_catalog();
        let idx = index_by_name(&items);
        // Three mags into a vest with no vest picked: 180 cm³, so capacity has nothing to say.
        let raw = picks_to_export(&picks(&[]), &[row("vest", "res://mag_stanag", 3)], "mp");
        let doc_picks = loadout_to_picks(Some(&raw));
        let (cargo, _) = rules::cargo_from_loadout(Some(&raw));

        let faults = loadout_faults(
            &doc_picks,
            &cargo,
            &attachment_feed(&[]),
            &idx,
            Some(&kit(&[])),
        );
        assert_eq!(faults.len(), 1, "the badge must still count it: {faults:?}");
        assert!(faults[0].message.contains("nowhere known to go"));

        assert!(
            buffer_refusals(&[buf("s", Some(&raw))], &items, &CompatFeed::default()).is_empty(),
            "a warning must never become an Apply refusal"
        );
        assert!(
            try_import(&raw, &items, &CompatFeed::default()).is_ok(),
            "…on either door"
        );
    }

    /// **Remove Everything must stay removed.** The strip writes an explicit empty document
    /// rather than clearing the field, because a cleared field has no `cargo` key and no `cargo`
    /// key is exactly the condition on which `seed_cargo` puts the character's default magazines
    /// back — a strip verb that undoes itself the next time the panel opens.
    ///
    /// RED: drop `"cargo": []` from `stripped_loadout` → "the strip must mark cargo as
    /// user-cleared".
    #[test]
    fn remove_everything_strips_the_kit_and_stops_the_cargo_reseed() {
        let stripped = stripped_loadout();
        assert!(
            loadout_to_picks(Some(&stripped)).is_empty(),
            "no wear row and no weapon survives a strip"
        );
        let (rows, present) = rules::cargo_from_loadout(Some(&stripped));
        assert!(rows.is_empty(), "no cargo survives a strip");
        assert!(present, "the strip must mark cargo as user-cleared");
        // Every wear row the persist path emits is present and null — the vocabulary comes from
        // ROWS, so this document and `picks_to_loadout`'s cannot drift apart.
        let v: serde_json::Value = serde_json::from_str(&stripped).expect("valid JSON");
        let wear = v["wear"].as_object().expect("a wear block");
        assert_eq!(
            wear.len(),
            ROWS.iter().filter(|r| r.weapon.is_none()).count()
        );
        assert!(wear.values().all(serde_json::Value::is_null));
        assert_eq!(v["weapons"], serde_json::json!([]));

        // THE HAZARD, demonstrated rather than asserted about: the seed rule really does fire on
        // a cleared field, and really does not fire on this document.
        let defaults = vec![row("vest", "res://mag_stanag", 3)];
        assert!(
            website_map_engine::data::store::operations::cargo_rules::seed_cargo(None, &defaults)
                .is_some(),
            "a cleared loadout field re-seeds — this is what the strip must not leave behind"
        );
        assert!(
            website_map_engine::data::store::operations::cargo_rules::seed_cargo(
                Some(&stripped),
                &defaults
            )
            .is_none(),
            "the stripped document must be seed-ineligible, or Remove Everything undoes itself"
        );

        // And the plan: one write per target, each carrying that document.
        let writes = plan_remove(&ids(&["a", "b"]));
        assert_eq!(writes.len(), 2);
        assert!(writes.iter().all(|w| w.source_id.is_none()));
        assert!(writes
            .iter()
            .all(|w| w.loadout_json.as_deref() == Some(stripped.as_str())));
    }

    /// `plan_remove` runs no gate because the stripped document cannot fail one. That is a claim
    /// about the RULES module, which can change, so it is a test and not a comment.
    #[test]
    fn the_stripped_document_passes_every_rule() {
        let items = capacity_catalog();
        for feed in [CompatFeed::default(), attachment_feed(&[])] {
            assert!(
                buffer_refusals(&[buf("s", Some(&stripped_loadout()))], &items, &feed).is_empty(),
                "a stripped loadout must be unconditionally applicable"
            );
        }
    }

    /// **The undo arithmetic, measured — not counted in the source text.**
    ///
    /// Wave 112's one-commit pin counted the literal `persist(` and stayed green under an N-step
    /// perturbation (T-736). This one runs the commit path against a sink that records what it
    /// was actually handed, so the number in the receipt is the number of documents written. An
    /// Apply over N entities is N transactions and therefore N undo steps — the core has no
    /// atomic multi-entity loadout write (T-732) — and the receipt says so out loud rather than
    /// claiming an atomicity nothing here provides.
    ///
    /// RED (a dropped write): force the sink to return `false` for one id →
    /// "3 writes planned, sink took 2" and the receipt WARNING arm.
    /// RED (count invocations again): `done += 1` unconditionally → miss path no longer red.
    /// RED (a fake one-step claim): report `1` instead of the commit count → the receipt no
    /// longer names the real number of Ctrl+Z presses.
    #[test]
    fn the_receipt_counts_the_writes_the_document_actually_took() {
        let writes = plan_remove(&ids(&["a", "b", "c"]));
        let mut sink: Vec<(String, Option<String>)> = Vec::new();
        let commits = commit_writes(&writes, |id, json| {
            sink.push((id.to_string(), json));
            true
        });

        assert_eq!(commits, 3, "one commit per planned write");
        assert_eq!(
            sink.len(),
            writes.len(),
            "{} writes planned, sink took {}",
            writes.len(),
            sink.len()
        );
        let seen: Vec<&str> = sink.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            seen,
            ["a", "b", "c"],
            "every target is written exactly once"
        );

        let line = remove_receipt(writes.len(), commits);
        assert!(line.contains("3 undo step(s)"), "{line}");
        assert!(
            line.contains("T-732"),
            "the receipt must cite the gap: {line}"
        );
        assert!(!line.contains("WARNING"), "{line}");

        // The honesty property: a sink that refuses one id (the production shape when
        // `update_slot_loadout` returns false) must shrink the counted commits and light the
        // WARNING arm. Counting loop invocations would keep commits==3 and hide the miss.
        let mut miss_sink: Vec<(String, Option<String>)> = Vec::new();
        let miss_commits = commit_writes(&writes, |id, json| {
            if id == "b" {
                return false;
            }
            miss_sink.push((id.to_string(), json));
            true
        });
        assert_eq!(miss_commits, 2, "refused ack must not count as a commit");
        assert_eq!(
            miss_sink.len(),
            2,
            "{} writes planned, sink took {}",
            writes.len(),
            miss_sink.len()
        );
        let dropped = remove_receipt(writes.len(), miss_commits);
        assert!(dropped.contains("WARNING"), "{dropped}");
        assert!(
            dropped.contains("3 write(s) were planned and 2 reached the document"),
            "{dropped}"
        );

        // Apply says the same three things: how many landed, that it is one step each, and why.
        let apply = apply_receipt(5, 2, 5);
        assert!(apply.contains("Applied 5 loadout(s)"), "{apply}");
        assert!(apply.contains("2-loadout buffer"), "{apply}");
        assert!(apply.contains("5 undo step(s)"), "{apply}");
        assert!(apply.contains("Ctrl+Z 5 times"), "{apply}");
        assert!(apply.contains("T-732"), "{apply}");
        assert!(apply_receipt(5, 2, 4).contains("WARNING"));
    }

    /// The Copy receipt counts the bare kits out loud — buffering forty empty soldiers and
    /// discovering it only after Apply is exactly the surprise a receipt exists to prevent.
    #[test]
    fn the_copy_receipt_reports_what_was_buffered_including_the_bare_ones() {
        let doc = kit_doc("res://rifle_m16");
        let line = copy_receipt(&[buf("a", Some(&doc)), buf("b", Some(&doc))]);
        assert!(line.contains("Copied 2 loadout(s)"), "{line}");
        assert!(line.contains("at random"), "{line}");
        assert!(!line.contains("no loadout at all"), "{line}");

        let mixed = copy_receipt(&[buf("a", Some(&doc)), buf("b", None), buf("c", None)]);
        assert!(mixed.contains("Copied 3 loadout(s)"), "{mixed}");
        assert!(
            mixed.contains("2 of them carry no loadout at all"),
            "{mixed}"
        );
    }
}
