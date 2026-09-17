use super::*;

/* ═══════════ T-503 — the Arsenal commits on the spot, and now says so ═══════════ */

/// arsenal.rs with everything **unreachable** removed, so a source pin cannot be greened by a
/// needle that no running build can reach.
///
/// T-601 moved the machinery to [`crate::v2::core::test_support::class_r_scrub`], which every Class-R pin in this crate
/// now shares. The behaviour it replaced was literal matching: `#[cfg(any())]` was a **string**
/// compare and the constant-false conditions were a **seven-entry whitelist**, so
/// `#[cfg( any() )]`, `if 1 > 2`, `if std::hint::black_box(false)` and `while false` all walked
/// straight past it (measured, wave 77 F3). The replacement parses the `cfg` predicate and
/// constant-folds the condition, so spelling and whitespace stop being the defence.
fn live_production_src() -> String {
    // T-934.8 — the Arsenal production surface spans three files. Every pin below is
    // scoped through `only_body`, and concatenating all three keeps the whole-surface
    // claims whole-surface. Each file is scrubbed SEPARATELY: `cut_test_module` truncates
    // a haystack at its first cfg-test attribute, so scrubbing a concatenation would stop
    // examining everything after the first file's test tail.
    [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/mod.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout/attachments_and_faults.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout/buffered_loadout_operations.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout/loadout_export.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout/loadout_import.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout/slot_loadout_serialization.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/tab_content.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/tab_content/catalog_header.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/tab_content/selection_grid.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/tab_content/status_and_persistence.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/arsenal/panels.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/arsenal/panels/cargo_panel.rs"
        )),
    ]
    .into_iter()
    .map(crate::v2::core::test_support::class_r_scrub::live_code)
    .collect()
}

use crate::v2::core::test_support::class_r_scrub::only_body as fn_body;

/// T-503 Class-R: the panel must state the persistence contract, because the platform's only
/// unsaved indicator (the top-strip `•`) sits behind this modal's blur scrim.
///
/// RED (removed): delete the `data-arsenal-persist` block from the view → "the Arsenal must
/// carry a data-arsenal-persist line".
/// RED (decoy): re-add it inside `if true == false { … }` → same failure.
#[test]
fn the_panel_states_the_persistence_contract() {
    let live = live_production_src();
    let tab = fn_body(&live, "pub(super) fn status_and_persistence(");
    assert!(
        tab.contains("data-arsenal-persist"),
        "the Arsenal must carry a data-arsenal-persist line the author can read"
    );
    for needle in [
        "PERSIST_ALWAYS",
        "PERSIST_CLEAN",
        "PERSIST_UNSAVED",
        "mission_has_unsaved_work()",
    ] {
        assert!(
            tab.contains(needle),
            "the persistence line must render {needle} on a live path"
        );
    }
    // The verdict badge and the per-row line both read `loadout_faults`, which is where the
    // T-504 warning lands — if either stops, the warning stops being visible.
    assert!(
        fn_body(&live, "pub(super) fn loaded_catalog(").contains("loadout_faults(")
            && fn_body(&live, "pub(super) fn selection_grid(").contains("loadout_faults("),
        "both the per-row line and the verdict badge must read loadout_faults"
    );

    // The shipped copy has to answer the question the author actually has ("did that stick?")
    // without claiming the mission is on the server, which is a different promise.
    assert!(
        PERSIST_ALWAYS.contains("no Save button"),
        "{PERSIST_ALWAYS}"
    );
    assert!(PERSIST_ALWAYS.contains("Ctrl+Z"), "{PERSIST_ALWAYS}");
    assert!(
        PERSIST_UNSAVED.contains("Save Version"),
        "{PERSIST_UNSAVED}"
    );
    assert!(
        PERSIST_CLEAN.contains("no unsaved changes"),
        "{PERSIST_CLEAN}"
    );
    assert!(!mission_has_unsaved_work(), "native shell hosts no editor");
}

/* ═══════════ T-686 — the import half of the round-trip ═══════════ */
//
// T-934.8 — the pure import-gate tests moved beside the code they pin
// (`loadout.rs::tests::t686`); the ArsenalTab wiring pin stays beside the view.

