use super::*;

/// A minimal tree in text form. Each test perturbs one field — the same discipline the gate
/// applies to itself at runtime, at unit-test speed.
const SEED_OK: &str = "-- starter library\nINSERT INTO user_factions (name, side)\n  VALUES ('US Army 1980s', 'BLUFOR');\n";
/// Built from [`WAVE_RUN_LINE`] rather than hand-written, so the fixture cannot drift from
/// the const the way it did when T-853 repointed the real call sites to `cargo xtask`.
fn wave_ok() -> String {
    format!(
        "const VERIFY_STEPS: &[(&str, &str)] = &[\n{WAVE_RUN_LINE}];\n\n\
pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {{\n    {VERIFY_LOOP} {{ let _ = (label, name); }}\n    0\n}}\n\n\
pub fn cmd_gate(ctx: &Ctx, base_arg: &str) -> u8 {{\n    {VERIFY_LOOP} {{ let _ = (label, name); }}\n    0\n}}\n"
    )
}

fn fails(seed: &str, seeds: &[&str], wave: &str) -> Vec<String> {
    assert_t440_pins(seed, seeds, wave)
        .expect("constant patterns compile")
        .iter()
        .map(|verdict| match verdict {
            Verdict::Held => unreachable!("Held is never collected"),
            Verdict::Failed(f) | Verdict::DidNotRun(_, f) => f.headline.clone(),
        })
        .collect()
}

#[test]
fn live_inputs_hold() {
    assert!(fails(SEED_OK, SEEDS, &wave_ok()).is_empty());
}

/// RED 1, the T-478 headline defect: the name in a `--` comment must not satisfy the pin.
#[test]
fn comment_only_starter_name_is_not_a_seed() {
    let out = fails(
        &format!("-- {STARTER_NAME}\nSELECT 1;\n"),
        SEEDS,
        &wave_ok(),
    );
    assert_eq!(out.len(), 1);
    assert!(out[0].starts_with("seed must contain live `INSERT INTO user_factions`"));
}

/// The `;` exclusion: an unrelated insert plus the name in a later statement is not a match.
#[test]
fn name_in_a_different_statement_is_not_a_seed() {
    let seed = "INSERT INTO user_factions (name) VALUES ('OPFOR');\nSELECT 'US Army 1980s';\n";
    assert_eq!(fails(seed, SEEDS, &wave_ok()).len(), 1);
}

/// An emptied seed is caught by the pin as well as by the `-s` pre-flight.
#[test]
fn emptied_seed_fails_the_pin() {
    assert_eq!(fails("", SEEDS, &wave_ok()).len(), 1);
}

/// RED 2 and RED 2b, post-T-897: the seed must be a MEMBER of the list the seeder walks, and
/// membership is by equality. Both fixtures are DERIVED from the live const.
#[test]
fn the_seeder_must_apply_the_file_not_merely_name_it() {
    let dropped = seeds_without(SEEDS, SEED_ENTRY, "test").expect("live const has the entry");
    let out = fails(SEED_OK, &borrow(&dropped), &wave_ok());
    assert_eq!(out.len(), 1);
    assert!(
        out[0].contains("must apply faction_library.sql"),
        "{}",
        out[0]
    );

    let mut lookalike = dropped.clone();
    lookalike.push(SEED_LOOKALIKE);
    let out = fails(SEED_OK, &borrow(&lookalike), &wave_ok());
    assert_eq!(out.len(), 1);
    assert!(out[0].contains("look-alike does not count"), "{}", out[0]);
}

/// A gutted list gets its own message: "applies nothing" is a different fix from "applies the
/// wrong things".
#[test]
fn an_empty_seed_list_is_its_own_failure() {
    let out = fails(SEED_OK, &[], &wave_ok());
    assert_eq!(out.len(), 1);
    assert!(out[0].contains("is empty"), "{}", out[0]);
}

/// The RED-arm setup guard: a const that no longer carries the entry must abort the proof
/// rather than print "→ FAIL (expected)" over a perturbation that changed nothing.
#[test]
fn red_setup_refuses_a_list_it_does_not_recognise() {
    assert!(seeds_without(&["other.sql"], SEED_ENTRY, "test").is_none());
}

