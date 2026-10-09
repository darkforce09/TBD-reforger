# Frontend UI source tree

The modules of `frontend_ui`, every one at the crate root: the interface primitives with the
overlay stack they share, and the helpers with no domain of their own.

## Contents

```text
crates/frontend/foundation/frontend_ui/src/
├── badge.rs            `badge_class`: the status pill's border, fill and text colours by variant name
├── byte_formatting.rs  `format_bytes` and `format_download_size`: byte counts in decimal and binary units
├── clipboard.rs        `write_clipboard`: copy text, and toast only once the browser confirms it
├── countdown.rs        `countdown_label`: the time left until a timestamp, in one rounded unit
├── datefmt.rs          a wire timestamp in the viewer's time zone, in three forms, and an uptime
├── dialog.rs           `Dialog`: the modal surface, a backdrop and centred panel, or no DOM when closed
├── icons.rs            `MaterialIcon`, the Material Symbols glyph, and the placeholder avatar
├── lib.rs              the module tree; re-exports the primitives and `safe_avatar_url`
├── modal_stack.rs      the stack the dialogs and sheets share: paint order and the one dismiss key
├── page_header.rs      `PageHeader`, the page title block, and `cn`, the class-string join
├── prelude.rs          the primitives and helpers most views import
├── safe_url.rs         `safe_link_href`, `safe_image_src`, `is_external_link`: authored URLs the app may write
├── sanitize.rs         `safe_avatar_url`: a stored web address, or the placeholder avatar
├── search_box.rs       `SearchBox`: a search input with its own glyph and clear button
├── select.rs           `Select`: a real `<select>` with the browser's arrow replaced by an icon
├── sheet.rs            `Sheet`: the dialog's shape anchored to an edge, for dossiers and detail panels
├── slider.rs           `Slider`: the range input with its track and handle painted
├── split_pane.rs       `SplitPane` and its row, filter field, empty state and match predicate
├── tests/              unit tests of the overlay stack, the URL policy, the byte sizes and the UTC instants
├── toast.rs            `Toasts`: transient notices, the context that raises them and their viewport
├── tokens.rs           `HOVER_FILL` and `DISABLED_GLYPH`: the hover and disabled state classes
└── utc_timestamp.rs    `UtcTimestamp`: UTC instants handled without the browser clock
```

## How it works

### Primitives

A primitive holds no state that belongs to its caller. The three form controls are uncontrolled:
each reads its value from the caller's signal and writes it to the DOM property, so the caller
owns the value and a fast-changing one costs a property write rather than a render. There is no
button, text input or tab primitive: callers write buttons and plain inputs inline, and each
surface that looks like tabs builds its own.

`Dialog` and `Sheet` render no DOM at all while closed, so a closed one cannot catch a click.
While open, each registers with `modal_stack`, the one piece of state here: the last-opened
surface paints on top and is the only one the Escape key closes, so a confirmation stacked over
an edit form closes alone. A small popover registers a closer instead of a stack entry, and any
overlay that opens closes it.

`Toasts`, provided once at the shell root, adds a notice through `success`, `error` or `message`
that removes itself after four seconds, and its viewport renders no DOM while the list is empty.
`SearchBox`, `Select` and `Slider` take their hover and disabled classes, `HOVER_FILL` and
`DISABLED_GLYPH`, from `tokens.rs`; the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s chrome layout
(`crates/frontend/workspaces/mission_creator_state/src/layout.rs`) re-exports the same two, so the chrome
and the form controls share one hover fill and one disabled dimming.

### Helpers

Every function is total: an input it cannot read gives a placeholder, never a panic.

| File | Gives |
|---|---|
| `datefmt.rs` | `format_local_datetime` ("Sat Aug 1, 21:00 GMT+2"), `format_short_date` ("Jun 12") and `log_stamp`, a fixed-width stamp for a log column, read through the browser's date object and time zone; an unreadable timestamp reads as a dash, or as a stamp of dashes in `log_stamp`; `format_uptime`, a duration as zero-padded hours, minutes and seconds |
| `countdown.rs` | `countdown_label`: "3 HOURS" and the like, "LIVE NOW" once the moment has passed, a dash for an unreadable timestamp |
| `utc_timestamp.rs` | `UtcTimestamp` parses an RFC 3339 instant only with a UTC designator, compares by value rather than by text, and converts to and from the `YYYY-MM-DDTHH:MM` value of a date-and-time field read as UTC; `utc_label` writes "YYYY-MM-DD HH:MM UTC", or the text itself when it is not a UTC instant |
| `safe_url.rs` | `safe_link_href` and `safe_image_src` give back an authored link target or image source when the content URL policy admits it, and `None` otherwise; `is_external_link` is true for a safe absolute `https`, `http` or `mailto` link. An image is `https://…` or a site path `/…`; a link may also be `http://…`, `mailto:…`, a `#fragment` or `/`. A backslash, a control character or whitespace anywhere, a protocol-relative `//host` and every other scheme are refused |
| `sanitize.rs` | `safe_avatar_url`: the stored URL when `is_http_url` admits it, otherwise the placeholder avatar, a data URI the app ships and so never filters |
| `byte_formatting.rs` | `format_bytes`: a byte count in decimal units — whole bytes below 1 000, whole kilobytes, then megabytes and gigabytes with one decimal ("999 B", "2 KB", "141.6 MB", "1.5 GB"); `format_download_size`: a modpack download size in binary units — "0 B" below one byte, whole megabytes of 1 024² B below a gibibyte, then gigabytes of 1 024³ B with one decimal ("500 MB", "1.5 GB") |
| `clipboard.rs` | `write_clipboard` (browser-only): awaits `navigator.clipboard.writeText`, then toasts the caller's success message, or the browser's reason when the write is refused |

Every surface that copies text calls `write_clipboard`, so "did the copy land" has one answer:
the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s clipboard exporters, the
server intel page's copy button and the
[server control](/documentation/glossary/n_to_z.md#server-control) page's credential sheet.

## Boundaries

- Depends on: `leptos`; `http_url_guard`, for `sanitize.rs`; `js-sys` for the browser's dates;
  `web-sys`, `wasm-bindgen` and `wasm-bindgen-futures` for the overlay stack's key listeners and
  timer and for the clipboard, in the wasm32 build only; the Material Symbols Outlined font
  `crates/frontend/shell/frontend_application/index.html` loads.
- Used by: the single-page app's route guard, content gates, app frame, features and pages, and
  the Mission Creator, whose dialogs and menus join the overlay stack and whose exporters call
  `write_clipboard`.
- Rules: only the topmost open overlay answers Escape
  (`only_the_topmost_open_overlay_answers_escape` in `tests/ui.rs`); the form controls never
  re-render per event; the form controls consume the state classes rather than re-typing a hover
  fill; `safe_url.rs` answers as the API's content URL policy
  (`crates/api/api_foundation/src/text/content_url_policy.rs`) does (`tests/safe_url.rs`); the
  clipboard write toasts success only on the promise's resolve arm; a UTC instant written back keeps its wire spelling, and anything but a UTC instant is refused
  (`the_wire_spellings_parse_and_write_back_unchanged` and
  `anything_but_a_valid_utc_instant_is_refused` in `tests/utc_timestamp.rs`).

## Related documentation

- [Frontend UI](/crates/frontend/foundation/frontend_ui/README.md) — the crate, its public
  surface and its dependencies.
