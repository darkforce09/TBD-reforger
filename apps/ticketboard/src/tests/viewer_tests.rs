use super::*;
use crate::board;
use crate::testutil::{Scratch, work};
use std::fs;

// ---- .md click predicate ----

/// The spec/plan/citation click split: `.md` (any ASCII case) opens the
/// in-app viewer; everything else keeps the external/plain behavior.
#[test]
fn wants_viewer_predicate() {
    assert!(wants_viewer("docs/platform/t917_ticket_schema_v2.md"));
    assert!(wants_viewer("docs/plans/t-918_4_plan.md"));
    assert!(wants_viewer("README.md"));
    assert!(wants_viewer("docs/UPPER.MD"), "ASCII case-insensitive");
    assert!(!wants_viewer("apps/ticketboard/src/app.rs"));
    assert!(!wants_viewer("docs/notes.md.bak"), "must END in .md");
    assert!(!wants_viewer("md"), "no extension at all");
    assert!(!wants_viewer("README"));
    assert!(!wants_viewer(""));
    assert!(!wants_viewer(".ai/tickets/T-918.4.toml"));
}

/// The predicate over REAL ticket fields — spec, plan and citations as the
/// detail panel sees them through `board::view`.
#[test]
fn md_click_predicate_over_spec_plan_citations() {
    let t = work(
        "T-1",
        "status = \"idea\"",
        "spec = \"docs/platform/spec.md\"\nplan = \"docs/plans/t-1_plan.md\"\n\
             citations = [\"docs/a.md\", \"apps/x/src/main.rs\", \"docs/b.MD\"]\n",
    );
    let v = board::view(&t);
    assert!(wants_viewer(v.spec.unwrap()), "spec click → viewer");
    assert!(wants_viewer(v.plan.unwrap()), "plan click → viewer");
    let flags: Vec<bool> = v.citations.iter().map(|c| wants_viewer(c)).collect();
    assert_eq!(
        flags,
        vec![true, false, true],
        "only .md citations go to the viewer; the .rs keeps current behavior"
    );
}

// ---- state machine ----

#[test]
fn state_machine_transitions() {
    let mut s = ViewerState::Closed;
    assert!(!s.is_open());
    assert_eq!(s.path(), None);

    // Closed → open → Loading.
    s.open("docs/a.md");
    assert_eq!(
        s,
        ViewerState::Loading {
            path: "docs/a.md".into()
        }
    );
    assert!(s.is_open());
    assert_eq!(s.path(), Some("docs/a.md"));

    // Loading + matching Rendered result → Rendered.
    s.land(LoadedDoc {
        rel: "docs/a.md".into(),
        outcome: DocOutcome::Rendered {
            text: "# hi".into(),
        },
    });
    assert_eq!(
        s,
        ViewerState::Rendered {
            path: "docs/a.md".into(),
            text: "# hi".into()
        }
    );

    // Open replaces a rendered doc (clicking another path).
    s.open("docs/b.md");
    assert_eq!(s.path(), Some("docs/b.md"));

    // Loading + matching Fallback result → Fallback.
    s.land(LoadedDoc {
        rel: "docs/b.md".into(),
        outcome: DocOutcome::Fallback {
            text: String::new(),
            note: "cannot read".into(),
        },
    });
    assert!(matches!(&s, ViewerState::Fallback { note, .. } if note == "cannot read"));

    // Close from any state → Closed.
    s.close();
    assert_eq!(s, ViewerState::Closed);
}

/// Stale results never render: a mismatched path is dropped while Loading,
/// and ANY result is dropped once the machine left Loading (Back mid-read,
/// or a second click superseding the first).
#[test]
fn stale_results_are_dropped() {
    let rendered = |rel: &str| LoadedDoc {
        rel: rel.into(),
        outcome: DocOutcome::Rendered {
            text: "stale".into(),
        },
    };
    // Superseded click: loading b, the old a-read lands — ignored.
    let mut s = ViewerState::Closed;
    s.open("docs/a.md");
    s.open("docs/b.md");
    s.land(rendered("docs/a.md"));
    assert_eq!(
        s,
        ViewerState::Loading {
            path: "docs/b.md".into()
        },
        "mismatched path must not land"
    );

    // Back mid-read: closed, then the read lands — ignored.
    s.close();
    s.land(rendered("docs/b.md"));
    assert_eq!(s, ViewerState::Closed);

    // Already-rendered: a duplicate late result is ignored too.
    let mut s = ViewerState::Rendered {
        path: "docs/a.md".into(),
        text: "kept".into(),
    };
    s.land(rendered("docs/a.md"));
    assert!(matches!(&s, ViewerState::Rendered { text, .. } if text == "kept"));
}