/// RED 3 — dropping the VERIFY_STEPS row, or either function's loop, is reported.
#[test]
fn both_wave_paths_are_pinned() {
    let row_gone = wave_ok().replacen(WAVE_RUN_LINE, "", 1);
    let out = fails(SEED_OK, SEEDS, &row_gone);
    assert!(
        out.iter().any(|h| h.contains("VERIFY_STEPS missing t440")),
        "{out:?}"
    );

    let commented = wave_ok().replacen(VERIFY_REL, &format!("// {VERIFY_REL}"), 1);
    assert!(
        fails(SEED_OK, SEEDS, &commented)
            .iter()
            .any(|h| h.contains("VERIFY_STEPS missing t440")),
        "commented row must not satisfy the pin"
    );

    let slice_unwired = wave_ok().replacen(
        &format!("pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {{\n    {VERIFY_LOOP}"),
        "pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {\n    // loop removed",
        1,
    );
    assert!(
        fails(SEED_OK, SEEDS, &slice_unwired)
            .iter()
            .any(|h| h.contains("gate_slice") && h.contains("VERIFY_STEPS")),
        "unwired slice path must RED"
    );

    let empty = fails(SEED_OK, SEEDS, "");
    assert!(
        empty
            .iter()
            .any(|h| h.contains("gate.rs missing `gate_slice()`")),
        "{empty:?}"
    );
}

#[test]
fn sql_stripper_keeps_literals_and_kills_comments() {
    assert_eq!(strip_sql_comments("a -- b\nc"), "a \nc");
    assert_eq!(strip_sql_comments("'-- kept'"), "'-- kept'");
    assert_eq!(strip_sql_comments("a /* x\ny */ b"), "a \n b");
    assert_eq!(strip_sql_comments("'it''s -- fine'"), "'it''s -- fine'");
}

#[test]
fn hash_stripper_respects_quotes() {
    assert_eq!(strip_hash_comments("a # b\nc"), "a \nc");
    assert_eq!(strip_hash_comments("\"a # b\""), "\"a # b\"");
    assert_eq!(strip_hash_comments("\t# only"), "\t");
    assert_eq!(
        strip_hash_comments("    (\"t440\"), // gone\n"),
        "    (\"t440\"), \n"
    );
    assert_eq!(strip_hash_comments("\"https://x\""), "\"https://x\"");
}

/// The evidence dump is byte-compared against the script, so `repr` must match CPython.
#[test]
fn py_repr_matches_cpython() {
    assert_eq!(
        py_repr("\tcd $(WEB) < seeds/x.sql"),
        "'\\tcd $(WEB) < seeds/x.sql'"
    );
    assert_eq!(py_repr("a \"b\" c"), "'a \"b\" c'");
    assert_eq!(py_repr("it's"), "\"it's\"");
    assert_eq!(py_repr("it's \"q\""), "'it\\'s \"q\"'");
    assert_eq!(py_repr("a\\b"), "'a\\\\b'");
    assert_eq!(py_repr("\x07"), "'\\x07'");
}

#[test]
fn fn_body_extraction_is_brace_balanced() {
    let src = "pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {\n  if x { y; }\n}\npub fn cmd_gate(ctx: &Ctx, base_arg: &str) -> u8 {\n  z\n}\n";
    let body = extract_fn_body(src, "gate_slice").unwrap().unwrap();
    assert!(body.starts_with('{') && body.ends_with('}'));
    assert!(!body.contains('z'), "must not run into the next function");
    assert!(extract_fn_body(src, "absent").unwrap().is_none());
    assert!(
        extract_fn_body(
            "pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {\n",
            "gate_slice"
        )
        .unwrap()
        .is_none()
    );
}

/// The RED-arm setup guards: a reflowed tree must abort the proof, not skip it.
#[test]
fn red_setup_refuses_a_wave_it_does_not_recognise() {
    assert!(delete_first_wave_run("nothing here").is_none());
    // Row already gone: delete_first_wave_run must refuse rather than perturb nothing.
    let single = wave_ok().replacen(WAVE_RUN_LINE, "", 1);
    assert!(delete_first_wave_run(&single).is_none());
    assert!(delete_first_wave_run(&wave_ok()).is_some());
}

struct SourceFixture(std::path::PathBuf);

