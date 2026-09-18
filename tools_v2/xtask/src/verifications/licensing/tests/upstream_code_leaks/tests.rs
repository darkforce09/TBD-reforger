use super::*;

const A: &str = "{AAAAAAAAAAAAAAAA}";
const B: &str = "{BBBBBBBBBBBBBBBB}";
const C: &str = "{0123456789ABCDEF}";

/// A throwaway four-lane tree (`mod`/`crf`/`ps`/`vanilla`). Never inside the repo: a fixture
/// under `apps/mod/` would be scanned by the gate it is testing.
struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str, files: &[(&str, &str)]) -> Fixture {
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

/// Exit code plus the exact stdout the script would have produced, for a four-lane root.
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

/// THE SKIP CONTRACT. An advisory skip, a comparison that never happened, and a comparison
/// that ran and found nothing are three different facts; neither SKIP may soften to an "OK".
#[test]
fn the_three_no_finding_wordings_stay_distinct() {
    // Both oracles absent -> two advisory SKIPs, and the run is still a PASS.
    let (code, out) = Fixture::new("skip", &[("mod/Data/r.json", C)]).run();
    assert_eq!(code, 0, "{out}");
    assert_eq!(out.matches("  OK (none)").count(), 2, "{out}");
    has(&out, &format!("  SKIP — CRF {SKIP_TAIL}"));
    has(&out, &format!("  SKIP — PlayableSelector {SKIP_TAIL}"));
    hasnt(&out, "OK (shared GUIDs"); // never claim a comparison that did not happen
    hasnt(&out, "OK (nothing to compare)");
    has(&out, "no-oracle-leak: PASS (CRF + PlayableSelector)");

    // The lane EXISTS but holds no UI/ or Prefabs/. This is the wording that caught the
    // `find -L` symlink bug, so it must stay blunt rather than cheerful.
    let files = [("mod/Data/r.json", C), ("crf/S/W.c", "// no assets\n")];
    let fx2 = Fixture::new("nodirs", &files);
    let (_, out2) = fx2.run();
    let at = fx2.0.join("crf");
    let blunt = "  SKIP — no UI/ or Prefabs/ dirs under";
    let made = "NO GUID comparison was made";
    has(&out2, &format!("{blunt} {}; {made}", at.display()));
    hasnt(&out2, "OK (nothing to compare)");

    // The oracle has assets and we have none: the comparison ran and found nothing.
    let files = [("mod/S/T.c", "class T {}\n"), ("crf/UI/M.layout", A)];
    let (code3, out3) = Fixture::new("empty", &files).run();
    assert_eq!(code3, 0, "{out3}");
    has(&out3, "  OK (nothing to compare)");
}

/// A real leak, and the two ways the script refuses to call one.
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

/// `head -20` truncation, and `--exclude-dir=EnfusionMCP` applying to arm 1 only.
#[test]
fn the_hit_list_truncates_and_the_mcp_bridge_is_exempt_from_arm_one() {
    let body: String = (0..25).map(|i| format!("x = CRF_X{i}();\n")).collect();
    let (code, out) = Fixture::new("head", &[("mod/S/Many.c", &body)]).run();
    assert_eq!(code, 1);
    assert_eq!(out.matches("Many.c:").count(), HEAD, "{out}");

    let emcp = "mod/S/WorkbenchGame/EnfusionMCP/EMCP.c";
    let mcp = Fixture::new("mcp", &[(emcp, "CRF_Thing t;\n{0123456789ABCDEF}\n")]);
    let (mcp_code, mcp_out) = mcp.run();
    assert_eq!(mcp_code, 0, "{mcp_out}");
    assert_eq!(mcp_out.matches("  OK (none)").count(), 2, "{mcp_out}");
    let re = Regex::new(GUID_RE).unwrap();
    let ours = guids_under(&re, &[mcp.0.join("mod").as_path()]).unwrap();
    assert!(ours.contains(C), "the GUID scan has no --exclude-dir");
}

/// The engine-fact filter over a real `grep -qla` subprocess: a GUID inside a `.pak` is
/// vanilla and exempt, one that is not is the leak. This is the arm that ships red.
#[test]
fn vanilla_paks_exempt_a_shared_guid_and_only_a_shared_guid() {
    // The "pak" is read with `grep -a`, so a bare hex run is all that matters.
    let both = "{AAAAAAAAAAAAAAAA}\n{BBBBBBBBBBBBBBBB}\n";
    let files = [
        ("mod/Data/r.json", both),
        ("crf/UI/M.layout", both),
        ("vanilla/data001.pak", "\u{0}junk AAAAAAAAAAAAAAAA more\n"),
    ];
    let (code, out) = Fixture::new("vanilla", &files).run();
    assert_eq!(code, 1, "{out}");
    let head = "FAIL: CRF-only asset GUIDs reused (not present in vanilla):";
    has(&out, &format!("{head}\n  {B}"));
    hasnt(&out, &format!("  {A}")); // in vanilla -> an engine fact, not a leak

    // Module docs oddity 7: no Steam install, so the `&&` short-circuits and EVERY shared GUID
    // is reported. Pinned so the false-accusation behaviour cannot drift unnoticed.
    let bare = [("mod/Data/r.json", A), ("crf/Prefabs/P.et", A)];
    let (bare_code, bare_out) = Fixture::new("nosteam", &bare).run();
    assert_eq!(bare_code, 1, "{bare_out}");
    has(&bare_out, &format!("  {A}"));
}

/// `find -L` is load-bearing: in a slice worktree the oracle lane is a symlink, and a walker
/// that does not descend prints the blunt SKIP while comparing nothing.
#[test]
fn asset_dirs_descend_through_symlinked_lanes() {
    // CRF nests UI at depth 1, PlayableSelector at depth 2 — why the script uses `find`.
    let files = [
        ("real/PlayableSelector/UI/M.layout", A),
        ("real/PlayableSelector/Prefabs/P.et", B),
        ("crf/UI/A.layout", C),
    ];
    let fx = Fixture::new("symlink", &files);
    std::os::unix::fs::symlink(fx.0.join("real"), fx.0.join("ps")).unwrap();
    let dirs = asset_dirs(&fx.0.join("ps")).unwrap();
    assert_eq!(dirs.len(), 2, "maxdepth 2 through the link: {dirs:?}");
    assert_eq!(asset_dirs(&fx.0.join("crf")).unwrap().len(), 1);
}

/// THE DEFECT THE CRATE EXISTS FOR. bash reads an absent mod tree as `OK (none)` + `OK
/// (nothing to compare)` and exits 0; a tree nobody read is not a clean tree.
#[test]
fn an_absent_mod_tree_does_not_read_as_clean() {
    let (code, out) = gate(Path::new("/nonexistent/tbd-crf"));
    assert_eq!(code, 2, "{out}");
    has(&out, "FAIL: no-oracle-leak could not examine the trees");
    hasnt(&out, "OK (none)");
}

/// The grep-compatibility helpers and the lane resolution, measured against `/usr/bin/grep`.
#[test]
fn reading_and_lane_resolution_match_the_script() {
    assert_eq!(grep_visible(b"hi\0there"), b"", "NUL in the first buffer");
    let late = [b"a".repeat(GREP_BUF + 1), b"\0tail".to_vec()].concat();
    assert_eq!(grep_visible(&late).len(), GREP_BUF + 1, "a late NUL cuts");
    assert_eq!(grep_visible(b"plain\n"), b"plain\n");

    // grep keeps the CR; `str::lines` would eat it.
    let crlf = [(1, "a\r".to_string()), (2, "b\r".to_string())];
    assert_eq!(numbered(b"a\r\nb\r\n"), crlf);
    let partial = [(1, "one".to_string()), (2, "two".to_string())];
    assert_eq!(numbered(b"one\ntwo"), partial, "no trailing newline");
    assert!(numbered(b"").is_empty());

    // The comment filter is anchored on the RENDERED line, path included.
    let c = pattern(COMMENT_RE).unwrap();
    assert!(c.is_match("/a/b.c:5:  // CRF_X") && c.is_match("/a/b.c:5:\t* CRF_X"));
    assert!(c.is_match("/a/b.c:5:/* X") && c.is_match("/a/b.c:5:# X"));
    assert!(!c.is_match("/a/b.c:5:  CRF_X();"));
    assert!(!c.is_match("/a/o:d.c:5:  // X"), "`[^:]+` spans no colon");

    let ps = pattern("(^|[^A-Za-z0-9_])PS_").unwrap();
    for longer in ["MAPS_X", "GROUPS_X", "OPS_X", "TIPS_X", "xPS_y"] {
        assert!(!ps.is_match(longer), "{longer} is not a PS_ identifier");
    }
    for real in ["PS_Thing", " PS_Thing", "\tPS_Thing", "(PS_Thing)"] {
        assert!(ps.is_match(real), "{real} is a PS_ identifier");
    }

    // The in-repo lane wins even over TBD_PS_ORACLE — the override is unconditional.
    let fx = Fixture::new("lanes", &[]);
    std::fs::create_dir_all(fx.0.join(PS_REPO_REL)).unwrap();
    assert_eq!(Lanes::from_env(&fx.0).ps, fx.0.join(PS_REPO_REL));
    let bare = Fixture::new("lanes2", &[]);
    assert_ne!(Lanes::from_env(&bare.0).ps, bare.0.join(PS_REPO_REL));
    assert_eq!(Lanes::from_env(&bare.0).mod_dir, bare.0.join(MOD_REL));
    assert_eq!(Lanes::from_env(&bare.0).export_dir, bare.0.join(EXPORT_REL));
}
