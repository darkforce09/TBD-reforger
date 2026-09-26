# Utilities

Small helpers with no domain of their own: timestamps formatted for the viewer, UTC instants read
and written without the browser clock, the countdown to a scheduled moment, the avatar sanitiser,
and the app's one clipboard write.

## Contents

```text
apps/website/frontend/src/v2/core/utils/
├── clipboard.rs      `write_clipboard`: copy text, and toast only once the browser confirms it
├── countdown.rs      `countdown_label`: the time left until a timestamp, in one rounded unit
├── datefmt.rs        a wire timestamp in the viewer's time zone, in three forms, and an uptime
├── mod.rs            the module tree; re-exports `countdown_label` and `safe_avatar_url`
├── sanitize.rs       `safe_avatar_url`: a stored web address, or the placeholder avatar
├── tests/            unit tests for the clipboard write and the UTC timestamps
└── utc_timestamp.rs  `UtcTimestamp`: UTC instants handled without the browser clock
```

## How it works

Every function is total: an input it cannot read gives a placeholder, never a panic.

| File | Gives |
|---|---|
| `datefmt.rs` | `format_local_datetime` ("Sat Aug 1, 21:00 GMT+2"), `format_short_date` ("Jun 12") and `log_stamp`, a fixed-width stamp for a log column, read through the browser's date object and time zone; an unreadable timestamp reads as a dash, or as a stamp of dashes in `log_stamp`; `format_uptime`, a duration as zero-padded hours, minutes and seconds |
| `countdown.rs` | `countdown_label`: "3 HOURS" and the like, "LIVE NOW" once the moment has passed, a dash for an unreadable timestamp |
| `utc_timestamp.rs` | `UtcTimestamp` parses an RFC 3339 instant only with a UTC designator, compares by value rather than by text, and converts to and from the `YYYY-MM-DDTHH:MM` value of a date-and-time field read as UTC; `utc_label` writes "YYYY-MM-DD HH:MM UTC", or the text itself when it is not a UTC instant |
| `sanitize.rs` | `safe_avatar_url`: the stored URL when `is_http_url` admits it, otherwise the placeholder avatar, a data URI the app ships and so never filters |
| `clipboard.rs` | `write_clipboard` (browser-only): awaits `navigator.clipboard.writeText`, then toasts the caller's success message, or the browser's reason when the write is refused |

Every surface that copies text calls `write_clipboard`, so "did the copy land" has one answer:
the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s clipboard exporters, the
server intel page's copy button and the
[server control](/documentation_v2/glossary/n_to_z.md#server-control) page's credential sheet.

## Boundaries

- Depends on: `crate::v2::core::auth::url_guard` and the placeholder avatar of
  `crate::v2::core::ui`, for `sanitize.rs`; the toast context of `crate::v2::core::ui`, for
  `clipboard.rs`; `js-sys` for the browser's dates; `web-sys`, `wasm-bindgen` and
  `wasm-bindgen-futures` for the clipboard in the browser build.
- Used by: the pages under `apps/website/frontend/src/v2/pages/`, among them the server intel
  copy button in `apps/website/frontend/src/v2/pages/command_center/server_intel/direct_connect.rs`
  and the credential sheet in
  `apps/website/frontend/src/v2/pages/administration/server_control/machine_credentials/credential_sheet.rs`;
  the Mission Creator's exporters in
  `apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/clipboard.rs`.
- Rules: the clipboard write toasts success only on the promise's resolve arm
  (`class_r_write_clipboard_toasts_only_on_the_resolve_arm` in `tests/clipboard.rs`); a UTC
  instant written back keeps its wire spelling, and anything but a UTC instant is refused
  (`the_wire_spellings_parse_and_write_back_unchanged` and
  `anything_but_a_valid_utc_instant_is_refused` in `tests/utc_timestamp.rs`).

## Related documentation

- [Frontend documentation](/documentation_v2/website/frontend/README.md#shared-foundations) — the shared foundations
  among the routes, pages and workspaces of the app.