mod t686 {
    use super::*;

    /// T-686 / T-736 Class-R: the import must reach the live document through EXACTLY ONE
    /// commit, so Ctrl+Z restores the whole pre-import loadout rather than the last field of
    /// it. The body is pinned by SYMBOL (`let apply_import =`) through `live_code` +
    /// `only_body`: three whole-document signal writes + one `persist`, and **nothing else**.
    ///
    /// A spelling blacklist of `for` / `while` / `.iter()` is NOT enough (wave-112 MINOR-1):
    /// `.into_iter().map(|p| persist1(p)).count()`, a bare `loop {}`, or a recursive helper
    /// all keep one textual `persist(` and dodge those four needles. The leftover-token check
    /// below is the wide negative — any extra alphanumeric residue is the N-step class.
    ///
    /// RED (N steps / `for`): per-pick `persist` loop → leftover tokens (and/or persist count).
    /// RED (N steps / `into_iter`): `doc.picks.into_iter().map(|p| persist1(p)).count()` →
    /// leftover `into_iter` / `map` / `count` (the old blacklist missed this — `.into_iter()`
    /// does not contain `.iter()`).
    /// RED (N steps / `loop`): bare `loop { persist(...); break; }` → leftover `loop`/`break`.
    /// RED (ungated): call `apply_import` outside the `Ok(doc)` arm → the `try_import` pin.
    /// RED (decoy, `#[cfg(any())]`): park the picker in a dead item → same failure.
    #[test]
    fn the_import_applies_in_one_commit() {
        let live = live_production_src();
        let tab = fn_body(&live, "pub(super) fn loaded_catalog(");
        assert!(
            tab.contains("try_import("),
            "the import must be gated on a live path"
        );
        assert!(
            tab.contains("apply_import(doc, &its)"),
            "only an accepted document may be applied"
        );
        assert!(
            tab.contains("data-loadout-import"),
            "the panel must carry an import control the author can reach"
        );

        // Locate by SYMBOL — never a whole-file needle hunt that can match the test module.
        let apply = fn_body(&live, "let apply_import =");
        let commits = apply.matches("persist(").count();
        assert_eq!(
            commits, 1,
            "an import is ONE undo step: apply_import must commit exactly once, found {commits}"
        );
        // Positive shape: the whole document lands through three signal writes, then the one
        // shared commit. Exact args so a per-field `picks.set(k, v)` walk cannot green this.
        const WHOLE_DOC: &[&str] = &[
            "picks.set(doc.picks)",
            "cargo.set(doc.cargo)",
            "cargo_present.set(doc.cargo_present)",
            "persist(&picks.get_untracked(), items)",
        ];
        let mut rest = apply.to_string();
        for needle in WHOLE_DOC {
            assert!(
                rest.contains(needle),
                "the apply must replace the whole loadout in one commit — missing `{needle}`"
            );
            rest = rest.replacen(needle, "", 1);
        }
        // Wide negative (T-736): after the four known live statements are removed, no
        // alphanumeric residue may remain. That is the class the spelling blacklist missed —
        // `into_iter` / `map` / `loop` / a recursive helper name all leave tokens here.
        let leftover: String = rest
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        assert!(
            leftover.is_empty(),
            "an import is ONE undo step: apply_import must be three whole-document signal \
             writes + one persist and nothing else — leftover tokens {leftover:?} betray an \
             N-step walk (for / into_iter / loop / recursion / …)"
        );
        assert!(
            !apply.contains("set_loadout"),
            "the apply must go through the same `persist` every other pick uses"
        );
    }
}

/* ═════════ T-699 — the loadout buffer: Copy · Apply (random) · Remove Everything ═════════ */
//
// T-934.8 — the pure planner/receipt tests moved beside the code they pin
// (`loadout.rs::tests::t699`); the ops and panel wiring pins stay here.

mod t699 {
    use super::*;

