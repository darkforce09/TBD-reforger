//! In-app markdown viewer model (T-918.4 / B.4) — pure state machine + worker
//! IO, no egui types.
//!
//! Spec, plan and `.md`-citation clicks render the document inside the board
//! instead of the external xdg-open hop ([`wants_viewer`] is the click
//! predicate; non-`.md` paths keep their old behavior). READ-ONLY by
//! construction: this module only ever reads, and only inside the repo root —
//! a citation string could contain anything, so resolution is lexically fenced
//! first ([`resolve_repo_rel`]: absolute paths refused, `..` never climbs above
//! the root) and then symlink-checked (the canonicalized target must stay under
//! the canonicalized root); an escaping path lands the Fallback note WITHOUT a
//! read. Reads run on a worker thread ([`spawn_read`], the corpus.rs pattern);
//! oversize files are cut at [`SIZE_CAP_BYTES`] with an explicit truncation
//! notice appended to the text; non-UTF8 falls back to lossy raw text.
//! Commonmark itself has no parse failure — every failure mode lives here, and
//! each one renders as Fallback (raw monospace text + a note naming why),
//! never a crash.

use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc;
use std::thread;

/// Viewer size cap (~512 KB): beyond this the file renders as truncated raw
/// text — a pathological document is never fed whole to the markdown renderer,
/// so it cannot freeze the paint path.
pub const SIZE_CAP_BYTES: usize = 512 * 1024;

/// Note attached to non-UTF8 fallbacks (shown verbatim in the pane header).
pub const NOTE_NON_UTF8: &str = "not valid UTF-8 — raw text shown lossily";

/// The `.md` click predicate: paths whose extension is literally `md` (ASCII
/// case-insensitive) open the in-app viewer; everything else keeps the
/// external-open behavior. Applied uniformly to spec, plan and citation paths.
pub fn wants_viewer(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

// ---- state machine ----

/// Viewer pane state. `path` is always the repo-relative string as clicked —
/// the display label and the stale-result identity in [`ViewerState::land`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerState {
    Closed,
    /// Worker read in flight.
    Loading {
        path: String,
    },
    /// Markdown rendering (egui_commonmark) of the read text.
    Rendered {
        path: String,
        text: String,
    },
    /// Raw monospace text plus the note naming why (read failure / non-UTF8 /
    /// oversize / escape) — the never-a-crash surface.
    Fallback {
        path: String,
        text: String,
        note: String,
    },
}

impl ViewerState {
    /// A spec/plan/citation click: enter `Loading` for `rel` (replacing
    /// whatever was open — clicking another doc restarts the machine).
    pub fn open(&mut self, rel: &str) {
        *self = ViewerState::Loading {
            path: rel.to_owned(),
        };
    }

    /// Back: the pane closes; board state (selection included) is not this
    /// machine's to touch.
    pub fn close(&mut self) {
        *self = ViewerState::Closed;
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, ViewerState::Closed)
    }

    /// The repo-relative path this pane is about (`None` when closed).
    pub fn path(&self) -> Option<&str> {
        match self {
            ViewerState::Closed => None,
            ViewerState::Loading { path }
            | ViewerState::Rendered { path, .. }
            | ViewerState::Fallback { path, .. } => Some(path),
        }
    }

    /// Land a worker result. Applies ONLY while `Loading` the same path — a
    /// stale read (superseded click, or Back pressed mid-read) is dropped on
    /// the floor, never rendered.
    pub fn land(&mut self, doc: LoadedDoc) {
        let ViewerState::Loading { path } = self else {
            return;
        };
        if *path != doc.rel {
            return;
        }
        let path = std::mem::take(path);
        *self = match doc.outcome {
            DocOutcome::Rendered { text } => ViewerState::Rendered { path, text },
            DocOutcome::Fallback { text, note } => ViewerState::Fallback { path, text, note },
        };
    }
}

/// What one read produced — the two terminal states of the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocOutcome {
    Rendered { text: String },
    Fallback { text: String, note: String },
}

/// Worker-thread result: the outcome tagged with the path it answers, so
/// [`ViewerState::land`] can drop stale reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDoc {
    pub rel: String,
    pub outcome: DocOutcome,
}

// ---- path resolution (the escape fence) ----

/// Lexical repo-root-relative resolution — refuses escapes BEFORE any IO, so
/// `../outside.md` is red even when nothing exists there. Absolute paths and
/// prefixes are refused outright (viewer paths are repo-root-relative by
/// contract); `.` segments drop; `..` pops within the accumulated depth and
/// refuses at depth zero. The symlink fence (a link inside the repo pointing
/// out) lives in [`load_doc`] — it needs the filesystem.
pub fn resolve_repo_rel(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(format!(
            "{rel} is absolute — viewer paths are repo-root-relative; refused, never read"
        ));
    }
    let mut kept: Vec<&std::ffi::OsStr> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::Normal(seg) => kept.push(seg),
            Component::CurDir => {}
            Component::ParentDir => {
                if kept.pop().is_none() {
                    return Err(format!("{rel} escapes the repo root — refused, never read"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "{rel} is absolute — viewer paths are repo-root-relative; refused, never read"
                ));
            }
        }
    }
    if kept.is_empty() {
        return Err(format!("{rel:?} names the repo root itself, not a file"));
    }
    let mut out = root.to_path_buf();
    out.extend(kept);
    Ok(out)
}

// ---- read + classification ----

