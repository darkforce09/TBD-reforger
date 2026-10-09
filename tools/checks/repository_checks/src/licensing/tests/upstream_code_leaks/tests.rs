use super::*;

const A: &str = "{AAAAAAAAAAAAAAAA}";
const B: &str = "{BBBBBBBBBBBBBBBB}";
const C: &str = "{0123456789ABCDEF}";

/// A throwaway four-lane tree (`mod`/`crf`/`ps`/`vanilla`). Never inside the repo: a fixture
/// under `mod/` would be scanned by the gate it is testing.
struct Fixture(PathBuf);

impl Fixture {
    /// Both oracle lanes present, each with an empty `UI/` folder unless `files` fills one.
    fn new(name: &str, files: &[(&str, &str)]) -> Fixture {
        let fx = Fixture::bare(name, files);
        for lane in ["crf/UI", "ps/UI"] {
            std::fs::create_dir_all(fx.0.join(lane)).unwrap();
        }
        fx
    }
    /// Only the trees `files` names, beside empty `mod` and `export` folders.
    fn bare(name: &str, files: &[(&str, &str)]) -> Fixture {
        let root = std::env::temp_dir().join(format!("tbd-crf-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("mod")).unwrap();
        std::fs::create_dir_all(root.join("export")).unwrap();
        for (rel, body) in files {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, body).unwrap();
        }
        Fixture(root)
    }
    fn run(&self) -> (u8, String) {
        gate(&self.0)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Exit code plus the exact transcript, for a four-lane root.
fn gate(root: &Path) -> (u8, String) {
    let at = |s: &str| root.join(s);
    let lanes = Lanes {
        mod_dir: at("mod"),
        export_dir: at("export"),
        crf: at("crf"),
        ps: at("ps"),
        vanilla: at("vanilla"),
    };
    let mut log = Log {
        lines: Vec::new(),
        echo: false,
    };
    (run(&lanes, &mut log), log.lines.join("\n"))
}

// Every assertion here is "the gate printed exactly this", so the transcript is the only
// useful failure message. Two helpers, so a one-line check stays a one-line check.
fn has(out: &str, want: &str) {
    assert!(out.contains(want), "MISSING {want:?} in:\n{out}");
}
fn hasnt(out: &str, bad: &str) {
    assert!(!out.contains(bad), "UNEXPECTED {bad:?} in:\n{out}");
}

/// THE FAIL-CLOSED CONTRACT. A lane that is absent, or present with nothing to compare, is
/// exit 2 naming the lane and the README that says how to fill it; no arm runs and nothing
/// passes. A comparison that ran and found nothing is the only no-finding PASS.
#[test]
fn a_lane_the_gate_cannot_compare_against_is_did_not_run() {
    let fill = format!("as {REFERENCES_DIR}/README.md describes");

    // Both lanes absent: refused on the first, before any arm prints.
    let fx = Fixture::bare("absent", &[("mod/Data/r.json", C)]);
    let (code, out) = fx.run();
    assert_eq!(code, 2, "{out}");
    let missing = "FAIL: no-oracle-leak could not examine the CRF lane — target file missing:";
    has(&out, &format!("{missing} {}", fx.0.join("crf").display()));
    has(&out, &fill);
    for never in ["SKIP", "OK (", "PASS", "==>"] {
        hasnt(&out, never);
    }

    // Only the PlayableSelector lane absent.
    let files = [("mod/Data/r.json", C), ("crf/UI/M.layout", A)];
    let (ps_code, ps_out) = Fixture::bare("ps-absent", &files).run();
    assert_eq!(ps_code, 2, "{ps_out}");
    has(
        &ps_out,
        "could not examine the PlayableSelector lane — target file missing:",
    );
    has(&ps_out, &fill);

    // The lane EXISTS but holds no UI/ or Prefabs/: nothing would be compared, which is how the
    // `find -L` symlink bug hid itself.
    let files = [("mod/Data/r.json", C), ("crf/S/W.c", "// no assets\n")];
    let fx2 = Fixture::bare("nodirs", &files);
    std::fs::create_dir_all(fx2.0.join("ps/UI")).unwrap();
    let (code2, out2) = fx2.run();
    assert_eq!(code2, 2, "{out2}");
    let unreadable = "could not examine the CRF lane — unreadable target:";
    has(
        &out2,
        &format!("{unreadable} {}", fx2.0.join("crf").display()),
    );
    has(&out2, "the lane holds no UI/ or Prefabs/ folder");
    hasnt(&out2, "PASS");

    // Both lanes compare and the comparison finds nothing: the run passes.
    let files = [("mod/S/T.c", "class T {}\n"), ("crf/UI/M.layout", A)];
    let (code3, out3) = Fixture::new("empty", &files).run();
    assert_eq!(code3, 0, "{out3}");
    has(&out3, "  OK (nothing to compare)");
    has(&out3, "no-oracle-leak: PASS (CRF + PlayableSelector)");
}

/// A real leak, and the two ways the gate refuses to call one.
#[test]
fn identifier_leaks_are_caught_but_comments_and_longer_words_are_not() {
    // Lines 1-4 are citations — the practice the gate exists to encourage. Line 5 is the leak.
    // Maps.c holds the four longer words a bare `grep PS_` would false-hit.
    let leaky = "//! CRF_PlayerCharacter.DisableAI port: mirrored, not copied\n\
                 /* CRF_Whatever in a block comment */\n\
                 * CRF_Whatever in a doc continuation\n\
                 # CRF_Whatever in a hash comment\n\
                 class TBD_SpawnManager { void go() { CRF_Registry.Get(); } }\n";
    let words = "int MAPS_C, GROUPS_M, OPS_T, TIPS_S;\n";
    let files = [("mod/S/Spawn.c", leaky), ("mod/S/Maps.c", words)];
    let (code, out) = Fixture::new("ident", &files).run();
    assert_eq!(code, 1, "{out}");
    has(
        &out,
        "FAIL: CRF_ symbols found in our mod trees (tbd-framework, tbd-export):",
    );
    has(&out, "Spawn.c:5:class TBD_SpawnManager");
    for commented in [":1:", ":2:", ":3:", ":4:"] {
        hasnt(&out, commented); // a comment naming the oracle is allowed
    }
    // PS_ must not hit MAPS_/GROUPS_/OPS_/TIPS_.
    let ps_arm =
        "==> PS_ identifiers in tbd-framework + tbd-export code (PlayableSelector, NO LICENCE)";
    has(&out, &format!("{ps_arm}\n  OK (none)"));
    has(&out, "Oracles are reference-only.");
    has(&out, EPILOGUE_PS);
}
/// The engine-fact filter: a shared GUID inside a `.pak` is vanilla and exempt, one that is not
/// is the leak, printed with the `path:line` of every reference in our addons.
#[test]
fn vanilla_paks_exempt_a_shared_guid_and_only_a_shared_guid() {
    // The "pak" is searched as bytes, so a bare hex run is all that matters.
    let both = "{AAAAAAAAAAAAAAAA}\n{BBBBBBBBBBBBBBBB} x {BBBBBBBBBBBBBBBB}\n";
    let files = [
        ("mod/Data/r.json", both),
        ("export/P/w.et", B),
        ("crf/UI/M.layout", both),
        ("vanilla/data001.pak", "\u{0}junk AAAAAAAAAAAAAAAA more\n"),
    ];
    let fx = Fixture::new("vanilla", &files);
    let (code, out) = fx.run();
    assert_eq!(code, 1, "{out}");
    let head = "FAIL: CRF-only asset GUIDs reused (not present in vanilla):";
    let export_ref = fx.0.join("export/P/w.et");
    let mod_ref = fx.0.join("mod/Data/r.json");
    let listed = format!(
        "{head}\n  {B}\n    {}:1\n    {}:2\n",
        export_ref.display(),
        mod_ref.display()
    );
    has(&out, &listed);
    hasnt(&out, &format!("  {A}")); // in vanilla -> an engine fact, not a leak

    // No game install: the gate cannot tell an engine fact from a leak, so EVERY shared GUID is
    // reported rather than exempted.
    let bare = [("mod/Data/r.json", A), ("crf/Prefabs/P.et", A)];
    let (bare_code, bare_out) = Fixture::new("nosteam", &bare).run();
    assert_eq!(bare_code, 1, "{bare_out}");
    has(&bare_out, &format!("  {A}"));

    // A pak that cannot be read is did-not-run naming it, never "not in vanilla".
    let fx3 = Fixture::new("badpak", &[("mod/Data/r.json", A), ("crf/UI/M.layout", A)]);
    std::fs::create_dir_all(fx3.0.join("vanilla/data001.pak")).unwrap();
    let (code3, out3) = fx3.run();
    assert_eq!(code3, 2, "{out3}");
    has(
        &out3,
        &fx3.0.join("vanilla/data001.pak").display().to_string(),
    );
    hasnt(&out3, "PASS");
}