// ---- path resolution: the escape fence ----

#[test]
fn resolve_happy_paths() {
    let root = Path::new("/repo");
    assert_eq!(
        resolve_repo_rel(root, "docs/a.md").unwrap(),
        PathBuf::from("/repo/docs/a.md")
    );
    // `.` drops; interior `..` pops within depth.
    assert_eq!(
        resolve_repo_rel(root, "./docs/./a.md").unwrap(),
        PathBuf::from("/repo/docs/a.md")
    );
    assert_eq!(
        resolve_repo_rel(root, "docs/sub/../a.md").unwrap(),
        PathBuf::from("/repo/docs/a.md")
    );
}

/// The red half of the guard: escapes are refused LEXICALLY — no file needs
/// to exist, nothing is read.
#[test]
fn resolve_refuses_escapes() {
    let root = Path::new("/repo");
    for rel in [
        "../outside.md",
        "docs/../../outside.md",
        "docs/../../../etc/passwd",
        "..",
    ] {
        let err = resolve_repo_rel(root, rel).unwrap_err();
        assert!(err.contains("escapes the repo root"), "{rel}: {err}");
        assert!(err.contains("never read"), "{rel}: {err}");
    }
    for rel in ["/etc/passwd", "/repo/docs/a.md"] {
        let err = resolve_repo_rel(root, rel).unwrap_err();
        assert!(err.contains("absolute"), "{rel}: {err}");
    }
    // The root itself is not a document.
    for rel in ["", ".", "docs/.."] {
        assert!(resolve_repo_rel(root, rel).is_err(), "{rel} must refuse");
    }
}

// ---- classify: size cap + UTF-8 ----

#[test]
fn classify_within_cap_renders() {
    let text = "# plan\n\n- step".to_owned();
    assert_eq!(
        classify(text.clone().into_bytes(), text.len() as u64, 1024),
        DocOutcome::Rendered { text }
    );
    // Exactly AT the cap still renders — the cap is a bound, not a slack.
    let at_cap = "x".repeat(16);
    assert_eq!(
        classify(at_cap.clone().into_bytes(), 16, 16),
        DocOutcome::Rendered { text: at_cap }
    );
}

#[test]
fn classify_oversize_truncates_with_notice() {
    // 17 bytes against a 16-byte cap (read_capped hands classify cap+1).
    let bytes = "abcdefghijklmnopq".as_bytes().to_vec();
    let DocOutcome::Fallback { text, note } = classify(bytes, 40, 16) else {
        panic!("oversize must fall back");
    };
    assert!(text.starts_with("abcdefghijklmnop"), "{text}");
    assert!(!text.contains('q'), "nothing beyond the cap: {text}");
    assert!(text.contains("[truncated — showing the first 16 bytes of 40 bytes"));
    assert!(text.contains("open externally"), "{text}");
    assert!(
        note.contains("exceeds the 16 bytes viewer cap (40 bytes)"),
        "{note}"
    );
    // The real cap formats as KB.
    assert!(oversize_note(600_000, SIZE_CAP_BYTES).contains("512 KB"));
}

/// A multi-byte char split by the cap is dropped whole — our own knife
/// never manufactures a replacement char or a lossy note.
#[test]
fn classify_oversize_cut_is_char_safe() {
    // "aé…" with the cap landing inside é (2 bytes, starts at index 1).
    let bytes = "aééé".as_bytes().to_vec(); // 7 bytes
    let DocOutcome::Fallback { text, note } = classify(bytes, 7, 2) else {
        panic!("oversize must fall back");
    };
    assert!(text.starts_with("a\n\n[truncated"), "{text}");
    assert!(!text.contains('\u{FFFD}'), "no replacement char: {text}");
    assert!(!note.contains("UTF-8"), "clean cut is not a UTF-8 problem");
}