/// Human size for the cap notes: whole KB when it divides evenly, raw bytes
/// otherwise (tests use tiny caps).
fn fmt_size(bytes: usize) -> String {
    if bytes >= 1024 && bytes.is_multiple_of(1024) {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{bytes} bytes")
    }
}

/// The notice appended to truncated text — the operator sees it at the end of
/// what IS shown, and "open externally" remains the whole-file path.
fn truncation_notice(total: u64, cap: usize) -> String {
    format!(
        "[truncated — showing the first {} of {total} bytes; \
         use \"open externally\" for the whole file]",
        fmt_size(cap)
    )
}

fn oversize_note(total: u64, cap: usize) -> String {
    format!(
        "file exceeds the {} viewer cap ({total} bytes) — raw text, truncated",
        fmt_size(cap)
    )
}

/// Pure classification of read bytes (unit-tested without a filesystem).
/// `bytes` is the capped read (at most `cap + 1` bytes — one past the cap
/// detects oversize); `total` is the on-disk size for the notices.
///
/// - within cap + valid UTF-8 → `Rendered`
/// - within cap + invalid UTF-8 → `Fallback` (lossy text, non-UTF8 note)
/// - over the cap → `Fallback`, text cut at `cap` with the truncation notice
///   appended. A multi-byte character split by the cut is dropped whole (never
///   a replacement char from our own knife); content that is invalid UTF-8
///   anyway goes lossy with the combined note.
pub fn classify(mut bytes: Vec<u8>, total: u64, cap: usize) -> DocOutcome {
    let oversize = bytes.len() > cap;
    if !oversize {
        return match String::from_utf8(bytes) {
            Ok(text) => DocOutcome::Rendered { text },
            Err(e) => DocOutcome::Fallback {
                text: String::from_utf8_lossy(e.as_bytes()).into_owned(),
                note: NOTE_NON_UTF8.to_owned(),
            },
        };
    }
    bytes.truncate(cap);
    let (text, lossy) = match String::from_utf8(bytes) {
        Ok(s) => (s, false),
        // error_len() == None: the ONLY error is an incomplete char at the very
        // end — our cut split it; drop the fragment, the prefix is clean.
        Err(e) if e.utf8_error().error_len().is_none() => {
            let valid = e.utf8_error().valid_up_to();
            let mut b = e.into_bytes();
            b.truncate(valid);
            let s = String::from_utf8(b).expect("prefix below valid_up_to is valid UTF-8");
            (s, false)
        }
        Err(e) => (String::from_utf8_lossy(e.as_bytes()).into_owned(), true),
    };
    let text = format!("{text}\n\n{}", truncation_notice(total, cap));
    let note = if lossy {
        format!("{}; {NOTE_NON_UTF8}", oversize_note(total, cap))
    } else {
        oversize_note(total, cap)
    };
    DocOutcome::Fallback { text, note }
}

/// Full load for one document: lexical resolve → symlink containment check →
/// capped read → [`classify`]. Every refusal and IO error becomes a Fallback
/// note; nothing here panics and nothing outside the root is ever opened.
pub fn load_doc(root: &Path, rel: &str) -> DocOutcome {
    load_doc_capped(root, rel, SIZE_CAP_BYTES)
}

fn load_doc_capped(root: &Path, rel: &str, cap: usize) -> DocOutcome {
    let refuse = |note: String| DocOutcome::Fallback {
        text: String::new(),
        note,
    };
    let abs = match resolve_repo_rel(root, rel) {
        Ok(abs) => abs,
        Err(note) => return refuse(note),
    };
    // Symlink fence: `..` was already refused lexically; this catches a link
    // INSIDE the repo whose target lives outside. Best-effort (canonicalize
    // then open is not atomic) — the fence is against pathological citation
    // strings and stray links, not a local adversary.
    let canon = match abs.canonicalize() {
        Ok(canon) => canon,
        Err(e) => return refuse(format!("cannot read {rel}: {e}")),
    };
    match root.canonicalize() {
        Ok(canon_root) if canon.starts_with(&canon_root) => {}
        Ok(_) => {
            return refuse(format!(
                "{rel} resolves outside the repo root (symlink) — refused, never read"
            ));
        }
        Err(e) => return refuse(format!("cannot resolve the repo root: {e}")),
    }
    match read_capped(&canon, cap) {
        Ok((bytes, total)) => classify(bytes, total, cap),
        Err(e) => refuse(format!("cannot read {rel}: {e}")),
    }
}

/// Read at most `cap + 1` bytes (one past the cap is the oversize signal) plus
/// the on-disk total for the notices — a multi-gigabyte file costs one cap's
/// worth of memory, never a full read.
fn read_capped(abs: &Path, cap: usize) -> std::io::Result<(Vec<u8>, u64)> {
    let file = File::open(abs)?;
    let total = file.metadata()?.len();
    let mut bytes = Vec::with_capacity(usize::try_from(total).unwrap_or(cap).min(cap + 1));
    file.take(cap as u64 + 1).read_to_end(&mut bytes)?;
    Ok((bytes, total))
}

/// Run [`load_doc`] on a worker thread — the UI thread never touches the disk
/// (the corpus.rs pattern). `on_done` fires after the result is sent (the app
/// passes `egui::Context::request_repaint`).
pub fn spawn_read(
    root: PathBuf,
    rel: String,
    on_done: impl FnOnce() + Send + 'static,
) -> mpsc::Receiver<LoadedDoc> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = load_doc(&root, &rel);
        let _ = tx.send(LoadedDoc { rel, outcome });
        on_done();
    });
    rx
}

#[cfg(test)]
mod tests {
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
}