    /// `editor_ops.rs` with everything unreachable removed. Note it carries **no test module at
    /// all**, so the T-759 hazard that makes an `include_str!` pin match its own fixtures cannot
    /// arise here — asserted below rather than assumed, because the day somebody adds one is the
    /// day this pin needs re-reading.
    fn live_ops_src() -> String {
        // T-934.7 — the ops module was split; the exclusion pins below are whole-module
        // claims, so the haystack concatenates every submodule.
        crate::v2::core::test_support::class_r_scrub::live_code(
            &[
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/v2/apps/editor/arsenal/loadout_commands.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/slot_loadouts.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/composition_library.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/data/store/operations/compositions.rs"
                )),
                crate::v2::core::test_support::editor_operations::CONTEXT,
                crate::v2::core::test_support::editor_operations::ENTITY,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/selection_transform.rs"
                )),
            ]
            .concat(),
        )
    }

    /// Class-R: the three verbs must reach the live document through the gated, counted path —
    /// and the excluded nine must not have crept in.
    ///
    /// RED (ungated): call `plan_remove` from the apply verb instead of `plan_apply` → "Apply
    /// must be gated on plan_apply".
    /// RED (fake atomicity): move `after_local_edit()` inside the write loop, or add a second
    /// one → the one-tail assertion.
    /// RED (scope creep): add `remove_nvgs_from_selection` → the exclusion assertion names it.
    #[test]
    fn the_ops_layer_wires_the_three_verbs_and_only_the_three() {
        let ops = live_ops_src();
        assert!(
            !ops.contains("#[cfg(test)]"),
            "editor_ops.rs grew a test module — this pin's SRC would now match its own \
             fixtures (T-759). Truncate SRC at the test module before trusting it again."
        );

        let copy = fn_body(&ops, "pub fn copy_loadouts_from_selection(");
        assert!(
            copy.contains("cargo::buffer_loadouts_from_selection("),
            "Copy must reach the buffer through the one buffering verb; body: {copy}"
        );
        let domain_cargo =
            crate::v2::core::test_support::class_r_scrub::live_code(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/data/store/operations/cargo.rs"
            )));
        let domain_copy = fn_body(&domain_cargo, "pub fn buffer_loadouts_from_selection(");
        assert!(
            domain_copy.contains("LOADOUT_BUFFER")
                && domain_copy.contains("copy_loadouts_from_selection("),
            "Copy must buffer every SELECTED slot; body: {domain_copy}"
        );
        assert!(
            fn_body(&domain_cargo, "pub fn copy_loadouts_from_selection(")
                .contains("BufferedLoadout"),
            "Copy must buffer bytes, not source ids — an id would be inheritance (T-687)"
        );

        let apply = fn_body(&ops, "pub fn apply_loadout_buffer_to_selection(");
        assert!(
            apply.contains("plan_apply("),
            "Apply must be gated on plan_apply — the T-686 rule pass"
        );
        assert!(
            !apply.contains("update_slot_loadout"),
            "Apply must not write the document behind the shared committer's back"
        );
        assert!(
            fn_body(&ops, "pub fn remove_all_loadouts_from_selection(").contains("plan_remove("),
            "Remove Everything must go through the same planner"
        );

        // The one place a loadout write reaches the document, and the undo arithmetic that
        // makes it honest: N transactions, ONE shared post-change tail (which is NOT an undo
        // boundary — see T-732).
        let commit = fn_body(&ops, "fn commit_loadout_writes(");
        assert!(commit.contains("cargo::commit_loadout_writes(core, writes)"));
        let domain =
            crate::v2::core::test_support::class_r_scrub::live_code(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/data/store/operations/cargo.rs"
            )));
        let domain_commit = fn_body(&domain, "pub fn commit_loadout_writes(");
        assert_eq!(
            domain_commit.matches("update_slot_loadout(").count(),
            1,
            "exactly one write call site: {commit}"
        );
        assert_eq!(
            commit.matches("after_local_edit(").count(),
            1,
            "exactly one shared tail, fired after the writes — not per write"
        );
        assert!(
            domain_commit.contains("commit_writes("),
            "the write loop is `arsenal::commit_writes`, so the count is testable natively"
        );

        // The nine per-category strip variants are `maybe` upstream and deliberately excluded.
        for excluded in [
            "remove_nvgs",
            "remove_vests",
            "remove_goggles",
            "remove_headgear",
            "remove_weapons",
            "remove_backpack",
        ] {
            assert!(
                !ops.contains(excluded),
                "`{excluded}` is one of the nine excluded per-category strip verbs (marked \
                 `maybe`); T-699 ships Remove Everything and nothing narrower"
            );
        }
    }

    /// Class-R: the panel must carry all three controls and route each to its verb, and the
    /// Apply must resync this modal's signals WITHOUT a second commit — an extra `persist` there
    /// would put one more Ctrl+Z press between the author and the state they had.
    #[test]
    fn the_panel_carries_the_three_verbs_and_resyncs_without_recommitting() {
        let live = live_production_src();
        let tab = fn_body(&live, "pub(super) fn loaded_catalog(");
        for needle in [
            "data-loadout-copy",
            "data-loadout-apply",
            "data-loadout-strip",
        ] {
            assert!(tab.contains(needle), "the panel must carry {needle}");
        }
        for wiring in [
            "on:click=copy_loadouts",
            "on:click=apply_loadouts",
            "on:click=strip_loadouts",
        ] {
            assert!(
                tab.contains(wiring),
                "an unwired control is not a verb: {wiring}"
            );
        }
        let resync = fn_body(&live, "let resync_open_slot =");
        assert!(
            resync.contains("read_loadout("),
            "the resync must re-read the live document"
        );
        assert!(
            !resync.contains("persist("),
            "the resync is signal writes only — a persist here is an extra undo step"
        );
    }
}