#[test]
fn classify_non_utf8_goes_lossy() {
    let DocOutcome::Fallback { text, note } = classify(vec![0x66, 0xFF, 0x66], 3, 1024) else {
        panic!("non-UTF8 must fall back");
    };
    assert_eq!(note, NOTE_NON_UTF8);
    assert!(text.contains('\u{FFFD}'), "lossy marker expected: {text}");
    // Oversize AND binary: both facts in the note.
    let mut big = vec![0xFF; 8];
    big.extend_from_slice(b"tail");
    let DocOutcome::Fallback { note, .. } = classify(big, 12, 4) else {
        panic!("must fall back");
    };
    assert!(note.contains("viewer cap"), "{note}");
    assert!(note.contains("UTF-8"), "{note}");
}

/// A file that itself ends mid-character (corrupt, NOT our cut) is honest
/// about it: lossy fallback, never silently trimmed into a clean render.
#[test]
fn classify_within_cap_tail_corruption_is_lossy() {
    let mut bytes = b"ok ".to_vec();
    bytes.push(0xC3); // first byte of a 2-byte char, missing its tail
    let DocOutcome::Fallback { note, .. } = classify(bytes, 4, 1024) else {
        panic!("corrupt tail must fall back");
    };
    assert_eq!(note, NOTE_NON_UTF8);
}

// ---- load_doc: the fence + read against a real filesystem ----

#[test]
fn load_doc_reads_markdown_inside_root() {
    let s = Scratch::new("viewer-happy");
    fs::create_dir_all(s.path().join("docs")).unwrap();
    fs::write(s.path().join("docs/plan.md"), "# T-1 plan\n\nbody\n").unwrap();
    assert_eq!(
        load_doc(s.path(), "docs/plan.md"),
        DocOutcome::Rendered {
            text: "# T-1 plan\n\nbody\n".into()
        }
    );
}

#[test]
fn load_doc_missing_file_names_the_error() {
    let s = Scratch::new("viewer-missing");
    let DocOutcome::Fallback { text, note } = load_doc(s.path(), "docs/nope.md") else {
        panic!("missing file must fall back");
    };
    assert!(text.is_empty());
    assert!(note.contains("cannot read docs/nope.md"), "{note}");
}

/// The escape fence end-to-end: a `..` citation refuses WITHOUT reading —
/// the outside file's content never appears, even though it exists.
#[test]
fn load_doc_refuses_escape_without_reading() {
    let s = Scratch::new("viewer-escape");
    let root = s.path().join("repo");
    fs::create_dir_all(&root).unwrap();
    fs::write(s.path().join("secret.md"), "OUTSIDE-CONTENT").unwrap();
    let DocOutcome::Fallback { text, note } = load_doc(&root, "../secret.md") else {
        panic!("escape must fall back");
    };
    assert!(text.is_empty(), "never read: {text}");
    assert!(note.contains("escapes the repo root"), "{note}");
    let DocOutcome::Fallback { text, note } =
        load_doc(&root, s.path().join("secret.md").to_str().unwrap())
    else {
        panic!("absolute path must fall back");
    };
    assert!(text.is_empty(), "never read: {text}");
    assert!(note.contains("absolute"), "{note}");
}

/// The symlink half of the fence: a link inside the repo pointing outside
/// refuses after canonicalization — lexically clean, physically escaping.
#[cfg(unix)]
#[test]
fn load_doc_refuses_symlink_escape() {
    let s = Scratch::new("viewer-symlink");
    let root = s.path().join("repo");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(s.path().join("outside.md"), "OUTSIDE-CONTENT").unwrap();
    std::os::unix::fs::symlink(s.path().join("outside.md"), root.join("docs/link.md")).unwrap();
    let DocOutcome::Fallback { text, note } = load_doc(&root, "docs/link.md") else {
        panic!("symlink escape must fall back");
    };
    assert!(text.is_empty(), "never read: {text}");
    assert!(note.contains("outside the repo root"), "{note}");
}

#[test]
fn load_doc_capped_truncates_on_disk_file() {
    let s = Scratch::new("viewer-cap");
    fs::write(s.path().join("big.md"), "0123456789").unwrap();
    let DocOutcome::Fallback { text, note } = load_doc_capped(s.path(), "big.md", 4) else {
        panic!("oversize must fall back");
    };
    assert!(text.starts_with("0123\n\n[truncated"), "{text}");
    assert!(note.contains("(10 bytes)"), "{note}");
    // The same file under the real cap renders.
    assert!(matches!(
        load_doc(s.path(), "big.md"),
        DocOutcome::Rendered { .. }
    ));
}
