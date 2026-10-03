use super::*;

const A: &str = "{AAAAAAAAAAAAAAAA}";
const B: &str = "{BBBBBBBBBBBBBBBB}";
const C: &str = "{0123456789ABCDEF}";

/// A throwaway four-lane tree (`mod`/`crf`/`ps`/`vanilla`). Never inside the repo: a fixture
/// under `apps/mod/` would be scanned by the gate it is testing.
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

/// A `@`-prefixed Workshop mod name is how a mission names a dependency, not a lane symbol: it is
/// no finding in a trailing doc comment or in a string, while a real `CRF_` symbol still is.
#[test]
fn a_workshop_mod_name_is_not_an_identifier_leak() {
    let names = "class TBD_MissionMod\n{\n\tstring m_sName;          //!< \"@CRF_Framework\"\n}\n\
                 mods.Insert(new TBD_MissionMod(\"@CRF_Framework\", \"v2.4.0\"));\n\
                 mods.Insert(new TBD_MissionMod(\"@PS_Lobby\", \"v1.0.0\"));\n";
    let (code, out) = Fixture::new("modname", &[("mod/S/Mock.c", names)]).run();
    assert_eq!(code, 0, "{out}");
    assert_eq!(out.matches("  OK (none)").count(), 2, "{out}");

    let symbol = "string m_sName; //!< \"@CRF_Framework\"\nCRF_Foo foo = new CRF_Foo();\n";
    let (code2, out2) = Fixture::new("modname-symbol", &[("mod/S/Leak.c", symbol)]).run();
    assert_eq!(code2, 1, "{out2}");
    has(&out2, "Leak.c:2:CRF_Foo foo = new CRF_Foo();");
    hasnt(&out2, "Leak.c:1:");
}

/// Hit-list truncation, and the `EnfusionMCP` exemption applying to the identifier step only.
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

/// One pass over the paks answers every GUID exactly as one `grep -qla <guid> *.pak` per GUID
/// does: GUIDs inside longer hex runs, overlapping each other, beside NUL and non-UTF-8 bytes,
/// split over two paks, and absent.
#[test]
fn one_vanilla_pass_answers_every_guid_as_a_grep_per_guid_does() {
    // A deterministic byte soup over a hex-heavy alphabet, so 16-digit runs occur and overlap.
    let alphabet = b"0123456789ABCDEF\0\n\xC3\xFFx";
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut soup = |len: usize| -> Vec<u8> {
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            alphabet[(state % alphabet.len() as u64) as usize]
        };
        (0..len).map(|_| next()).collect()
    };
    let overlapping = b"--0123456789ABCDEF0123--";
    let mut first = soup(3000);
    first.extend_from_slice(overlapping);
    let second = soup(3000);

    let fx = Fixture::bare("vanilla-equivalence", &[]);
    let dir = fx.0.join("vanilla");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("data001.pak"), &first).unwrap();
    std::fs::write(dir.join("data002.pak"), &second).unwrap();

    let hex = |w: &[u8]| {
        w.iter()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_lowercase())
    };
    let mut wanted: BTreeSet<String> = BTreeSet::new();
    for pak in [&first, &second] {
        for w in pak.windows(16).filter(|w| hex(w)).step_by(3).take(40) {
            wanted.insert(String::from_utf8(w.to_vec()).unwrap());
        }
    }
    // Overlapping pair inside `overlapping`, plus GUIDs that occur nowhere.
    wanted.insert("0123456789ABCDEF".into());
    wanted.insert("456789ABCDEF0123".into());
    wanted.insert("FFFFFFFFFFFFFFF0".into());
    wanted.insert("0000000000000001".into());

    let paks = vanilla_pak_probe::paks(&dir);
    let mut by_grep: BTreeSet<String> = BTreeSet::new();
    for g in &wanted {
        let probe = process_runner::Run::new("grep")
            .arg("-qla")
            .arg(g)
            .args(&paks);
        if probe.status().expect("grep runs") == 0 {
            by_grep.insert(g.clone());
        }
    }
    assert!(
        by_grep.len() > 4 && by_grep.len() < wanted.len(),
        "{by_grep:?}"
    );

    assert!(
        by_grep.contains("456789ABCDEF0123"),
        "the overlapping GUID is in the fixture"
    );
    assert_eq!(present_in_paks(&dir, &wanted).unwrap(), by_grep);
}

/// In a slice worktree each lane is a symlink; the asset-folder walk follows it, or the lane would
/// hold nothing to compare.
#[test]
fn asset_dirs_descend_through_symlinked_lanes() {
    // CRF nests UI at depth 1, PlayableSelector at depth 2, so the walk goes two levels deep.
    let files = [
        ("real/PlayableSelector/UI/M.layout", A),
        ("real/PlayableSelector/Prefabs/P.et", B),
        ("crf/UI/A.layout", C),
    ];
    let fx = Fixture::bare("symlink", &files);
    std::os::unix::fs::symlink(fx.0.join("real"), fx.0.join("ps")).unwrap();
    let dirs = asset_dirs(&fx.0.join("ps")).unwrap();
    assert_eq!(dirs.len(), 2, "maxdepth 2 through the link: {dirs:?}");
    assert_eq!(asset_dirs(&fx.0.join("crf")).unwrap().len(), 1);
}

/// An absent addon tree is did-not-run (exit 2), never `OK (none)`: a tree nobody read is not a
/// clean tree.
#[test]
fn an_absent_mod_tree_does_not_read_as_clean() {
    let fx = Fixture::new("nomod", &[]);
    std::fs::remove_dir_all(fx.0.join("mod")).unwrap();
    let (code, out) = fx.run();
    assert_eq!(code, 2, "{out}");
    has(&out, "FAIL: no-oracle-leak could not examine the trees");
    hasnt(&out, "OK (none)");
}

/// The grep-compatible reading helpers, the identifier pattern and the lane resolution.
#[test]
fn reading_helpers_identifier_pattern_and_lane_resolution() {
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

    let ps = pattern(&identifier_pattern("PS_")).unwrap();
    for longer in [
        "MAPS_X",
        "GROUPS_X",
        "OPS_X",
        "TIPS_X",
        "xPS_y",
        "\"@PS_Lobby\"",
    ] {
        assert!(!ps.is_match(longer), "{longer} is not a PS_ identifier");
    }
    for real in ["PS_Thing", " PS_Thing", "\tPS_Thing", "(PS_Thing)"] {
        assert!(ps.is_match(real), "{real} is a PS_ identifier");
    }

    // Every lane resolves inside the references folder; a non-empty TBD_PS_ORACLE replaces the
    // PlayableSelector lane and an empty one does not.
    let (root, home) = (Path::new("/repo"), Path::new("/home/u"));
    let lanes = Lanes::resolve(root, home, None);
    assert_eq!(lanes.crf, root.join(CRF_FRAMEWORK_REFERENCE));
    assert_eq!(lanes.ps, root.join(PLAYABLE_SELECTOR_REFERENCE));
    assert!(lanes.crf.starts_with(root.join(REFERENCES_DIR)));
    assert!(lanes.ps.starts_with(root.join(REFERENCES_DIR)));
    assert_eq!(lanes.mod_dir, root.join(MOD_REL));
    assert_eq!(lanes.export_dir, root.join(EXPORT_REL));
    let empty = Lanes::resolve(root, home, Some("".into()));
    assert_eq!(empty.ps, root.join(PLAYABLE_SELECTOR_REFERENCE));
    let named = Lanes::resolve(root, home, Some("/elsewhere/PlayableSelector".into()));
    assert_eq!(named.ps, Path::new("/elsewhere/PlayableSelector"));
}