/// T-737 — the rendering wiring half; the refusal-construction tests moved beside the
/// code they pin (`loadout.rs::tests::t737`).
mod t737 {
    use super::*;

    /// Class-R: the fix has to be in the panel, not merely available to it. Both refusal lists
    /// — the import's and the Apply's — must render through `refusal_line`.
    ///
    /// RED: revert either list to `.map(|e| e.message)` → the count drops to 1.
    #[test]
    fn both_refusal_lists_render_through_refusal_line() {
        let live = live_production_src();
        let tab = fn_body(&live, "pub(super) fn loaded_catalog(");
        assert_eq!(
            tab.matches("refusal_line").count(),
            2,
            "both refusal lists must name the row; found: {}",
            tab.matches("refusal_line").count()
        );
    }
}

/* ═══════════ T-739 — inverted suppress-on-multi claim cannot return ═══════════ */

/// Class-R for the wave-112 NIT that became T-739: gap_analysis asserted multi-selection
/// **suppresses** the Attributes modal, and a T-648 comment in `editor_ops` still said the
/// same after T-649 inverted the guard. Pins are semantic (no false phrase) plus live line
/// cites for `set_loadout` / its `after_local_edit` tail — hardcoding a stale number goes red
/// the moment either cite drifts again.
///
/// RED (false comment returns): restore `(it suppresses on a multi-selection)` in
/// `rotate_selection_to_face`'s doc → "editor_ops must not re-claim suppress-on-multi".
/// RED (gap falsehood returns): restore `multi-selection **suppresses**` in gap_analysis →
/// "gap_analysis must not re-claim suppress-on-multi".
/// RED (stale arsenal cite): change either `editor_ops.rs:NNNN` cite away from the live
/// `pub fn set_loadout` / its tail line → "arsenal must cite the live set_loadout line".
mod t739 {
    /// Production `editor_ops` source (comments kept — the defect lives in a doc
    /// comment). T-934.7 — the module was split; the suppress-on-multi absences are
    /// whole-module claims, so this concatenates every submodule.
    fn ops_src() -> String {
        [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/loadout_commands.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/editing/hosted_commands/slot_loadouts.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/editing/hosted_commands/composition_library.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/data/store/operations/compositions.rs"
            )),
            crate::v2::core::test_support::editor_operations::CONTEXT,
            crate::v2::core::test_support::editor_operations::ENTITY,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../map-engine/src/editing/hosted_commands/selection_transform.rs"
            )),
        ]
        .concat()
    }

    /// `loadout_commands.rs` alone — the file `set_loadout` lives in, so the computed line
    /// numbers below are REAL lines of that file.
    fn cargo_src() -> &'static str {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout_commands.rs"
        ))
    }

    fn arsenal_production_src() -> String {
        // Keep comments (the cites live there). Truncate each file at its first
        // `#[cfg(test)]` so the pin modules' own RED prose cannot green or red the
        // production-cite asserts. T-934.8 — the absence claims below are whole-surface
        // claims, so every arsenal production half concatenates.
        [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/mod.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/loadout.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/loadout/attachments_and_faults.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/loadout/buffered_loadout_operations.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/loadout/loadout_export.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/loadout/loadout_import.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/loadout/slot_loadout_serialization.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/tab_content.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/tab_content/catalog_header.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/tab_content/selection_grid.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/arsenal/tab_content/status_and_persistence.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/arsenal/panels.rs"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/arsenal/panels/cargo_panel.rs"
            )),
        ]
        .into_iter()
        .map(|full| full.split("#[cfg(test)]").next().unwrap_or(full))
        .collect()
    }

    fn gap_src() -> &'static str {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md"
        ))
    }

    fn live_set_loadout_lines(ops: &str) -> (usize, usize) {
        let lines: Vec<&str> = ops.lines().collect();
        let set_idx = lines
            .iter()
            .position(|l| l.starts_with("pub fn set_loadout"))
            .expect("pub fn set_loadout must exist");
        let next_pub = lines[set_idx + 1..]
            .iter()
            .position(|l| l.starts_with("pub fn "))
            .map(|i| set_idx + 1 + i)
            .expect("a following pub fn after set_loadout");
        let tail_idx = lines[set_idx..next_pub]
            .iter()
            .position(|l| l.contains("mission_history::after_local_edit()"))
            .map(|i| set_idx + i)
            .expect("set_loadout must fire after_local_edit");
        (set_idx + 1, tail_idx + 1)
    }

    #[test]
    fn editor_ops_must_not_reclaim_suppress_on_multi() {
        let ops = ops_src();
        assert!(
            !ops.contains("it suppresses on a multi-selection"),
            "T-739: editor_ops must not re-claim suppress-on-multi after T-649 inverted it"
        );
        assert!(
            !ops.contains("suppresses on a multi-selection"),
            "T-739: editor_ops must not re-claim suppress-on-multi after T-649 inverted it"
        );
        // Positive: the shared opener still documents the inversion.
        assert!(
            ops.contains("sel.len() > 1 && sel.contains(&id)")
                && ops.contains("A multi-selection now OPENS the modal"),
            "T-739: open_attrs_modal must keep the T-649 inversion prose"
        );
    }

    #[test]
    fn gap_analysis_must_not_reclaim_suppress_on_multi() {
        let gap = gap_src();
        assert!(
            !gap.contains("multi-selection **suppresses**"),
            "T-739: gap_analysis ATTR-OPEN-001 must not re-claim suppress-on-multi"
        );
        assert!(
            !gap.contains("multi-select suppression"),
            "T-739: gap_analysis must not re-claim multi-select suppression"
        );
        assert!(
            gap.contains("a multi-selection now OPENS multi-edit") && gap.contains("T-649 ✅"),
            "T-739: gap_analysis must state the T-649 open-on-multi truth"
        );
    }

    #[test]
    fn arsenal_cites_live_set_loadout_lines() {
        let ops = cargo_src();
        // Production only — the pin module itself names the old numbers in RED prose, and
        // include_str!(arsenal.rs) would otherwise false-fail on its own commentary.
        let arsenal = arsenal_production_src();
        let (set_line, tail_line) = live_set_loadout_lines(ops);
        let set_cite = format!("loadout_commands.rs:{set_line}");
        let tail_cite = format!("loadout_commands.rs:{tail_line}");
        assert!(
            arsenal.contains(&set_cite),
            "T-739: arsenal module docs must cite live set_loadout at {set_cite}"
        );
        assert!(
            arsenal.contains(&tail_cite),
            "T-739: arsenal import undo note must cite live after_local_edit at {tail_cite}"
        );
        // Stale numbers from the wave-112 filing must stay gone from production source.
        assert!(
            !arsenal.contains("editor_ops.rs:777"),
            "T-739: arsenal production source must not keep drifted cite editor_ops.rs:777"
        );
        assert!(
            !arsenal.contains("editor_ops.rs:1611"),
            "T-739: arsenal production source must not keep drifted cite editor_ops.rs:1611"
        );
    }
}