impl SourceFixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let suffix = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tbd-faction-library-sources-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for SourceFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Copies only the source inputs inspected by this verifier into an isolated repository tree.
fn copied_live_inputs() -> SourceFixture {
    use crate::verifications::architecture::wave_gate_sources::WAVE_CHILDREN;

    let root = crate::core::repository_root::test_repo_root();
    let fixture = SourceFixture::new();
    let mut paths = vec![std::path::PathBuf::from(SEED_REL), WAVE_REL.into()];
    paths.extend(WAVE_CHILDREN.map(|(module_name, _)| {
        std::path::Path::new(WAVE_REL)
            .with_extension("")
            .join(format!("{module_name}.rs"))
    }));
    for relative in paths {
        let destination = fixture.path().join(&relative);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(root.join(relative), destination).unwrap();
    }
    fixture
}

#[test]
fn live_entrypoint_reads_both_linked_wave_implementations() {
    let fixture = copied_live_inputs();
    assert_eq!(verify_t440(fixture.path()).unwrap(), 0);
}

#[test]
fn missing_wave_implementation_fails_closed() {
    use crate::verifications::architecture::wave_gate_sources::WAVE_CHILDREN;

    for (module_name, _) in WAVE_CHILDREN {
        let fixture = copied_live_inputs();
        let facade = fixture.path().join(WAVE_REL);
        let child = facade.with_extension("").join(format!("{module_name}.rs"));
        std::fs::remove_file(&child).unwrap();
        assert!(matches!(
            read_pair(&fixture.path().join(SEED_REL), &facade),
            Err(Verdict::DidNotRun(_, _))
        ));
        assert_eq!(verify_t440(fixture.path()).unwrap(), 1);
    }
}

#[test]
fn disconnected_wave_implementation_fails_closed() {
    use crate::verifications::architecture::wave_gate_sources::WAVE_CHILDREN;

    for (module_name, function_name) in WAVE_CHILDREN {
        for declaration in [
            format!("mod {module_name};"),
            format!("pub use {module_name}::{function_name};"),
        ] {
            let fixture = copied_live_inputs();
            let facade = fixture.path().join(WAVE_REL);
            let original = std::fs::read_to_string(&facade).unwrap();
            assert!(original.contains(&declaration));
            std::fs::write(&facade, original.replacen(&declaration, "", 1)).unwrap();
            assert!(matches!(
                read_pair(&fixture.path().join(SEED_REL), &facade),
                Err(Verdict::Failed(_))
            ));
            assert_eq!(verify_t440(fixture.path()).unwrap(), 1);
        }
    }
}

#[test]
fn hollowed_wave_implementation_cannot_borrow_the_other_paths_loop() {
    use crate::verifications::architecture::wave_gate_sources::WAVE_CHILDREN;

    for (module_name, function_name) in WAVE_CHILDREN {
        let fixture = copied_live_inputs();
        let facade = fixture.path().join(WAVE_REL);
        let child = facade.with_extension("").join(format!("{module_name}.rs"));
        std::fs::write(&child, format!("pub fn {function_name}() -> u8 {{ 0 }}\n")).unwrap();
        let (seed, wave) = read_pair(&fixture.path().join(SEED_REL), &facade).unwrap();
        let failures = fails(&seed, SEEDS, &wave);
        assert!(failures.iter().any(|failure| {
            failure.contains(function_name) && failure.contains("does not iterate VERIFY_STEPS")
        }));
        assert_eq!(verify_t440(fixture.path()).unwrap(), 1);
    }
}

#[test]
fn linked_wave_inputs_preserve_universal_newline_reading() {
    use crate::verifications::architecture::wave_gate_sources::WAVE_CHILDREN;

    let fixture = copied_live_inputs();
    let seed = fixture.path().join(SEED_REL);
    let facade = fixture.path().join(WAVE_REL);
    let expected = read_pair(&seed, &facade).unwrap();
    let mut paths = vec![seed.clone(), facade.clone()];
    paths.extend(
        WAVE_CHILDREN
            .map(|(module_name, _)| facade.with_extension("").join(format!("{module_name}.rs"))),
    );
    for path in paths {
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, text.replace('\n', "\r\n")).unwrap();
    }
    assert_eq!(read_pair(&seed, &facade).unwrap(), expected);
    assert_eq!(verify_t440(fixture.path()).unwrap(), 0);
}
