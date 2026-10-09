# Document reading

The file side of the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s document column:
which clicked paths open in the viewer, the fence that keeps every read inside the repository, the
bounded read on a worker thread, and the sorting of the bytes read into Markdown or a named
raw-text fallback.

## Contents

```text
tools/tickets/ticketboard_model/src/document_viewer/services/
├── document_loading.rs  `wants_viewer`, `resolve_repo_rel`, `classify`, `load_doc` and `spawn_read`
└── mod.rs               the module tree
```

## How it works

`wants_viewer` sends a path to the viewer only when its extension is `md`, in any ASCII case;
every other path keeps the operating system's handler. `spawn_read(root, rel, on_done)` runs
`load_doc` on a new thread, sends the result tagged with `rel` as a `LoadedDocument`, then calls
`on_done`, which the application sets to a repaint request.

`load_doc` refuses before any read in two steps. `resolve_repo_rel` works on the text alone: an
absolute path, a prefix, a `..` that climbs above the root, or a path that names the root itself is
refused, even when nothing exists there. The resolved path is then canonicalized, symbolic links
included, and must still lie under the canonical repository root. A read takes at most
`SIZE_CAP_BYTES` (512 KB) plus one byte, the extra byte signalling an oversized file, so a huge
file costs one cap of memory.

`classify` turns the bytes into an outcome: valid UTF-8 within the cap renders as Markdown;
invalid UTF-8 falls back to lossy raw text with the note `NOTE_NON_UTF8`; an oversized file falls
back to text cut at the cap, a multi-byte character split by the cut dropped whole, followed by a
truncation notice that points at "open externally". Every refusal and I/O error becomes a fallback
whose note names the path and the error, with no text.

## Boundaries

- Depends on: `crate::document_viewer::models`, whose three types it re-exports; the crate's
  `Error` and `Result` (`DocumentRefused`); `std` (files, paths, channels and threads).
- Used by: the desktop application: `tools/tickets/ticketboard_desktop/src/application/lifecycle.rs`, which calls
  `spawn_read` from `open_doc`, with the rest of `tools/tickets/ticketboard_desktop/src/application/` naming
  `ViewerState` and `LoadedDocument` through this module; the ticket details
  (`tools/tickets/ticketboard_desktop/src/ticket_browser/ui/detail_panel/cells.rs` and `body_sections.rs`), which
  call `wants_viewer` to
  choose between the viewer and the external handler; and
  `tools/tickets/ticketboard_desktop/src/document_viewer/ui/`, which names `ViewerState` through it.
- Rules: nothing outside the repository root is opened, by `..` or by a symbolic link
  (`resolve_refuses_escapes`, `load_doc_refuses_escape_without_reading`,
  `load_doc_refuses_symlink_escape`); no read passes the cap
  (`classify_oversize_truncates_with_notice`, `load_doc_capped_truncates_on_disk_file`); no input
  panics, every failure ends as a fallback note (`load_doc_missing_file_names_the_error`). The
  symbolic-link check is best effort, since canonicalizing and then opening is not atomic.