/* ═══════════ T-779 — the single write path must not fake its acknowledgement ═══════════ */

/// T-770 gave `MissionDocCore::update_slot_loadout` a `bool` and taught the BATCH path
/// ([`commit_writes`]) to count it. The frontend half never landed: `loadout_commands::set_loadout`
/// called the mutator as a statement and hardcoded `true` for `did`, so the history tail fired
/// whenever `EDITOR_CONTEXT` and the document merely existed. A pick against a slot id the mission no
/// longer held dirtied the mission and minted an undo step over a document that had not
/// changed, and no receipt on that path could tell a write from a no-op.
///
/// Two pins, because the defect has two halves and one test cannot see both:
///
/// * **Behaviour** — [`commit_one_write`] is driven with a refusing sink, which is the exact
///   production shape for an unknown id. This is the only half that can be *run*: `editor_ops`
///   is `cfg(target_arch = "wasm32")` from its first line, `MissionDocCore` is behind the
///   wasm-only `doc` feature, and neither is reachable from a native test.
/// * **Wiring** — the live, scrubbed `editor_ops.rs` must actually route through that gate and
///   must not carry the discarded-ack statement anywhere. The negative runs over the WHOLE
///   live module and is never scoped to an item, so moving the offending statement elsewhere
///   cannot green it.
///
/// RED (restore the hardcoded `true`): put `core.update_slot_loadout(id, loadout_json);` back
/// as a statement with `true` under it → "T-779: editor_ops must not discard the
/// update_slot_loadout acknowledgement".
/// RED (ungate the tail): fire the tail unconditionally inside `commit_one_write` →
/// "T-779: a refused write must mint no history tail".
/// RED (swallow the answer): drop `-> bool` from `set_loadout` → "T-779: set_loadout must
/// return the document's answer".
/// RED (bypass the gate): call `after_local_edit` directly from `set_loadout` again →
/// "T-779: set_loadout must gate its tail through arsenal::commit_one_write".
mod t779 {
    use super::*;

