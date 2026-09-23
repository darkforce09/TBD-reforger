//! Repository-contained document reads with bounded size and explicit fallback text.
//!
//! Paths are checked lexically and after canonicalization before any read. Worker
//! threads tag results with the requested path; oversized or non-UTF8 documents
//! produce a named raw-text fallback without blocking the rendering thread.

pub use super::super::models::{DocumentOutcome, LoadedDocument, ViewerState};
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
pub fn classify(mut bytes: Vec<u8>, total: u64, cap: usize) -> DocumentOutcome {
    let oversize = bytes.len() > cap;
    if !oversize {
        return match String::from_utf8(bytes) {
            Ok(text) => DocumentOutcome::Rendered { text },
            Err(e) => DocumentOutcome::Fallback {
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
    DocumentOutcome::Fallback { text, note }
}

/// Full load for one document: lexical resolve → symlink containment check →
/// capped read → [`classify`]. Every refusal and IO error becomes a Fallback
/// note; nothing here panics and nothing outside the root is ever opened.
pub fn load_doc(root: &Path, rel: &str) -> DocumentOutcome {
    load_doc_capped(root, rel, SIZE_CAP_BYTES)
}

fn load_doc_capped(root: &Path, rel: &str, cap: usize) -> DocumentOutcome {
    let refuse = |note: String| DocumentOutcome::Fallback {
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

/// Read a bounded document on a worker thread. The completion callback wakes the
/// UI after the path-tagged result is sent through the channel.
pub fn spawn_read(
    root: PathBuf,
    rel: String,
    on_done: impl FnOnce() + Send + 'static,
) -> mpsc::Receiver<LoadedDocument> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = load_doc(&root, &rel);
        let _ = tx.send(LoadedDocument { rel, outcome });
        on_done();
    });
    rx
}

#[cfg(test)]
#[path = "tests/document_loading.rs"]
mod tests;