    /// `editor_ops.rs` with comments and unreachable constructs removed, and string/char
    /// literals blanked — the needles below are calls and shapes, never copy, so a decoy
    /// parked in a literal must not match. It carries no test module of its own (asserted).
    fn live_ops() -> String {
        // T-934.7 — whole-module negatives (the discarded-ack statement must appear NOWHERE
        // in the ops surface), so the haystack concatenates every submodule.
        crate::v2::core::test_support::class_r_scrub::live_code(
            &[
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/v2/apps/editor/arsenal/loadout_commands.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/slot_loadouts.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/composition_library.rs"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/data/store/operations/compositions.rs"
                )),
                crate::v2::core::test_support::editor_operations::CONTEXT,
                crate::v2::core::test_support::editor_operations::ENTITY,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/editing/hosted_commands/selection_transform.rs"
                )),
            ]
            .concat(),
        )
    }

    /// The live wiring: `set_loadout` returns the document's answer and gates the tail on it.
    #[test]
    fn set_loadout_returns_the_documents_answer_instead_of_a_hardcoded_true() {
        let ops = live_ops();
        assert!(
            !ops.contains("#[cfg(test)]"),
            "editor_ops.rs grew a test module — this pin's SRC would now match its own \
             fixtures (T-759). Truncate SRC at the test module before trusting it again."
        );

        // NEGATIVE — deliberately unscoped. The defect is a statement that discards the
        // mutator's `bool`; scoping this to `set_loadout` would let the same shape reappear in
        // any sibling and stay green.
        assert!(
            !ops.contains("core.update_slot_loadout(id, loadout_json);"),
            "T-779: editor_ops must not discard the update_slot_loadout acknowledgement — \
             that `bool` is what T-770 added and what tells a write from a no-op"
        );

        let item =
            crate::v2::core::test_support::class_r_scrub::only_item(&ops, "pub fn set_loadout(");
        assert!(
            item.contains("-> bool"),
            "T-779: set_loadout must return the document's answer so the caller can surface a \
             refusal: {item}"
        );
        assert!(
            item.contains("commit_one_write("),
            "T-779: set_loadout must gate its tail through arsenal::commit_one_write, the one \
             seam where the refusal→no-tail arithmetic can be driven natively: {item}"
        );
        assert_eq!(
            item.matches("after_local_edit(").count(),
            1,
            "T-779: exactly one tail, and it must sit inside the gate: {item}"
        );
        assert_eq!(
            item.matches("update_slot_loadout(").count(),
            1,
            "T-779: exactly one write call site in set_loadout: {item}"
        );

        // The seed siblings carried the same shape and were fixed in the same pass.
        assert!(
            !ops.contains("core.update_slot_loadout(id, Some(json));"),
            "T-779: seed_cargo_in_core must return the sink's answer, not a hardcoded true"
        );
        assert!(
            !ops.contains("core.update_slot_loadout(id, Some(json.clone()));"),
            "T-779: seed_slot_cargo's Option must carry the sink's answer — its own tail is \
             gated on that Option being Some"
        );
    }

    /// The refusal has to reach the OPERATOR. Gating the tail correctly means a refused pick no
    /// longer dirties the mission — so the persistence line, left alone, would answer the
    /// author's "did that stick?" with a green "The mission has no unsaved changes" over a pick
    /// that never landed. That is the wave-129 rule violated by the fix itself, so the panel
    /// grows a third state that overrides both of the others.
    #[test]
    fn a_refused_pick_is_visible_in_the_panel_not_silent() {
        let live = live_production_src();
        let tab = fn_body(&live, "pub(super) fn status_and_persistence(");
        assert!(
            tab.contains("persist_refused"),
            "T-779: the panel must hold the refusal state, or a refused pick is silent"
        );
        assert!(
            tab.contains("PERSIST_REFUSED"),
            "T-779: the persistence line must be able to render the refusal copy"
        );
        // The refusal must be checked BEFORE the dirty flag: `mission_has_unsaved_work()` stays
        // accurate during a refusal and would otherwise get to answer the wrong question.
        let refused_at = tab
            .find("persist_refused.get()")
            .expect("T-779: the persistence line must READ the refusal state, not just hold it");
        let unsaved_at = tab
            .find("mission_has_unsaved_work()")
            .expect("the dirty read must still be there");
        assert!(
            refused_at < unsaved_at,
            "T-779: the refusal must be decided before the dirty flag — a mission with no \
             unsaved work is a true statement and a misleading answer when the last pick was \
             refused"
        );

        // The commit must CAPTURE the answer rather than call and forget. Checked structurally
        // (is the call bound to something?) and not by matching one formatting of one line.
        let persist = fn_body(&live, "let persist =");
        let call_at = persist
            .find("loadout_commands::set_loadout(")
            .expect("T-779: the Arsenal must still reach set_loadout on a live path");
        let before = &persist[..call_at];
        assert!(
            before.trim_end().ends_with('='),
            "T-779: the Arsenal must not call set_loadout as a bare statement — the return is \
             the only thing that can tell the author the write was refused. Preceding text: {}",
            &before[before.len().saturating_sub(80)..]
        );
        assert!(
            persist.contains("persist_refused.set("),
            "T-779: the captured answer must reach the panel state, or it is captured and \
             thrown away"
        );

        // The shipped copy has to say what happened and what to do, without claiming the
        // mission is broken. Constants, so this reads the live strings.
        assert!(
            PERSIST_REFUSED.contains("did NOT reach"),
            "{PERSIST_REFUSED}"
        );
        assert!(
            PERSIST_REFUSED.contains("no longer in the mission"),
            "the refusal must name the CAUSE, or the author cannot act on it: \
             {PERSIST_REFUSED}"
        );
        assert!(
            !PERSIST_REFUSED.contains("no unsaved changes"),
            "the refusal must not repeat the clean verdict: {PERSIST_REFUSED}"
        );
    }
}
